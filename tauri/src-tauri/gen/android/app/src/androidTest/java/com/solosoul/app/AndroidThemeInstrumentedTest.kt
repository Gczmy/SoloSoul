package com.solosoul.app

import android.app.UiModeManager
import android.content.Context
import android.content.res.Configuration
import android.graphics.Bitmap
import android.graphics.Color
import android.os.Build
import android.util.TypedValue
import android.util.Base64
import android.view.View
import android.view.ViewGroup
import android.view.ViewTreeObserver
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
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.math.abs

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
        var native = JSONObject()
        scenario.onActivity { activity ->
            val controller = requireNotNull(WindowCompat.getInsetsController(activity.window, activity.window.decorView))
            statusBarLight = controller.isAppearanceLightStatusBars
            navigationBarLight = controller.isAppearanceLightNavigationBars
            val view = webView(activity.window.decorView)
            native = nativeThemeState(activity, view)
            // 冷启动时 WebView 尚未挂树；由有界轮询等待，不能在主线程断言导致进程崩溃。
            if (view == null) { done.countDown(); return@onActivity }
            view.evaluateJavascript(
                """JSON.stringify({
                    platform: document.documentElement.dataset.platform || '',
                    theme: document.documentElement.dataset.theme || '',
                    darkMedia: matchMedia('(prefers-color-scheme: dark)').matches,
                    colorScheme: getComputedStyle(document.documentElement).colorScheme,
                    colorSchemeMeta: document.querySelector('meta[name="color-scheme"]')?.content || '',
                    userAgent: navigator.userAgent.slice(0, 384),
                    mounted: (document.getElementById('root')?.childElementCount || 0) > 0,
                    startupGone: !document.getElementById('startup-screen'),
                    background: getComputedStyle(document.documentElement).getPropertyValue('--bg-base').trim(),
                    surface: getComputedStyle(document.documentElement).getPropertyValue('--bg-elevated').trim(),
                    foreground: getComputedStyle(document.documentElement).getPropertyValue('--text-primary').trim(),
                    accent: getComputedStyle(document.documentElement).getPropertyValue('--accent-primary').trim(),
                    materialContainer: getComputedStyle(document.documentElement).getPropertyValue('--md-primary-container').trim(),
                    materialInk: getComputedStyle(document.documentElement).getPropertyValue('--md-on-primary').trim(),
                    render: (() => {
                        const card = document.querySelector('[class*="bootstrapCard"]');
                        const rect = card?.getBoundingClientRect();
                        const points = rect ? [{x: rect.left + 20, y: rect.top + 20},
                            {x: rect.right - 20, y: rect.top + 20}].map(point => ({...point,
                                hitsCard: document.elementFromPoint(point.x, point.y) === card})) : [];
                        return {path: location.pathname, viewportWidth: innerWidth, viewportHeight: innerHeight,
                            devicePixelRatio, viewportScale: visualViewport?.scale || 1,
                            viewportOffsetX: visualViewport?.offsetLeft || 0, viewportOffsetY: visualViewport?.offsetTop || 0,
                            bodyBackground: getComputedStyle(document.body).backgroundColor,
                            cardFound: !!card, cardBackground: card ? getComputedStyle(card).backgroundColor : '',
                            points};
                    })()
                })""".trimIndent()
            ) { value -> raw = value; done.countDown() }
        }
        assertTrue("WebView evaluation must finish", done.await(5, TimeUnit.SECONDS))
        val value = JSONArray("[${raw ?: "null"}]").opt(0)
        // 页面未加载时 evaluateJavascript 可返回 null；不放宽最终主题/挂载断言。
        val result = if (value is String && value.startsWith("{")) JSONObject(value) else JSONObject()
        return result.put("statusBarLight", statusBarLight).put("navigationBarLight", navigationBarLight)
            .put("native", native)
    }

    /** RF-1124：只观察真实配置/主题/provider，不修改 WebView 或主题决策。 */
    @Suppress("DEPRECATION")
    private fun nativeThemeState(activity: MainActivity, view: WebView?): JSONObject {
        fun lightTheme(attribute: Int): Any {
            val value = TypedValue()
            return if (attribute != 0 && activity.theme.resolveAttribute(attribute, value, true) &&
                value.type == TypedValue.TYPE_INT_BOOLEAN) value.data != 0 else JSONObject.NULL
        }
        val provider = WebView.getCurrentWebViewPackage()
        val manager = activity.getSystemService(Context.UI_MODE_SERVICE) as UiModeManager
        return JSONObject()
            .put("api", Build.VERSION.SDK_INT)
            .put("nightMode", manager.nightMode)
            .put("activityNight", activity.resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK)
            .put("applicationNight", activity.applicationContext.resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK)
            .put("systemResourcesNight", android.content.res.Resources.getSystem().configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK)
            .put("androidLightTheme", lightTheme(activity.resources.getIdentifier("isLightTheme", "attr", "android")))
            .put("appCompatLightTheme", lightTheme(androidx.appcompat.R.attr.isLightTheme))
            .put("providerPackage", provider?.packageName ?: JSONObject.NULL)
            .put("providerVersion", provider?.versionName ?: JSONObject.NULL)
            .put("forceDark", if (Build.VERSION.SDK_INT >= 29 && view != null) view.settings.forceDark else JSONObject.NULL)
            .put("algorithmicDarkeningAllowed", if (Build.VERSION.SDK_INT >= 33 && view != null)
                view.settings.isAlgorithmicDarkeningAllowed else JSONObject.NULL)
            .put("processId", android.os.Process.myPid())
            .put("webViewIdentity", if (view == null) JSONObject.NULL else System.identityHashCode(view))
            .put("attached", view?.isAttachedToWindow ?: false)
            .put("shown", view?.isShown ?: false)
            .put("windowFocused", view?.hasWindowFocus() ?: false)
            .put("hardwareAccelerated", view?.isHardwareAccelerated ?: false)
    }

    /** RF-1125：同一 WebView 的视觉提交后实际绘制，再验证系统合成像素。 */
    private fun captureThemeFrame(scenario: ActivityScenario<MainActivity>, accepted: JSONObject,
        label: String, record: JSONObject) {
        val frame = JSONObject().put("visualCallback", false).put("nextDrawCompleted", false)
            .put("pixelMatched", false)
        record.put("renderCapture", frame)
        val painted = CountDownLatch(1)
        val active = AtomicBoolean(true)
        val postedDraw = AtomicBoolean(false)
        var boundView: WebView? = null
        var observer: ViewTreeObserver? = null
        var listener: ViewTreeObserver.OnDrawListener? = null
        try {
            scenario.onActivity { activity ->
                val view = requireNotNull(webView(activity.window.decorView))
                boundView = view
                assertTrue("Capture needs the attached, visible, focused real WebView",
                    view.isAttachedToWindow && view.isShown && view.hasWindowFocus())
                assertEquals(accepted.getJSONObject("native").getInt("webViewIdentity"), System.identityHashCode(view))
                val request = System.nanoTime()
                frame.put("requestId", request).put("webViewIdentity", System.identityHashCode(view))
                view.postVisualStateCallback(request, object : WebView.VisualStateCallback() {
                    override fun onComplete(requestId: Long) {
                        if (!active.get()) return
                        frame.put("callbackRequestId", requestId).put("visualCallback", true).put("callbackAtNs", System.nanoTime())
                        if (request != requestId) { painted.countDown(); return }
                        val tree = view.viewTreeObserver
                        observer = tree
                        val drawn = object : ViewTreeObserver.OnDrawListener {
                            override fun onDraw() {
                                if (!active.get() || !postedDraw.compareAndSet(false, true)) return
                                frame.put("drawAtNs", System.nanoTime())
                                // onDraw 内不能移除监听器；主线程下一任务确认绘制已返回。
                                view.post {
                                    if (tree.isAlive) tree.removeOnDrawListener(this)
                                    if (active.get()) {
                                        frame.put("nextDrawCompleted", true)
                                        painted.countDown()
                                    }
                                }
                            }
                        }
                        listener = drawn
                        tree.addOnDrawListener(drawn)
                        view.invalidate()
                    }
                })
            }
            assertTrue("WebView must draw the requested visual state", painted.await(10, TimeUnit.SECONDS))
            assertEquals(frame.getLong("requestId"), frame.getLong("callbackRequestId"))
            assertTrue("The matching visual callback must be followed by a completed real draw", frame.getBoolean("nextDrawCompleted"))
            instrumentation.uiAutomation.waitForIdle(350, 3000)
            val fresh = state(scenario)
            frame.put("stateAtCapture", fresh)
            for (key in listOf("platform", "theme", "background", "surface", "foreground", "accent", "materialInk",
                "darkMedia", "statusBarLight", "navigationBarLight", "mounted", "startupGone")) {
                assertEquals("Theme must stay bound through actual draw: $key", accepted.get(key), fresh.get(key))
            }
            val render = fresh.getJSONObject("render")
            assertEquals("Theme pixel fixture must stay on the real bootstrap page", "/bootstrap", render.getString("path"))
            assertTrue("Real production bootstrap card must exist", render.getBoolean("cardFound"))
            assertEquals(1.0, render.getDouble("viewportScale"), 0.001)
            assertEquals(0.0, render.getDouble("viewportOffsetX"), 0.001)
            assertEquals(0.0, render.getDouble("viewportOffsetY"), 0.001)
            val expectedColor = Color.parseColor(accepted.getString("surface"))
            val rgb = Regex("""rgb\((\d+),\s*(\d+),\s*(\d+)\)""").matchEntire(render.getString("cardBackground"))
                ?: error("The real bootstrap card surface must be opaque RGB: $render")
            val expected = listOf(Color.red(expectedColor), Color.green(expectedColor), Color.blue(expectedColor))
            assertEquals("Rendered card CSS must actually use the accepted theme surface", expected,
                (1..3).map { rgb.groupValues[it].toInt() })
            val location = IntArray(2)
            var ratio = 0.0
            scenario.onActivity { activity ->
                val view = requireNotNull(webView(activity.window.decorView))
                assertSame("No replacement WebView may reuse a capture", boundView, view)
                assertEquals(fresh.getJSONObject("native").getInt("webViewIdentity"), System.identityHashCode(view))
                assertTrue(view.isAttachedToWindow && view.isShown && view.hasWindowFocus())
                view.getLocationOnScreen(location)
                ratio = view.width / render.getDouble("viewportWidth")
                assertTrue("Actual WebView viewport must have concrete geometry", ratio > 0 && view.height > 0)
                frame.put("screenX", location[0]).put("screenY", location[1])
                    .put("webViewWidth", view.width).put("webViewHeight", view.height).put("cssToPixelRatio", ratio)
            }
            val bitmap = requireNotNull(instrumentation.uiAutomation.takeScreenshot())
            try {
                frame.put("captureAtNs", System.nanoTime()).put("imageWidth", bitmap.width).put("imageHeight", bitmap.height)
                val run = InstrumentationRegistry.getArguments().getString("themeDiagnosticRun")
                val name = if (run == null) "rf201-$label.png" else "rf201-$label-$run.png"
                File(instrumentation.targetContext.getExternalFilesDir(null), name)
                    .outputStream().use { assertTrue("The actual screenshot must be encoded", bitmap.compress(Bitmap.CompressFormat.PNG, 100, it)) }
                val points = render.getJSONArray("points")
                assertEquals(2, points.length())
                val pixels = JSONArray()
                frame.put("pixels", pixels)
                var matched = true
                for (index in 0 until points.length()) {
                    val point = points.getJSONObject(index)
                    assertTrue("Pixel point must hit uncovered card padding", point.getBoolean("hitsCard"))
                    val x = location[0] + (point.getDouble("x") * ratio).toInt()
                    val y = location[1] + (point.getDouble("y") * ratio).toInt()
                    assertTrue("Pixel must be inside the actual native image", x in 0 until bitmap.width && y in 0 until bitmap.height)
                    val value = bitmap.getPixel(x, y)
                    val actual = listOf(Color.red(value), Color.green(value), Color.blue(value))
                    val pass = actual.zip(expected).all { (a, e) -> abs(a - e) <= 6 }
                    matched = matched && pass
                    pixels.put(JSONObject().put("x", x).put("y", y).put("actualRgb", JSONArray(actual))
                        .put("expectedRgb", JSONArray(expected)).put("matched", pass))
                }
                frame.put("pixelMatched", matched)
                assertTrue("Composed pixels must match the real accepted card CSS: $frame", matched)
            } finally { bitmap.recycle() }
        } finally {
            active.set(false)
            scenario.onActivity {
                val tree = observer
                val drawn = listener
                if (tree?.isAlive == true && drawn != null) tree.removeOnDrawListener(drawn)
            }
        }
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
                "foreground" to "#111111", "accent" to "#112233")
            else mapOf("background" to "#1a211d", "surface" to "#242d28",
                "foreground" to "#ffffff", "accent" to "#112233")
        } else {
            if (theme == "light") mapOf("background" to "#fafaf6", "surface" to "#fdfcf9",
                "foreground" to "#111111", "accent" to "#5b7c99")
            else mapOf("background" to "#1f1c18", "surface" to "#2a2620",
                "foreground" to "#ffffff", "accent" to "#7a9ab5")
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
                val record = JSONObject(last.toString()).put("stage", label)
                records.put(record)
                captureThemeFrame(scenario, last, label, record)
                return
            }
            Thread.sleep(100)
        } while (System.nanoTime() < deadline)
        records.put(JSONObject(last.toString()).put("stage", label).put("result", "timeout"))
        fail("$label: theme=$theme, media=$darkMedia required; actual=$last")
    }

    private fun verify(preset: String, customSchemes: Boolean = false) {
        val context = instrumentation.targetContext
        val diagnosticRun = InstrumentationRegistry.getArguments().getString("themeDiagnosticRun")
        require(diagnosticRun == null || Regex("[0-9a-f]{32}").matches(diagnosticRun))
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
            if (diagnosticRun == null) {
                File(context.getExternalFilesDir(null), "rf201-${if (customSchemes) "custom-schemes" else preset}.json").writeText(records.toString(2))
            }
            if (originalPrefs == null) prefs.delete() else prefs.writeBytes(originalPrefs)
            setNight(originalNight)
            if (diagnosticRun != null) {
                File(context.getExternalFilesDir(null), "rf1124-$diagnosticRun.json").writeText(
                    JSONObject().put("schema", 1).put("runId", diagnosticRun).put("preset", preset)
                        .put("records", records).toString(2)
                )
            }
        }
    }

    /** 仅修复专用设备上由驱动 adb push 产生的 shell 所有者证据文件。 */
    @Test fun restoreOwnedThemeEvidence() {
        val args = InstrumentationRegistry.getArguments()
        require(args.getString("privateDataBackedUp") == "true")
        val run = requireNotNull(args.getString("themeDiagnosticRun"))
        require(Regex("[0-9a-f]{32}").matches(run))
        val context = instrumentation.targetContext
        val manifest = File(context.applicationInfo.dataDir, "rf1124-restore-$run.json")
        val records = JSONObject(manifest.readText())
        val allowed = setOf("rf201-system.json", "rf201-system-light.png", "rf201-system-dark.png", "rf201-system-light-again.png")
        val names = records.keys().asSequence().toSet()
        require(names.isNotEmpty() && names.all { it in allowed })
        val folder = requireNotNull(context.getExternalFilesDir(null))
        for (name in names) {
            val file = File(folder, name)
            // 驱动只移除了明确的 shell 所有者文件；不得覆盖任何再次出现的证据。
            require(!file.exists())
            file.writeBytes(Base64.decode(records.getString(name), Base64.DEFAULT))
            assertEquals(android.os.Process.myUid(), android.system.Os.stat(file.absolutePath).st_uid)
        }
        assertTrue(manifest.delete())
    }

    @Test fun systemThemeFollowsLightDarkLight() = verify("system")
    @Test fun explicitLightSurvivesSystemChanges() = verify("light")
    @Test fun explicitDarkSurvivesSystemChanges() = verify("dark")
    @Test fun savedSchemesAndCustomAccentSurviveColdStartup() = verify("system", customSchemes = true)
}
