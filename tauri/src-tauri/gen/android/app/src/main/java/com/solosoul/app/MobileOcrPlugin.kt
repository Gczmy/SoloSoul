package com.solosoul.app

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.provider.MediaStore
import androidx.activity.result.ActivityResult
import androidx.core.content.FileProvider
import androidx.core.net.toUri
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import com.google.mlkit.vision.common.InputImage
import com.google.mlkit.vision.text.TextRecognition
import com.google.mlkit.vision.text.TextRecognizer
import com.google.mlkit.vision.text.chinese.ChineseTextRecognizerOptions
import java.io.File

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
@TauriPlugin
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
                        val boxes = mutableListOf<JSObject>()
                        visionText.textBlocks.forEach { block ->
                            val rect = block.boundingBox
                            if (rect == null) return@forEach

                            val points = arrayOf(
                                arrayOf(rect.left.toFloat(), rect.top.toFloat()),
                                arrayOf(rect.right.toFloat(), rect.top.toFloat()),
                                arrayOf(rect.right.toFloat(), rect.bottom.toFloat()),
                                arrayOf(rect.left.toFloat(), rect.bottom.toFloat())
                            )

                            val box = JSObject()
                            box.put("text", block.text)
                            // ML Kit 不暴露单字置信度，使用 1.0 占位。
                            box.put("confidence", 1.0)
                            box.put("points", points)
                            boxes.add(box)
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
            val captureFile = File(activity.cacheDir, "ocr_capture_${System.currentTimeMillis()}.jpg")
            pendingCapturePath = captureFile.absolutePath

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
                invoke.reject("NO_CAMERA_HANDLER")
                return
            }

            startActivityForResult(invoke, intent, "takePhotoResult")
        } catch (e: Exception) {
            android.util.Log.e("SoloSoul", "takePhoto failed: ${e.message}", e)
            pendingCapturePath = null
            invoke.reject("TAKE_PHOTO_FAILED: ${e.message}")
        }
    }

    @ActivityCallback
    fun takePhotoResult(invoke: Invoke, result: ActivityResult) {
        val response = JSObject()
        if (result.resultCode == Activity.RESULT_OK && pendingCapturePath != null) {
            val path = pendingCapturePath!!
            val file = File(path)
            if (file.exists() && file.length() > 0) {
                response.put("path", "file://$path")
            }
        }
        // 取消或文件未生成 → response 无 path 字段，前端视为用户取消
        pendingCapturePath = null
        invoke.resolve(response)
    }

    /**
     * 插件销毁时释放 ML Kit 原生资源。
     */
    override fun onDestroy() {
        super.onDestroy()
        recognizer?.let {
            it.close()
        }
    }
}
