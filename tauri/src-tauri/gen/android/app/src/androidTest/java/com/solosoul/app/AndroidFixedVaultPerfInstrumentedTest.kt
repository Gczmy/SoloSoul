package com.solosoul.app

import android.content.pm.ApplicationInfo
import android.graphics.Bitmap
import android.view.PixelCopy
import android.os.Handler
import android.os.Looper
import android.os.Debug
import android.os.SystemClock
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import android.webkit.WebView
import androidx.test.core.app.ActivityScenario
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicReference

/** Fixed encrypted public datasets only. This proposal is not a native result.
 * Startup is Activity launch to usable password form and two committed frames:
 * it excludes runner/process launch and includes instrumentation overhead.
 * Memory is serial sampled main-process PSS, including this test runner.
 * Media/OCR, renderer memory, physical devices and other platforms stay pending.
 */
@RunWith(AndroidJUnit4::class)
class AndroidFixedVaultPerfInstrumentedTest {
    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()
    private val phases = JSONArray()
    private val touches = JSONArray()
    private val memory = JSONArray()
    private val sampling = AtomicBoolean(false)
    private val memoryFailure = AtomicReference<Throwable?>()
    private lateinit var evidence: File
    private lateinit var runId: String
    private var expectedAccount = ""
    private lateinit var report: JSONObject
    private var clockOrigin: Double? = null

    private fun webView(view: View): WebView? {
        if (view is WebView) return view
        if (view is ViewGroup) for (i in 0 until view.childCount) webView(view.getChildAt(i))?.let { return it }
        return null
    }

    private fun js(scenario: ActivityScenario<MainActivity>, expression: String): JSONObject {
        val done = CountDownLatch(1)
        var raw: String? = null
        scenario.onActivity { activity ->
            val view=webView(activity.window.decorView)
            if(view==null)done.countDown() else view.evaluateJavascript(expression) {
                raw = it; done.countDown()
            }
        }
        assertTrue("Actual target WebView must respond", done.await(3, TimeUnit.SECONDS))
        val parsed = JSONArray("[${raw ?: "null"}]").opt(0)
        return if (parsed is String && parsed.startsWith("{")) JSONObject(parsed) else JSONObject()
    }

    private fun waitFor(scenario: ActivityScenario<MainActivity>, expression: String,
                        began: Long = SystemClock.elapsedRealtimeNanos(),
                        predicate: (JSONObject) -> Boolean): JSONObject {
        var last = JSONObject()
        while (SystemClock.elapsedRealtimeNanos() - began < TimeUnit.SECONDS.toNanos(45)) {
            last = js(scenario, expression)
            if (predicate(last)) return last
            Thread.sleep(50)
        }
        throw AssertionError("Fixed public UI checkpoint unavailable: $last")
    }

    private fun painted(scenario: ActivityScenario<MainActivity>, budgetBegan: Long? = null) {
        val committed = CountDownLatch(1)
        scenario.onActivity { activity ->
            val view = requireNotNull(webView(activity.window.decorView))
            assertTrue(view.isAttachedToWindow && view.isShown && view.hasWindowFocus())
            view.postVisualStateCallback(SystemClock.elapsedRealtimeNanos(), object : WebView.VisualStateCallback() {
                override fun onComplete(requestId: Long) {
                    view.postOnAnimation { view.postOnAnimation { committed.countDown() } }
                }
            })
        }
        val budget=if(budgetBegan==null) TimeUnit.SECONDS.toNanos(10) else
            minOf(TimeUnit.SECONDS.toNanos(10),TimeUnit.SECONDS.toNanos(45)-(SystemClock.elapsedRealtimeNanos()-budgetBegan))
        assertTrue("Target view must commit and draw two frames within the shared budget",budget>0 && committed.await(budget,TimeUnit.NANOSECONDS))
    }

