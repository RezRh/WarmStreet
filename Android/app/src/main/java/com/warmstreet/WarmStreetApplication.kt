package com.warmstreet

import android.app.Application
import android.os.Build
import android.os.StrictMode
import android.util.Log
import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.CloudOff
import androidx.compose.material.icons.filled.Error
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Divider
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalLifecycleOwner
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.ProcessLifecycleOwner
import com.google.firebase.FirebaseApp
import com.google.firebase.crashlytics.FirebaseCrashlytics
import com.warmstreet.shared.Event
import com.warmstreet.shared.ViewState
import kotlinx.coroutines.launch
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

        val levelName = when (level) {
            TRIM_MEMORY_RUNNING_MODERATE -> "RUNNING_MODERATE"
            TRIM_MEMORY_RUNNING_LOW -> "RUNNING_LOW"
            TRIM_MEMORY_RUNNING_CRITICAL -> "RUNNING_CRITICAL"
            TRIM_MEMORY_UI_HIDDEN -> "UI_HIDDEN"
            TRIM_MEMORY_BACKGROUND -> "BACKGROUND"
            TRIM_MEMORY_MODERATE -> "MODERATE"
            TRIM_MEMORY_COMPLETE -> "COMPLETE"
            else -> "UNKNOWN($level)"
        }

        Log.d(TAG, "Trim memory: $levelName")

        if (::core.isInitialized) {
            core.onTrimMemory(level)
        }
    }

    fun isReady(): Boolean = isInitialized.get() && ::core.isInitialized
}

private val WarmOrange = Color(0xFFFF6B35)
private val WarmOrangeDark = Color(0xFFE55A2B)
private val WarmYellow = Color(0xFFFFC107)
private val DeepRed = Color(0xFFD32F2F)

private val LightColorScheme = lightColorScheme(
    primary = WarmOrange,
    onPrimary = Color.White,
    primaryContainer = Color(0xFFFFE0D6),
    onPrimaryContainer = Color(0xFF3D1500),
    secondary = WarmYellow,
    onSecondary = Color.Black,
    secondaryContainer = Color(0xFFFFECB3),
    onSecondaryContainer = Color(0xFF261A00),
    tertiary = Color(0xFF6D5E0F),
    onTertiary = Color.White,
    tertiaryContainer = Color(0xFFF8E287),
    onTertiaryContainer = Color(0xFF221B00),
    error = DeepRed,
    onError = Color.White,
    errorContainer = Color(0xFFFFDAD6),
    onErrorContainer = Color(0xFF410002),
    background = Color(0xFFFFFBFF),
    onBackground = Color(0xFF201A17),
    surface = Color(0xFFFFFBFF),
    onSurface = Color(0xFF201A17),
    surfaceVariant = Color(0xFFF5DED4),
    onSurfaceVariant = Color(0xFF53443C),
    outline = Color(0xFF85736A),
    outlineVariant = Color(0xFFD8C2B8)
)

private val DarkColorScheme = darkColorScheme(
    primary = Color(0xFFFFB599),
    onPrimary = Color(0xFF5F1500),
    primaryContainer = WarmOrangeDark,
    onPrimaryContainer = Color(0xFFFFDBCF),
    secondary = Color(0xFFFFE082),
    onSecondary = Color(0xFF3F2E00),
    secondaryContainer = Color(0xFF5C4300),
    onSecondaryContainer = Color(0xFFFFECB3),
    tertiary = Color(0xFFDBC66E),
    onTertiary = Color(0xFF393000),
    tertiaryContainer = Color(0xFF524600),
    onTertiaryContainer = Color(0xFFF8E287),
    error = Color(0xFFFFB4AB),
    onError = Color(0xFF690005),
    errorContainer = Color(0xFF93000A),
    onErrorContainer = Color(0xFFFFDAD6),
    background = Color(0xFF201A17),
    onBackground = Color(0xFFEDE0DB),
    surface = Color(0xFF201A17),
    onSurface = Color(0xFFEDE0DB),
    surfaceVariant = Color(0xFF53443C),
    onSurfaceVariant = Color(0xFFD8C2B8),
    outline = Color(0xFFA08D84),
    outlineVariant = Color(0xFF53443C)
)

