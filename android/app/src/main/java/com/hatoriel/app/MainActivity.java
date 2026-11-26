package com.hatoriel.app;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.SharedPreferences;
import android.os.Bundle;
import android.graphics.Color;
import android.view.View;
import android.widget.*;
import org.json.*;

public final class MainActivity extends Activity {
    private final int accent=Color.rgb(92,228,213);
    private SharedPreferences prefs;
    private JSONArray history=new JSONArray();
    private String base,model;
    private TextView status;
    private LinearLayout messages, settings;
    private ScrollView scroll;
    private EditText input, endpoint, modelField;
    private Button send,stop,clear;
    private ChatTransport active;
    private boolean ready,destroyed;
    private int dp(int value){return Math.round(value*getResources().getDisplayMetrics().density);}
    private TextView label(String value,int size){TextView t=new TextView(this);t.setText(value);t.setTextSize(size);t.setTextColor(Color.WHITE);return t;}
    private EditText field(String value,String hint){EditText e=new EditText(this);e.setText(value);e.setHint(hint);e.setSingleLine(true);return e;}
    private void append(String role,String content)throws JSONException{
        history.put(new JSONObject().put("role",role).put("content",content));
        while(history.length()>100)history.remove(0);
        prefs.edit().putString("history",history.toString()).apply();
    }
    private void render(){
        messages.removeAllViews();
        for(int i=0;i<history.length();i++){
            JSONObject m=history.optJSONObject(i);if(m==null)continue;
            boolean user="user".equals(m.optString("role"));
            TextView who=label(user?"ТИ":"HATORIEL",12);who.setTextColor(user?0xffb6c4d0:accent);messages.addView(who);
            TextView body=label(m.optString("content"),16);body.setTextIsSelectable(true);body.setPadding(dp(12),dp(10),dp(12),dp(10));body.setBackgroundColor(user?0xff25434c:0xff192831);
            LinearLayout.LayoutParams lp=new LinearLayout.LayoutParams(-1,-2);lp.bottomMargin=dp(12);messages.addView(body,lp);
        }
        scroll.post(()->scroll.fullScroll(View.FOCUS_DOWN));
    }
    private void busy(boolean value){send.setEnabled(ready&&!value);stop.setEnabled(value);clear.setEnabled(!value);endpoint.setEnabled(!value);modelField.setEnabled(!value);}
    @Override public void onCreate(Bundle state){
        super.onCreate(state);
        prefs=getSharedPreferences("hatoriel_chat",MODE_PRIVATE);
        base=prefs.getString("endpoint","http://127.0.0.1:11434");model=prefs.getString("model","qwen2.5:3b");
        try{history=new JSONArray(prefs.getString("history","[]"));}catch(JSONException ignored){history=new JSONArray();}
        LinearLayout root=new LinearLayout(this);root.setOrientation(LinearLayout.VERTICAL);root.setBackgroundColor(0xff0b1119);
        root.setPadding(dp(14),dp(12),dp(14),dp(12));
        root.setOnApplyWindowInsetsListener((v,insets)->{if(android.os.Build.VERSION.SDK_INT>=30){android.graphics.Insets bars=insets.getInsets(android.view.WindowInsets.Type.systemBars()|android.view.WindowInsets.Type.ime());v.setPadding(dp(14)+bars.left,dp(12)+bars.top,dp(14)+bars.right,dp(12)+bars.bottom);}return insets;});
        if(android.os.Build.VERSION.SDK_INT<30)root.setFitsSystemWindows(true);
        TextView title=label("HATORIEL  /  MOBILE",24);title.setTextColor(accent);root.addView(title);
        status=label("Ініціалізація…",12);root.addView(status);
        Button config=new Button(this);config.setText("Налаштування підключення");root.addView(config);
        settings=new LinearLayout(this);settings.setOrientation(LinearLayout.VERTICAL);settings.setVisibility(View.GONE);root.addView(settings);
        endpoint=field(base,"URL Ollama-сервера");modelField=field(model,"Назва моделі");settings.addView(endpoint);settings.addView(modelField);
        settings.addView(label("HTTP дозволено лише на цьому телефоні. HTTPS — для явно вибраного сервера. Історія надсилається на вибраний сервер.",12));
        Button save=new Button(this);save.setText("Зберегти підключення");settings.addView(save);
        config.setOnClickListener(v->settings.setVisibility(settings.getVisibility()==View.VISIBLE?View.GONE:View.VISIBLE));
        save.setOnClickListener(v->{if(active!=null)return;String b=endpoint.getText().toString().trim(),m=modelField.getText().toString().trim();
            try{ChatTransport.endpoint(b);if(m.isEmpty())throw new IllegalArgumentException("Вкажи модель");
                new AlertDialog.Builder(this).setTitle("Підтвердити сервер").setMessage("Повідомлення та частина історії надсилатимуться на "+b+". Продовжити?").setNegativeButton("Скасувати",null).setPositiveButton("Зберегти",(d,w)->{base=b;model=m;prefs.edit().putString("endpoint",b).putString("model",m).apply();settings.setVisibility(View.GONE);status.setText("Сервер налаштовано (ще не перевірено)");}).show();
            }catch(Exception e){status.setText(e.getMessage());}});
        clear=new Button(this);clear.setText("Очистити історію");settings.addView(clear);
        clear.setOnClickListener(v->new AlertDialog.Builder(this).setTitle("Видалити локальну історію?").setNegativeButton("Скасувати",null).setPositiveButton("Видалити",(d,w)->{if(active==null){history=new JSONArray();prefs.edit().remove("history").apply();render();}}).show());
        scroll=new ScrollView(this);messages=new LinearLayout(this);messages.setOrientation(LinearLayout.VERTICAL);scroll.addView(messages);root.addView(scroll,new LinearLayout.LayoutParams(-1,0,1));
        input=new EditText(this);input.setHint("Напиши повідомлення…");input.setMinLines(2);input.setMaxLines(4);root.addView(input);
        LinearLayout buttons=new LinearLayout(this);send=new Button(this);send.setText("Надіслати");stop=new Button(this);stop.setText("Зупинити");buttons.addView(send,new LinearLayout.LayoutParams(0,-2,1));buttons.addView(stop,new LinearLayout.LayoutParams(0,-2,1));root.addView(buttons);
        try{status.setText("Rust Core "+NativeCore.version()+" · "+base);ready=true;}catch(LinkageError e){status.setText("Rust Core недоступне: "+e.getMessage());}
        send.setOnClickListener(v->generate());stop.setOnClickListener(v->{if(active!=null){active.cancel();status.setText("Зупинення…");}});
        setContentView(root);root.requestApplyInsets();render();busy(false);
    }
    private void generate(){
        String query=input.getText().toString().trim();if(!ready||active!=null||query.isEmpty())return;
        if(query.codePointCount(0,query.length())>12000){status.setText("Максимум 12000 символів");return;}
        try{append("user",query);}catch(JSONException e){status.setText(e.toString());return;}
        input.setText("");render();
        JSONArray context=new JSONArray();for(int i=Math.max(0,history.length()-32);i<history.length();i++)context.put(history.opt(i));
        // Bound the JNI input while retaining the newest user message.
        while(context.length()>1&&context.toString().getBytes(java.nio.charset.StandardCharsets.UTF_8).length>240000)context.remove(0);
        final String data=context.toString(),server=base,selectedModel=model;
        ChatTransport transport=new ChatTransport();active=transport;busy(true);status.setText("Hatoriel думає…");
        new Thread(()->{
            String reply=null,error=null;
            try{String prepared=NativeCore.prepareHistory(data);if(prepared==null||!prepared.startsWith("["))throw new IllegalStateException("Core: "+prepared);
                reply=transport.request(server,selectedModel,new JSONArray(prepared));
            }catch(Exception|LinkageError e){error=e.getMessage()==null?e.toString():e.getMessage();}
            final String result=reply,failure=error;
            runOnUiThread(()->{if(destroyed||active!=transport)return;active=null;busy(false);
                if(transport.isCancelled()){status.setText("Зупинено");return;}
                if(failure!=null){status.setText("Помилка: "+failure);return;}
                try{append("assistant",result);render();status.setText("Готово");}catch(JSONException e){status.setText(e.toString());}
            });
        },"hatoriel-inference").start();
    }
    @Override protected void onDestroy(){destroyed=true;if(active!=null)active.cancel();super.onDestroy();}
}
