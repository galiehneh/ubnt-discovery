package com.galiehneh.ubntdiscovery

import android.app.Application
import android.content.Context
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.net.wifi.WifiManager
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

data class UiState(
    val devices: Map<String, Device> = emptyMap(),
    val query: String = "",
    val sort: SortSpec = SortSpec(),
    val scanning: Boolean = false,
    val buttonLabel: String = "Scan",
    val message: String? = null,
    val selected: Set<String> = emptySet(),
) {
    val visible: List<Device> get() = filterAndSort(devices.values, query, sort)
}

class DiscoveryViewModel(app: Application) : AndroidViewModel(app) {
    private val _state = MutableStateFlow(UiState())
    val state: StateFlow<UiState> = _state.asStateFlow()

    private var autostartJob: Job? = null
    private var scanJob: Job? = null

    init {
        autostartJob = viewModelScope.launch {
            for (left in AUTOSTART_DELAY_SECS downTo 1) {
                _state.update { it.copy(buttonLabel = "Scan in ${left}s") }
                delay(1000)
            }
            startScan()
        }
    }

    fun onScanClick() = if (_state.value.scanning) stopScan() else startScan()

    private fun startScan() {
        autostartJob?.cancel()
        if (scanJob?.isActive == true) return
        if (!onLocalNetwork()) {
            _state.update { it.copy(buttonLabel = "Scan", message = "Not connected to Wi-Fi or Ethernet.") }
            return
        }
        _state.update { it.copy(scanning = true, message = null, buttonLabel = countdown(0)) }
        scanJob = viewModelScope.launch {
            val ticker = launch {
                for (elapsed in 1..SCAN_SECONDS) {
                    delay(1000)
                    _state.update { it.copy(buttonLabel = countdown(elapsed)) }
                }
            }
            val error = withContext(Dispatchers.IO) { runScan() }
            ticker.cancel()
            _state.update {
                it.copy(scanning = false, buttonLabel = "Scan", message = error ?: it.message)
            }
        }
    }

    /** Blocking; returns an error message or null. */
    private fun runScan(): String? {
        val wifi = getApplication<Application>().applicationContext
            .getSystemService(Context.WIFI_SERVICE) as? WifiManager
        val lock = wifi?.createMulticastLock("ubnt-discovery")?.apply { setReferenceCounted(false) }
        return try {
            lock?.acquire()
            NativeBridge.scan(SCAN_SECONDS * 1000L) { json -> Device.fromJson(json)?.let(::addDevice) }
            null
        } catch (e: Throwable) {
            "Scan failed: ${e.message ?: e.javaClass.simpleName}"
        } finally {
            if (lock?.isHeld == true) lock.release()
        }
    }

    private fun stopScan() {
        runCatching { NativeBridge.cancel() }
    }

    override fun onCleared() {
        runCatching { NativeBridge.cancel() }
    }

    private fun addDevice(d: Device) = _state.update { s ->
        s.copy(devices = s.devices + (d.mac to (s.devices[d.mac]?.mergedWith(d) ?: d)))
    }

    private fun onLocalNetwork(): Boolean {
        val cm = getApplication<Application>().getSystemService(Context.CONNECTIVITY_SERVICE) as? ConnectivityManager
        val caps = cm?.getNetworkCapabilities(cm.activeNetwork) ?: return false
        return caps.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) ||
            caps.hasTransport(NetworkCapabilities.TRANSPORT_ETHERNET)
    }

    fun clear() = _state.update { it.copy(devices = emptyMap(), selected = emptySet(), message = null) }

    fun setQuery(q: String) = _state.update { it.copy(query = q) }

    fun toggleSort(c: SortColumn) = _state.update { it.copy(sort = it.sort.toggled(c)) }

    fun toggleSelected(mac: String) = _state.update {
        it.copy(selected = if (mac in it.selected) it.selected - mac else it.selected + mac)
    }

    fun clearSelection() = _state.update { it.copy(selected = emptySet()) }

    /** TSV of selected visible rows, or all visible rows when none are selected. */
    fun copyText(): String {
        val s = _state.value
        val rows = s.visible.let { v -> if (s.selected.isEmpty()) v else v.filter { it.mac in s.selected } }
        return buildTsv(rows)
    }

    fun postMessage(msg: String) = _state.update { it.copy(message = msg) }

    private fun countdown(elapsed: Int) = "Scanning (${(SCAN_SECONDS - elapsed).coerceAtLeast(0)}s)"
}