@Composable
fun WarmStreetTheme(
    darkTheme: Boolean = androidx.compose.foundation.isSystemInDarkTheme(),
    dynamicColor: Boolean = false,
    content: @Composable () -> Unit
) {
    val colorScheme = when {
        dynamicColor && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S -> {
            val context = LocalContext.current
            if (darkTheme) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
        }
        darkTheme -> DarkColorScheme
        else -> LightColorScheme
    }

    MaterialTheme(
        colorScheme = colorScheme,
        content = content
    )
}

@Composable
fun WarmStreetApp(core: Core) {
    val viewModel = core.view
    val snackbarHostState = remember { SnackbarHostState() }
    val scope = rememberCoroutineScope()

    var showErrorDialog by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(viewModel.error) {
        viewModel.error?.let { error ->
            if (error.isTransient) {
                scope.launch {
                    snackbarHostState.showSnackbar(
                        message = error.message,
                        actionLabel = if (error.isRetryable) "Retry" else null
                    )
                }
                core.update(Event.ErrorDismissed)
            } else {
                showErrorDialog = error.message
            }
        }
    }

    LaunchedEffect(viewModel.toast) {
        viewModel.toast?.let { message ->
            scope.launch {
                snackbarHostState.showSnackbar(message)
            }
            core.update(Event.ToastShown)
        }
    }

    Scaffold(
        snackbarHost = { SnackbarHost(snackbarHostState) },
        containerColor = MaterialTheme.colorScheme.background
    ) { paddingValues ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
        ) {
            AnimatedContent(
                targetState = viewModel.state,
                transitionSpec = {
                    val isForward = getStateOrder(targetState) > getStateOrder(initialState)
                    if (isForward) {
                        slideInHorizontally(tween(300)) { it } + fadeIn(tween(300)) togetherWith
                                slideOutHorizontally(tween(300)) { -it } + fadeOut(tween(300))
                    } else {
                        slideInHorizontally(tween(300)) { -it } + fadeIn(tween(300)) togetherWith
                                slideOutHorizontally(tween(300)) { it } + fadeOut(tween(300))
                    }
                },
                label = "main_content"
            ) { state ->
                when (state) {
                    is ViewState.Loading -> {
                        FullScreenLoading(message = state.message)
                    }

                    is ViewState.Unauthenticated -> {
                        BackHandler(enabled = false) {}
                        LoginScreenPlaceholder(
                            onLogin = { core.update(Event.LoginRequested(it)) },
                            isLoading = false
                        )
                    }

                    is ViewState.Authenticating -> {
                        BackHandler { core.update(Event.CancelAuthentication) }
                        LoginScreenPlaceholder(
                            onLogin = {},
                            isLoading = true
                        )
                    }

                    is ViewState.OnboardingLocation -> {
                        BackHandler { core.update(Event.BackPressed) }
                        LocationPermissionScreenPlaceholder(
                            onRequestPermission = { core.update(Event.RequestLocationPermission) },
                            onDropPin = { core.update(Event.ShowPinDrop) }
                        )
                    }

                    is ViewState.PinDrop -> {
                        BackHandler { core.update(Event.ClosePinDrop) }
                        PinDropScreenPlaceholder(
                            onLocationSelected = { core.update(Event.LocationPinned(it)) },
                            onCancel = { core.update(Event.ClosePinDrop) }
                        )
                    }

                    is ViewState.OnboardingRadius -> {
                        BackHandler { core.update(Event.BackPressed) }
                        RadiusPickerScreenPlaceholder(
                            radius = state.radius,
                            onRadiusChanged = { core.update(Event.RadiusChanged(it)) },
                            onConfirm = { core.update(Event.ConfirmRadius) }
                        )
                    }

                    is ViewState.CameraCapture -> {
                        BackHandler { core.update(Event.CancelCapture) }
                        CameraScreenPlaceholder(
                            onCancel = { core.update(Event.CancelCapture) }
                        )
                    }

                    is ViewState.Ready -> {
                        BackHandler { core.update(Event.BackPressed) }
                        ReadyScreenPlaceholder(
                            onEvent = { core.update(it) }
                        )
                    }

                    is ViewState.Error -> {
                        BackHandler { core.update(Event.BackPressed) }
                        FullScreenError(
                            title = state.title,
                            message = state.message,
                            isRetryable = state.isRetryable,
                            onRetry = { core.update(Event.Retry) },
                            onBack = { core.update(Event.BackPressed) }
                        )
                    }
                }
            }

            if (viewModel.isGlobalLoading) {
                LoadingOverlay()
            }
        }
    }

    showErrorDialog?.let { message ->
        ErrorDialog(
            message = message,
            onDismiss = {
                showErrorDialog = null
                core.update(Event.ErrorDismissed)
            },
            onRetry = if (viewModel.error?.isRetryable == true) {
                {
                    showErrorDialog = null
                    core.update(Event.Retry)
                }
            } else null
        )
    }

    LifecycleHandler(core)
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
        onDispose { lifecycleOwner.lifecycle.removeObserver(observer) }
    }
}