    private fun touchSnapshot(scenario: ActivityScenario<MainActivity>, selector: String): JSONObject =
        js(scenario, """(() => {
          const nodes=[...document.querySelectorAll(${JSONObject.quote(selector)})];
          const node=nodes.length===1?nodes[0]:null,r=node?.getBoundingClientRect();
          const x=r?r.x+r.width/2:null,y=r?r.y+r.height/2:null;
          const hit=r?document.elementFromPoint(x,y):null;
          return JSON.stringify({nodes:nodes.length,valid:!!node&&!node.disabled&&r.width>0&&r.height>0,
            x,y,width:r?.width,height:r?.height,viewportWidth:innerWidth,viewportHeight:innerHeight,
            visualViewportHeight:visualViewport?.height,visualViewportOffsetY:visualViewport?.offsetTop,
            hitIsTarget:!!hit&&(hit===node||node.contains(hit)),hitTag:hit?.tagName,
            hitInToast:!!hit?.closest('[data-toast-container]'),toastCount:document.querySelectorAll('[data-toast-container]').length,
            notificationHeight:document.querySelector('[data-shell-notifications]')?.getBoundingClientRect().height,
            path:location.pathname,search:location.search,
            frame:window.top===window,focused:document.hasFocus()});})()""")

    private fun readyTarget(scenario: ActivityScenario<MainActivity>, selector: String, began: Long): JSONObject {
        var prior:String?=null;var stableAt=0L;var stableFrames=0;var last=JSONObject()
        val fields=listOf("nodes","valid","x","y","width","height","viewportWidth","viewportHeight",
            "visualViewportHeight","visualViewportOffsetY","toastCount","notificationHeight","path","search","frame","focused","hitIsTarget","hitInToast")
        while(SystemClock.elapsedRealtimeNanos()-began<TimeUnit.SECONDS.toNanos(45)) {
            painted(scenario,began)
            last=touchSnapshot(scenario,selector)
            var nativeMatches=false
            scenario.onActivity { activity ->
                val view=requireNotNull(webView(activity.window.decorView))
                val viewportWidth=last.optDouble("viewportWidth",0.0)
                val viewportHeight=last.optDouble("viewportHeight",0.0)
                nativeMatches=viewportWidth>0 && viewportHeight>0 && view.hasWindowFocus() &&
                    kotlin.math.abs(view.height-view.width/viewportWidth*viewportHeight)<=2
            }
            val valid=last.optBoolean("valid") && last.optBoolean("frame") && last.optBoolean("focused") &&
                last.optBoolean("hitIsTarget") && !last.optBoolean("hitInToast") && nativeMatches
            val key=JSONArray(fields.map { last.opt(it) ?: JSONObject.NULL }).toString()
            val now=SystemClock.elapsedRealtimeNanos()
            if(!valid || key!=prior) { stableAt=now;stableFrames=0 }
            prior=if(valid)key else null
            if(valid) {
                stableFrames++
                if(stableFrames>=3 && now-stableAt>=TimeUnit.MILLISECONDS.toNanos(350)) {
                    return last.put("readinessMs",(now-began)/1_000_000.0).put("stableFrames",stableFrames)
                }
            }
            Thread.sleep(50)
        }
        throw AssertionError("Native target geometry/viewport/hit must settle within original budget: $last")
    }

    private fun touch(scenario: ActivityScenario<MainActivity>, selector: String,
                      began: Long = SystemClock.elapsedRealtimeNanos()) {
        val target=readyTarget(scenario,selector,began)
        val trace=JSONObject().put("selector",selector).put("before",target).put("nativeBeforeAtNs",SystemClock.elapsedRealtimeNanos())
        touches.put(trace);report.put("touches",touches)
        report.put("lastNativeTouchTarget",target)
        // Persist after dispatch: do not add file I/O between coordinate binding
        // and the single native touch pair.
        assertTrue("Actual fixed-target precondition: $target",target.getBoolean("valid") && target.getBoolean("frame") && target.getBoolean("focused"))
        scenario.onActivity { activity ->
            val view = requireNotNull(webView(activity.window.decorView))
            assertTrue(view.hasWindowFocus())
            trace.put("viewWidthPx",view.width).put("viewHeightPx",view.height)
                .put("nativeDispatchAtNs",SystemClock.elapsedRealtimeNanos())
            val scale=view.width/target.getDouble("viewportWidth")
            val x=(target.getDouble("x")*scale).toFloat()
            val y=(target.getDouble("y")*scale).toFloat()
            assertTrue(x>0 && y>0 && x<view.width && y<view.height)
            val down=SystemClock.uptimeMillis()
            for (action in listOf(MotionEvent.ACTION_DOWN,MotionEvent.ACTION_UP)) {
                val event=MotionEvent.obtain(down,SystemClock.uptimeMillis(),action,x,y,0)
                try { assertTrue(view.dispatchTouchEvent(event)) } finally { event.recycle() }
            }
        }
        trace.put("after",touchSnapshot(scenario,selector)).put("nativeAfterAtNs",SystemClock.elapsedRealtimeNanos())
        writeReport()
    }

