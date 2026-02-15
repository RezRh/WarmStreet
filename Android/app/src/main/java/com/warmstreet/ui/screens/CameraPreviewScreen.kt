package com.warmstreet.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Camera
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import com.warmstreet.shared.CaptureConfig

@Composable
fun CameraPreviewScreen(
    config: CaptureConfig,
    onPhotoCaptured: (ByteArray) -> Unit,
    onCancel: () -> Unit
) {
    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(Color.Black)
    ) {
        // Placeholder for actual camera preview logic
        Column(
            modifier = Modifier.align(Alignment.Center),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            Text(
                text = "Camera Preview",
                color = Color.White,
                style = MaterialTheme.typography.headlineMedium
            )
            Spacer(modifier = Modifier.size(16.dp))
            Text(
                text = "Facing: ${config.facing}",
                color = Color.White.copy(alpha = 0.7f)
            )
        }

        // Controls
        Row(
            modifier = Modifier
                .align(Alignment.BottomCenter)
                .fillMaxWidth()
                .padding(bottom = 64.dp),
            horizontalArrangement = Arrangement.SpaceEvenly,
            verticalAlignment = Alignment.CenterVertically
        ) {
            IconButton(onClick = onCancel) {
                Icon(
                    imageVector = Icons.Default.Close,
                    contentDescription = "Cancel",
                    tint = Color.White,
                    modifier = Modifier.size(32.dp)
                )
            }

            IconButton(
                onClick = { 
                    // Simulate capture with empty byte array for now
                    onPhotoCaptured(ByteArray(0)) 
                },
                modifier = Modifier
                    .size(80.dp)
                    .background(Color.White, shape = MaterialTheme.shapes.extraLarge)
            ) {
                Icon(
                    imageVector = Icons.Default.Camera,
                    contentDescription = "Capture",
                    tint = Color.Black,
                    modifier = Modifier.size(48.dp)
                )
            }
            
            // Spacer to balance the layout
            Spacer(modifier = Modifier.size(48.dp))
        }
    }
}
