package com.solosoul.app

import android.graphics.Bitmap
import android.view.View
import android.view.ViewGroup
import android.view.KeyEvent
import android.view.MotionEvent
import android.os.SystemClock
import android.view.accessibility.AccessibilityNodeInfo
import android.webkit.WebView
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
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

/** FE2-014：生产页面、真实 IPC / 账户，不替换认证 store 或首页数据。
 * 会创建公开的临时账户；仅允许已备份私有目录的专用模拟器驱动运行。
 * 驱动必须在进程退出后恢复完整私有目录，不能在 Vault 持锁时覆盖文件。
 */
@RunWith(AndroidJUnit4::class)
class AndroidHomeInstrumentedTest {
    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()
    private val records = JSONArray()
    private lateinit var evidence: File
    private var directoryAttempts = 0
    private val unlockAttempts = JSONArray()
    private var notificationPrompts = 0
    private var overlayChecks = false
    private var keyboardChecks = false

    private fun writeReport() {
        File(evidence, "report.json").writeText(JSONObject().put("records", records)
            .put("directoryAttempts", directoryAttempts).put("unlockAttempts", unlockAttempts)
            .put("notificationPrompts", notificationPrompts)
            .put("scenario", if (overlayChecks) "overlays" else if (keyboardChecks) "keyboard" else "baseline")
            .put("scope", "production UI bootstrap/appearance/lock/password unlock, encrypted preferences, populated home/editor/list/detail/actions and Android system back" +
                if (overlayChecks) ", real backup reminder inside attachment/history overlays and nested system back"
                else if (keyboardChecks) ", real touch/IME/typing, backup reminder, Save reachability and keyboard-first system back"
                else "")
            .put("cleanup", "external driver must force-stop and restore full private data backup").toString(2))
    }

    private fun record(result: JSONObject) {
        result.put("recordedAtMs", System.currentTimeMillis())
        records.put(result)
        // 原生进程退出时 finally 未必执行，已完成阶段仍应有落盘诊断。
        writeReport()
    }

    private fun handleNotificationPrompt() {
        // 系统切换到 permissioncontroller 的首帧可能尚无可访问性文字。
        // 有界重读当前树；只有完整识别 SoloSoul 通知请求后才操作按钮。
        val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(3)
        do {
            val root = instrumentation.uiAutomation.rootInActiveWindow ?: return
            if (root.packageName?.toString()?.endsWith(".permissioncontroller") != true) return
            val message = root.findAccessibilityNodeInfosByText("notifications")
            val known = message.any { it.text?.toString()?.contains("SoloSoul") == true }
            val deny = root.findAccessibilityNodeInfosByText("allow").firstOrNull {
                it.text?.toString()?.replace('\u2019', '\'') == "Don't allow" && it.isClickable
            }
            if (known && deny != null) {
                assertTrue("通过真实原生 UI 拒绝通知", deny.performAction(AccessibilityNodeInfo.ACTION_CLICK))
                notificationPrompts++
                instrumentation.waitForIdleSync()
                return
            }
            Thread.sleep(100)
        } while (System.nanoTime() < deadline)
        throw AssertionError("未知或持续不完整的权限弹窗不能自动处理")
    }

    private fun webView(view: View): WebView? {
        if (view is WebView) return view
        if (view is ViewGroup) for (index in 0 until view.childCount) {
            webView(view.getChildAt(index))?.let { return it }
        }
        return null
    }

    private fun js(scenario: ActivityScenario<MainActivity>, expression: String): JSONObject {
        handleNotificationPrompt()
        val done = CountDownLatch(1)
        var raw: String? = null
        scenario.onActivity { activity ->
            val view = webView(activity.window.decorView)
            if (view == null) done.countDown() else view.evaluateJavascript(expression) {
                raw = it
                done.countDown()
            }
        }
        assertTrue("生产 WebView 必须响应", done.await(3, TimeUnit.SECONDS))
        val parsed = JSONArray("[${raw ?: "null"}]").opt(0)
        return if (parsed is String && parsed.startsWith("{")) JSONObject(parsed) else JSONObject()
    }

    private fun waitFor(scenario: ActivityScenario<MainActivity>, label: String,
                        expression: String, predicate: (JSONObject) -> Boolean): JSONObject {
        val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(45)
        var last = JSONObject()
        do {
            last = js(scenario, expression)
            if (predicate(last)) return last
            Thread.sleep(100)
        } while (System.nanoTime() < deadline)
        throw AssertionError("$label: $last")
    }

    private fun clickText(scenario: ActivityScenario<MainActivity>, text: String) {
        val result = js(scenario, """(() => {
          const target = [...document.querySelectorAll('button,[role="button"]')]
            .find(e => !e.disabled && e.getBoundingClientRect().height > 0 && e.textContent.trim() === ${JSONObject.quote(text)});
          target?.click(); return JSON.stringify({clicked:!!target});
        })()""".trimIndent())
        assertTrue("必须点击真实按钮: $text", result.optBoolean("clicked"))
    }

    private fun clickSelector(scenario: ActivityScenario<MainActivity>, selector: String) {
        val result = js(scenario, """(() => {
          const target = document.querySelector(${JSONObject.quote(selector)});
          const enabled=!!target && !target.disabled && target.getBoundingClientRect().height>0;
          if(enabled)target.click(); return JSON.stringify({clicked:enabled});
        })()""".trimIndent())
        assertTrue("真实入口不存在: $selector", result.optBoolean("clicked"))
    }

