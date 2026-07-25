package com.solosoul.app

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.provider.DocumentsContract
import android.provider.OpenableColumns
import androidx.core.content.FileProvider
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import androidx.activity.result.ActivityResult
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
class CopyContentUriArgs {
  lateinit var contentUri: String
  lateinit var destPath: String
}

@InvokeArg
class ExportToTreeUriArgs {
  lateinit var srcPath: String
  lateinit var treeUri: String
  lateinit var fileName: String
  lateinit var mimeType: String
}

@InvokeArg
class OpenFileArgs {
  lateinit var path: String
  lateinit var mimeType: String
}

@InvokeArg
class SyncDirArgs {
  lateinit var localDir: String
  lateinit var treeUri: String
}

@InvokeArg
class ScheduleFallbackSyncArgs {
  lateinit var localDir: String
  lateinit var treeUri: String
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

  /**
   * 通用 content:// URI 复制：把 content URI 流式复制到调用方指定的本地路径。
   * 用于导入包中转等不需要写入 Vault 附件目录的场景。
   */
  @Command
  fun copyContentUriToFile(invoke: Invoke) {
    try {
      val args = invoke.parseArgs(CopyContentUriArgs::class.java)
      val uri = Uri.parse(args.contentUri)
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
      result.put("localPath", destFile.absolutePath)
      result.put("sizeBytes", destFile.length())
      invoke.resolve(result)
    } catch (e: IOException) {
      invoke.reject("复制文件失败: ${e.message}")
    } catch (e: Exception) {
      invoke.reject("复制 content URI 失败: ${e.message}")
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
   * 在 parentUri 下创建临时 .tmp 文件，写入 sourceFile 内容。
   * 写入成功后返回临时文档 URI；写入失败时自动清理 .tmp 并返回 null。
   */
  private fun createTempDocumentAndWrite(
    parentUri: Uri,
    tempName: String,
    mimeType: String,
    sourceFile: File
  ): Uri? {
    val tempDoc = DocumentsContract.createDocument(
      activity.contentResolver, parentUri, mimeType, tempName
    ) ?: return null

    try {
      activity.contentResolver.openOutputStream(tempDoc)?.use { output ->
        FileInputStream(sourceFile).use { input ->
          input.copyTo(output)
        }
      } ?: run {
        // 无法打开输出流，清理临时文件
        try {
          DocumentsContract.deleteDocument(activity.contentResolver, tempDoc)
        } catch (_: Exception) {}
        return null
      }
    } catch (e: Exception) {
      // 写入失败，清理临时文件，不碰原文件
      try {
        DocumentsContract.deleteDocument(activity.contentResolver, tempDoc)
      } catch (_: Exception) {}
      return null
    }
    return tempDoc
  }

  /**
   * 以原始文件名为基准，把目标 URI 重命名为唯一且扩展名正确的名字。
   * 例如系统已生成 a.pdf(1)，会优先修正为 a(1).pdf；若该名也存在则递增。
   */
  private fun renameToUniqueDisplayName(uri: Uri, originalName: String): Uri {
    val currentName = queryDisplayName(uri)

    // 如果当前文件名看起来已经是 "原始名"，直接返回
    if (currentName == originalName) return uri

    // 先尝试修正当前可能错误放置的序号：a.pdf(1) -> a(1).pdf
    if (currentName != null) {
      val corrected = sanitizeDuplicateSuffix(currentName)
      if (corrected != currentName) {
        tryRenameDocument(uri, corrected)?.let { return it }
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
      tryRenameDocument(uri, name)?.let { return it }
    }
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
      }
      renamed
    } catch (e: Exception) {
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
  fun exportToTreeUri(invoke: Invoke) {
    try {
      val args = invoke.parseArgs(ExportToTreeUriArgs::class.java)
      val srcFile = File(args.srcPath)
      if (!srcFile.exists()) {
        invoke.reject("源文件不存在: ${args.srcPath}")
        return
      }

      val treeUri = Uri.parse(args.treeUri)
      val parent = DocumentsContract.buildDocumentUriUsingTree(treeUri, DocumentsContract.getTreeDocumentId(treeUri))
        ?: run {
        invoke.reject("无法从 tree URI 解析目标目录: ${args.treeUri}")
        return
      }

      // 清理文件名，防止路径遍历
      val safeName = File(args.fileName).name.takeIf { it.isNotBlank() && it != "/" }
        ?: args.fileName

      val mimeType = args.mimeType.ifBlank { "application/octet-stream" }

      // 原子写入：先创建 .tmp 文件写入，成功后重命名为最终文件名
      // 避免写入中途失败时在目标目录留下损坏的残缺文件
      val tempName = "${safeName}.tmp"
      val tempDoc = createTempDocumentAndWrite(parent, tempName, mimeType, srcFile)
      if (tempDoc == null) {
        invoke.reject("无法写入临时文件 $tempName")
        return
      }

      // 使用系统 API 创建唯一文件名（仅在 .tmp 重命名与目标冲突时使用）
      val uniqueName = createUniqueFileName(parent, safeName)
      val renamed = DocumentsContract.renameDocument(
        activity.contentResolver, tempDoc, uniqueName
      )
      if (renamed == null) {
        invoke.reject("无法重命名临时文件为 $uniqueName")
        return
      }

      invoke.resolve(JSObject())
    } catch (e: IOException) {
      invoke.reject("复制文件失败: ${e.message}")
    } catch (e: Exception) {
      invoke.reject("导出到 tree URI 失败: ${e.message}")
    }
  }

  /**
   * 在指定 document URI 所在目录生成一个唯一的文件名。
   * 优先使用原始文件名；若已存在则追加 "(1)","(2)" 等序号。
   */
  private fun createUniqueFileName(parentUri: Uri, originalName: String): String {
    if (!fileExistsInTree(parentUri, originalName)) {
      return originalName
    }
    val ext = originalName.substringAfterLast(".", "")
    val stem = if (ext.isEmpty()) originalName else originalName.substringBeforeLast(".")
    var n = 1
    while (true) {
      val candidate = if (ext.isEmpty()) "${stem}($n)" else "${stem}($n).${ext}"
      if (!fileExistsInTree(parentUri, candidate)) {
        return candidate
      }
      n++
      if (n > 1000) {
        // 兜底：使用时间戳避免无限循环
        val timestamp = System.currentTimeMillis()
        return if (ext.isEmpty()) "${stem}_${timestamp}" else "${stem}_${timestamp}.${ext}"
      }
    }
  }

  private fun fileExistsInTree(parentUri: Uri, name: String): Boolean {
    return try {
      val childrenUri = DocumentsContract.buildChildDocumentsUriUsingTree(parentUri, DocumentsContract.getTreeDocumentId(parentUri))
      activity.contentResolver.query(childrenUri, arrayOf(DocumentsContract.Document.COLUMN_DISPLAY_NAME), null, null, null)?.use { cursor ->
        while (cursor.moveToNext()) {
          val displayName = cursor.getString(0)
          if (displayName == name) return true
        }
      }
      false
    } catch (e: Exception) {
      false
    }
  }

  @Command
  fun pickTreeUri(invoke: Invoke) {
    try {
      val intent = Intent(Intent.ACTION_OPEN_DOCUMENT_TREE).apply {
        addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION)
      }
      // 预检：部分 ROM 缺少 SAF 目录选择器（DocumentsUI），直接报明确错误
      if (intent.resolveActivity(activity.packageManager) == null) {
        android.util.Log.e("SoloSoul", "pickTreeUri: no activity handles ACTION_OPEN_DOCUMENT_TREE")
        invoke.reject("NO_TREE_PICKER_HANDLER")
        return
      }
      startActivityForResult(invoke, intent, "treePickerResult")
    } catch (e: Exception) {
      android.util.Log.e("SoloSoul", "pickTreeUri failed: ${e.message}", e)
      invoke.reject("PICK_TREE_FAILED: ${e.message}")
    }
  }

  /**
   * 选择 Vault 数据目录。与 pickTreeUri 不同，此处总是持久化读写授权，
   * 并把 tree URI 作为应用级配置返回给 Rust 端。
   */
  @Command
  fun pickVaultDir(invoke: Invoke) {
    try {
      val intent = Intent(Intent.ACTION_OPEN_DOCUMENT_TREE).apply {
        addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION)
      }
      if (intent.resolveActivity(activity.packageManager) == null) {
        android.util.Log.e("SoloSoul", "pickVaultDir: no activity handles ACTION_OPEN_DOCUMENT_TREE")
        invoke.reject("NO_TREE_PICKER_HANDLER")
        return
      }
      startActivityForResult(invoke, intent, "vaultDirResult")
    } catch (e: Exception) {
      android.util.Log.e("SoloSoul", "pickVaultDir failed: ${e.message}", e)
      invoke.reject("PICK_VAULT_DIR_FAILED: ${e.message}")
    }
  }

  @ActivityCallback
  fun treePickerResult(invoke: Invoke, result: ActivityResult) {
    val response = JSObject()
    if (result.resultCode == Activity.RESULT_OK && result.data?.data != null) {
      val uri = result.data?.data!!
      try {
        activity.contentResolver.takePersistableUriPermission(
          uri,
          Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION
        )
      } catch (e: SecurityException) {
        // Not all providers support persistable permissions; non-persistable is fine for batch download
        android.util.Log.w("SoloSoul", "Could not take persistable URI permission: ${e.message}")
      }
      response.put("uri", uri.toString())
    }
    invoke.resolve(response)
  }

  /**
   * 同步本地目录到 SAF tree URI（覆盖远端）。
   * 递归遍历 localDir 下所有文件，在 treeUri 下创建对应目录结构与文件。
   * 在后台线程执行，同步期间通过 trigger 发送进度事件。
   */
  @Command
  fun syncDirToRemote(invoke: Invoke) {
    val args: SyncDirArgs = try {
      invoke.parseArgs(SyncDirArgs::class.java)
    } catch (e: Exception) {
      invoke.reject("参数解析失败: ${e.message}")
      return
    }

    Thread {
      try {
        val localDir = File(args.localDir)
        var fileCount = 0
        val result = SafSyncHelper.syncLocalDirToTree(activity, localDir, args.treeUri) { fileName ->
          fileCount++
          val payload = JSObject().apply {
            put("phase", "syncing")
            put("fileName", fileName)
            put("fileCount", fileCount)
          }
          activity.runOnUiThread { this@AttachmentImportPlugin.trigger("sync-progress", payload) }
        }
        if (result.isSuccess) {
          activity.runOnUiThread { invoke.resolve(JSObject()) }
        } else {
          val ex = result.exceptionOrNull()
          android.util.Log.e("SoloSoul", "syncDirToRemote failed: ${ex?.message}", ex)
          activity.runOnUiThread { invoke.reject("同步到 SAF 失败: ${ex?.message}") }
        }
      } catch (e: Exception) {
        android.util.Log.e("SoloSoul", "syncDirToRemote failed: ${e.message}", e)
        activity.runOnUiThread { invoke.reject("同步到 SAF 失败: ${e.message}") }
      }
    }.start()
  }

  /**
   * 从 SAF tree URI 同步到本地目录（覆盖本地）。
   * 递归遍历 treeUri 下所有文件，在 localDir 下创建对应目录结构与文件。
   * 在后台线程执行，同步期间通过 trigger 发送进度事件。
   */
  @Command
  fun syncDirFromRemote(invoke: Invoke) {
    val args: SyncDirArgs = try {
      invoke.parseArgs(SyncDirArgs::class.java)
    } catch (e: Exception) {
      invoke.reject("参数解析失败: ${e.message}")
      return
    }

    Thread {
      try {
        val localDir = File(args.localDir)
        var fileCount = 0
        val result = SafSyncHelper.syncTreeToLocalDir(activity, localDir, args.treeUri) { fileName ->
          fileCount++
          val payload = JSObject().apply {
            put("phase", "syncing")
            put("fileName", fileName)
            put("fileCount", fileCount)
          }
          activity.runOnUiThread { this@AttachmentImportPlugin.trigger("sync-progress", payload) }
        }
        if (result.isSuccess) {
          activity.runOnUiThread { invoke.resolve(JSObject()) }
        } else {
          val ex = result.exceptionOrNull()
          android.util.Log.e("SoloSoul", "syncDirFromRemote failed: ${ex?.message}", ex)
          activity.runOnUiThread { invoke.reject("从 SAF 同步失败: ${ex?.message}") }
        }
      } catch (e: Exception) {
        android.util.Log.e("SoloSoul", "syncDirFromRemote failed: ${e.message}", e)
        activity.runOnUiThread { invoke.reject("从 SAF 同步失败: ${e.message}") }
      }
    }.start()
  }

  /**
   * 调度 WorkManager 周期性后台同步兜底任务。
   * 在 SAF 模式启用时调用，确保应用被系统回收后仍能定期同步到 SAF。
   */
  @Command
  fun scheduleFallbackSync(invoke: Invoke) {
    try {
      val args = invoke.parseArgs(ScheduleFallbackSyncArgs::class.java)
      SafFallbackWorker.schedule(activity, args.localDir, args.treeUri)
      invoke.resolve(JSObject())
    } catch (e: Exception) {
      android.util.Log.e("SoloSoul", "scheduleFallbackSync failed: ${e.message}", e)
      invoke.reject("调度后台同步失败: ${e.message}")
    }
  }

  /**
   * 取消 WorkManager 周期性后台同步兜底任务。
   */
  @Command
  fun cancelFallbackSync(invoke: Invoke) {
    try {
      SafFallbackWorker.cancel(activity)
      invoke.resolve(JSObject())
    } catch (e: Exception) {
      android.util.Log.e("SoloSoul", "cancelFallbackSync failed: ${e.message}", e)
      invoke.reject("取消后台同步失败: ${e.message}")
    }
  }

  /**
   * 把本地目录内容同步到 SAF tree 下。
   * 使用原子写入模式：先写入 .tmp 临时文件，成功后再删除旧文件并重命名为最终文件名。
   * 写入失败时自动清理 .tmp，不碰原文件，避免同步中途失败导致数据丢失。
   *
   * 实现采用 BFS 迭代，每个目录只查询一次子文档列表，避免递归深度过大和
   * 在同步过程中长期持有 Cursor。
   *
   * 注意：本地镜像已由 Rust 端 migrate_vault_data 过滤应用级目录，此处顶层过滤
   * 仅为防御性措施，防止旧版本 SAF 目录中残留应用级条目被回写。
   */
  // 应用级目录/文件名称由 build.rs 从 app_level_names.json 自动生成到
  // AppLevelNames.kt，避免 Rust/Kotlin 手动同步。


  /**
   * 检查 SAF tree URI 是否仍然可访问（授权未被撤销）。
   * 通过尝试查询 tree URI 的子文档来验证。
   * 返回 { accessible: boolean }。
   */
  @InvokeArg
  class CheckAccessArgs {
    lateinit var treeUri: String
  }

  @Command
  fun checkVaultDirAccess(invoke: Invoke) {
    try {
      val args = invoke.parseArgs(CheckAccessArgs::class.java)
      val treeUriStr = args.treeUri
      if (treeUriStr.isNullOrBlank()) {
        invoke.resolve(JSObject().apply { put("accessible", false) })
        return
      }
      val treeUri = Uri.parse(treeUriStr)
      val treeDocId = DocumentsContract.getTreeDocumentId(treeUri)
      val docUri = DocumentsContract.buildDocumentUriUsingTree(treeUri, treeDocId)
      if (docUri == null) {
        invoke.resolve(JSObject().apply { put("accessible", false) })
        return
      }
      // 尝试查询 tree URI 的子文档列表，成功即表示可访问
      val childrenUri = DocumentsContract.buildChildDocumentsUriUsingTree(treeUri, treeDocId)
      activity.contentResolver.query(childrenUri, arrayOf(DocumentsContract.Document.COLUMN_DOCUMENT_ID), null, null, null)?.use { cursor ->
        // 至少能打开 cursor，说明可访问
        cursor.moveToFirst()
        invoke.resolve(JSObject().apply { put("accessible", true) })
      } ?: run {
        invoke.resolve(JSObject().apply { put("accessible", false) })
      }
    } catch (e: SecurityException) {
      // 最常见的授权撤销场景
      android.util.Log.w("SoloSoul", "checkVaultDirAccess: SAF access revoked: ${e.message}")
      invoke.resolve(JSObject().apply { put("accessible", false) })
    } catch (e: Exception) {
      android.util.Log.e("SoloSoul", "checkVaultDirAccess failed: ${e.message}", e)
      invoke.resolve(JSObject().apply { put("accessible", false) })
    }
  }

  @ActivityCallback
  fun vaultDirResult(invoke: Invoke, result: ActivityResult) {
    val response = JSObject()
    if (result.resultCode == Activity.RESULT_OK && result.data?.data != null) {
      val uri = result.data?.data!!
      try {
        activity.contentResolver.takePersistableUriPermission(
          uri,
          Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION
        )
        response.put("uri", uri.toString())
      } catch (e: SecurityException) {
        android.util.Log.e("SoloSoul", "vaultDirResult: failed to take persistable URI permission: ${e.message}")
        invoke.reject("VAULT_DIR_PERMISSION_DENIED")
        return
      }
    }
    invoke.resolve(response)
  }

  @Command
  fun openFile(invoke: Invoke) {
    try {
      val args = invoke.parseArgs(OpenFileArgs::class.java)
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

      val authority = "${activity.packageName}.fileprovider"
      val uri = FileProvider.getUriForFile(activity, authority, tempFile)
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
