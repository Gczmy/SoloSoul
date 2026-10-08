package com.solosoul.app

import android.graphics.Bitmap
import android.graphics.Color
import android.provider.Settings
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
import kotlin.math.abs

/** RF-121：实际 Android WebView + 生产 Card；不访问账户，不把 DOM 当作像素通过。 */
@RunWith(AndroidJUnit4::class)
class AndroidCardSurfaceInstrumentedTest {
    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()

    private fun evaluate(scenario: ActivityScenario<CardSurfaceRegressionActivity>, script: String): JSONObject {
        val done = CountDownLatch(1)
        var raw: String? = null
        scenario.onActivity { it.webView.evaluateJavascript(script) { value -> raw = value; done.countDown() } }
        assertTrue("WebView evaluation must finish", done.await(5, TimeUnit.SECONDS))
        return JSONObject(JSONArray("[${raw ?: "null"}]").getString(0))
    }

    private fun capture(scenario: ActivityScenario<CardSurfaceRegressionActivity>, theme: String, surface: String): JSONObject {
        evaluate(scenario, """JSON.stringify((() => {
            const card = document.querySelector('[data-ui-card="$surface"]');
            window.scrollTo({ top: scrollY + card.getBoundingClientRect().top - 24, behavior: 'instant' });
            return {scrolled: true};
        })())""")
        instrumentation.uiAutomation.waitForIdle(350, 3000)
        val point = evaluate(scenario, """JSON.stringify((() => {
            const card = document.querySelector('[data-ui-card="$surface"]');
            const rect = card.getBoundingClientRect();
            return {x: rect.left + 16, y: rect.top + 12, viewportWidth: innerWidth,
                background: getComputedStyle(card).backgroundColor};
        })())""")
        var ratio = 0.0
        val location = IntArray(2)
        scenario.onActivity {
            ratio = it.webView.width / point.getDouble("viewportWidth")
            it.webView.getLocationOnScreen(location)
        }
        val bitmap = requireNotNull(instrumentation.uiAutomation.takeScreenshot())
        try {
            File(instrumentation.targetContext.getExternalFilesDir(null), "rf121-card-$theme-$surface.png")
                .outputStream().use { bitmap.compress(Bitmap.CompressFormat.PNG, 100, it) }
            val x = location[0] + (point.getDouble("x") * ratio).toInt()
            val y = location[1] + (point.getDouble("y") * ratio).toInt()
            assertTrue("Card sample must lie inside actual native screenshot", x in 0 until bitmap.width && y in 0 until bitmap.height)
            val expected = Regex("""rgb\((\d+),\s*(\d+),\s*(\d+)\)""").matchEntire(point.getString("background"))
                ?: error("Card baseline must be opaque RGB")
            val pixel = bitmap.getPixel(x, y)
            val channels = listOf(Color.red(pixel), Color.green(pixel), Color.blue(pixel))
            channels.forEachIndexed { index, value ->
                assertTrue("Native pixel must match real CSS surface: $point; channels=$channels",
                    abs(value - expected.groupValues[index + 1].toInt()) <= 6)
            }
            return point.put("pixelX", x).put("pixelY", y).put("pixelRgb", JSONArray(channels))
        } finally { bitmap.recycle() }
    }

    @Test fun lightAndDarkCardsRenderWithoutOverflow() {
        val context = instrumentation.targetContext
        assertEquals("This lane validates default contrast only", 0,
            Settings.Secure.getInt(context.contentResolver, "high_text_contrast_enabled", 0))
        assertTrue("This lane preserves normal system animations",
            Settings.Global.getFloat(context.contentResolver, Settings.Global.ANIMATOR_DURATION_SCALE, 1f) > 0f)
        val records = JSONArray()
        ActivityScenario.launch(CardSurfaceRegressionActivity::class.java).use { scenario ->
            var ready = false
            val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(15)
            while (!ready && System.nanoTime() < deadline) {
                scenario.onActivity { ready = it.ready && it.webView.hasWindowFocus() }
                if (!ready) Thread.sleep(50)
            }
            assertTrue("Fixture must load and own native window focus", ready)
            for (theme in listOf("light", "dark")) {
                scenario.onActivity {
                    assertTrue(it.reports.isEmpty())
                    // 中性正文 Card 的默认表面；不伪装成 Android 原生弹窗材质或辅助功能矩阵。
                    it.webView.evaluateJavascript("window.__runCardSample('$theme', {material:'solid',reduceMotion:false,highContrast:false}, 'android')", null)
                }
                var raw: String? = null
                val reportDeadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(6)
                while (raw == null && System.nanoTime() < reportDeadline) {
                    scenario.onActivity { raw = it.reports.poll() }
                    if (raw == null) Thread.sleep(30)
                }
                val report = JSONObject(requireNotNull(raw) { "Actual paint-frame report missing" })
                assertFalse("Paint timeout cannot pass", report.has("error"))
                assertEquals(theme, report.getString("theme"))
                assertFalse(report.getBoolean("pageOverflow"))
                val cards = report.getJSONArray("cards")
                assertEquals(2, cards.length())
                for (index in 0 until cards.length()) {
                    val card = cards.getJSONObject(index)
                    assertTrue(card.getBoolean("visible"))
                    assertFalse(card.getBoolean("overflow"))
                    assertEquals(card.getString("expectedBackground"), card.getString("background"))
                    assertNotEquals(card.getString("background"), card.getString("color"))
                    assertEquals(if (index == 0) "default" else "floating", card.getString("surface"))
                }
                val pixels = JSONArray()
                for (surface in listOf("default", "floating")) pixels.put(capture(scenario, theme, surface))
                records.put(report.put("nativePixels", pixels))
            }
        }
        File(context.getExternalFilesDir(null), "rf121-android-card-surfaces.json").writeText(records.toString(2))
    }

