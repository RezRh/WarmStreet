package com.warmstreet

import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedContentTransitionScope
import androidx.compose.animation.ContentTransform
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.warmstreet.shared.Event
import com.warmstreet.shared.ViewModel
import com.warmstreet.shared.ViewState
import com.warmstreet.ui.components.ErrorDialog
import com.warmstreet.ui.components.FullScreenError
import com.warmstreet.ui.components.FullScreenLoading
import com.warmstreet.ui.screens.CameraPreviewScreen
import com.warmstreet.ui.screens.LocationPermissionScreen
import com.warmstreet.ui.screens.LoginScreen
import com.warmstreet.ui.screens.PinDropScreen
import com.warmstreet.ui.screens.RadiusPickerScreen
import com.warmstreet.ui.screens.ReadyScreen
import kotlinx.coroutines.launch

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
                transitionSpec = { createTransitionSpec(initialState, targetState) },
                label = "main_content"
            ) { state ->
                when (state) {
                    is ViewState.Loading -> {
                        FullScreenLoading(
                            message = state.message
                        )
                    }

                    is ViewState.Unauthenticated -> {
                        BackHandler(enabled = false) {}

                        LoginScreen(
                            onLogin = { provider ->
                                core.update(Event.LoginRequested(provider))
                            },
                            onContinueAsGuest = {
                                core.update(Event.ContinueAsGuest)
                            },
                            isLoading = false
                        )
                    }

                    is ViewState.Authenticating -> {
                        BackHandler {
                            core.update(Event.CancelAuthentication)
                        }

                        LoginScreen(
                            onLogin = {},
                            onContinueAsGuest = {},
                            isLoading = true
                        )
                    }

                    is ViewState.OnboardingLocation -> {
                        BackHandler {
                            core.update(Event.BackPressed)
                        }

                        LocationPermissionScreen(
                            permissionState = state.permissionState,
                            onRequestPermission = {
                                core.update(Event.RequestLocationPermission)
                            },
                            onUseCurrentLocation = {
                                core.update(Event.UseCurrentLocation)
                            },
                            onDropPin = {
                                core.update(Event.ShowPinDrop)
                            },
                            onOpenSettings = {
                                core.update(Event.OpenAppSettings)
                            }
                        )
                    }

                    is ViewState.PinDrop -> {
                        BackHandler {
                            core.update(Event.ClosePinDrop)
                        }

                        PinDropScreen(
                            initialLocation = state.initialLocation,
                            onLocationSelected = { latLon ->
                                core.update(Event.LocationPinned(latLon))
                            },
                            onCancel = {
                                core.update(Event.ClosePinDrop)
                            }
                        )
                    }

                    is ViewState.OnboardingRadius -> {
                        BackHandler {
                            core.update(Event.BackPressed)
                        }

                        RadiusPickerScreen(
                            location = state.location,
                            selectedRadius = state.radius,
                            onRadiusChanged = { radius ->
                                core.update(Event.RadiusChanged(radius))
                            },
                            onConfirm = {
                                core.update(Event.ConfirmRadius)
                            },
                            onBack = {
                                core.update(Event.BackPressed)
                            }
                        )
                    }

                    is ViewState.CameraCapture -> {
                        BackHandler {
                            core.update(Event.CancelCapture)
                        }

                        CameraPreviewScreen(
                            config = state.config,
                            onPhotoCaptured = { photo ->
                                core.update(Event.PhotoCaptured(photo))
                            },
                            onCancel = {
                                core.update(Event.CancelCapture)
                            }
                        )
                    }

                    is ViewState.Ready -> {
                        BackHandler {
                            core.update(Event.BackPressed)
                        }

                        ReadyScreen(
                            state = state,
                            onEvent = { event -> core.update(event) }
                        )
                    }

                    is ViewState.Error -> {
                        BackHandler {
                            core.update(Event.BackPressed)
                        }

                        FullScreenError(
                            title = state.title,
                            message = state.message,
                            isRetryable = state.isRetryable,
                            onRetry = {
                                core.update(Event.Retry)
                            },
                            onBack = {
                                core.update(Event.BackPressed)
                            }
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
}

@Composable
private fun LoadingOverlay() {
    Box(
        modifier = Modifier.fillMaxSize(),
        contentAlignment = Alignment.Center
    ) {
        Box(
            modifier = Modifier
                .size(80.dp)
                .align(Alignment.Center),
            contentAlignment = Alignment.Center
        ) {
            CircularProgressIndicator(
                modifier = Modifier.size(48.dp),
                color = MaterialTheme.colorScheme.primary,
                strokeWidth = 4.dp
            )
        }
    }
}

private fun createTransitionSpec(
    initialState: ViewState,
    targetState: ViewState
): ContentTransform {
    val isForward = getStateOrder(targetState) > getStateOrder(initialState)

    return if (isForward) {
        slideInHorizontally(
            animationSpec = tween(300),
            initialOffsetX = { fullWidth -> fullWidth }
        ) + fadeIn(animationSpec = tween(300)) togetherWith
                slideOutHorizontally(
                    animationSpec = tween(300),
                    targetOffsetX = { fullWidth -> -fullWidth }
                ) + fadeOut(animationSpec = tween(300))
    } else {
        slideInHorizontally(
            animationSpec = tween(300),
            initialOffsetX = { fullWidth -> -fullWidth }
        ) + fadeIn(animationSpec = tween(300)) togetherWith
                slideOutHorizontally(
                    animationSpec = tween(300),
                    targetOffsetX = { fullWidth -> fullWidth }
                ) + fadeOut(animationSpec = tween(300))
    }
}

private fun getStateOrder(state: ViewState): Int {
    return when (state) {
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
}