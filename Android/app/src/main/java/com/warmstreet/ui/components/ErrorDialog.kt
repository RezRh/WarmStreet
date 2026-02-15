package com.warmstreet.ui.components

import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable

@Composable
fun ErrorDialog(
    message: String,
    onDismiss: () -> Unit,
    title: String = "Error",
    onRetry: (() -> Unit)? = null,
    dismissText: String = if (onRetry != null) "Cancel" else "OK",
    retryText: String = "Retry"
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(text = title, style = MaterialTheme.typography.headlineSmall) },
        text = { Text(text = message, style = MaterialTheme.typography.bodyMedium) },
        confirmButton = {
            if (onRetry != null) {
                Button(onClick = onRetry) { Text(retryText) }
            } else {
                Button(onClick = onDismiss) { Text(dismissText) }
            }
        },
        dismissButton = if (onRetry != null) {
            { TextButton(onClick = onDismiss) { Text(dismissText) } }
        } else null
    )
}
