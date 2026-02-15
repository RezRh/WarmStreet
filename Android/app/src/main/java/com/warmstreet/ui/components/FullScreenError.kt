package com.warmstreet.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CloudOff
import androidx.compose.material.icons.filled.Error
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.Button
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp

enum class ErrorType { GENERAL, NETWORK, SERVER }

@Composable
fun FullScreenError(
    title: String,
    message: String,
    isRetryable: Boolean,
    onRetry: () -> Unit,
    onBack: (() -> Unit)? = null,
    errorType: ErrorType = ErrorType.GENERAL,
    modifier: Modifier = Modifier
) {
    val icon = when (errorType) {
        ErrorType.NETWORK -> Icons.Filled.CloudOff
        ErrorType.SERVER -> Icons.Filled.Error
        ErrorType.GENERAL -> Icons.Filled.Warning
    }

    Column(
        modifier = modifier
            .fillMaxSize()
            .padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        Icon(
            imageVector = icon,
            contentDescription = null,
            modifier = Modifier.size(72.dp),
            tint = MaterialTheme.colorScheme.error
        )

        Spacer(modifier = Modifier.height(24.dp))

        Text(
            text = title,
            style = MaterialTheme.typography.headlineSmall,
            textAlign = TextAlign.Center
        )

        Spacer(modifier = Modifier.height(12.dp))

        Text(
            text = message,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center
        )

        Spacer(modifier = Modifier.height(32.dp))

        if (isRetryable) {
            Button(
                onClick = onRetry,
                modifier = Modifier.fillMaxWidth(0.6f)
            ) {
                Text("Try Again")
            }

            if (onBack != null) {
                Spacer(modifier = Modifier.height(12.dp))
                OutlinedButton(
                    onClick = onBack,
                    modifier = Modifier.fillMaxWidth(0.6f)
                ) {
                    Text("Go Back")
                }
            }
        } else if (onBack != null) {
            Button(
                onClick = onBack,
                modifier = Modifier.fillMaxWidth(0.6f)
            ) {
                Text("Go Back")
            }
        }
    }
}
