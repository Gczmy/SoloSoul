package com.solosoul.app

import android.app.Activity
import android.net.Uri
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.io.File
import java.io.FileInputStream
import java.io.FileOutputStream
import java.io.IOException

@InvokeArg
class ImportContentUriArgs {
  lateinit var destPath: String
  lateinit var contentUri: String
  lateinit var fileName: String
  lateinit var attachmentId: String
  lateinit var objectId: String
}

@InvokeArg
class ExportContentUriArgs {
  lateinit var srcPath: String
  lateinit var destUri: String
}

/**
 * Android content:// URI 附件导入/导出插件。
 *
 * Tauri 的 `plugin-dialog` 在 Android 上返回/接受 `content://` URI，而 `plugin-fs`
 * 无法直接复制这种 URI。该插件通过 `ContentResolver` 在 Vault 目录与 content URI
 * 之间流式复制文件，避免前端先把大文件读进内存。
 */
@TauriPlugin
class AttachmentImportPlugin(private val activity: Activity): Plugin(activity) {
  @Command
  fun importContentUri(invoke: Invoke) {
    try {
      val args = invoke.parseArgs(ImportContentUriArgs::class.java)
      val uri = Uri.parse(args.contentUri)

      // 确保目标目录存在
      val destFile = File(args.destPath)
      destFile.parentFile?.mkdirs()

      activity.contentResolver.openInputStream(uri)?.use { input ->
        FileOutputStream(destFile).use { output ->
          input.copyTo(output)
        }
      } ?: run {
        invoke.reject("无法打开文件: ${args.contentUri}")
        return
      }

      val result = JSObject()
      result.put("vaultPath", destFile.absolutePath)
      result.put("sizeBytes", destFile.length())
      invoke.resolve(result)
    } catch (e: IOException) {
      invoke.reject("复制文件失败: ${e.message}")
    } catch (e: Exception) {
      invoke.reject("导入附件失败: ${e.message}")
    }
  }

  @Command
  fun exportContentUri(invoke: Invoke) {
    try {
      val args = invoke.parseArgs(ExportContentUriArgs::class.java)
      val srcFile = File(args.srcPath)
      if (!srcFile.exists()) {
        invoke.reject("源文件不存在: ${args.srcPath}")
        return
      }

      val uri = Uri.parse(args.destUri)
      activity.contentResolver.openOutputStream(uri)?.use { output ->
        FileInputStream(srcFile).use { input ->
          input.copyTo(output)
        }
      } ?: run {
        invoke.reject("无法打开目标 URI: ${args.destUri}")
        return
      }

      invoke.resolve(JSObject())
    } catch (e: IOException) {
      invoke.reject("复制文件失败: ${e.message}")
    } catch (e: Exception) {
      invoke.reject("导出附件失败: ${e.message}")
    }
  }
}