    private fun screenshot(scenario: ActivityScenario<MainActivity>): Bitmap {
        // DOM 提交不等于屏幕绘制完成；先等待对应 WebView 的视觉状态，再取系统合成帧。
        val painted = CountDownLatch(1)
        var committed = false
        handleNotificationPrompt()
        scenario.onActivity { activity ->
            val view = requireNotNull(webView(activity.window.decorView))
            view.invalidate()
            view.postVisualStateCallback(System.nanoTime(), object : WebView.VisualStateCallback() {
                override fun onComplete(requestId: Long) {
                    committed = true
                    view.invalidate()
                    view.postOnAnimation { painted.countDown() }
                }
            })
        }
        val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(10)
        while (painted.count > 0 && System.nanoTime() < deadline) {
            // 系统权限弹窗可能在回调排队后出现，先通过原生界面关闭它，
            // 仍须等待同一个视觉提交；不能接受失焦时的旧截图。
            handleNotificationPrompt()
            painted.await(100, TimeUnit.MILLISECONDS)
        }
        if (painted.count > 0) {
            var state = JSONObject()
            scenario.onActivity { activity ->
                val view = requireNotNull(webView(activity.window.decorView))
                state = JSONObject().put("shown", view.isShown).put("attached", view.isAttachedToWindow)
                    .put("windowVisibility", view.windowVisibility).put("focused", view.hasWindowFocus())
                    .put("hardwareAccelerated", view.isHardwareAccelerated).put("width", view.width)
                    .put("height", view.height).put("committed", committed)
            }
            throw AssertionError("生产 WebView 必须提交视觉状态: $state")
        }
        instrumentation.waitForIdleSync()
        Thread.sleep(400)
        return requireNotNull(instrumentation.uiAutomation.takeScreenshot())
    }

    private val page = """JSON.stringify({path:location.pathname,
      buttons:[...document.querySelectorAll('button')].filter(e=>!e.disabled).map(e=>e.textContent.trim()),
      mounted:!!document.querySelector('#root>*'), startupGone:!document.querySelector('#startup-screen'),
      platform:document.documentElement.dataset.platform,text:document.body.innerText,
      glass:document.documentElement.dataset.androidGlass,hidden:document.hidden,createTrigger:window.fe2MenuTrigger,
      home:!!document.querySelector('[data-testid="android-home"]'),
      alerts:[...document.querySelectorAll('[role="alert"]')].map(e=>e.textContent.trim())})"""

    private val home = """(() => {
      const root=document.documentElement, style=getComputedStyle(root);
      const copy=document.querySelector('.android-overview-copy'), art=document.querySelector('.android-liquid-artwork');
      const rect=e=>{if(!e)return null;const r=e.getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height,right:r.right,bottom:r.bottom}};
      const visible=e=>{if(!e)return false;let n=e;while(n){const s=getComputedStyle(n);if(s.display==='none'||s.visibility==='hidden'||Number(s.opacity)===0)return false;n=n.parentElement}return e.getBoundingClientRect().height>0};
      const canvas=art?.querySelector('canvas');
      return JSON.stringify({path:location.pathname,home:!!document.querySelector('[data-testid="android-home"]'),
        theme:root.dataset.theme,glass:root.dataset.androidGlass,
        background:style.getPropertyValue('--bg-base').trim(),surface:style.getPropertyValue('--bg-elevated').trim(),
        foreground:style.getPropertyValue('--text-primary').trim(),accent:style.getPropertyValue('--accent-primary').trim(),
        count:copy?.querySelector('strong')?.textContent,copyText:copy?.textContent,copyVisible:visible(copy),
        name:document.querySelector('.android-home-intro h2')?.textContent,
        copyColor:copy?getComputedStyle(copy.querySelector('strong')).color:null,copyRect:rect(copy),artRect:rect(art),
        ready:art?.dataset.liquidReady==='true',canvasVisible:visible(canvas),
        canvasWidth:canvas?.width,canvasHeight:canvas?.height,
        viewportWidth:innerWidth,overflow:document.documentElement.scrollWidth>innerWidth,
        navCount:document.querySelectorAll('.android-navigation a').length});
    })()"""

    private fun capture(scenario: ActivityScenario<MainActivity>, stage: String,
                        dark: Boolean, enhanced: Boolean, count: String = "0") {
        val result = waitFor(scenario, stage, home) {
            it.optBoolean("home") && it.optString("count") == count && it.optBoolean("copyVisible") &&
            it.optString("theme") == (if (dark) "dark" else "light") &&
            it.optString("glass") == (if (enhanced) "enhanced" else "local") &&
            (!enhanced || it.optBoolean("ready"))
        }
        assertTrue(result.getString("name").contains("FE2 public visual test"))
        assertTrue(result.getString("copyText").contains("My vault"))
        assertTrue(result.getString("copyText").contains("Local vault"))
        assertFalse("首页不能横向溢出", result.getBoolean("overflow"))
        assertEquals(4, result.getInt("navCount"))
        assertEquals(if (dark) "#1f1c18" else "#fafaf6", result.getString("background"))
        assertEquals(if (dark) "#2a2620" else "#fdfcf9", result.getString("surface"))
        assertEquals(if (dark) "#ddd8c8" else "#1f1c18", result.getString("foreground"))
        if (enhanced) {
            assertTrue("实际首页画布必须可见", result.getBoolean("canvasVisible"))
            assertTrue(result.getInt("canvasWidth") > 0 && result.getInt("canvasHeight") > 0)
            val copy = result.getJSONObject("copyRect")
            val art = result.getJSONObject("artRect")
            assertTrue("装饰不得盖住资料库文案", copy.getDouble("right") <= art.getDouble("x") + 1)
        }
        val bitmap = screenshot(scenario)
        // 检查最终合成帧里的真实文字像素，不能仅以 DOM 存在作为“没有被材质遮住”。
        var viewWidth = 0
        val location = IntArray(2)
        scenario.onActivity { activity ->
            val view = requireNotNull(webView(activity.window.decorView))
            viewWidth = view.width
            view.getLocationOnScreen(location)
        }
        val scale = viewWidth.toDouble() / result.getDouble("viewportWidth")
        val copy = result.getJSONObject("copyRect")
        val left = (location[0] + copy.getDouble("x") * scale).toInt().coerceIn(0, bitmap.width - 1)
        val top = (location[1] + copy.getDouble("y") * scale).toInt().coerceIn(0, bitmap.height - 1)
        val right = (location[0] + copy.getDouble("right") * scale).toInt().coerceIn(left + 1, bitmap.width)
        val bottom = (location[1] + copy.getDouble("bottom") * scale).toInt().coerceIn(top + 1, bitmap.height)
        val target = if (dark) intArrayOf(221, 216, 200) else intArrayOf(31, 28, 24)
        var inkPixels = 0
        for (y in top until bottom) for (x in left until right) {
            val pixel = bitmap.getPixel(x, y)
            if (kotlin.math.abs(android.graphics.Color.red(pixel) - target[0]) <= 8 &&
                kotlin.math.abs(android.graphics.Color.green(pixel) - target[1]) <= 8 &&
                kotlin.math.abs(android.graphics.Color.blue(pixel) - target[2]) <= 8) inkPixels++
        }
        assertTrue("最终首页帧必须显示文字和数量: pixels=$inkPixels", inkPixels >= 20)
        File(evidence, "$stage.png").outputStream().use {
            bitmap.compress(Bitmap.CompressFormat.PNG, 100, it)
        }
        result.put("stage", stage).put("screenshotWidth", bitmap.width).put("screenshotHeight", bitmap.height)
            .put("copyInkPixels", inkPixels).put("webViewScale", scale)
        bitmap.recycle()
        record(result)
    }