private fun getStateOrder(state: ViewState): Int = when (state) {
    is ViewState.Loading -> 0
    is ViewState.Unauthenticated -> 1
    is ViewState.Authenticating -> 2
    is ViewState.OnboardingLocation -> 3
    is ViewState.PinDrop -> 4
    is ViewState.OnboardingRadius -> 5
    is ViewState.CameraCapture -> 6
    is ViewState.Ready -> 7
    is ViewState.Error -> -1
}

@Composable
fun FullScreenLoading(
    message: String? = null,
    modifier: Modifier = Modifier
) {
    Column(
        modifier = modifier.fillMaxSize(),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        CircularProgressIndicator(
            modifier = Modifier.size(48.dp),
            color = MaterialTheme.colorScheme.primary,
            strokeWidth = 4.dp
        )

        if (message != null) {
            Spacer(modifier = Modifier.height(24.dp))
            Text(
                text = message,
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = TextAlign.Center
            )
        }
    }
}

@Composable
fun LoadingOverlay(modifier: Modifier = Modifier) {
    Box(
        modifier = modifier
            .fillMaxSize()
            .background(Color.Black.copy(alpha = 0.3f)),
        contentAlignment = Alignment.Center
    ) {
        Surface(
            shape = RoundedCornerShape(16.dp),
            color = MaterialTheme.colorScheme.surface,
            shadowElevation = 8.dp
        ) {
            Box(
                modifier = Modifier.padding(32.dp),
                contentAlignment = Alignment.Center
            ) {
                CircularProgressIndicator(
                    modifier = Modifier.size(48.dp),
                    color = MaterialTheme.colorScheme.primary
                )
            }
        }
    }
}

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

@Composable
fun InlineError(
    message: String,
    onRetry: (() -> Unit)? = null,
    modifier: Modifier = Modifier
) {
    Row(
        modifier = modifier
            .fillMaxWidth()
            .padding(16.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.Center
    ) {
        Icon(
            imageVector = Icons.Filled.Warning,
            contentDescription = null,
            modifier = Modifier.size(20.dp),
            tint = MaterialTheme.colorScheme.error
        )

        Spacer(modifier = Modifier.width(8.dp))

        Text(
            text = message,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.error,
            modifier = Modifier.weight(1f, fill = false)
        )

        if (onRetry != null) {
            Spacer(modifier = Modifier.width(8.dp))
            OutlinedButton(onClick = onRetry) {
                Text("Retry")
            }
        }
    }
}

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

@Composable
fun ConfirmationDialog(
    title: String,
    message: String,
    onConfirm: () -> Unit,
    onDismiss: () -> Unit,
    confirmText: String = "Confirm",
    dismissText: String = "Cancel",
    isDestructive: Boolean = false
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(text = title, style = MaterialTheme.typography.headlineSmall) },
        text = { Text(text = message, style = MaterialTheme.typography.bodyMedium) },
        confirmButton = {
            Button(
                onClick = onConfirm,
                colors = if (isDestructive) {
                    ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.error)
                } else ButtonDefaults.buttonColors()
            ) {
                Text(confirmText)
            }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text(dismissText) } }
    )
}