    private fun failureScreenshot(scenario: ActivityScenario<MainActivity>, name: String) {
        val done=CountDownLatch(1);var bitmap:Bitmap?=null;var result=-1
        scenario.onActivity { activity ->
            val decor=activity.window.decorView
            bitmap=Bitmap.createBitmap(decor.width,decor.height,Bitmap.Config.ARGB_8888)
            PixelCopy.request(activity.window,requireNotNull(bitmap),{ code -> result=code;done.countDown() },Handler(Looper.getMainLooper()))
        }
        try {
            assertTrue("Diagnostic PixelCopy must finish",done.await(5,TimeUnit.SECONDS))
            assertEquals(PixelCopy.SUCCESS,result)
            File(evidence,"failure-$name.png").outputStream().use { out -> assertTrue(requireNotNull(bitmap).compress(Bitmap.CompressFormat.PNG,100,out)) }
            report.put("failureScreenshot","failure-$name.png")
        } finally { bitmap?.recycle() }
    }

    private fun typePublic(scenario: ActivityScenario<MainActivity>, selector: String, text: String) {
        require(text=="perf-baseline-only-password" || text=="needle")
        val began=SystemClock.elapsedRealtimeNanos()
        touch(scenario,selector,began)
        val expression="""JSON.stringify({focused:document.activeElement===document.querySelector(${JSONObject.quote(selector)}),
          empty:document.querySelector(${JSONObject.quote(selector)})?.value==='',
          activeTag:document.activeElement?.tagName,documentFocused:document.hasFocus()})"""
        val focus=waitFor(scenario,expression,began) {
            report.put("lastNativeInputFocus",it);writeReport()
            assertTrue("Public target must remain empty before native typing: $it",it.optBoolean("empty"))
            it.optBoolean("focused") && it.optBoolean("documentFocused")
        }
        assertTrue(focus.getBoolean("focused") && focus.getBoolean("empty"))
        instrumentation.sendStringSync(text)
        waitFor(scenario,"JSON.stringify({filled:document.querySelector(${JSONObject.quote(selector)})?.value===${JSONObject.quote(text)}})",began) { it.optBoolean("filled") }
    }

    private val page get() = """JSON.stringify({path:location.pathname,search:location.search,
      mounted:!!document.querySelector('#root>*'),startupGone:!document.querySelector('#startup-screen'),
      platform:document.documentElement.dataset.platform,
      home:!!document.querySelector('[data-testid="android-home"]'),
      count:document.querySelector('.android-overview-copy strong')?.textContent,
      password:!!document.querySelector('[data-login-method-region="password"] input'),
      enabled:!!document.querySelector('[data-login-password-submit]')&&!document.querySelector('[data-login-password-submit]').disabled,
      singleAccount:document.querySelectorAll('[data-login-account]').length===1 && !document.querySelector('[data-login-account] select'),
      publicAccountName:document.querySelector('[data-login-account] > div > div:first-child')?.textContent==='Performance Fixture',
      publicAccountId:document.querySelector('[data-login-account] > div > div:last-child')?.textContent===${JSONObject.quote(expectedAccount)},
      cards:document.querySelectorAll('[data-testid="workspace-object-card"]').length,
      firstPublicObject:[...document.querySelectorAll('[data-testid="workspace-object-card"]')].some(e=>e.textContent.includes('Record 00000000')),
      searchResults:document.querySelectorAll('[data-ui-card][role="button"]').length,
      publicSearch:document.querySelector('input')?.value==='needle',
      errors:document.querySelectorAll('[role="alert"]').length})"""

