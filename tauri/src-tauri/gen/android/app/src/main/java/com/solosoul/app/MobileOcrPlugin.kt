package com.solosoul.app

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.MediaStore
import androidx.activity.result.ActivityResult
import androidx.core.content.FileProvider
import androidx.core.net.toUri
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.Permission
import app.tauri.annotation.PermissionCallback
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import app.tauri.PermissionState
import com.google.mlkit.vision.common.InputImage
import com.google.mlkit.vision.text.TextRecognition
import org.json.JSONArray
import com.google.mlkit.vision.text.TextRecognizer
import com.google.mlkit.vision.text.chinese.ChineseTextRecognizerOptions
import java.io.File
import java.io.FileOutputStream

@InvokeArg
class ScanImageArgs {
    lateinit var filePath: String
}

/**
 * 移动端 OCR 插件（Android ML Kit Text Recognition v2）。
 *
 * 通过 ML Kit 对图片进行文字识别，支持中文与拉丁文。
 * 识别结果映射为与桌面端 PP-OCRv6 一致的字段结构返回给 Rust。
 */
@TauriPlugin(
    permissions = [
        Permission(
            strings = [android.Manifest.permission.CAMERA],
            alias = "camera",
        ),
    ],
)
class MobileOcrPlugin(private val activity: Activity): Plugin(activity) {
    /**
     * 复用 TextRecognizer 实例，避免每次扫描重复初始化模型。
     * 使用可空类型 + 手动延迟加载，首次扫描时才创建。
     */
    private var recognizer: TextRecognizer? = null

    /**
     * 保存待写入的拍照临时文件路径，在 Activity 回调中取回。
     */
    private var pendingCapturePath: String? = null

    /**
     * 用于在进程被系统回收并恢复后仍能取回拍照路径。
     */
    private val prefs by lazy {
        activity.getSharedPreferences("MobileOcrPlugin", Context.MODE_PRIVATE)
    }

    companion object {
        private const val PREF_PENDING_CAPTURE_PATH = "pending_capture_path"
    }

    /**
     * 将 pendingCapturePath 持久化，以应对相机 Activity 期间进程被回收的场景。
     */
    private fun persistCapturePath(path: String?) {
        prefs.edit().apply {
            if (path == null) {
                remove(PREF_PENDING_CAPTURE_PATH)
            } else {
                putString(PREF_PENDING_CAPTURE_PATH, path)
            }
            apply()
        }
    }

    /**
     * 从 SharedPreferences 恢复拍照路径（进程恢复场景）。
     */
    private fun restoreCapturePath(): String? {
        return prefs.getString(PREF_PENDING_CAPTURE_PATH, null)
    }

    /**
     * 清理 cacheDir 中残留的拍照临时文件（ocr_capture_*.jpg）。
     * 跳过当前正在使用的 pendingCapturePath 文件。
     */
    private fun cleanupOldCaptureFiles() {
        try {
            val files = activity.cacheDir.listFiles { file ->
                file.name.startsWith("ocr_capture_") && file.name.endsWith(".jpg") &&
                    file.absolutePath != pendingCapturePath
            }
            files?.forEach { it.delete() }
        } catch (e: Exception) {
            android.util.Log.w("SoloSoul", "cleanupOldCaptureFiles: ${e.message}")
        }
    }

    private fun getRecognizer(): TextRecognizer {
        if (recognizer == null) {
            recognizer = TextRecognition.getClient(ChineseTextRecognizerOptions.Builder().build())
        }
        return recognizer!!
    }

