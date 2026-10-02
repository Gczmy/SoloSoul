package com.solosoul.app

import java.io.File
import java.io.IOException
import java.io.InputStream
import java.nio.file.Files
import java.security.MessageDigest

data class ResourceEntry(val path: String, val size: Long, val sha256: String)

data class ResourceManifest(val version: String, val files: List<ResourceEntry>) {
  init {
    require(files.isNotEmpty() && files.map { it.path }.distinct().size == files.size)
    files.forEach {
      require(it.path.startsWith("docs/") || it.path.startsWith("SoloSoul_plugin_market/"))
      require(it.path.split('/').none { part -> part.isEmpty() || part == "." || part == ".." })
      require(it.path.none { char -> char in "\\\t\r\n\u0000" })
      require(it.size >= 0 && it.sha256.matches(Regex("[0-9a-f]{64}")))
    }
    require(version == contentVersion(files))
  }

  fun marker(): String = version + "\n" + files.sortedBy { it.path }.joinToString("") {
    "${it.path}\t${it.size}\t${it.sha256}\n"
  }

  companion object {
    fun contentVersion(files: List<ResourceEntry>): String {
      val text = files.sortedBy { it.path }.joinToString("") {
        "${it.path}\u0000${it.size}\u0000${it.sha256}\n"
      }
      return digest(text.toByteArray())
    }

    fun fromMarker(text: String): ResourceManifest {
      val lines = text.trimEnd('\n').split('\n')
      val entries = lines.drop(1).map {
        val parts = it.split('\t'); require(parts.size == 3)
        ResourceEntry(parts[0], parts[1].toLong(), parts[2])
      }
      return ResourceManifest(lines.first(), entries)
    }

    internal fun digest(bytes: ByteArray): String = MessageDigest.getInstance("SHA-256")
      .digest(bytes).joinToString("") { "%02x".format(it) }
  }
}

data class ResourceInstallResult(val skipped: Boolean, val writtenFiles: Int, val version: String)

/** 仅管理 APK 内置内容；同步准备完成后才返回。暂存/备份位于已排除同步的 resources 下。 */
class ResourceInstaller(private val dataDir: File) {
  private val ready = File(dataDir, "app_resources")
  private val scratch = File(dataDir, "resources/.solosoul-resource-install")
  private val stage = File(scratch, "stage")
  private val backup = File(scratch, "backup")
  private val markerName = ".bundled-resource-manifest"

  fun install(manifest: ResourceManifest, openAsset: (String) -> InputStream): ResourceInstallResult =
    synchronized(lock) {
      // 目录切换中断时先恢复上一个完整树，绝不把 stage 当成 ready。
      ensureNoLinks(dataDir, scratch)
      ensureNoLinks(dataDir, ready)
      if (backup.exists()) {
        if (!ready.exists()) move(backup, ready) else remove(backup)
      }
      val previous = readPrevious()
      if (previous == manifest && manifest.files.all { matches(File(ready, it.path), it) }) {
        remove(stage)
        return@synchronized ResourceInstallResult(true, 0, manifest.version)
      }
      remove(stage)
      if (!stage.mkdirs()) throw IOException("无法创建资源暂存目录")
      // 升级只移除旧清单明确拥有的文件，其他文件完整保留；首次升级无清单时保守保留。
      val owned = previous?.files?.map { it.path }?.toSet() ?: emptySet()
      if (ready.exists()) preserveUnmanaged(ready, "", owned)
      for (entry in manifest.files) {
        val dest = File(stage, entry.path)
        dest.parentFile?.mkdirs()
        openAsset(entry.path).use { input ->
          dest.outputStream().use { output -> input.copyTo(output); output.fd.sync() }
        }
        if (!matches(dest, entry)) throw IOException("内置资源校验失败: ${entry.path}")
      }
      // 全部文件校验后写完成标记，再切换完整目录；旧树保留到新树落位。
      File(stage, markerName).outputStream().use { output ->
        output.write(manifest.marker().toByteArray()); output.fd.sync()
      }
      if (ready.exists()) move(ready, backup)
      try { move(stage, ready) }
      catch (error: IOException) {
        if (backup.exists() && !ready.exists()) move(backup, ready)
        throw error
      }
      remove(backup)
      ResourceInstallResult(false, manifest.files.size, manifest.version)
    }

  private fun readPrevious(): ResourceManifest? = try {
    val marker = File(ready, markerName)
    ensureNoLinks(dataDir, marker)
    if (marker.isFile) ResourceManifest.fromMarker(marker.readText()) else null
  } catch (_: IllegalArgumentException) { null }
    catch (_: IOException) { null }

  private fun matches(file: File, entry: ResourceEntry): Boolean {
    ensureNoLinks(dataDir, file)
    if (!file.isFile || file.length() != entry.size) return false
    val digest = MessageDigest.getInstance("SHA-256")
    file.inputStream().use { input ->
      val buffer = ByteArray(64 * 1024)
      while (true) { val count = input.read(buffer); if (count < 0) break; digest.update(buffer, 0, count) }
    }
    return digest.digest().joinToString("") { "%02x".format(it) } == entry.sha256
  }

  private fun preserveUnmanaged(dir: File, prefix: String, owned: Set<String>) {
    for (file in dir.listFiles() ?: throw IOException("无法读取现有资源目录")) {
      ensureNoLinks(dataDir, file)
      val path = prefix + file.name
      if (file.isDirectory) preserveUnmanaged(file, "$path/", owned)
      else if (path != markerName && path !in owned) {
        val target = File(stage, path); target.parentFile?.mkdirs(); file.copyTo(target)
      }
    }
  }

  private fun move(from: File, to: File) {
    to.parentFile?.mkdirs()
    if (!from.renameTo(to)) throw IOException("无法切换内置资源目录")
  }

  private fun remove(file: File) {
    ensureNoLinks(dataDir, file)
    if (file.isDirectory) file.walkTopDown().forEach { ensureNoLinks(dataDir, it) }
    if (file.exists() && !file.deleteRecursively()) throw IOException("无法清理内置资源暂存目录")
  }

  private fun ensureNoLinks(base: File, target: File) {
    val root = base.toPath().toAbsolutePath().normalize()
    val path = target.toPath().toAbsolutePath().normalize()
    if (!path.startsWith(root)) throw IOException("资源路径超出应用数据目录")
    var current = root
    for (part in root.relativize(path)) {
      current = current.resolve(part)
      if (Files.isSymbolicLink(current)) throw IOException("资源路径不能包含符号链接")
    }
  }

  companion object { private val lock = Any() }
}
