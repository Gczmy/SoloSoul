package com.solosoul.solosoul_flutter

import android.content.Context
import android.content.SharedPreferences
import android.os.Build
import android.os.Bundle
import android.view.WindowManager
import androidx.biometric.BiometricManager
import androidx.biometric.BiometricPrompt
import androidx.core.content.ContextCompat
import androidx.fragment.app.FragmentActivity
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import io.flutter.embedding.android.FlutterFragmentActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel
import java.util.concurrent.Executor

class MainActivity : FlutterFragmentActivity() {
    private val channelName = "com.solosoul/keychain"
    private lateinit var methodChannel: MethodChannel

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.setFlags(
            WindowManager.LayoutParams.FLAG_SECURE,
            WindowManager.LayoutParams.FLAG_SECURE
        )
    }

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)

        methodChannel = MethodChannel(flutterEngine.dartExecutor.binaryMessenger, channelName)
        methodChannel.setMethodCallHandler { call, result ->
            when (call.method) {
                "authenticateWithBiometrics" -> authenticateWithBiometrics(result)
                "saveToKeychain" -> {
                    val key = call.argument<String>("key")
                    val value = call.argument<String>("value")
                    if (key != null && value != null) {
                        saveToKeychain(key, value, result)
                    } else {
                        result.error("INVALID_ARGS", "Key and value are required", null)
                    }
                }
                "readFromKeychain" -> {
                    val key = call.argument<String>("key")
                    if (key != null) {
                        readFromKeychain(key, result)
                    } else {
                        result.error("INVALID_ARGS", "Key is required", null)
                    }
                }
                "deleteFromKeychain" -> {
                    val key = call.argument<String>("key")
                    if (key != null) {
                        deleteFromKeychain(key, result)
                    } else {
                        result.error("INVALID_ARGS", "Key is required", null)
                    }
                }
                else -> result.notImplemented()
            }
        }
    }

    private fun authenticateWithBiometrics(result: MethodChannel.Result) {
        val biometricManager = BiometricManager.from(this)
        val canAuthenticate = biometricManager.canAuthenticate(BiometricManager.Authenticators.BIOMETRIC_STRONG)

        when (canAuthenticate) {
            BiometricManager.BIOMETRIC_SUCCESS -> {
                val executor: Executor = ContextCompat.getMainExecutor(this)
                val biometricPrompt = BiometricPrompt(this, executor,
                    object : BiometricPrompt.AuthenticationCallback() {
                        override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                            super.onAuthenticationError(errorCode, errString)
                            result.success(mapOf(
                                "success" to false,
                                "error" to errString.toString(),
                                "biometryType" to getBiometryType()
                            ))
                        }

                        override fun onAuthenticationSucceeded(authResult: BiometricPrompt.AuthenticationResult) {
                            super.onAuthenticationSucceeded(authResult)
                            result.success(mapOf(
                                "success" to true,
                                "biometryType" to getBiometryType()
                            ))
                        }

                        override fun onAuthenticationFailed() {
                            super.onAuthenticationFailed()
                            // Don't send result here - let user retry
                        }
                    }
                )

                val promptInfo = BiometricPrompt.PromptInfo.Builder()
                    .setTitle("Biometric Authentication")
                    .setSubtitle("Authenticate to access sensitive data")
                    .setNegativeButtonText("Cancel")
                    .setAllowedAuthenticators(BiometricManager.Authenticators.BIOMETRIC_STRONG)
                    .build()

                biometricPrompt.authenticate(promptInfo)
            }
            BiometricManager.BIOMETRIC_ERROR_NO_HARDWARE -> {
                result.success(mapOf(
                    "success" to false,
                    "error" to "No biometric hardware available",
                    "biometryType" to "none"
                ))
            }
            BiometricManager.BIOMETRIC_ERROR_HW_UNAVAILABLE -> {
                result.success(mapOf(
                    "success" to false,
                    "error" to "Biometric hardware unavailable",
                    "biometryType" to "none"
                ))
            }
            BiometricManager.BIOMETRIC_ERROR_NONE_ENROLLED -> {
                result.success(mapOf(
                    "success" to false,
                    "error" to "No biometrics enrolled",
                    "biometryType" to getBiometryType()
                ))
            }
            else -> {
                result.success(mapOf(
                    "success" to false,
                    "error" to "Unknown biometric status",
                    "biometryType" to "none"
                ))
            }
        }
    }

    private fun getBiometryType(): String {
        val biometricManager = BiometricManager.from(this)
        if (biometricManager.canAuthenticate(BiometricManager.Authenticators.BIOMETRIC_STRONG) == BiometricManager.BIOMETRIC_SUCCESS) {
            // Android doesn't provide a direct way to query specific biometric type
            // Heuristics based on device characteristics
            return when {
                Build.VERSION.SDK_INT >= Build.VERSION_CODES.R -> {
                    // Android 11+ - use package manager to check
                    val hasFace = packageManager.hasSystemFeature("android.hardware.biometrics.face")
                    val hasFingerprint = packageManager.hasSystemFeature("android.hardware.fingerprint")
                    when {
                        hasFace -> "faceID"
                        hasFingerprint -> "touchID"
                        else -> "biometric"
                    }
                }
                else -> "touchID" // Default to fingerprint on older devices
            }
        }
        return "none"
    }

    private fun getEncryptedSharedPreferences(): SharedPreferences {
        val masterKey = MasterKey.Builder(this)
            .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
            .build()

        return EncryptedSharedPreferences.create(
            this,
            "solosoul_keychain",
            masterKey,
            EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
            EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
        )
    }

    private fun saveToKeychain(key: String, value: String, result: MethodChannel.Result) {
        try {
            val prefs = getEncryptedSharedPreferences()
            prefs.edit().putString(key, value).apply()
            result.success(mapOf("success" to true))
        } catch (e: Exception) {
            result.success(mapOf("success" to false, "error" to e.message))
        }
    }

    private fun readFromKeychain(key: String, result: MethodChannel.Result) {
        try {
            val prefs = getEncryptedSharedPreferences()
            val value = prefs.getString(key, null)
            if (value != null) {
                result.success(mapOf("success" to true, "value" to value))
            } else {
                result.success(mapOf("success" to false, "error" to "Key not found"))
            }
        } catch (e: Exception) {
            result.success(mapOf("success" to false, "error" to e.message))
        }
    }

    private fun deleteFromKeychain(key: String, result: MethodChannel.Result) {
        try {
            val prefs = getEncryptedSharedPreferences()
            prefs.edit().remove(key).apply()
            result.success(mapOf("success" to true))
        } catch (e: Exception) {
            result.success(mapOf("success" to false, "error" to e.message))
        }
    }
}
