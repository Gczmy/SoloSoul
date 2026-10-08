package com.solosoul.app

import android.app.Activity
import android.os.Build
import android.view.ViewTreeObserver
import android.webkit.WebView
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.webkit.ScriptHandler
import androidx.webkit.WebViewCompat
import androidx.webkit.WebViewFeature
import org.json.JSONObject

/** 系统栏只约束内容；WebView / 顶栏背景仍铺到窗口边缘，不增加不透明原生遮罩。 */
class AndroidSafeArea(private val activity: Activity, private val webView: WebView) {
    private var scriptHandler: ScriptHandler? = null
    private var lastScript: String? = null
    private val documentStart = WebViewFeature.isFeatureSupported(WebViewFeature.DOCUMENT_START_SCRIPT)
    private var pendingDocument = true
    private var lastUrl: String? = null
    // 老 WebView 没有文档开始注入能力：仅页面加载完成时重新交付，不持续执行 JS。
    private val fallbackDraw = ViewTreeObserver.OnPreDrawListener {
        val url = webView.url
        if (webView.progress < 100 || url != lastUrl) pendingDocument = true
        lastUrl = url
        if (pendingDocument && webView.progress == 100 && url != null && !url.startsWith("about:")) {
            pendingDocument = false
            ViewCompat.getRootWindowInsets(webView)?.let { update(it) }
        }
        true
    }

    init {
        publish(JSONObject().put("top",0).put("right",0).put("bottom",0).put("left",0).put("width",0))
        if (!documentStart) webView.viewTreeObserver.addOnPreDrawListener(fallbackDraw)
    }

    fun update(insets: WindowInsetsCompat) {
        if (webView.width <= 0 || webView.height <= 0) return
        val bars = insets.getInsets(WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout())
        val location = IntArray(2)
        webView.getLocationOnScreen(location)
        val bounds = if (Build.VERSION.SDK_INT >= 30) activity.windowManager.currentWindowMetrics.bounds
            else android.graphics.Rect(0,0,activity.resources.displayMetrics.widthPixels,activity.resources.displayMetrics.heightPixels)
        // 仅交付 WebView 仍覆盖的部分：系统已经适配小窗或 IME 后，不能重复留白。
        publish(JSONObject()
            .put("top",(bounds.top+bars.top-location[1]).coerceIn(0,webView.height))
            .put("left",(bounds.left+bars.left-location[0]).coerceIn(0,webView.width))
            .put("right",(location[0]+webView.width-bounds.right+bars.right).coerceIn(0,webView.width))
            .put("bottom",(location[1]+webView.height-bounds.bottom+bars.bottom).coerceIn(0,webView.height))
            .put("width",webView.width))
    }

    private fun publish(values: JSONObject) {
        val script = """(() => {
          if (window !== window.top) return;
          window.__SOLOSOUL_ANDROID_SAFE_AREA__ = $values;
          if (!window.__SOLOSOUL_APPLY_SAFE_AREA__) {
            const apply = () => {
              const root=document.documentElement,area=window.__SOLOSOUL_ANDROID_SAFE_AREA__;
              if (!root) return false;
              const scale=area.width>0&&innerWidth>0?area.width/innerWidth:devicePixelRatio||1;
              for (const edge of ['top','right','bottom','left'])
                root.style.setProperty('--android-native-safe-area-'+edge,(area[edge]/scale)+'px');
              return true;
            };
            window.__SOLOSOUL_APPLY_SAFE_AREA__=apply;
            window.addEventListener('resize',apply);
            if (!apply()) {
              const observer=new MutationObserver(() => {if(apply()) observer.disconnect()});
              observer.observe(document,{childList:true,subtree:true});
            }
          }
          window.__SOLOSOUL_APPLY_SAFE_AREA__();
        })()""".trimIndent()
        if (script != lastScript && documentStart) {
            scriptHandler?.remove()
            // 只写布局长度，且只在主文档执行；不创建 JS→原生桥、网络或账户能力。
            // 通配 origin 保留 Tauri 自定义域名及局域网开发地址的兼容性。
            scriptHandler=WebViewCompat.addDocumentStartJavaScript(webView,script,setOf("*"))
        }
        lastScript=script
        webView.evaluateJavascript(script,null)
    }

    fun dispose() {
        scriptHandler?.remove()
        scriptHandler=null
        if (!documentStart && webView.viewTreeObserver.isAlive)
            webView.viewTreeObserver.removeOnPreDrawListener(fallbackDraw)
    }
}
