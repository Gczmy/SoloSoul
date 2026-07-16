package com.solosoul.app

import android.content.Intent
import android.content.res.AssetManager
import android.content.res.Configuration
import android.os.Bundle
import android.webkit.WebView
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
    // 处理快捷方式 intent（冷启动）
    handleShortcutIntent(intent)
  }

  override fun onNewIntent(intent: Intent?) {
    super.onNewIntent(intent)
    // 处理快捷方式 intent（热启动）
    handleShortcutIntent(intent)
  }

  /**
   * 读取 intent 中的 shortcut_action extra，并通过 WebView 注入自定义 DOM 事件
   * 通知前端触发「新建对象」流程。若 WebView 尚未就绪则事件会被前端缓存消费。
   */
  private fun handleShortcutIntent(intent: Intent?) {
    val action = intent?.getStringExtra("shortcut_action") ?: return
    if (action != "new_object") return
    val webView = findWebView(window.decorView) ?: return
    val script = """
      (function() {
        if (window.__SOLOSOUL_HANDLE_SHORTCUT__) {
          window.__SOLOSOUL_HANDLE_SHORTCUT__(${quoteJsString(action)});
        } else {
          try { sessionStorage.setItem('solosoul_pending_shortcut', ${quoteJsString(action)}); } catch(e) {}
        }
      })();
    """.trimIndent()
    runOnUiThread {
      webView.evaluateJavascript(script, null)
    }
  }

  /** 简单转义字符串供 JS 使用，避免引入额外依赖 */
  private fun quoteJsString(s: String): String {
    return "\"" + s.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n") + "\""
  }

  /**
   * 递归查找 Tauri 注入的 WebView（Tauri 2 不会暴露固定 ID）。
   */
  private fun findWebView(root: android.view.View?): WebView? {
    if (root == null) return null
    if (root is WebView) return root
    if (root is android.view.ViewGroup) {
      for (i in 0 until root.childCount) {
        val child = root.getChildAt(i)
        findWebView(child)?.let { return it }
      }
    }
    return null
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
     * - docs 完整复制（帮助文档）。
     * - SoloSoul_plugin_market 仅复制 registry.json 与每个插件的 manifest.json、plugin.wasm，
     *   避免把插件源码 target/ 编译产物打包进 APK / 复制到设备。
     * Tauri v2 打包后的资源可能位于 assets 根目录或 assets/resources/ 子目录，
     * 因此根目录找不到时会回退到 resources/ 子目录。
     */
    @JvmStatic
    fun extractAssetsToDataDir(assetManager: AssetManager, dataDir: File) {
      val destRoot = File(dataDir, "resources")
      android.util.Log.i("SoloSoul", "开始复制资源到: ${destRoot.absolutePath}")

      // 1. 复制 docs
      val docsCopied = tryCopyAssetDir(assetManager, "docs", File(destRoot, "docs"))
      if (!docsCopied) {
        val fallbackCopied = tryCopyAssetDir(assetManager, "resources/docs", File(destRoot, "docs"))
        android.util.Log.i("SoloSoul", "docs 资源回退复制: $fallbackCopied")
      }

      // 2. 复制 SoloSoul_plugin_market 的精简内容
      extractPluginMarket(assetManager, destRoot)

      // 关键资源存在性校验，帮助后续排查
      val guideIndex = File(destRoot, "docs/guides/index.json")
      if (!guideIndex.exists()) {
        android.util.Log.e("SoloSoul", "帮助索引未找到: ${guideIndex.absolutePath}")
      } else {
        android.util.Log.i("SoloSoul", "帮助索引已就绪: ${guideIndex.absolutePath}")
      }
    }

    /**
     * 仅复制插件市场运行所需的最小文件集合：
     * - registry.json
     * - plugins/<id>/manifest.json
     * - plugins/<id>/plugin.wasm
     */
    @JvmStatic
    private fun extractPluginMarket(assetManager: AssetManager, destRoot: File) {
      val sourcePrefixes = listOf("SoloSoul_plugin_market", "resources/SoloSoul_plugin_market")
      var anyCopied = false

      for (prefix in sourcePrefixes) {
        val registrySrc = "$prefix/registry.json"
        val registryDest = File(destRoot, "SoloSoul_plugin_market/registry.json")
        if (assetExists(assetManager, registrySrc)) {
          try {
            copyAssetFile(assetManager, registrySrc, registryDest)
            anyCopied = true
          } catch (e: IOException) {
            android.util.Log.w("SoloSoul", "复制注册表失败: ${e.message}")
          }
        }

        val pluginsSrc = "$prefix/plugins"
        val pluginIds = assetManager.list(pluginsSrc) ?: emptyArray()
        for (pluginId in pluginIds) {
          val pluginDirSrc = "$pluginsSrc/$pluginId"
          val pluginDirDest = File(destRoot, "SoloSoul_plugin_market/plugins/$pluginId")
          listOf("manifest.json", "plugin.wasm").forEach { fileName ->
            val fileSrc = "$pluginDirSrc/$fileName"
            if (assetExists(assetManager, fileSrc)) {
              try {
                copyAssetFile(assetManager, fileSrc, File(pluginDirDest, fileName))
              } catch (e: IOException) {
                android.util.Log.w("SoloSoul", "复制插件文件失败 $fileSrc: ${e.message}")
              }
            }
          }
        }
      }

      android.util.Log.i("SoloSoul", "插件市场资源复制完成: $anyCopied")
    }

    @JvmStatic
    private fun assetExists(assetManager: AssetManager, path: String): Boolean {
      return try {
        assetManager.open(path).close()
        true
      } catch (_: IOException) {
        false
      }
    }

    @JvmStatic
    private fun tryCopyAssetDir(assetManager: AssetManager, assetPath: String, destDir: File): Boolean {
      return try {
        copyAssetDir(assetManager, assetPath, destDir)
        val children = assetManager.list(assetPath) ?: emptyArray()
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
