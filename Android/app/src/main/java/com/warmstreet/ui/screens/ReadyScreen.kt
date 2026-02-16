package com.warmstreet.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.warmstreet.shared.Event
import com.warmstreet.shared.ViewState

@Composable
fun ReadyScreen(
    state: ViewState.Ready,
    onEvent: (Event) -> Unit
) {
    Column(
        modifier = Modifier.fillMaxSize(),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        Text("You're All Set!", style = MaterialTheme.typography.displaySmall)
        Text("Welcome to WarmStreet.")
        
        Spacer(modifier = Modifier.height(32.dp))
        
        // Feed view mode toggle or list of items could go here
        Text("Feed View: ${state.feedView}")
        
        if (state.feedView == com.warmstreet.shared.FeedViewMode.List) {
            Text("Items count: ${state.listItems.size}")
        }
        
        Button(onClick = { onEvent(Event.BackPressed) }) {
            Text("Back")
        }
    }
}
