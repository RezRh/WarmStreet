package com.warmstreet.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import com.warmstreet.Core
import com.warmstreet.shared.CaseDetail
import com.warmstreet.shared.ClaimState
import com.warmstreet.shared.Event
// import coil.compose.AsyncImage

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CaseDetailSheet(detail: CaseDetail, onDismiss: () -> Unit, core: Core) {
    ModalBottomSheet(
        onDismissRequest = onDismiss,
        containerColor = Color.Transparent,
        dragHandle = {
            BottomSheetDefaults.DragHandle(color = Color.White.copy(alpha = 0.3f))
        }
    ) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .background(Brush.verticalGradient(listOf(Color(0xFF0D1117), Color(0xFF161B22))))
                .verticalScroll(rememberScrollState())
                .padding(20.dp),
            verticalArrangement = Arrangement.spacedBy(20.dp)
        ) {
            // Header Image Card
            GlassSurface {
                Box(modifier = Modifier.height(250.dp).fillMaxWidth()) {
                    if (detail.photoUrl != null) {
                        // In a real app, use Coil here: AsyncImage(model = detail.photoUrl, ...)
                        Box(Modifier.fillMaxSize().background(Color.Gray.copy(alpha = 0.1f)), contentAlignment = Alignment.Center) {
                            Text("Animal Photo", color = Color.White.copy(alpha = 0.5f))
                        }
                    } else {
                        Box(Modifier.fillMaxSize().background(Color.White.copy(alpha = 0.05f)), contentAlignment = Alignment.Center) {
                            Icon(Icons.Default.Info, contentDescription = null, modifier = Modifier.size(48.dp), tint = Color.Gray)
                        }
                    }
                    
                    // Status Badge
                    Surface(
                        modifier = Modifier.align(Alignment.BottomEnd).padding(12.dp),
                        color = Color(0xFF2196F3).copy(alpha = 0.8f),
                        shape = RoundedCornerShape(12.dp)
                    ) {
                        Text(
                            detail.status.replace("_", " ").uppercase(),
                            modifier = Modifier.padding(horizontal = 10.dp, vertical = 6.dp),
                            color = Color.White,
                            fontSize = 11.sp,
                            fontWeight = FontWeight.Bold
                        )
                    }
                }
            }
            
            // Gemini Diagnosis Card
            if (detail.geminiDiagnosis != null) {
                GlassSurface {
                    Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            Icon(Icons.Default.Star, contentDescription = null, tint = Color(0xFF9C27B0), modifier = Modifier.size(20.dp))
                            Text("AI Diagnosis", color = Color(0xFF9C27B0), fontWeight = FontWeight.Bold, fontSize = 16.sp)
                            Spacer(Modifier.weight(1f))
                            Text("GEMINI 2.0", color = Color(0xFF9C27B0).copy(alpha = 0.6f), fontSize = 10.sp, fontWeight = FontWeight.Bold)
                        }
                        Text(
                            detail.geminiDiagnosis!!,
                            color = Color.White.copy(alpha = 0.8f),
                            fontSize = 14.sp,
                            lineHeight = 20.sp,
                            fontStyle = androidx.compose.ui.text.font.FontStyle.Italic
                        )
                    }
                }
            }
            
            // Description & Info Card
            GlassSurface {
                Column(Modifier.padding(20.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
                    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                        Text(detail.timeAgo, color = Color.Gray, fontSize = 14.sp, fontWeight = FontWeight.Bold)
                        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                            Icon(Icons.Default.LocationOn, contentDescription = null, tint = Color(0xFF2196F3), modifier = Modifier.size(16.dp))
                            Text(detail.distanceText, color = Color(0xFF2196F3), fontSize = 14.sp)
                        }
                    }
                    
                    Text(
                        detail.description ?: "No description provided.",
                        color = Color.White,
                        fontSize = 16.sp,
                        lineHeight = 24.sp
                    )
                }
            }
            
            // Actions
            Box(Modifier.padding(vertical = 12.dp)) {
                when (detail.claimState) {
                    ClaimState.Available -> {
                         Button(
                            onClick = { core.update(Event.ClaimRequested(detail.id)) },
                            modifier = Modifier.fillMaxWidth().height(56.dp),
                            shape = CircleShape,
                            colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF4CAF50))
                        ) {
                            Text("Claim This Rescue", fontWeight = FontWeight.Bold)
                        }
                    }
                    ClaimState.ClaimedByMe -> {
                         Column(horizontalArrangement = Arrangement.spacedBy(12.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
                             Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
                                 Icon(Icons.Default.CheckCircle, contentDescription = null, tint = Color(0xFF4CAF50))
                                 Text("You claimed this rescue", color = Color(0xFF4CAF50), fontWeight = FontWeight.Bold)
                             }
                             
                             Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                 detail.availableTransitions.forEach { trans ->
                                     Button(
                                         onClick = { core.update(Event.TransitionRequested(detail.id, trans)) },
                                         modifier = Modifier.weight(1f).height(48.dp),
                                         shape = RoundedCornerShape(12.dp),
                                         colors = ButtonDefaults.buttonColors(
                                             containerColor = if (trans == "cancel" || trans == "unreachable") Color(0xFFF44336) else Color(0xFF2196F3)
                                         )
                                     ) {
                                         Text(trans.replace("_", " ").uppercase(), fontSize = 12.sp, fontWeight = FontWeight.Bold)
                                     }
                                 }
                             }
                         }
                    }
                    ClaimState.ClaimedByOther -> {
                        GlassSurface {
                             Row(Modifier.padding(16.dp).fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.Center) {
                                 Icon(Icons.Default.AccountCircle, contentDescription = null, tint = Color.Gray)
                                 Spacer(Modifier.width(8.dp))
                                 Text("Claimed by another volunteer", color = Color.Gray, fontWeight = FontWeight.Bold)
                             }
                        }
                    }
                    else -> {}
                }
            }
            
            Spacer(Modifier.height(40.dp))
        }
    }
}
