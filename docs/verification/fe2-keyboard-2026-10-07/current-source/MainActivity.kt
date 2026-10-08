package com.solosoul.app

import android.content.Intent
import android.content.res.AssetManager
import android.content.res.Configuration
import android.os.Bundle
import android.os.Build
import android.webkit.WebView
import android.view.ViewGroup
import androidx.activity.enableEdgeToEdge
import androidx.activity.OnBackPressedCallback
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import java.io.File
import java.io.IOException
import org.json.JSONObject
import com.google.android.material.snackbar.Snackbar

class MainActivity : TauriActivity() {
  private var removeResourceObserver: (() -> Unit)? = null
  private var resourceErrorNotice: Snackbar? = null
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

  override fun onWebViewCreate(webView: WebView) {
    super.onWebViewCreate(webView)
    var originalBottomMargin: Int? = null
    ViewCompat.setOnApplyWindowInsetsListener(webView) { view, insets ->
      val params = view.layoutParams as? ViewGroup.MarginLayoutParams
      val parent = view.parent as? android.view.View
      if (params != null && parent != null && parent.height > 0) {
        val original = originalBottomMargin ?: params.bottomMargin.also { originalBottomMargin = it }
        val imeHeight = insets.getInsets(WindowInsetsCompat.Type.ime()).bottom
        val parentLocation = IntArray(2)
        parent.getLocationOnScreen(parentLocation)
        // edge-to-edge 下 adjustResize 未必缩小 WebView，visualViewport 也可能
        // 仍报告全屏。只避让父容器实际被 IME 覆盖的部分，让 CSS 的固定按钮、
        // dvh 和可滚动区获得真实高度；父容器已缩小时不重复扣除键盘高度。
        // WindowMetrics 使用窗口实际边界；不能拿已被 adjustResize 缩小的
        // rootView.height 再减 IME，否则会重复扣除。旧系统全屏取显示边界，
        // 多窗口仍交由既有 decorFitsSystemWindows 的系统布局避让。
        val windowBottom = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R)
          windowManager.currentWindowMetrics.bounds.bottom
        else android.util.DisplayMetrics().also {
          @Suppress("DEPRECATION")
          windowManager.defaultDisplay.getRealMetrics(it)
        }.heightPixels
        val keyboardTop = windowBottom - imeHeight
        val overlap = if (!isInMultiWindowMode && insets.isVisible(WindowInsetsCompat.Type.ime()))
          (parentLocation[1] + parent.height - keyboardTop).coerceAtLeast(0)
        else 0
        val bottomMargin = maxOf(original, overlap)
        if (params.bottomMargin != bottomMargin) {
          params.bottomMargin = bottomMargin
          view.layoutParams = params
        }
      }
      // 保留系统栏 / cutout 的原有分发，不消费或修改 insets。
      insets
    }
    webView.addOnLayoutChangeListener { view, _, _, _, _, _, _, _, _ ->
      // 初次挂树、旋转和多窗口布局变化后重新按当前父容器计算。
      ViewCompat.requestApplyInsets(view)
    }
    webView.post { ViewCompat.requestApplyInsets(webView) }
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    // 小窗/多窗口模式下不启用 edge-to-edge：decorFitsSystemWindows=true 时
    // 系统自动把 WebView 内容排在窗口标题栏（caption bar）之下，
    // 避免前端 env(safe-area-inset-top)=0 导致内容被标题栏遮挡。
    if (!isInMultiWindowMode) {
      enableEdgeToEdge()
    }
    android.os.Trace.beginSection("SoloSoul.resources.enqueue")
    try { AndroidResources.prepare(applicationContext) }
    finally { android.os.Trace.endSection() }
    super.onCreate(savedInstanceState)
    // TauriActivity 不安装 Wry 返回回调；且 WebView.canGoBack() 在当前
    // 本地 SPA 中不能反映 pushState 历史。用 Router 的 idx 判断是否可退，
    // 让前端 popstate 守卫先关闭浮层，根页面才交还 Android 系统。
    onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
      private var pending = false

      override fun handleOnBackPressed() {
        if (pending) return
        val view = findWebView(window.decorView)
        if (view == null) {
          systemBack()
          return
        }
        pending = true
        view.evaluateJavascript("""
          (() => {
            const index = window.history.state?.idx;
            if (Number.isInteger(index) && index > 0) {
              window.history.back();
              return true;
            }
            return false;
          })()
        """.trimIndent()) { handled ->
          pending = false
          if (isDestroyed || isFinishing) return@evaluateJavascript
          if (handled != "true") systemBack()
        }
      }

      private fun systemBack() {
        isEnabled = false
        try { onBackPressedDispatcher.onBackPressed() }
        finally { isEnabled = true }
      }
    })
    // 启动时根据系统主题同步状态栏图标颜色，避免 WebView 加载前出现黑白不匹配。
    syncStatusBarStyleWithSystemTheme()
    // 将 APK assets 中的只读资源复制到应用私有文件目录，
    // 供 Rust 后端通过 std::fs 读取（Tauri Android 的 resource_dir 返回 asset:// URL）。
    // 注意：Rust 端使用 BaseDirectory::Data 解析到应用数据目录根，因此目标根目录也必须是 dataDir。
    removeResourceObserver = AndroidResources.observe { state ->
      runOnUiThread {
        if (isDestroyed || isFinishing || AndroidResources.snapshot() != state) return@runOnUiThread
        resourceErrorNotice?.dismiss()
        resourceErrorNotice = null
        if (state.status == "error") {
          val chinese = resources.configuration.locales[0].language == "zh"
          resourceErrorNotice = Snackbar.make(
            window.decorView,
            if (chinese) "帮助与内置插件资源准备失败" else "Help and bundled plugin resources could not be prepared",
            Snackbar.LENGTH_INDEFINITE
          ).setAction(if (chinese) "重试" else "Retry") {
            AndroidResources.prepare(applicationContext, retry = true)
          }.also { it.show() }
        }
      }
    }
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
    removeResourceObserver?.invoke()
    removeResourceObserver = null
    resourceErrorNotice?.dismiss()
    resourceErrorNotice = null
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
    /** 保持同步就绪语义：只在完整清单校验及目录切换完成后返回。 */
    @JvmStatic
    fun extractAssetsToDataDir(assetManager: AssetManager, dataDir: File): ResourceInstallResult {
      fun openAsset(path: String): java.io.InputStream = try {
        assetManager.open(path)
      } catch (_: IOException) {
        assetManager.open("resources/$path")
      }
      val json = openAsset("bundled-resource-manifest.json").bufferedReader().use { JSONObject(it.readText()) }
      require(json.getInt("schema") == 1) { "不支持的内置资源清单版本" }
      val entries = json.getJSONArray("files")
      val files = (0 until entries.length()).map { index ->
        val entry = entries.getJSONObject(index)
        ResourceEntry(entry.getString("path"), entry.getLong("size"), entry.getString("sha256"))
      }
      val result = ResourceInstaller(dataDir).install(ResourceManifest(json.getString("version"), files), ::openAsset)
      android.util.Log.i("SoloSoul", "内置资源就绪 version=${result.version} skipped=${result.skipped} writtenFiles=${result.writtenFiles}")
      return result
    }
  }
}
