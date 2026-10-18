package com.galiehneh.ubntdiscovery

import android.app.Activity
import android.content.Context
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.SideEffect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalView
import androidx.core.view.WindowCompat

private val LightColors = lightColorScheme(
    primary = Color(0xFF0B57D0), onPrimary = Color(0xFFFFFFFF),
    primaryContainer = Color(0xFFD3E3FD), onPrimaryContainer = Color(0xFF041E49),
    secondaryContainer = Color(0xFFD3E3FD), onSecondaryContainer = Color(0xFF041E49),
    error = Color(0xFFB3261E), onError = Color(0xFFFFFFFF),
    background = Color(0xFFFFFFFF), onBackground = Color(0xFF111111),
    surface = Color(0xFFF3F3F3), onSurface = Color(0xFF111111),
    surfaceVariant = Color(0xFFF3F3F3), onSurfaceVariant = Color(0xFF3D3D3D),
    surfaceContainerLowest = Color(0xFFFFFFFF), surfaceContainerLow = Color(0xFFF3F3F3),
    surfaceContainer = Color(0xFFF3F3F3), surfaceContainerHigh = Color(0xFFF3F3F3),
    surfaceContainerHighest = Color(0xFFF3F3F3),
    outline = Color(0xFF5F5F5F), outlineVariant = Color(0xFFBDBDBD),
)

private val DarkColors = darkColorScheme(
    primary = Color(0xFF8AB4F8), onPrimary = Color(0xFF062E6F),
    primaryContainer = Color(0xFF1F3A60), onPrimaryContainer = Color(0xFFD3E3FD),
    secondaryContainer = Color(0xFF1F3A60), onSecondaryContainer = Color(0xFFD3E3FD),
    error = Color(0xFFFFB4AB), onError = Color(0xFF690005),
    background = Color(0xFF121212), onBackground = Color(0xFFF2F2F2),
    surface = Color(0xFF1F1F1F), onSurface = Color(0xFFF2F2F2),
    surfaceVariant = Color(0xFF1F1F1F), onSurfaceVariant = Color(0xFFC7C7C7),
    surfaceContainerLowest = Color(0xFF121212), surfaceContainerLow = Color(0xFF1F1F1F),
    surfaceContainer = Color(0xFF1F1F1F), surfaceContainerHigh = Color(0xFF1F1F1F),
    surfaceContainerHighest = Color(0xFF1F1F1F),
    outline = Color(0xFF9A9A9A), outlineVariant = Color(0xFF4A4A4A),
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
    val scheme = if (dark) DarkColors else LightColors
    val view = LocalView.current
    if (!view.isInEditMode) {
        SideEffect {
            val window = (view.context as Activity).window
            WindowCompat.getInsetsController(window, view).apply {
                isAppearanceLightStatusBars = !dark
                isAppearanceLightNavigationBars = !dark
            }
        }
    }
    MaterialTheme(colorScheme = scheme, content = content)
}
