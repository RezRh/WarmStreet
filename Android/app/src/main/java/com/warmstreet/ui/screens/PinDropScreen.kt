package com.warmstreet.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import com.warmstreet.Core
import com.warmstreet.shared.Event

@Composable
fun PinDropScreen(core: Core) {
    Box(modifier = Modifier.fillMaxSize()) {
        Box(
            modifier = Modifier
                .fillMaxSize()
                .background(Color.Gray.copy(alpha = 0.3f))
        ) {
            Text("MapLibre View Placeholder", modifier = Modifier.align(Alignment.Center))
        }
        
        // Pin center (static UI overlay)
        
        Button(
            onClick = {
                // Determine center coordinates from map view
                core.update(Event.LocationPinDropped(lat = 0.0, lng = 0.0))
            },
            modifier = Modifier
                .align(Alignment.BottomCenter)
                .padding(32.dp)
        ) {
            Text("Confirm Location")
        }
    }
}
