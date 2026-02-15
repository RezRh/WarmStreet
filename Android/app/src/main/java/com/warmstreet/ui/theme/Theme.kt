package com.warmstreet.ui.theme

import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext

private val WarmOrange = Color(0xFFFF6B35)
private val WarmOrangeDark = Color(0xFFE55A2B)
private val WarmYellow = Color(0xFFFFC107)
private val DeepRed = Color(0xFFD32F2F)

private val LightColorScheme = lightColorScheme(
    primary = WarmOrange,
    onPrimary = Color.White,
    primaryContainer = Color(0xFFFFE0D6),
    onPrimaryContainer = Color(0xFF3D1500),
    secondary = WarmYellow,
    onSecondary = Color.Black,
    secondaryContainer = Color(0xFFFFECB3),
    onSecondaryContainer = Color(0xFF261A00),
    tertiary = Color(0xFF6D5E0F),
    onTertiary = Color.White,
    tertiaryContainer = Color(0xFFF8E287),
    onTertiaryContainer = Color(0xFF221B00),
    error = DeepRed,
    onError = Color.White,
    errorContainer = Color(0xFFFFDAD6),
    onErrorContainer = Color(0xFF410002),
    background = Color(0xFFFFFBFF),
    onBackground = Color(0xFF201A17),
    surface = Color(0xFFFFFBFF),
    onSurface = Color(0xFF201A17),
    surfaceVariant = Color(0xFFF5DED4),
    onSurfaceVariant = Color(0xFF53443C),
    outline = Color(0xFF85736A),
    outlineVariant = Color(0xFFD8C2B8)
)

private val DarkColorScheme = darkColorScheme(
    primary = Color(0xFFFFB599),
    onPrimary = Color(0xFF5F1500),
    primaryContainer = WarmOrangeDark,
    onPrimaryContainer = Color(0xFFFFDBCF),
    secondary = Color(0xFFFFE082),
    onSecondary = Color(0xFF3F2E00),
    secondaryContainer = Color(0xFF5C4300),
    onSecondaryContainer = Color(0xFFFFECB3),
    tertiary = Color(0xFFDBC66E),
    onTertiary = Color(0xFF393000),
    tertiaryContainer = Color(0xFF524600),
    onTertiaryContainer = Color(0xFFF8E287),
    error = Color(0xFFFFB4AB),
    onError = Color(0xFF690005),
    errorContainer = Color(0xFF93000A),
    onErrorContainer = Color(0xFFFFDAD6),
    background = Color(0xFF201A17),
    onBackground = Color(0xFFEDE0DB),
    surface = Color(0xFF201A17),
    onSurface = Color(0xFFEDE0DB),
    surfaceVariant = Color(0xFF53443C),
    onSurfaceVariant = Color(0xFFD8C2B8),
    outline = Color(0xFFA08D84),
    outlineVariant = Color(0xFF53443C)
)

@Composable
fun WarmStreetTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    dynamicColor: Boolean = false,
    content: @Composable () -> Unit
) {
    val colorScheme = when {
        dynamicColor && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S -> {
            val context = LocalContext.current
            if (darkTheme) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
        }
        darkTheme -> DarkColorScheme
        else -> LightColorScheme
    }

    MaterialTheme(
        colorScheme = colorScheme,
        content = content
    )
}
