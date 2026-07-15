package com.solosoul.app

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.provider.DocumentsContract
import android.provider.OpenableColumns
import androidx.core.content.FileProvider
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

@InvokeArg
class OpenFileArgs {
  lateinit var path: String
  lateinit var mimeType: String
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

      // 通过 ContentResolver 查询 content URI 的真实显示名称，
      // 而不是使用 URI 路径中的 document ID。
      val displayName = queryDisplayName(uri) ?: File(args.destPath).name

      // 确保目标目录存在，并用真实文件名替换前端传入的（可能是 document ID 的）文件名
      val destFile = File(File(args.destPath).parentFile, displayName)
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
      result.put("displayName", destFile.name)
      invoke.resolve(result)
    } catch (e: IOException) {
      invoke.reject("复制文件失败: ${e.message}")
    } catch (e: Exception) {
      invoke.reject("导入附件失败: ${e.message}")
    }
  }

  /**
   * 查询 content URI 的 OpenableColumns.DISPLAY_NAME。
   * 某些 Provider 可能不支持该列，返回 null；失败时尝试从 URI path/lastPathSegment 兜底。
   */
  private fun queryDisplayName(uri: Uri): String? {
    // 1) 优先使用 ContentResolver 的 DISPLAY_NAME 列
    val displayName = try {
      activity.contentResolver.query(uri, null, null, null, null)?.use { cursor ->
        if (cursor.moveToFirst()) {
          val idx = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
          if (idx >= 0) cursor.getString(idx) else null
        } else {
          null
        }
      }
    } catch (e: Exception) {
      null
    }
    if (!displayName.isNullOrBlank()) {
      return displayName.trim()
    }

    // 2) 从 URI 的 path/lastPathSegment 兜底解析
    return try {
      val path = uri.path
      if (!path.isNullOrBlank()) {
        File(path).name.takeIf { it.isNotBlank() && it != "/" }
      } else {
        null
      }
    } catch (e: Exception) {
      null
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

      var uri = Uri.parse(args.destUri)
      activity.contentResolver.openOutputStream(uri)?.use { output ->
        FileInputStream(srcFile).use { input ->
          input.copyTo(output)
        }
      } ?: run {
        invoke.reject("无法打开目标 URI: ${args.destUri}")
        return
      }

      // 系统文件选择器若因同名自动追加序号，通常会放在扩展名之后（如 a.pdf(1)）。
      // 这里把它修正到扩展名之前（a(1).pdf），避免文件无法识别。
      uri = sanitizeDuplicateDisplayName(uri)

      invoke.resolve(JSObject())
    } catch (e: IOException) {
      invoke.reject("复制文件失败: ${e.message}")
    } catch (e: Exception) {
      invoke.reject("导出附件失败: ${e.message}")
    }
  }

  /**
   * 修正系统因同名自动追加的序号位置。
   * 例如 "a.pdf(1)" → "a(1).pdf"；无扩展名时 "a(1)" 保持不变。
   */
  private fun sanitizeDuplicateDisplayName(uri: Uri): Uri {
    val name = queryDisplayName(uri) ?: return uri
    val match = Regex("^(.*?)(\\.\\w+)?\\s*\\((\\d+)\\)$").find(name) ?: return uri
    val base = match.groupValues[1]
    val ext = match.groupValues[2]
    val num = match.groupValues[3]
    val newName = if (ext.isNotEmpty()) "${base}(${num})${ext}" else "${base}(${num})"
    return try {
      val docId = DocumentsContract.getDocumentId(uri)
      val docUri = DocumentsContract.buildDocumentUriUsingTree(uri, docId)
      DocumentsContract.renameDocument(activity.contentResolver, docUri, newName) ?: uri
    } catch (e: Exception) {
      uri
    }
  }

  /**
   * 打开本地文件。
   * - PDF 文件使用内置 PdfPreviewActivity 渲染，避免依赖外部阅读器。
   * - 其他类型使用系统默认应用打开；通过 createChooser 绕过 Android 11+
   *   对 resolveActivity 的限制，并捕获 ActivityNotFoundException。
   */
  @Command
  fun openFile(invoke: Invoke) {
    try {
      val args = invoke.parseArgs(OpenFileArgs::class.java)
      android.util.Log.d("SoloSoul", "openFile called: path=${args.path}, mime=${args.mimeType}")
      val srcFile = File(args.path)
      if (!srcFile.exists()) {
        android.util.Log.e("SoloSoul", "openFile: source file does not exist: ${args.path}")
        invoke.reject("文件不存在: ${args.path}")
        return
      }

      // PDF 使用原生预览 Activity
      if (args.mimeType == "application/pdf") {
        val intent = android.content.Intent(activity, PdfPreviewActivity::class.java).apply {
          putExtra(PdfPreviewActivity.EXTRA_PATH, srcFile.absolutePath)
          putExtra(PdfPreviewActivity.EXTRA_TITLE, srcFile.name)
          flags = android.content.Intent.FLAG_ACTIVITY_NEW_TASK
        }
        activity.startActivity(intent)
        invoke.resolve(JSObject())
        return
      }

      // 其他类型：复制到 filesDir/open_temp/ 后通过 FileProvider 暴露给外部应用。
      val tempDir = File(activity.filesDir, "open_temp").apply { mkdirs() }
      val tempFile = File(tempDir, srcFile.name)
      FileInputStream(srcFile).use { input ->
        FileOutputStream(tempFile).use { output ->
          input.copyTo(output)
        }
      }
      android.util.Log.d("SoloSoul", "openFile: copied to ${tempFile.absolutePath}")

      val authority = "${activity.packageName}.fileprovider"
      val uri = FileProvider.getUriForFile(activity, authority, tempFile)
      android.util.Log.d("SoloSoul", "openFile: uri=$uri")
      val intent = android.content.Intent(android.content.Intent.ACTION_VIEW).apply {
        setDataAndType(uri, args.mimeType)
        flags = android.content.Intent.FLAG_GRANT_READ_URI_PERMISSION or android.content.Intent.FLAG_ACTIVITY_NEW_TASK
      }

      // 使用 createChooser，避免 Android 11+ resolveActivity 受 package visibility 限制。
      val chooser = android.content.Intent.createChooser(intent, null)
      try {
        activity.startActivity(chooser)
        invoke.resolve(JSObject())
      } catch (e: android.content.ActivityNotFoundException) {
        android.util.Log.e("SoloSoul", "openFile: no app can handle ${args.mimeType}")
        invoke.reject("没有应用可以打开此文件类型，请安装对应阅读器")
      }
    } catch (e: Exception) {
      android.util.Log.e("SoloSoul", "openFile failed: ${e.message}", e)
      invoke.reject("打开文件失败: ${e.message}")
    }
  }
}
