package com.warmstreet.capabilities

import android.content.Context
import android.content.pm.PackageManager
import androidx.core.content.ContextCompat
import com.google.firebase.messaging.FirebaseMessaging
import com.warmstreet.shared.*
import kotlinx.coroutines.tasks.await

class PushHandler(private val context: Context) {

    suspend fun handle(operation: PushOperation): PushResult {
        return when (operation) {
            is PushOperation.CheckPermission -> checkPermission()
            is PushOperation.RequestToken -> requestToken()
            else -> PushResult.Error(PushError.NotSupported)
        }
    }

    private fun checkPermission(): PushResult {
        val status = if (ContextCompat.checkSelfPermission(context, android.Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED) {
            PermissionStatus.Granted
        } else {
            PermissionStatus.Denied
        }
        return PushResult.Ok(PushOutput.PermissionStatus(status))
    }

    private suspend fun requestToken(): PushResult {
        return try {
            val token = FirebaseMessaging.getInstance().token.await()
            PushResult.Ok(PushOutput.Token(token))
        } catch (e: Exception) {
            PushResult.Error(PushError.RegistrationFailed(e.message ?: "Failed to get token"))
        }
    }
}
