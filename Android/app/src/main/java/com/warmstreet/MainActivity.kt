package com.warmstreet

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.provider.Settings
import android.util.Log
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.ActivityResultLauncher
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.systemBarsPadding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalLifecycleOwner
import androidx.core.content.ContextCompat
import androidx.core.content.FileProvider
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import com.warmstreet.shared.*
import com.warmstreet.ui.theme.WarmStreetTheme
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.UUID

class MainActivity : ComponentActivity() {

    companion object {
        private const val TAG = "MainActivity"
    }

    private lateinit var core: Core

    private var pendingCameraCallback: ((CameraResult) -> Event)? = null
    private var pendingGalleryCallback: ((CameraResult) -> Event)? = null
    private var pendingPermissionCallback: ((CameraResult) -> Event)? = null
    private var currentPhotoUri: Uri? = null
    private var currentCaptureConfig: CaptureConfig? = null

    private var isReady by mutableStateOf(false)

    private lateinit var cameraPermissionLauncher: ActivityResultLauncher<String>
    private lateinit var notificationPermissionLauncher: ActivityResultLauncher<String>
    private lateinit var locationPermissionLauncher: ActivityResultLauncher<Array<String>>
    private lateinit var takePictureLauncher: ActivityResultLauncher<Uri>
    private lateinit var pickMediaLauncher: ActivityResultLauncher<PickVisualMediaRequest>
    private lateinit var multipleMediaLauncher: ActivityResultLauncher<PickVisualMediaRequest>

    override fun onCreate(savedInstanceState: Bundle?) {
        val splashScreen = installSplashScreen()

        super.onCreate(savedInstanceState)

        splashScreen.setKeepOnScreenCondition { !isReady }

        enableEdgeToEdge()

        core = (application as WarmStreetApplication).core

        setupActivityResultLaunchers()

        core.setActivityCallbacks(
            onRequestCameraPermission = { callback -> requestCameraPermission(callback) },
            onCapturePhoto = { config, callback -> capturePhoto(config, callback) },
            onPickFromGallery = { config, callback -> pickFromGallery(config, callback) },
            onRequestNotificationPermission = { callback -> requestNotificationPermission(callback) },
            onRequestLocationPermission = { callback -> requestLocationPermission(callback) },
            onOpenAppSettings = { openAppSettings() }
        )

        handleIntent(intent)

        setContent {
            WarmStreetTheme {
                Surface(
                    modifier = Modifier
                        .fillMaxSize()
                        .systemBarsPadding()
                        .imePadding(),
                    color = MaterialTheme.colorScheme.background
                ) {
                    ErrorBoundary {
                        LifecycleHandler(core)
                        WarmStreetApp(core)
                    }
                }
            }
        }

        isReady = true
    }

