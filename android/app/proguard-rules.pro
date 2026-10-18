# JNI looks these up by name from native code.
-keep class com.galiehneh.ubntdiscovery.NativeBridge { *; }
-keep interface com.galiehneh.ubntdiscovery.DeviceListener { *; }
-keepclasseswithmembernames class * { native <methods>; }
