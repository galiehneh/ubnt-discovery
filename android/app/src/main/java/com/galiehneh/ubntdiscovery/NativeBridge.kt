package com.galiehneh.ubntdiscovery

/** Receives one JSON document per discovered device (called from the native thread). */
fun interface DeviceListener {
    fun onDevice(json: String)
}

object NativeBridge {
    init {
        System.loadLibrary("ubnt_discovery_jni")
    }

    /** Blocks for [durationMs]; [mode] is [ScanMode.ordinal]. Call off the main thread. */
    external fun scan(mode: Int, durationMs: Long, listener: DeviceListener)

    external fun cancel()
}
