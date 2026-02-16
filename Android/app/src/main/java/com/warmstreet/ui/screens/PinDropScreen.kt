package com.warmstreet.ui.screens

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import com.warmstreet.shared.Location

@Composable
fun PinDropScreen(
    initialLocation: Location?,
    onLocationSelected: (Location) -> Unit,
    onCancel: () -> Unit
) {
    Box(
        modifier = Modifier.fillMaxSize(),
        contentAlignment = Alignment.Center
    ) {
        Text("Map View - Drop Pin Here. Initial: $initialLocation")
    }
}
