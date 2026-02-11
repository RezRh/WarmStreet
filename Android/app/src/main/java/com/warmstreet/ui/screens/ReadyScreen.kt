package com.warmstreet.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import com.warmstreet.Core
// import com.warmstreet.shared.Model

@Composable
fun ReadyScreen(core: Core) {
    val state = core.view.state
    
    // Check if view state is Ready
    // assuming generated bindings have ViewState.Ready
    
    Column(
        modifier = Modifier.fillMaxSize(),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        // Checkmark icon
        Text("You're All Set!", style = androidx.compose.material3.MaterialTheme.typography.displaySmall)
        Text("Welcome to WarmStreet.")
        
        // Show status
        // Text(if (state is ViewState.Ready && state.online) "Online" else "Offline")
        
        // List cases
    }
}
