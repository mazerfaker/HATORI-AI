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
    private final int accent=0xffce8297;
    private TextView draftView;
    private Button retry;
    private String draftText="";
    private SharedPreferences prefs;
    private MobileMemory memory;
    private Button memoryButton;
    private JSONArray history=new JSONArray();
    private String base,model;
    private TextView status;
    private LinearLayout messages, settings;
    private ScrollView scroll;
    private EditText input, endpoint, modelField;
    private Button send,stop,clear;
    private ChatTransport active;
    private boolean ready,destroyed,importing;
    private TextView localModelStatus;
    private Button importModel;
    private Thread importWorker;
    private int dp(int value){return Math.round(value*getResources().getDisplayMetrics().density);}
    private TextView label(String value,int size){TextView t=new TextView(this);t.setText(value);t.setTextSize(size);t.setTextColor(0xfff4f0f2);return t;}
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
            TextView who=label(user?"ТИ":"HATORIEL",12);who.setTextColor(user?0xffb5aab0:accent);messages.addView(who);
            TextView body=label(m.optString("content"),16);body.setTextIsSelectable(true);body.setPadding(dp(12),dp(10),dp(12),dp(10));android.graphics.drawable.GradientDrawable shape=new android.graphics.drawable.GradientDrawable();shape.setColor(user?0xff39212b:0xff161417);shape.setCornerRadius(dp(12));body.setBackground(shape);body.setOnLongClickListener(v->{android.content.ClipboardManager clipboard=(android.content.ClipboardManager)getSystemService(CLIPBOARD_SERVICE);clipboard.setPrimaryClip(android.content.ClipData.newPlainText("Hatoriel",m.optString("content")));Toast.makeText(this,"Скопійовано",Toast.LENGTH_SHORT).show();return true;});
            LinearLayout.LayoutParams lp=new LinearLayout.LayoutParams(-1,-2);lp.bottomMargin=dp(12);messages.addView(body,lp);
        }
        draftView=label(draftText,16);draftView.setTextColor(0xfff4f0f2);draftView.setPadding(dp(12),dp(10),dp(12),dp(10));draftView.setBackgroundColor(0xff161417);draftView.setVisibility(active==null?View.GONE:View.VISIBLE);messages.addView(draftView);
        scroll.post(()->scroll.fullScroll(View.FOCUS_DOWN));
    }
    private void busy(boolean value){send.setEnabled(ready&&!value);stop.setEnabled(value);clear.setEnabled(!value);memoryButton.setEnabled(!value);retry.setEnabled(ready&&!value&&history.length()>0);endpoint.setEnabled(!value);modelField.setEnabled(!value);importModel.setEnabled(!value&&!importing);}
    @Override public void onCreate(Bundle state){
        super.onCreate(state);
        prefs=getSharedPreferences("hatoriel_chat",MODE_PRIVATE);memory=new MobileMemory(prefs);
        base=prefs.getString("endpoint","http://127.0.0.1:11434");model=prefs.getString("model","qwen2.5:3b");
        try{history=new JSONArray(prefs.getString("history","[]"));}catch(JSONException ignored){history=new JSONArray();}
        LinearLayout root=new LinearLayout(this);root.setOrientation(LinearLayout.VERTICAL);root.setBackgroundColor(0xff09090b);
        root.setPadding(dp(14),dp(12),dp(14),dp(12));
        root.setOnApplyWindowInsetsListener((v,insets)->{if(android.os.Build.VERSION.SDK_INT>=30){android.graphics.Insets bars=insets.getInsets(android.view.WindowInsets.Type.systemBars()|android.view.WindowInsets.Type.ime());v.setPadding(dp(14)+bars.left,dp(12)+bars.top,dp(14)+bars.right,dp(12)+bars.bottom);}return insets;});
        if(android.os.Build.VERSION.SDK_INT<30)root.setFitsSystemWindows(true);
        TextView title=label("HATORIEL  /  MOBILE",24);title.setTextColor(0xfff4f0f2);root.addView(title);
        status=label("Ініціалізація…",12);status.setTextColor(0xffb5aab0);root.addView(status);
        Button config=new Button(this);config.setText("Налаштування підключення");root.addView(config);memoryButton=new Button(this);memoryButton.setText("Пам’ять та профіль");root.addView(memoryButton);memoryButton.setOnClickListener(v->showMemory());
        settings=new LinearLayout(this);settings.setOrientation(LinearLayout.VERTICAL);settings.setVisibility(View.GONE);root.addView(settings);
        endpoint=field(base,"URL Ollama-сервера");modelField=field(model,"Назва моделі");settings.addView(endpoint);settings.addView(modelField);
        settings.addView(label("HTTP дозволено лише на цьому телефоні. HTTPS — для явно вибраного сервера. Історія надсилається на вибраний сервер.",12));
        Button save=new Button(this);save.setText("Зберегти підключення");settings.addView(save);
        config.setOnClickListener(v->settings.setVisibility(settings.getVisibility()==View.VISIBLE?View.GONE:View.VISIBLE));
        save.setOnClickListener(v->{if(active!=null)return;String b=endpoint.getText().toString().trim(),m=modelField.getText().toString().trim();
            try{ChatTransport.endpoint(b);if(m.isEmpty())throw new IllegalArgumentException("Вкажи модель");
                new AlertDialog.Builder(this).setTitle("Підтвердити сервер").setMessage("Повідомлення, частина історії та, якщо дозволено, профіль і релевантні спогади надсилатимуться на "+b+". Продовжити?").setNegativeButton("Скасувати",null).setPositiveButton("Зберегти",(d,w)->{base=b;model=m;prefs.edit().putString("endpoint",b).putString("model",m).apply();settings.setVisibility(View.GONE);status.setText("Сервер налаштовано (ще не перевірено)");}).show();
            }catch(Exception e){status.setText(e.getMessage());}});
        localModelStatus=label("Локальна модель: підготовка runtime",12);settings.addView(localModelStatus);
        importModel=new Button(this);importModel.setText("Імпортувати GGUF");settings.addView(importModel);
        importModel.setOnClickListener(v->{if(importing||active!=null)return;new AlertDialog.Builder(this).setTitle("Імпорт локальної моделі").setMessage("Файл копіюється в приватне сховище Hatoriel. Попередня локальна модель буде замінена після успішного імпорту. Запуск моделі ще експериментальний і не підключений.").setNegativeButton("Скасувати",null).setPositiveButton("Обрати файл",(d,w)->{android.content.Intent intent=new android.content.Intent(android.content.Intent.ACTION_OPEN_DOCUMENT);intent.setType("*/*");intent.addCategory(android.content.Intent.CATEGORY_OPENABLE);try{startActivityForResult(intent,71);}catch(android.content.ActivityNotFoundException e){status.setText("Файловий менеджер недоступний");}}).show();});
        refreshLocalModel();
        clear=new Button(this);clear.setText("Очистити історію");settings.addView(clear);
        clear.setOnClickListener(v->new AlertDialog.Builder(this).setTitle("Видалити локальну історію?").setNegativeButton("Скасувати",null).setPositiveButton("Видалити",(d,w)->{if(active==null){history=new JSONArray();prefs.edit().remove("history").apply();render();}}).show());
        scroll=new ScrollView(this);messages=new LinearLayout(this);messages.setOrientation(LinearLayout.VERTICAL);scroll.addView(messages);root.addView(scroll,new LinearLayout.LayoutParams(-1,0,1));
        input=new EditText(this);input.setHint("Напиши повідомлення…");input.setMinLines(2);input.setMaxLines(4);root.addView(input);
        LinearLayout buttons=new LinearLayout(this);send=new Button(this);send.setText("Надіслати");stop=new Button(this);stop.setText("Зупинити");buttons.addView(send,new LinearLayout.LayoutParams(0,-2,1));buttons.addView(stop,new LinearLayout.LayoutParams(0,-2,1));root.addView(buttons);send.setBackgroundTintList(android.content.res.ColorStateList.valueOf(0xff68263a));retry=new Button(this);retry.setText("Повторити останній запит");root.addView(retry);retry.setOnClickListener(v->retryLast());
        try{status.setText("Rust Core "+NativeCore.version()+" · "+base);ready=true;}catch(LinkageError e){status.setText("Rust Core недоступне: "+e.getMessage());}
        send.setOnClickListener(v->generate());stop.setOnClickListener(v->{if(active!=null){active.cancel();status.setText("Зупинення…");}});
        setContentView(root);root.requestApplyInsets();render();busy(false);
    }
    private void generate(){
        String query=input.getText().toString().trim();if(!ready||active!=null||query.isEmpty())return;
        if(query.codePointCount(0,query.length())>12000){status.setText("Максимум 12000 символів");return;}
        try{append("user",query);}catch(JSONException e){status.setText(e.toString());return;}
        input.setText("");offerMemory(query);startRequest();
    }
    private void retryLast(){
        if(!ready||active!=null||history.length()==0)return;
        JSONObject last=history.optJSONObject(history.length()-1);
        if(last==null)return;
        if("user".equals(last.optString("role"))){startRequest();return;}
        if(history.length()<2||!"assistant".equals(last.optString("role")))return;
        JSONObject user=history.optJSONObject(history.length()-2);
        if(user==null||!"user".equals(user.optString("role")))return;
        new AlertDialog.Builder(this).setTitle("Повторити генерацію?").setMessage("Попередня відповідь буде замінена.").setNegativeButton("Скасувати",null).setPositiveButton("Повторити",(d,w)->{if(active!=null)return;history.remove(history.length()-1);prefs.edit().putString("history",history.toString()).apply();startRequest();}).show();
    }
    private void startRequest(){
        JSONArray context=new JSONArray();for(int i=Math.max(0,history.length()-32);i<history.length();i++)context.put(history.opt(i));
        // Bound the JNI input while retaining the newest user message.
        while(context.length()>1&&context.toString().getBytes(java.nio.charset.StandardCharsets.UTF_8).length>240000)context.remove(0);
        final String data;try{data=memory.context(context);}catch(JSONException e){status.setText(e.toString());return;}final String server=base,selectedModel=model;
        ChatTransport transport=new ChatTransport();active=transport;draftText="";render();busy(true);status.setText("Hatoriel думає…");
        new Thread(()->{
            String reply=null,error=null;
            try{String prepared=NativeCore.prepareMemory(data);if(prepared==null||!prepared.startsWith("["))throw new IllegalStateException("Core: "+prepared);
                final long[] lastUpdate={0};reply=transport.request(server,selectedModel,new JSONArray(prepared),partial->{long now=android.os.SystemClock.elapsedRealtime();if(now-lastUpdate[0]<60)return;lastUpdate[0]=now;runOnUiThread(()->{if(destroyed||active!=transport||transport.isCancelled())return;draftText=partial;draftView.setText(partial);scroll.post(()->scroll.fullScroll(View.FOCUS_DOWN));});});
            }catch(Exception|LinkageError e){error=e.getMessage()==null?e.toString():e.getMessage();}
            final String result=reply,failure=error;
            runOnUiThread(()->{if(destroyed||active!=transport)return;active=null;draftText="";render();busy(false);
                if(transport.isCancelled()){status.setText("Зупинено");return;}
                if(failure!=null){status.setText("Помилка: "+failure);return;}
                try{append("assistant",result);render();busy(false);status.setText("Готово");send.performHapticFeedback(android.view.HapticFeedbackConstants.CLOCK_TICK);}catch(JSONException e){status.setText(e.toString());}
            });
        },"hatoriel-inference").start();
    }
    private void offerMemory(String query){
        if(!memory.suggestions())return;
        try{String raw=NativeCore.suggestMemory(query);if(raw==null||"null".equals(raw))return;JSONObject m=new JSONObject(raw);
            String content=m.getString("content");if(memory.contains(content))return;
            new AlertDialog.Builder(this).setTitle("Запам’ятати?").setMessage(content+"\n"+m.optString("category")+" · "+m.optInt("importance",50)+"/100")
                .setNegativeButton("Відхилити",null).setPositiveButton("Зберегти",(d,w)->{try{memory.save(content,m.optString("category","General"),m.optInt("importance",50));Toast.makeText(this,"Спогад збережено",Toast.LENGTH_SHORT).show();}catch(Exception e){status.setText(e.getMessage());}}).show();
        }catch(Exception|LinkageError e){status.setText("Помилка пропозиції спогаду");}
    }
    private void showMemory(){
        if(active!=null)return;
        ScrollView sv=new ScrollView(this);LinearLayout panel=new LinearLayout(this);panel.setOrientation(LinearLayout.VERTICAL);panel.setPadding(dp(20),dp(8),dp(20),dp(8));sv.addView(panel);
        CheckBox use=new CheckBox(this);use.setText("Використовувати пам’ять у відповідях");use.setChecked(memory.enabled());panel.addView(use);
        use.setOnCheckedChangeListener((b,checked)->{if(!checked){memory.setEnabled(false);return;}new AlertDialog.Builder(this).setTitle("Дозволити контекст пам’яті?").setMessage("Релевантні спогади та профіль передаватимуться вибраному AI-серверу: "+base).setNegativeButton("Ні",(d,w)->use.setChecked(false)).setPositiveButton("Дозволити",(d,w)->memory.setEnabled(true)).setOnCancelListener(d->use.setChecked(false)).show();});
        CheckBox suggest=new CheckBox(this);suggest.setText("Пропонувати нові спогади");suggest.setChecked(memory.suggestions());panel.addView(suggest);suggest.setOnCheckedChangeListener((b,checked)->memory.setSuggestions(checked));
        Button add=new Button(this);add.setText("Додати спогад");panel.addView(add);
        Button profile=new Button(this);profile.setText("Редагувати профіль");panel.addView(profile);
        JSONArray rows=memory.records();
        for(int i=rows.length()-1;i>=0;i--){JSONObject m=rows.optJSONObject(i);if(m==null)continue;TextView item=label(m.optString("content")+"\n"+m.optString("category")+" · "+m.optInt("importance")+"/100",14);panel.addView(item);
            Button remove=new Button(this);remove.setText("Видалити");panel.addView(remove);remove.setOnClickListener(v->new AlertDialog.Builder(this).setTitle("Видалити спогад?").setNegativeButton("Скасувати",null).setPositiveButton("Видалити",(d,w)->{memory.remove(m.optLong("id"));item.setVisibility(View.GONE);remove.setVisibility(View.GONE);}).show());
        }
        AlertDialog dialog=new AlertDialog.Builder(this).setTitle("Пам’ять Hatoriel").setView(sv).setPositiveButton("Закрити",null).create();
        add.setOnClickListener(v->{dialog.dismiss();addMemory();});profile.setOnClickListener(v->{dialog.dismiss();editProfile();});dialog.show();
    }
    private void addMemory(){
        LinearLayout panel=new LinearLayout(this);panel.setOrientation(LinearLayout.VERTICAL);panel.setPadding(dp(20),0,dp(20),0);
        EditText text=new EditText(this);text.setHint("Текст спогаду");panel.addView(text);
        Spinner category=new Spinner(this);category.setAdapter(new ArrayAdapter<String>(this,android.R.layout.simple_spinner_dropdown_item,MobileMemory.CATEGORIES));panel.addView(category);
        SeekBar priority=new SeekBar(this);priority.setMax(100);priority.setProgress(50);panel.addView(label("Важливість",12));panel.addView(priority);
        AlertDialog d=new AlertDialog.Builder(this).setTitle("Новий спогад").setView(panel).setNegativeButton("Скасувати",null).setPositiveButton("Зберегти",null).create();
        d.setOnShowListener(v->d.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener(b->{try{memory.save(text.getText().toString(),String.valueOf(category.getSelectedItem()),priority.getProgress());d.dismiss();showMemory();}catch(Exception e){text.setError(e.getMessage());}}));d.show();
    }
    private void editProfile(){
        JSONObject p=memory.profile();LinearLayout panel=new LinearLayout(this);panel.setOrientation(LinearLayout.VERTICAL);panel.setPadding(dp(20),0,dp(20),0);
        EditText interests=field(p.optString("interests"),"Інтереси"),communication=field(p.optString("communication"),"Стиль спілкування"),goals=field(p.optString("goals"),"Цілі");
        panel.addView(interests);panel.addView(communication);panel.addView(goals);
        new AlertDialog.Builder(this).setTitle("Профіль").setView(panel).setNegativeButton("Скасувати",null).setPositiveButton("Зберегти",(d,w)->{try{memory.saveProfile(interests.getText().toString(),communication.getText().toString(),goals.getText().toString());showMemory();}catch(JSONException e){status.setText(e.toString());}}).show();
    }
    private void refreshLocalModel(){try{java.io.File file=ModelImport.selected(this);localModelStatus.setText(file.isFile()?String.format(java.util.Locale.ROOT,"GGUF: %.2f GiB · імпортовано, запуск ще недоступний",file.length()/1073741824.0):"GGUF не імпортовано");}catch(java.io.IOException e){localModelStatus.setText(e.getMessage());}}
    @Override protected void onActivityResult(int request,int result,android.content.Intent data){super.onActivityResult(request,result,data);if(request!=71||result!=RESULT_OK||data==null||data.getData()==null||importing)return;android.net.Uri uri=data.getData();importing=true;importModel.setEnabled(false);localModelStatus.setText("Імпорт GGUF…");android.content.Context app=getApplicationContext();importWorker=new Thread(()->{String failure=null;try{ModelImport.copy(app,uri);}catch(Exception e){failure=e.getMessage();}final String error=failure;runOnUiThread(()->{if(destroyed)return;importing=false;importModel.setEnabled(active==null);if(error==null)refreshLocalModel();else localModelStatus.setText("Імпорт: "+error);});},"hatoriel-model-import");importWorker.start();}
    @Override protected void onDestroy(){destroyed=true;if(importWorker!=null)importWorker.interrupt();if(active!=null)active.cancel();super.onDestroy();}
}
