package com.warmstreet.shared

import android.graphics.Bitmap

// Generics and core types
class App {
    constructor()
    fun view(): ViewModel = ViewModel(ViewState.Loading())
    suspend fun update(event: Event): List<Effect> = emptyList()
}

data class ViewModel(
    val state: ViewState,
    val error: ErrorDetail? = null,
    val toast: String? = null,
    val isGlobalLoading: Boolean = false
) {
    companion object {
        val Loading = ViewModel(ViewState.Loading())
        fun Error(message: String) = ViewModel(ViewState.Error(message = message))
    }
}

data class ErrorDetail(
    val message: String,
    val isTransient: Boolean,
    val isRetryable: Boolean
)

sealed class ViewState {
    data class Loading(val message: String? = null) : ViewState()
    object Unauthenticated : ViewState()
    object Authenticating : ViewState()
    data class OnboardingLocation(val permissionState: Any? = null) : ViewState()
    data class OnboardingRadius(val location: Location, val radius: Double) : ViewState()
    data class PinDrop(val initialLocation: Location?) : ViewState()
    data class CameraCapture(val config: CaptureConfig) : ViewState()
    data class Ready(
        val listItems: List<CaseListItem> = emptyList(),
        val selectedDetail: CaseDetail? = null,
        val stagedPhoto: CapturedImage? = null,
        val stagedCrop: CapturedImage? = null,
        val feedView: FeedViewMode = FeedViewMode.Map,
        val detectionCount: Int = 0,
        val topConfidence: Float = 0f,
        val areaCenter: Pair<Double, Double>? = null
    ) : ViewState()
    data class Error(
        val title: String = "Error",
        val message: String,
        val isRetryable: Boolean = false
    ) : ViewState()
}

enum class FeedViewMode { Map, List }

data class Location(val latitude: Double, val longitude: Double)

data class CaseListItem(
    val id: String,
    val descriptionPreview: String,
    val distanceText: String,
    val timeAgo: String,
    val photoUrl: String?
)

data class CaseDetail(
    val id: String,
    val description: String,
    val status: String,
    val photoUrl: String?,
    val distanceText: String,
    val timeAgo: String,
    val geminiDiagnosis: String?,
    val claimState: ClaimState,
    val availableTransitions: List<String>
)

enum class ClaimState { None, ClaimedByMe, ClaimedByOther, Available }

sealed class Event {
    // Auth
    data class LoginRequested(val provider: String) : Event()
    object ContinueAsGuest : Event()
    object CancelAuthentication : Event()
    
    // Navigation & UI
    object BackPressed : Event()
    object Retry : Event()
    object ErrorDismissed : Event()
    object ToastShown : Event()
    object SwitchToMap : Event()
    object SwitchToList : Event()
    
    // Location
    object RequestLocationPermission : Event()
    object UseCurrentLocation : Event()
    object ShowPinDrop : Event()
    object ClosePinDrop : Event()
    data class LocationPinned(val location: Location) : Event()
    data class RadiusChanged(val radius: Double) : Event()
    object ConfirmRadius : Event()
    data class LocationPinDropped(val location: Location) : Event()
    object OpenAppSettings : Event()
    data class RadiusSelected(val meters: Long) : Event()
    
    // Camera
    object CapturePhotoRequested : Event()
    object CancelCapture : Event()
    data class PhotoCaptured(val image: CapturedImage) : Event()
    object PhotoCancelled : Event()
    
    // Cases
    data class CaseMarkerTapped(val id: String) : Event()
    object CaseDismissed : Event()
    data class ClaimRequested(val id: String) : Event()
    data class TransitionRequested(val id: String, val transition: String) : Event()
    data class CreateCaseRequested(val payload: CreateCasePayload) : Event()
    
    // System
    data class SystemError(val message: String, val source: String) : Event()
    data class NotificationPermissionResult(val granted: Boolean) : Event()
    data class LocationPermissionResult(val fineLocation: Boolean, val coarseLocation: Boolean) : Event()
    data class DeepLink(val path: String, val params: Map<String, String?>) : Event()
    data class UniversalLink(val path: String, val params: Map<String, String?>) : Event()
    data class OAuthError(val error: String, val description: String?) : Event()
    data class OAuthCallback(val code: String, val state: String?) : Event()
    
    // KeyValue Result
    data class KeyValueResult(val result: com.warmstreet.shared.KvResult) : Event()

    // Push
    data class PushReceived(val payload: PushPayload) : Event()

    // Lifecycle
    object LifecycleStarted : Event()
    object LifecycleResumed : Event()
    object LifecyclePaused : Event()
    object LifecycleStopped : Event()
}

