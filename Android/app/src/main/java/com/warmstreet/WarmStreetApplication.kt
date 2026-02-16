package com.warmstreet

import android.app.Application
import android.os.StrictMode
import android.util.Log
import com.google.firebase.FirebaseApp
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.ProcessLifecycleOwner
import java.util.concurrent.atomic.AtomicBoolean

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
        initializeFirebase()
        initializeCore()
        setupProcessLifecycle()

        isInitialized.set(true)
        Log.i(TAG, "Application initialized successfully")
    }

    private fun setupStrictMode() {
        try {
            val buildConfigClass = Class.forName("${packageName}.BuildConfig")
            val debugField = buildConfigClass.getField("DEBUG")
            if (debugField.getBoolean(null)) {
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
        } catch (e: Exception) {
            // Ignore if BuildConfig not found or DEBUG is false
        }
    }

    private fun initializeFirebase() {
        try {
            FirebaseApp.initializeApp(this)
            Log.i(TAG, "Firebase initialized")
        } catch (e: Exception) {
            Log.e(TAG, "Failed to initialize Firebase", e)
        }
    }

    private fun initializeCore() {
        try {
            core = Core(this)
            Log.i(TAG, "Core initialized")
        } catch (e: Exception) {
            Log.e(TAG, "Failed to initialize Core", e)
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

    fun isReady(): Boolean = isInitialized.get() && ::core.isInitialized
}
