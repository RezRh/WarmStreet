package com.warmstreet

import android.app.Application
import android.os.StrictMode
import android.util.Log
import com.google.firebase.FirebaseApp
import com.google.firebase.crashlytics.FirebaseCrashlytics
import java.util.concurrent.atomic.AtomicBoolean
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.ProcessLifecycleOwner

class WarmStreetApplication : Application() {

    companion object {
        private const val TAG = "WarmStreetApp"

        @Volatile
        private var instance: WarmStreetApplication? = null

        fun getInstance(): WarmStreetApplication {
            return instance ?: throw IllegalStateException("Application not initialized")
        }
    }

    lateinit var core: Core
        private set

    private val isInitialized = AtomicBoolean(false)

    var isInForeground: Boolean = false
        private set

    override fun onCreate() {
        super.onCreate()
        instance = this

        setupStrictMode()
        setupCrashHandling()
        initializeFirebase()
        loadNativeLibrary()
        initializeCore()
        setupProcessLifecycle()

        isInitialized.set(true)
        Log.i(TAG, "Application initialized successfully")
    }

    private fun setupStrictMode() {
        if (BuildConfig.DEBUG) {
            StrictMode.setThreadPolicy(
                StrictMode.ThreadPolicy.Builder()
                    .detectDiskReads()
                    .detectDiskWrites()
                    .detectNetwork()
                    .penaltyLog()
                    .build()
            )

            StrictMode.setVmPolicy(
                StrictMode.VmPolicy.Builder()
                    .detectLeakedSqlLiteObjects()
                    .detectLeakedClosableObjects()
                    .detectActivityLeaks()
                    .penaltyLog()
                    .build()
            )

            Log.d(TAG, "StrictMode enabled for debug build")
        }
    }

    private fun setupCrashHandling() {
        val defaultHandler = Thread.getDefaultUncaughtExceptionHandler()

        Thread.setDefaultUncaughtExceptionHandler { thread, throwable ->
            Log.e(TAG, "Uncaught exception on thread ${thread.name}", throwable)

            try {
                if (::core.isInitialized) {
                    core.onUnhandledException(throwable)
                }
            } catch (e: Exception) {
                Log.e(TAG, "Failed to notify core of crash", e)
            }

            defaultHandler?.uncaughtException(thread, throwable)
        }
    }

    private fun initializeFirebase() {
        try {
            FirebaseApp.initializeApp(this)

            if (!BuildConfig.DEBUG) {
                FirebaseCrashlytics.getInstance().setCrashlyticsCollectionEnabled(true)
            } else {
                FirebaseCrashlytics.getInstance().setCrashlyticsCollectionEnabled(false)
            }

            Log.i(TAG, "Firebase initialized")
        } catch (e: Exception) {
            Log.e(TAG, "Failed to initialize Firebase", e)
        }
    }

    private fun loadNativeLibrary() {
        try {
            Core.ensureLibraryLoaded()
            Log.i(TAG, "Native library loaded")
        } catch (e: UnsatisfiedLinkError) {
            Log.e(TAG, "Failed to load native library", e)
            FirebaseCrashlytics.getInstance().recordException(e)
            throw RuntimeException("Failed to load native library", e)
        }
    }

    private fun initializeCore() {
        try {
            core = Core(this)
            Log.i(TAG, "Core initialized")
        } catch (e: Exception) {
            Log.e(TAG, "Failed to initialize Core", e)
            FirebaseCrashlytics.getInstance().recordException(e)
            throw RuntimeException("Failed to initialize Core", e)
        }
    }

    private fun setupProcessLifecycle() {
        ProcessLifecycleOwner.get().lifecycle.addObserver(
            object : DefaultLifecycleObserver {
                override fun onStart(owner: LifecycleOwner) {
                    isInForeground = true
                    Log.d(TAG, "App entered foreground")

                    if (::core.isInitialized) {
                        core.onAppForeground()
                    }
                }

                override fun onStop(owner: LifecycleOwner) {
                    isInForeground = false
                    Log.d(TAG, "App entered background")

                    if (::core.isInitialized) {
                        core.onAppBackground()
                    }
                }
            }
        )
    }

    override fun onLowMemory() {
        super.onLowMemory()
        Log.w(TAG, "Low memory warning")

        if (::core.isInitialized) {
            core.onLowMemory()
        }
    }

    override fun onTrimMemory(level: Int) {
        super.onTrimMemory(level)

        Log.d(TAG, "Trim memory: $level")

        if (::core.isInitialized) {
            core.onTrimMemory(level)
        }
    }

    fun isReady(): Boolean = isInitialized.get() && ::core.isInitialized
}