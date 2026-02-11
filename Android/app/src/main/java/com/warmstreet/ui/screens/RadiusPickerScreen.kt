package com.warmstreet.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.material3.Button
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.warmstreet.Core
import com.warmstreet.shared.Event
import com.warmstreet.ui.components.RadiusOption

@Composable
fun RadiusPickerScreen(core: Core) {
    var selectedRadius by remember { mutableStateOf(5000) }
    val options = listOf(2000, 5000, 10000, 20000, 25000)

    Column(
        modifier = Modifier.fillMaxSize().padding(16.dp),
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        Text("Select Alert Radius", style = androidx.compose.material3.MaterialTheme.typography.headlineSmall)
        
        Spacer(modifier = Modifier.height(20.dp))
        
        // Map Preview with Circle Overlay (stub)
        Box(modifier = Modifier.size(200.dp).padding(16.dp)) {
            Text("Map Preview")
        }
        
        Spacer(modifier = Modifier.height(20.dp))
        
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            options.take(3).forEach { radius ->
                RadiusOption(meters = radius, isSelected = selectedRadius == radius) {
                    selectedRadius = radius
                }
            }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            options.drop(3).forEach { radius ->
                RadiusOption(meters = radius, isSelected = selectedRadius == radius) {
                    selectedRadius = radius
                }
            }
        }
        
        Spacer(modifier = Modifier.weight(1f))
        
        Button(
            onClick = {
                core.update(Event.RadiusSelected(meters = selectedRadius.toLong()))
            },
            modifier = Modifier.fillMaxWidth()
        ) {
            Text("Continue")
        }
    }
}
