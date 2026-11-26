package com.hatoriel.app;
import android.content.*;
public final class ReminderReceiver extends BroadcastReceiver {
    @Override public void onReceive(Context context,Intent intent){
        if(MobileReminders.ACTION.equals(intent.getAction()))MobileReminders.deliver(context);
        else if(Intent.ACTION_BOOT_COMPLETED.equals(intent.getAction())||Intent.ACTION_MY_PACKAGE_REPLACED.equals(intent.getAction())){
            MobileReminders.channel(context);MobileReminders.schedule(context);
        }
    }
}