    private fun captureContent(scenario: ActivityScenario<MainActivity>, stage: String,
                               selector: String, text: String) {
        val checkToast = stage == "object-editor-dark" || (!overlayChecks && !keyboardChecks && stage == "object-actions-dark") ||
            stage == "object-attachments-dark" || stage == "object-history-dark"
        val labels = when (stage) {
            "object-editor-dark" -> listOf("Save")
            "object-actions-dark" -> listOf("Edit", "History", "Attachments", "Delete")
            "object-attachments-dark" -> listOf("Upload", "Close")
            "object-history-dark" -> listOf("Close")
            else -> emptyList()
        }
        val expression = """(() => {
          const e=document.querySelector(${JSONObject.quote(selector)}),s=e?getComputedStyle(e):null,r=e?.getBoundingClientRect();
          const toast=document.querySelector('[data-toast-container]'),tr=toast?.getBoundingClientRect();
          const labels=${JSONArray(labels)};
          const controls=labels.map(label=>{
            const scope=${if (stage == "object-editor-dark") "document" else "e"};
            const b=[...(scope?.querySelectorAll('button')||[])].find(b=>!b.disabled&&(b.textContent.trim().startsWith(label)||b.getAttribute('aria-label')===label)),br=b?.getBoundingClientRect();
            const hit=br?document.elementFromPoint(br.x+br.width/2,br.y+br.height/2):null;
            return {label,found:!!b,visible:!!br&&br.y>=0&&br.bottom<=innerHeight,
              hittable:!!b&&(hit===b||b.contains(hit)),
              overlapped:!!tr&&!!br&&tr.left<br.right&&tr.right>br.left&&tr.top<br.bottom&&tr.bottom>br.top};
          });
          return JSON.stringify({path:location.pathname,search:location.search,theme:document.documentElement.dataset.theme,
            historyIndex:history.state?.idx,historyLength:history.length,overlayMarker:history.state?.solosoulOverlayLayer===true,
            text:e?.textContent,visible:!!r&&r.height>0&&s.display!=='none'&&s.visibility!=='hidden',
            settled:!!e&&e.getAnimations().every(a=>a.playState!=='running'),
            background:s?.backgroundColor,foreground:s?.color,backdrop:s?.backdropFilter,
            rect:r?{x:r.x,y:r.y,right:r.right,bottom:r.bottom}:null,
            toastCount:document.querySelectorAll('[data-toast-container]').length,
            backupReminder:!!toast&&[...toast.querySelectorAll('button')].some(b=>b.textContent.trim()==='Back Up Now'),
            toastTrace:window.fe2ToastTrace||[],
            toastInFlow:!!toast&&getComputedStyle(toast).position==='static',
            toastInSheet:!!toast?.closest('.android-sheet'),toastInPanel:!!e?.contains(toast),
            toastBackdrop:toast?.closest('[data-macos-glass-backdrop]')?.style.zIndex,
            toastVisible:!!tr&&tr.top>=0&&tr.bottom<=innerHeight,
            toastHittable:!!tr&&toast.contains(document.elementFromPoint(tr.x+tr.width/2,tr.y+tr.height/2)),controls,
            viewportWidth:innerWidth,viewportHeight:innerHeight,overflow:document.documentElement.scrollWidth>innerWidth});
        })()""".trimIndent()
        val result = waitFor(scenario, stage, expression) {
            it.optBoolean("visible") && it.optBoolean("settled") && it.optString("text").contains(text) &&
                (!checkToast || (it.optInt("toastCount") == 1 && it.optBoolean("backupReminder")))
        }
        assertFalse("内容不能横向溢出", result.getBoolean("overflow"))
        if (checkToast) {
            // 必须捕获真实备份提醒仍存在的窗口，不能靠过期或测试主动关闭来通过。
            assertEquals("当前阶段必须有且仅有一份真实提醒", 1, result.getInt("toastCount"))
            assertTrue("必须是实际备份提醒，不能用保存成功提醒代替", result.getBoolean("backupReminder"))
            assertTrue("提醒必须正常流占位", result.getBoolean("toastInFlow"))
            if (stage == "object-actions-dark") assertTrue("菜单提醒不能留在 inert 页面中", result.getBoolean("toastInSheet"))
            if (stage == "object-attachments-dark" || stage == "object-history-dark") {
                assertTrue("通知必须属于当前附件/历史面板", result.getBoolean("toastInPanel"))
                assertEquals("通知必须在实际前景层级", "5100", result.getString("toastBackdrop"))
                assertTrue("通知必须可见并可命中", result.getBoolean("toastVisible") && result.getBoolean("toastHittable"))
            }
            val controls = result.getJSONArray("controls")
            for (index in 0 until controls.length()) {
                val control = controls.getJSONObject(index)
                assertTrue("底部操作必须存在且可见: $control", control.getBoolean("found") && control.getBoolean("visible"))
                assertFalse("提醒不能覆盖底部操作: $control", control.getBoolean("overlapped"))
                assertTrue("底部操作必须可命中: $control", control.getBoolean("hittable"))
            }
        }
        assertEquals("对象内容应保留深色主题", "dark", result.getString("theme"))
        val rect = result.getJSONObject("rect")
        assertTrue("内容不能超出视口左右沿", rect.getDouble("x") >= -1 &&
            rect.getDouble("right") <= result.getDouble("viewportWidth") + 1)
        if (selector.contains("dialog") || selector.contains("object-detail-modal")) {
            assertTrue("弹层应位于可视高度内", rect.getDouble("y") >= -1 &&
                rect.getDouble("bottom") <= result.getDouble("viewportHeight") + 1)
        }
        val bitmap = screenshot(scenario)
        if (checkToast) {
            val stillPresent = js(scenario, """JSON.stringify({backupReminder:[...document.querySelectorAll('[data-toast-container] button')].some(b=>b.textContent.trim()==='Back Up Now')})""")
            assertTrue("最终合成截图时提醒必须仍存在", stillPresent.getBoolean("backupReminder"))
        }
        File(evidence, "$stage.png").outputStream().use {
            bitmap.compress(Bitmap.CompressFormat.PNG, 100, it)
        }
        result.put("stage", stage).put("screenshotWidth", bitmap.width).put("screenshotHeight", bitmap.height)
        bitmap.recycle()
        record(result)
    }

