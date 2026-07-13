package com.solosoul.app

import android.content.res.AssetManager
import android.content.res.Configuration
import android.os.Bundle
import androidx.activity.enableEdgeToEdge
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsControllerCompat
import java.io.File
import java.io.IOException

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    // 启动时根据系统主题同步状态栏图标颜色，避免 WebView 加载前出现黑白不匹配。
    syncStatusBarStyleWithSystemTheme()
    // 将 APK assets 中的只读资源复制到应用私有文件目录，
    // 供 Rust 后端通过 std::fs 读取（Tauri Android 的 resource_dir 返回 asset:// URL）。
    // 注意：Rust 端使用 BaseDirectory::Data 解析到应用数据目录根，因此目标根目录也必须是 dataDir。
    extractAssetsToDataDir(assets, dataDir)
  }

  /**
   * 根据系统当前 day/night 模式设置状态栏/导航栏图标颜色，
   * 使启动闪屏与 WebView 加载期间的系统栏风格与系统主题一致。
   * 前端加载完成后会通过 status-bar plugin 重新按应用内主题覆盖。
   */
  private fun syncStatusBarStyleWithSystemTheme() {
    val isNight = (resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK) ==
      Configuration.UI_MODE_NIGHT_YES
    val window = window
    val rootView = window.decorView.rootView
    val controller = WindowCompat.getInsetsController(window, rootView)
      ?: return
    // 深色系统主题 → 浅色图标/文字；浅色系统主题 → 深色图标/文字。
    controller.isAppearanceLightStatusBars = !isNight
    controller.isAppearanceLightNavigationBars = !isNight
  }

  companion object {
    /**
     * 把 assets 下指定的资源目录递归复制到 [destRoot]/resources/。
     * 仅复制 docs 与 SoloSoul_plugin_market（当前 Rust 代码主要使用的两类资源）。
     * Tauri v2 打包后的资源可能位于 assets 根目录或 assets/resources/ 子目录，
     * 因此根目录找不到时会回退到 resources/ 子目录。
     */
    @JvmStatic
    fun extractAssetsToDataDir(assetManager: AssetManager, dataDir: File) {
      val destRoot = File(dataDir, "resources")
      android.util.Log.i("SoloSoul", "开始复制资源到: ${destRoot.absolutePath}")
      android.util.Log.d("SoloSoul", "Assets 根目录列表: ${assetManager.list("")?.joinToString()}")
      listOf("docs" to "docs", "SoloSoul_plugin_market" to "SoloSoul_plugin_market").forEach { (assetDir, destDirName) ->
        val copied = tryCopyAssetDir(assetManager, assetDir, File(destRoot, destDirName))
        if (!copied) {
          // 回退：Tauri 可能把资源放在 assets/resources/ 下
          val fallbackCopied = tryCopyAssetDir(assetManager, "resources/$assetDir", File(destRoot, destDirName))
          android.util.Log.i("SoloSoul", "资源回退复制 $assetDir: $fallbackCopied")
        }
      }
      // 关键资源存在性校验，帮助后续排查
      val guideIndex = File(destRoot, "docs/guides/index.json")
      if (!guideIndex.exists()) {
        android.util.Log.e("SoloSoul", "帮助索引未找到: ${guideIndex.absolutePath}")
      } else {
        android.util.Log.i("SoloSoul", "帮助索引已就绪: ${guideIndex.absolutePath}")
      }
    }

    @JvmStatic
    private fun tryCopyAssetDir(assetManager: AssetManager, assetPath: String, destDir: File): Boolean {
      return try {
        copyAssetDir(assetManager, assetPath, destDir)
        val children = assetManager.list(assetPath) ?: emptyArray()
        android.util.Log.d("SoloSoul", "资源复制完成: $assetPath -> ${destDir.absolutePath} (entries=${children.size})")
        children.isNotEmpty() || destDir.exists()
      } catch (e: IOException) {
        android.util.Log.w("SoloSoul", "跳过资源复制 $assetPath: ${e.message}")
        false
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
