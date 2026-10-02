package com.solosoul.app

import android.os.Handler
import android.os.Looper
import android.view.Choreographer
import android.view.View
import android.view.ViewGroup
import android.webkit.WebView
import androidx.test.core.app.ActivityScenario
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.espresso.Espresso.onView
import androidx.test.espresso.action.ViewActions.click
import androidx.test.espresso.matcher.ViewMatchers.withText
import org.hamcrest.Matchers.anyOf
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.io.FilterInputStream
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.atomic.AtomicBoolean

@RunWith(AndroidJUnit4::class)
class ResourcePreparationInstrumentedTest {
  private val context = InstrumentationRegistry.getInstrumentation().targetContext

  private fun webView(root: View): WebView? {
    if (root is WebView) return root
    if (root is ViewGroup) for (index in 0 until root.childCount) {
      webView(root.getChildAt(index))?.let { return it }
    }
    return null
  }

  private fun js(scenario: ActivityScenario<MainActivity>, expression: String): JSONObject {
    val done = CountDownLatch(1)
    var reply: String? = null
    scenario.onActivity { activity ->
      val view = webView(activity.window.decorView)
      if (view == null) done.countDown() else view.evaluateJavascript(expression) { reply = it; done.countDown() }
    }
    assertTrue("JS 必须能在准备期间响应", done.await(3, TimeUnit.SECONDS))
    val parsed = JSONArray("[${reply ?: "null"}]").opt(0)
    return if (parsed is String && parsed.startsWith("{")) JSONObject(parsed) else JSONObject()
  }

  private fun waitFor(scenario: ActivityScenario<MainActivity>, script: String, predicate: (JSONObject) -> Boolean): JSONObject {
    val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(30)
    var last = JSONObject()
    do {
      last = js(scenario, script)
      if (predicate(last)) return last
      Thread.sleep(100)
    } while (System.nanoTime() < deadline)
    throw AssertionError("等待失败: $last")
  }

  private val mounted = """JSON.stringify({mounted:(document.getElementById('root')?.childElementCount||0)>0,
    startupGone:!document.getElementById('startup-screen'),bridge:!!window.__TAURI_INTERNALS__})"""
  private val read = "JSON.stringify(window.rf207 || {})"

  private fun invokeConsumers(scenario: ActivityScenario<MainActivity>) {
    js(scenario, """(function(){
      window.rf207={guide:'pending',plugins:'pending',info:'pending'};
      window.__TAURI_INTERNALS__.invoke('guide_load_index',{}).then(x=>window.rf207.guide=x.guides.length>0?'ready':'empty',e=>{window.rf207.guide='error';window.rf207.error=String(e)});
      window.__TAURI_INTERNALS__.invoke('plugin_list_all',{}).then(x=>window.rf207.plugins=Array.isArray(x)&&x.length>0?'ready':'empty',e=>window.rf207.plugins='error');
      window.__TAURI_INTERNALS__.invoke('get_app_info',{}).then(()=>window.rf207.info='ready',()=>window.rf207.info='error');
      return JSON.stringify(window.rf207);
    })()""".trimIndent())
  }

