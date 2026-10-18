package com.galiehneh.ubntdiscovery

import org.json.JSONObject

const val APP_VERSION = "2026.10.18.1"
const val AUTHOR = "galiehneh"
const val REPO_URL = "https://github.com/galiehneh/ubnt-discovery"
const val SCAN_SECONDS = 6
const val AUTOSTART_DELAY_SECS = 4
private const val SECS_PER_DAY = 86_400L

data class DeviceAddress(val mac: String, val ip: String?)

data class Device(
    val mac: String,
    val ip: String?,
    val hostname: String?,
    val essid: String?,
    val firmware: String?,
    val modelCode: String?,
    val product: String?,
    val uptime: String?,
    val addresses: List<DeviceAddress>,
) {
    val displayProduct: String
        get() = product.nonBlank() ?: modelCode.nonBlank() ?: "Unknown"
    val displayIp: String get() = ip.orEmpty()

    /** Six list cells in column order. */
    fun cells(): List<String> = listOf(
        displayProduct, displayIp, mac, hostname.orEmpty(), essid.orEmpty(), firmware.orEmpty(),
    )

    /** Incoming non-blank values win; blanks keep the existing value. */
    fun mergedWith(n: Device): Device = Device(
        mac = mac,
        ip = n.ip ?: ip,
        hostname = n.hostname.nonBlank()?.let { n.hostname } ?: hostname,
        essid = n.essid.nonBlank()?.let { n.essid } ?: essid,
        firmware = n.firmware.nonBlank()?.let { n.firmware } ?: firmware,
        modelCode = n.modelCode.nonBlank()?.let { n.modelCode } ?: modelCode,
        product = n.product.nonBlank()?.let { n.product } ?: product,
        uptime = n.uptime ?: uptime,
        addresses = (addresses + n.addresses).distinct(),
    )

    fun matches(query: String): Boolean = listOf(
        displayProduct, displayIp, mac, hostname.orEmpty(), essid.orEmpty(),
        firmware.orEmpty(), modelCode.orEmpty(),
    ).joinToString("\u0001").lowercase().contains(query.lowercase())

    /** Reported addresses, or the primary pair. */
    fun addressRows(): List<DeviceAddress> = addresses.ifEmpty { listOf(DeviceAddress(mac, ip)) }

    companion object {
        /** Defensive parse; returns null when there is no MAC. */
        fun fromJson(json: String): Device? = runCatching {
            val o = JSONObject(json)
            val mac = o.str("mac")?.trim()?.uppercase().orEmpty()
            if (mac.isEmpty()) return null
            val addrs = o.optJSONArray("addresses")?.let { arr ->
                (0 until arr.length()).mapNotNull { i ->
                    arr.optJSONObject(i)?.let { a ->
                        a.str("mac")?.let { DeviceAddress(it.uppercase(), a.str("ip")) }
                    }
                }
            }.orEmpty()
            Device(
                mac = mac,
                ip = o.str("ip"),
                hostname = o.str("hostname"),
                essid = o.str("essid"),
                firmware = o.str("firmware"),
                modelCode = o.str("model_code"),
                product = o.str("product"),
                uptime = o.str("uptime"),
                addresses = addrs,
            )
        }.getOrNull()

        private fun JSONObject.str(key: String): String? =
            if (isNull(key)) null else opt(key)?.toString()
    }
}

private fun String?.nonBlank(): String? = this?.takeIf { it.isNotBlank() }

enum class SortColumn(val label: String) {
    Product("Product"), Ip("IP"), Mac("MAC"), Name("Name"), Ssid("SSID"), Firmware("Firmware"),
}

data class SortSpec(val column: SortColumn = SortColumn.Product, val ascending: Boolean = true) {
    /** Same column flips direction; a new column starts ascending. */
    fun toggled(c: SortColumn) = SortSpec(c, column != c || !ascending)
}

private fun ipKey(ip: String?): Long {
    val parts = ip?.split('.') ?: return -1
    if (parts.size != 4) return -1
    return parts.fold(0L) { acc, p -> (acc shl 8) or (p.toIntOrNull()?.toLong() ?: return -1) }
}

fun filterAndSort(devices: Collection<Device>, query: String, sort: SortSpec): List<Device> {
    val q = query.trim()
    val cmp = Comparator<Device> { a, b ->
        val c = if (sort.column == SortColumn.Ip) {
            ipKey(a.ip).compareTo(ipKey(b.ip))
        } else {
            val i = sort.column.ordinal
            a.cells()[i].lowercase().compareTo(b.cells()[i].lowercase())
        }
        (if (sort.ascending) c else -c).let { if (it != 0) it else a.mac.compareTo(b.mac) }
    }
    return devices.filter { q.isEmpty() || it.matches(q) }.sortedWith(cmp)
}

fun buildTsv(devices: List<Device>): String =
    devices.joinToString("") { it.cells().joinToString("\t") + "\r\n" }

fun formatUptime(raw: String?): String {
    val s = raw?.trim().orEmpty()
    val total = s.toLongOrNull() ?: return s
    val days = total / SECS_PER_DAY
    val rem = total % SECS_PER_DAY
    val clock = "%02d:%02d:%02d".format(rem / 3600, (rem % 3600) / 60, rem % 60)
    return when (days) {
        0L -> clock
        1L -> "1 day $clock"
        else -> "$days days $clock"
    }
}