    private fun verifyPopulatedContent(scenario: ActivityScenario<MainActivity>) {
        clickSelector(scenario, ".android-navigation a[href='/settings']")
        val appearance = js(scenario, """(() => {const e=[...document.querySelectorAll('[role="button"]')].find(e=>e.textContent.includes('Theme & Appearance'));e?.click();return JSON.stringify({clicked:!!e})})()""")
        assertTrue(appearance.getBoolean("clicked"))
        waitFor(scenario, "外观页面", page) { it.optString("path") == "/settings/appearance" }
        clickText(scenario, "Local glass")
        clickSelector(scenario, ".android-navigation a[href='/']")
        waitFor(scenario, "新建前局部玻璃首页", home) {
            it.optBoolean("home") && it.optString("glass") == "local"
        }
        // 首页路由 DOM 就绪不等于 AndroidNavigation 的入口已完成视觉提交。
        // 实际 WebView 绘制后再操作；这不替换菜单 handler 或重试失败操作。
        screenshot(scenario).recycle()
        js(scenario, """(() => {const b=document.querySelector('.android-fab');
          window.fe2MenuTrigger={path:location.pathname,glass:document.documentElement.dataset.androidGlass,
            hidden:document.hidden,busy:b?.getAttribute('aria-busy'),expanded:b?.getAttribute('aria-expanded'),
            disabled:b?.disabled};return JSON.stringify(window.fe2MenuTrigger);})()""")
        clickSelector(scenario, ".android-fab")
        waitFor(scenario, "真实新建菜单", page) { it.optString("text").contains("New object") }
        val newObject = js(scenario, """(() => {const b=[...document.querySelectorAll('.android-sheet button')].find(e=>e.textContent.trim().startsWith('New object'));b?.click();return JSON.stringify({clicked:!!b})})()""")
        assertTrue(newObject.getBoolean("clicked"))
        waitFor(scenario, "对象归属选择", """JSON.stringify({picker:!!document.querySelector('[data-testid="object-destination-picker"]')})""") { it.optBoolean("picker") }
        clickText(scenario, "Travel")
        waitFor(scenario, "出行模板加载", """JSON.stringify({path:location.pathname,search:location.search,templates:[...document.querySelectorAll('button[aria-pressed]')].map(e=>e.textContent)})""") {
            it.optString("path") == "/editor" && it.optString("search") == "?section=travel" &&
                it.optJSONArray("templates")?.toString()?.contains("Visa") == true
        }
        val selected = js(scenario, """(() => {const b=[...document.querySelectorAll('button[aria-pressed]')].find(e=>e.textContent.trim().startsWith('Visa'));b?.click();return JSON.stringify({clicked:!!b})})()""")
        assertTrue(selected.getBoolean("clicked"))
        waitFor(scenario, "签证公开字段", """JSON.stringify({country:[...document.querySelectorAll('label[for]')].some(e=>e.textContent.trim().startsWith('Country'))})""") { it.optBoolean("country") }
        val filled = js(scenario, """(() => {
          const name=document.querySelector('input[aria-label="Object Name"]');
          const label=[...document.querySelectorAll('label[for]')].find(e=>e.textContent.trim().startsWith('Country'));
          const country=label?document.getElementById(label.htmlFor):null;
          if(!name||!country)return JSON.stringify({filled:false});
          for(const [e,value] of [[name,'FE2 public object'],[country,'FE2 public country']]) {
            Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(e,value);
            e.dispatchEvent(new Event('input',{bubbles:true}));
          }return JSON.stringify({filled:true});
        })()""")
        assertTrue(filled.getBoolean("filled"))
        captureContent(scenario, "object-editor-dark", "[data-testid='object-save-destination']", "Travel")
        if (keyboardChecks) verifyKeyboard(scenario)
        clickText(scenario, "Save")
        waitFor(scenario, "保存后首页数量", home) { it.optBoolean("home") && it.optString("count") == "1" }
        val category = js(scenario, """(() => {const b=[...document.querySelectorAll('.android-category button')].find(e=>e.querySelector('strong')?.textContent==='Travel');b?.click();return JSON.stringify({clicked:!!b})})()""")
        assertTrue(category.getBoolean("clicked"))
        captureContent(scenario, "object-list-dark", "[data-testid='workspace-object-card']", "FE2 public object")
        clickSelector(scenario, "[data-testid='workspace-object-card'] .android-row-button")
        captureContent(scenario, "object-detail-dark", "[data-testid='object-detail-modal']", "FE2 public country")
        if (overlayChecks) verifyNestedOverlays(scenario)
        val back = js(scenario, """JSON.stringify({index:history.state?.idx,marker:history.state?.solosoulOverlayLayer===true})""")
        assertTrue("详情必须有可返回的浮层历史标记", back.getInt("index") > 0 && back.getBoolean("marker"))
        // 注入 Android 实际返回键，不直接关闭组件或修改 history/state。
        instrumentation.sendKeyDownUpSync(KeyEvent.KEYCODE_BACK)
        waitFor(scenario, "详情系统返回保留对象页面", """JSON.stringify({path:location.pathname,search:location.search,detail:!!document.querySelector('[data-testid="object-detail-modal"]'),card:!!document.querySelector('[data-testid="workspace-object-card"]')})""") {
            it.optString("path") == "/workspace" && it.optString("search").contains("section=travel") &&
                !it.optBoolean("detail") && it.optBoolean("card")
        }
        clickSelector(scenario, "[data-testid='workspace-object-card'] button[aria-haspopup='dialog']")
        captureContent(scenario, "object-actions-dark", ".android-sheet[role='dialog']", "Attachments")
        instrumentation.sendKeyDownUpSync(KeyEvent.KEYCODE_BACK)
        waitFor(scenario, "操作菜单系统返回保留对象页面", """JSON.stringify({path:location.pathname,search:location.search,sheet:!!document.querySelector('.android-sheet'),card:!!document.querySelector('[data-testid="workspace-object-card"]')})""") {
            it.optString("path") == "/workspace" && it.optString("search").contains("section=travel") &&
                !it.optBoolean("sheet") && it.optBoolean("card")
        }
        captureContent(scenario, "object-list-after-system-back", "[data-testid='workspace-object-card']", "FE2 public object")
        clickSelector(scenario, ".android-navigation a[href='/settings']")
        val finalAppearance = js(scenario, """(() => {const e=[...document.querySelectorAll('[role="button"]')].find(e=>e.textContent.includes('Theme & Appearance'));e?.click();return JSON.stringify({clicked:!!e})})()""")
        assertTrue(finalAppearance.getBoolean("clicked"))
        waitFor(scenario, "外观页面", page) { it.optString("path") == "/settings/appearance" }
        clickText(scenario, "Enhanced glass")
        clickSelector(scenario, ".android-navigation a[href='/']")
        capture(scenario, "populated-enhanced-dark", dark = true, enhanced = true, count = "1")
        clickSelector(scenario, ".android-navigation a[href='/settings']")
        val lightAppearance = js(scenario, """(() => {const e=[...document.querySelectorAll('[role="button"]')].find(e=>e.textContent.includes('Theme & Appearance'));e?.click();return JSON.stringify({clicked:!!e})})()""")
        assertTrue(lightAppearance.getBoolean("clicked"))
        waitFor(scenario, "外观页面", page) { it.optString("path") == "/settings/appearance" }
        clickText(scenario, "Light")
        clickSelector(scenario, ".android-navigation a[href='/']")
        capture(scenario, "populated-enhanced-light", dark = false, enhanced = true, count = "1")
    }

