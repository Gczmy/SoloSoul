package com.solosoul.app

import android.content.Context
import android.net.Uri
import android.provider.DocumentsContract
import android.provider.OpenableColumns
import java.io.File
import java.io.FileInputStream
import java.io.FileOutputStream
import java.io.RandomAccessFile
import java.nio.channels.FileLock
import java.util.ArrayDeque

/**
 * SAF 同步辅助类。
 *
 * 将 `AttachmentImportPlugin` 中的同步逻辑抽取出来，使其可以脱离 Tauri Plugin
 * 生命周期，在 [Context] 下运行。供应用内同步与 WorkManager 后台同步共用。
 *
 * 特性：
 * - 所有同步方法接收 [Context]，不依赖 Activity。
 * - 通过本地目录下的 `.saf_sync.lock` 文件锁避免应用内同步与 WorkManager 同步并发执行。
 * - 支持可选的进度回调。
 */
object SafSyncHelper {

    private const val LOCK_FILE_NAME = ".saf_sync.lock"

    /**
     * JVM 级互斥锁。FileLock 只能防止跨进程冲突，同一 JVM 内多线程并发加锁会抛出
     * OverlappingFileLockException；因此先用 synchronized 保证进程内互斥，再用
     * FileLock 保证跨进程互斥。
     */
    private val jvmLock = Any()

    /**
     * 同步本地目录到 SAF tree 远端。
     *
     * @param context 用于获取 ContentResolver。
     * @param localDir 本地 Vault 缓存目录。
     * @param treeUri SAF tree URI 字符串。
     * @param onProgress 可选进度回调，参数为已处理的文件名。
     * @return 同步是否成功。
     */
    fun syncLocalDirToTree(
        context: Context,
        localDir: File,
        treeUri: String,
        onProgress: (String) -> Unit = {}
    ): Result<Unit> {
        if (!localDir.exists()) {
            return Result.success(Unit)
        }
        val parent = DocumentsContract.buildDocumentUriUsingTree(
            Uri.parse(treeUri),
            DocumentsContract.getTreeDocumentId(Uri.parse(treeUri))
        ) ?: return Result.failure(IllegalArgumentException("无法从 tree URI 解析目标目录: $treeUri"))

        return Result.success(withFileLock(localDir) {
            syncLocalDirToTreeInternal(context, localDir, parent, onProgress)
        })
    }

    /**
     * 从 SAF tree 远端同步到本地目录。
     */
    fun syncTreeToLocalDir(
        context: Context,
        localDir: File,
        treeUri: String,
        onProgress: (String) -> Unit = {}
    ): Result<Unit> {
        localDir.mkdirs()
        val parent = DocumentsContract.buildDocumentUriUsingTree(
            Uri.parse(treeUri),
            DocumentsContract.getTreeDocumentId(Uri.parse(treeUri))
        ) ?: return Result.failure(IllegalArgumentException("无法从 tree URI 解析源目录: $treeUri"))

        return Result.success(withFileLock(localDir) {
            syncTreeToLocalDirInternal(context, parent, localDir, onProgress)
        })
    }

    /**
     * 在 localDir 目录下获取文件锁并执行 [block]。
     * 文件锁为进程级，即使应用被系统回收，锁也会自动释放。
     */
    private fun <T> withFileLock(localDir: File, block: () -> T): T {
        // 进程内互斥：防止同一 JVM 内多个线程同时进入同步逻辑。
        synchronized(jvmLock) {
            localDir.mkdirs()
            val lockFile = File(localDir, LOCK_FILE_NAME)
            RandomAccessFile(lockFile, "rw").use { raf ->
                raf.channel.use { channel ->
                    var lock: FileLock? = null
                    try {
                        lock = channel.lock()
                        return block()
                    } finally {
                        try {
                            lock?.release()
                        } catch (_: Exception) {
                        }
                    }
                }
            }
        }
    }

