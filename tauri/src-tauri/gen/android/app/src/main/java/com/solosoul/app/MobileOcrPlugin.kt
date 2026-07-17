package com.solosoul.app

import android.app.Activity
import android.net.Uri
import androidx.core.net.toUri
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import com.google.mlkit.vision.common.InputImage
import com.google.mlkit.vision.text.TextRecognition
import com.google.mlkit.vision.text.chinese.ChineseTextRecognizerOptions

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
     * 使用 lazy 延迟加载，首次扫描时才创建。
     */
    private val recognizer: TextRecognizer by lazy {
        TextRecognition.getClient(ChineseTextRecognizerOptions.Builder().build())
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

            recognizer.process(image)
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
     * 插件销毁时释放 ML Kit 原生资源。
     */
    override fun onDestroy() {
        super.onDestroy()
        if (::recognizer.isInitialized) {
            recognizer.close()
        }
    }
}
