package com.warmstreet.capabilities

import android.content.Context
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import com.warmstreet.shared.Event
import com.warmstreet.shared.KeyValueOperation
import com.warmstreet.shared.KeyValueOutput

class KeyValueHandler(context: Context) {
    private val masterKey = MasterKey.Builder(context)
        .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
        .build()

    private val sharedPreferences = EncryptedSharedPreferences.create(
        context,
        "warmstreet_secure_prefs",
        masterKey,
        EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
        EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
    )

    fun handle(operation: KeyValueOperation): Event {
        return when (operation) {
            is KeyValueOperation.Get -> {
                val value = sharedPreferences.getString(operation.key, null)?.toByteArray()
                // Assuming generic generated Value type or list<u8>
                 Event.KeyValueResult(KeyValueOutput.Get(value = value?.toList()))
            }
            is KeyValueOperation.Set -> {
                val strVal = String(operation.value.toByteArray()) // Simple string storage for now
                sharedPreferences.edit().putString(operation.key, strVal).apply()
                 Event.KeyValueResult(KeyValueOutput.Set)
            }
            is KeyValueOperation.Delete -> {
                sharedPreferences.edit().remove(operation.key).apply()
                 Event.KeyValueResult(KeyValueOutput.Delete)
            }
            else -> Event.KeyValueResult(KeyValueOutput.Failure("Unsupported operation"))
        }
    }
}
