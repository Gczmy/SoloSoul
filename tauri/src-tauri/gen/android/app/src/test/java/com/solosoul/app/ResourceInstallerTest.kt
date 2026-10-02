package com.solosoul.app

import org.junit.Assert.*
import org.junit.Test
import java.io.ByteArrayInputStream
import java.io.File
import java.nio.file.Files
import java.security.MessageDigest

class ResourceInstallerTest {
  private fun manifest(files: Map<String, String>): ResourceManifest {
    val entries = files.toSortedMap().map { (path, text) ->
      val bytes = text.toByteArray()
      ResourceEntry(path, bytes.size.toLong(), MessageDigest.getInstance("SHA-256")
        .digest(bytes).joinToString("") { "%02x".format(it) })
    }
    return ResourceManifest(ResourceManifest.contentVersion(entries), entries)
  }

  private fun withData(block: (File) -> Unit) {
    val data = Files.createTempDirectory("solosoul-resource-test").toFile()
    try { block(data) } finally { data.deleteRecursively() }
  }

  private fun install(data: File, files: Map<String, String>, expected: ResourceManifest = manifest(files)) =
    ResourceInstaller(data).install(expected) { ByteArrayInputStream(files.getValue(it).toByteArray()) }

  @Test fun firstInstallAndSameVersionHaveNoRepeatedWrites() = withData { data ->
    val files = mapOf("docs/guides/index.json" to "guide", "SoloSoul_plugin_market/registry.json" to "registry")
    val first = install(data, files)
    assertFalse(first.skipped)
    assertEquals(2, first.writtenFiles)
    val guide = File(data, "app_resources/docs/guides/index.json")
    guide.setLastModified(123000L)
    val again = ResourceInstaller(data).install(manifest(files)) { error("同版本不应打开 APK 资源") }
    assertTrue(again.skipped)
    assertEquals(0, again.writtenFiles)
    assertEquals(123000L, guide.lastModified())
    assertEquals("guide", guide.readText())
  }

  @Test fun upgradeRemovesOnlyPreviouslyManagedFiles() = withData { data ->
    install(data, mapOf("docs/old.md" to "old", "docs/guide.md" to "v1"))
    File(data, "app_resources/docs/personal.md").writeText("user")
    File(data, "account.dat").writeText("vault")
    install(data, mapOf("docs/guide.md" to "v2"))
    assertEquals("v2", File(data, "app_resources/docs/guide.md").readText())
    assertFalse(File(data, "app_resources/docs/old.md").exists())
    assertEquals("user", File(data, "app_resources/docs/personal.md").readText())
    assertEquals("vault", File(data, "account.dat").readText())
  }

  @Test fun checksumFailureKeepsReadyTreeAndRetryRecovers() = withData { data ->
    install(data, mapOf("docs/guide.md" to "old"))
    val next = mapOf("docs/guide.md" to "new")
    try { install(data, mapOf("docs/guide.md" to "bad"), manifest(next)); fail("应拒绝错误摘要") }
    catch (_: java.io.IOException) { }
    assertEquals("old", File(data, "app_resources/docs/guide.md").readText())
    install(data, next)
    assertEquals("new", File(data, "app_resources/docs/guide.md").readText())
  }

  @Test fun interruptedCopyNeverExposesHalfResources() = withData { data ->
    install(data, mapOf("docs/guide.md" to "old"))
    val files = mapOf("docs/a.md" to "one", "docs/b.md" to "two")
    try {
      ResourceInstaller(data).install(manifest(files)) {
        if (it.endsWith("b.md")) throw java.io.IOException("模拟复制中断")
        ByteArrayInputStream(files.getValue(it).toByteArray())
      }
      fail("应抛出复制失败")
    } catch (_: java.io.IOException) { }
    assertFalse(File(data, "app_resources/docs/a.md").exists())
    assertEquals("old", File(data, "app_resources/docs/guide.md").readText())
    install(data, files)
    assertEquals("two", File(data, "app_resources/docs/b.md").readText())
  }

  @Test fun interruptedDirectorySwitchRestoresBackupBeforeRetry() = withData { data ->
    install(data, mapOf("docs/guide.md" to "old"))
    val ready = File(data, "app_resources")
    val scratch = File(data, "resources/.solosoul-resource-install")
    scratch.mkdirs()
    assertTrue(ready.renameTo(File(scratch, "backup")))
    File(scratch, "stage").mkdirs()
    File(scratch, "stage/partial").writeText("incomplete")
    install(data, mapOf("docs/guide.md" to "new"))
    assertEquals("new", File(ready, "docs/guide.md").readText())
    assertFalse(File(ready, "partial").exists())
  }

  @Test fun corruptInstalledFileIsRepairedDespiteMatchingVersion() = withData { data ->
    val files = mapOf("docs/guide.md" to "good")
    install(data, files)
    File(data, "app_resources/docs/guide.md").writeText("evil")
    assertFalse(install(data, files).skipped)
    assertEquals("good", File(data, "app_resources/docs/guide.md").readText())
  }

  @Test fun invalidPathsAndVersionsCannotWriteOutsideResources() = withData { data ->
    for (path in listOf("../account.dat", "/tmp/file", "docs/../account.dat", "docs\\file", "docs/a\nfile")) {
      try { install(data, mapOf(path to "bad")); fail("应拒绝非法路径") }
      catch (_: IllegalArgumentException) { }
    }
    val valid = manifest(mapOf("docs/good.md" to "good"))
    try { install(data, mapOf("docs/good.md" to "good"), valid.copy(version = "0".repeat(64))); fail("应拒绝版本不匹配") }
    catch (_: IllegalArgumentException) { }
    assertFalse(File(data, "account.dat").exists())
  }
}