    private data class LocalDirEntry(val localDir: File, val parentUri: Uri, val isRoot: Boolean)

    private data class RemoteDirEntry(val parentUri: Uri, val localDir: File, val isRoot: Boolean)

    private data class ExistingChild(
        val uri: Uri,
        val lastModified: Long,
        val size: Long
    )

    private data class RemoteChild(
        val docUri: Uri,
        val displayName: String,
        val mimeType: String,
        val lastModified: Long,
        val size: Long
    )

    private fun syncLocalDirToTreeInternal(
        context: Context,
        localDir: File,
        parentUri: Uri,
        onProgress: (String) -> Unit
    ) {
        val queue = ArrayDeque<LocalDirEntry>()
        queue.add(LocalDirEntry(localDir, parentUri, true))
        val contentResolver = context.contentResolver

        while (queue.isNotEmpty()) {
            val entry = queue.removeFirst()
            val currentDir = entry.localDir
            val currentParentUri = entry.parentUri
            val files = currentDir.listFiles() ?: continue

            val existingChildren = mutableMapOf<String, ExistingChild>()
            val childrenUri = DocumentsContract.buildChildDocumentsUriUsingTree(
                currentParentUri,
                DocumentsContract.getTreeDocumentId(currentParentUri)
            ) ?: continue
            contentResolver.query(
                childrenUri,
                arrayOf(
                    DocumentsContract.Document.COLUMN_DOCUMENT_ID,
                    DocumentsContract.Document.COLUMN_DISPLAY_NAME,
                    DocumentsContract.Document.COLUMN_LAST_MODIFIED,
                    DocumentsContract.Document.COLUMN_SIZE
                ),
                null, null, null
            )?.use { cursor ->
                while (cursor.moveToNext()) {
                    val docId = cursor.getString(0)
                    val displayName = cursor.getString(1)
                    val mtimeIdx = cursor.getColumnIndex(DocumentsContract.Document.COLUMN_LAST_MODIFIED)
                    val sizeIdx = cursor.getColumnIndex(DocumentsContract.Document.COLUMN_SIZE)
                    val lastModified = if (mtimeIdx >= 0) cursor.getLong(mtimeIdx) else -1L
                    val size = if (sizeIdx >= 0) cursor.getLong(sizeIdx) else -1L
                    if (!docId.isNullOrBlank() && !displayName.isNullOrBlank()) {
                        existingChildren[displayName] = ExistingChild(
                            DocumentsContract.buildDocumentUriUsingTree(currentParentUri, docId),
                            lastModified,
                            size
                        )
                    }
                }
            }

            for (file in files) {
                try {
                    if (entry.isRoot && file.name in AppLevelNames.NAMES) {
                        continue
                    }
                    if (file.isDirectory) {
                        val existingChild = existingChildren[file.name]
                        val dirUri = if (existingChild != null) {
                            existingChild.uri
                        } else {
                            val created = DocumentsContract.createDocument(
                                contentResolver,
                                currentParentUri,
                                DocumentsContract.Document.MIME_TYPE_DIR,
                                file.name
                            )
                            if (created == null) {
                                android.util.Log.w(
                                    "SoloSoul",
                                    "syncLocalDirToTree: failed to create dir '${file.name}' in SAF tree, skipping"
                                )
                                continue
                            }
                            created
                        }
                        queue.add(LocalDirEntry(file, dirUri, false))
                    } else {
                        val existingChild = existingChildren[file.name]
                        if (existingChild != null &&
                            existingChild.lastModified > 0 &&
                            existingChild.size > 0 &&
                            existingChild.lastModified == file.lastModified() &&
                            existingChild.size == file.length()) {
                            continue
                        }

                        try {
                            val mimeType = getMimeType(file.name)
                            val tempName = "${file.name}.tmp"
                            val tempDoc = createTempDocumentAndWrite(
                                context, currentParentUri, tempName, mimeType, file
                            )
                            if (tempDoc == null) {
                                android.util.Log.w("SoloSoul", "写入临时文件失败，跳过: ${file.name}")
                                continue
                            }

                            if (existingChild != null) {
                                try {
                                    DocumentsContract.deleteDocument(contentResolver, existingChild.uri)
                                } catch (e: Exception) {
                                    android.util.Log.w("SoloSoul", "删除旧文件失败: ${e.message}")
                                }
                            }

                            val renamed = DocumentsContract.renameDocument(
                                contentResolver, tempDoc, file.name
                            )
                            if (renamed == null) {
                                android.util.Log.w(
                                    "SoloSoul",
                                    "重命名临时文件失败，尝试清理: $tempName"
                                )
                                try {
                                    DocumentsContract.deleteDocument(contentResolver, tempDoc)
                                } catch (e: Exception) {
                                    android.util.Log.w("SoloSoul", "清理失败临时文件失败: ${e.message}")
                                }
                            } else {
                                onProgress(file.name)
                            }
                        } catch (e: Exception) {
                            val msg = e.message ?: ""
                            if (msg.contains("ENAMETOOLONG")) {
                                android.util.Log.w(
                                    "SoloSoul",
                                    "syncLocalDirToTree: skipping file '${file.name}' (name too long)"
                                )
                            } else {
                                android.util.Log.w(
                                    "SoloSoul",
                                    "syncLocalDirToTree: skipping file '${file.name}' in ${currentDir.path}: ${e.message}"
                                )
                            }
                        }
                    }
                } catch (e: Exception) {
                    val msg = e.message ?: ""
                    android.util.Log.w(
                        "SoloSoul",
                        "syncLocalDirToTree: skipping child '${file.name}' in ${currentDir.path}: $msg"
                    )
                }
            }
        }
    }

