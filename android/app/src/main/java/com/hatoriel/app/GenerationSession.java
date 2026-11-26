package com.hatoriel.app;

import android.content.Context;
import android.content.SharedPreferences;
import android.os.Handler;
import android.os.Looper;
import android.os.SystemClock;
import org.json.*;

// Main-thread owner; network work executes on one worker. No Activity references.
final class GenerationSession {
    private static final Handler MAIN=new Handler(Looper.getMainLooper());
    static ChatTransport active;
    static String partial="",status="";
    static boolean background;
    static long revision,job;
    static void start(Context context,String data,String server,String model,String sessionKey,boolean keepRunning){
        if(active!=null)throw new IllegalStateException("Generation already active");
        Context app=context.getApplicationContext();
        ChatTransport transport=new ChatTransport();active=transport;background=keepRunning;
        long id=++job;partial="";status="Hatoriel думає…";revision++;
        MAIN.postDelayed(()->{if(active==transport){status="Ліміт часу генерації";transport.cancel();revision++;}},300000);
        new Thread(()->{
            String reply=null,error=null;
            try{
                String prepared=NativeCore.prepareMemory(data);
                if(prepared==null||!prepared.startsWith("["))throw new IllegalStateException("Core: "+prepared);
                long[] last={0};
                reply=transport.request(server,model,new JSONArray(prepared),sessionKey,text->{
                    long now=SystemClock.elapsedRealtime();
                    if(now-last[0]<100)return;last[0]=now;
                    MAIN.post(()->{if(active==transport&&!transport.isCancelled()){partial=text;revision++;}});
                });
            }catch(Exception|LinkageError e){error=e.getMessage()==null?e.toString():e.getMessage();}
            final String result=reply,failure=error;
            MAIN.post(()->{
                if(active!=transport||job!=id)return;
                if(transport.isCancelled())status="Зупинено";
                else if(failure!=null)status="Помилка: "+failure;
                else try{
                    SharedPreferences prefs=app.getSharedPreferences("hatoriel_chat",Context.MODE_PRIVATE);
                    JSONArray rows=new JSONArray(prefs.getString("history","[]"));
                    rows.put(new JSONObject().put("role","assistant").put("content",result));
                    while(rows.length()>100)rows.remove(0);
                    if(!prefs.edit().putString("history",rows.toString()).commit())throw new IllegalStateException("Не вдалося зберегти відповідь");
                    status="Готово";
                }catch(Exception e){status="Помилка збереження: "+e.getMessage();}
                active=null;background=false;partial="";revision++;
            });
        },"hatoriel-inference").start();
    }
    static void cancel(){if(active!=null){active.cancel();status="Зупинення…";revision++;}}
}
