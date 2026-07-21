package com.solosoul.app

import android.app.Activity
import android.content.pm.PackageManager
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyPermanentlyInvalidatedException
import android.security.keystore.KeyProperties
import androidx.biometric.BiometricManager
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
 * - authenticateAndSave：加密数据并持久化。
 * - authenticateAndRead：解密已保存的数据。
 * - delete：删除 Keystore 中的密钥别名。
 *
 * 双路径设计：
 * - Class 3（指纹/强人脸）：BiometricPrompt + CryptoObject，密钥受生物识别保护，
 *   提示框仅允许 BIOMETRIC_STRONG（keystore 密钥只能被 Class 3 解锁）。
 *   密钥启用 setInvalidatedByBiometricEnrollment(true)，生物识别数据变更即失效。
 * - 仅 Class 2（弱人脸）：普通 BiometricPrompt（无 CryptoObject）+ 免授权密钥
 *   （{alias}_weak，setUserAuthenticationRequired(false)），平台限制下
 *   弱生物识别无法解锁 keystore，安全性差异由 UI 弱生物识别警告告知。
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

            when {
                isStrongAvailable() -> {
                    // Class 3：CryptoObject 绑定生物识别。提示框必须只允许 STRONG——
                    // keystore 密钥只能被 Class 3 解锁，若允许 Class 2 人脸通过提示，
                    // doFinal 会因密钥未解锁抛 UserNotAuthenticatedException
                    val secretKey = getOrCreateKey(alias)
                    val cipher = Cipher.getInstance(TRANSFORMATION)
                    cipher.init(Cipher.ENCRYPT_MODE, secretKey)

                    showBiometricPrompt(
                        cipher = cipher,
                        useCryptoObject = true,
                        allowedAuthenticators = BiometricManager.Authenticators.BIOMETRIC_STRONG,
                        title = args.title ?: "SoloSoul",
                        subtitle = args.subtitle ?: "Verify your identity",
                        cancelTitle = args.cancelTitle ?: "Cancel",
                        onSuccess = { authenticatedCipher ->
                            resolveEncrypted(invoke, authenticatedCipher ?: cipher, plaintext)
                        },
                        onError = { error -> invoke.reject(error) }
                    )
                }
                isWeakAvailable() -> {
                    // 仅 Class 2：keystore 密钥无法由弱生物识别解锁（平台限制），
                    // 改用普通提示 + 免授权密钥；安全性差异由 UI 弱生物识别警告告知
                    val secretKey = getOrCreateWeakKey(alias)
                    val cipher = Cipher.getInstance(TRANSFORMATION)
                    cipher.init(Cipher.ENCRYPT_MODE, secretKey)

                    showBiometricPrompt(
                        cipher = cipher,
                        useCryptoObject = false,
                        allowedAuthenticators = BiometricManager.Authenticators.BIOMETRIC_WEAK,
                        title = args.title ?: "SoloSoul",
                        subtitle = args.subtitle ?: "Verify your identity",
                        cancelTitle = args.cancelTitle ?: "Cancel",
                        onSuccess = { resolveEncrypted(invoke, cipher, plaintext) },
                        onError = { error -> invoke.reject(error) }
                    )
                }
                else -> invoke.reject("BIOMETRIC_UNAVAILABLE")
            }
        } catch (e: Exception) {
            android.util.Log.e("SoloSoul", "Keystore save setup failed: ${e.message}", e)
            invoke.reject("Keystore save setup failed: ${e.message}")
        }
    }

    private fun resolveEncrypted(invoke: Invoke, cipher: Cipher, plaintext: ByteArray) {
        try {
            val iv = cipher.iv
            val ciphertext = cipher.doFinal(plaintext)
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
    }

    @Command
    fun authenticateAndRead(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(AuthenticateAndReadArgs::class.java)
            val alias = normalizeAlias(args.alias)
            val iv = hexToBytes(args.iv)
            val ciphertext = hexToBytes(args.ciphertext)

            when {
                isStrongAvailable() -> {
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
                        useCryptoObject = true,
                        allowedAuthenticators = BiometricManager.Authenticators.BIOMETRIC_STRONG,
                        title = args.title ?: "SoloSoul",
                        subtitle = args.subtitle ?: "Unlock with biometric authentication",
                        cancelTitle = args.cancelTitle ?: "Cancel",
                        onSuccess = { authenticatedCipher ->
                            resolveDecrypted(invoke, authenticatedCipher ?: cipher, ciphertext)
                        },
                        onError = { error -> invoke.reject(error) }
                    )
                }
                isWeakAvailable() -> {
                    val secretKey = getKey(weakAlias(alias))
                        ?: run {
                            invoke.reject("BIOMETRIC_KEY_NOT_FOUND")
                            return
                        }
                    val cipher = Cipher.getInstance(TRANSFORMATION)
                    val spec = GCMParameterSpec(GCM_TAG_LENGTH, iv)
                    cipher.init(Cipher.DECRYPT_MODE, secretKey, spec)

                    showBiometricPrompt(
                        cipher = cipher,
                        useCryptoObject = false,
                        allowedAuthenticators = BiometricManager.Authenticators.BIOMETRIC_WEAK,
                        title = args.title ?: "SoloSoul",
                        subtitle = args.subtitle ?: "Unlock with biometric authentication",
                        cancelTitle = args.cancelTitle ?: "Cancel",
                        onSuccess = { resolveDecrypted(invoke, cipher, ciphertext) },
                        onError = { error -> invoke.reject(error) }
                    )
                }
                else -> invoke.reject("BIOMETRIC_UNAVAILABLE")
            }
        } catch (e: Exception) {
            android.util.Log.e("SoloSoul", "Keystore read setup failed: ${e.message}", e)
            invoke.reject("Keystore read setup failed: ${e.message}")
        }
    }

    private fun resolveDecrypted(invoke: Invoke, cipher: Cipher, ciphertext: ByteArray) {
        try {
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
            deleteKey(alias)
            invoke.resolve(JSObject())
        } catch (e: Exception) {
            android.util.Log.e("SoloSoul", "Keystore delete failed: ${e.message}", e)
            invoke.reject("Keystore delete failed: ${e.message}")
        }
    }

    @Command
    fun checkBiometricAvailability(invoke: Invoke) {
        try {
            val strongAvailable = isStrongAvailable()
            // 如果强生物识别不可用，检查弱生物识别（Class 2：前置摄像头人脸）
            val weakAvailable = if (!strongAvailable) isWeakAvailable() else false

            val result = JSObject()
            result.put("strongAvailable", strongAvailable)
            result.put("weakAvailable", weakAvailable)
            // 诊断字段：排查 Class 2 人脸设备上的可用性判定（adb logcat -s SoloSoul 可见）
            result.put("sdkInt", Build.VERSION.SDK_INT)
            result.put(
                "faceFeature",
                activity.packageManager.hasSystemFeature(PackageManager.FEATURE_FACE)
            )
            result.put(
                "weakRaw",
                rawCanAuthenticate(BiometricManager.Authenticators.BIOMETRIC_WEAK)
            )
            result.put(
                "strongRaw",
                rawCanAuthenticate(BiometricManager.Authenticators.BIOMETRIC_STRONG)
            )
            android.util.Log.i("SoloSoul", "checkBiometricAvailability: $result")
            invoke.resolve(result)
        } catch (e: Exception) {
            android.util.Log.e("SoloSoul", "checkBiometricAvailability failed: ${e.message}", e)
            invoke.reject("checkBiometricAvailability failed: ${e.message}")
        }
    }

    private fun rawCanAuthenticate(authenticators: Int): Int {
        return try {
            BiometricManager.from(activity).canAuthenticate(authenticators)
        } catch (_: Exception) {
            -1
        }
    }

    private fun canAuthenticate(authenticators: Int): Boolean {
        return try {
            BiometricManager.from(activity).canAuthenticate(authenticators) ==
                BiometricManager.BIOMETRIC_SUCCESS
        } catch (_: Exception) {
            false
        }
    }

    private fun isStrongAvailable(): Boolean =
        canAuthenticate(BiometricManager.Authenticators.BIOMETRIC_STRONG)

    private fun isWeakAvailable(): Boolean {
        if (canAuthenticate(BiometricManager.Authenticators.BIOMETRIC_WEAK)) return true
        // API <30 的 androidx BiometricManager 无法检测 Class 2 人脸（退化为指纹检查），
        // 用系统特性兜底：有 Face 硬件即认为可能可用；
        // 若用户实际未录入，保存/解锁时提示框会返回 BIOMETRIC_NOT_ENROLLED
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.R) {
            return activity.packageManager.hasSystemFeature(PackageManager.FEATURE_FACE)
        }
        return false
    }

    private fun showBiometricPrompt(
        cipher: Cipher,
        useCryptoObject: Boolean,
        allowedAuthenticators: Int,
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

            val promptInfo = BiometricPrompt.PromptInfo.Builder()
                .setTitle(title)
                .setSubtitle(subtitle)
                .setNegativeButtonText(cancelTitle)
                .setConfirmationRequired(false)
                .setAllowedAuthenticators(allowedAuthenticators)
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

            if (useCryptoObject) {
                prompt.authenticate(promptInfo, BiometricPrompt.CryptoObject(cipher))
            } else {
                prompt.authenticate(promptInfo)
            }
        }
    }

    private fun normalizeAlias(alias: String): String {
        // Android Keystore 别名限制：仅允许字母、数字、下划线、点
        return alias.replace(Regex("[^a-zA-Z0-9_.]"), "_")
    }

    /** Class 2（弱生物识别）路径使用独立别名，与 Class 3 的授权绑定密钥隔离。 */
    private fun weakAlias(alias: String): String = "${alias}_weak"

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

    private fun getOrCreateWeakKey(alias: String): SecretKey {
        getKey(weakAlias(alias))?.let { return it }

        // Class 2 路径：密钥不绑定用户授权（弱生物识别无法解锁 keystore），
        // 仅作为应用内加密材料，访问由普通生物识别提示把关
        val keyGenerator = KeyGenerator.getInstance(
            KeyProperties.KEY_ALGORITHM_AES,
            ANDROID_KEYSTORE
        )
        keyGenerator.init(
            KeyGenParameterSpec.Builder(
                weakAlias(alias),
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setUserAuthenticationRequired(false)
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
            // 同时清理 Class 2 路径的免授权密钥
            val weak = weakAlias(alias)
            if (keyStore.containsAlias(weak)) {
                keyStore.deleteEntry(weak)
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