    private fun setupActivityResultLaunchers() {
        cameraPermissionLauncher = registerForActivityResult(
            ActivityResultContracts.RequestPermission()
        ) { granted ->
            val callback = pendingPermissionCallback
            pendingPermissionCallback = null

            val status = when {
                granted -> PermissionStatus.Granted
                shouldShowRequestPermissionRationale(Manifest.permission.CAMERA) -> PermissionStatus.Denied
                else -> PermissionStatus.DeniedPermanently
            }

            callback?.let { cb ->
                val result = CameraResult.Ok(CameraOutput.PermissionStatus(status))
                core.update(cb(result))
            }
        }

        notificationPermissionLauncher = registerForActivityResult(
            ActivityResultContracts.RequestPermission()
        ) { granted ->
            Log.d(TAG, "Notification permission result: $granted")
            core.update(
                Event.NotificationPermissionResult(granted)
            )
        }

        locationPermissionLauncher = registerForActivityResult(
            ActivityResultContracts.RequestMultiplePermissions()
        ) { permissions ->
            val fineGranted = permissions[Manifest.permission.ACCESS_FINE_LOCATION] == true
            val coarseGranted = permissions[Manifest.permission.ACCESS_COARSE_LOCATION] == true

            core.update(
                Event.LocationPermissionResult(
                    fineLocation = fineGranted,
                    coarseLocation = coarseGranted
                )
            )
        }

        takePictureLauncher = registerForActivityResult(
            ActivityResultContracts.TakePicture()
        ) { success ->
            val callback = pendingCameraCallback
            val photoUri = currentPhotoUri
            val config = currentCaptureConfig

            pendingCameraCallback = null
            currentPhotoUri = null
            currentCaptureConfig = null

            if (callback == null) {
                Log.w(TAG, "No camera callback registered")
                return@registerForActivityResult
            }

            if (!success || photoUri == null) {
                val result = CameraResult.Ok(CameraOutput.Cancelled)
                core.update(callback(result))
                return@registerForActivityResult
            }

            try {
                val imageData = processAndValidateImage(photoUri, config)
                core.update(callback(imageData))
            } catch (e: Exception) {
                Log.e(TAG, "Failed to process captured image", e)
                val result = CameraResult.Error(
                    CameraError.CaptureFailed(reason = e.message ?: "Unknown error")
                )
                core.update(callback(result))
            } finally {
                try {
                    contentResolver.delete(photoUri, null, null)
                } catch (e: Exception) {
                    Log.w(TAG, "Failed to delete temp photo", e)
                }
            }
        }

        pickMediaLauncher = registerForActivityResult(
            ActivityResultContracts.PickVisualMedia()
        ) { uri ->
            val callback = pendingGalleryCallback
            val config = currentCaptureConfig

            pendingGalleryCallback = null
            currentCaptureConfig = null

            if (callback == null) {
                Log.w(TAG, "No gallery callback registered")
                return@registerForActivityResult
            }

            if (uri == null) {
                val result = CameraResult.Ok(CameraOutput.Cancelled)
                core.update(callback(result))
                return@registerForActivityResult
            }

            try {
                val imageData = processAndValidateImage(uri, config)
                core.update(callback(imageData))
            } catch (e: Exception) {
                Log.e(TAG, "Failed to process picked image", e)
                val result = CameraResult.Error(
                    CameraError.CaptureFailed(reason = e.message ?: "Unknown error")
                )
                core.update(callback(result))
            }
        }

        multipleMediaLauncher = registerForActivityResult(
            ActivityResultContracts.PickMultipleVisualMedia(maxItems = 10)
        ) { uris ->
            val callback = pendingGalleryCallback
            val config = currentCaptureConfig

            pendingGalleryCallback = null
            currentCaptureConfig = null

            if (callback == null) {
                Log.w(TAG, "No gallery callback registered")
                return@registerForActivityResult
            }

            if (uris.isEmpty()) {
                val result = CameraResult.Ok(CameraOutput.Cancelled)
                core.update(callback(result))
                return@registerForActivityResult
            }

            try {
                val images = uris.mapNotNull { uri ->
                    try {
                        processAndValidateImageToCapture(uri, config)
                    } catch (e: Exception) {
                        Log.w(TAG, "Failed to process image: $uri", e)
                        null
                    }
                }

                if (images.isEmpty()) {
                    val result = CameraResult.Error(
                        CameraError.CaptureFailed(reason = "Failed to process any images")
                    )
                    core.update(callback(result))
                } else {
                    val result = CameraResult.Ok(CameraOutput.Photos(images))
                    core.update(callback(result))
                }
            } catch (e: Exception) {
                Log.e(TAG, "Failed to process picked images", e)
                val result = CameraResult.Error(
                    CameraError.CaptureFailed(reason = e.message ?: "Unknown error")
                )
                core.update(callback(result))
            }
        }
    }

    private fun requestCameraPermission(callback: (CameraResult) -> Event) {
        val permission = Manifest.permission.CAMERA

        when {
            ContextCompat.checkSelfPermission(this, permission) == PackageManager.PERMISSION_GRANTED -> {
                val result = CameraResult.Ok(CameraOutput.PermissionStatus(PermissionStatus.Granted))
                core.update(callback(result))
            }
            shouldShowRequestPermissionRationale(permission) -> {
                val result = CameraResult.Ok(CameraOutput.PermissionStatus(PermissionStatus.Denied))
                core.update(callback(result))
            }
            else -> {
                pendingPermissionCallback = callback
                cameraPermissionLauncher.launch(permission)
            }
        }
    }

