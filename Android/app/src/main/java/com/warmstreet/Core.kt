package com.warmstreet

import android.app.Application
import android.util.Log
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.warmstreet.capabilities.*
import com.warmstreet.shared.*
import kotlinx.coroutines.*
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import java.util.concurrent.atomic.AtomicBoolean

class Core(application: Application) : AndroidViewModel(application) {

    companion object {
        private const val TAG = "WarmStreetCore"
        private val libraryLoaded = AtomicBoolean(false)
        private val _isLibraryLoaded = kotlinx.coroutines.flow.MutableStateFlow(false)

        suspend fun ensureLibraryLoaded() {
            if (libraryLoaded.compareAndSet(false, true)) {
                withContext(Dispatchers.IO) {
                    try {
                        System.loadLibrary("warmstreet")
                        Log.i(TAG, "Native library loaded successfully")
                        _isLibraryLoaded.value = true
                    } catch (e: UnsatisfiedLinkError) {
                        Log.e(TAG, "Failed to load native library", e)
                        // In production we might want to recover or show fatal error
                        throw RuntimeException("Failed to load warmstreet native library", e)
                    }
                }
            } else {
                 // waits until loaded if another thread is loading?
                 // Simple check for now
            }
        }
    }

    private lateinit var app: App
    private val coreMutex = Mutex()
    private val effectChannel = Channel<Effect>(Channel.UNLIMITED)
    private var effectProcessorJob: Job? = null

    // Replaced ActivityCallbacks with Channel
    sealed class CoreCommand {
        data class RequestCameraPermission(val callback: (CameraResult) -> Event) : CoreCommand()
        data class CapturePhoto(val config: CaptureConfig, val callback: (CameraResult) -> Event) : CoreCommand()
        data class PickFromGallery(val config: GalleryPickConfig, val callback: (CameraResult) -> Event) : CoreCommand()
        data class RequestNotificationPermission(val callback: (PushResult) -> Event) : CoreCommand()
        data class RequestLocationPermission(val callback: (LocationResult) -> Event) : CoreCommand()
        object OpenAppSettings : CoreCommand()
    }

    private val _commands = Channel<CoreCommand>(Channel.BUFFERED)
    val commands = _commands.receiveAsFlow()

    var view: ViewModel by mutableStateOf(ViewModel.Loading)
        private set

    private val httpHandler: HttpHandler
    private val keyValueHandler: KeyValueHandler
    private val locationHandler: LocationHandler
    private val cameraHandler: CameraHandler
    private val cryptoHandler: CryptoHandler
    private val pushHandler: PushHandler

    private val isProcessing = AtomicBoolean(false)

    // Callback definition removed in favor of CoreCommand

    init {
        val context = application.applicationContext

        httpHandler = HttpHandler()
        keyValueHandler = KeyValueHandler(context)
        locationHandler = LocationHandler(context)
        cameraHandler = CameraHandler(context)
        cryptoHandler = CryptoHandler()
        pushHandler = PushHandler(context)

        // Initialize App on background thread
        viewModelScope.launch(Dispatchers.IO) {
            ensureLibraryLoaded()
            
            try {
                // Initialize Rust Core
                app = App()
                
                // Get initial view
                val initialView = app.view()
                
                withContext(Dispatchers.Main) {
                   view = initialView
                }
            } catch (e: Exception) {
                Log.e(TAG, "Failed to initialize Crux app asynchronously", e)
                withContext(Dispatchers.Main) {
                    view = ViewModel.Error("Failed to initialize system")
                }
            }
        }
        
        // Lateinit var app must be handled carefully. 
        // We need to change `private val app: App` to `private lateinit var app: App` 
        // or make it nullable / atomic ref.
        // But since we can't change the field definition in this chunk easily without context,
        // we'll assume we can change the field definition in another chunk or rely on it being initialized before use?
        // Wait, `private val app: App` needs immediate initialization in init block or it must be lateinit.
        // The original code was `private val app: App`.
        // I need to change line 38 as well.
        
        startEffectProcessor()
    }

