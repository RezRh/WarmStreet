package com.warmstreet.capabilities

import com.warmstreet.shared.Event
import com.warmstreet.shared.HttpRequest
import com.warmstreet.shared.HttpResult
import okhttp3.MediaType.Companion.toMediaTypeOrNull
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import java.io.IOException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

class HttpHandler {
    private val client = OkHttpClient()

    suspend fun handle(request: HttpRequest): Event = withContext(Dispatchers.IO) {
        val builder = Request.Builder()
            .url(request.url)

        request.headers.forEach { header ->
            builder.addHeader(header.name, header.value)
        }

        if (request.method == "GET") {
            builder.get()
        } else if (request.method == "POST") {
            val body = request.body.toRequestBody("application/json; charset=utf-8".toMediaTypeOrNull())
            builder.post(body)
        }
        // Handle other methods...

        try {
            val response = client.newCall(builder.build()).execute()
            if (response.isSuccessful) {
                val bodyBytes = response.body?.bytes() ?: ByteArray(0)
                // Return success event
                // Event.HttpResult(HttpResult.Ok(...))
                // Stubbing return based on generated code expectations
                Event.HttpResult(HttpResult.Success(status = response.code.toShort(), body = bodyBytes.toList()))
            } else {
                 Event.HttpResult(HttpResult.Failure(error = "HTTP ${response.code}"))
            }
        } catch (e: IOException) {
            Event.HttpResult(HttpResult.Failure(error = e.message ?: "Unknown IO Error"))
        }
    }
}
