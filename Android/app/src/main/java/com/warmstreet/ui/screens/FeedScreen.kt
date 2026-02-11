package com.warmstreet.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import dev.chrisbanes.haze.HazeState
import dev.chrisbanes.haze.hazeSource
import com.warmstreet.Core
import com.warmstreet.shared.Event
import com.warmstreet.shared.ViewState
import com.warmstreet.ui.components.LocalHazeState
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import com.warmstreet.shared.Event
import com.warmstreet.shared.ViewState

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun FeedScreen(core: Core) {
    val state = core.view.state
    
    if (state !is ViewState.Ready) return

    val hazeState = remember { HazeState() }
    val currentTab = if (state.feedView == "Map") 0 else 1
    CompositionLocalProvider(LocalHazeState provides hazeState) {
        Scaffold(
            bottomBar = {
                if (core.view.stagedPhoto == null) {
                    com.warmstreet.ui.components.GlassNavigationBar {
                        NavigationBarItem(
                            icon = { Icon(painter = painterResource(android.R.drawable.ic_dialog_map), contentDescription = "Map") },
                            label = { Text("Map") },
                            selected = currentTab == 0,
                            onClick = { core.update(Event.SwitchToMap) },
                            colors = NavigationBarItemDefaults.colors(
                                selectedIconColor = Color.White,
                                unselectedIconColor = Color.White.copy(alpha = 0.5f),
                                selectedTextColor = Color.White,
                                unselectedTextColor = Color.White.copy(alpha = 0.5f),
                                indicatorColor = Color.White.copy(alpha = 0.1f)
                            )
                        )
                        NavigationBarItem(
                            icon = { Icon(painter = painterResource(android.R.drawable.ic_menu_agenda), contentDescription = "List") },
                            label = { Text("List") },
                            selected = currentTab == 1,
                            onClick = { core.update(Event.SwitchToList) },
                            colors = NavigationBarItemDefaults.colors(
                                selectedIconColor = Color.White,
                                unselectedIconColor = Color.White.copy(alpha = 0.5f),
                                selectedTextColor = Color.White,
                                unselectedTextColor = Color.White.copy(alpha = 0.5f),
                                indicatorColor = Color.White.copy(alpha = 0.1f)
                            )
                        )
                        NavigationBarItem(
                            icon = { Icon(painter = painterResource(android.R.drawable.ic_input_add), contentDescription = "Report") },
                            label = { Text("Report") },
                            selected = false,
                            onClick = { core.update(Event.CapturePhotoRequested) },
                            colors = NavigationBarItemDefaults.colors(
                                selectedIconColor = Color.White,
                                unselectedIconColor = Color.White.copy(alpha = 0.5f),
                                selectedTextColor = Color.White,
                                unselectedTextColor = Color.White.copy(alpha = 0.5f),
                                indicatorColor = Color.White.copy(alpha = 0.1f)
                            )
                        )
                        NavigationBarItem(
                            icon = { Icon(painter = painterResource(android.R.drawable.ic_menu_my_calendar), contentDescription = "Profile") },
                            label = { Text("Profile") },
                            selected = false,
                            onClick = { /* Navigate to Profile */ },
                            colors = NavigationBarItemDefaults.colors(
                                selectedIconColor = Color.White,
                                unselectedIconColor = Color.White.copy(alpha = 0.5f),
                                selectedTextColor = Color.White,
                                unselectedTextColor = Color.White.copy(alpha = 0.5f),
                                indicatorColor = Color.White.copy(alpha = 0.1f)
                            )
                        )
                    }
                }
            },
            containerColor = Color.Transparent
        ) { padding ->
            Box(modifier = Modifier.fillMaxSize()) {
                // Background Layer (to be blurred)
                Box(modifier = Modifier
                    .fillMaxSize()
                    .background(Brush.verticalGradient(listOf(Color(0xFF0D1117), Color(0xFF161B22))))
                    .hazeSource(state = hazeState)
                    .padding(padding)
                ) {
                     if (currentTab == 0) {
                         MapFeedScreen(core)
                     } else {
                         ListFeedScreen(core)
                     }
                }
                
                // Overlay Layer (Glass components here)
                Box(modifier = Modifier.fillMaxSize().padding(padding)) {
                    // Full-screen overlay for Report
                    if (core.view.stagedPhoto != null) {
                        ReportScreen(core)
                    }
                }
            }
            
            // Modal Bottom Sheet for selection
            if (state.selectedDetail != null) {
                CaseDetailSheet(
                    detail = state.selectedDetail!!,
                    onDismiss = { core.update(Event.CaseDismissed) }
                )
            }
        }
    }
}
