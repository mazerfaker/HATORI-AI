package com.hatoriel.app;
public final class NativeCore {
    static { System.loadLibrary("hatoriel_android"); }
    private NativeCore() {}
    public static native String version();
    public static native String prepare(String input);
    public static native String prepareHistory(String historyJson);
}
