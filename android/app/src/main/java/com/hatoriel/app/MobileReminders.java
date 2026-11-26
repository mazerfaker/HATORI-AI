package com.hatoriel.app;

import android.Manifest;
import android.app.*;
import android.content.*;
import android.content.pm.PackageManager;
import android.os.Build;
import org.json.*;

final class MobileReminders {
    static final String CHANNEL="hatoriel_reminders_v1";
    static final String ACTION="com.hatoriel.app.REMINDER_DUE";
    static SharedPreferences prefs(Context c){return c.getSharedPreferences("hatoriel_reminders",Context.MODE_PRIVATE);}
    static JSONArray rows(Context c){try{return new JSONArray(prefs(c).getString("rows","[]"));}catch(JSONException e){return new JSONArray();}}
    static boolean enabled(Context c){return prefs(c).getBoolean("enabled",false);}
    static boolean quiet(Context c){return prefs(c).getBoolean("quiet",false);}
    static boolean permitted(Context c){
        NotificationManager nm=c.getSystemService(NotificationManager.class);
        return nm!=null&&nm.areNotificationsEnabled()&&(Build.VERSION.SDK_INT<33||c.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS)==PackageManager.PERMISSION_GRANTED);
    }
    static void channel(Context c){
        NotificationManager nm=c.getSystemService(NotificationManager.class);
        NotificationChannel ch=new NotificationChannel(CHANNEL,"Нагадування Hatoriel",NotificationManager.IMPORTANCE_DEFAULT);
        ch.setDescription("Локальні нагадування");ch.enableVibration(true);nm.createNotificationChannel(ch);
    }
    private static PendingIntent alarm(Context c){
        return PendingIntent.getBroadcast(c,451,new Intent(c,ReminderReceiver.class).setAction(ACTION),PendingIntent.FLAG_UPDATE_CURRENT|PendingIntent.FLAG_IMMUTABLE);
    }
    static synchronized void schedule(Context c){
        AlarmManager am=c.getSystemService(AlarmManager.class);if(am==null)return;
        PendingIntent pi=alarm(c);am.cancel(pi);
        if(!enabled(c)||quiet(c)||!permitted(c))return;
        NotificationChannel ch=c.getSystemService(NotificationManager.class).getNotificationChannel(CHANNEL);
        if(ch!=null&&ch.getImportance()==NotificationManager.IMPORTANCE_NONE)return;
        long next=Long.MAX_VALUE;JSONArray rows=rows(c);
        for(int i=0;i<rows.length();i++){JSONObject row=rows.optJSONObject(i);if(row!=null&&!row.optBoolean("delivered"))next=Math.min(next,row.optLong("due",Long.MAX_VALUE));}
        if(next!=Long.MAX_VALUE)am.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP,Math.max(System.currentTimeMillis()+1000,next),pi);
    }
    static synchronized void add(Context c,String content,int minutes)throws JSONException{
        String s=content.trim();if(s.isEmpty()||s.codePointCount(0,s.length())>500||minutes<1||minutes>10080)throw new IllegalArgumentException("Текст 1–500 символів; час 1–10080 хв");
        JSONArray rows=rows(c);if(rows.length()>=100)throw new IllegalArgumentException("Ліміт 100 нагадувань");
        int id=prefs(c).getInt("next_id",1);if(id==Integer.MAX_VALUE)throw new IllegalStateException("Ліміт ID");
        rows.put(new JSONObject().put("id",id).put("text",s).put("due",System.currentTimeMillis()+minutes*60000L).put("delivered",false));
        if(!prefs(c).edit().putString("rows",rows.toString()).putInt("next_id",id+1).commit())throw new IllegalStateException("Не вдалося зберегти");
        schedule(c);
    }
    static synchronized void remove(Context c,int id){
        JSONArray rows=rows(c);for(int i=0;i<rows.length();i++){JSONObject row=rows.optJSONObject(i);if(row!=null&&row.optInt("id")==id){rows.remove(i);break;}}
        prefs(c).edit().putString("rows",rows.toString()).commit();c.getSystemService(NotificationManager.class).cancel(id);schedule(c);
    }
    static synchronized void deliver(Context c){
        channel(c);if(!enabled(c)||quiet(c)||!permitted(c))return;
        NotificationManager nm=c.getSystemService(NotificationManager.class);
        NotificationChannel ch=nm.getNotificationChannel(CHANNEL);if(ch==null||ch.getImportance()==NotificationManager.IMPORTANCE_NONE)return;
        JSONArray rows=rows(c);long now=System.currentTimeMillis();
        for(int i=0;i<rows.length();i++){
            JSONObject row=rows.optJSONObject(i);if(row==null||row.optBoolean("delivered")||row.optLong("due",Long.MAX_VALUE)>now)continue;
            int id=row.optInt("id");Intent open=new Intent(c,MainActivity.class).setFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP|Intent.FLAG_ACTIVITY_SINGLE_TOP);
            PendingIntent content=PendingIntent.getActivity(c,id,open,PendingIntent.FLAG_UPDATE_CURRENT|PendingIntent.FLAG_IMMUTABLE);
            Notification n=new Notification.Builder(c,CHANNEL).setSmallIcon(android.R.drawable.ic_dialog_info).setContentTitle("Hatoriel · Нагадування").setContentText(row.optString("text")).setStyle(new Notification.BigTextStyle().bigText(row.optString("text"))).setVisibility(Notification.VISIBILITY_PRIVATE).setContentIntent(content).setAutoCancel(true).build();
            try{nm.notify(id,n);row.put("delivered",true);}catch(SecurityException|JSONException e){break;}
        }
        prefs(c).edit().putString("rows",rows.toString()).commit();schedule(c);
    }
}
