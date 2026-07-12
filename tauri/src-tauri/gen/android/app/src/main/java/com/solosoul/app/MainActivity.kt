package com.solosoul.app

import android.content.res.AssetManager
import android.os.Bundle
import androidx.activity.enableEdgeToEdge
import java.io.File
import java.io.IOException

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    // 将 APK assets 中的只读资源复制到应用私有文件目录，
    // 供 Rust 后端通过 std::fs 读取（Tauri Android 的 resource_dir 返回 asset:// URL）。
    extractAssetsToFilesDir(assets, filesDir)
  }

  companion object {
    /**
     * 把 assets 下指定的资源目录递归复制到 [destRoot]/resources/。
     * 仅复制 docs 与 SoloSoul_plugin_market（当前 Rust 代码主要使用的两类资源）。
     */
    @JvmStatic
    fun extractAssetsToFilesDir(assetManager: AssetManager, filesDir: File) {
      val destRoot = File(filesDir, "resources")
      listOf("docs", "SoloSoul_plugin_market").forEach { assetDir ->
        try {
          copyAssetDir(assetManager, assetDir, File(destRoot, assetDir))
        } catch (e: IOException) {
          android.util.Log.w("SoloSoul", "跳过资源复制 $assetDir: ${e.message}")
        }
      }
    }

    @JvmStatic
    @Throws(IOException::class)
    private fun copyAssetDir(assetManager: AssetManager, assetPath: String, destDir: File) {
      val children = assetManager.list(assetPath) ?: return
      if (children.isEmpty()) {
        // 单个文件
        copyAssetFile(assetManager, assetPath, destDir)
        return
      }
      destDir.mkdirs()
      for (child in children) {
        val src = "$assetPath/$child"
        val dst = File(destDir, child)
        val subChildren = assetManager.list(src)
        if (subChildren.isNullOrEmpty()) {
          copyAssetFile(assetManager, src, dst)
        } else {
          copyAssetDir(assetManager, src, dst)
        }
      }
    }

    @JvmStatic
    @Throws(IOException::class)
    private fun copyAssetFile(assetManager: AssetManager, assetPath: String, destFile: File) {
      destFile.parentFile?.mkdirs()
      assetManager.open(assetPath).use { input ->
        destFile.outputStream().use { output ->
          input.copyTo(output)
        }
      }
    }
  }
}
