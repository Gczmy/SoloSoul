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
                    background: getComputedStyle(document.documentElement).getPropertyValue('--bg-base').trim(),
                    surface: getComputedStyle(document.documentElement).getPropertyValue('--bg-elevated').trim(),
                    foreground: getComputedStyle(document.documentElement).getPropertyValue('--text-primary').trim(),
                    accent: getComputedStyle(document.documentElement).getPropertyValue('--accent-primary').trim(),
                    materialContainer: getComputedStyle(document.documentElement).getPropertyValue('--md-primary-container').trim(),
                    materialInk: getComputedStyle(document.documentElement).getPropertyValue('--md-on-primary').trim()
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
        records: JSONArray,
        customSchemes: Boolean = false
    ) {
        val expected = if (customSchemes) {
            if (theme == "light") mapOf("background" to "#f6f8fa", "surface" to "#ffffff",
                "foreground" to "#1f2328", "accent" to "#112233")
            else mapOf("background" to "#1a211d", "surface" to "#242d28",
                "foreground" to "#d6ddd8", "accent" to "#112233")
        } else {
            if (theme == "light") mapOf("background" to "#fafaf6", "surface" to "#fdfcf9",
                "foreground" to "#1f1c18", "accent" to "#5b7c99")
            else mapOf("background" to "#1f1c18", "surface" to "#2a2620",
                "foreground" to "#ddd8c8", "accent" to "#7a9ab5")
        }
        val expectedInk = if (customSchemes) "#ffffff" else "#000000"
        val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(45)
        var last = JSONObject()
        do {
            last = state(scenario)
            if (last.optString("platform") == "android" && last.optBoolean("mounted") &&
                last.optBoolean("startupGone") &&
                last.optString("theme") == theme && last.optBoolean("darkMedia") == darkMedia &&
                last.optBoolean("statusBarLight") == (theme == "light") &&
                last.optBoolean("navigationBarLight") == (theme == "light") &&
                // 同模式缓存首帧不能证明新偏好已交付，必须等到完整待测色板。
                expected.all { (key, value) -> last.optString(key) == value } &&
                last.optString("materialInk") == expectedInk
            ) {
                assertTrue("Actual scheme must be applied", last.getString("background").isNotBlank())
                // FE2-010/011：真实客户端不能再覆盖为 Android 的独立绿灰色板。
                expected.forEach { (key, value) -> assertEquals("Shared theme $key", value, last.getString(key)) }
                assertTrue("Native menu material color must be concrete hex",
                    Regex("#[0-9a-f]{6}").matches(last.getString("materialContainer")))
                assertEquals(expectedInk, last.getString("materialInk"))
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

    private fun verify(preset: String, customSchemes: Boolean = false) {
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
            val values = JSONObject().put("theme", preset).put("accentColor", if (customSchemes) "custom" else "ocean")
                .put("language", "en-US").put("hasSeenOnboarding", true)
                .put("notificationPermissionRequested", true)
                .put("defaultLightTheme", if (customSchemes) "clean-slate" else "warm-stone")
                .put("defaultDarkTheme", if (customSchemes) "forest-night" else "warm-stone-dark")
            if (customSchemes) values.put("customAccentHex", "#112233")
            prefs.writeText(values.toString())
            setNight("no")
            // Tauri 关闭最后一个 Activity 会退出整个原生进程，连同同进程 JUnit runner。
            // 每个用例由外部驱动独立运行并 force-stop；保留 Activity 到结果报告完成。
            val scenario = ActivityScenario.launch(MainActivity::class.java)
            val label = if (customSchemes) "custom-schemes" else preset
            waitForTheme(scenario, if (preset == "system") "light" else preset, false, "$label-light", records, customSchemes)
            setNight("yes")
            waitForTheme(scenario, if (preset == "system") "dark" else preset, true, "$label-dark", records, customSchemes)
            setNight("no")
            waitForTheme(scenario, if (preset == "system") "light" else preset, false, "$label-light-again", records, customSchemes)
        } finally {
            File(context.getExternalFilesDir(null), "rf201-${if (customSchemes) "custom-schemes" else preset}.json").writeText(records.toString(2))
            if (originalPrefs == null) prefs.delete() else prefs.writeBytes(originalPrefs)
            setNight(originalNight)
        }
    }

    @Test fun systemThemeFollowsLightDarkLight() = verify("system")
    @Test fun explicitLightSurvivesSystemChanges() = verify("light")
    @Test fun explicitDarkSurvivesSystemChanges() = verify("dark")
    @Test fun savedSchemesAndCustomAccentSurviveColdStartup() = verify("system", customSchemes = true)
}
