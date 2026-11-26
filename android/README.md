# Hatoriel Android diagnostic — 0.3.9
This is a JNI smoke-test application, not an AI chatbot. The native bridge exposes version and prompt preparation only; it makes no network requests and does not run inference. No model is bundled.
Target: ARM64 (arm64-v8a), Android API 26+, compile/target SDK 35. Java 17, Gradle 8.9, AGP 8.7.3, NDK 27.2.12479018.
GitHub Actions Android CI builds the Rust library and assembles a debug APK; download artifact Hatoriel-Android-ARM64-Diagnostic only after a successful run. No Gradle wrapper is committed yet. Open android/ in Android Studio using the compatible Gradle installation; the native library must be built and copied into app/src/main/jniLibs/arm64-v8a before assembling. The workflow provides Linux build steps.
Next: real conversation UI, local or explicitly authorized remote inference, persistence, lifecycle and cancellation, Android permissions, device tests. No iOS bridge, release signing or production-ready APK yet.
