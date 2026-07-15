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
      // 以原始文件名为基准，尝试重命名为原始名或 a(1).pdf、a(2).pdf，确保扩展名在前。
      uri = renameToUniqueDisplayName(uri, srcFile.name)

      invoke.resolve(JSObject())
    } catch (e: IOException) {
      invoke.reject("复制文件失败: ${e.message}")
    } catch (e: Exception) {
      invoke.reject("导出附件失败: ${e.message}")
    }
  }

  /**
   * 以原始文件名为基准，把目标 URI 重命名为唯一且扩展名正确的名字。
   * 例如系统已生成 a.pdf(1)，会优先修正为 a(1).pdf；若该名也存在则递增。
   */
  private fun renameToUniqueDisplayName(uri: Uri, originalName: String): Uri {
    val currentName = queryDisplayName(uri)
    android.util.Log.d("SoloSoul", "renameToUniqueDisplayName: original=$originalName, current=$currentName, uri=$uri")

    // 如果当前文件名看起来已经是 "原始名"，直接返回
    if (currentName == originalName) {
      android.util.Log.d("SoloSoul", "currentName equals originalName, skip")
      return uri
    }

    // 先尝试修正当前可能错误放置的序号：a.pdf(1) -> a(1).pdf
    if (currentName != null) {
      val corrected = sanitizeDuplicateSuffix(currentName)
      android.util.Log.d("SoloSoul", "corrected=$corrected")
      if (corrected != currentName) {
        tryRenameDocument(uri, corrected)?.let {
          android.util.Log.d("SoloSoul", "renamed to corrected: $it")
          return it
        }
      }
    }

    // 否则以 originalName 为基准生成候选名：originalName, a(1).pdf, a(2).pdf...
    val ext = originalName.substringAfterLast(".", "")
    val stem = if (ext.isEmpty()) originalName else originalName.substringBeforeLast(".")
    val candidates = mutableListOf(originalName)
    for (i in 1..1000) {
      candidates.add(if (ext.isEmpty()) "${stem}($i)" else "${stem}($i).${ext}")
    }

    for (name in candidates) {
      tryRenameDocument(uri, name)?.let {
        android.util.Log.d("SoloSoul", "renamed to: $name, newUri=$it")
        return it
      }
    }
    android.util.Log.d("SoloSoul", "all rename attempts failed, keep uri=$uri")
    return uri
  }

  private fun tryRenameDocument(uri: Uri, newName: String): Uri? {
    return try {
      // 系统保存对话框返回的通常是 document URI，直接使用即可
      val renamed = DocumentsContract.renameDocument(activity.contentResolver, uri, newName)
      if (renamed != null) {
        // 对 Download Provider 等场景，renameDocument 可能只更新元数据，
        // 再用 ContentResolver.update 更新 DISPLAY_NAME 作为兜底。
        updateDisplayName(renamed, newName)
        val verified = queryDisplayName(renamed)
        android.util.Log.d("SoloSoul", "after rename: newName=$newName, verified=$verified")
      }
      renamed
    } catch (e: Exception) {
      android.util.Log.d("SoloSoul", "renameDocument failed for $newName: ${e.message}")
      null
    }
  }

  private fun updateDisplayName(uri: Uri, newName: String): Boolean {
    return try {
      val values = android.content.ContentValues().apply {
        put(OpenableColumns.DISPLAY_NAME, newName)
      }
      activity.contentResolver.update(uri, values, null, null) > 0
    } catch (e: Exception) {
      android.util.Log.d("SoloSoul", "updateDisplayName failed: ${e.message}")
      false
    }
  }

  private fun sanitizeDuplicateSuffix(name: String): String {
    // 找到最后一个 "(num)" 模式
    var lastOpen = -1
    var lastClose = -1
    var i = 0
    while (i < name.length) {
      if (name[i] == '(') {
        var j = i + 1
        while (j < name.length && name[j].isDigit()) {
          j++
        }
        if (j < name.length && name[j] == ')') {
          lastOpen = i
          lastClose = j
          i = j + 1
          continue
        }
      }
      i++
    }
    if (lastOpen < 0 || lastClose < 0) return name

    val num = name.substring(lastOpen + 1, lastClose)
    val before = name.substring(0, lastOpen).trimEnd()
    val after = name.substring(lastClose + 1).trimStart()

    return if (after.isEmpty()) {
      // 如 a.pdf(1)：把 before 末尾的扩展名移到序号之后
      val dot = before.lastIndexOf('.')
      if (dot > 0) {
        val ext = before.substring(dot)
        if (ext.length > 1 && ext.substring(1).all { it.isLetterOrDigit() }) {
          val base = before.substring(0, dot).trimEnd()
          "${base}(${num})${ext}"
        } else {
          "${before}(${num})"
        }
      } else {
        "${before}(${num})"
      }
    } else {
      // 如 a(1).pdf 或 a (1).pdf：after 就是扩展名
      "${before}(${num})${after}"
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