    @Command
    fun scanImage(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(ScanImageArgs::class.java)
            val uri = try {
                Uri.parse(args.filePath)
            } catch (e: Exception) {
                invoke.reject("无效的文件路径或 URI: ${args.filePath}")
                return
            }

            val image = InputImage.fromFilePath(activity, uri)

            getRecognizer().process(image)
                .addOnSuccessListener { visionText ->
                    try {
                        val boxes = JSONArray()
                        visionText.textBlocks.forEach { block ->
                            val rect = block.boundingBox
                            if (rect == null) return@forEach

                            val points = JSONArray()
                            listOf(
                                listOf(rect.left.toFloat(), rect.top.toFloat()),
                                listOf(rect.right.toFloat(), rect.top.toFloat()),
                                listOf(rect.right.toFloat(), rect.bottom.toFloat()),
                                listOf(rect.left.toFloat(), rect.bottom.toFloat()),
                            ).forEach { pt ->
                                points.put(JSONArray().apply { pt.forEach(::put) })
                            }

                            val box = JSObject()
                            box.put("text", block.text)
                            // ML Kit 不暴露单字置信度，使用 1.0 占位。
                            box.put("confidence", 1.0)
                            box.put("points", points)
                            boxes.put(box)
                        }

                        val result = JSObject()
                        result.put("text", visionText.text)
                        result.put("confidence", 1.0)
                        result.put("boxes", boxes)
                        invoke.resolve(result)
                    } catch (e: Exception) {
                        invoke.reject("解析识别结果失败: ${e.message}")
                    }
                }
                .addOnFailureListener { e ->
                    invoke.reject("文字识别失败: ${e.message}")
                }
        } catch (e: Exception) {
            invoke.reject("OCR 调用失败: ${e.message}")
        }
    }

    /**
     * 启动系统相机拍照，照片写入 cacheDir 临时文件后返回 file:// URI。
     * 取消拍照时返回空对象（无 path 字段），前端静默处理。
     */
    @Command
    fun takePhoto(invoke: Invoke) {
        try {
            when (getPermissionState("camera")) {
                PermissionState.GRANTED -> launchCamera(invoke)
                else -> requestPermissionForAlias("camera", invoke, "cameraPermissionResult")
            }
        } catch (e: Exception) {
            android.util.Log.e("SoloSoul", "takePhoto failed: ${e.message}", e)
            pendingCapturePath = null
            invoke.reject("TAKE_PHOTO_FAILED: ${e.message}")
        }
    }

    /**
     * 实际的相机启动逻辑：清理旧文件 → 创建临时文件 → 预检 → 启动相机。
     */
    private fun launchCamera(invoke: Invoke) {
        try {
            // 清理上一次拍照残留的临时文件
            cleanupOldCaptureFiles()

            val captureFile = File(activity.cacheDir, "ocr_capture_${System.currentTimeMillis()}.jpg")
            pendingCapturePath = captureFile.absolutePath
            persistCapturePath(pendingCapturePath)

            val authority = "${activity.packageName}.fileprovider"
            val photoUri = FileProvider.getUriForFile(activity, authority, captureFile)

            val intent = Intent(MediaStore.ACTION_IMAGE_CAPTURE).apply {
                putExtra(MediaStore.EXTRA_OUTPUT, photoUri)
                addFlags(Intent.FLAG_GRANT_WRITE_URI_PERMISSION or Intent.FLAG_GRANT_READ_URI_PERMISSION)
            }

            // 预检：无相机应用处理该 intent 时直接报错
            if (intent.resolveActivity(activity.packageManager) == null) {
                android.util.Log.e("SoloSoul", "takePhoto: no activity handles ACTION_IMAGE_CAPTURE")
                pendingCapturePath = null
                persistCapturePath(null)
                invoke.reject("NO_CAMERA_HANDLER")
                return
            }

            startActivityForResult(invoke, intent, "takePhotoResult")
        } catch (e: Exception) {
            android.util.Log.e("SoloSoul", "launchCamera failed: ${e.message}", e)
            pendingCapturePath = null
            persistCapturePath(null)
            invoke.reject("TAKE_PHOTO_FAILED: ${e.message}")
        }
    }

    /**
     * 相机权限申请结果回调。
     */
    @PermissionCallback
    fun cameraPermissionResult(invoke: Invoke) {
        when (getPermissionState("camera")) {
            PermissionState.GRANTED -> launchCamera(invoke)
            else -> {
                pendingCapturePath = null
                invoke.reject("CAMERA_PERMISSION_DENIED")
            }
        }
    }

    @ActivityCallback
    fun takePhotoResult(invoke: Invoke, result: ActivityResult) {
        val response = JSObject()
        var path = pendingCapturePath ?: restoreCapturePath()

        if (result.resultCode == Activity.RESULT_OK && path != null) {
            val file = File(path)
            if (file.exists() && file.length() > 0) {
                response.put("path", "file://$path")
            } else if (result.data != null) {
                // 部分 ROM 的相机会把图片 URI 放在 result.data 中，或忽略 EXTRA_OUTPUT。
                // 尝试从 result.data 读取并复制到 cache 文件。
                val uri = result.data!!.data
                if (uri != null) {
                    try {
                        val input = activity.contentResolver.openInputStream(uri)
                        val targetFile = File(path)
                        input?.use { stream ->
                            FileOutputStream(targetFile).use { output ->
                                stream.copyTo(output)
                            }
                        }
                        if (targetFile.exists() && targetFile.length() > 0) {
                            response.put("path", "file://${targetFile.absolutePath}")
                        }
                    } catch (e: Exception) {
                        android.util.Log.w("SoloSoul", "takePhotoResult: failed to copy from intent data: ${e.message}")
                    }
                }
            }
        }
        // 取消或文件未生成 → response 无 path 字段，前端视为用户取消
        pendingCapturePath = null
        persistCapturePath(null)
        invoke.resolve(response)
    }

    /**
     * 插件销毁时释放 ML Kit 原生资源，并清理拍照临时文件。
     */
    override fun onDestroy() {
        super.onDestroy()
        recognizer?.let {
            it.close()
        }
        pendingCapturePath = null
        cleanupOldCaptureFiles()
    }
}