    // Removed setActivityCallbacks

    private fun startEffectProcessor() {
        effectProcessorJob = viewModelScope.launch(Dispatchers.Default) {
            effectChannel.receiveAsFlow().collect { effect ->
                try {
                    processEffectSafely(effect)
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    Log.e(TAG, "Unhandled error processing effect: $effect", e)
                    sendErrorToCore(e)
                }
            }
        }
    }

    fun update(event: Event) {
        viewModelScope.launch(Dispatchers.Main.immediate) {
            updateInternal(event)
        }
    }

    private suspend fun updateInternal(event: Event) {
        val effects = coreMutex.withLock {
            try {
                if (!::app.isInitialized) {
                     Log.w(TAG, "App not initialized yet, ignoring update")
                     return@withLock emptyList()
                }
                val effects = app.update(event)
                updateView()
                effects
            } catch (e: Exception) {
                Log.e(TAG, "Error in app.update for event: $event", e)
                emptyList()
            }
        }

        for (effect in effects) {
            effectChannel.send(effect)
        }
    }

    private fun updateView() {
        try {
            if (!::app.isInitialized) return
            view = app.view()
        } catch (e: Exception) {
            Log.e(TAG, "Error getting view", e)
        }
    }

    private suspend fun processEffectSafely(effect: Effect) {
        when (effect) {
            is Effect.Render -> {
                withContext(Dispatchers.Main) {
                    updateView()
                }
            }

            is Effect.Http -> {
                processHttpEffect(effect)
            }

            is Effect.Kv -> {
                processKvEffect(effect)
            }

            is Effect.Crypto -> {
                processCryptoEffect(effect)
            }

            is Effect.Camera -> {
                processCameraEffect(effect)
            }

            is Effect.Push -> {
                processPushEffect(effect)
            }

            is Effect.Location -> {
                processLocationEffect(effect)
            }

            else -> {
                Log.w(TAG, "Unhandled effect type: ${effect::class.simpleName}")
            }
        }
    }

    private suspend fun processHttpEffect(effect: Effect.Http) {
        val result = try {
            httpHandler.handle(effect.operation)
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            Log.e(TAG, "HTTP request failed", e)
            HttpResult.Error(
                HttpError.Network(
                    message = e.message ?: "Unknown error",
                    status = null
                )
            )
        }

        val responseEvent = effect.callback(result)
        updateInternal(responseEvent)
    }

    private suspend fun processKvEffect(effect: Effect.Kv) {
        val result = try {
            keyValueHandler.handle(effect.operation)
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            Log.e(TAG, "KV operation failed", e)
            KvResult.Error(
                KvError.Storage(
                    code = StorageErrorCode.IoError,
                    message = e.message ?: "Unknown error",
                    retryable = false
                )
            )
        }

        val responseEvent = effect.callback(result)
        updateInternal(responseEvent)
    }

    private suspend fun processCryptoEffect(effect: Effect.Crypto) {
        val result = try {
            cryptoHandler.handle(effect.operation)
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            Log.e(TAG, "Crypto operation failed", e)
            CryptoResult.Error(
                CryptoError.Internal(message = e.message ?: "Unknown error")
            )
        }

        val responseEvent = effect.callback(result)
        updateInternal(responseEvent)
    }

    private suspend fun processCameraEffect(effect: Effect.Camera) {

        when (effect.operation) {
            is CameraOperation.RequestPermission -> {
                _commands.send(CoreCommand.RequestCameraPermission(effect.callback))
            }

            is CameraOperation.CapturePhoto -> {
                _commands.send(CoreCommand.CapturePhoto(effect.operation.config, effect.callback))
            }

            is CameraOperation.PickFromGallery -> {
                _commands.send(CoreCommand.PickFromGallery(effect.operation.config, effect.callback))
            }

            else -> {
                val result = try {
                    cameraHandler.handle(effect.operation)
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    Log.e(TAG, "Camera operation failed", e)
                    CameraResult.Error(
                        CameraError.Internal(message = e.message ?: "Unknown error")
                    )
                }

                val responseEvent = effect.callback(result)
                updateInternal(responseEvent)
            }
        }
    }

