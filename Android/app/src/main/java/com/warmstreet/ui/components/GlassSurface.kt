package com.warmstreet.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.blur
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import dev.chrisbanes.haze.HazeState
import com.warmstreet.ui.components.liquidGlassEffect
import com.warmstreet.ui.components.LocalHazeState

@Composable
fun GlassSurface(
    modifier: Modifier = Modifier,
    cornerRadius: Dp = 24.dp,
    isHovered: Boolean = false,
    mousePos: androidx.compose.ui.geometry.Offset = androidx.compose.ui.geometry.Offset.Zero,
    content: @Composable () -> Unit
) {
    Box(
        modifier = modifier
            .clip(RoundedCornerShape(cornerRadius))
            .liquidGlassEffect(
                hazeState = LocalHazeState.current,
                isHovered = isHovered,
                mousePos = mousePos
            )
    ) {
        content()
    }
}
