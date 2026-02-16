package com.warmstreet.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp

@Composable
fun LocationPermissionScreen(
    permissionState: Any?,
    onRequestPermission: () -> Unit,
    onUseCurrentLocation: () -> Unit,
    onDropPin: () -> Unit,
    onOpenSettings: () -> Unit
) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        Text(
            text = "Location Access",
            style = MaterialTheme.typography.headlineMedium
        )

        Spacer(modifier = Modifier.height(16.dp))

        Text(
            text = "We need your location to show nearby cases",
            style = MaterialTheme.typography.bodyLarge,
            textAlign = TextAlign.Center,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )

        // Text("Debug State: $permissionState")

        Spacer(modifier = Modifier.height(32.dp))

        Button(
            onClick = onRequestPermission,
            modifier = Modifier.fillMaxWidth()
        ) {
            Text("Enable Location")
        }

        Spacer(modifier = Modifier.height(16.dp))

        OutlinedButton(
            onClick = onUseCurrentLocation,
            modifier = Modifier.fillMaxWidth()
        ) {
            Text("Use Current Location")
        }

        Spacer(modifier = Modifier.height(16.dp))

        OutlinedButton(
            onClick = onDropPin,
            modifier = Modifier.fillMaxWidth()
        ) {
            Text("Choose on Map")
        }

        Spacer(modifier = Modifier.height(32.dp))

        TextButton(onClick = onOpenSettings) {
            Text("Open App Settings")
        }
    }
}
