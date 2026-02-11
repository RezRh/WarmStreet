package com.warmstreet.ui.screens

import android.net.Uri
import androidx.browser.customtabs.CustomTabsIntent
import androidx.compose.foundation.layout.*
import androidx.compose.material3.Button
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp

@Composable
fun LoginScreen(core: Core) {
    val context = LocalContext.current
    
    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(Brush.verticalGradient(listOf(Color(0xFF0D1117), Color(0xFF161B22))))
    ) {
        // Decorative Blobs
        Box(
            modifier = Modifier
                .size(300.dp)
                .offset(x = (-150).dp, y = (-100).dp)
                .background(Color(0xFF2196F3).copy(alpha = 0.1f), CircleShape)
        )
        
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(32.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.SpaceBetween
        ) {
            Spacer(modifier = Modifier.height(60.dp))
            
            // Logo Section
            Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(16.dp)) {
                Icon(
                    Icons.Default.Star, // Placeholder for shield
                    contentDescription = null,
                    modifier = Modifier.size(80.dp),
                    tint = Color(0xFF2196F3)
                )
                Text(
                    text = "WarmStreet",
                    color = Color.White,
                    fontSize = 40.sp,
                    fontWeight = FontWeight.Black
                )
                Text(
                    text = "COMMUNITY ANIMAL RESCUE",
                    color = Color(0xFF2196F3).copy(alpha = 0.7f),
                    fontSize = 12.sp,
                    fontWeight = FontWeight.Bold,
                    letterSpacing = 4.sp
                )
            }
            
            // Login Card
            GlassSurface {
                Column(
                    modifier = Modifier.padding(32.dp),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.spacedBy(24.dp)
                ) {
                    Text("Welcome Back", color = Color.White, fontWeight = FontWeight.Bold, fontSize = 20.sp)
                    
                    Button(
                        onClick = {
                            val url = "https://your-neon-auth-url/login?callback=warmstreet://auth"
                            val intent = CustomTabsIntent.Builder().build()
                            intent.launchUrl(context, Uri.parse(url))
                        },
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(56.dp),
                        shape = CircleShape,
                        colors = ButtonDefaults.buttonColors(containerColor = Color.Transparent),
                        contentPadding = PaddingValues()
                    ) {
                        Box(
                            modifier = Modifier
                                .fillMaxWidth()
                                .fillMaxHeight()
                                .background(Brush.linearGradient(listOf(Color(0xFF2196F3), Color(0xFF00BCD4))))
                                .padding(vertical = 12.dp),
                            contentAlignment = Alignment.Center
                        ) {
                            Text("Continue with Google", fontWeight = FontWeight.Bold, color = Color.White)
                        }
                    }
                }
            }
            
            Text(
                "By continuing, you agree to our Terms of Service.",
                color = Color.White.copy(alpha = 0.4f),
                fontSize = 12.sp,
                modifier = Modifier.padding(bottom = 20.dp)
            )
        }
    }
}
