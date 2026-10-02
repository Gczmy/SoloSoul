package com.solosoul.app

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.security.MessageDigest

/** 对实际 APK 资源执行安装及重启零重写检查，数据仅放专用测试临时目录。 */
@RunWith(AndroidJUnit4::class)
class ResourceInstallerInstrumentedTest {
  @Test fun packagedResourcesInstallAndSecondStartWritesNothing() {
    val context = InstrumentationRegistry.getInstrumentation().targetContext
    val data = File(context.cacheDir, "rf206-resource-install-test")
    data.deleteRecursively()
    data.mkdirs()
    try {
      MainActivity.extractAssetsToDataDir(context.assets, data)
      val manifest = context.assets.open("bundled-resource-manifest.json").bufferedReader()
        .use { JSONObject(it.readText()) }
      val files = manifest.getJSONArray("files")
      val snapshots = mutableMapOf<String, Long>()
      for (index in 0 until files.length()) {
        val entry = files.getJSONObject(index)
        val file = File(data, "app_resources/${entry.getString("path")}")
        assertEquals(entry.getLong("size"), file.length())
        val digest = MessageDigest.getInstance("SHA-256").digest(file.readBytes())
          .joinToString("") { "%02x".format(it) }
        assertEquals(entry.getString("sha256"), digest)
        assertTrue(file.setLastModified(123000L))
        snapshots[entry.getString("path")] = file.lastModified()
      }
      MainActivity.extractAssetsToDataDir(context.assets, data)
      for ((path, modified) in snapshots) {
        assertEquals("重复启动重写了 $path", modified, File(data, "app_resources/$path").lastModified())
      }
      assertTrue(JSONObject(File(data, "app_resources/docs/guides/index.json").readText()).length() > 0)
      assertTrue(JSONObject(File(data, "app_resources/SoloSoul_plugin_market/registry.json").readText()).length() > 0)
      assertTrue(File(data, "app_resources/SoloSoul_plugin_market/plugins").walkTopDown()
        .any { it.name == "plugin.wasm" && it.length() > 8 })
      android.util.Log.i("SoloSoul", "RF206 实际 APK ${files.length()} 文件校验与重复启动零重写通过")
    } finally { data.deleteRecursively() }
  }
}