    private fun nativeViewport(scenario: ActivityScenario<MainActivity>): JSONObject {
        var result = JSONObject()
        scenario.onActivity { activity ->
            val view = requireNotNull(webView(activity.window.decorView))
            val insets = requireNotNull(ViewCompat.getRootWindowInsets(view))
            val location = IntArray(2)
            view.getLocationOnScreen(location)
            result = JSONObject().put("imeVisible", insets.isVisible(WindowInsetsCompat.Type.ime()))
                .put("imeHeightPx", insets.getInsets(WindowInsetsCompat.Type.ime()).bottom)
                .put("screenHeightPx", activity.window.decorView.rootView.height)
                .put("webViewWidthPx", view.width).put("webViewHeightPx", view.height)
                .put("webViewScreenX", location[0]).put("webViewScreenY", location[1])
        }
        return result
    }

    private fun waitForIme(scenario: ActivityScenario<MainActivity>, visible: Boolean,
                           restoredHeight: Int? = null): JSONObject {
        val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(10)
        var state = JSONObject()
        do {
            state = nativeViewport(scenario)
            if (state.optBoolean("imeVisible") == visible && (!visible || state.optInt("imeHeightPx") > 0) &&
                (restoredHeight == null || state.optInt("webViewHeightPx") == restoredHeight)) return state
            Thread.sleep(100)
        } while (System.nanoTime() < deadline)
        throw AssertionError("真实输入法可见性必须为 $visible: $state")
    }

