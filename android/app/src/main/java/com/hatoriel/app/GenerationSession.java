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
    static long revision,job=new java.security.SecureRandom().nextLong() & Long.MAX_VALUE;
    static void start(Context context,String data,String server,String model,String sessionKey,boolean keepRunning,String replaceExpected){
        if(active!=null)throw new IllegalStateException("Generation already active");
        Context app=context.getApplicationContext();
        ChatTransport transport=new ChatTransport();active=transport;background=keepRunning;
        long id=++job;partial="";status="Hatoriel думає…";revision++;
        final Runnable deadline=()->{if(active==transport){status="Ліміт часу генерації";transport.cancel();revision++;}};
        MAIN.postDelayed(deadline,300000);
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
                MAIN.removeCallbacks(deadline);
                if(active!=transport||job!=id)return;
                if(transport.isCancelled()){if(!"Ліміт часу генерації".equals(status))status="Зупинено";}
                else if(failure!=null)status="Помилка: "+failure;
                else try{
                    SharedPreferences prefs=app.getSharedPreferences("hatoriel_chat",Context.MODE_PRIVATE);
                    JSONArray rows=new JSONArray(prefs.getString("history","[]"));
                    if(replaceExpected!=null){JSONObject last=rows.optJSONObject(rows.length()-1);if(last==null||!"assistant".equals(last.optString("role"))||!replaceExpected.equals(last.optString("content")))throw new IllegalStateException("Історія змінилася; попередню відповідь збережено");rows.remove(rows.length()-1);}
                    rows.put(new JSONObject().put("role","assistant").put("content",result));
                    while(rows.length()>100)rows.remove(0);
                    if(!prefs.edit().putString("history",rows.toString()).commit())throw new IllegalStateException("Не вдалося зберегти відповідь");
                    status="Готово";
                }catch(Exception e){status="Помилка збереження: "+e.getMessage();}
                active=null;background=false;partial="";revision++;
            });
        },"hatoriel-inference").start();
    }
    static void cancel(){if(active!=null&&!active.isCancelled()){active.cancel();status="Зупинення…";revision++;}}
}
