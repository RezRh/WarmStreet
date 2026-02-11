package com.warmstreet.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import com.warmstreet.Core
import com.warmstreet.shared.Event
import com.warmstreet.shared.CaseListItem
import com.warmstreet.shared.ViewState

@Composable
fun ListFeedScreen(core: Core) {
    val state = core.view.state as? ViewState.Ready ?: return
    
    LazyColumn(modifier = Modifier.fillMaxSize()) {
        items(state.listItems) { item ->
            CaseItemRow(item = item, onClick = {
                core.update(Event.CaseMarkerTapped(caseId = item.id))
            })
            Divider()
        }
    }
}

@Composable
fun CaseItemRow(item: CaseListItem, onClick: () -> Unit) {
    ListItem(
        headlineContent = { Text(item.descriptionPreview) },
        supportingContent = { 
            Row {
                Text(item.distanceText)
                Spacer(modifier = Modifier.width(8.dp))
                Text(item.timeAgo)
            }
        },
        leadingContent = {
            // Status dot
            Box(modifier = Modifier.size(12.dp).androidx.compose.foundation.background(Color.Red, androidx.compose.foundation.shape.CircleShape))
        },
        modifier = Modifier.clickable(onClick = onClick)
    )
}