    private fun captureKeyboard(scenario: ActivityScenario<MainActivity>, stage: String, input: Boolean,
                                withReminder: Boolean = true, typedDraft: String? = null): JSONObject {
        val result = waitFor(scenario, stage, """(() => {
          const name=document.querySelector('input[aria-label="Object Name"]');
          const save=[...document.querySelectorAll('button')].find(e=>e.textContent.trim()==='Save');
          const e=${if (input) "name" else "save"},r=e?.getBoundingClientRect(),v=visualViewport;
          const toast=document.querySelector('[data-toast-container]'),tr=toast?.getBoundingClientRect();
          const hit=r?document.elementFromPoint(r.x+r.width/2,r.y+r.height/2):null;
          return JSON.stringify({path:location.pathname,search:location.search,historyIndex:history.state?.idx,
            theme:document.documentElement.dataset.theme,draft:name?.value,focused:document.activeElement===name,
            viewportWidth:innerWidth,viewportHeight:innerHeight,visualHeight:v?.height,visualOffset:v?.offsetTop,
            keyboardInset:getComputedStyle(document.documentElement).getPropertyValue('--android-keyboard-inset'),
            rect:r?{x:r.x,y:r.y,right:r.right,bottom:r.bottom}:null,
            hittable:!!e&&(hit===e||e.contains(hit)),toastCount:document.querySelectorAll('[data-toast-container]').length,
            unobscured:!!e&&!!r&&[[r.x+r.width/2,r.top+4],[r.left+4,r.y+r.height/2],[r.right-4,r.y+r.height/2],[r.x+r.width/2,r.bottom-4]].every(([x,y])=>{const h=document.elementFromPoint(x,y);return h===e||e.contains(h)}),
            backupReminder:!!toast&&[...toast.querySelectorAll('button')].some(b=>b.textContent.trim()==='Back Up Now'),
            toastTrace:window.fe2ToastTrace||[],sampledAtMs:Date.now(),
            overlapped:!!tr&&!!r&&tr.left<r.right&&tr.right>r.left&&tr.top<r.bottom&&tr.bottom>r.top,
            overflow:document.documentElement.scrollWidth>innerWidth});
        })()""".trimIndent()) {
            it.optString("path") == "/editor" && it.has("rect") && it.optBoolean("hittable") &&
                (typedDraft == null || it.optString("draft") == typedDraft) &&
                (!withReminder || it.optBoolean("backupReminder"))
        }
        val native = nativeViewport(scenario)
        val scale = native.getDouble("webViewWidthPx") / result.getDouble("viewportWidth")
        val rect = result.getJSONObject("rect")
        val keyboardTop = native.getDouble("screenHeightPx") - native.getDouble("imeHeightPx")
        val bottomOnScreen = native.getDouble("webViewScreenY") + rect.getDouble("bottom") * scale
        assertTrue("操作必须完全位于真实键盘上方: $result / $native", bottomOnScreen <= keyboardTop + 1)
        assertTrue("操作不能超出视觉视口", rect.getDouble("y") >= -1 &&
            rect.getDouble("bottom") <= result.getDouble("visualHeight") + result.getDouble("visualOffset") + 1)
        assertFalse("提醒不能覆盖当前输入/保存操作", result.getBoolean("overlapped"))
        assertTrue("输入/保存不能被其他固定操作栏局部覆盖: $result / $native", result.getBoolean("unobscured"))
        assertFalse("键盘状态不应横向溢出", result.getBoolean("overflow"))
        if (withReminder) assertEquals("提醒必须保持唯一容器", 1, result.getInt("toastCount"))
        if (input && native.getBoolean("imeVisible")) assertTrue("输入焦点必须保留", result.getBoolean("focused"))
        assertEquals("dark", result.getString("theme"))
        val bitmap = screenshot(scenario)
        if (withReminder) {
            assertTrue("截图时真实提醒仍应存在", js(scenario, """JSON.stringify({present:[...document.querySelectorAll('[data-toast-container] button')].some(b=>b.textContent.trim()==='Back Up Now')})""").getBoolean("present"))
        }
        File(evidence, "$stage.png").outputStream().use { bitmap.compress(Bitmap.CompressFormat.PNG, 100, it) }
        bitmap.recycle()
        result.put("stage", stage).put("nativeViewport", native).put("controlBottomOnScreenPx", bottomOnScreen)
            .put("keyboardTopOnScreenPx", keyboardTop)
        record(result)
        return result
    }

    private fun verifyKeyboard(scenario: ActivityScenario<MainActivity>) {
        val target = js(scenario, """(() => {const e=document.querySelector('input[aria-label="Object Name"]');
          e?.scrollIntoView({block:'center'});const r=e?.getBoundingClientRect();
          return JSON.stringify({x:r?.x+r?.width/2,y:r?.y+r?.height/2,width:innerWidth});})()""")
        val native = nativeViewport(scenario)
        val scale = native.getDouble("webViewWidthPx") / target.getDouble("width")
        val x = (native.getDouble("webViewScreenX") + target.getDouble("x") * scale).toFloat()
        val y = (native.getDouble("webViewScreenY") + target.getDouble("y") * scale).toFloat()
        val downTime = SystemClock.uptimeMillis()
        for (action in listOf(MotionEvent.ACTION_DOWN, MotionEvent.ACTION_UP)) {
            val event = MotionEvent.obtain(downTime, SystemClock.uptimeMillis(), action, x, y, 0)
            instrumentation.sendPointerSync(event)
            event.recycle()
        }
        waitForIme(scenario, true)
        // 先捕获真实 8 秒提醒与 IME 同时存在的布局，不能为键盘录入延长提醒。
        val focused = captureKeyboard(scenario, "editor-keyboard-dark", input = true)
        js(scenario, """(() => {const e=[...document.querySelectorAll('button')].find(e=>e.textContent.trim()==='Save');
          e?.scrollIntoView({block:'center'});return JSON.stringify({scrolled:!!e});})()""")
        captureKeyboard(scenario, "editor-keyboard-save-dark", input = false)
        // 只设置公开字段的光标位置，文本经实际 Android 键盘事件写入。
        js(scenario, """(() => {const e=document.querySelector('input[aria-label="Object Name"]');
          e.setSelectionRange(e.value.length,e.value.length);return JSON.stringify({selected:true});})()""")
        instrumentation.sendStringSync(" XYZ")
        val typed = waitFor(scenario, "真实键盘输入完成", """JSON.stringify({draft:document.querySelector('input[aria-label="Object Name"]')?.value,
          path:location.pathname,historyIndex:history.state?.idx})""") {
            it.optString("draft") == focused.getString("draft") + " XYZ"
        }
        instrumentation.sendKeyDownUpSync(KeyEvent.KEYCODE_BACK)
        waitForIme(scenario, false, restoredHeight = native.getInt("webViewHeightPx"))
        val dismissed = captureKeyboard(scenario, "editor-keyboard-after-back", input = false,
            withReminder = false, typedDraft = typed.getString("draft"))
        // 落盘实际键盘返回前的草稿，外部驱动再次核对，不以测试生成值替代采样。
        dismissed.put("draftBeforeBack", typed.getString("draft"))
            .put("historyBeforeBack", typed.getInt("historyIndex"))
            .put("webViewHeightBeforeImePx", native.getInt("webViewHeightPx"))
        writeReport()
        assertEquals("第一次返回只收起键盘，不改变路由历史", focused.getInt("historyIndex"), dismissed.getInt("historyIndex"))
        assertEquals("键盘返回不能丢失草稿", typed.getString("draft"), dismissed.getString("draft"))
    }