data class CreateCasePayload(
    val photo: CapturedImage,
    val location: Location,
    val description: String?,
    val woundSeverity: Int? = null
)

sealed class PushPayload {
    data class NewRescue(val caseId: String, val location: Location) : PushPayload()
    data class Mute(val caseId: String, val claimedBy: String) : PushPayload()
    data class CaseUpdate(val caseId: String, val newStatus: String) : PushPayload()
}

sealed class Effect {
    object Render : Effect()
    data class Http(val operation: HttpOperation, val callback: (HttpResult) -> Event) : Effect()
    data class Kv(val operation: KvOperation, val callback: (com.warmstreet.shared.KvResult) -> Event) : Effect()
    data class Crypto(val operation: CryptoOperation, val callback: (CryptoResult) -> Event) : Effect()
    data class Camera(val operation: CameraOperation, val callback: (CameraResult) -> Event) : Effect()
    data class Push(val operation: PushOperation, val callback: (PushResult) -> Event) : Effect()
    data class Location(val operation: LocationOperation, val callback: (LocationResult) -> Event) : Effect()
}

// HTTP Capabilities
sealed class HttpOperation {
    data class Execute(val request: HttpRequest) : HttpOperation()
}

data class HttpRequest(
    val url: UrlWrapper,
    val method: HttpMethod,
    val headers: HttpHeaders,
    val body: ByteArray?,
    val maxResponseSize: Long = 10 * 1024 * 1024,
    val requestId: String = "",
    val timeoutMs: Long = 30000
)

data class UrlWrapper(private val url: String) {
    fun asStr(): String = url
}

class HttpHeaders {
    private val headers = mutableListOf<Pair<String, String>>()
    fun iter(): Iterator<Pair<String, String>> = headers.iterator()
    fun get(name: String): String? = headers.find { it.first.equals(name, ignoreCase = true) }?.second
    fun insert(name: String, value: String) { headers.add(name to value) }
}

enum class HttpMethod { Get, Post, Put, Patch, Delete, Head, Options }

sealed class HttpResult {
    data class Ok(val response: HttpResponse) : HttpResult()
    data class Error(val error: HttpError) : HttpResult()
}

data class HttpResponse(
    val status: UShort,
    val headers: HttpHeaders,
    val body: ByteArray,
    val requestId: String,
    val durationMs: ULong
)

sealed class HttpError {
    data class Network(val message: String, val status: Int?) : HttpError()
    data class InvalidUrl(val url: String, val reason: String) : HttpError()
    data class ResponseTooLarge(val size: Int, val max: Long) : HttpError()
    data class Timeout(val timeoutMs: Long, val requestId: String) : HttpError()
    data class DnsError(val host: String, val message: String) : HttpError()
    data class TlsError(val host: String, val message: String) : HttpError()
    data class ConnectionError(val host: String, val message: String) : HttpError()
}

// KV Capabilities
sealed class KvOperation {
    data class Get(val key: String) : KvOperation()
    data class Set(val key: String, val value: ByteArray) : KvOperation()
    data class Delete(val key: String) : KvOperation()
}

sealed class KvResult {
    data class Ok(val output: KvOutput) : KvResult()
    data class Error(val error: KvError) : KvResult()
}

sealed class KvOutput {
    data class Value(val value: ByteArray?) : KvOutput()
    object Written : KvOutput()
    object Deleted : KvOutput()
}

sealed class KvError {
    data class Storage(val code: StorageErrorCode, val message: String, val retryable: Boolean) : KvError()
}

enum class StorageErrorCode { IoError, Corrupted }

// Crypto Capabilities
sealed class CryptoOperation {
    data class Hash(val algorithm: HashAlgorithm, val data: ByteArray) : CryptoOperation()
    data class GenerateKey(val algorithm: KeyAlgorithm) : CryptoOperation()
    data class RandomBytes(val length: UInt) : CryptoOperation()
}

sealed class CryptoResult {
    data class Ok(val output: CryptoOutput) : CryptoResult()
    data class Error(val error: CryptoError) : CryptoResult()
}

sealed class CryptoOutput {
    data class Hash(val hash: ByteArray) : CryptoOutput()
    data class Key(val key: ByteArray) : CryptoOutput()
    data class RandomBytes(val bytes: ByteArray) : CryptoOutput()
}

sealed class CryptoError {
    data class Internal(val message: String) : CryptoError()
    data class NotSupported(val operation: String) : CryptoError()
}

enum class HashAlgorithm { Sha256, Sha384, Sha512 }
enum class KeyAlgorithm { Aes256 }

