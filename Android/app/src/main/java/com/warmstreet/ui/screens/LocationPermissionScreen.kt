package com.warmstreet.ui.screens

import android.Manifest
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.*
import androidx.compose.material3.Button
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.warmstreet.Core
import com.warmstreet.shared.Event

@Composable
fun LocationPermissionScreen(core: Core, onPinDrop: () -> Unit) {
    val locationLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.RequestMultiplePermissions()
    ) { permissions ->
        if (permissions[Manifest.permission.ACCESS_FINE_LOCATION] == true || 
            permissions[Manifest.permission.ACCESS_COARSE_LOCATION] == true) {
            // Permission granted, core will request location via handler -> sends event
            // core.locationHandler.getLastLocation(...) called from Core logic or ViewState effect
            // For now, simulate button press triggering location fetch in Core
        } else {
            // Denied
        }
    }

    Column(
        modifier = Modifier.fillMaxSize().padding(24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        Text("Enable Location", style = androidx.compose.material3.MaterialTheme.typography.headlineMedium)
        Text("We need your location to find nearby rescues.", modifier = Modifier.padding(16.dp))
        
        Button(
            onClick = { 
                locationLauncher.launch(arrayOf(
                    Manifest.permission.ACCESS_FINE_LOCATION,
                    Manifest.permission.ACCESS_COARSE_LOCATION
                ))
            },
            modifier = Modifier.fillMaxWidth()
        ) {
            Text("Use My Current Location")
        }
        
        TextButton(onClick = onPinDrop) {
            Text("Drop a Pin Instead")
        }
    }
}