    private suspend fun processPushEffect(effect: Effect.Push) {
        if (effect.operation is PushOperation.RequestPermission) {
            _commands.send(CoreCommand.RequestNotificationPermission(effect.callback))
            return
        }

        val result = try {
            pushHandler.handle(effect.operation)
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            Log.e(TAG, "Push operation failed", e)
            PushResult.Error(
                PushError.Internal(message = e.message ?: "Unknown error")
            )
        }

        val responseEvent = effect.callback(result)
        updateInternal(responseEvent)
    }

    private suspend fun processLocationEffect(effect: Effect.Location) {
        if (effect.operation is LocationOperation.RequestPermission) {
            _commands.send(CoreCommand.RequestLocationPermission(effect.callback))
            return
        }

        val result = try {
            locationHandler.handle(effect.operation)
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            Log.e(TAG, "Location operation failed", e)
            LocationResult.Error(
                LocationError.Internal(message = e.message ?: "Unknown error")
            )
        }

        val responseEvent = effect.callback(result)
        updateInternal(responseEvent)
    }

    private suspend fun sendErrorToCore(error: Exception) {
        val errorEvent = Event.SystemError(
            message = error.message ?: "Unknown error",
            source = error::class.simpleName ?: "Unknown"
        )
        updateInternal(errorEvent)
    }

    fun sendEvent(event: Event) {
        update(event)
    }

    override fun onCleared() {
        super.onCleared()

        effectProcessorJob?.cancel()
        effectChannel.close()

        try {
            httpHandler.close()
            keyValueHandler.close()
            locationHandler.close()
            cameraHandler.close()
        } catch (e: Exception) {
            Log.w(TAG, "Error during cleanup", e)
        }

        Log.i(TAG, "Core ViewModel cleared")
    }
}

sealed class ViewModelState {
    object Loading : ViewModelState()
    data class Ready(val model: ViewModel) : ViewModelState()
    data class Error(val message: String) : ViewModelState()
}

interface EffectHandler<Op, Result> {
    suspend fun handle(operation: Op): Result
    fun close() {}
}

class HttpHandler : EffectHandler<HttpOperation, HttpResult> {

    private val client = okhttp3.OkHttpClient.Builder()
        .connectTimeout(30, java.util.concurrent.TimeUnit.SECONDS)
        .readTimeout(30, java.util.concurrent.TimeUnit.SECONDS)
        .writeTimeout(30, java.util.concurrent.TimeUnit.SECONDS)
        .followRedirects(true)
        .followSslRedirects(true)
        .build()

    override suspend fun handle(operation: HttpOperation): HttpResult {
        return withContext(Dispatchers.IO) {
            when (operation) {
                is HttpOperation.Execute -> executeRequest(operation.request)
            }
        }
    }

