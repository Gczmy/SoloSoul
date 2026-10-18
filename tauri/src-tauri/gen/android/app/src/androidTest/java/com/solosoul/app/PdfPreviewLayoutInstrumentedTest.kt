package com.solosoul.app

import android.content.Intent
import android.graphics.Bitmap
import android.graphics.Rect
import android.os.Handler
import android.os.Looper
import android.view.PixelCopy
import android.view.View
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.TextView
import androidx.test.core.app.ActivityScenario
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.security.MessageDigest
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

/** Public PDF layout correctness only; target instrumentation invalidates performance claims. */
@RunWith(AndroidJUnit4::class)
class PdfPreviewLayoutInstrumentedTest {
    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()
    private fun sha(bytes: ByteArray) = MessageDigest.getInstance("SHA-256").digest(bytes)
        .joinToString("") { "%02x".format(it) }
    private fun rect(view: View): JSONArray {
        val location = IntArray(2)
        view.getLocationOnScreen(location)
        return JSONArray(listOf(location[0], location[1], location[0] + view.width, location[1] + view.height))
    }
    private fun painted(scenario: ActivityScenario<PdfPreviewActivity>) {
        val done = CountDownLatch(1)
        scenario.onActivity { activity ->
            val decor = activity.window.decorView
            decor.postOnAnimation { decor.postOnAnimation { done.countDown() } }
            decor.invalidate()
        }
        assertTrue("Native PDF must draw within ten seconds", done.await(10, TimeUnit.SECONDS))
    }
    private fun capture(scenario: ActivityScenario<PdfPreviewActivity>, output: File, name: String): JSONObject {
        painted(scenario)
        val done = CountDownLatch(1)
        val result = JSONObject()
        var copied: Bitmap? = null
        var status = -1
        scenario.onActivity { activity ->
            val counter = activity.findViewById<TextView>(R.id.pdfPageInfo)
            val image = activity.findViewById<ImageView>(R.id.pdfPageImage)
            val visible = Rect()
            result.put("pageInfo", counter.text.toString())
                .put("counterWidth", counter.width).put("counterHeight", counter.height)
                .put("counterShown", counter.isShown).put("counterGlobalVisible", counter.getGlobalVisibleRect(visible))
                .put("counterBounds", rect(counter)).put("imageBounds", rect(image))
                .put("controlsBounds", rect(activity.findViewById(R.id.pdfControls)))
                .put("title", activity.findViewById<TextView>(R.id.pdfTitle).text.toString())
                .put("previousEnabled", activity.findViewById<ImageButton>(R.id.pdfPrevButton).isEnabled)
                .put("nextEnabled", activity.findViewById<ImageButton>(R.id.pdfNextButton).isEnabled)
                .put("imageDrawablePresent", image.drawable != null)
            val decor = activity.window.decorView
            val bitmap = Bitmap.createBitmap(decor.width, decor.height, Bitmap.Config.ARGB_8888)
            copied = bitmap
            PixelCopy.request(activity.window, bitmap, { code -> status = code; done.countDown() }, Handler(Looper.getMainLooper()))
        }
        assertTrue("Actual native window PixelCopy must finish", done.await(3, TimeUnit.SECONDS))
        assertEquals(PixelCopy.SUCCESS, status)
        val png = File(output, "$name.png")
        png.outputStream().use { requireNotNull(copied).compress(Bitmap.CompressFormat.PNG, 100, it) }
        copied?.recycle()
        result.put("screenshot", png.name).put("imageSha256", sha(png.readBytes()))
        File(output, "$name.json").writeText(result.toString(2))
        return result
    }

    @Test fun firstPageCounterIsVisibleBeforeNavigation() {
        val context = instrumentation.targetContext
        val output = File(context.cacheDir, "rf312-pdf-layout-test")
        assertFalse("Use a new public test evidence namespace", output.exists())
        assertTrue(output.mkdirs())
        val bytes = instrumentation.context.assets.open("text_only.pdf").use { it.readBytes() }
        assertEquals("ca60313e25ffa64f848d86780201a9570a0dbcb3bf733bfb5e4a65ed73f4fdfe", sha(bytes))
        val input = File(output, "input.pdf").apply { writeBytes(bytes) }
        val intent = Intent(context, PdfPreviewActivity::class.java)
            .putExtra(PdfPreviewActivity.EXTRA_PATH, input.absolutePath)
            .putExtra(PdfPreviewActivity.EXTRA_TITLE, "text_only.pdf")
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        val report = JSONObject().put("scope", "native-pdf-layout-correctness-only")
            .put("targetInstrumentationInjected", true).put("defaultReleaseAccepted", false)
            .put("performanceDistributionAccepted", false).put("publicFixtureSha256", sha(bytes))
        try {
            ActivityScenario.launch<PdfPreviewActivity>(intent).use { scenario ->
                val first = capture(scenario, output, "initial")
                scenario.onActivity { it.findViewById<ImageButton>(R.id.pdfNextButton).performClick() }
                val second = capture(scenario, output, "after-next")
                report.put("initial", first).put("afterNext", second)
                File(output, "report.json").writeText(report.toString(2))
                assertEquals("1 / 2", first.getString("pageInfo"))
                assertEquals("2 / 2", second.getString("pageInfo"))
                assertTrue(first.getBoolean("imageDrawablePresent"))
                assertFalse(first.getBoolean("previousEnabled"))
                assertTrue(first.getBoolean("nextEnabled"))
                assertTrue(second.getBoolean("previousEnabled"))
                assertFalse(second.getBoolean("nextEnabled"))
                assertTrue("Initial page counter has no visible width", first.getInt("counterWidth") > 0)
                assertTrue(first.getBoolean("counterGlobalVisible"))
                assertTrue(second.getInt("counterWidth") > 0)
            }
        } finally {
            assertTrue("Remove only the copied public PDF input", input.delete())
        }
    }
}