    private fun verifyNestedOverlays(scenario: ActivityScenario<MainActivity>) {
        val panel = "[data-macos-glass-backdrop][style*='z-index: 5100'] > [data-macos-glass='panel']"
        clickSelector(scenario, ".android-object-detail-footer button[aria-label^='Attachments']")
        captureContent(scenario, "object-attachments-dark", panel, "Active")
        instrumentation.sendKeyDownUpSync(KeyEvent.KEYCODE_BACK)
        waitFor(scenario, "附件系统返回仅关闭内层", """JSON.stringify({path:location.pathname,search:location.search,
          detail:!!document.querySelector('[data-testid="object-detail-modal"]'),
          inner:!!document.querySelector(${JSONObject.quote(panel)})})""") {
            it.optString("path") == "/workspace" && it.optString("search").contains("section=travel") &&
                it.optBoolean("detail") && !it.optBoolean("inner")
        }
        clickSelector(scenario, ".android-object-detail-footer button[aria-haspopup='dialog']")
        waitFor(scenario, "详情更多菜单", """JSON.stringify({ready:!!document.querySelector('.android-sheet')})""") { it.optBoolean("ready") }
        clickText(scenario, "History")
        captureContent(scenario, "object-history-dark", panel, "History")
        instrumentation.sendKeyDownUpSync(KeyEvent.KEYCODE_BACK)
        waitFor(scenario, "历史系统返回仅关闭内层", """JSON.stringify({path:location.pathname,search:location.search,
          detail:!!document.querySelector('[data-testid="object-detail-modal"]'),
          inner:!!document.querySelector(${JSONObject.quote(panel)}),sheet:!!document.querySelector('.android-sheet')})""") {
            it.optString("path") == "/workspace" && it.optString("search").contains("section=travel") &&
                it.optBoolean("detail") && !it.optBoolean("inner") && !it.optBoolean("sheet")
        }
        record(JSONObject().put("stage", "nested-overlays-after-system-back").put("path", "/workspace")
            .put("search", "?section=travel").put("detailRetained", true))
    }