    private fun executeRequest(request: HttpRequest): HttpResult {
        val url = try {
            okhttp3.HttpUrl.Builder()
                .parse(request.url.asStr())
                .build()
        } catch (e: Exception) {
            return HttpResult.Error(
                HttpError.InvalidUrl(
                    url = request.url.asStr(),
                    reason = e.message ?: "Invalid URL"
                )
            )
        }

        val requestBuilder = okhttp3.Request.Builder().url(url)

        for ((name, value) in request.headers.iter()) {
            requestBuilder.addHeader(name, value)
        }

        val body = request.body?.let { bytes ->
            val contentType = request.headers.get("Content-Type")
                ?.let { okhttp3.MediaType.parse(it) }
                ?: okhttp3.MediaType.parse("application/octet-stream")
            okhttp3.RequestBody.create(contentType, bytes)
        }

        when (request.method) {
            HttpMethod.Get -> requestBuilder.get()
            HttpMethod.Post -> requestBuilder.post(body ?: okhttp3.RequestBody.create(null, ByteArray(0)))
            HttpMethod.Put -> requestBuilder.put(body ?: okhttp3.RequestBody.create(null, ByteArray(0)))
            HttpMethod.Patch -> requestBuilder.patch(body ?: okhttp3.RequestBody.create(null, ByteArray(0)))
            HttpMethod.Delete -> if (body != null) requestBuilder.delete(body) else requestBuilder.delete()
            HttpMethod.Head -> requestBuilder.head()
            HttpMethod.Options -> requestBuilder.method("OPTIONS", null)
        }

        val startTime = System.currentTimeMillis()

        return try {
            val call = client.newCall(requestBuilder.build())
            val response = call.execute()
            val duration = System.currentTimeMillis() - startTime

            val responseHeaders = HttpHeaders()
            for ((name, value) in response.headers()) {
                try {
                    responseHeaders.insert(name, value)
                } catch (e: Exception) {
                }
            }

            val responseBody = response.body()?.bytes() ?: ByteArray(0)

            if (responseBody.size > request.maxResponseSize) {
                return HttpResult.Error(
                    HttpError.ResponseTooLarge(
                        size = responseBody.size,
                        max = request.maxResponseSize
                    )
                )
            }

            HttpResult.Ok(
                HttpResponse(
                    status = response.code().toUShort(),
                    headers = responseHeaders,
                    body = responseBody,
                    requestId = request.requestId,
                    durationMs = duration.toULong()
                )
            )
        } catch (e: java.net.SocketTimeoutException) {
            HttpResult.Error(
                HttpError.Timeout(
                    timeoutMs = request.timeoutMs,
                    requestId = request.requestId
                )
            )
        } catch (e: java.net.UnknownHostException) {
            HttpResult.Error(
                HttpError.DnsError(
                    host = url.host(),
                    message = e.message ?: "Unknown host"
                )
            )
        } catch (e: javax.net.ssl.SSLException) {
            HttpResult.Error(
                HttpError.TlsError(
                    host = url.host(),
                    message = e.message ?: "TLS error"
                )
            )
        } catch (e: java.io.IOException) {
            HttpResult.Error(
                HttpError.ConnectionError(
                    host = url.host(),
                    message = e.message ?: "Connection error"
                )
            )
        }
    }

    override fun close() {
        client.dispatcher().executorService().shutdown()
        client.connectionPool().evictAll()
    }
}

class KeyValueHandler(private val context: android.content.Context) : EffectHandler<KvOperation, KvResult> {

    private val prefs = context.getSharedPreferences("warmstreet_kv", android.content.Context.MODE_PRIVATE)
    private val mutex = Mutex()

    override suspend fun handle(operation: KvOperation): KvResult {
        return mutex.withLock {
            when (operation) {
                is KvOperation.Get -> get(operation.key)
                is KvOperation.Set -> set(operation.key, operation.value, operation.ifVersion)
                is KvOperation.Delete -> delete(operation.key, operation.ifVersion)
                is KvOperation.Exists -> exists(operation.key)
                is KvOperation.List -> list(operation.namespace, operation.prefix, operation.limit, operation.cursor)
                is KvOperation.GetMulti -> getMulti(operation.keys)
                is KvOperation.DeleteMulti -> deleteMulti(operation.keys)
            }
        }
    }

    private fun get(key: KvKey): KvResult {
        val raw = prefs.getString(key.raw(), null)
            ?: return KvResult.Ok(KvOutput.Value(null))

        return try {
            val value = decodeValue(raw)
            KvResult.Ok(KvOutput.Value(value))
        } catch (e: Exception) {
            KvResult.Error(
                KvError.Storage(
                    code = StorageErrorCode.Corrupted,
                    message = "Failed to decode value: ${e.message}",
                    retryable = false
                )
            )
        }
    }

