package com.hatoriel.app;

import android.app.ActivityManager;
import android.content.Context;
import android.os.Build;
import android.os.PowerManager;
import android.os.StatFs;

final class DeviceProfile {
    static final long MIB=1024L*1024L;
    static final long GIB=1024L*MIB;
    final long totalRam,availableRam,storage;
    final int cores,thermal;
    final boolean lowMemory,powerSave,arm64;
    final String device,soc;
    private DeviceProfile(long total,long available,long storage,int cores,int thermal,boolean low,boolean save,boolean arm64,String device,String soc){
        totalRam=total;availableRam=available;this.storage=storage;this.cores=cores;this.thermal=thermal;
        lowMemory=low;powerSave=save;this.arm64=arm64;this.device=device;this.soc=soc;
    }
    static DeviceProfile read(Context c){
        ActivityManager manager=(ActivityManager)c.getSystemService(Context.ACTIVITY_SERVICE);
        ActivityManager.MemoryInfo m=new ActivityManager.MemoryInfo();manager.getMemoryInfo(m);
        PowerManager power=(PowerManager)c.getSystemService(Context.POWER_SERVICE);
        int thermal=Build.VERSION.SDK_INT>=29?power.getCurrentThermalStatus():-1;
        boolean arm64=false;for(String abi:Build.SUPPORTED_ABIS){if("arm64-v8a".equals(abi))arm64=true;}
        String soc=Build.VERSION.SDK_INT>=31?Build.SOC_MANUFACTURER+" "+Build.SOC_MODEL:Build.HARDWARE;
        return new DeviceProfile(m.totalMem,m.availMem,new StatFs(c.getFilesDir().getAbsolutePath()).getAvailableBytes(),
            Runtime.getRuntime().availableProcessors(),thermal,m.lowMemory,power.isPowerSaveMode(),arm64,Build.MANUFACTURER+" "+Build.MODEL,soc);
    }
    static int threads(int cores,String profile){
        int divisor="Eco".equals(profile)?4:2;
        int cap="Eco".equals(profile)?2:"Performance".equals(profile)?6:4;
        return Math.max(1,Math.min(cap,cores/divisor));
    }
    static int context(String profile){return "Eco".equals(profile)?1024:"Performance".equals(profile)?4096:2048;}
    static long estimate(long bytes,int context){
        if(bytes<=0)return 0;
        // Heuristic, not a measured GGUF architecture-specific KV-cache calculation.
        return bytes+bytes/4+512*MIB+(long)context*128*1024;
    }
    long budget(){return Math.max(0,availableRam-Math.max(768*MIB,totalRam/8));}
    String recommendation(){
        if(!arm64||lowMemory||budget()<GIB)return "Віддалений AI-сервер; недостатній запас RAM для локального запуску";
        if(budget()<2*GIB)return "Орієнтир: 0.5–1B, GGUF Q4";
        if(budget()<4*GIB)return "Орієнтир: 1–3B, GGUF Q4";
        return "Орієнтир: 3–7B, GGUF Q4; потрібна перевірка конкретної моделі";
    }
    String warning(long bytes,String profile){
        if(!arm64)return "ARM64 не виявлено";
        if(lowMemory||thermal>=3)return "Обмеження пам’яті або нагрівання — відкласти локальний запуск";
        if(bytes<=0)return "Модель не імпортовано";
        if(estimate(bytes,context(profile))>budget())return "Оцінка перевищує поточний запас RAM";
        return "Попередня оцінка в межах запасу RAM; запуск не перевірено";
    }
}