    /** FE2：生产 WebGL 渲染器与 CSS 的独立补证，不代表完整 AndroidHome 或实体 GPU 性能。 */
    @Test fun sharedThemeLiquidRendersAndRecoversWithoutContinuousDrawing() {
        val records = JSONArray()
        ActivityScenario.launch(CardSurfaceRegressionActivity::class.java).use { scenario ->
            var loaded = false
            val loadDeadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(15)
            while (!loaded && System.nanoTime() < loadDeadline) {
                scenario.onActivity { loaded = it.ready && it.webView.hasWindowFocus() }
                if (!loaded) Thread.sleep(50)
            }
            assertTrue("Native fixture must load with window focus", loaded)
            fun state() = evaluate(scenario, "JSON.stringify(window.__readLiquidProbe())")
            fun awaitReady(expected: Boolean): JSONObject {
                val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(6)
                var value = state()
                while (value.getBoolean("ready") != expected && System.nanoTime() < deadline) {
                    Thread.sleep(50)
                    value = state()
                }
                assertEquals("Actual context availability must change", expected, value.getBoolean("ready"))
                return value
            }
            for ((theme, accent) in listOf("light" to "#112233", "dark" to "#ffee00")) {
                evaluate(scenario, "JSON.stringify((() => {window.__startLiquidProbe('$theme','$accent'); return {started:true}})())")
                awaitReady(true)
                val paintDeadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(6)
                while (state().getInt("paintFrames") < 8 && System.nanoTime() < paintDeadline) Thread.sleep(50)
                assertEquals("Current DOM must finish real paint frames", 8, state().getInt("paintFrames"))
                instrumentation.uiAutomation.waitForIdle(350, 3000)
                val first = state()
                assertFalse("Hidden WebView cannot prove rendering", first.getBoolean("hidden"))
                assertTrue("Artwork must not cover copy", first.getDouble("textRight") <= first.getDouble("canvasLeft") + 1)
                assertEquals("Text remains opaque", "1", first.getString("textOpacity"))
                assertTrue(first.getString("text").contains("42"))
                assertEquals("Ready canvas replaces CSS fallback", "none", first.getString("fallbackDisplay"))
                assertTrue(first.getInt("draws") > 0)
                val gpu = first.getJSONObject("gpu")
                assertEquals("Actual GL draw has no error", 0, gpu.getInt("error"))
                assertEquals(255, gpu.getJSONArray("pixel").getInt(3))
                val colors = first.getJSONObject("colors")
                assertEquals(accent, colors.getString("--accent-primary"))
                for ((uniform, token) in listOf("container" to "--android-liquid-base", "accent" to "--accent-primary",
                    "secondary" to "--md-secondary-container", "tertiary" to "--md-tertiary-container")) {
                    val hex = colors.getString(token).removePrefix("#")
                    val actual = gpu.getJSONObject("uniforms").getJSONArray(uniform)
                    for (index in 0..2) assertEquals("GPU uniform must match applied shared theme: $uniform",
                        hex.substring(index * 2, index * 2 + 2).toInt(16) / 255.0, actual.getDouble(index), 0.00001)
                }
                val location = IntArray(2)
                var scale = 0.0
                scenario.onActivity {
                    it.webView.getLocationOnScreen(location)
                    scale = it.webView.width / first.getDouble("viewportWidth")
                }
                val bitmap = requireNotNull(instrumentation.uiAutomation.takeScreenshot())
                try {
                    File(instrumentation.targetContext.getExternalFilesDir(null), "fe2-liquid-$theme.png")
                        .outputStream().use { bitmap.compress(Bitmap.CompressFormat.PNG, 100, it) }
                    val x = location[0] + ((first.getDouble("x") + 12) * scale).toInt()
                    val y = location[1] + ((first.getDouble("y") + 12) * scale).toInt()
                    assertTrue(x in 0 until bitmap.width && y in 0 until bitmap.height)
                    val expected = Regex("""rgb\((\d+),\s*(\d+),\s*(\d+)\)""").matchEntire(first.getString("background"))
                        ?: error("Copy surface must be opaque RGB")
                    val pixel = bitmap.getPixel(x, y)
                    val channels = listOf(Color.red(pixel), Color.green(pixel), Color.blue(pixel))
                    channels.forEachIndexed { index, value ->
                        assertTrue("Copy region pixel must remain the solid theme surface: theme=$theme; actual=$channels; expected=${expected.value}; point=($x,$y)", abs(value - expected.groupValues[index + 1].toInt()) <= 6)
                    }
                    first.put("nativeCopyPixel", JSONArray(channels))
                } finally { bitmap.recycle() }
                evaluate(scenario, "JSON.stringify((() => {window.__pokeReducedLiquidProbe(); return {poked:true}})())")
                Thread.sleep(500)
                assertEquals("Static/reduced artwork must not keep drawing", first.getInt("draws"), state().getInt("draws"))
                evaluate(scenario, "JSON.stringify((() => {window.__loseLiquidProbeContext(true); return {lost:true}})())")
                assertNotEquals("CSS fallback must be visible on actual context loss", "none", awaitReady(false).getString("fallbackDisplay"))
                evaluate(scenario, "JSON.stringify((() => {window.__loseLiquidProbeContext(false); return {restored:true}})())")
                val restored = awaitReady(true)
                assertTrue(restored.getInt("draws") > first.getInt("draws"))
                assertEquals(0, restored.getJSONObject("gpu").getInt("error"))
                records.put(first.put("theme", theme).put("restoredDraws", restored.getInt("draws")))
            }
        }
        File(instrumentation.targetContext.getExternalFilesDir(null), "fe2-android-liquid.json").writeText(records.toString(2))
    }
}
