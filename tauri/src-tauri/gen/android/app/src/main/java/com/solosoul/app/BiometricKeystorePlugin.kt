package com.solosoul.app

import android.app.Activity
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyPermanentlyInvalidatedException
import android.security.keystore.KeyProperties
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import app.tauri.plugin.Invoke
import java.nio.ByteBuffer
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

@InvokeArg
class KeystoreSaveArgs {
    lateinit var alias: String
    lateinit var data: String
}

@InvokeArg
class KeystoreReadArgs {
    lateinit var alias: String
    lateinit var iv: String
    lateinit var ciphertext: String
}

@InvokeArg
class KeystoreDeleteArgs {
    lateinit var alias: String
}

/**
 * Android Keystore 生物识别凭证安全存储插件。
 *
 * 提供以下能力：
 * - save：使用 Android Keystore 中受生物识别保护的 AES 密钥加密数据，返回 IV 和密文。
 * - read：使用 Android Keystore 中的密钥解密数据。
 * - delete：删除 Keystore 中的密钥别名。
 *
 * 密钥生成时启用 setInvalidatedByBiometricEnrollment(true)，
 * 当用户新增/删除指纹或人脸时，旧密钥会永久失效，从而阻止攻击者
 * 用旧生物识别数据解密凭证。
 */
@TauriPlugin
class BiometricKeystorePlugin(private val activity: Activity): Plugin(activity) {

    companion object {
        private const val ANDROID_KEYSTORE = "AndroidKeyStore"
        private const val TRANSFORMATION = "AES/GCM/NoPadding"
        private const val GCM_TAG_LENGTH = 128
    }

    @Command
    fun save(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(KeystoreSaveArgs::class.java)
            val alias = normalizeAlias(args.alias)
            val plaintext = args.data.toByteArray(Charsets.UTF_8)

            val secretKey = getOrCreateKey(alias)
            val cipher = Cipher.getInstance(TRANSFORMATION)
            cipher.init(Cipher.ENCRYPT_MODE, secretKey)

            val iv = cipher.iv
            val ciphertext = cipher.doFinal(plaintext)

            val result = JSObject().apply {
                put("iv", bytesToHex(iv))
                put("ciphertext", bytesToHex(ciphertext))
            }
            invoke.resolve(result)
        } catch (e: Exception) {
            android.util.Log.e("SoloSoul", "Keystore save failed: ${e.message}", e)
            invoke.reject("Keystore save failed: ${e.message}")
        }
    }

    @Command
    fun read(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(KeystoreReadArgs::class.java)
            val alias = normalizeAlias(args.alias)
            val iv = hexToBytes(args.iv)
            val ciphertext = hexToBytes(args.ciphertext)

            val secretKey = getKey(alias)
                ?: run {
                    invoke.reject("BIOMETRIC_KEY_NOT_FOUND")
                    return
                }

            val cipher = Cipher.getInstance(TRANSFORMATION)
            val spec = GCMParameterSpec(GCM_TAG_LENGTH, iv)
            cipher.init(Cipher.DECRYPT_MODE, secretKey, spec)

            val plaintext = cipher.doFinal(ciphertext)
            val result = JSObject().apply {
                put("data", String(plaintext, Charsets.UTF_8))
            }
            invoke.resolve(result)
        } catch (e: KeyPermanentlyInvalidatedException) {
            android.util.Log.w("SoloSoul", "Keystore key invalidated by biometric enrollment change")
            invoke.reject("BIOMETRIC_KEY_INVALIDATED")
        } catch (e: Exception) {
            android.util.Log.e("SoloSoul", "Keystore read failed: ${e.message}", e)
            invoke.reject("Keystore read failed: ${e.message}")
        }
    }

    @Command
    fun delete(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(KeystoreDeleteArgs::class.java)
            val alias = normalizeAlias(args.alias)
            val keyStore = KeyStore.getInstance(ANDROID_KEYSTORE)
            keyStore.load(null)
            if (keyStore.containsAlias(alias)) {
                keyStore.deleteEntry(alias)
            }
            invoke.resolve(JSObject())
        } catch (e: Exception) {
            android.util.Log.e("SoloSoul", "Keystore delete failed: ${e.message}", e)
            invoke.reject("Keystore delete failed: ${e.message}")
        }
    }

    private fun normalizeAlias(alias: String): String {
        // Android Keystore 别名限制：仅允许字母、数字、下划线、点
        return alias.replace(Regex("[^a-zA-Z0-9_.]"), "_")
    }

    private fun getOrCreateKey(alias: String): SecretKey {
        getKey(alias)?.let { return it }

        val keyGenerator = KeyGenerator.getInstance(
            KeyProperties.KEY_ALGORITHM_AES,
            ANDROID_KEYSTORE
        )
        keyGenerator.init(
            KeyGenParameterSpec.Builder(
                alias,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setUserAuthenticationRequired(true)
                .setInvalidatedByBiometricEnrollment(true)
                .setRandomizedEncryptionRequired(true)
                .build()
        )
        return keyGenerator.generateKey()
    }

    private fun getKey(alias: String): SecretKey? {
        val keyStore = KeyStore.getInstance(ANDROID_KEYSTORE)
        keyStore.load(null)
        val entry = keyStore.getEntry(alias, null) as? KeyStore.SecretKeyEntry
        return entry?.secretKey
    }

    private fun bytesToHex(bytes: ByteArray): String {
        return bytes.joinToString("") { "%02x".format(it) }
    }

    private fun hexToBytes(hex: String): ByteArray {
        val len = hex.length
        val data = ByteArray(len / 2)
        var i = 0
        while (i < len) {
            data[i / 2] = ((Character.digit(hex[i], 16) shl 4) +
                    Character.digit(hex[i + 1], 16)).toByte()
            i += 2
        }
        return data
    }
}
