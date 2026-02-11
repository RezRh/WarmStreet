package com.warmstreet.ui.components

import android.graphics.RenderEffect
import android.graphics.RuntimeShader
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asComposeRenderEffect
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.unit.dp
import dev.chrisbanes.haze.HazeState
import dev.chrisbanes.haze.HazeStyle
import dev.chrisbanes.haze.haze

const val LIQUID_GLASS_SHADER_STRING = """
uniform float2 size;
uniform float2 mousePos;
uniform float refractionIntensity; // 0.0 to 0.05
uniform shader composable;

half4 main(float2 fragCoord) {
    float2 distVec = fragCoord - mousePos;
    float dist = length(distVec);
    
    // Create a refractive warp based on distance
    float2 distortion = distVec * (refractionIntensity / (dist + 50.0));
    
    // Sample channels with slight offsets for Chromatic Aberration
    half4 r = composable.eval(fragCoord + distortion * 1.1);
    half4 g = composable.eval(fragCoord + distortion);
    half4 b = composable.eval(fragCoord + distortion * 0.9);
    
    return half4(r.r, g.g, b.b, g.a);
}
"""

val LocalHazeState = compositionLocalOf { HazeState() }

@Composable
fun Modifier.liquidGlassEffect(
    hazeState: HazeState,
    isHovered: Boolean,
    mousePos: androidx.compose.ui.geometry.Offset = androidx.compose.ui.geometry.Offset.Zero
): Modifier {
    val refractionByHover by animateFloatAsState(
        targetValue = if (isHovered) 0.05f else 0.01f,
        animationSpec = spring(stiffness = Spring.StiffnessLow),
        label = "refraction"
    )

    return this
        .haze(
            state = hazeState,
            style = HazeStyle(
                backgroundColor = Color.White.copy(alpha = 0.05f),
                blurRadius = 20.dp,
                noiseFactor = 0.02f // Adds a "silk" texture
            )
        )
        .graphicsLayer {
            // Initialize the AGSL shader
            if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.TIRAMISU) {
                val shader = RuntimeShader(LIQUID_GLASS_SHADER_STRING)
                shader.setFloatUniform("refractionIntensity", refractionByHover)
                shader.setFloatUniform("mousePos", mousePos.x, mousePos.y)
                shader.setFloatUniform("size", size.width, size.height)
                renderEffect = RenderEffect.createRuntimeShaderEffect(shader, "composable")
                    .asComposeRenderEffect()
            }
        }
        .border(0.5.dp, Color.White.copy(alpha = 0.2f), RoundedCornerShape(24.dp))
}

@Composable
fun LiquidGlassSurface(
    modifier: Modifier = Modifier,
    hazeState: HazeState = LocalHazeState.current,
    isHovered: Boolean = false,
    mousePos: androidx.compose.ui.geometry.Offset = androidx.compose.ui.geometry.Offset.Zero,
    content: @Composable BoxScope.() -> Unit
) {
    Box(
        modifier = modifier.liquidGlassEffect(hazeState, isHovered, mousePos),
        content = content
    )
}