    private fun observer(scenario: ActivityScenario<MainActivity>, bind: Boolean = false): JSONObject {
        val root = js(scenario,"""(() => {
          const o=window.__SOLOSOUL_ANDROID_NATIVE_PERF__;
          const d=Object.getOwnPropertyDescriptor(window.__TAURI_INTERNALS__,'invoke');
          return JSON.stringify({mainFrame:window.top===window,frames:document.querySelectorAll('iframe,frame').length,
            invokeReadonly:d?.writable===false&&d?.configurable===false,
            snapshot:o?o.${if(bind) "bind('$runId')" else "snapshot()"}:null});})()""")
        assertTrue(root.getBoolean("mainFrame") && root.getBoolean("invokeReadonly"))
        assertEquals(0,root.getInt("frames"))
        val observed=root.getJSONObject("snapshot")
        assertEquals("android-native-ipc-transport-observer",observed.getString("scope"))
        assertEquals(runId,observed.getString("runId")); assertTrue(observed.getBoolean("valid"))
        assertEquals(0,observed.getJSONArray("invalidReasons").length())
        val origin=observed.getDouble("timeOriginMs")
        if(clockOrigin==null)clockOrigin=origin else assertEquals(clockOrigin!!,origin,0.0)
        assertEquals(observed.getInt("observedCount"),observed.getJSONArray("commands").length())
        val commands=observed.getJSONArray("commands")
        for(i in 0 until commands.length()) {
            val command=commands.getJSONObject(i)
            assertEquals(setOf("command","transport","atMs"),command.keys().asSequence().toSet())
            assertTrue(Regex("[A-Za-z0-9_.:|/-]{1,128}").matches(command.getString("command")))
            assertTrue(command.getString("transport") in setOf("android-java-bridge","android-channel-fetch"))
            assertTrue(command.getDouble("atMs")>=observed.getDouble("installedAtMs") && command.getDouble("atMs")<=observed.getDouble("atMs"))
        }
        return observed
    }

    private fun phase(scenario: ActivityScenario<MainActivity>, name: String, began: Long,
                      before: JSONObject?, action: () -> Unit) {
        val row=JSONObject().put("name",name).put("nativeStartedAtNs",began).put("success",false)
        phases.put(row)
        try {
            action();painted(scenario)
            val after=observer(scenario, before==null)
            val commands=after.getJSONArray("commands")
            val prefix=before?.getJSONArray("commands")
            if(prefix!=null) {
                assertTrue(commands.length()>=prefix.length())
                for(i in 0 until prefix.length()) assertEquals(prefix.getJSONObject(i).toString(),commands.getJSONObject(i).toString())
            }
            row.put("ipcStart",prefix?.length() ?: 0).put("ipcEnd",commands.length())
                .put("documentFromAtMs",before?.getDouble("atMs") ?: after.getDouble("installedAtMs"))
                .put("documentToAtMs",after.getDouble("atMs"))
                .put("transportAttempts",commands.length()-(prefix?.length() ?: 0))
                .put("observer",after).put("ui",js(scenario,page)).put("success",true)
        } catch(error: Throwable) {
            row.put("errorClass",error.javaClass.name)
            try {
                row.put("failureUi",js(scenario,page)).put("failureObserver",observer(scenario))
                failureScreenshot(scenario,name)
            } catch(diagnostic:Throwable) { row.put("diagnosticErrorClass",diagnostic.javaClass.name) }
            throw error
        } finally {
            val end=SystemClock.elapsedRealtimeNanos()
            row.put("nativeFinishedAtNs",end).put("durationMs",(end-began)/1_000_000.0)
            writeReport()
        }
    }

    private fun writeReport() {
        report.put("phases",phases)
        synchronized(memory) { report.put("memorySamples",JSONArray(memory.toString())) }
        File(evidence,"report.json").writeText(report.toString(2))
    }

