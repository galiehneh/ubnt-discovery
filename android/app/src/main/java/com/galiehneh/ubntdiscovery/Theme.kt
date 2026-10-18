package com.galiehneh.ubntdiscovery

import android.content.Context
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

private val LightColors = lightColorScheme(
    primary = Color(0xFF0B57D0), onPrimary = Color(0xFFFFFFFF),
    primaryContainer = Color(0xFFDCE6FF), onPrimaryContainer = Color(0xFF001A41),
    secondary = Color(0xFF4E5D78), onSecondary = Color(0xFFFFFFFF),
    secondaryContainer = Color(0xFFD9E2F5), onSecondaryContainer = Color(0xFF0E1B2E),
    error = Color(0xFFB3261E), onError = Color(0xFFFFFFFF),
    errorContainer = Color(0xFFF9DEDC), onErrorContainer = Color(0xFF410E0B),
    background = Color(0xFFF6F8FC), onBackground = Color(0xFF0F172A),
    surface = Color(0xFFF6F8FC), onSurface = Color(0xFF0F172A),
    surfaceVariant = Color(0xFFE1E7F0), onSurfaceVariant = Color(0xFF3C4858),
    surfaceContainerLowest = Color(0xFFFFFFFF), surfaceContainerLow = Color(0xFFF0F3F9),
    surfaceContainer = Color(0xFFEBEFF6), surfaceContainerHigh = Color(0xFFE5EAF2),
    surfaceContainerHighest = Color(0xFFDFE5EE),
    outline = Color(0xFF6B778A), outlineVariant = Color(0xFFC4CCD8),
)

private val DarkColors = darkColorScheme(
    primary = Color(0xFFA8C7FA), onPrimary = Color(0xFF062E6F),
    primaryContainer = Color(0xFF0842A0), onPrimaryContainer = Color(0xFFD3E3FD),
    secondary = Color(0xFFBAC7DC), onSecondary = Color(0xFF243042),
    secondaryContainer = Color(0xFF2F3B4F), onSecondaryContainer = Color(0xFFD9E3F8),
    error = Color(0xFFFFB4AB), onError = Color(0xFF690005),
    errorContainer = Color(0xFF93000A), onErrorContainer = Color(0xFFFFDAD6),
    background = Color(0xFF0F1419), onBackground = Color(0xFFE3E8EF),
    surface = Color(0xFF0F1419), onSurface = Color(0xFFE3E8EF),
    surfaceVariant = Color(0xFF2A323D), onSurfaceVariant = Color(0xFFBFC8D4),
    surfaceContainerLowest = Color(0xFF0A0E12), surfaceContainerLow = Color(0xFF161C23),
    surfaceContainer = Color(0xFF1B222A), surfaceContainerHigh = Color(0xFF232B34),
    surfaceContainerHighest = Color(0xFF2C343E),
    outline = Color(0xFF8A94A3), outlineVariant = Color(0xFF3A434F),
)

private const val PREFS = "settings"
private const val KEY_THEME = "theme"

/** Saved choice ("dark"/"light"), or null to follow the system. */
fun Context.savedDark(): Boolean? =
    when (getSharedPreferences(PREFS, Context.MODE_PRIVATE).getString(KEY_THEME, null)) {
        "dark" -> true
        "light" -> false
        else -> null
    }

fun Context.saveDark(dark: Boolean) {
    getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit().putString(KEY_THEME, if (dark) "dark" else "light").apply()
}

@Composable
fun AppTheme(dark: Boolean, content: @Composable () -> Unit) {
    MaterialTheme(colorScheme = if (dark) DarkColors else LightColors, content = content)
}