@Composable
fun LoadingButton(
    text: String,
    onClick: () -> Unit,
    isLoading: Boolean,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    loadingText: String? = null
) {
    Button(
        onClick = onClick,
        modifier = modifier,
        enabled = enabled && !isLoading,
        contentPadding = PaddingValues(horizontal = 24.dp, vertical = 12.dp)
    ) {
        if (isLoading) {
            CircularProgressIndicator(
                modifier = Modifier.size(20.dp),
                color = MaterialTheme.colorScheme.onPrimary,
                strokeWidth = 2.dp
            )
            if (loadingText != null) {
                Spacer(modifier = Modifier.width(8.dp))
                Text(loadingText)
            }
        } else {
            Text(text)
        }
    }
}

@Composable
fun LoadingOutlinedButton(
    text: String,
    onClick: () -> Unit,
    isLoading: Boolean,
    modifier: Modifier = Modifier,
    enabled: Boolean = true
) {
    OutlinedButton(
        onClick = onClick,
        modifier = modifier,
        enabled = enabled && !isLoading,
        contentPadding = PaddingValues(horizontal = 24.dp, vertical = 12.dp)
    ) {
        if (isLoading) {
            CircularProgressIndicator(
                modifier = Modifier.size(20.dp),
                color = MaterialTheme.colorScheme.primary,
                strokeWidth = 2.dp
            )
        } else {
            Text(text)
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AppTopBar(
    title: String,
    onBackClick: (() -> Unit)? = null,
    actions: @Composable () -> Unit = {}
) {
    TopAppBar(
        title = { Text(text = title, style = MaterialTheme.typography.titleLarge) },
        navigationIcon = {
            if (onBackClick != null) {
                IconButton(onClick = onBackClick) {
                    Icon(imageVector = Icons.Filled.ArrowBack, contentDescription = "Back")
                }
            }
        },
        actions = { actions() },
        colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.surface)
    )
}

@Composable
fun SectionHeader(title: String, modifier: Modifier = Modifier) {
    Text(
        text = title.uppercase(),
        style = MaterialTheme.typography.labelMedium,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        fontWeight = FontWeight.SemiBold,
        modifier = modifier.padding(horizontal = 16.dp, vertical = 8.dp)
    )
}

@Composable
fun ListItem(
    title: String,
    subtitle: String? = null,
    leadingIcon: ImageVector? = null,
    trailingContent: @Composable (() -> Unit)? = null,
    onClick: (() -> Unit)? = null,
    modifier: Modifier = Modifier
) {
    Row(
        modifier = modifier
            .fillMaxWidth()
            .then(if (onClick != null) Modifier.clickable(onClick = onClick) else Modifier)
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        if (leadingIcon != null) {
            Icon(
                imageVector = leadingIcon,
                contentDescription = null,
                modifier = Modifier.size(24.dp),
                tint = MaterialTheme.colorScheme.onSurfaceVariant
            )
            Spacer(modifier = Modifier.width(16.dp))
        }

        Column(modifier = Modifier.weight(1f)) {
            Text(text = title, style = MaterialTheme.typography.bodyLarge)
            if (subtitle != null) {
                Spacer(modifier = Modifier.height(2.dp))
                Text(
                    text = subtitle,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }

        trailingContent?.invoke()
    }
}

@Composable
fun StatusIndicator(
    isActive: Boolean,
    activeColor: Color = MaterialTheme.colorScheme.primary,
    inactiveColor: Color = MaterialTheme.colorScheme.surfaceVariant,
    modifier: Modifier = Modifier
) {
    Box(
        modifier = modifier
            .size(12.dp)
            .clip(CircleShape)
            .background(if (isActive) activeColor else inactiveColor)
    )
}

@Composable
fun InfoCard(
    title: String,
    content: String,
    icon: ImageVector? = null,
    modifier: Modifier = Modifier
) {
    Card(
        modifier = modifier.fillMaxWidth(),
        shape = RoundedCornerShape(12.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant)
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            verticalAlignment = Alignment.Top
        ) {
            if (icon != null) {
                Icon(
                    imageVector = icon,
                    contentDescription = null,
                    modifier = Modifier.size(24.dp),
                    tint = MaterialTheme.colorScheme.primary
                )
                Spacer(modifier = Modifier.width(12.dp))
            }

            Column {
                Text(
                    text = title,
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.SemiBold
                )
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = content,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }
    }
}

@Composable
fun SuccessCheckmark(modifier: Modifier = Modifier, size: Int = 48) {
    Box(
        modifier = modifier
            .size(size.dp)
            .clip(CircleShape)
            .background(MaterialTheme.colorScheme.primary),
        contentAlignment = Alignment.Center
    ) {
        Icon(
            imageVector = Icons.Filled.Check,
            contentDescription = "Success",
            modifier = Modifier.size((size * 0.6).dp),
            tint = MaterialTheme.colorScheme.onPrimary
        )
    }
}

@Composable
fun CloseButton(onClick: () -> Unit, modifier: Modifier = Modifier) {
    IconButton(onClick = onClick, modifier = modifier) {
        Icon(imageVector = Icons.Filled.Close, contentDescription = "Close")
    }
}

@Composable
fun SectionDivider(modifier: Modifier = Modifier) {
    Divider(
        modifier = modifier.padding(vertical = 8.dp),
        color = MaterialTheme.colorScheme.outlineVariant
    )
}

@Composable
private fun LoginScreenPlaceholder(
    onLogin: (String) -> Unit,
    isLoading: Boolean
) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        Text(
            text = "WarmStreet",
            style = MaterialTheme.typography.headlineLarge,
            color = MaterialTheme.colorScheme.primary
        )

        Spacer(modifier = Modifier.height(48.dp))

        LoadingButton(
            text = "Sign in with Google",
            onClick = { onLogin("google") },
            isLoading = isLoading,
            modifier = Modifier.fillMaxWidth()
        )

        Spacer(modifier = Modifier.height(16.dp))

        LoadingButton(
            text = "Sign in with Apple",
            onClick = { onLogin("apple") },
            isLoading = isLoading,
            modifier = Modifier.fillMaxWidth()
        )
    }
}

@Composable
private fun LocationPermissionScreenPlaceholder(
    onRequestPermission: () -> Unit,
    onDropPin: () -> Unit
) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        Text(
            text = "Location Access",
            style = MaterialTheme.typography.headlineMedium
        )

        Spacer(modifier = Modifier.height(16.dp))

        Text(
            text = "We need your location to show nearby cases",
            style = MaterialTheme.typography.bodyLarge,
            textAlign = TextAlign.Center,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )

        Spacer(modifier = Modifier.height(32.dp))

        Button(
            onClick = onRequestPermission,
            modifier = Modifier.fillMaxWidth()
        ) {
            Text("Enable Location")
        }

        Spacer(modifier = Modifier.height(16.dp))

        OutlinedButton(
            onClick = onDropPin,
            modifier = Modifier.fillMaxWidth()
        ) {
            Text("Choose on Map")
        }
    }
}

