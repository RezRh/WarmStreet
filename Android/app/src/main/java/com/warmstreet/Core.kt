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
import okhttp3.HttpUrl.Companion.toHttpUrlOrNull
import okhttp3.MediaType.Companion.toMediaTypeOrNull
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
                        throw RuntimeException("Failed to load warmstreet native library", e)
                    }
                }
            }
        }
    }

    private lateinit var app: App
    private val coreMutex = Mutex()
    private val effectChannel = Channel<Effect>(Channel.UNLIMITED)
    private var effectProcessorJob: Job? = null

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

    init {
        val context = application.applicationContext

        httpHandler = HttpHandler()
        keyValueHandler = KeyValueHandler(context)
        locationHandler = LocationHandler(context)
        cameraHandler = CameraHandler(context)
        cryptoHandler = CryptoHandler()
        pushHandler = PushHandler(context)

        viewModelScope.launch(Dispatchers.IO) {
            ensureLibraryLoaded()
            
            try {
                app = App()
                val initialView = app.view()
                
                withContext(Dispatchers.Main) {
                   view = initialView
                }
            } catch (e: Exception) {
                Log.e(TAG, "Failed to initialize app asynchronously", e)
                withContext(Dispatchers.Main) {
                    view = ViewModel.Error("Failed to initialize system")
                }
            }
        }
        
        startEffectProcessor()
    }

    // Lifecycle methods called by Application class
    fun onUnhandledException(throwable: Throwable) {
        Log.e(TAG, "Unhandled exception in application", throwable)
        update(Event.SystemError(throwable.message ?: "Unknown crash", "UncaughtException"))
    }

    fun onAppForeground() {
        update(Event.LifecycleResumed)
    }

    fun onAppBackground() {
        update(Event.LifecyclePaused)
    }

    fun onLowMemory() {
        Log.w(TAG, "Low memory reported")
    }

    fun onTrimMemory(level: Int) {
        Log.w(TAG, "Trim memory reported: $level")
    }

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
            is Effect.Http -> processHttpEffect(effect)
            is Effect.Kv -> processKvEffect(effect)
            is Effect.Crypto -> processCryptoEffect(effect)
            is Effect.Camera -> processCameraEffect(effect)
            is Effect.Push -> processPushEffect(effect)
            is Effect.Location -> processLocationEffect(effect)
            else -> Log.w(TAG, "Unhandled effect type: ${effect::class.simpleName}")
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
        updateInternal(effect.callback(result))
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
        updateInternal(effect.callback(result))
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
        updateInternal(effect.callback(result))
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
                updateInternal(effect.callback(result))
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
        updateInternal(effect.callback(result))
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
        updateInternal(effect.callback(result))
    }

    private suspend fun sendErrorToCore(error: Exception) {
        updateInternal(Event.SystemError(
            message = error.message ?: "Unknown error",
            source = error::class.simpleName ?: "Unknown"
        ))
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
    }
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
        .build()

    override suspend fun handle(operation: HttpOperation): HttpResult {
        return withContext(Dispatchers.IO) {
            when (operation) {
                is HttpOperation.Execute -> executeRequest(operation.request)
            }
        }
    }

    private fun executeRequest(request: HttpRequest): HttpResult {
        val url = request.url.asStr().toHttpUrlOrNull()
            ?: return HttpResult.Error(HttpError.InvalidUrl(request.url.asStr(), "Invalid URL"))
        val requestBuilder = okhttp3.Request.Builder().url(url)

        for ((name, value) in request.headers.iter()) {
            requestBuilder.addHeader(name, value)
        }

        val body = request.body?.let { bytes ->
            val contentType = request.headers.get("Content-Type")?.toMediaTypeOrNull()
                ?: "application/octet-stream".toMediaTypeOrNull()
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
            val response = client.newCall(requestBuilder.build()).execute()
            val duration = System.currentTimeMillis() - startTime
            val responseHeaders = HttpHeaders()
            for ((name, value) in response.headers) {
                try { responseHeaders.insert(name, value) } catch (e: Exception) {}
            }
            val responseBody = response.body?.bytes() ?: ByteArray(0)
            HttpResult.Ok(
                HttpResponse(
                    status = response.code.toUShort(),
                    headers = responseHeaders,
                    body = responseBody,
                    requestId = request.requestId,
                    durationMs = duration.toULong()
                )
            )
        } catch (e: Exception) {
            HttpResult.Error(HttpError.Network(message = e.message ?: "Network error", status = null))
        }
    }

    override fun close() {
        client.dispatcher.executorService.shutdown()
        client.connectionPool.evictAll()
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
        val raw = prefs.getString(key.raw(), null) ?: return KvResult.Ok(KvOutput.Value(null))
        return decodeValue(raw)?.let { KvResult.Ok(KvOutput.Value(it)) } 
            ?: KvResult.Error(KvError.Storage(StorageErrorCode.Corrupted, "Decode failed", false))
    }

    private fun set(key: KvKey, value: ByteArray, ifVersion: Long?): KvResult {
        val raw = key.raw()
        if (ifVersion != null) {
            val existing = prefs.getString(raw, null)
            val currentVersion = existing?.let { decodeValue(it)?.version?.toLong() } ?: 0L
            if (currentVersion != ifVersion) {
                return KvResult.Error(KvError.VersionMismatch(ifVersion.toULong(), currentVersion.toULong()))
            }
        }
        val existingValue = prefs.getString(raw, null)?.let { decodeValue(it) }
        val newVersion = (existingValue?.version ?: 0uL) + 1uL
        val now = System.currentTimeMillis().toULong()
        val kvValue = KvValue(value, newVersion, existingValue?.createdAt ?: now, now)
        prefs.edit().putString(raw, encodeValue(kvValue)).apply()
        return KvResult.Ok(KvOutput.Written(newVersion))
    }

    private fun delete(key: KvKey, ifVersion: Long?): KvResult {
        val raw = key.raw()
        if (ifVersion != null) {
            val existing = prefs.getString(raw, null)
            val currentVersion = existing?.let { decodeValue(it)?.version?.toLong() } ?: 0L
            if (currentVersion != ifVersion) {
                return KvResult.Error(KvError.VersionMismatch(ifVersion.toULong(), currentVersion.toULong()))
            }
        }
        val existed = prefs.contains(raw)
        prefs.edit().remove(raw).apply()
        return KvResult.Ok(KvOutput.Deleted(existed))
    }

    private fun exists(key: KvKey): KvResult = KvResult.Ok(KvOutput.Exists(prefs.contains(key.raw())))

    private fun list(namespace: KeyNamespace, prefix: String?, limit: UInt, cursor: String?): KvResult {
        val nsPrefix = "${namespace.prefix()}:"
        val fullPrefix = prefix?.let { "$nsPrefix$it" } ?: nsPrefix
        val allKeys = prefs.all.keys.filter { it.startsWith(fullPrefix) }.sorted()
        val startIndex = cursor?.let { c -> allKeys.indexOfFirst { it > c }.takeIf { it >= 0 } ?: allKeys.size } ?: 0
        val entries = allKeys.drop(startIndex).take(limit.toInt()).mapNotNull { key ->
            decodeValue(prefs.getString(key, "") ?: "")?.let {
                KvListEntry(key.removePrefix(nsPrefix), it.version, it.data.size.toULong(), it.updatedAt)
            }
        }
        val hasMore = startIndex + entries.size < allKeys.size
        return KvResult.Ok(KvOutput.List(entries, if (hasMore) entries.lastOrNull()?.key else null, hasMore))
    }

    private fun getMulti(keys: List<KvKey>): KvResult = KvResult.Ok(KvOutput.Multi(keys.map { decodeValue(prefs.getString(it.raw(), "") ?: "") }))

    private fun deleteMulti(keys: List<KvKey>): KvResult {
        val editor = prefs.edit()
        var count = 0
        keys.forEach { if (prefs.contains(it.raw())) { editor.remove(it.raw()); count++ } }
        editor.apply()
        return KvResult.Ok(KvOutput.DeletedMulti(count.toULong()))
    }

    private fun encodeValue(v: KvValue): String = org.json.JSONObject().apply {
        put("data", android.util.Base64.encodeToString(v.data, android.util.Base64.NO_WRAP))
        put("version", v.version.toLong())
        put("createdAt", v.createdAt.toLong())
        put("updatedAt", v.updatedAt.toLong())
    }.toString()

    private fun decodeValue(s: String): KvValue? = try {
        val j = org.json.JSONObject(s)
        KvValue(android.util.Base64.decode(j.getString("data"), android.util.Base64.NO_WRAP),
            j.getLong("version").toULong(), j.getLong("createdAt").toULong(), j.getLong("updatedAt").toULong())
    } catch (e: Exception) { null }
}