    private fun set(key: KvKey, value: ByteArray, ifVersion: Long?): KvResult {
        val raw = key.raw()

        if (ifVersion != null) {
            val existing = prefs.getString(raw, null)
            val currentVersion = existing?.let { decodeValue(it)?.version } ?: 0L

            if (currentVersion != ifVersion) {
                return KvResult.Error(
                    KvError.VersionMismatch(
                        expected = ifVersion.toULong(),
                        found = currentVersion.toULong()
                    )
                )
            }
        }

        val now = System.currentTimeMillis()
        val existingValue = prefs.getString(raw, null)?.let { decodeValue(it) }
        val newVersion = (existingValue?.version ?: 0L) + 1

        val kvValue = KvValue(
            data = value,
            version = newVersion.toULong(),
            createdAt = existingValue?.createdAt ?: now.toULong(),
            updatedAt = now.toULong()
        )

        val encoded = encodeValue(kvValue)
        prefs.edit().putString(raw, encoded).apply()

        return KvResult.Ok(KvOutput.Written(version = newVersion.toULong()))
    }

    private fun delete(key: KvKey, ifVersion: Long?): KvResult {
        val raw = key.raw()

        if (ifVersion != null) {
            val existing = prefs.getString(raw, null)
            val currentVersion = existing?.let { decodeValue(it)?.version } ?: 0L

            if (currentVersion != ifVersion) {
                return KvResult.Error(
                    KvError.VersionMismatch(
                        expected = ifVersion.toULong(),
                        found = currentVersion.toULong()
                    )
                )
            }
        }

        val existed = prefs.contains(raw)
        prefs.edit().remove(raw).apply()

        return KvResult.Ok(KvOutput.Deleted(existed = existed))
    }

    private fun exists(key: KvKey): KvResult {
        val exists = prefs.contains(key.raw())
        return KvResult.Ok(KvOutput.Exists(exists))
    }

    private fun list(namespace: KeyNamespace, prefix: String?, limit: UInt, cursor: String?): KvResult {
        val namespacePrefix = "${namespace.prefix()}:"
        val fullPrefix = prefix?.let { "$namespacePrefix$it" } ?: namespacePrefix

        val allKeys = prefs.all.keys
            .filter { it.startsWith(fullPrefix) }
            .sorted()

        val startIndex = cursor?.let { c ->
            allKeys.indexOfFirst { it > c }.takeIf { it >= 0 } ?: allKeys.size
        } ?: 0

        val entries = allKeys
            .drop(startIndex)
            .take(limit.toInt())
            .mapNotNull { key ->
                val value = prefs.getString(key, null)?.let { decodeValue(it) }
                value?.let {
                    KvListEntry(
                        key = key.removePrefix(namespacePrefix),
                        version = it.version,
                        size = it.data.size.toULong(),
                        updatedAt = it.updatedAt
                    )
                }
            }

        val hasMore = startIndex + entries.size < allKeys.size
        val nextCursor = if (hasMore) entries.lastOrNull()?.key else null

        return KvResult.Ok(
            KvOutput.List(
                entries = entries,
                nextCursor = nextCursor,
                hasMore = hasMore
            )
        )
    }

    private fun getMulti(keys: List<KvKey>): KvResult {
        val values = keys.map { key ->
            prefs.getString(key.raw(), null)?.let { decodeValue(it) }
        }
        return KvResult.Ok(KvOutput.Multi(values))
    }

    private fun deleteMulti(keys: List<KvKey>): KvResult {
        val editor = prefs.edit()
        var deletedCount = 0

        for (key in keys) {
            if (prefs.contains(key.raw())) {
                editor.remove(key.raw())
                deletedCount++
            }
        }

        editor.apply()
        return KvResult.Ok(KvOutput.DeletedMulti(deletedCount = deletedCount.toULong()))
    }

    private fun encodeValue(value: KvValue): String {
        val json = org.json.JSONObject().apply {
            put("data", android.util.Base64.encodeToString(value.data, android.util.Base64.NO_WRAP))
            put("version", value.version.toLong())
            put("createdAt", value.createdAt.toLong())
            put("updatedAt", value.updatedAt.toLong())
        }
        return json.toString()
    }

