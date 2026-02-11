package com.warmstreet.ui.screens

import android.graphics.BitmapFactory
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowForward
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Star
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.warmstreet.Core
import com.warmstreet.shared.CreateCasePayload
import com.warmstreet.shared.Event
import com.warmstreet.ui.components.GlassSurface

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ReportScreen(core: Core) {
    val view = core.view
    var description by remember { mutableStateOf("") }
    var woundSeverity by remember { mutableStateOf(1f) }
    
    val imageData = view.stagedCrop ?: view.stagedPhoto
    val bitmap = remember(imageData) {
        imageData?.let { BitmapFactory.decodeByteArray(it, 0, it.size) }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("New Report", fontWeight = FontWeight.Bold) },
                navigationIcon = {
                    IconButton(onClick = { core.update(Event.PhotoCancelled) }) {
                        Icon(Icons.Default.Close, contentDescription = "Cancel")
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = Color.Transparent
                )
            )
        },
        containerColor = Color.Transparent
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(20.dp),
            verticalArrangement = Arrangement.spacedBy(24.dp)
        ) {
            // Image Preview Card
            GlassSurface {
                Box(modifier = Modifier.height(250.dp).fillMaxWidth()) {
                    if (bitmap != null) {
                        Image(
                            bitmap = bitmap.asImageBitmap(),
                            contentDescription = "Animal photo",
                            modifier = Modifier.fillMaxSize(),
                            contentScale = ContentScale.Crop
                        )
                    } else {
                        Box(
                            modifier = Modifier.fillMaxSize().background(Color.White.copy(alpha = 0.05f)),
                            contentAlignment = Alignment.Center
                        ) {
                            Text("No photo captured", color = Color.Gray)
                        }
                    }
                    
                    // AI Detection Badge
                    if (view.detectionCount > 0) {
                        Surface(
                            modifier = Modifier
                                .align(Alignment.TopEnd)
                                .padding(12.dp),
                            color = Color(0xFF4CAF50).copy(alpha = 0.8f),
                            shape = RoundedCornerShape(20.dp),
                            tonalElevation = 4.dp
                        ) {
                            Row(
                                modifier = Modifier.padding(horizontal = 12.dp, vertical = 6.dp),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(6.dp)
                            ) {
                                Icon(
                                    Icons.Default.Star,
                                    contentDescription = null,
                                    tint = Color.White,
                                    modifier = Modifier.size(16.dp)
                                )
                                Text(
                                    "Animal Detected",
                                    color = Color.White,
                                    fontSize = 13.sp,
                                    fontWeight = FontWeight.Bold
                                )
                                Text(
                                    "${(view.topConfidence * 100).toInt()}%",
                                    color = Color.White.copy(alpha = 0.9f),
                                    fontSize = 11.sp,
                                    modifier = Modifier
                                        .background(Color.White.copy(alpha = 0.2f), RoundedCornerShape(4.dp))
                                        .padding(horizontal = 4.dp)
                                )
                            }
                        }
                    }
                }
            }
            
            // Input Fields
            GlassSurface {
                Column(
                    modifier = Modifier.padding(20.dp),
                    verticalArrangement = Arrangement.spacedBy(20.dp)
                ) {
                    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text(
                            "Description",
                            fontSize = 14.sp,
                            fontWeight = FontWeight.Bold,
                            color = Color.Gray
                        )
                        TextField(
                            value = description,
                            onValueChange = { description = it },
                            modifier = Modifier.fillMaxWidth().height(100.dp),
                            colors = TextFieldDefaults.textFieldColors(
                                containerColor = Color.White.copy(alpha = 0.05f),
                                focusedIndicatorColor = Color.Transparent,
                                unfocusedIndicatorColor = Color.Transparent
                            ),
                            shape = RoundedCornerShape(8.dp),
                            placeholder = { Text("Describe the animal/situation...") }
                        )
                    }
                    
                    Divider(color = Color.White.copy(alpha = 0.1f))
                    
                    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                        Row(
                            Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Text(
                                "Wound Severity",
                                fontSize = 14.sp,
                                fontWeight = FontWeight.Bold,
                                color = Color.Gray
                            )
                            Text(
                                woundSeverity.toInt().toString(),
                                fontSize = 20.sp,
                                fontWeight = FontWeight.Bold,
                                color = getSeverityColor(woundSeverity.toInt())
                            )
                        }
                        
                        Slider(
                            value = woundSeverity,
                            onValueChange = { woundSeverity = it },
                            valueRange = 1f..5f,
                            steps = 3,
                            colors = SliderDefaults.colors(
                                thumbColor = getSeverityColor(woundSeverity.toInt()),
                                activeTrackColor = getSeverityColor(woundSeverity.toInt())
                            )
                        )
                    }
                }
            }
            
            Spacer(modifier = Modifier.weight(1f).heightIn(min = 40.dp))
            
            // Submit Button
            Button(
                onClick = {
                    val lat = view.areaCenter?.first ?: 0.0
                    val lng = view.areaCenter?.second ?: 0.0
                    core.update(Event.CreateCaseRequested(
                        CreateCasePayload(
                            location = Pair(lat, lng),
                            description = if (description.isEmpty()) null else description,
                            woundSeverity = woundSeverity.toInt()
                        )
                    ))
                },
                modifier = Modifier.fillMaxWidth(),
                shape = CircleShape,
                colors = ButtonDefaults.buttonColors(containerColor = Color.Transparent),
                contentPadding = PaddingValues()
            ) {
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .background(
                            Brush.linearGradient(listOf(Color(0xFF2196F3), Color(0xFF00BCD4)))
                        )
                        .padding(vertical = 16.dp),
                    contentAlignment = Alignment.Center
                ) {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        Text("Report to WarmStreet", fontWeight = FontWeight.Bold, color = Color.White)
                        Icon(Icons.Default.ArrowForward, contentDescription = null, tint = Color.White)
                    }
                }
            }
        }
    }
}

private fun getSeverityColor(severity: Int): Color {
    return when (severity) {
        1 -> Color(0xFF4CAF50)
        2 -> Color(0xFFFFEB3B)
        3 -> Color(0xFFFF9800)
        4 -> Color(0xFFF44336)
        5 -> Color(0xFF9C27B0)
        else -> Color.Gray
    }
}
