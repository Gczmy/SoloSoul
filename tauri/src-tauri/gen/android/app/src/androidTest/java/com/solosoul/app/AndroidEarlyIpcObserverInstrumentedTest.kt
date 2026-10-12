package com.solosoul.app

import android.content.pm.ApplicationInfo
import android.view.View
import android.view.ViewGroup
import android.webkit.WebView
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

/** RF-312: reuse the complete production-home method and observe only metadata.
 * The external driver must own/backup/restore the complete target private root.
 */
@RunWith(AndroidJUnit4::class)
class AndroidEarlyIpcObserverInstrumentedTest {
    private fun webView(view: View): WebView? {
        if (view is WebView) return view
        if (view is ViewGroup) for (i in 0 until view.childCount) {
            webView(view.getChildAt(i))?.let { return it }
        }
        return null
    }

    @Test fun productionHomeWithEarlyIpcTransportObserver() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val args = InstrumentationRegistry.getArguments()
        assertEquals("true", args.getString("privateDataBackedUp"))
        val runId = requireNotNull(args.getString("observerRunId"))
        require(Regex("[a-f0-9]{32}").matches(runId))
        val tag = requireNotNull(args.getString("evidenceTag"))
        require(Regex("fe2-home-[a-f0-9]{32}").matches(tag))
        val context = instrumentation.targetContext
        val monitor = instrumentation.addMonitor(MainActivity::class.java.name, null, false)
        val proof = JSONObject().put("acceptedEarlyTransportObservation", false)
            .put("processId", android.os.Process.myPid()).put("uid", android.os.Process.myUid())
            .put("targetPackage", context.packageName)
            .put("targetDebuggable", context.applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE != 0)
            .put("scope", "whole production-home regression plus early IPC transport sends; not fixed-Vault performance")
        try {
            // No alternate bootstrap, auth state, time budget or reduced assertions.
            AndroidHomeInstrumentedTest().realHomePreservesGlassAndCopyAcrossLockUnlock()
            val activity = requireNotNull(monitor.lastActivity) as MainActivity
            val done = CountDownLatch(1)
            var raw: String? = null
            instrumentation.runOnMainSync {
                val view = requireNotNull(webView(activity.window.decorView))
                view.evaluateJavascript("""(() => {
                  const observer=window.__SOLOSOUL_ANDROID_NATIVE_PERF__;
                  const descriptor=Object.getOwnPropertyDescriptor(window.__TAURI_INTERNALS__,'invoke');
                  return JSON.stringify({present:!!observer,platform:document.documentElement.dataset.platform,
                    mainFrame:window.top===window,frames:document.querySelectorAll('iframe,frame').length,
                    invokeReadonly:descriptor?.writable===false&&descriptor?.configurable===false,
                    observer:observer?observer.bind('$runId'):null});
                })()""") { value -> raw = value; done.countDown() }
            }
            assertTrue("Actual production WebView must respond", done.await(3, TimeUnit.SECONDS))
            val parsed = JSONArray("[${raw ?: "null"}]").opt(0)
            val observed = if (parsed is String && parsed.startsWith("{")) JSONObject(parsed) else JSONObject()
            proof.put("document", observed)
            assertTrue("Early observer requires its non-default Android build", observed.optBoolean("present"))
            assertEquals("android", observed.getString("platform"))
            assertTrue(observed.getBoolean("mainFrame")); assertEquals(0, observed.getInt("frames"))
            assertTrue("Tauri readonly invoke must remain unchanged", observed.getBoolean("invokeReadonly"))
            val observer = observed.getJSONObject("observer")
            assertEquals("android-native-ipc-transport-observer", observer.getString("scope"))
            assertEquals(runId, observer.getString("runId")); assertTrue(observer.getBoolean("valid"))
            assertEquals(0, observer.getJSONArray("invalidReasons").length())
            val events = observer.getJSONArray("commands")
            assertTrue(events.length() > 0); assertEquals(events.length(), observer.getInt("observedCount"))
            assertTrue(observer.getDouble("installedAtMs") <= events.getJSONObject(0).getDouble("atMs"))
            var sawLogin = false
            for (index in 0 until events.length()) {
                val event = events.getJSONObject(index)
                assertEquals(setOf("command", "transport", "atMs"), event.keys().asSequence().toSet())
                assertTrue(event.getString("transport") in setOf("android-java-bridge", "android-channel-fetch"))
                assertTrue(Regex("[A-Za-z0-9_.:|/-]{1,128}").matches(event.getString("command")))
                if (event.getString("command") == "login") sawLogin = true
                assertTrue(event.getDouble("atMs") <= observer.getDouble("atMs"))
            }
            assertTrue("Actual production password unlock must be counted", sawLogin)
            proof.put("acceptedEarlyTransportObservation", true)
        } catch (error: Throwable) {
            proof.put("error", error.javaClass.name + ": " + error.message)
            throw error
        } finally {
            instrumentation.removeMonitor(monitor)
            // The original method may reject before it creates its owned evidence
            // directory. Do not mask that failure or create the directory first.
            val out = File(requireNotNull(context.getExternalFilesDir(null)), tag)
            if (out.isDirectory) File(out, "observer.json").writeText(proof.toString(2))
        }
    }
}
