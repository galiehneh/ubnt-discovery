package com.galiehneh.ubntdiscovery

/** Receives one JSON document per discovered device (called from the native thread). */
fun interface DeviceListener {
    fun onDevice(json: String)
}

object NativeBridge {
    init {
        System.loadLibrary("ubnt_discovery_jni")
    }

    /** Blocks for [durationMs]; call off the main thread. */
    external fun scan(durationMs: Long, listener: DeviceListener)

    external fun cancel()
}
