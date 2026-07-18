package com.solosoul.app

import android.app.Activity
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyPermanentlyInvalidatedException
import android.security.keystore.KeyProperties
import androidx.biometric.BiometricPrompt
import androidx.core.content.ContextCompat
import androidx.fragment.app.FragmentActivity
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import app.tauri.plugin.Invoke
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

@InvokeArg
class AuthenticateAndSaveArgs {
    lateinit var alias: String
    lateinit var data: String
    var title: String? = null
    var subtitle: String? = null
    var cancelTitle: String? = null
}

@InvokeArg
class AuthenticateAndReadArgs {
    lateinit var alias: String
    lateinit var iv: String
    lateinit var ciphertext: String
    var title: String? = null
    var subtitle: String? = null
    var cancelTitle: String? = null
}

@InvokeArg
class KeystoreDeleteArgs {
    lateinit var alias: String
}

/**
 * Android Keystore 生物识别凭证安全存储插件。
 *
 * 提供以下能力：
 * - authenticateAndSave：通过 BiometricPrompt + CryptoObject 加密数据，密钥受生物识别保护。
 * - authenticateAndRead：通过 BiometricPrompt + CryptoObject 解密数据。
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
    fun authenticateAndSave(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(AuthenticateAndSaveArgs::class.java)
            val alias = normalizeAlias(args.alias)
            val plaintext = args.data.toByteArray(Charsets.UTF_8)

            val secretKey = getOrCreateKey(alias)
            val cipher = Cipher.getInstance(TRANSFORMATION)
            cipher.init(Cipher.ENCRYPT_MODE, secretKey)

            showBiometricPrompt(
                cipher = cipher,
                title = args.title ?: "SoloSoul",
                subtitle = args.subtitle ?: "Verify your identity",
                cancelTitle = args.cancelTitle ?: "Cancel",
                onSuccess = { authenticatedCipher ->
                    try {
                        val finalCipher = authenticatedCipher ?: cipher
                        val iv = finalCipher.iv
                        val ciphertext = finalCipher.doFinal(plaintext)
                        val result = JSObject().apply {
                            put("iv", bytesToHex(iv))
                            put("ciphertext", bytesToHex(ciphertext))
                        }
                        invoke.resolve(result)
                    } catch (e: KeyPermanentlyInvalidatedException) {
                        // 生物识别数据变更导致密钥失效，提示用户重新启用生物识别
                        android.util.Log.w("SoloSoul", "Keystore key invalidated by biometric enrollment change")
                        invoke.reject("BIOMETRIC_KEY_INVALIDATED")
                    } catch (e: Exception) {
                        android.util.Log.e("SoloSoul", "Keystore save failed: ${e.message}", e)
                        invoke.reject("Keystore save failed: ${e.message}")
                    }
                },
                onError = { error ->
                    invoke.reject(error)
                }
            )
        } catch (e: Exception) {
            android.util.Log.e("SoloSoul", "Keystore save setup failed: ${e.message}", e)
            invoke.reject("Keystore save setup failed: ${e.message}")
        }
    }

    @Command
    fun authenticateAndRead(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(AuthenticateAndReadArgs::class.java)
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

            showBiometricPrompt(
                cipher = cipher,
                title = args.title ?: "SoloSoul",
                subtitle = args.subtitle ?: "Unlock with biometric authentication",
                cancelTitle = args.cancelTitle ?: "Cancel",
                onSuccess = { authenticatedCipher ->
                    try {
                        val finalCipher = authenticatedCipher ?: cipher
                        val plaintext = finalCipher.doFinal(ciphertext)
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
                },
                onError = { error ->
                    invoke.reject(error)
                }
            )
        } catch (e: Exception) {
            android.util.Log.e("SoloSoul", "Keystore read setup failed: ${e.message}", e)
            invoke.reject("Keystore read setup failed: ${e.message}")
        }
    }

    @Command
    fun delete(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(KeystoreDeleteArgs::class.java)
            val alias = normalizeAlias(args.alias)
            deleteKey(alias)
            invoke.resolve(JSObject())
        } catch (e: Exception) {
            android.util.Log.e("SoloSoul", "Keystore delete failed: ${e.message}", e)
            invoke.reject("Keystore delete failed: ${e.message}")
        }
    }

    private fun showBiometricPrompt(
        cipher: Cipher,
        title: String,
        subtitle: String,
        cancelTitle: String,
        onSuccess: (Cipher?) -> Unit,
        onError: (String) -> Unit,
    ) {
        val fragmentActivity = activity as? FragmentActivity
        if (fragmentActivity == null) {
            onError("Biometric prompt requires FragmentActivity")
            return
        }

        activity.runOnUiThread {
            val executor = ContextCompat.getMainExecutor(activity)
            val cryptoObject = BiometricPrompt.CryptoObject(cipher)

            val promptInfo = BiometricPrompt.PromptInfo.Builder()
                .setTitle(title)
                .setSubtitle(subtitle)
                .setNegativeButtonText(cancelTitle)
                .setConfirmationRequired(false)
                .setAllowedAuthenticators(0x0F) /* Authenticators.BIOMETRIC_STRONG */
                .build()

            val prompt = BiometricPrompt(
                fragmentActivity,
                executor,
                object : BiometricPrompt.AuthenticationCallback() {
                    override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                        val authenticatedCipher = result.cryptoObject?.cipher
                        onSuccess(authenticatedCipher)
                    }

                    override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                        when (errorCode) {
                            BiometricPrompt.ERROR_CANCELED,
                            BiometricPrompt.ERROR_USER_CANCELED,
                            BiometricPrompt.ERROR_NEGATIVE_BUTTON -> {
                                onError("BIOMETRIC_CANCELLED")
                            }
                            BiometricPrompt.ERROR_NO_BIOMETRICS -> {
                                onError("BIOMETRIC_NOT_ENROLLED")
                            }
                            BiometricPrompt.ERROR_LOCKOUT,
                            BiometricPrompt.ERROR_LOCKOUT_PERMANENT -> {
                                onError("BIOMETRIC_LOCKOUT")
                            }
                            BiometricPrompt.ERROR_HW_NOT_PRESENT,
                            BiometricPrompt.ERROR_HW_UNAVAILABLE,
                            BiometricPrompt.ERROR_NO_DEVICE_CREDENTIAL -> {
                                onError("BIOMETRIC_UNAVAILABLE")
                            }
                            else -> {
                                onError("BIOMETRIC_ERROR: $errorCode - $errString")
                            }
                        }
                    }
                }
            )

            prompt.authenticate(promptInfo, cryptoObject)
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

    private fun deleteKey(alias: String) {
        try {
            val keyStore = KeyStore.getInstance(ANDROID_KEYSTORE)
            keyStore.load(null)
            if (keyStore.containsAlias(alias)) {
                keyStore.deleteEntry(alias)
            }
        } catch (e: Exception) {
            android.util.Log.w("SoloSoul", "Failed to delete Keystore key: ${e.message}")
        }
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