    private fun syncTreeToLocalDirInternal(
        context: Context,
        parentUri: Uri,
        localDir: File,
        onProgress: (String) -> Unit
    ) {
        val queue = ArrayDeque<RemoteDirEntry>()
        queue.add(RemoteDirEntry(parentUri, localDir, true))
        val contentResolver = context.contentResolver

        while (queue.isNotEmpty()) {
            val entry = queue.removeFirst()
            val currentParentUri = entry.parentUri
            val currentLocalDir = entry.localDir
            val childrenUri = DocumentsContract.buildChildDocumentsUriUsingTree(
                currentParentUri,
                DocumentsContract.getTreeDocumentId(currentParentUri)
            ) ?: continue

            val children = mutableListOf<RemoteChild>()
            contentResolver.query(
                childrenUri,
                arrayOf(
                    DocumentsContract.Document.COLUMN_DOCUMENT_ID,
                    DocumentsContract.Document.COLUMN_DISPLAY_NAME,
                    DocumentsContract.Document.COLUMN_MIME_TYPE,
                    DocumentsContract.Document.COLUMN_LAST_MODIFIED,
                    DocumentsContract.Document.COLUMN_SIZE
                ),
                null, null, null
            )?.use { cursor ->
                while (cursor.moveToNext()) {
                    val docId = cursor.getString(0)
                    val displayName = cursor.getString(1)
                    val mimeType = cursor.getString(2)
                    val mtimeIdx = cursor.getColumnIndex(DocumentsContract.Document.COLUMN_LAST_MODIFIED)
                    val sizeIdx = cursor.getColumnIndex(DocumentsContract.Document.COLUMN_SIZE)
                    val remoteMtime = if (mtimeIdx >= 0) cursor.getLong(mtimeIdx) else -1L
                    val remoteSize = if (sizeIdx >= 0) cursor.getLong(sizeIdx) else -1L
                    val docUri = DocumentsContract.buildDocumentUriUsingTree(currentParentUri, docId)
                    children.add(RemoteChild(docUri, displayName, mimeType, remoteMtime, remoteSize))
                }
            }

            for (child in children) {
                try {
                    if (entry.isRoot && child.displayName in AppLevelNames.NAMES) {
                        continue
                    }
                    if (child.mimeType == DocumentsContract.Document.MIME_TYPE_DIR) {
                        val childDir = File(currentLocalDir, child.displayName)
                        if (!childDir.mkdirs()) {
                            android.util.Log.w(
                                "SoloSoul",
                                "syncTreeToLocalDir: failed to create directory '${child.displayName}' in ${currentLocalDir.path}, skipping"
                            )
                            continue
                        }
                        queue.add(RemoteDirEntry(child.docUri, childDir, false))
                    } else {
                        try {
                            val file = File(currentLocalDir, child.displayName)
                            if (file.exists() && child.lastModified > 0 && child.size > 0 &&
                                file.lastModified() == child.lastModified && file.length() == child.size) {
                                continue
                            }
                            file.parentFile?.mkdirs()

                            val tmpFile = File(currentLocalDir, "${child.displayName}.tmp")
                            contentResolver.openInputStream(child.docUri)?.use { input ->
                                FileOutputStream(tmpFile).use { output ->
                                    input.copyTo(output)
                                }
                            } ?: continue

                            if (!tmpFile.renameTo(file)) {
                                android.util.Log.w(
                                    "SoloSoul",
                                    "原子重命名失败，尝试直接替换: ${child.displayName}"
                                )
                                file.delete()
                                tmpFile.renameTo(file)
                            }

                            if (child.lastModified > 0) {
                                file.setLastModified(child.lastModified)
                            }

                            onProgress(child.displayName)
                        } catch (e: Exception) {
                            val msg = e.message ?: ""
                            if (msg.contains("ENAMETOOLONG")) {
                                android.util.Log.w(
                                    "SoloSoul",
                                    "syncTreeToLocalDir: skipping file '${child.displayName}' (name too long)"
                                )
                            } else {
                                android.util.Log.w(
                                    "SoloSoul",
                                    "syncTreeToLocalDir: skipping file '${child.displayName}' in ${currentLocalDir.path}: ${e.message}"
                                )
                            }
                        }
                    }
                } catch (e: Exception) {
                    val msg = e.message ?: ""
                    android.util.Log.w(
                        "SoloSoul",
                        "syncTreeToLocalDir: skipping child '${child.displayName}' in ${currentLocalDir.path}: $msg"
                    )
                }
            }
        }
    }