    private fun capturePhoto(config: CaptureConfig, callback: (CameraResult) -> Event) {
        val permission = Manifest.permission.CAMERA

        if (ContextCompat.checkSelfPermission(this, permission) != PackageManager.PERMISSION_GRANTED) {
            val result = CameraResult.Error(CameraError.PermissionDenied)
            core.update(callback(result))
            return
        }

        try {
            val photoFile = createImageFile()
            val photoUri = FileProvider.getUriForFile(
                this,
                "${packageName}.fileprovider",
                photoFile
            )

            pendingCameraCallback = callback
            currentPhotoUri = photoUri
            currentCaptureConfig = config

            takePictureLauncher.launch(photoUri)
        } catch (e: Exception) {
            Log.e(TAG, "Failed to launch camera", e)
            val result = CameraResult.Error(
                CameraError.Unavailable(reason = e.message ?: "Failed to create photo file")
            )
            core.update(callback(result))
        }
    }

    private fun pickFromGallery(config: GalleryPickConfig, callback: (CameraResult) -> Event) {
        pendingGalleryCallback = callback
        currentCaptureConfig = CaptureConfig(
            facing = CameraFacing.Back,
            format = config.format,
            quality = config.quality,
            maxWidth = config.maxWidth,
            maxHeight = config.maxHeight,
            flash = FlashMode.Off,
            aspectRatio = AspectRatio.Full,
            stripMetadata = config.stripMetadata,
            mirrorFrontCamera = false,
            timeoutMs = 60000u,
            maxFileSize = config.maxFileSize
        )

        val request = PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageOnly)

