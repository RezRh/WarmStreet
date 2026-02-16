package com.warmstreet.capabilities

import android.content.Context
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import com.warmstreet.shared.Event
import com.warmstreet.shared.KvOperation
import com.warmstreet.shared.KvOutput
import com.warmstreet.shared.KvResult
import com.warmstreet.shared.StorageErrorCode

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

    fun handle(operation: KvOperation): KvResult {
        return when (operation) {
            is KvOperation.Get -> {
                val value = sharedPreferences.getString(operation.key, null)?.toByteArray()
                KvResult.Ok(KvOutput.Value(value))
            }
            is KvOperation.Set -> {
                val strVal = String(operation.value)
                sharedPreferences.edit().putString(operation.key, strVal).apply()
                KvResult.Ok(KvOutput.Written)
            }
            is KvOperation.Delete -> {
                sharedPreferences.edit().remove(operation.key).apply()
                KvResult.Ok(KvOutput.Deleted)
            }
        }
    }
}
