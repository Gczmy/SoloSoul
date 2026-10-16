package com.solosoul.app

import android.app.Activity
import android.database.ContentObserver
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.provider.Settings
import android.view.ViewTreeObserver
import android.view.accessibility.AccessibilityManager
import android.webkit.WebView
import androidx.webkit.ScriptHandler
import androidx.webkit.WebViewCompat
import androidx.webkit.WebViewFeature

/** WebView 的媒体查询未必接收系统高文字对比；仅向主文档交付这一只读偏好。 */
class AndroidHighTextContrast(private val activity: Activity, private val webView: WebView) {
    private val manager = requireNotNull(activity.getSystemService(AccessibilityManager::class.java))
    private val documentStart = WebViewFeature.isFeatureSupported(WebViewFeature.DOCUMENT_START_SCRIPT)
    private var scriptHandler: ScriptHandler? = null
    private var lastScript: String? = null
    private var disposed = false
    private var pendingDocument = true
    private var lastUrl: String? = null
    private val observer = object : ContentObserver(Handler(Looper.getMainLooper())) {
        override fun onChange(selfChange: Boolean) { refresh() }
    }
    private var removePlatformListener: (() -> Unit)? = null
    private val fallbackDraw = ViewTreeObserver.OnPreDrawListener {
        val url = webView.url
        if (webView.progress < 100 || url != lastUrl) pendingDocument = true
        lastUrl = url
        if (pendingDocument && webView.progress == 100 && !url.isNullOrBlank() && !url.startsWith("about:")) {
            pendingDocument = false
            refresh()
        }
        true
    }

    init {
        if (Build.VERSION.SDK_INT >= 36) {
            removePlatformListener = Api36.observe(manager, activity) { refresh() }
        } else {
            activity.contentResolver.registerContentObserver(Settings.Secure.getUriFor(KEY), false, observer)
        }
        refresh()
        if (!documentStart) webView.viewTreeObserver.addOnPreDrawListener(fallbackDraw)
    }

    fun refresh() {
        if (disposed) return
        val enabled = if (Build.VERSION.SDK_INT >= 36) Api36.enabled(manager)
            else Settings.Secure.getInt(activity.contentResolver, KEY, 0) == 1
        val script = """(() => {
          if (window !== window.top) return;
          window.__SOLOSOUL_ANDROID_HIGH_TEXT_CONTRAST__ = $enabled;
          const apply = () => {
            const root=document.documentElement;
            if (!root) return false;
            root.dataset.androidHighTextContrast=String(window.__SOLOSOUL_ANDROID_HIGH_TEXT_CONTRAST__);
            return true;
          };
          if (!apply()) {
            const observer=new MutationObserver(() => {if(apply()) observer.disconnect()});
            observer.observe(document,{childList:true,subtree:true});
          }
        })()""".trimIndent()
        if (script != lastScript) {
            if (documentStart) {
                scriptHandler?.remove()
                // 与安全区交付相同：兼容 Tauri 与开发域名，不开放 JS→原生调用。
                scriptHandler = WebViewCompat.addDocumentStartJavaScript(webView, script, setOf("*"))
            }
            lastScript = script
        }
        webView.evaluateJavascript(script, null)
    }

    fun dispose() {
        if (disposed) return
        disposed = true
        removePlatformListener?.invoke()
        removePlatformListener = null
        if (Build.VERSION.SDK_INT < 36) activity.contentResolver.unregisterContentObserver(observer)
        scriptHandler?.remove()
        scriptHandler = null
        if (!documentStart && webView.viewTreeObserver.isAlive)
            webView.viewTreeObserver.removeOnPreDrawListener(fallbackDraw)
    }

    // 隔离 API36 类型，旧 Android 不需要解析新的监听器接口。
    @android.annotation.TargetApi(36)
    private object Api36 {
        fun enabled(manager: AccessibilityManager): Boolean = manager.isHighContrastTextEnabled

        fun observe(manager: AccessibilityManager, activity: Activity, changed: () -> Unit): () -> Unit {
            val listener = AccessibilityManager.HighContrastTextStateChangeListener { changed() }
            manager.addHighContrastTextStateChangeListener(activity.mainExecutor, listener)
            return { manager.removeHighContrastTextStateChangeListener(listener) }
        }
    }

    private companion object { const val KEY = "high_text_contrast_enabled" }
}