    @Test fun realHomePreservesGlassAndCopyAcrossLockUnlock() {
        val arguments = InstrumentationRegistry.getArguments()
        val scenarioName = arguments.getString("scenario") ?: "baseline"
        require(scenarioName == "baseline" || scenarioName == "overlays" || scenarioName == "keyboard")
        overlayChecks = scenarioName == "overlays"
        keyboardChecks = scenarioName == "keyboard"
        assertEquals("必须通过完整备份/恢复驱动运行", "true", arguments.getString("privateDataBackedUp"))
        val tag = requireNotNull(arguments.getString("evidenceTag"))
        require(Regex("fe2-home-[a-z0-9-]+").matches(tag))
        val context = instrumentation.targetContext
        // 拒绝任何已有账户，包括未被当前占位 Vault 加载的孤立目录。
        val data = File(context.applicationInfo.dataDir)
        for (file in data.walkTopDown()) {
            check(!file.name.startsWith("acc_")) { "专用应用已有账户，拒绝修改" }
            if (file.name == "accounts.json") check(JSONArray(file.readText()).length() == 0)
        }
        evidence = File(context.getExternalFilesDir(null), tag).also { check(it.mkdir()) }
        var liveScenario: ActivityScenario<MainActivity>? = null
        try {
            File(data, "ui_preferences.json").writeText(JSONObject()
                .put("theme", "light").put("accentColor", "ocean").put("language", "en-US")
                .put("defaultLightTheme", "warm-stone").put("defaultDarkTheme", "warm-stone-dark")
                .put("androidGlass", "local").put("hasSeenOnboarding", false)
                .put("notificationPermissionRequested", true).toString())
            val scenario = ActivityScenario.launch(MainActivity::class.java)
            liveScenario = scenario
            waitFor(scenario, "引导就绪", page) {
                it.optBoolean("mounted") && it.optBoolean("startupGone") &&
                it.optString("platform") == "android" && it.optJSONArray("buttons")?.toString()?.contains("Next") == true
            }
            val observing = js(scenario, """(() => {
              window.fe2ToastTrace=[];
              let previous=-1;
              const sample=()=>{
                const count=document.querySelectorAll('[data-toast-container]').length;
                if(count===previous)return;previous=count;
                window.fe2ToastTrace.push({time:performance.now(),count,path:location.pathname});
              };
              window.fe2ToastObserver=new MutationObserver(sample);
              window.fe2ToastObserver.observe(document.documentElement,{childList:true,subtree:true});
              sample();return JSON.stringify({observing:true});
            })()""")
            assertTrue("当前生产文档必须安装只读时序观察", observing.getBoolean("observing"))
            clickText(scenario, "Next")
            waitFor(scenario, "本地目录入口", page) { it.optJSONArray("buttons")?.toString()?.contains("App Private Directory") == true }
            // 此按钮含描述文本，匹配标题并保持真实 onClick / init_vault_directory 路径。
            for (attempt in 1..3) {
                directoryAttempts = attempt
                val local = js(scenario, """(() => {const b=[...document.querySelectorAll('button')].find(e=>!e.disabled&&e.textContent.includes('App Private Directory'));b?.click();return JSON.stringify({clicked:!!b})})()""")
                assertTrue(local.getBoolean("clicked"))
                try {
                    waitFor(scenario, "目录初始化后下一步", page) { it.optJSONArray("buttons")?.toString()?.contains("Next") == true }
                    break
                } catch (error: AssertionError) {
                    // 启动更新源检查可能仍持有真实 root activity；只允许经 UI 重试
                    // 此明确暂态错误。其他错误立即失败，不能直接 invoke 绕过引导。
                    if (attempt == 3 || !error.message.orEmpty().contains("IMPORT_OPERATIONS_ACTIVE")) throw error
                }
            }
            repeat(3) { clickText(scenario, "Next"); Thread.sleep(100) }
            clickText(scenario, "Done")
            waitFor(scenario, "账户来源", page) { it.optJSONArray("buttons")?.toString()?.contains("No, create a new account") == true }
            clickText(scenario, "No, create a new account")
            waitFor(scenario, "创建账户页面", page) { it.optString("path") == "/bootstrap" && it.optJSONArray("buttons")?.toString()?.contains("Create Account") == true }
            val filled = js(scenario, """(() => {
              const form=document.querySelector('form'), inputs=[...form.querySelectorAll('input')];
              if(inputs.length!==4)return JSON.stringify({filled:false,count:inputs.length});
              ['FE2 public visual test','FE2-public-test-password-20261007','FE2-public-test-password-20261007',''].forEach((value,i)=>{
                Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(inputs[i],value);
                inputs[i].dispatchEvent(new Event('input',{bubbles:true}));
              });return JSON.stringify({filled:true});
            })()""".trimIndent())
            assertTrue(filled.getBoolean("filled"))
            clickText(scenario, "Create Account")
            capture(scenario, "local-light", dark = false, enhanced = false)
            clickSelector(scenario, ".android-navigation a[href='/settings']")
            val appearance = js(scenario, """(() => {const e=[...document.querySelectorAll('[role="button"]')].find(e=>e.textContent.includes('Theme & Appearance'));e?.click();return JSON.stringify({clicked:!!e})})()""")
            assertTrue(appearance.getBoolean("clicked"))
            waitFor(scenario, "外观页面", page) { it.optString("path") == "/settings/appearance" }
            clickText(scenario, "Enhanced glass")
            waitFor(scenario, "增强偏好交付", "JSON.stringify({glass:document.documentElement.dataset.androidGlass})") { it.optString("glass") == "enhanced" }
            clickSelector(scenario, ".android-navigation a[href='/']")
            capture(scenario, "enhanced-light", dark = false, enhanced = true)
            clickSelector(scenario, ".android-navigation a[href='/settings']")
            val dark = js(scenario, """(() => {const e=[...document.querySelectorAll('[role="button"]')].find(e=>e.textContent.includes('Theme & Appearance'));e?.click();return JSON.stringify({clicked:!!e})})()""")
            assertTrue(dark.getBoolean("clicked"))
            waitFor(scenario, "外观页面", page) { it.optString("path") == "/settings/appearance" }
            clickText(scenario, "Dark")
            clickSelector(scenario, ".android-navigation a[href='/']")
            capture(scenario, "enhanced-dark", dark = true, enhanced = true)
            // 真正的前端锁定 action + Rust logout；不是只改地址或 store。
            clickSelector(scenario, "button[aria-label='Lock Vault']")
            waitFor(scenario, "锁定页面", """(() => {const b=document.querySelector('[data-login-password-submit]');return JSON.stringify({path:location.pathname,form:!!b,enabled:!!b&&!b.disabled,pending:!!document.querySelector('[data-login-method-region="pending"]'),home:!!document.querySelector('[data-testid="android-home"]')})})()""") {
                it.optString("path") == "/login" && it.optBoolean("form") &&
                    it.optBoolean("enabled") && !it.optBoolean("pending") && !it.optBoolean("home")
            }
            val password = js(scenario, """(() => {const input=document.querySelector('[data-login-method-region="password"] input');
              Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(input,'FE2-public-test-password-20261007');
              input.dispatchEvent(new Event('input',{bubbles:true}));return JSON.stringify({filled:true})})()""")
            assertTrue(password.getBoolean("filled"))
            // 更新检查等真实后台任务可占用 root activity。仅对明确的维护忙提示
            // 做有限 UI 重试，保留每次结果；密码错误、其他错误及持续繁忙仍失败。
            for (attempt in 1..30) {
                clickSelector(scenario, "[data-login-password-submit]")
                val outcome = waitFor(scenario, "真实解锁结果", page) {
                    it.optBoolean("home") || it.optJSONArray("alerts")?.length()?.let { n -> n > 0 } == true
                }
                if (outcome.optBoolean("home")) {
                    unlockAttempts.put(JSONObject().put("attempt", attempt).put("result", "home"))
                    break
                }
                val alerts = outcome.getJSONArray("alerts")
                unlockAttempts.put(JSONObject().put("attempt", attempt).put("alerts", alerts))
                assertEquals("只允许维护忙的暂态错误重试", 1, alerts.length())
                assertEquals("The vault is under maintenance. Try again later.", alerts.getString(0))
                assertTrue("维护持续繁忙，不能接受解锁验收", attempt < 30)
                Thread.sleep(2000)
            }
            capture(scenario, "enhanced-dark-after-unlock", dark = true, enhanced = true)
            // 真实持久化 IPC 核对，等待 debounce 完成，不假定 DOM 即代表保存。
            js(scenario, """(() => {window.fe2HomePrefs={pending:true};
              window.__TAURI_INTERNALS__.invoke('vault_list_accounts',{}).then(accounts=>{
                if(accounts.length!==1||accounts[0].name!=='FE2 public visual test')throw new Error('账户隔离失败');
                return window.__TAURI_INTERNALS__.invoke('user_data_get_preferences',{accountId:accounts[0].id});
              }).then(prefs=>window.fe2HomePrefs={prefs},error=>window.fe2HomePrefs={error:String(error)});
              return JSON.stringify({started:true});})()""")
            val saved = waitFor(scenario, "账户偏好已加密保存", "JSON.stringify(window.fe2HomePrefs||{})") { it.has("prefs") || it.has("error") }
            assertFalse(saved.toString(), saved.has("error"))
            assertEquals("enhanced", saved.getJSONObject("prefs").getString("androidGlass"))
            assertEquals("dark", saved.getJSONObject("prefs").getString("theme"))
            record(JSONObject().put("stage", "encrypted-preferences").put("theme", "dark").put("androidGlass", "enhanced"))
            verifyPopulatedContent(scenario)
        } catch (error: Throwable) {
            // 仅公开临时账户；记录可见文案，不读取任何输入值或私有 Vault 文件。
            liveScenario?.let { scenario ->
                try {
                    record(js(scenario, page).put("stage", "failure").put("reason", error.message))
                    // 失败诊断允许保存最后一帧，不能将此帧算入通过证据。
                    val bitmap = requireNotNull(instrumentation.uiAutomation.takeScreenshot())
                    File(evidence, "failure.png").outputStream().use {
                        bitmap.compress(Bitmap.CompressFormat.PNG, 100, it)
                    }
                    bitmap.recycle()
                } catch (diagnostic: Throwable) {
                    record(JSONObject().put("stage", "diagnostic-failure").put("reason", diagnostic.message))
                }
            }
            throw error
        } finally {
            writeReport()
        }
    }
}