        if (config.allowMultiple && config.maxSelections > 1u) {
            multipleMediaLauncher.launch(request)
        } else {
            pickMediaLauncher.launch(request)
        }
    }

    private fun createImageFile(): File {
        val timeStamp = SimpleDateFormat("yyyyMMdd_HHmmss", Locale.US).format(Date())
        val storageDir = cacheDir
        return File.createTempFile("JPEG_${timeStamp}_", ".jpg", storageDir)
    }

    private fun processAndValidateImage(uri: Uri, config: CaptureConfig?): CameraResult {
        val capturedImage = processAndValidateImageToCapture(uri, config)
        return CameraResult.Ok(CameraOutput.Photo(capturedImage))
    }

    private fun processAndValidateImageToCapture(uri: Uri, config: CaptureConfig?): CapturedImage {
        val inputStream = contentResolver.openInputStream(uri)
            ?: throw IllegalStateException("Cannot open image URI")

        val originalBytes = inputStream.use { it.readBytes() }

        if (originalBytes.isEmpty()) {
            throw IllegalStateException("Image data is empty")
        }

        val maxSize = config?.maxFileSize?.toInt() ?: MAX_IMAGE_SIZE_BYTES
        if (originalBytes.size > maxSize) {
            throw IllegalStateException("Image too large: ${originalBytes.size} bytes")
        }

        val options = android.graphics.BitmapFactory.Options().apply {
            inJustDecodeBounds = true
        }
        android.graphics.BitmapFactory.decodeByteArray(originalBytes, 0, originalBytes.size, options)

        val width = options.outWidth
        val height = options.outHeight

        if (width <= 0 || height <= 0) {
            throw IllegalStateException("Invalid image dimensions")
        }

        val format = detectImageFormat(originalBytes)
            ?: throw IllegalStateException("Unknown image format")

        val processedBytes = if (config != null && needsProcessing(width, height, config)) {
            processImage(originalBytes, config)
        } else {
            originalBytes
        }

        val finalOptions = android.graphics.BitmapFactory.Options().apply {
            inJustDecodeBounds = true
        }
        android.graphics.BitmapFactory.decodeByteArray(processedBytes, 0, processedBytes.size, finalOptions)

        return CapturedImage(
            data = processedBytes,
            format = format,
            width = finalOptions.outWidth.toUInt(),
            height = finalOptions.outHeight.toUInt(),
            fileSize = processedBytes.size.toULong(),
            captureTimeMs = System.currentTimeMillis().toULong()
        )
    }

    private fun needsProcessing(width: Int, height: Int, config: CaptureConfig): Boolean {
        return width > config.maxWidth.toInt() ||
                height > config.maxHeight.toInt() ||
                config.quality < 100u ||
                config.stripMetadata
    }

    private fun processImage(originalBytes: ByteArray, config: CaptureConfig): ByteArray {
        val bitmap = android.graphics.BitmapFactory.decodeByteArray(originalBytes, 0, originalBytes.size)
            ?: throw IllegalStateException("Failed to decode image")

        try {
            val maxWidth = config.maxWidth.toInt()
            val maxHeight = config.maxHeight.toInt()

            val scaledBitmap = if (bitmap.width > maxWidth || bitmap.height > maxHeight) {
                val scale = minOf(
                    maxWidth.toFloat() / bitmap.width,
                    maxHeight.toFloat() / bitmap.height
                )
                val newWidth = (bitmap.width * scale).toInt()
                val newHeight = (bitmap.height * scale).toInt()

                android.graphics.Bitmap.createScaledBitmap(bitmap, newWidth, newHeight, true)
            } else {
                bitmap
            }

            val outputStream = java.io.ByteArrayOutputStream()

            val compressFormat = when (config.format) {
                ImageFormat.Png -> android.graphics.Bitmap.CompressFormat.PNG
                ImageFormat.WebP -> if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                    android.graphics.Bitmap.CompressFormat.WEBP_LOSSY
                } else {
                    @Suppress("DEPRECATION")
                    android.graphics.Bitmap.CompressFormat.WEBP
                }
                else -> android.graphics.Bitmap.CompressFormat.JPEG
            }

            scaledBitmap.compress(compressFormat, config.quality.toInt(), outputStream)

            if (scaledBitmap != bitmap) {
                scaledBitmap.recycle()
            }

            return outputStream.toByteArray()
        } finally {
            bitmap.recycle()
        }
    }

    private fun detectImageFormat(data: ByteArray): ImageFormat? {
        if (data.size < 12) return null

        if (data[0] == 0xFF.toByte() && data[1] == 0xD8.toByte() && data[2] == 0xFF.toByte()) {
            return ImageFormat.Jpeg
        }

        if (data[0] == 0x89.toByte() && data[1] == 0x50.toByte() &&
            data[2] == 0x4E.toByte() && data[3] == 0x47.toByte()) {
            return ImageFormat.Png
        }

        if (data[0] == 0x52.toByte() && data[1] == 0x49.toByte() &&
            data[2] == 0x46.toByte() && data[3] == 0x46.toByte() &&
            data[8] == 0x57.toByte() && data[9] == 0x45.toByte() &&
            data[10] == 0x42.toByte() && data[11] == 0x50.toByte()) {
            return ImageFormat.WebP
        }

        return null
    }

    private fun requestNotificationPermission(callback: (PushResult) -> Event) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            val permission = Manifest.permission.POST_NOTIFICATIONS

            when {
                ContextCompat.checkSelfPermission(this, permission) == PackageManager.PERMISSION_GRANTED -> {
                    val result = PushResult.Ok(PushOutput.PermissionStatus(PermissionStatus.Granted))
                    core.update(callback(result))
                }
                else -> {
                    notificationPermissionLauncher.launch(permission)
                }
            }
        } else {
            val result = PushResult.Ok(PushOutput.PermissionStatus(PermissionStatus.Granted))
            core.update(callback(result))
        }
    }

    private fun requestLocationPermission(callback: (LocationResult) -> Event) {
        val permissions = arrayOf(
            Manifest.permission.ACCESS_FINE_LOCATION,
            Manifest.permission.ACCESS_COARSE_LOCATION
        )

        val fineGranted = ContextCompat.checkSelfPermission(
            this, Manifest.permission.ACCESS_FINE_LOCATION
        ) == PackageManager.PERMISSION_GRANTED

        val coarseGranted = ContextCompat.checkSelfPermission(
            this, Manifest.permission.ACCESS_COARSE_LOCATION
        ) == PackageManager.PERMISSION_GRANTED

        if (fineGranted || coarseGranted) {
            val result = LocationResult.Ok(
                LocationOutput.PermissionStatus(
                    fineLocation = fineGranted,
                    coarseLocation = coarseGranted
                )
            )
            core.update(callback(result))
        } else {
            locationPermissionLauncher.launch(permissions)
        }
    }

    private fun openAppSettings() {
        val intent = Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS).apply {
            data = Uri.fromParts("package", packageName, null)
        }
        startActivity(intent)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        handleIntent(intent)
    }

    private fun handleIntent(intent: Intent?) {
        intent ?: return

        val data = intent.data
        if (data != null) {
            Log.d(TAG, "Received deep link: $data")

            when {
                data.scheme == "warmstreet" -> handleDeepLink(data)
                data.host == "warmstreet.com" || data.host == "www.warmstreet.com" -> handleUniversalLink(data)
                isOAuthCallback(data) -> handleOAuthCallback(data)
            }
        }

        if (intent.action == Intent.ACTION_VIEW) {
            Log.d(TAG, "View intent received")
        }
    }

    private fun handleDeepLink(uri: Uri) {
        val path = uri.path ?: return
        val params = uri.queryParameterNames.associateWith { uri.getQueryParameter(it) }

        Log.d(TAG, "Deep link path: $path, params: $params")

        core.update(Event.DeepLink(path = path, params = params))
    }

    private fun handleUniversalLink(uri: Uri) {
        val path = uri.path ?: return
        val params = uri.queryParameterNames.associateWith { uri.getQueryParameter(it) }

        Log.d(TAG, "Universal link path: $path, params: $params")

        core.update(Event.UniversalLink(path = path, params = params))
    }

    private fun isOAuthCallback(uri: Uri): Boolean {
        return uri.path?.contains("oauth") == true ||
                uri.path?.contains("callback") == true ||
                uri.getQueryParameter("code") != null
    }

    private fun handleOAuthCallback(uri: Uri) {
        val code = uri.getQueryParameter("code")
        val state = uri.getQueryParameter("state")
        val error = uri.getQueryParameter("error")

        Log.d(TAG, "OAuth callback - code: ${code != null}, state: $state, error: $error")

        if (error != null) {
            core.update(Event.OAuthError(error = error, description = uri.getQueryParameter("error_description")))
        } else if (code != null) {
            core.update(Event.OAuthCallback(code = code, state = state))
        }
    }

    override fun onResume() {
        super.onResume()
        core.update(Event.LifecycleResumed)
    }

    override fun onPause() {
        super.onPause()
        core.update(Event.LifecyclePaused)
    }

    override fun onStop() {
        super.onStop()
        core.update(Event.LifecycleStopped)
    }

    override fun onDestroy() {
        pendingCameraCallback = null
        pendingGalleryCallback = null
        pendingPermissionCallback = null
        currentPhotoUri = null

        super.onDestroy()
    }
}