class LocationHandler(private val context: android.content.Context) : EffectHandler<LocationOperation, LocationResult> {
    override suspend fun handle(operation: LocationOperation): LocationResult = LocationResult.Error(LocationError.NotSupported)
}

class CameraHandler(private val context: android.content.Context) : EffectHandler<CameraOperation, CameraResult> {
    override suspend fun handle(operation: CameraOperation): CameraResult = when (operation) {
        is CameraOperation.CheckPermission -> {
            val granted = androidx.core.content.ContextCompat.checkSelfPermission(context, android.Manifest.permission.CAMERA) == android.content.pm.PackageManager.PERMISSION_GRANTED
            CameraResult.Ok(CameraOutput.PermissionStatus(if (granted) PermissionStatus.Granted else PermissionStatus.NotDetermined))
        }
        else -> CameraResult.Error(CameraError.Internal("Must be handled by Activity"))
    }
}

class CryptoHandler : EffectHandler<CryptoOperation, CryptoResult> {
    override suspend fun handle(operation: CryptoOperation): CryptoResult = withContext(Dispatchers.Default) {
        try {
            when (operation) {
                is CryptoOperation.Hash -> {
                    val digest = java.security.MessageDigest.getInstance(when(operation.algorithm) {
                        HashAlgorithm.Sha256 -> "SHA-256"
                        HashAlgorithm.Sha384 -> "SHA-384"
                        HashAlgorithm.Sha512 -> "SHA-512"
                    })
                    CryptoResult.Ok(CryptoOutput.Hash(digest.digest(operation.data)))
                }
                is CryptoOperation.RandomBytes -> {
                    val b = ByteArray(operation.length.toInt())
                    java.security.SecureRandom().nextBytes(b)
                    CryptoResult.Ok(CryptoOutput.RandomBytes(b))
                }
                else -> CryptoResult.Error(CryptoError.NotSupported(operation::class.simpleName ?: ""))
            }
        } catch (e: Exception) { CryptoResult.Error(CryptoError.Internal(e.message ?: "")) }
    }
}

class PushHandler(private val context: android.content.Context) : EffectHandler<PushOperation, PushResult> {
    override suspend fun handle(operation: PushOperation): PushResult = try {
        when (operation) {
            is PushOperation.RequestToken -> {
                val token = com.google.android.gms.tasks.Tasks.await(com.google.firebase.messaging.FirebaseMessaging.getInstance().token)
                PushResult.Ok(PushOutput.Token(token))
            }
            else -> PushResult.Error(PushError.NotSupported)
        }
    } catch (e: Exception) { PushResult.Error(PushError.Internal(e.message ?: "")) }
}
