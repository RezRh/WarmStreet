package com.warmstreet.capabilities

import android.content.Context
import android.content.pm.PackageManager
import androidx.core.content.ContextCompat
import com.warmstreet.shared.*

class CameraHandler(private val context: Context) {

    suspend fun handle(operation: CameraOperation): CameraResult {
        return when (operation) {
            is CameraOperation.CheckPermission -> checkPermission()
            is CameraOperation.GetCapabilities -> getCapabilities()
            is CameraOperation.CancelPending -> CameraResult.Ok(CameraOutput.Cancelled)
            else -> CameraResult.Error(CameraError.Unavailable("Operation not supported in handler directly"))
        }
    }

    private fun checkPermission(): CameraResult {
        val status = if (ContextCompat.checkSelfPermission(context, android.Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED) {
            PermissionStatus.Granted
        } else {
            PermissionStatus.Denied
        }
        return CameraResult.Ok(CameraOutput.PermissionStatus(status))
    }

    private fun getCapabilities(): CameraResult {
        val hasFront = context.packageManager.hasSystemFeature(PackageManager.FEATURE_CAMERA_FRONT)
        val hasBack = context.packageManager.hasSystemFeature(PackageManager.FEATURE_CAMERA)
        val hasFlash = context.packageManager.hasSystemFeature(PackageManager.FEATURE_CAMERA_FLASH)
        
        val capabilities = CameraCapabilities(
            hasFrontCamera = hasFront,
            hasBackCamera = hasBack,
            hasFlash = hasFlash,
            hasTorch = hasFlash,
            supportsHeic = false, // standard android doesn't guarantee heic
            supportsVideo = true,
            maxPhotoResolution = null,
            isSimulator = false, // could check build model
            platform = CameraPlatform.Android
        )
        return CameraResult.Ok(CameraOutput.Capabilities(capabilities))
    }
}
