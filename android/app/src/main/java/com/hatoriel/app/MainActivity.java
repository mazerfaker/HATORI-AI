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
    private final int accent=CrimsonAvatar.RED;
    private CrimsonAvatar avatar;
    private long nextAvatarMoodRefresh=0;
    private String avatarMood="Calm";
    private int avatarIntensity=0;
    private VoiceOutput voice;
    private static final int ATTACH=73, SAVE_TRANSFER=74;
    private FileTransfer transfer;
    private FileTransfer.Received receivedTransfer;
    private TextView transferStatus;
    private final AttachmentDraft attachments=new AttachmentDraft();
    private LinearLayout attachmentRows;
    private boolean attaching;
    private Thread attachmentWorker;
    private Button attachButton;
    private boolean voiceMode, foreground, externalDictation;
    private CrimsonAvatar welcomeAvatar;
    private Long voiceReplyJob;
    private TextView voiceState;
    private static final int DICTATION=72;
    private ScrollView settingsScroll;
    private LinearLayout composer, bottomNav;
    private TextView draftView;
    private Button retry;
    private String draftText="";
    private SharedPreferences prefs;
    private MobileMemory memory;
    private Button memoryButton;
    private JSONArray history=new JSONArray();
    private String base,model;
    private static String bridgeKey="",bridgeEndpoint="";
    private EditText bridgeKeyField;
    private TextView status;
    private LinearLayout messages, settings;
    private ScrollView scroll;
    private EditText input, endpoint, modelField;
    private Button send,stop,clear;
    private ChatTransport active;
    private final android.os.Handler sessionHandler=new android.os.Handler(android.os.Looper.getMainLooper());
    private long sessionRevision=-1;
    private final Runnable sessionWatch=new Runnable(){public void run(){syncSession();sessionHandler.postDelayed(this,150);}};
    private boolean ready,destroyed,importing;
    private TextView localModelStatus;
    private Button importModel;
    private Button hardwareButton;
    private Thread importWorker;
    private int dp(int value){return Math.round(value*getResources().getDisplayMetrics().density);}
    private TextView label(String value,int size){TextView t=new TextView(this);t.setText(value);t.setTextSize(size);t.setTextColor(0xfff2f2f2);return t;}
    private EditText field(String value,String hint){EditText e=new EditText(this);e.setText(value);e.setHint(hint);e.setSingleLine(true);return e;}
    private void append(String role,String content)throws JSONException{
        history.put(new JSONObject().put("role",role).put("content",content));
        while(history.length()>100)history.remove(0);
        prefs.edit().putString("history",history.toString()).apply();
    }
    private void render(){
        messages.removeAllViews();welcomeAvatar=null;
        if(history.length()==0&&active==null){CrimsonAvatar welcome=new CrimsonAvatar(this);welcomeAvatar=welcome;welcome.setMotion(prefs.getBoolean("avatar_motion",true));welcome.setMood(avatarMood,avatarIntensity);messages.addView(welcome,new LinearLayout.LayoutParams(-1,dp(160)));TextView greeting=label("Готова до діалогу",20);greeting.setGravity(android.view.Gravity.CENTER);messages.addView(greeting);TextView hint=label("Чим можу допомогти?",14);hint.setGravity(android.view.Gravity.CENTER);messages.addView(hint);}
        for(int i=0;i<history.length();i++){
            JSONObject m=history.optJSONObject(i);if(m==null)continue;
            boolean user="user".equals(m.optString("role"));
            TextView who=label(user?"ТИ":"HATORIEL",12);who.setTextColor(user?0xffbcbcbc:accent);messages.addView(who);
            TextView body=label(m.optString("content"),16);body.setTextIsSelectable(true);body.setPadding(dp(12),dp(10),dp(12),dp(10));android.graphics.drawable.GradientDrawable shape=new android.graphics.drawable.GradientDrawable();shape.setColor(user?0xff352526:0xff2b2b2e);shape.setCornerRadius(dp(12));body.setBackground(shape);body.setOnLongClickListener(v->{android.content.ClipboardManager clipboard=(android.content.ClipboardManager)getSystemService(CLIPBOARD_SERVICE);clipboard.setPrimaryClip(android.content.ClipData.newPlainText("Hatoriel",m.optString("content")));Toast.makeText(this,"Скопійовано",Toast.LENGTH_SHORT).show();return true;});
            LinearLayout.LayoutParams lp=new LinearLayout.LayoutParams(-1,-2);lp.bottomMargin=dp(12);messages.addView(body,lp);
        }
        draftView=label(draftText,16);draftView.setTextColor(0xfff2f2f2);draftView.setPadding(dp(12),dp(10),dp(12),dp(10));draftView.setBackgroundColor(0xff2b2b2e);draftView.setVisibility(active==null?View.GONE:View.VISIBLE);messages.addView(draftView);
        scroll.post(()->scroll.fullScroll(View.FOCUS_DOWN));
    }
    private void busy(boolean value){send.setEnabled(ready&&!value&&!attaching&&attachments.items.isEmpty());if(attachButton!=null)attachButton.setEnabled(!value&&!attaching&&attachments.items.size()<AttachmentDraft.MAX_COUNT);stop.setEnabled(value);clear.setEnabled(!value);memoryButton.setEnabled(!value);hardwareButton.setEnabled(!value&&!importing);retry.setEnabled(ready&&!value&&history.length()>0);endpoint.setEnabled(!value);modelField.setEnabled(!value);bridgeKeyField.setEnabled(!value);importModel.setEnabled(!value&&!importing);}
    @Override public void onCreate(Bundle state){
        super.onCreate(state);
        if(android.os.Build.VERSION.SDK_INT>=30)getWindow().setDecorFitsSystemWindows(false);
        prefs=getSharedPreferences("hatoriel_chat",MODE_PRIVATE);memory=new MobileMemory(prefs);voice=new VoiceOutput(this);
        base=prefs.getString("endpoint","http://127.0.0.1:11434");model=prefs.getString("model","qwen2.5:3b");
        try{history=new JSONArray(prefs.getString("history","[]"));}catch(JSONException ignored){history=new JSONArray();}
        LinearLayout root=new LinearLayout(this);root.setOrientation(LinearLayout.VERTICAL);root.setBackgroundColor(0xff1c1c1e);
        root.setPadding(dp(14),dp(12),dp(14),dp(12));
        root.setOnApplyWindowInsetsListener((v,insets)->{if(android.os.Build.VERSION.SDK_INT>=30){android.graphics.Insets bars=insets.getInsets(android.view.WindowInsets.Type.systemBars()|android.view.WindowInsets.Type.ime());v.setPadding(dp(14)+bars.left,dp(12)+bars.top,dp(14)+bars.right,dp(12)+bars.bottom);if(bottomNav!=null)bottomNav.setVisibility(insets.isVisible(android.view.WindowInsets.Type.ime())?View.GONE:View.VISIBLE);}return insets;});
        if(android.os.Build.VERSION.SDK_INT<30)root.setFitsSystemWindows(true);
        LinearLayout header=new LinearLayout(this);header.setOrientation(LinearLayout.HORIZONTAL);header.setGravity(android.view.Gravity.CENTER_VERTICAL);root.addView(header);
        avatar=new CrimsonAvatar(this);avatar.setMotion(prefs.getBoolean("avatar_motion",true));header.addView(avatar,new LinearLayout.LayoutParams(dp(56),dp(56)));
        LinearLayout brand=new LinearLayout(this);brand.setOrientation(LinearLayout.VERTICAL);header.addView(brand,new LinearLayout.LayoutParams(0,-2,1));
        TextView title=label("H A T O R I E L",20);title.setTypeface(null,android.graphics.Typeface.BOLD);brand.addView(title);
        TextView edition=label("CRIMSON CORE",11);edition.setTextColor(accent);brand.addView(edition);
        status=label("Ініціалізація…",12);status.setTextColor(0xffbcbcbc);root.addView(status);
        Button config=new Button(this);config.setText("Налашт.");
        memoryButton=new Button(this);memoryButton.setText("Пам’ять");memoryButton.setOnClickListener(v->showMemory());
        hardwareButton=new Button(this);hardwareButton.setText("Моделі");hardwareButton.setOnClickListener(v->showHardware());
        Button reminders=new Button(this);reminders.setText("Завдання");reminders.setOnClickListener(v->showReminders());
        settingsScroll=new ScrollView(this);settingsScroll.setVisibility(View.GONE);
        settings=new LinearLayout(this);settings.setOrientation(LinearLayout.VERTICAL);settingsScroll.addView(settings);root.addView(settingsScroll,new LinearLayout.LayoutParams(-1,0,1));
        endpoint=field(base,"URL Ollama-сервера");modelField=field(model,"Назва моделі");settings.addView(endpoint);settings.addView(modelField);bridgeKeyField=field(base.equals(bridgeEndpoint)?bridgeKey:"","Ключ сеансу Hatoriel (необов’язково)");bridgeKeyField.setInputType(android.text.InputType.TYPE_CLASS_TEXT|android.text.InputType.TYPE_TEXT_VARIATION_PASSWORD);bridgeKeyField.setSaveEnabled(false);settings.addView(bridgeKeyField);settings.addView(label("Для шлюзу ПК потрібен HTTPS і ключ із розділу «Пристрої». Ключ зберігається лише в оперативній пам’яті до завершення процесу.",12));
        settings.addView(label("HTTP дозволено лише на цьому телефоні. HTTPS — для явно вибраного сервера. Історія надсилається на вибраний сервер.",12));
        Button save=new Button(this);save.setText("Зберегти підключення");settings.addView(save);
        config.setOnClickListener(v->showSettings(settingsScroll.getVisibility()!=View.VISIBLE));
        save.setOnClickListener(v->{if(GenerationSession.active!=null)return;String b=endpoint.getText().toString().trim(),m=modelField.getText().toString().trim(),key=bridgeKeyField.getText().toString().trim();
            try{java.net.URL url=ChatTransport.endpoint(b);if(!key.isEmpty()&&(!"https".equalsIgnoreCase(url.getProtocol())||!key.matches("[0-9a-f]{64}")))throw new IllegalArgumentException("Для ключа потрібен HTTPS та 64 hex-символи");if(m.isEmpty())throw new IllegalArgumentException("Вкажи модель");
                new AlertDialog.Builder(this).setTitle("Підтвердити сервер").setMessage("Повідомлення, частина історії та, якщо дозволено, профіль і релевантні спогади надсилатимуться на "+b+". Продовжити?").setNegativeButton("Скасувати",null).setPositiveButton("Зберегти",(d,w)->{if(!base.equals(b))memory.setEnabled(false);base=b;model=m;bridgeKey=key;bridgeEndpoint=key.isEmpty()?"":b;prefs.edit().putString("endpoint",b).putString("model",m).apply();showSettings(false);status.setText("Сервер налаштовано (ще не перевірено)");}).show();
            }catch(Exception e){status.setText(e.getMessage());}});
        EditText transferCode=field("","Одноразовий код файлу з ПК");transferCode.setSaveEnabled(false);settings.addView(transferCode);
        transferStatus=label("Файл із ПК · до 8 МіБ",12);settings.addView(transferStatus);
        Button receiveFile=new Button(this);receiveFile.setText("Отримати файл з ПК");settings.addView(receiveFile);receiveFile.setOnClickListener(v->{String code=transferCode.getText().toString().trim();transferCode.setText("");receiveTransfer(code);});
        Button cancelFile=new Button(this);cancelFile.setText("Скасувати отримання");settings.addView(cancelFile);cancelFile.setOnClickListener(v->{if(transfer!=null)transfer.cancel();receivedTransfer=null;transferStatus.setText("Скасовано");});
        Button personalityButton=new Button(this);personalityButton.setText("Особистість Hatoriel");settings.addView(personalityButton);personalityButton.setOnClickListener(v->showPersonality());
        localModelStatus=label("Локальна модель: підготовка runtime",12);settings.addView(localModelStatus);
        importModel=new Button(this);importModel.setText("Імпортувати GGUF");settings.addView(importModel);
        importModel.setOnClickListener(v->{if(importing||active!=null)return;new AlertDialog.Builder(this).setTitle("Імпорт локальної моделі").setMessage("Файл копіюється в приватне сховище Hatoriel. Попередня локальна модель буде замінена після успішного імпорту. Запуск моделі ще експериментальний і не підключений.").setNegativeButton("Скасувати",null).setPositiveButton("Обрати файл",(d,w)->{android.content.Intent intent=new android.content.Intent(android.content.Intent.ACTION_OPEN_DOCUMENT);intent.setType("*/*");intent.addCategory(android.content.Intent.CATEGORY_OPENABLE);try{startActivityForResult(intent,71);}catch(android.content.ActivityNotFoundException e){status.setText("Файловий менеджер недоступний");}}).show();});
        refreshLocalModel();
        CheckBox background=new CheckBox(this);background.setText("Продовжувати генерацію після згортання");background.setChecked(prefs.getBoolean("background_generation",false));settings.addView(background);
        background.setOnCheckedChangeListener((b,value)->{prefs.edit().putBoolean("background_generation",value).apply();if(value&&android.os.Build.VERSION.SDK_INT>=33&&checkSelfPermission(android.Manifest.permission.POST_NOTIFICATIONS)!=android.content.pm.PackageManager.PERMISSION_GRANTED)requestPermissions(new String[]{android.Manifest.permission.POST_NOTIFICATIONS},46);});
        settings.addView(label("Фоновий режим лише для поточного запиту, до 5 хвилин. Постійний індикатор Android дозволяє зупинити генерацію.",12));
        CheckBox avatarMotion=new CheckBox(this);avatarMotion.setText("Анімація аватара");avatarMotion.setChecked(prefs.getBoolean("avatar_motion",true));settings.addView(avatarMotion);avatarMotion.setOnCheckedChangeListener((b,value)->{prefs.edit().putBoolean("avatar_motion",value).apply();avatar.setMotion(value);if(welcomeAvatar!=null)welcomeAvatar.setMotion(value);});
        CheckBox englishVoice=new CheckBox(this);englishVoice.setText("English voice / dictation");englishVoice.setChecked(prefs.getBoolean("voice_english",false));settings.addView(englishVoice);englishVoice.setOnCheckedChangeListener((b,value)->prefs.edit().putBoolean("voice_english",value).apply());
        settings.addView(label("Диктування відкриває системний сервіс Android, який може використовувати мережу. Озвучування обирає встановлений локальний голос.",12));
        clear=new Button(this);clear.setText("Очистити історію");settings.addView(clear);
        clear.setOnClickListener(v->new AlertDialog.Builder(this).setTitle("Видалити локальну історію?").setNegativeButton("Скасувати",null).setPositiveButton("Видалити",(d,w)->{if(GenerationSession.active==null){history=new JSONArray();prefs.edit().remove("history").apply();memory.resetMood();render();busy(false);}}).show());
        scroll=new ScrollView(this);messages=new LinearLayout(this);messages.setOrientation(LinearLayout.VERTICAL);scroll.addView(messages);root.addView(scroll,new LinearLayout.LayoutParams(-1,0,1));
        composer=new LinearLayout(this);composer.setOrientation(LinearLayout.VERTICAL);root.addView(composer);input=new EditText(this);input.setHint("Напиши повідомлення…");input.setMinLines(1);input.setMaxLines(4);composer.addView(input);
        attachButton=new Button(this);attachButton.setText("＋ Файл");composer.addView(attachButton);attachButton.setOnClickListener(v->pickAttachment());
        HorizontalScrollView attachmentScroll=new HorizontalScrollView(this);attachmentRows=new LinearLayout(this);attachmentRows.setOrientation(LinearLayout.HORIZONTAL);attachmentScroll.addView(attachmentRows);composer.addView(attachmentScroll);
        LinearLayout buttons=new LinearLayout(this);send=new Button(this);send.setText("Надіслати");stop=new Button(this);stop.setText("Зупинити");buttons.addView(send,new LinearLayout.LayoutParams(0,-2,1));buttons.addView(stop,new LinearLayout.LayoutParams(0,-2,1));composer.addView(buttons);send.setBackgroundTintList(android.content.res.ColorStateList.valueOf(0xffa81920));retry=new Button(this);retry.setText("Повторити останній запит");composer.addView(retry);retry.setOnClickListener(v->retryLast());
        CheckBox voiceToggle=new CheckBox(this);voiceToggle.setText("Голосовий діалог");voiceToggle.setChecked(false);composer.addView(voiceToggle);
        voiceState=label("",12);composer.addView(voiceState);
        voiceToggle.setOnCheckedChangeListener((b,value)->{voiceMode=value;if(!value){voiceReplyJob=null;voice.stop();}updateVoiceState();});
        LinearLayout voiceRow=new LinearLayout(this);
        Button dictate=new Button(this);dictate.setText("Диктувати");dictate.setOnClickListener(v->dictate());
        Button speak=new Button(this);speak.setText("Озвучити");speak.setOnClickListener(v->{if(GenerationSession.active!=null)return;for(int i=history.length()-1;i>=0;i--){JSONObject m=history.optJSONObject(i);if(m!=null&&"assistant".equals(m.optString("role"))){status.setText(voice.speak(m.optString("content"),voiceLanguage()));return;}}status.setText("Немає відповіді");});
        Button silence=new Button(this);silence.setText("Стоп звук");silence.setOnClickListener(v->{voiceReplyJob=null;voice.stop();updateVoiceState();});
        for(Button b:new Button[]{dictate,speak,silence}){b.setTextSize(11);b.setMinWidth(0);b.setMinimumWidth(0);b.setPadding(0,0,0,0);voiceRow.addView(b,new LinearLayout.LayoutParams(0,dp(48),1));}composer.addView(voiceRow);
        LinearLayout nav=new LinearLayout(this);bottomNav=nav;nav.setOrientation(LinearLayout.HORIZONTAL);nav.setBackgroundColor(0xff242426);root.addView(nav);
        Button chat=new Button(this);chat.setText("Чат");chat.setOnClickListener(v->showSettings(false));
        for(Button btn:new Button[]{chat,memoryButton,hardwareButton,reminders,config}){btn.setTextSize(11);btn.setAllCaps(false);btn.setMinWidth(0);btn.setMinimumWidth(0);btn.setPadding(0,0,0,0);nav.addView(btn,new LinearLayout.LayoutParams(0,dp(52),1));}
        try{status.setText("Rust Core "+NativeCore.version()+" · "+base);ready=true;}catch(LinkageError e){status.setText("Rust Core недоступне: "+e.getMessage());}
        send.setOnClickListener(v->generate());stop.setOnClickListener(v->{voiceReplyJob=null;voice.stop();GenerationSession.cancel();syncSession();});
        setContentView(root);root.requestApplyInsets();render();busy(false);syncSession();MobileReminders.channel(this);
    }
    private void updateVoiceState(){
        if(voiceState==null||voice==null)return;
        CrimsonAvatar.State state=CrimsonAvatar.State.select(externalDictation,GenerationSession.active!=null,!GenerationSession.partial.isEmpty(),voice.speaking());
        avatar.setState(state);if(welcomeAvatar!=null)welcomeAvatar.setState(state);
        long elapsed=android.os.SystemClock.elapsedRealtime();
        if(elapsed>=nextAvatarMoodRefresh){nextAvatarMoodRefresh=elapsed+1000;avatarMood="Calm";avatarIntensity=0;
            try{JSONObject opts=memory.options(),mood=opts.optJSONObject("mood");
                if(opts.optBoolean("emotion_enabled",false)&&mood!=null){long now=System.currentTimeMillis()/1000;long age=Math.max(0,now-Math.min(now,Math.max(0,mood.optLong("updated"))));avatarIntensity=Math.max(0,Math.min(100,mood.optInt("intensity"))-(int)Math.min(100,age/60*5));avatarMood=avatarIntensity==0?"Calm":mood.optString("mood","Calm");}
            }catch(JSONException ignored){}
        }
        avatar.setMood(avatarMood,avatarIntensity);if(welcomeAvatar!=null)welcomeAvatar.setMood(avatarMood,avatarIntensity);
        voiceState.setVisibility(voiceMode?View.VISIBLE:View.GONE);
        if(voiceMode)voiceState.setText(state==CrimsonAvatar.State.IDLE?"Готова до репліки · диктуй, перевір текст, надішли":state.label);
    }
    private String voiceLanguage(){return prefs.getBoolean("voice_english",false)?"en-US":"uk-UA";}
    private void dictate(){
        if(GenerationSession.active!=null)return;
        new AlertDialog.Builder(this).setTitle("Системне диктування").setMessage("Android відкриє встановлений сервіс розпізнавання. Він може передавати аудіо через мережу, навіть якщо запитано офлайн-режим. Результат стане чернеткою, не буде надісланий автоматично.").setNegativeButton("Скасувати",null).setPositiveButton("Продовжити",(d,w)->{
            if(GenerationSession.active!=null)return;voice.stop();android.content.Intent i=new android.content.Intent(android.speech.RecognizerIntent.ACTION_RECOGNIZE_SPEECH);i.putExtra(android.speech.RecognizerIntent.EXTRA_LANGUAGE_MODEL,android.speech.RecognizerIntent.LANGUAGE_MODEL_FREE_FORM);i.putExtra(android.speech.RecognizerIntent.EXTRA_LANGUAGE,voiceLanguage());i.putExtra(android.speech.RecognizerIntent.EXTRA_PREFER_OFFLINE,true);i.putExtra(android.speech.RecognizerIntent.EXTRA_MAX_RESULTS,1);try{externalDictation=true;updateVoiceState();startActivityForResult(i,DICTATION);}catch(android.content.ActivityNotFoundException e){externalDictation=false;updateVoiceState();status.setText("Сервіс диктування недоступний");}
        }).show();
    }
    private void showSettings(boolean visible){if(visible){voiceReplyJob=null;voice.stop();}settingsScroll.setVisibility(visible?View.VISIBLE:View.GONE);scroll.setVisibility(visible?View.GONE:View.VISIBLE);composer.setVisibility(visible?View.GONE:View.VISIBLE);}
    private void showPersonality(){
        if(GenerationSession.active!=null)return;
        try{
            JSONObject opts=memory.options();String[] modes={"Companion","Assistant","Developer","Researcher"};
            String[] names={"Аналітичність","Емпатія","Допитливість","Гумор","Ініціативність","Емоційність","Сарказм"};
            int[] defaults={90,80,90,90,90,80,70},values=defaults.clone();
            JSONArray stored=opts.optJSONArray("traits");if(stored!=null&&stored.length()==7)for(int i=0;i<7;i++)values[i]=Math.max(0,Math.min(100,stored.optInt(i,defaults[i])));
            LinearLayout panel=new LinearLayout(this);panel.setOrientation(LinearLayout.VERTICAL);panel.setPadding(dp(16),dp(8),dp(16),dp(8));
            android.widget.Spinner mode=new android.widget.Spinner(this);
            android.widget.ArrayAdapter<String> adapter=new android.widget.ArrayAdapter<>(this,android.R.layout.simple_spinner_dropdown_item,modes);mode.setAdapter(adapter);mode.setSelection(Math.max(0,java.util.Arrays.asList(modes).indexOf(opts.optString("mode"))));panel.addView(mode);
            android.widget.SeekBar[] bars=new android.widget.SeekBar[7];
            for(int i=0;i<7;i++){final int index=i;TextView title=label(names[i]+" · "+values[i],14);panel.addView(title);
                android.widget.SeekBar bar=new android.widget.SeekBar(this);bar.setMax(100);bar.setProgress(values[i]);bars[i]=bar;panel.addView(bar);
                bar.setOnSeekBarChangeListener(new android.widget.SeekBar.OnSeekBarChangeListener(){
                    public void onProgressChanged(android.widget.SeekBar b,int value,boolean user){values[index]=value;title.setText(names[index]+" · "+value);}
                    public void onStartTrackingTouch(android.widget.SeekBar b){}
                    public void onStopTrackingTouch(android.widget.SeekBar b){}
                });
            }
            android.widget.CheckBox dynamic=new android.widget.CheckBox(this);dynamic.setText("Адаптивний стиль");dynamic.setChecked(opts.optBoolean("dynamic_style",true));panel.addView(dynamic);
            android.widget.CheckBox emotional=new android.widget.CheckBox(this);emotional.setText("Симуляція емоцій");emotional.setChecked(opts.optBoolean("emotion_enabled",false));panel.addView(emotional);
            JSONObject mood=opts.optJSONObject("mood");if(mood!=null&&opts.optBoolean("emotion_enabled",false))panel.addView(label("Останній збережений тон: "+mood.optString("mood","Calm")+" · "+mood.optInt("intensity",0)+"/100 (умовний стан)",12));
            Button reset=new Button(this);reset.setText("Відновити базові риси");panel.addView(reset);reset.setOnClickListener(v->{for(int i=0;i<7;i++)bars[i].setProgress(defaults[i]);});
            panel.addView(label("Риси визначають тон, а не права доступу. Адаптивний стиль враховує тип завдання, короткі продовження і дозволені уподобання без зміни базових рис. Налаштування використовуються у контексті вибраної моделі.",12));
            ScrollView scroll=new ScrollView(this);scroll.addView(panel);
            new AlertDialog.Builder(this).setTitle("Personality Engine 2.0").setView(scroll).setNegativeButton("Скасувати",null).setPositiveButton("Зберегти",(d,w)->{
                if(GenerationSession.active!=null)return;
                try{memory.savePersonality(modes[mode.getSelectedItemPosition()],values,dynamic.isChecked(),emotional.isChecked());status.setText("Особистість збережено");}catch(Exception e){status.setText(e.getMessage());}
            }).show();
        }catch(Exception e){status.setText("Не вдалося відкрити особистість: "+e.getMessage());}
    }
    private void receiveTransfer(String code){
        if(transfer!=null||receivedTransfer!=null||GenerationSession.active!=null)return;
        final String server=base,key=base.equals(bridgeEndpoint)?bridgeKey:"";
        if(key.isEmpty()||!code.matches("[0-9a-f]{32}")){transferStatus.setText("Збережи HTTPS-підключення до ПК та введи 32-символьний код");return;}
        new AlertDialog.Builder(this).setTitle("Отримати оригінал файлу?").setMessage("Запит до "+server+". Код одноразовий; файл перевіряється за SHA-256 перед збереженням.").setNegativeButton("Скасувати",null).setPositiveButton("Отримати",(d,w)->{
            if(transfer!=null||!base.equals(server)||!bridgeKey.equals(key))return;
            FileTransfer job=new FileTransfer();transfer=job;transferStatus.setText("Отримання…");
            new Thread(()->{FileTransfer.Received result=null;String error=null;try{result=job.receive(server,key,code,p->runOnUiThread(()->{if(!destroyed&&transfer==job&&!job.isCancelled())transferStatus.setText("Отримання "+p+"%");}));}catch(Exception e){error=e.getMessage();}
                final FileTransfer.Received file=result;final String failure=error;runOnUiThread(()->{if(destroyed||transfer!=job)return;transfer=null;
                    if(job.isCancelled()||!foreground){transferStatus.setText("Отримання скасовано");return;}
                    if(file==null||!base.equals(server)||!bridgeKey.equals(key)||!bridgeEndpoint.equals(server)){transferStatus.setText("Передавання не завершено: "+failure);return;}
                    receivedTransfer=file;transferStatus.setText("SHA-256 перевірено: "+file.name);
                    android.content.Intent i=new android.content.Intent(android.content.Intent.ACTION_CREATE_DOCUMENT);i.setType("application/octet-stream");i.addCategory(android.content.Intent.CATEGORY_OPENABLE);i.putExtra(android.content.Intent.EXTRA_TITLE,file.name);
                    try{startActivityForResult(i,SAVE_TRANSFER);}catch(android.content.ActivityNotFoundException e){receivedTransfer=null;transferStatus.setText("Збереження недоступне");}
                });
            },"hatoriel-file-transfer").start();
        }).show();
    }
    private void generate(){
        if(attaching||!attachments.items.isEmpty()){status.setText("Видали вкладення перед надсиланням: аналіз ще недоступний");return;}
        String query=input.getText().toString().trim();if(!ready||GenerationSession.active!=null||query.isEmpty())return;voice.stop();
        if(query.codePointCount(0,query.length())>12000){status.setText("Максимум 12000 символів");return;}
        try{append("user",query);}catch(JSONException e){status.setText(e.toString());return;}
        input.setText("");offerMemory(query);startRequest();
    }
    private void pickAttachment(){
        if(attaching||GenerationSession.active!=null||attachments.items.size()>=AttachmentDraft.MAX_COUNT)return;
        android.content.Intent i=new android.content.Intent(android.content.Intent.ACTION_OPEN_DOCUMENT);i.setType("*/*");i.addCategory(android.content.Intent.CATEGORY_OPENABLE);try{startActivityForResult(i,ATTACH);}catch(android.content.ActivityNotFoundException e){status.setText("Файловий менеджер недоступний");}
    }
    private void renderAttachments(){
        attachmentRows.removeAllViews();
        if(!attachments.items.isEmpty()){
            for(AttachmentDraft.Item item:attachments.items){LinearLayout tile=new LinearLayout(this);tile.setOrientation(LinearLayout.VERTICAL);tile.setPadding(dp(4),dp(4),dp(4),dp(4));attachmentRows.addView(tile,new LinearLayout.LayoutParams(dp(110),-2));
                if(item.preview!=null){ImageView image=new ImageView(this);image.setImageBitmap(item.preview);image.setScaleType(ImageView.ScaleType.FIT_CENTER);tile.addView(image,new LinearLayout.LayoutParams(-1,dp(56)));}
                TextView name=label(item.name,11);name.setMaxLines(1);name.setEllipsize(android.text.TextUtils.TruncateAt.END);tile.addView(name);tile.addView(label((item.bytes.length/1024)+" KiB",10));Button remove=new Button(this);remove.setText("Видалити");remove.setTextSize(10);tile.addView(remove);remove.setOnClickListener(v->{attachments.items.remove(item);renderAttachments();});}
            Button clearFiles=new Button(this);clearFiles.setText("Очистити");attachmentRows.addView(clearFiles);clearFiles.setOnClickListener(v->{attachments.items.clear();renderAttachments();});
        }
        if(!attachments.items.isEmpty())status.setText("Вкладення локальні. Видали їх, щоб надіслати текст.");busy(GenerationSession.active!=null);
    }
    private void importAttachment(android.net.Uri uri){
        if(attaching||GenerationSession.active!=null||attachments.items.size()>=AttachmentDraft.MAX_COUNT)return;attaching=true;busy(false);status.setText("Читання файлу…");final int limit=attachments.remaining();final android.content.ContentResolver resolver=getApplicationContext().getContentResolver();
        attachmentWorker=new Thread(()->{AttachmentDraft.Item item=null;String error=null;try{item=AttachmentDraft.read(resolver,uri,limit);}catch(Exception e){error=e.getMessage();}final AttachmentDraft.Item result=item;final String failure=error;runOnUiThread(()->{if(destroyed)return;attaching=false;if(result!=null&&attachments.add(result)){renderAttachments();}else{status.setText(failure==null?"Дублікат або перевищено ліміт":failure);busy(GenerationSession.active!=null);}});},"hatoriel-attachment");attachmentWorker.start();
    }
    private void retryLast(){
        if(!ready||GenerationSession.active!=null||history.length()==0)return;
        JSONObject last=history.optJSONObject(history.length()-1);
        if(last==null)return;
        if("user".equals(last.optString("role"))){startRequest();return;}
        if(history.length()<2||!"assistant".equals(last.optString("role")))return;
        JSONObject user=history.optJSONObject(history.length()-2);
        if(user==null||!"user".equals(user.optString("role")))return;
        new AlertDialog.Builder(this).setTitle("Повторити генерацію?").setMessage("Попередня відповідь буде замінена лише після успішної генерації.").setNegativeButton("Скасувати",null).setPositiveButton("Повторити",(d,w)->{if(GenerationSession.active!=null)return;startRequest(last.optString("content"));}).show();
    }
    private void startRequest(){startRequest(null);}
    private void startRequest(String replaceExpected){
        if(GenerationSession.active!=null)return;
        int end=history.length()-(replaceExpected==null?0:1);
        JSONArray context=new JSONArray();for(int i=Math.max(0,end-32);i<end;i++)context.put(history.opt(i));
        // Bound the JNI input while retaining the newest user message.
        while(context.length()>1&&context.toString().getBytes(java.nio.charset.StandardCharsets.UTF_8).length>240000)context.remove(0);
        final String data;try{data=memory.context(context);}catch(JSONException e){status.setText(e.toString());return;}final String server=base,selectedModel=model;
        boolean keep=prefs.getBoolean("background_generation",false);
        android.os.PowerManager power=getSystemService(android.os.PowerManager.class);
        if(power!=null&&power.isPowerSaveMode())keep=false;
        if(!MobileReminders.permitted(this))keep=false;
        try{
            voice.stop();voiceReplyJob=null;
            GenerationSession.start(this,data,server,selectedModel,server.equals(bridgeEndpoint)?bridgeKey:"",keep,replaceExpected);
            if(voiceMode&&foreground)voiceReplyJob=GenerationSession.job;
            if(keep){try{startForegroundService(new android.content.Intent(this,GenerationService.class).putExtra("job",GenerationSession.job));}catch(RuntimeException e){GenerationSession.background=false;GenerationSession.status="Фоновий сервіс недоступний; генерація лише у відкритому чаті";GenerationSession.revision++;}}
            syncSession();
        }catch(RuntimeException e){status.setText("Помилка: "+e.getMessage());}
    }
    private void syncSession(){
        updateVoiceState();if(sessionRevision==GenerationSession.revision)return;
        sessionRevision=GenerationSession.revision;
        ChatTransport previous=active;active=GenerationSession.active;
        draftText=GenerationSession.partial;
        if(previous!=active||active==null){
            try{history=new JSONArray(prefs.getString("history","[]"));}catch(JSONException ignored){}
            render();busy(active!=null);
            if(previous!=null&&active==null&&"Готово".equals(GenerationSession.status))send.performHapticFeedback(android.view.HapticFeedbackConstants.CLOCK_TICK);
        }else if(draftView!=null){draftView.setText(draftText);scroll.post(()->scroll.fullScroll(View.FOCUS_DOWN));}
        if(!GenerationSession.status.isEmpty())status.setText(GenerationSession.status);
        if(active==null&&voiceReplyJob!=null){boolean speak=voiceMode&&foreground&&voiceReplyJob.longValue()==GenerationSession.job&&"Готово".equals(GenerationSession.status)&&settingsScroll.getVisibility()!=View.VISIBLE;voiceReplyJob=null;if(speak){JSONObject last=history.optJSONObject(history.length()-1);if(last!=null&&"assistant".equals(last.optString("role")))status.setText(voice.speak(last.optString("content"),voiceLanguage()));}}updateVoiceState();
    }
    private void offerMemory(String query){
        if(!memory.suggestions())return;
        try{showMemoryDraft(NativeCore.suggestMemory(query));}catch(Exception|LinkageError e){status.setText("Помилка пропозиції спогаду");}
    }
    private void showMemoryDraft(String raw){
        if(!memory.suggestions())return;
        try{if(raw==null||"null".equals(raw))return;JSONObject m=new JSONObject(raw);
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
        Button notes=new Button(this);notes.setText("Запропонувати підсумок розмови");notes.setEnabled(memory.suggestions());panel.addView(notes);
        Button add=new Button(this);add.setText("Додати спогад");panel.addView(add);
        Button profile=new Button(this);profile.setText("Редагувати профіль");panel.addView(profile);
        Button transfer=new Button(this);transfer.setText("Захищене перенесення");panel.addView(transfer);
        JSONArray rows=memory.records();
        for(int i=rows.length()-1;i>=0;i--){JSONObject m=rows.optJSONObject(i);if(m==null)continue;TextView item=label(m.optString("content")+"\n"+m.optString("category")+" · "+m.optInt("importance")+"/100",14);panel.addView(item);
            Button remove=new Button(this);remove.setText("Видалити");panel.addView(remove);remove.setOnClickListener(v->new AlertDialog.Builder(this).setTitle("Видалити спогад?").setNegativeButton("Скасувати",null).setPositiveButton("Видалити",(d,w)->{memory.remove(m.optLong("id"));item.setVisibility(View.GONE);remove.setVisibility(View.GONE);}).show());
        }
        AlertDialog dialog=new AlertDialog.Builder(this).setTitle("Пам’ять Hatoriel").setView(sv).setPositiveButton("Закрити",null).create();
        suggest.setOnCheckedChangeListener((b,checked)->{memory.setSuggestions(checked);notes.setEnabled(checked);});
        notes.setOnClickListener(v->{dialog.dismiss();if(!memory.suggestions())return;try{JSONArray recent=new JSONArray();for(int i=Math.max(0,history.length()-64);i<history.length();i++)recent.put(history.opt(i));String raw=NativeCore.suggestSummary(recent.toString());if(raw==null||"null".equals(raw))status.setText("Недостатньо явних фактів для підсумку");else showMemoryDraft(raw);}catch(Exception|LinkageError e){status.setText("Помилка підсумку");}});
        add.setOnClickListener(v->{dialog.dismiss();addMemory();});profile.setOnClickListener(v->{dialog.dismiss();editProfile();});transfer.setOnClickListener(v->{dialog.dismiss();showTransfer();});dialog.show();
    }
    private void showTransfer(){
        if(GenerationSession.active!=null)return;
        LinearLayout panel=new LinearLayout(this);panel.setOrientation(LinearLayout.VERTICAL);panel.setPadding(dp(20),dp(8),dp(20),dp(8));
        ScrollView scroll=new ScrollView(this);scroll.addView(panel);
        panel.addView(label("Перенесення пам’яті, профілю та особистості. Пароль передавай окремо.",14));
        EditText pass=new EditText(this);pass.setHint("Парольна фраза 12+ символів");pass.setInputType(129);pass.setSaveEnabled(false);panel.addView(pass);
        EditText pack=new EditText(this);pack.setHint("Зашифрований пакет");pack.setMinLines(3);pack.setMaxLines(6);pack.setSaveEnabled(false);panel.addView(pack);
        Button seal=new Button(this);seal.setText("Створити пакет");panel.addView(seal);
        Button open=new Button(this);open.setText("Переглянути імпорт");panel.addView(open);
        TextView report=label("",14);panel.addView(report);
        AlertDialog dialog=new AlertDialog.Builder(this).setTitle("Перенесення Hatoriel").setView(scroll).setPositiveButton("Закрити",null).create();
        java.util.function.Consumer<Boolean> execute=export->{
            try{
                JSONObject local=memory.snapshot();String baseline=local.toString(),secret=pass.getText().toString(),encoded=pack.getText().toString();
                if(encoded.length()>2097152)throw new IllegalArgumentException("Пакет завеликий");
                JSONObject command=new JSONObject().put("action",export?"seal":"preview").put("password",secret);
                if(export)command.put("snapshot",local);else command.put("local",local).put("package",encoded);
                pass.setText("");seal.setEnabled(false);open.setEnabled(false);report.setText("Обробка…");
                new Thread(()->{
                    try{
                        JSONObject result=new JSONObject(NativeCore.sync(command.toString()));
                        if(result.has("error"))throw new IllegalStateException(result.getString("error"));
                        runOnUiThread(()->{
                            if(!dialog.isShowing()||isFinishing()||isDestroyed())return;
                            seal.setEnabled(true);open.setEnabled(true);
                            if(export){pack.setText(result.optString("package"));report.setText("Пакет створено");return;}
                            JSONArray additions=result.optJSONArray("additions");
                            if(additions==null){report.setText("Некоректний результат перегляду");return;}
                            String summary="Нові: "+(additions==null?0:additions.length())+"; дублікати: "+result.optInt("duplicates")+"; конфлікти: "+result.optInt("conflicts");
                            LinearLayout review=new LinearLayout(this);review.setOrientation(LinearLayout.VERTICAL);review.setPadding(dp(20),0,dp(20),0);review.addView(label(summary,14));
                            review.addView(label("Нові записи (локальні конфлікти не змінюються):",14));
                            for(int i=0;i<additions.length();i++){
                                JSONObject row=additions.optJSONObject(i);if(row==null)continue;
                                TextView item=label(row.optString("category")+" · "+row.optInt("importance")+"/100\n"+row.optString("content"),14);
                                item.setTextIsSelectable(true);review.addView(item);
                            }
                            JSONObject incomingProfile=result.optJSONObject("profile"), incomingOptions=result.optJSONObject("options");
                            if(incomingProfile!=null)review.addView(label("Вхідний профіль:\nІнтереси: "+incomingProfile.optString("interests")+"\nСтиль: "+incomingProfile.optString("communication")+"\nЦілі: "+incomingProfile.optString("goals"),14));
                            if(incomingOptions!=null)review.addView(label("Вхідна особистість: "+incomingOptions.optString("mode")+"\nРиси: "+incomingOptions.optString("traits")+"\nАдаптивний стиль: "+incomingOptions.optBoolean("dynamic_style")+"\nЕмоції: "+incomingOptions.optBoolean("emotion_enabled"),14));
                            CheckBox profile=new CheckBox(this);profile.setText("Замінити профіль");review.addView(profile);
                            CheckBox personality=new CheckBox(this);personality.setText("Замінити особистість");review.addView(personality);
                            ScrollView reviewScroll=new ScrollView(this);reviewScroll.addView(review);
                            new AlertDialog.Builder(this).setTitle("Підтвердити імпорт?").setView(reviewScroll).setNegativeButton("Скасувати",null).setPositiveButton("Об’єднати",(d,w)->{
                                try{
                                    if(!dialog.isShowing()||isFinishing()||isDestroyed()||GenerationSession.active!=null||!baseline.equals(memory.snapshot().toString())||!encoded.equals(pack.getText().toString()))throw new IllegalStateException("Дані змінилися. Повтори перегляд.");
                                    memory.applySync(result,profile.isChecked(),personality.isChecked());report.setText("Імпортовано; передачу пам’яті серверу вимкнено.");
                                }catch(Exception e){report.setText(e.getMessage());}
                            }).show();
                        });
                    }catch(Exception|LinkageError e){runOnUiThread(()->{if(dialog.isShowing()){seal.setEnabled(true);open.setEnabled(true);report.setText("Помилка: "+e.getMessage());}});}
                },"hatoriel-sync").start();
            }catch(Exception e){report.setText(e.getMessage());}
        };
        seal.setOnClickListener(v->execute.accept(true));open.setOnClickListener(v->execute.accept(false));
        dialog.setOnDismissListener(d->pass.setText(""));dialog.show();
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
    private String gib(long bytes){return String.format(java.util.Locale.ROOT,"%.2f GiB",bytes/1073741824.0);}
    private void showHardware(){
        if(active!=null||importing)return;
        ScrollView sv=new ScrollView(this);LinearLayout panel=new LinearLayout(this);panel.setOrientation(LinearLayout.VERTICAL);panel.setPadding(dp(20),dp(8),dp(20),dp(8));sv.addView(panel);
        TextView report=label("",14);panel.addView(report);
        panel.addView(label("Профіль майбутнього локального runtime",14));
        Spinner mode=new Spinner(this);String[] profiles={"Eco","Balanced","Performance"};
        mode.setAdapter(new ArrayAdapter<String>(this,android.R.layout.simple_spinner_dropdown_item,profiles));
        String saved=prefs.getString("runtime_profile","Balanced");mode.setSelection("Eco".equals(saved)?0:"Performance".equals(saved)?2:1);panel.addView(mode);
        Runnable refresh=()->{try{
            DeviceProfile d=DeviceProfile.read(this);String profile=String.valueOf(mode.getSelectedItem());
            java.io.File file=ModelImport.selected(this);long size=file.isFile()?file.length():0;
            report.setText(d.device+"\nSoC: "+d.soc+"\nCPU: "+d.cores+" доступних логічних ядер\nARM64: "+(d.arm64?"так":"ні")+"\nRAM: "+gib(d.totalRam)+" · доступно "+gib(d.availableRam)+"\nВільне сховище: "+gib(d.storage)+"\nЕнергозбереження: "+(d.powerSave?"увімкнено":"вимкнено")+"\nТермальний статус: "+(d.thermal<0?"невідомий":String.valueOf(d.thermal))+"\n\n"+d.recommendation()+"\n\nGGUF: "+(size>0?gib(size):"немає")+"\nОрієнтовна RAM: "+(size>0?gib(DeviceProfile.estimate(size,DeviceProfile.context(profile))):"—")+"\nБюджет RAM: "+gib(d.budget())+"\n"+d.warning(size,profile)+"\n\nПотоки: "+DeviceProfile.threads(d.cores,profile)+" · контекст: "+DeviceProfile.context(profile)+"\nПрофіль поки не керує генерацією.");
        }catch(Exception e){report.setText("Діагностика недоступна: "+e.getMessage());}};
        mode.setOnItemSelectedListener(new android.widget.AdapterView.OnItemSelectedListener(){public void onItemSelected(android.widget.AdapterView<?> p,View v,int pos,long id){prefs.edit().putString("runtime_profile",profiles[pos]).apply();refresh.run();}public void onNothingSelected(android.widget.AdapterView<?> p){}});
        Button scan=new Button(this);scan.setText("Оновити діагностику");panel.addView(scan);scan.setOnClickListener(v->refresh.run());
        Button remove=new Button(this);remove.setText("Видалити імпортовану GGUF");panel.addView(remove);
        remove.setOnClickListener(v->new AlertDialog.Builder(this).setTitle("Видалити локальну модель?").setMessage("Видаляється лише копія в Hatoriel. Початковий файл не змінюється.").setNegativeButton("Скасувати",null).setPositiveButton("Видалити",(d,w)->{if(importing||active!=null)return;try{if(!ModelImport.remove(this))throw new java.io.IOException("Не вдалося видалити");refreshLocalModel();refresh.run();}catch(Exception e){status.setText(e.getMessage());}}).show());
        refresh.run();new AlertDialog.Builder(this).setTitle("Hardware & Model Manager").setView(sv).setPositiveButton("Закрити",null).show();
    }
    @Override protected void onResume(){super.onResume();foreground=true;syncSession();sessionHandler.removeCallbacks(sessionWatch);sessionHandler.post(sessionWatch);MobileReminders.channel(this);MobileReminders.schedule(this);}
    @Override protected void onPause(){foreground=false;if(transfer!=null)transfer.cancel();voiceReplyJob=null;sessionHandler.removeCallbacks(sessionWatch);if(voice!=null)voice.stop();super.onPause();}
    @Override protected void onStop(){if(!GenerationSession.background&&!isChangingConfigurations())GenerationSession.cancel();super.onStop();}
    @Override public void onRequestPermissionsResult(int request,String[] permissions,int[] results){super.onRequestPermissionsResult(request,permissions,results);if(request==45){MobileReminders.schedule(this);if(!MobileReminders.permitted(this))Toast.makeText(this,"Сповіщення не дозволено",Toast.LENGTH_LONG).show();}}
    private void showReminders(){
        ScrollView sv=new ScrollView(this);LinearLayout panel=new LinearLayout(this);panel.setOrientation(LinearLayout.VERTICAL);panel.setPadding(dp(20),dp(8),dp(20),dp(8));sv.addView(panel);
        CheckBox enabled=new CheckBox(this);enabled.setText("Дозволити нагадування");enabled.setChecked(MobileReminders.enabled(this));panel.addView(enabled);
        CheckBox quiet=new CheckBox(this);quiet.setText("Не турбувати");quiet.setChecked(MobileReminders.quiet(this));panel.addView(quiet);
        enabled.setOnCheckedChangeListener((b,value)->{MobileReminders.prefs(this).edit().putBoolean("enabled",value).commit();if(value&&android.os.Build.VERSION.SDK_INT>=33&&!MobileReminders.permitted(this))requestPermissions(new String[]{android.Manifest.permission.POST_NOTIFICATIONS},45);MobileReminders.schedule(this);});
        quiet.setOnCheckedChangeListener((b,value)->{MobileReminders.prefs(this).edit().putBoolean("quiet",value).commit();MobileReminders.schedule(this);});
        CheckBox preview=new CheckBox(this);preview.setText("Показувати текст у сповіщенні");preview.setChecked(MobileReminders.prefs(this).getBoolean("show_text",false));panel.addView(preview);
        preview.setOnCheckedChangeListener((b,value)->{if(!value){MobileReminders.prefs(this).edit().putBoolean("show_text",false).commit();return;}new AlertDialog.Builder(this).setTitle("Показувати текст?").setMessage("Текст може бути видимим на екрані блокування.").setNegativeButton("Ні",(d,w)->preview.setChecked(false)).setPositiveButton("Дозволити",(d,w)->MobileReminders.prefs(this).edit().putBoolean("show_text",true).commit()).setOnCancelListener(d->preview.setChecked(false)).show();});
        Button system=new Button(this);system.setText("Системні налаштування звуку та вібрації");panel.addView(system);system.setOnClickListener(v->{android.content.Intent i=new android.content.Intent(android.provider.Settings.ACTION_CHANNEL_NOTIFICATION_SETTINGS).putExtra(android.provider.Settings.EXTRA_APP_PACKAGE,getPackageName()).putExtra(android.provider.Settings.EXTRA_CHANNEL_ID,MobileReminders.CHANNEL);try{startActivity(i);}catch(android.content.ActivityNotFoundException e){Toast.makeText(this,"Налаштування недоступні",Toast.LENGTH_SHORT).show();}});
        panel.addView(label("Нагадування приблизні: Android може затримувати їх. У режимі тиші очікують до його вимкнення.",12));
        EditText text=new EditText(this);text.setHint("Про що нагадати?");panel.addView(text);
        EditText minutes=field("30","Через скільки хвилин");minutes.setInputType(android.text.InputType.TYPE_CLASS_NUMBER);panel.addView(minutes);
        Button add=new Button(this);add.setText("Додати нагадування");panel.addView(add);
        LinearLayout list=new LinearLayout(this);list.setOrientation(LinearLayout.VERTICAL);panel.addView(list);
        Runnable refresh=()->{list.removeAllViews();JSONArray rows=MobileReminders.rows(this);for(int j=rows.length()-1;j>=0;j--){JSONObject row=rows.optJSONObject(j);if(row==null)continue;LinearLayout item=new LinearLayout(this);item.setOrientation(LinearLayout.VERTICAL);list.addView(item);
            String date=java.text.DateFormat.getDateTimeInstance().format(new java.util.Date(row.optLong("due")));
            item.addView(label(row.optString("text")+"\n"+date+" · "+(row.optBoolean("delivered")?"Сповіщено":"Очікує"),14));
            Button snooze=new Button(this);snooze.setText("Відкласти на 15 хв");item.addView(snooze);snooze.setOnClickListener(v->{try{MobileReminders.snooze(this,row.optInt("id"),15);snooze.setEnabled(false);item.addView(label("Відкладено на 15 хв. Відкрий список повторно для оновлення.",12));}catch(Exception e){Toast.makeText(this,e.getMessage(),Toast.LENGTH_SHORT).show();}});
            Button remove=new Button(this);remove.setText("Видалити");item.addView(remove);remove.setOnClickListener(v->{MobileReminders.remove(this,row.optInt("id"));list.removeView(item);});
        }};
        add.setOnClickListener(v->{try{MobileReminders.add(this,text.getText().toString(),Integer.parseInt(minutes.getText().toString()));text.setText("");refresh.run();}catch(Exception e){text.setError(e.getMessage());}});
        refresh.run();new AlertDialog.Builder(this).setTitle("Нагадування Hatoriel").setView(sv).setPositiveButton("Закрити",null).show();
    }
    private void refreshLocalModel(){try{java.io.File file=ModelImport.selected(this);localModelStatus.setText(file.isFile()?String.format(java.util.Locale.ROOT,"GGUF: %.2f GiB · імпортовано, запуск ще недоступний",file.length()/1073741824.0):"GGUF не імпортовано");}catch(java.io.IOException e){localModelStatus.setText(e.getMessage());}}
    @Override protected void onActivityResult(int request,int result,android.content.Intent data){super.onActivityResult(request,result,data);if(request==SAVE_TRANSFER){FileTransfer.Received file=receivedTransfer;receivedTransfer=null;if(result==RESULT_OK&&file!=null&&data!=null&&data.getData()!=null){android.net.Uri dest=data.getData();new Thread(()->{String message;try(java.io.OutputStream out=getContentResolver().openOutputStream(dest,"wt")){if(out==null)throw new java.io.IOException("Cannot open destination");out.write(file.bytes);message="Файл записано";}catch(Exception e){message="Помилка збереження; можливий неповний файл: "+e.getMessage();}final String m=message;runOnUiThread(()->{if(!destroyed)transferStatus.setText(m);});},"hatoriel-save-transfer").start();}else transferStatus.setText("Збереження скасовано");return;}if(request==ATTACH){if(result==RESULT_OK&&data!=null&&data.getData()!=null)importAttachment(data.getData());return;}if(request==DICTATION){externalDictation=false;updateVoiceState();if(result==RESULT_OK&&data!=null&&GenerationSession.active==null){java.util.ArrayList<String> values=data.getStringArrayListExtra(android.speech.RecognizerIntent.EXTRA_RESULTS);if(values!=null&&!values.isEmpty()&&values.get(0)!=null){String old=input.getText().toString(),next=old.isEmpty()?values.get(0):old+" "+values.get(0);int n=next.codePointCount(0,next.length());if(n>12000)next=next.substring(0,next.offsetByCodePoints(0,12000));input.setText(next);input.setSelection(next.length());status.setText("Чернетка готова");}}return;}if(request!=71||result!=RESULT_OK||data==null||data.getData()==null||importing)return;android.net.Uri uri=data.getData();importing=true;importModel.setEnabled(false);hardwareButton.setEnabled(false);localModelStatus.setText("Імпорт GGUF…");android.content.Context app=getApplicationContext();importWorker=new Thread(()->{String failure=null;try{ModelImport.copy(app,uri);}catch(Exception e){failure=e.getMessage();}final String error=failure;runOnUiThread(()->{if(destroyed)return;importing=false;importModel.setEnabled(active==null);hardwareButton.setEnabled(active==null);if(error==null)refreshLocalModel();else localModelStatus.setText("Імпорт: "+error);});},"hatoriel-model-import");importWorker.start();}
    @Override protected void onDestroy(){destroyed=true;if(transfer!=null)transfer.cancel();transfer=null;receivedTransfer=null;if(attachmentWorker!=null)attachmentWorker.interrupt();attachments.items.clear();if(voice!=null)voice.close();if(importWorker!=null)importWorker.interrupt();sessionHandler.removeCallbacks(sessionWatch);if(isFinishing()&&!GenerationSession.background)GenerationSession.cancel();super.onDestroy();}
}
