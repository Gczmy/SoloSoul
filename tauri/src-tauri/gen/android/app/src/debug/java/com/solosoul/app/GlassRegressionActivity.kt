package com.solosoul.app

import android.os.Bundle
import android.webkit.WebView
import androidx.appcompat.app.AppCompatActivity

/** 无 Vault/Tauri 启动的原生测试页，系统模糊直接采样这个虚构 WebView。 */
class GlassRegressionActivity : AppCompatActivity() {
    lateinit var glass: AndroidGlassPlugin
    lateinit var webView: WebView
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        webView = WebView(this)
        setContentView(webView)
        webView.loadDataWithBaseURL(null, """
            <meta name="viewport" content="width=device-width,initial-scale=1">
            <style>body{font:18px sans-serif;background:#f9f9f3;color:#20251f;padding:20px}h1{font-size:28px}
            article{padding:24px;margin:12px 0;border-radius:22px;background:#d9e7f8}article:nth-child(2n){background:#e4eaca}</style>
            <h1>SoloSoul / Material test</h1><p>Fictional objects only</p>
            <article>Identity archive</article><article>Travel notes</article><article>Work profile</article>
            <article>Family documents</article><article>Personal records</article>
        """.trimIndent(), "text/html", "UTF-8", null)
        glass = AndroidGlassPlugin(this)
        glass.load(webView)
    }
    override fun onPause() { glass.onPause(); super.onPause() }
    override fun onStop() { glass.onStop(); super.onStop() }
    override fun onDestroy() { glass.onDestroy(this); webView.destroy(); super.onDestroy() }
}