    /**
     * 在 parentUri 下创建临时 .tmp 文件，写入 sourceFile 内容。
     */
    private fun createTempDocumentAndWrite(
        context: Context,
        parentUri: Uri,
        tempName: String,
        mimeType: String,
        sourceFile: File
    ): Uri? {
        val tempDoc = DocumentsContract.createDocument(
            context.contentResolver, parentUri, mimeType, tempName
        ) ?: return null

        try {
            context.contentResolver.openOutputStream(tempDoc)?.use { output ->
                FileInputStream(sourceFile).use { input ->
                    input.copyTo(output)
                }
            } ?: run {
                try {
                    DocumentsContract.deleteDocument(context.contentResolver, tempDoc)
                } catch (_: Exception) {
                }
                return null
            }
        } catch (e: Exception) {
            try {
                DocumentsContract.deleteDocument(context.contentResolver, tempDoc)
            } catch (_: Exception) {
            }
            return null
        }
        return tempDoc
    }

    private fun getMimeType(fileName: String): String {
        val ext = fileName.substringAfterLast(".", "")
        return when (ext.lowercase()) {
            "jpg", "jpeg" -> "image/jpeg"
            "png" -> "image/png"
            "gif" -> "image/gif"
            "webp" -> "image/webp"
            "pdf" -> "application/pdf"
            "txt" -> "text/plain"
            "json" -> "application/json"
            "db" -> "application/x-sqlite3"
            else -> "application/octet-stream"
        }
    }
}
