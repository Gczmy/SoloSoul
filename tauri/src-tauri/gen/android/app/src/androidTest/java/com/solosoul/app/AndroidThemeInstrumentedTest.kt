package com.solosoul.app

import android.app.UiModeManager
import android.content.Context
import android.graphics.Bitmap
import android.view.View
import android.view.ViewGroup
import android.webkit.WebView
import androidx.core.view.WindowCompat
import androidx.test.core.app.ActivityScenario
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

/** RF-201：只在专用设备运行，使用真实客户端页面，不替换主题解析或事件。 */
@RunWith(AndroidJUnit4::class)
class AndroidThemeInstrumentedTest {
    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()

    private fun webView(view: View): WebView? {
        if (view is WebView) return view
        if (view is ViewGroup) {
            for (index in 0 until view.childCount) {
                webView(view.getChildAt(index))?.let { return it }
            }
        }
        return null
    }

    private fun state(scenario: ActivityScenario<MainActivity>): JSONObject {
        val done = CountDownLatch(1)
        var raw: String? = null
        var statusBarLight = false
        var navigationBarLight = false
        scenario.onActivity { activity ->
            val controller = requireNotNull(WindowCompat.getInsetsController(activity.window, activity.window.decorView))
            statusBarLight = controller.isAppearanceLightStatusBars
            navigationBarLight = controller.isAppearanceLightNavigationBars
            val view = webView(activity.window.decorView)
            // 冷启动时 WebView 尚未挂树；由有界轮询等待，不能在主线程断言导致进程崩溃。
            if (view == null) { done.countDown(); return@onActivity }
            view.evaluateJavascript(
                """JSON.stringify({
                    platform: document.documentElement.dataset.platform || '',
                    theme: document.documentElement.dataset.theme || '',
                    darkMedia: matchMedia('(prefers-color-scheme: dark)').matches,
                    mounted: (document.getElementById('root')?.childElementCount || 0) > 0,
                    startupGone: !document.getElementById('startup-screen'),
                    background: getComputedStyle(document.documentElement).getPropertyValue('--bg-base')
                })""".trimIndent()
            ) { value -> raw = value; done.countDown() }
        }
        assertTrue("WebView evaluation must finish", done.await(5, TimeUnit.SECONDS))
        val value = JSONArray("[${raw ?: "null"}]").opt(0)
        // 页面未加载时 evaluateJavascript 可返回 null；不放宽最终主题/挂载断言。
        val result = if (value is String && value.startsWith("{")) JSONObject(value) else JSONObject()
        return result.put("statusBarLight", statusBarLight).put("navigationBarLight", navigationBarLight)
    }

    private fun setNight(mode: String) {
        android.os.ParcelFileDescriptor.AutoCloseInputStream(
            instrumentation.uiAutomation.executeShellCommand("cmd uimode night $mode")
        ).use { it.readBytes() }
    }

    private fun waitForTheme(
        scenario: ActivityScenario<MainActivity>,
        theme: String,
        darkMedia: Boolean,
        label: String,
        records: JSONArray
    ) {
        val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(45)
        var last = JSONObject()
        do {
            last = state(scenario)
            if (last.optString("platform") == "android" && last.optBoolean("mounted") &&
                last.optBoolean("startupGone") &&
                last.optString("theme") == theme && last.optBoolean("darkMedia") == darkMedia &&
                last.optBoolean("statusBarLight") == (theme == "light") &&
                last.optBoolean("navigationBarLight") == (theme == "light")
            ) {
                assertTrue("Actual scheme must be applied", last.getString("background").isNotBlank())
                records.put(JSONObject(last.toString()).put("stage", label))
                // 等待启动淡出/系统栏切色完成绘制，再截图；主题断言仍在上方独立执行。
                instrumentation.uiAutomation.waitForIdle(500, 3000)
                val bitmap = instrumentation.uiAutomation.takeScreenshot()
                File(instrumentation.targetContext.getExternalFilesDir(null), "rf201-$label.png")
                    .outputStream().use { bitmap.compress(Bitmap.CompressFormat.PNG, 100, it) }
                bitmap.recycle()
                return
            }
            Thread.sleep(100)
        } while (System.nanoTime() < deadline)
        fail("$label: theme=$theme, media=$darkMedia required; actual=$last")
    }

    private fun verify(preset: String) {
        val context = instrumentation.targetContext
        val manager = context.getSystemService(Context.UI_MODE_SERVICE) as UiModeManager
        val originalNight = when (manager.nightMode) {
            UiModeManager.MODE_NIGHT_NO -> "no"
            UiModeManager.MODE_NIGHT_YES -> "yes"
            UiModeManager.MODE_NIGHT_AUTO -> "auto"
            else -> throw IllegalStateException("Test device must use a known initial night mode")
        }
        // 使用客户端真实登录前偏好格式；保留并恢复原字节，不创建账户或改 Vault。
        // Tauri PathPlugin.getDataDir 返回 application dataDir，而不是 filesDir。
        val prefs = File(context.applicationInfo.dataDir, "ui_preferences.json")
        val originalPrefs = if (prefs.exists()) prefs.readBytes() else null
        val records = JSONArray()
        try {
            prefs.writeText(JSONObject().put("theme", preset).put("accentColor", "ocean")
                .put("language", "en-US").put("hasSeenOnboarding", true)
                .put("notificationPermissionRequested", true).toString())
            setNight("no")
            // Tauri 关闭最后一个 Activity 会退出整个原生进程，连同同进程 JUnit runner。
            // 每个用例由外部驱动独立运行并 force-stop；保留 Activity 到结果报告完成。
            val scenario = ActivityScenario.launch(MainActivity::class.java)
            waitForTheme(scenario, if (preset == "system") "light" else preset, false, "$preset-light", records)
            setNight("yes")
            waitForTheme(scenario, if (preset == "system") "dark" else preset, true, "$preset-dark", records)
            setNight("no")
            waitForTheme(scenario, if (preset == "system") "light" else preset, false, "$preset-light-again", records)
        } finally {
            File(context.getExternalFilesDir(null), "rf201-$preset.json").writeText(records.toString(2))
            if (originalPrefs == null) prefs.delete() else prefs.writeBytes(originalPrefs)
            setNight(originalNight)
        }
    }

    @Test fun systemThemeFollowsLightDarkLight() = verify("system")
    @Test fun explicitLightSurvivesSystemChanges() = verify("light")
    @Test fun explicitDarkSurvivesSystemChanges() = verify("dark")
}