    private fun decodeValue(encoded: String): KvValue? {
        return try {
            val json = org.json.JSONObject(encoded)
            KvValue(
                data = android.util.Base64.decode(json.getString("data"), android.util.Base64.NO_WRAP),
                version = json.getLong("version").toULong(),
                createdAt = json.getLong("createdAt").toULong(),
                updatedAt = json.getLong("updatedAt").toULong()
            )
        } catch (e: Exception) {
            null
        }
    }

    override fun close() {
    }
}

class LocationHandler(private val context: android.content.Context) : EffectHandler<LocationOperation, LocationResult> {

    override suspend fun handle(operation: LocationOperation): LocationResult {
        return LocationResult.Error(
            LocationError.NotSupported
        )
    }

    override fun close() {
    }
}

class CameraHandler(private val context: android.content.Context) : EffectHandler<CameraOperation, CameraResult> {

    override suspend fun handle(operation: CameraOperation): CameraResult {
        return when (operation) {
            is CameraOperation.CheckPermission -> checkPermission()
            is CameraOperation.RequestPermission -> {
                CameraResult.Error(
                    CameraError.Internal(
                        message = "Permission request must be handled by Activity"
                    )
                )
            }
            is CameraOperation.GetCapabilities -> getCapabilities()
            is CameraOperation.CapturePhoto -> {
                CameraResult.Error(
                    CameraError.Internal(
                        message = "Photo capture must be handled by Activity"
                    )
                )
            }
            is CameraOperation.PickFromGallery -> {
                CameraResult.Error(
                    CameraError.Internal(
                        message = "Gallery pick must be handled by Activity"
                    )
                )
            }
            is CameraOperation.CancelPending -> {
                CameraResult.Ok(CameraOutput.Cancelled)
            }
        }
    }

    private fun checkPermission(): CameraResult {
        val permission = android.Manifest.permission.CAMERA
        val status = when (androidx.core.content.ContextCompat.checkSelfPermission(context, permission)) {
            android.content.pm.PackageManager.PERMISSION_GRANTED -> PermissionStatus.Granted
            else -> PermissionStatus.NotDetermined
        }
        return CameraResult.Ok(CameraOutput.PermissionStatus(status))
    }

    private fun getCapabilities(): CameraResult {
        val cameraManager = context.getSystemService(android.content.Context.CAMERA_SERVICE) as android.hardware.camera2.CameraManager

        var hasFront = false
        var hasBack = false
        var hasFlash = false

        try {
            for (cameraId in cameraManager.cameraIdList) {
                val characteristics = cameraManager.getCameraCharacteristics(cameraId)
                val facing = characteristics.get(android.hardware.camera2.CameraCharacteristics.LENS_FACING)

                when (facing) {
                    android.hardware.camera2.CameraCharacteristics.LENS_FACING_FRONT -> hasFront = true
                    android.hardware.camera2.CameraCharacteristics.LENS_FACING_BACK -> hasBack = true
                }

                val flashAvailable = characteristics.get(android.hardware.camera2.CameraCharacteristics.FLASH_INFO_AVAILABLE)
                if (flashAvailable == true) hasFlash = true
            }
        } catch (e: Exception) {
            return CameraResult.Error(
                CameraError.Unavailable(reason = e.message ?: "Failed to query cameras")
            )
        }

        val isEmulator = android.os.Build.FINGERPRINT.contains("generic") ||
                android.os.Build.FINGERPRINT.contains("emulator") ||
                android.os.Build.MODEL.contains("Emulator") ||
                android.os.Build.MODEL.contains("Android SDK")

        return CameraResult.Ok(
            CameraOutput.Capabilities(
                CameraCapabilities(
                    hasFrontCamera = hasFront,
                    hasBackCamera = hasBack,
                    hasFlash = hasFlash,
                    hasTorch = hasFlash,
                    supportsHeic = android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.Q,
                    supportsVideo = true,
                    maxPhotoResolution = null,
                    isSimulator = isEmulator,
                    platform = CameraPlatform.Android
                )
            )
        )
    }