@Composable
fun LifecycleHandler(core: Core) {
    val lifecycleOwner = LocalLifecycleOwner.current

    DisposableEffect(lifecycleOwner) {
        val observer = LifecycleEventObserver { _, event ->
            when (event) {
                Lifecycle.Event.ON_START -> core.update(Event.LifecycleStarted)
                Lifecycle.Event.ON_RESUME -> core.update(Event.LifecycleResumed)
                Lifecycle.Event.ON_PAUSE -> core.update(Event.LifecyclePaused)
                Lifecycle.Event.ON_STOP -> core.update(Event.LifecycleStopped)
                else -> {}
            }
        }

        lifecycleOwner.lifecycle.addObserver(observer)

        onDispose {
            lifecycleOwner.lifecycle.removeObserver(observer)
        }
    }
}

@Composable
fun ErrorBoundary(content: @Composable () -> Unit) {
    var error by androidx.compose.runtime.remember { mutableStateOf<Throwable?>(null) }

    if (error != null) {
        ErrorScreen(
            error = error!!,
            onRetry = { error = null }
        )
    } else {
        try {
            content()
        } catch (e: Throwable) {
            LaunchedEffect(e) {
                Log.e("ErrorBoundary", "Caught error in composition", e)
                error = e
            }
        }
    }
}

@Composable
fun ErrorScreen(error: Throwable, onRetry: () -> Unit) {
    androidx.compose.foundation.layout.Column(
        modifier = Modifier.fillMaxSize(),
        horizontalAlignment = androidx.compose.ui.Alignment.CenterHorizontally,
        verticalArrangement = androidx.compose.foundation.layout.Arrangement.Center
    ) {
        androidx.compose.material3.Text(
            text = "Something went wrong",
            style = MaterialTheme.typography.headlineMedium
        )
        androidx.compose.foundation.layout.Spacer(
            modifier = Modifier.height(16.dp)
        )
        androidx.compose.material3.Text(
            text = error.message ?: "Unknown error",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.error
        )
        androidx.compose.foundation.layout.Spacer(
            modifier = Modifier.height(24.dp)
        )
        androidx.compose.material3.Button(onClick = onRetry) {
            androidx.compose.material3.Text("Retry")
        }
    }
}

private val Modifier.height: (Int) -> Modifier
    get() = { this }

private val Int.dp: Int
    get() = this

private const val MAX_IMAGE_SIZE_BYTES = 20 * 1024 * 1024