package com.warmstreet.capabilities

import com.warmstreet.shared.*
import okhttp3.HttpUrl.Companion.toHttpUrlOrNull
import okhttp3.MediaType.Companion.toMediaTypeOrNull
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

class HttpHandler {
    private val client = okhttp3.OkHttpClient.Builder()
        .connectTimeout(30, java.util.concurrent.TimeUnit.SECONDS)
        .readTimeout(30, java.util.concurrent.TimeUnit.SECONDS)
        .build()

    suspend fun handle(operation: HttpOperation): HttpResult = withContext(Dispatchers.IO) {
        when (operation) {
            is HttpOperation.Execute -> executeRequest(operation.request)
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

        return try {
            val response = client.newCall(requestBuilder.build()).execute()
            val responseHeaders = HttpHeaders()
            for ((name, value) in response.headers) {
                try { responseHeaders.insert(name, value) } catch (e: Exception) {}
            }
            HttpResult.Ok(
                HttpResponse(
                    status = response.code.toUShort(),
                    headers = responseHeaders,
                    body = response.body?.bytes() ?: ByteArray(0),
                    requestId = request.requestId,
                    durationMs = 0uL
                )
            )
        } catch (e: Exception) {
            HttpResult.Error(HttpError.Network(e.message ?: "Network error", null))
        }
    }

    fun close() {
        client.dispatcher.executorService.shutdown()
        client.connectionPool.evictAll()
    }
}