@Composable
private fun PinDropScreenPlaceholder(
    onLocationSelected: (Any) -> Unit,
    onCancel: () -> Unit
) {
    Box(
        modifier = Modifier.fillMaxSize(),
        contentAlignment = Alignment.Center
    ) {
        Text("Map View - Drop Pin Here")
    }
}

@Composable
private fun RadiusPickerScreenPlaceholder(
    radius: Int,
    onRadiusChanged: (Int) -> Unit,
    onConfirm: () -> Unit
) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        Text(
            text = "Choose Radius",
            style = MaterialTheme.typography.headlineMedium
        )

        Spacer(modifier = Modifier.height(16.dp))

        Text(
            text = "${radius}km",
            style = MaterialTheme.typography.displayMedium,
            color = MaterialTheme.colorScheme.primary
        )

        Spacer(modifier = Modifier.height(32.dp))

        Button(
            onClick = onConfirm,
            modifier = Modifier.fillMaxWidth()
        ) {
            Text("Confirm")
        }
    }
}

@Composable
private fun CameraScreenPlaceholder(onCancel: () -> Unit) {
    Box(
        modifier = Modifier.fillMaxSize(),
        contentAlignment = Alignment.Center
    ) {
        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            Text("Camera Preview")
            Spacer(modifier = Modifier.height(16.dp))
            OutlinedButton(onClick = onCancel) {
                Text("Cancel")
            }
        }
    }
}

@Composable
private fun ReadyScreenPlaceholder(onEvent: (Event) -> Unit) {
    Box(
        modifier = Modifier.fillMaxSize(),
        contentAlignment = Alignment.Center
    ) {
        Text(
            text = "Ready!",
            style = MaterialTheme.typography.headlineLarge
        )
    }
}