package com.lfi.sovereign

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.unit.sp

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            SovereignTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = Color(0xFF050505)
                ) {
                    LfiTerminal()
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun LfiTerminal() {
    var logs by remember { mutableStateOf(listOf("[SYSTEM] LFI v5.6.8 Sovereign Link Established", "[FORENSIC] Mobile Sensorium Active", "[AUDIT] Zero-Trust Protocol Engaged")) }
    var input by remember { mutableStateOf("") }

    Column(modifier = Modifier.padding(16.dp)) {
        // Header
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween
        ) {
            Text(
                text = "LFI // SOVEREIGN_MOBILE",
                color = Color(0xFF3B82F6),
                fontFamily = FontFamily.Monospace,
                style = TextStyle(fontWeight = androidx.compose.ui.text.font.FontWeight.Bold, fontSize = 14.sp)
            )
            Text(
                text = "STATUS: ONLINE",
                color = Color(0xFF10B981),
                fontFamily = FontFamily.Monospace,
                style = TextStyle(fontSize = 10.sp)
            )
        }
        
        Spacer(modifier = Modifier.height(24.dp))
        
        // Output / Log Area
        LazyColumn(
            modifier = Modifier
                .weight(1f)
                .fillMaxWidth()
                .background(Color(0xFF0A0A0A))
                .padding(8.dp)
        ) {
            items(logs) { log ->
                Text(
                    text = log,
                    color = if (log.contains("[AUDIT]")) Color(0xFFF59E0B) else Color(0xFF9CA3AF),
                    fontFamily = FontFamily.Monospace,
                    style = TextStyle(fontSize = 12.sp),
                    modifier = Modifier.padding(bottom = 4.dp)
                )
            }
        }
        
        Spacer(modifier = Modifier.height(16.dp))
        
        // Input Area
        OutlinedTextField(
            value = input,
            onValueChange = { input = it },
            modifier = Modifier.fillMaxWidth(),
            textStyle = TextStyle(color = Color.White, fontFamily = FontFamily.Monospace),
            placeholder = { Text("Direct Directive to Core...", color = Color(0xFF4B5563)) },
            trailingIcon = {
                Button(
                    onClick = {
                        if (input.isNotBlank()) {
                            logs = logs + "> $input"
                            input = ""
                            // Simulate response
                            logs = logs + "[DEBUGLOG] Directive ingested into VSA space."
                        }
                    },
                    colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF3B82F6))
                ) {
                    Text("SEND", color = Color.White, style = TextStyle(fontSize = 10.sp))
                }
            },
            colors = TextFieldDefaults.outlinedTextFieldColors(
                focusedBorderColor = Color(0xFF3B82F6),
                unfocusedBorderColor = Color(0xFF1F2937)
            )
        )
    }
}

@Composable
fun SovereignTheme(content: @Composable () -> Unit) {
    MaterialTheme(content = content)
}
