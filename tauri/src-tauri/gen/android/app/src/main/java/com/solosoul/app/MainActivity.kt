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
  // 冷启动时 WebView 可能尚未挂树，先把快捷方式 action 暂存到这里，
  // 等 WebView 就绪后通过 tryFlushPendingShortcut 注入前端 sessionStorage。
  private var pendingShortcutAction: String? = null
  private val shortcutFlushHandler = android.os.Handler(android.os.Looper.getMainLooper())
  private var shortcutFlushAttempts = 0
  private val shortcutFlushRunnable = object : Runnable {
    override fun run() {
      val action = pendingShortcutAction ?: return
      tryFlushPendingShortcut(action) { success ->
        if (success) {
          pendingShortcutAction = null
          return@tryFlushPendingShortcut
        }
        // 最多重试 30 次（约 7.5 秒），覆盖低端机冷启动
        shortcutFlushAttempts++
        if (shortcutFlushAttempts >= 30) {
          android.util.Log.w("SoloSoul", "快捷方式注入重试次数已达上限，放弃: $action")
          return@tryFlushPendingShortcut
        }
        shortcutFlushHandler.postDelayed(this, 250)
      }
    }
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    // 小窗/多窗口模式下不启用 edge-to-edge：decorFitsSystemWindows=true 时
    // 系统自动把 WebView 内容排在窗口标题栏（caption bar）之下，
    // 避免前端 env(safe-area-inset-top)=0 导致内容被标题栏遮挡。
    if (!isInMultiWindowMode) {
      enableEdgeToEdge()
    }
    super.onCreate(savedInstanceState)
    // 启动时根据系统主题同步状态栏图标颜色，避免 WebView 加载前出现黑白不匹配。
    syncStatusBarStyleWithSystemTheme()
    // 将 APK assets 中的只读资源复制到应用私有文件目录，
    // 供 Rust 后端通过 std::fs 读取（Tauri Android 的 resource_dir 返回 asset:// URL）。
    // 注意：Rust 端使用 BaseDirectory::Data 解析到应用数据目录根，因此目标根目录也必须是 dataDir。
    extractAssetsToDataDir(assets, dataDir)
    // 处理快捷方式 intent（冷启动）
    handleShortcutIntent(intent)
    // 延迟重试注入 pending shortcut，以覆盖 WebView 尚未就绪的冷启动场景
    schedulePendingShortcutFlush()
  }

  override fun onNewIntent(intent: Intent) {
    super.onNewIntent(intent)
    // 处理快捷方式 intent（热启动）
    handleShortcutIntent(intent)
    // 热启动时也可能遇到 WebView 尚未就绪，重新启动 flush 轮询
    schedulePendingShortcutFlush()
  }

  override fun onMultiWindowModeChanged(isInMultiWindowMode: Boolean, newConfig: Configuration) {
    super.onMultiWindowModeChanged(isInMultiWindowMode, newConfig)
    // 运行时进出小窗/分屏模式时同步切换 decor 布局模式：
    // 多窗口下由系统把内容排在标题栏之下，回全屏恢复 edge-to-edge 沉浸。
    WindowCompat.setDecorFitsSystemWindows(window, !isInMultiWindowMode)
  }

  override fun onResume() {
    super.onResume()
    // 每次回到前台时尝试清空可能因 WebView 未就绪而遗留的 pending shortcut
    tryFlushPendingShortcut { /* no-op */ }
  }

  /**
   * 读取 intent 中的 shortcut_action extra，并通过 WebView 注入自定义 DOM 事件
   * 通知前端触发「新建对象」流程。若 WebView 尚未就绪则缓存到 pendingShortcutAction，
   * 稍后通过 schedulePendingShortcutFlush / tryFlushPendingShortcut 重试。
   */
  private fun handleShortcutIntent(intent: Intent?) {
    val action = intent?.getStringExtra("shortcut_action") ?: return
    if (action != "new_object") return
    tryFlushPendingShortcut(action) { success ->
      if (success) {
        pendingShortcutAction = null
      } else {
        pendingShortcutAction = action
        android.util.Log.w("SoloSoul", "WebView 未就绪，暂存快捷方式 action: $action")
      }
    }
  }

  private fun schedulePendingShortcutFlush() {
    shortcutFlushHandler.removeCallbacks(shortcutFlushRunnable)
    shortcutFlushAttempts = 0
    shortcutFlushHandler.postDelayed(shortcutFlushRunnable, 250)
  }

  override fun onDestroy() {
    super.onDestroy()
    shortcutFlushHandler.removeCallbacks(shortcutFlushRunnable)
  }

  private fun tryFlushPendingShortcut(onResult: (Boolean) -> Unit = {}) {
    val action = pendingShortcutAction
    if (action == null) {
      onResult(true)
      return
    }
    tryFlushPendingShortcut(action, onResult)
  }

  private fun tryFlushPendingShortcut(action: String, onResult: (Boolean) -> Unit) {
    val webView = findWebView(window.decorView)
    if (webView == null) {
      onResult(false)
      return
    }
    // 避免在 about:blank 等临时 origin 上写入 sessionStorage，
    // 否则前端加载后无法读取到 pending action。
    val url = webView.url
    if (url.isNullOrBlank() || url.startsWith("about:")) {
      android.util.Log.d("SoloSoul", "WebView 尚未加载应用页面，暂存快捷方式 action: $action")
      onResult(false)
      return
    }
    val script = """
      (function() {
        if (window.__SOLOSOUL_HANDLE_SHORTCUT__) {
          window.__SOLOSOUL_HANDLE_SHORTCUT__(${quoteJsString(action)});
          return true;
        } else {
          try {
            sessionStorage.setItem('solosoul_pending_shortcut', ${quoteJsString(action)});
            return true;
          } catch(e) {
            return false;
          }
        }
      })();
    """.trimIndent()
    webView.evaluateJavascript(script) { value ->
      onResult(value == "true")
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
     * 把 assets 下指定的资源目录递归复制到 dataDir/app_resources/。
     * - docs 完整复制（帮助文档）。
     * - SoloSoul_plugin_market 仅复制 registry.json 与每个插件的 manifest.json、plugin.wasm，
     *   避免把插件源码 target/ 编译产物打包进 APK / 复制到设备。
     * Tauri v2 打包后的资源可能位于 assets 根目录或 assets/resources/ 子目录，
     * 因此根目录找不到时会回退到 resources/ 子目录。
     */
    @JvmStatic
    fun extractAssetsToDataDir(assetManager: AssetManager, dataDir: File) {
      // 资源目录与 Vault 数据目录物理隔离，避免 SAF 同步时误把资源当 Vault 数据。
      val destRoot = File(dataDir, "app_resources")
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
