package com.hatoriel.app;

import android.app.*;
import android.content.*;
import android.content.pm.ServiceInfo;
import android.os.*;

public final class GenerationService extends Service {
    private static final String CHANNEL="hatoriel_generation_v1";
    private static final int NOTIFICATION=2000000001;
    static final String STOP="com.hatoriel.app.STOP_GENERATION";
    private final Handler handler=new Handler(Looper.getMainLooper());
    private long owner=-1;
    private final Runnable check=new Runnable(){public void run(){
        if(GenerationSession.active==null||!GenerationSession.background||GenerationSession.job!=owner){stopSelf();return;}
        handler.postDelayed(this,500);
    }};
    @Override public IBinder onBind(Intent i){return null;}
    @Override public int onStartCommand(Intent intent,int flags,int startId){
        if(intent!=null&&STOP.equals(intent.getAction())){if(intent.getLongExtra("job",-1)==GenerationSession.job){GenerationSession.cancel();stopSelf();}else if(owner<0){stopSelf();}return START_NOT_STICKY;}
        NotificationManager nm=getSystemService(NotificationManager.class);
        nm.createNotificationChannel(new NotificationChannel(CHANNEL,"Генерація Hatoriel",NotificationManager.IMPORTANCE_LOW));
        PendingIntent open=PendingIntent.getActivity(this,601,new Intent(this,MainActivity.class).setFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP|Intent.FLAG_ACTIVITY_SINGLE_TOP),PendingIntent.FLAG_IMMUTABLE|PendingIntent.FLAG_UPDATE_CURRENT);
        PendingIntent stop=PendingIntent.getService(this,602,new Intent(this,GenerationService.class).setAction(STOP).setData(android.net.Uri.parse("hatoriel://generation/"+GenerationSession.job)).putExtra("job",GenerationSession.job),PendingIntent.FLAG_IMMUTABLE|PendingIntent.FLAG_UPDATE_CURRENT);
        Notification n=new Notification.Builder(this,CHANNEL).setSmallIcon(android.R.drawable.ic_dialog_info).setContentTitle("Hatoriel").setContentText("Триває обробка запиту").setOngoing(true).setVisibility(Notification.VISIBILITY_PRIVATE).setContentIntent(open).addAction(new Notification.Action.Builder(null,"Зупинити",stop).build()).build();
        try{
            if(Build.VERSION.SDK_INT>=29)startForeground(NOTIFICATION,n,ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC);
            else startForeground(NOTIFICATION,n);
        }catch(RuntimeException e){GenerationSession.background=false;GenerationSession.revision++;stopSelf();return START_NOT_STICKY;}
        owner=intent==null?-1:intent.getLongExtra("job",-1);handler.removeCallbacks(check);handler.post(check);return START_NOT_STICKY;
    }
    @Override public void onTimeout(int startId,int fgsType){if(GenerationSession.job==owner)GenerationSession.cancel();stopSelf(startId);}
    @Override public void onDestroy(){handler.removeCallbacks(check);if(GenerationSession.job==owner&&GenerationSession.background&&GenerationSession.active!=null)GenerationSession.cancel();stopForeground(STOP_FOREGROUND_REMOVE);super.onDestroy();}
}
