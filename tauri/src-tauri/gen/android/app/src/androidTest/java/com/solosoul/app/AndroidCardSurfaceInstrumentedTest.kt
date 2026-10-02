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
}
