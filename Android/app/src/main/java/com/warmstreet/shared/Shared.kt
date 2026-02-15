package com.warmstreet.shared

// Generics and core types
class App {
    constructor()
    fun view(): ViewModel = ViewModel.Loading
    suspend fun update(event: Event): List<Effect> = emptyList()
}

abstract class ViewModel {
    object Loading : ViewModel()
    data class Error(val message: String) : ViewModel()
    // Add other view states as needed
}

sealed class Event {
    data class SystemError(val message: String, val source: String) : Event()
    // Add other events as needed
}

sealed class Effect {
    object Render : Effect()
    data class Http(val operation: HttpOperation, val callback: (HttpResult) -> Event) : Effect()
    data class Kv(val operation: KvOperation, val callback: (KvResult) -> Event) : Effect()
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

// Url wrapper to match usage url.asStr()
data class UrlWrapper(private val url: String) {
    fun asStr(): String = url
}

// HttpHeaders
class HttpHeaders {
    private val headers = mutableListOf<Pair<String, String>>()
    
    fun iter(): Iterator<Pair<String, String>> = headers.iterator()
    fun get(name: String): String? = headers.find { it.first.equals(name, ignoreCase = true) }?.second
    fun insert(name: String, value: String) {
        headers.add(name to value)
    }
}

enum class HttpMethod {
    Get, Post, Put, Patch, Delete, Head, Options
}

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
    data class Get(val key: KvKey) : KvOperation()
    data class Set(val key: KvKey, val value: ByteArray, val ifVersion: Long?) : KvOperation()
    data class Delete(val key: KvKey, val ifVersion: Long?) : KvOperation()
    data class Exists(val key: KvKey) : KvOperation()
    data class List(val namespace: KeyNamespace, val prefix: String?, val limit: UInt, val cursor: String?) : KvOperation()
    data class GetMulti(val keys: kotlin.collections.List<KvKey>) : KvOperation()
    data class DeleteMulti(val keys: kotlin.collections.List<KvKey>) : KvOperation()
}

data class KvKey(private val key: String) {
    fun raw(): String = key
}

data class KeyNamespace(private val prefix: String) {
    fun prefix(): String = prefix
}

sealed class KvResult {
    data class Ok(val output: KvOutput) : KvResult()
    data class Error(val error: KvError) : KvResult()
}

sealed class KvOutput {
    data class Value(val value: KvValue?) : KvOutput()
    data class Written(val version: ULong) : KvOutput()
    data class Deleted(val existed: Boolean) : KvOutput()
    data class Exists(val exists: Boolean) : KvOutput()
    data class List(val entries: kotlin.collections.List<KvListEntry>, val nextCursor: String?, val hasMore: Boolean) : KvOutput()
    data class Multi(val values: kotlin.collections.List<KvValue?>) : KvOutput()
    data class DeletedMulti(val deletedCount: ULong) : KvOutput()
}

data class KvValue(
    val data: ByteArray,
    val version: ULong,
    val createdAt: ULong,
    val updatedAt: ULong
)

data class KvListEntry(
    val key: String,
    val version: ULong,
    val size: ULong,
    val updatedAt: ULong
)

sealed class KvError {
    data class Storage(val code: StorageErrorCode, val message: String, val retryable: Boolean) : KvError()
    data class VersionMismatch(val expected: ULong, val found: ULong) : KvError()
}

enum class StorageErrorCode {
    IoError, Corrupted
}

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

enum class HashAlgorithm {
    Sha256, Sha384, Sha512
}

enum class KeyAlgorithm {
    Aes256
}

// Camera Capabilities
sealed class CameraOperation {
    object CheckPermission : CameraOperation()
    object RequestPermission : CameraOperation()
    object GetCapabilities : CameraOperation()
    data class CapturePhoto(val config: CaptureConfig) : CameraOperation()
    data class PickFromGallery(val config: GalleryPickConfig) : CameraOperation()
    object CancelPending : CameraOperation()
}

data class CaptureConfig(val todo: Any? = null)
data class GalleryPickConfig(val todo: Any? = null)

sealed class CameraResult {
    data class Ok(val output: CameraOutput) : CameraResult()
    data class Error(val error: CameraError) : CameraResult()
}

sealed class CameraOutput {
    data class PermissionStatus(val status: com.warmstreet.shared.PermissionStatus) : CameraOutput()
    data class Capabilities(val capabilities: CameraCapabilities) : CameraOutput()
    object Cancelled : CameraOutput()
}

sealed class CameraError {
    data class Internal(val message: String) : CameraError()
    data class Unavailable(val reason: String) : CameraError()
}

enum class PermissionStatus {
    Granted, NotDetermined, Denied
}

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

enum class CameraPlatform {
    Android, iOS
}

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
}

sealed class LocationResult {
    data class Ok(val output: LocationOutput) : LocationResult()
    data class Error(val error: LocationError) : LocationResult()
}

class LocationOutput // Empty

sealed class LocationError {
    data class Internal(val message: String) : LocationError()
    object NotSupported : LocationError()
}
