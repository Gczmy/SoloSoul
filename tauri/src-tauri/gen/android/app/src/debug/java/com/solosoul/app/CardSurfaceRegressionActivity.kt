package com.solosoul.app

import android.annotation.SuppressLint
import android.os.Bundle
import android.webkit.JavascriptInterface
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.appcompat.app.AppCompatActivity
import java.util.concurrent.ConcurrentLinkedQueue

/** 仅 Debug：合成 Card 页面，不加载 MainActivity、Tauri、Vault 或正式插件。 */
class CardSurfaceRegressionActivity : AppCompatActivity() {
    lateinit var webView: WebView
    val reports = ConcurrentLinkedQueue<String>()
    @Volatile var ready = false

    @SuppressLint("SetJavaScriptEnabled")
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        webView = WebView(this)
        setContentView(webView)
        webView.settings.apply {
            javaScriptEnabled = true
            blockNetworkLoads = true
            allowFileAccess = false
            allowContentAccess = false
            domStorageEnabled = false
        }
        webView.addJavascriptInterface(object {
            @JavascriptInterface fun report(value: String) {
                require(value.length <= 16384 && reports.size < 16)
                reports.add(value)
            }
        }, "AndroidCardFixture")
        webView.webViewClient = object : WebViewClient() {
            override fun shouldOverrideUrlLoading(view: WebView, request: android.webkit.WebResourceRequest) = true
            override fun onPageFinished(view: WebView, url: String) {
                if (url == "file:///android_asset/rf121-card-surfaces.html") {
                    view.evaluateJavascript("typeof window.__runCardSample === 'function'") { ready = it == "true" }
                }
            }
        }
        webView.loadUrl("file:///android_asset/rf121-card-surfaces.html")
    }

    override fun onDestroy() {
        webView.removeJavascriptInterface("AndroidCardFixture")
        webView.destroy()
        super.onDestroy()
    }
}