  @Test fun slowInstallDoesNotBlockFramesAndRecreationDoesNotDuplicateIt() {
    val executor = Executors.newSingleThreadExecutor()
    val release = CountDownLatch(1)
    val started = CountDownLatch(1)
    val count = AtomicInteger()
    val copied = AtomicInteger()
    val firstRead = AtomicBoolean(true)
    val records = JSONObject()
    // 只删除专用应用的内置资源完成标记，强制真实复制；账户/偏好不变。
    File(context.dataDir, "app_resources/.bundled-resource-manifest").delete()
    AndroidResources.overrideForTest(ResourcePreparationCoordinator(executor)) {
      count.incrementAndGet()
      assertNotEquals("安装不能在主线程", Looper.getMainLooper(), Looper.myLooper())
      val json = context.assets.open("bundled-resource-manifest.json").bufferedReader()
        .use { JSONObject(it.readText()) }
      val files = json.getJSONArray("files")
      val entries = (0 until files.length()).map {
        val entry = files.getJSONObject(it)
        ResourceEntry(entry.getString("path"), entry.getLong("size"), entry.getString("sha256"))
      }
      val result = ResourceInstaller(context.dataDir).install(ResourceManifest(json.getString("version"), entries)) { path ->
        object : FilterInputStream(context.assets.open(path)) {
          override fun read(buffer: ByteArray, offset: Int, length: Int): Int {
            if (firstRead.compareAndSet(true, false)) {
              started.countDown()
              check(release.await(40, TimeUnit.SECONDS))
            }
            return super.read(buffer, offset, length)
          }
        }
      }
      copied.set(result.writtenFiles)
      assertFalse("必须执行真实复制", result.skipped)
      result
    }
    val scenario = ActivityScenario.launch(MainActivity::class.java)
    try {
      assertTrue(started.await(10, TimeUnit.SECONDS))
      val frame = CountDownLatch(1)
      Handler(Looper.getMainLooper()).post { Choreographer.getInstance().postFrameCallback { frame.countDown() } }
      assertTrue("慢安装期间仍须绘制帧", frame.await(2, TimeUnit.SECONDS))
      waitFor(scenario, mounted) { it.optBoolean("mounted") && it.optBoolean("startupGone") && it.optBoolean("bridge") }
      invokeConsumers(scenario)
      val before = waitFor(scenario, read) { it.optString("info") == "ready" }
      assertEquals("pending", before.getString("guide"))
      assertEquals("pending", before.getString("plugins"))
      records.put("before", before)
      scenario.recreate()
      waitFor(scenario, mounted) { it.optBoolean("mounted") && it.optBoolean("startupGone") && it.optBoolean("bridge") }
      assertEquals(1, count.get())
      invokeConsumers(scenario)
      val recreated = waitFor(scenario, read) { it.optString("info") == "ready" }
      assertEquals("pending", recreated.getString("guide"))
      assertEquals("pending", recreated.getString("plugins"))
      records.put("recreated", recreated)
      release.countDown()
      records.put("ready", waitFor(scenario, read) { it.optString("guide") == "ready" && it.optString("plugins") == "ready" })
      records.put("installCount", count.get()).put("mainFrameDuringInstall", true)
      records.put("writtenFiles", copied.get())
      assertTrue(copied.get() > 0)
      File(context.getExternalFilesDir(null), "rf207-slow-install.json").writeText(records.toString(2))
    // Tauri 最后一个 Activity 的关闭会退出同进程 runner；外部按用例报告后 force-stop。
    } finally { release.countDown(); executor.shutdown(); assertTrue(executor.awaitTermination(10, TimeUnit.SECONDS)) }
  }

  @Test fun errorIsVisibleAndRetryUnblocksRealConsumers() {
    val executor = Executors.newSingleThreadExecutor()
    val count = AtomicInteger()
    AndroidResources.overrideForTest(ResourcePreparationCoordinator(executor)) {
      if (count.incrementAndGet() == 1) throw java.io.IOException("专用失败注入")
      MainActivity.extractAssetsToDataDir(context.assets, context.dataDir)
    }
    val scenario = ActivityScenario.launch(MainActivity::class.java)
    try {
      waitFor(scenario, mounted) { it.optBoolean("mounted") && it.optBoolean("startupGone") && it.optBoolean("bridge") }
      invokeConsumers(scenario)
      val failed = waitFor(scenario, read) { it.optString("guide") == "error" && it.optString("plugins") == "error" && it.optString("info") == "ready" }
      assertEquals("error", failed.getString("plugins"))
      assertTrue(failed.getString("error").contains("ANDROID_RESOURCES_FAILED"))
      onView(anyOf(withText("重试"), withText("Retry"))).perform(click())
      val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(10)
      while (AndroidResources.snapshot().status != "ready" && System.nanoTime() < deadline) Thread.sleep(50)
      assertEquals("ready", AndroidResources.snapshot().status)
      invokeConsumers(scenario)
      val recovered = waitFor(scenario, read) { it.optString("guide") == "ready" && it.optString("plugins") == "ready" }
      assertEquals(2, count.get())
      File(context.getExternalFilesDir(null), "rf207-error-retry.json")
        .writeText(JSONObject().put("failed", failed).put("recovered", recovered).put("attempts", count.get()).toString(2))
    } finally { executor.shutdown(); assertTrue(executor.awaitTermination(10, TimeUnit.SECONDS)) }
  }
}