    @Test fun fixedVaultUiTransportBaseline() {
        val args=InstrumentationRegistry.getArguments()
        assertEquals("true",args.getString("privateDataBackedUp"))
        runId=requireNotNull(args.getString("observerRunId"));require(Regex("[a-f0-9]{32}").matches(runId))
        val count=requireNotNull(args.getString("objectCount")).toInt();require(count==100 || count==5000)
        val tag=requireNotNull(args.getString("evidenceTag"));require(tag=="rf312-perf-$runId")
        val context=instrumentation.targetContext
        assertEquals("com.solosoul.app",context.packageName)
        assertFalse("Main app must be optimized non-debuggable Release",context.applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE!=0)
        val root=File(context.applicationInfo.dataDir)
        val config=File(root,"app_config.json")
        if(config.exists()) assertTrue("Fixed private fixture must not redirect to a SAF root",JSONObject(config.readText()).optString("saf_tree_uri").isBlank())
        val marker=JSONObject(File(root,"rf312-fixture.json").readText())
        assertEquals("synthetic-native-vault-fixture",marker.getString("scope"))
        assertEquals(count,marker.getInt("objectCount"));assertEquals("release",marker.getString("buildProfile"))
        val kdf=marker.getJSONObject("kdf")
        assertEquals(65536,kdf.getInt("memoryKiB"));assertEquals(3,kdf.getInt("iterations"));assertEquals(4,kdf.getInt("parallelism"))
        val account="acc_rf312_$count"
        expectedAccount=account
        val accounts=JSONArray(File(root,"accounts.json").readText())
        assertEquals(1,accounts.length());assertEquals(account,accounts.getJSONObject(0).getString("id"))
        assertEquals("Performance Fixture",accounts.getJSONObject(0).getString("name"))
        assertTrue(File(root,"$account/vault.db").isFile)
        for(file in root.walkTopDown()) if(file.name.startsWith("acc_")) assertEquals(account,file.name)
        evidence=File(requireNotNull(context.getExternalFilesDir(null)),tag).also { check(it.mkdir()) }
        report=JSONObject().put("schemaVersion",1).put("scope","android-native-fixed-vault-ui-transport-baseline")
            .put("measurementDiagnosticOnly",false)
            .put("touchReadinessDefinition","three committed observations and >=350ms stable target/viewport/notification geometry; native bounds and DOM hit match; one native tap only; typing shares original45s budget")
            .put("runId",runId).put("processId",android.os.Process.myPid()).put("uid",android.os.Process.myUid())
            .put("objectCount",count).put("success",false)
            .put("fixtureSearchMatches",count/20).put("displayedSearchResults",minOf(count/20,50)).put("productionSearchResultLimit",50)
            .put("nativeClock","Android elapsedRealtimeNanos; not converted to document clock")
            .put("startupDefinition","Activity launch to usable production password form, focused own WebView and two frames; excludes runner/process launch")
            .put("inputMethod","target WebView native touch dispatch and Instrumentation.sendStringSync; includes instrumentation/polling/drawing overhead")
            .put("memoryDefinition","serial main-process PSS sampling including runner, not renderer memory or continuous peak")
            .put("ipcDefinition","actual transport sends from early observer, including startup/retries/channels; no request arguments or bodies")
            .put("unmeasured",JSONArray(listOf("OCR","attachment-preview","PDF-preview","renderer-memory","process-cold-start","physical-device","other-platforms")))
        sampling.set(true)
        val worker=Thread {
            try {
                while(sampling.get()) {
                    val started=SystemClock.elapsedRealtimeNanos();val info=Debug.MemoryInfo();Debug.getMemoryInfo(info)
                    assertTrue("Actual main-process PSS must be available",info.totalPss>0)
                    synchronized(memory) {
                        check(memory.length()<1024)
                        memory.put(JSONObject().put("startedAtNs",started).put("finishedAtNs",SystemClock.elapsedRealtimeNanos()).put("pssBytes",info.totalPss.toLong()*1024))
                    }
                    Thread.sleep(1000)
                }
            } catch(error:Throwable) { memoryFailure.set(error) }
        }.also { it.start() }
        try {
            val began=SystemClock.elapsedRealtimeNanos()
            val scenario=ActivityScenario.launch(MainActivity::class.java)
            phase(scenario,"activity-startup",began,null) {
                waitFor(scenario,page,began) { it.optBoolean("mounted") && it.optBoolean("startupGone") && it.optString("platform")=="android" &&
                    it.optString("path")=="/login" && it.optBoolean("password") && it.optBoolean("enabled") &&
                    it.optBoolean("singleAccount") && it.optBoolean("publicAccountName") && it.optBoolean("publicAccountId") }
                scenario.onActivity { activity ->
                    val view=requireNotNull(webView(activity.window.decorView))
                    assertTrue("Only the shown target view in its already focused native window may receive focus",view.isAttachedToWindow && view.isShown && view.hasWindowFocus())
                    assertTrue("Target WebView must accept local input focus",view.requestFocus())
                }
                waitFor(scenario,"JSON.stringify({mainFrame:window.top===window,focused:document.hasFocus()})",began) {
                    it.optBoolean("mainFrame") && it.optBoolean("focused")
                }
            }
            fun measure(name:String,action:()->Unit) {
                val before=observer(scenario);phase(scenario,name,SystemClock.elapsedRealtimeNanos(),before,action)
            }
            fun unlock() {
                typePublic(scenario,"[data-login-method-region='password'] input","perf-baseline-only-password")
                touch(scenario,"[data-login-password-submit]")
                waitFor(scenario,page) { it.optBoolean("home") && it.optString("count")==count.toString() && it.optInt("errors")==0 }
            }
            measure("password-unlock") { unlock() }
            measure("workspace") {
                touch(scenario,".android-category:first-child > button:first-child")
                waitFor(scenario,page) { it.optString("path")=="/workspace" && it.optString("search")=="?section=identity" && it.optInt("errors")==0 }
                // The frozen fixture uses type=note, section=identity. Match the
                // existing Windows journey: enter identity then select All.
                touch(scenario,".android-chips > button:first-child")
                waitFor(scenario,page) { it.optString("path")=="/workspace" && it.optString("search")=="" && it.optInt("cards")==minOf(count,50) && it.optBoolean("firstPublicObject") && it.optInt("errors")==0 }
            }
            touch(scenario,".android-navigation a[href='/tools']")
            waitFor(scenario,page) { it.optString("path")=="/tools" }
            val search=js(scenario,"""(() => {const nodes=[...document.querySelectorAll('.android-tool')];
              return JSON.stringify({index:nodes.findIndex(e=>e.querySelector('strong')?.textContent==='Search')});})()""")
            assertTrue(search.getInt("index")>=0)
            touch(scenario,".android-tool:nth-child(${search.getInt("index")+1})")
            waitFor(scenario,page) { it.optString("path")=="/search" }
            measure("search-needle") {
                typePublic(scenario,"input","needle")
                waitFor(scenario,page) { it.optString("path")=="/search" && it.optBoolean("publicSearch") && it.optInt("searchResults")==minOf(count/20,50) && it.optInt("errors")==0 }
                var keyboardVisible=false
                scenario.onActivity { activity ->
                    keyboardVisible=requireNotNull(ViewCompat.getRootWindowInsets(activity.window.decorView)).isVisible(WindowInsetsCompat.Type.ime())
                }
                if(keyboardVisible)instrumentation.sendKeyDownUpSync(KeyEvent.KEYCODE_BACK)
                waitFor(scenario,page) { it.optString("path")=="/search" && it.optBoolean("publicSearch") && it.optInt("searchResults")==minOf(count/20,50) }
            }
            measure("application-lock") {
                touch(scenario,"button[aria-label='Lock Vault']")
                waitFor(scenario,page) { it.optString("path")=="/login" && !it.optBoolean("home") && it.optBoolean("password") && it.optBoolean("enabled") }
            }
            measure("password-reunlock") { unlock() }
            assertEquals(6,phases.length());assertNull(memoryFailure.get())
            report.put("success",true)
        } catch(error:Throwable) {
            report.put("errorClass",error.javaClass.name);throw error
        } finally {
            sampling.set(false);worker.join(2000)
            if(worker.isAlive || memoryFailure.get()!=null) report.put("success",false).put("memoryError",true)
            report.put("memorySamplerExited",!worker.isAlive);writeReport()
            // Keep the owned activity alive until the external driver stops and
            // restores the target; ActivityScenario.close may stop the runner.
            assertFalse("Memory sampler must stop",worker.isAlive);assertNull(memoryFailure.get())
        }
    }
}
