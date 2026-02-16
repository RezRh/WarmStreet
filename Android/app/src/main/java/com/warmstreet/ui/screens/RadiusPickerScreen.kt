package com.warmstreet.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.warmstreet.shared.Location
import com.warmstreet.ui.components.RadiusOption

@Composable
fun RadiusPickerScreen(
    location: Location,
    radius: Double,
    onRadiusChanged: (Double) -> Unit,
    onConfirm: () -> Unit,
    onBack: () -> Unit
) {
    val options = listOf(2000.0, 5000.0, 10000.0, 20000.0, 25000.0)

    Column(
        modifier = Modifier.fillMaxSize().padding(16.dp),
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        Text("Select Alert Radius", style = MaterialTheme.typography.headlineSmall)
        
        Spacer(modifier = Modifier.height(20.dp))
        
        // Map Preview with Circle Overlay (stub)
        Box(modifier = Modifier.size(200.dp).padding(16.dp)) {
            Text("Map Preview for $location")
        }
        
        Spacer(modifier = Modifier.height(20.dp))
        
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            options.take(3).forEach { opt ->
                RadiusOption(meters = opt.toInt(), isSelected = radius == opt) {
                    onRadiusChanged(opt)
                }
            }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            options.drop(3).forEach { opt ->
                RadiusOption(meters = opt.toInt(), isSelected = radius == opt) {
                    onRadiusChanged(opt)
                }
            }
        }
        
        Spacer(modifier = Modifier.weight(1f))
        
        Button(
            onClick = onConfirm,
            modifier = Modifier.fillMaxWidth()
        ) {
            Text("Continue")
        }

        Spacer(modifier = Modifier.height(8.dp))

        Button(
            onClick = onBack,
            modifier = Modifier.fillMaxWidth()
        ) {
            Text("Back")
        }
    }
}