// Camera Capabilities
sealed class CameraOperation {
    object CheckPermission : CameraOperation()
    object RequestPermission : CameraOperation()
    object GetCapabilities : CameraOperation()
    data class CapturePhoto(val config: CaptureConfig) : CameraOperation()
    data class PickFromGallery(val config: GalleryPickConfig) : CameraOperation()
    object CancelPending : CameraOperation()
}

data class CaptureConfig(
    val facing: CameraFacing = CameraFacing.Back,
    val format: ImageFormat = ImageFormat.Jpeg,
    val quality: UInt = 80u,
    val maxWidth: UInt = 1920u,
    val maxHeight: UInt = 1080u,
    val flash: FlashMode = FlashMode.Off,
    val aspectRatio: AspectRatio = AspectRatio.Full,
    val stripMetadata: Boolean = true,
    val mirrorFrontCamera: Boolean = false,
    val timeoutMs: ULong = 30000uL,
    val maxFileSize: UInt = 10u * 1024u * 1024u
)

data class GalleryPickConfig(
    val allowMultiple: Boolean = false,
    val maxSelections: UInt = 1u,
    val format: ImageFormat = ImageFormat.Jpeg,
    val quality: UInt = 80u,
    val maxWidth: UInt = 1920u,
    val maxHeight: UInt = 1080u,
    val stripMetadata: Boolean = true,
    val maxFileSize: UInt = 10u * 1024u * 1024u
)

enum class CameraFacing { Front, Back }
enum class ImageFormat { Jpeg, Png, WebP }
enum class FlashMode { On, Off, Auto, Torch }
enum class AspectRatio { Square, FourThree, SixteenNine, Full }

data class CapturedImage(
    val data: ByteArray,
    val format: ImageFormat,
    val width: UInt,
    val height: UInt,
    val fileSize: ULong,
    val captureTimeMs: ULong
)

sealed class CameraResult {
    data class Ok(val output: CameraOutput) : CameraResult()
    data class Error(val error: CameraError) : CameraResult()
}

sealed class CameraOutput {
    data class PermissionStatus(val status: com.warmstreet.shared.PermissionStatus) : CameraOutput()
    data class Capabilities(val capabilities: CameraCapabilities) : CameraOutput()
    data class Photo(val image: CapturedImage) : CameraOutput()
    data class Photos(val images: List<CapturedImage>) : CameraOutput()
    object Cancelled : CameraOutput()
}

sealed class CameraError {
    data class Internal(val message: String) : CameraError()
    data class Unavailable(val reason: String) : CameraError()
    object PermissionDenied : CameraError()
    data class CaptureFailed(val reason: String) : CameraError()
}

enum class PermissionStatus { Granted, NotDetermined, Denied, DeniedPermanently }

data class CameraCapabilities(
    val hasFrontCamera: Boolean,
    val hasBackCamera: Boolean,
    val hasFlash: Boolean,
    val hasTorch: Boolean,
    val supportsHeic: Boolean,
    val supportsVideo: Boolean,
    val maxPhotoResolution: Any?,
    val isSimulator: Boolean,
    val platform: CameraPlatform
)

enum class CameraPlatform { Android, iOS }

// Push Capabilities
sealed class PushOperation {
    object RequestPermission : PushOperation()
    object RequestToken : PushOperation()
    object CheckPermission : PushOperation()
}

sealed class PushResult {
    data class Ok(val output: PushOutput) : PushResult()
    data class Error(val error: PushError) : PushResult()
}

sealed class PushOutput {
    data class Token(val token: String) : PushOutput()
    data class PermissionStatus(val status: com.warmstreet.shared.PermissionStatus) : PushOutput()
}

sealed class PushError {
    data class Internal(val message: String) : PushError()
    object NotSupported : PushError()
    data class RegistrationFailed(val message: String) : PushError()
}

// Location Capabilities
sealed class LocationOperation {
    object RequestPermission : LocationOperation()
    object GetLocation : LocationOperation()
}

sealed class LocationResult {
    data class Ok(val output: LocationOutput) : LocationResult()
    data class Error(val error: LocationError) : LocationResult()
}

data class LocationOutput(
    val status: PermissionStatus? = null,
    val location: Location? = null
) {
    companion object {
        fun PermissionStatus(fineLocation: Boolean, coarseLocation: Boolean) = LocationOutput(
            status = if (fineLocation || coarseLocation) PermissionStatus.Granted else PermissionStatus.Denied
        )
    }
}

sealed class LocationError {
    data class Internal(val message: String) : LocationError()
    object NotSupported : LocationError()
}
