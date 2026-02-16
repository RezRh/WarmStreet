package com.warmstreet.capabilities

import com.warmstreet.shared.*
import java.security.MessageDigest
import java.security.SecureRandom
import javax.crypto.KeyGenerator

class CryptoHandler {

    suspend fun handle(operation: CryptoOperation): CryptoResult {
        return try {
            when (operation) {
                is CryptoOperation.Hash -> hash(operation)
                is CryptoOperation.GenerateKey -> generateKey(operation)
                is CryptoOperation.RandomBytes -> randomBytes(operation)
            }
        } catch (e: Exception) {
            CryptoResult.Error(CryptoError.Internal(e.message ?: "Crypto error"))
        }
    }

    private fun hash(op: CryptoOperation.Hash): CryptoResult {
        val algorithm = when (op.algorithm) {
            HashAlgorithm.Sha256 -> "SHA-256"
            HashAlgorithm.Sha384 -> "SHA-384"
            HashAlgorithm.Sha512 -> "SHA-512"
        }
        val digest = MessageDigest.getInstance(algorithm)
        val hash = digest.digest(op.data)
        return CryptoResult.Ok(CryptoOutput.Hash(hash))
    }

    private fun generateKey(op: CryptoOperation.GenerateKey): CryptoResult {
        val algorithm = "AES" // op.algorithm is KeyAlgorithm.Aes256
        val keyGen = KeyGenerator.getInstance(algorithm)
        keyGen.init(256)
        val key = keyGen.generateKey()
        return CryptoResult.Ok(CryptoOutput.Key(key.encoded))
    }

    private fun randomBytes(op: CryptoOperation.RandomBytes): CryptoResult {
        val bytes = ByteArray(op.length.toInt())
        SecureRandom().nextBytes(bytes)
        return CryptoResult.Ok(CryptoOutput.RandomBytes(bytes))
    }
}