    override fun close() {
    }
}

class CryptoHandler : EffectHandler<CryptoOperation, CryptoResult> {

    override suspend fun handle(operation: CryptoOperation): CryptoResult {
        return withContext(Dispatchers.Default) {
            when (operation) {
                is CryptoOperation.Hash -> hash(operation.algorithm, operation.data)
                is CryptoOperation.GenerateKey -> generateKey(operation.algorithm)
                is CryptoOperation.RandomBytes -> randomBytes(operation.length)
                else -> CryptoResult.Error(
                    CryptoError.NotSupported(operation = operation::class.simpleName ?: "Unknown")
                )
            }
        }
    }

    private fun hash(algorithm: HashAlgorithm, data: ByteArray): CryptoResult {
        val algorithmName = when (algorithm) {
            HashAlgorithm.Sha256 -> "SHA-256"
            HashAlgorithm.Sha384 -> "SHA-384"
            HashAlgorithm.Sha512 -> "SHA-512"
        }

        return try {
            val digest = java.security.MessageDigest.getInstance(algorithmName)
            val hash = digest.digest(data)
            CryptoResult.Ok(CryptoOutput.Hash(hash))
        } catch (e: Exception) {
            CryptoResult.Error(
                CryptoError.Internal(message = e.message ?: "Hash failed")
            )
        }
    }

    private fun generateKey(algorithm: KeyAlgorithm): CryptoResult {
        return try {
            val keyGen = when (algorithm) {
                KeyAlgorithm.Aes256 -> {
                    javax.crypto.KeyGenerator.getInstance("AES").apply {
                        init(256, java.security.SecureRandom())
                    }
                }
            }
            val key = keyGen.generateKey()
            CryptoResult.Ok(CryptoOutput.Key(key.encoded))
        } catch (e: Exception) {
            CryptoResult.Error(
                CryptoError.Internal(message = e.message ?: "Key generation failed")
            )
        }
    }

    private fun randomBytes(length: UInt): CryptoResult {
        return try {
            val bytes = ByteArray(length.toInt())
            java.security.SecureRandom().nextBytes(bytes)
            CryptoResult.Ok(CryptoOutput.RandomBytes(bytes))
        } catch (e: Exception) {
            CryptoResult.Error(
                CryptoError.Internal(message = e.message ?: "Random generation failed")
            )
        }
    }

    override fun close() {
    }
}

class PushHandler(private val context: android.content.Context) : EffectHandler<PushOperation, PushResult> {

    override suspend fun handle(operation: PushOperation): PushResult {
        return when (operation) {
            is PushOperation.RequestToken -> requestToken()
            is PushOperation.CheckPermission -> checkPermission()
            else -> PushResult.Error(
                PushError.NotSupported
            )
        }
    }

    private suspend fun requestToken(): PushResult {
        return try {
            val token = kotlinx.coroutines.tasks.await(
                com.google.firebase.messaging.FirebaseMessaging.getInstance().token
            )
            PushResult.Ok(PushOutput.Token(token))
        } catch (e: Exception) {
            PushResult.Error(
                PushError.RegistrationFailed(message = e.message ?: "Failed to get FCM token")
            )
        }
    }

    private fun checkPermission(): PushResult {
        val enabled = if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.TIRAMISU) {
            androidx.core.content.ContextCompat.checkSelfPermission(
                context,
                android.Manifest.permission.POST_NOTIFICATIONS
            ) == android.content.pm.PackageManager.PERMISSION_GRANTED
        } else {
            androidx.core.app.NotificationManagerCompat.from(context).areNotificationsEnabled()
        }

        return PushResult.Ok(
            PushOutput.PermissionStatus(
                if (enabled) PermissionStatus.Granted else PermissionStatus.Denied
            )
        )
    }

    override fun close() {
    }
}
