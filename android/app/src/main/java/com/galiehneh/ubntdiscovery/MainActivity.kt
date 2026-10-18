package com.galiehneh.ubntdiscovery

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.viewModels
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.Image
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.systemBarsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import android.os.Build

class MainActivity : ComponentActivity() {
    private val vm: DiscoveryViewModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent { AppTheme { Screen(vm) } }
    }
}

@Composable
private fun AppTheme(content: @Composable () -> Unit) {
    val dark = isSystemInDarkTheme()
    val ctx = LocalContext.current
    val scheme = when {
        Build.VERSION.SDK_INT >= 31 -> if (dark) dynamicDarkColorScheme(ctx) else dynamicLightColorScheme(ctx)
        dark -> darkColorScheme()
        else -> lightColorScheme()
    }
    MaterialTheme(colorScheme = scheme, content = content)
}

private fun Context.openUrl(url: String) {
    runCatching {
        startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun Screen(vm: DiscoveryViewModel) {
    val s by vm.state.collectAsStateWithLifecycle()
    val ctx = LocalContext.current
    var detail by remember { mutableStateOf<String?>(null) }
    var showAbout by remember { mutableStateOf(false) }
    val visible = s.visible
    val detailDevice = detail?.let { s.devices[it] }

    fun copy() {
        val text = vm.copyText()
        if (text.isEmpty()) return vm.postMessage("Nothing to copy.")
        val cm = ctx.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        cm.setPrimaryClip(ClipData.newPlainText("devices", text))
        vm.postMessage("Copied.")
    }

    Column(Modifier.fillMaxSize().systemBarsPadding().padding(horizontal = 12.dp)) {
        Row(Modifier.fillMaxWidth().padding(top = 8.dp), verticalAlignment = Alignment.CenterVertically) {
            Text("LAN Device Discovery", style = MaterialTheme.typography.titleLarge, modifier = Modifier.weight(1f))
            TextButton(onClick = { showAbout = true }) { Text("About") }
        }
        Row(Modifier.fillMaxWidth().padding(vertical = 6.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(onClick = vm::onScanClick, modifier = Modifier.weight(1f)) {
                Text(if (s.scanning) "${s.buttonLabel} - tap to stop" else s.buttonLabel, maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
            OutlinedButton(onClick = vm::clear, enabled = s.devices.isNotEmpty()) { Text("Clear") }
            OutlinedButton(onClick = ::copy, enabled = visible.isNotEmpty()) {
                Text(if (s.selected.isEmpty()) "Copy" else "Copy (${s.selected.size})")
            }
        }
        OutlinedTextField(
            value = s.query,
            onValueChange = vm::setQuery,
            label = { Text("Search") },
            singleLine = true,
            modifier = Modifier.fillMaxWidth(),
        )
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(vertical = 6.dp), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            SortColumn.entries.forEach { c ->
                val on = s.sort.column == c
                FilterChip(
                    selected = on,
                    onClick = { vm.toggleSort(c) },
                    label = { Text(c.label + if (on) (if (s.sort.ascending) " ▲" else " ▼") else "") },
                )
            }
        }
        s.message?.let {
            Text(it, color = MaterialTheme.colorScheme.error, modifier = Modifier.padding(vertical = 4.dp))
        }
        LazyColumn(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            items(visible, key = { it.mac }) { d ->
                DeviceCard(d, d.mac in s.selected, s.selected.isNotEmpty(),
                    onTap = { if (s.selected.isNotEmpty()) vm.toggleSelected(d.mac) else detail = d.mac },
                    onLong = { vm.toggleSelected(d.mac) })
            }
        }
        Row(Modifier.fillMaxWidth().padding(vertical = 6.dp), verticalAlignment = Alignment.CenterVertically) {
            Text("Total: ${visible.size}", style = MaterialTheme.typography.labelLarge, modifier = Modifier.weight(1f))
            if (s.selected.isNotEmpty()) TextButton(onClick = vm::clearSelection) { Text("Deselect") }
        }
    }

    if (detailDevice != null) {
        ModalBottomSheet(onDismissRequest = { detail = null }) {
            DetailSheet(detailDevice, onWeb = { ip -> ctx.openUrl("http://$ip") }, onClose = { detail = null })
        }
    }
    if (showAbout) AboutDialog(onDismiss = { showAbout = false }, onOpen = { ctx.openUrl(REPO_URL) })
}

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun DeviceCard(d: Device, selected: Boolean, selecting: Boolean, onTap: () -> Unit, onLong: () -> Unit) {
    val colors = if (selected) CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.primaryContainer)
    else CardDefaults.cardColors()
    Card(colors = colors, modifier = Modifier.fillMaxWidth().combinedClickable(onClick = onTap, onLongClick = onLong)) {
        Column(Modifier.padding(12.dp)) {
            Text(d.displayProduct, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
            Text(listOf(d.displayIp, d.mac).filter { it.isNotEmpty() }.joinToString("  |  "), style = MaterialTheme.typography.bodyMedium)
            val extra = listOf(d.hostname, d.essid, d.firmware).mapNotNull { it?.takeIf(String::isNotBlank) }
            if (extra.isNotEmpty()) {
                Text(extra.joinToString("  |  "), style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 2, overflow = TextOverflow.Ellipsis)
            }
        }
    }
}

@Composable
private fun DetailSheet(d: Device, onWeb: (String) -> Unit, onClose: () -> Unit) {
    fun unknown(v: String?) = v?.takeIf { it.isNotBlank() } ?: "Unknown"
    Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp).padding(bottom = 24.dp)) {
        listOf(
            "Product" to d.displayProduct, "MAC" to d.mac, "IP" to d.displayIp,
            "Name" to unknown(d.hostname), "SSID" to unknown(d.essid), "Version" to unknown(d.firmware),
            "Uptime" to unknown(formatUptime(d.uptime)),
        ).forEach { (k, v) ->
            Row(Modifier.padding(vertical = 3.dp)) {
                Text(k, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.weight(0.3f))
                Text(v, modifier = Modifier.weight(0.7f))
            }
        }
        HorizontalDivider(Modifier.padding(vertical = 10.dp))
        Text("Network addresses", style = MaterialTheme.typography.titleSmall)
        d.addressRows().forEach { a ->
            Row(Modifier.fillMaxWidth().padding(vertical = 3.dp)) {
                Text(a.mac, modifier = Modifier.weight(1f))
                Text(a.ip.orEmpty(), modifier = Modifier.weight(1f))
            }
        }
        Row(Modifier.fillMaxWidth().padding(top = 16.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(onClick = { d.ip?.let(onWeb) }, enabled = !d.ip.isNullOrBlank(), modifier = Modifier.weight(1f)) {
                Text("Open Web UI")
            }
            OutlinedButton(onClick = onClose) { Text("Close") }
        }
    }
}

@Composable
private fun AboutDialog(onDismiss: () -> Unit, onOpen: () -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        confirmButton = { TextButton(onClick = onOpen) { Text("Open GitHub") } },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Close") } },
        title = { Text("LAN Device Discovery") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                Image(
                    painterResource(R.drawable.avatar), contentDescription = null, contentScale = ContentScale.Crop,
                    modifier = Modifier.size(80.dp).clip(CircleShape),
                )
                Text("Version $APP_VERSION")
                Text("by $AUTHOR")
                Text(REPO_URL, style = MaterialTheme.typography.bodySmall, modifier = Modifier.clickable(onClick = onOpen))
                Text("License: MIT OR Apache-2.0", style = MaterialTheme.typography.bodySmall)
                Text("Unofficial; not affiliated with Ubiquiti Inc.", style = MaterialTheme.typography.bodySmall)
            }
        },
    )
}
