package com.warmstreet

import android.app.Application
import android.util.Log
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.warmstreet.shared.*
import com.warmstreet.capabilities.*
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

    fun onUnhandledException(throwable: Throwable) {
        update(Event.SystemError(throwable.message ?: "Unknown crash", "UncaughtException"))
    }

    fun onAppForeground() {
        update(Event.LifecycleResumed)
    }

    fun onAppBackground() {
        update(Event.LifecyclePaused)
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

    fun pushTokenReceived(token: String) {
        update(Event.NotificationPermissionResult(true)) // Simplified
        // You might want a specific event for token received if the core needs it
    }

    fun pushReceived(payload: String) {
        // Here we'd normally parse the JSON payload into a PushPayload object
        // For now, let's assume we have a way to dispatch it
        // core.update(Event.PushReceived(...))
    }

    override fun onCleared() {
        super.onCleared()
        effectProcessorJob?.cancel()
        effectChannel.close()
    }
}
