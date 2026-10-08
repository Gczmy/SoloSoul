package com.solosoul.app

import android.graphics.Bitmap
import android.view.View
import android.view.ViewGroup
import android.view.KeyEvent
import android.view.MotionEvent
import android.os.SystemClock
import android.os.ParcelFileDescriptor
import android.content.res.Configuration
import android.view.accessibility.AccessibilityNodeInfo
import android.webkit.WebView
import androidx.core.content.FileProvider
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
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

/** FE2-014：生产页面、真实 IPC / 账户，不替换认证 store 或首页数据。
 * 会创建公开的临时账户；仅允许已备份私有目录的专用模拟器驱动运行。
 * 驱动必须在进程退出后恢复完整私有目录，不能在 Vault 持锁时覆盖文件。
 */
@RunWith(AndroidJUnit4::class)
class AndroidHomeInstrumentedTest {

    private val safPickerChecks get() = InstrumentationRegistry.getArguments().getString("safPickerChecks") == "true"
    private val ownedSafUris = mutableListOf<android.net.Uri>()
    private fun nativeNodes(root: AccessibilityNodeInfo): List<AccessibilityNodeInfo> {
        val result = mutableListOf<AccessibilityNodeInfo>()
        fun visit(node: AccessibilityNodeInfo) {
            result.add(node)
            for (i in 0 until node.childCount) node.getChild(i)?.let { visit(it) }
        }
        visit(root)
        return result
    }
    private fun waitNative(label: String, predicate: (AccessibilityNodeInfo) -> Boolean): AccessibilityNodeInfo {
        val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(25)
        do {
            instrumentation.uiAutomation.rootInActiveWindow?.let { if (predicate(it)) return it }
            Thread.sleep(150)
        } while (System.nanoTime() < deadline)
        val root = instrumentation.uiAutomation.rootInActiveWindow
        val visible = root?.let { nativeNodes(it).map { n -> "${n.viewIdResourceName}: ${n.text ?: n.contentDescription}" }.joinToString("\n") }
        throw AssertionError("$label: package=${root?.packageName}\n$visible")
    }
    private fun clickNative(node: AccessibilityNodeInfo) {
        var target: AccessibilityNodeInfo? = node
        repeat(6) {
            val current = target ?: return@repeat
            if (current.isClickable && current.isEnabled) {
                assertTrue("实际系统控件点击", current.performAction(AccessibilityNodeInfo.ACTION_CLICK))
                instrumentation.waitForIdleSync()
                return
            }
            target = current.parent
        }
        throw AssertionError("系统控件无可点击祖先: ${node.text} / ${node.contentDescription}")
    }
    private fun pickPublicDocument(name: String, index: Int) {
        var root = waitNative("实际系统文件选择器", { it.packageName?.toString()?.contains("documentsui") == true })
        File(evidence,"saf-picker-open-$index.png").outputStream().use { stream ->
            val frame=requireNotNull(instrumentation.uiAutomation.takeScreenshot());frame.compress(Bitmap.CompressFormat.PNG,100,stream);frame.recycle()
        }
        record(JSONObject().put("stage","saf-picker-open-$index").put("package",root.packageName.toString()).put("publicFile",name))
        val search = nativeNodes(root).firstOrNull {
            it.contentDescription?.toString() == "Search" || it.viewIdResourceName?.endsWith(":id/option_menu_search") == true
        } ?: throw AssertionError("系统选择器没有可见搜索入口")
        clickNative(search)
        root = waitNative("系统搜索输入", { nativeNodes(it).any { n -> n.isEditable } })
        val input = nativeNodes(root).first { it.isEditable }
        val args = android.os.Bundle().apply { putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE,name) }
        assertTrue("搜索仅公开夹具文件名",input.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT,args))
        // UiAutomation 是测试系统 UI 的通道；普通 Instrumentation 不能向其他应用发键。
        val now = SystemClock.uptimeMillis()
        assertTrue("实际系统搜索回车按下", instrumentation.uiAutomation.injectInputEvent(
            KeyEvent(now, now, KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_ENTER, 0), true))
        assertTrue("实际系统搜索回车松开", instrumentation.uiAutomation.injectInputEvent(
            KeyEvent(now, SystemClock.uptimeMillis(), KeyEvent.ACTION_UP, KeyEvent.KEYCODE_ENTER, 0), true))
        root = waitNative("系统选择器真实文件搜索结果", { nativeNodes(it).any { n -> n.text?.toString() == name && !n.isEditable } })
        File(evidence,"saf-picker-result-$index.png").outputStream().use { stream ->
            val frame=requireNotNull(instrumentation.uiAutomation.takeScreenshot());frame.compress(Bitmap.CompressFormat.PNG,100,stream);frame.recycle()
        }
        val item=nativeNodes(root).last { it.text?.toString() == name && !it.isEditable }
        val bounds=android.graphics.Rect();item.getBoundsInScreen(bounds)
        assertTrue("文件结果行须位于搜索栏下方",bounds.top>200)
        clickNative(item)
        waitNative("选择文件后返回SoloSoul", { it.packageName?.toString() == "com.solosoul.app" })
        record(JSONObject().put("stage","saf-picker-selected-$index").put("publicFile",name).put("actualNativeSelection",true).put("nodeResource",item.viewIdResourceName).put("bounds",bounds.toShortString()))
    }
    private fun seedActualSafFixtures(scenario: ActivityScenario<MainActivity>, files: List<Pair<File,String>>) {
        val resolver=instrumentation.targetContext.contentResolver
        val tag="SoloSoul-FE2-SAF-${java.util.UUID.randomUUID()}"
        for ((file,mime) in files) {
            val values=android.content.ContentValues().apply {
                put(android.provider.MediaStore.MediaColumns.DISPLAY_NAME,file.name)
                put(android.provider.MediaStore.MediaColumns.MIME_TYPE,mime)
                put(android.provider.MediaStore.MediaColumns.RELATIVE_PATH,"Download/$tag/")
                put(android.provider.MediaStore.MediaColumns.IS_PENDING,1)
            }
            val uri=requireNotNull(resolver.insert(android.provider.MediaStore.Downloads.EXTERNAL_CONTENT_URI,values))
            ownedSafUris.add(uri)
            requireNotNull(resolver.openOutputStream(uri)).use { out -> file.inputStream().use { it.copyTo(out) } }
            resolver.update(uri,android.content.ContentValues().apply { put(android.provider.MediaStore.MediaColumns.IS_PENDING,0) },null,null)
        }
        clickSelector(scenario,".android-object-detail-footer button[aria-label^='Attachments']")
        val panel="[data-macos-glass-backdrop][style*='z-index: 5100'] > [data-macos-glass='panel']"
        waitFor(scenario,"附件真实上传入口", "JSON.stringify({ready:!!document.querySelector(\"button[title='Upload']\")})") { it.optBoolean("ready") }
        js(scenario,"""(() => {
          window.fe2SafToasts=[];
          const sample=()=>{for(const e of document.querySelectorAll('[data-toast-container] > div')){
            const text=e.innerText;if(text&&!window.fe2SafToasts.some(x=>x.text===text))window.fe2SafToasts.push({text,time:performance.now()});
          }};
          window.fe2SafObserver=new MutationObserver(sample);window.fe2SafObserver.observe(document.documentElement,{childList:true,subtree:true});sample();
          return JSON.stringify({observing:true});
        })()""")
        for((index,entry) in files.withIndex()) {
            clickSelector(scenario,"button[title='Upload']")
            pickPublicDocument(entry.first.name,index+1)
            waitFor(scenario,"系统文件选择后附件列表可见", "JSON.stringify({ready:!!document.querySelector(\"button[aria-haspopup='dialog'][aria-label*='${entry.first.name}']\"),toasts:window.fe2SafToasts,body:document.body.innerText,observerAlive:!!window.fe2SafObserver})") { it.optBoolean("ready") }
        }
        js(scenario,"""(() => {window.fe2SafVerification={pending:true};(async()=>{
            const invoke=window.__TAURI_INTERNALS__.invoke;
            const accounts=await invoke('vault_list_accounts',{});
            if(accounts.length!==1||accounts[0].name!=='FE2 public visual test')throw Error('not owned synthetic account');
            const objects=await invoke('object_list',{accountId:accounts[0].id,filter:null});
            if(objects.length!==1||objects[0].name!=='FE2 public object')throw Error('not owned object');
            const rows=await invoke('attachment_list',{objectId:objects[0].id,showDeleted:false});
            window.fe2SafVerification={rows:rows.map(f=>({name:f.fileName,path:f.vaultPath,size:f.sizeBytes,source:f.srcPath}))};
        })().catch(e=>window.fe2SafVerification={error:String(e)});return JSON.stringify({started:true});})()""")
        val result=waitFor(scenario,"系统选择器导入真实加密附件", "JSON.stringify(window.fe2SafVerification||{})") { it.has("rows") || it.has("error") }
        assertFalse(result.toString(),result.has("error"))
        val rows=result.getJSONArray("rows");assertEquals(2,rows.length())
        for(i in 0 until rows.length()) {
            val row=rows.getJSONObject(i);assertTrue(row.toString(),row.getLong("size")>0)
            assertTrue("保存原始系统content URI",row.getString("source").startsWith("content://"))
            val magic=ByteArray(4);File(row.getString("path")).inputStream().use{assertEquals(4,it.read(magic))}
            assertEquals("SOLC",String(magic,Charsets.US_ASCII))
        }
        record(JSONObject().put("stage","preview-fixtures-encrypted").put("savedCount",2).put("encrypted",true)
            .put("source","Actual Android DocumentsUI selection through Upload UI and production attachment pipeline").put("actualSafPicker",true))
        instrumentation.sendKeyDownUpSync(KeyEvent.KEYCODE_BACK)
        waitFor(scenario,"上传后回对象详情", "JSON.stringify({panel:!!document.querySelector(${JSONObject.quote(panel)}),detail:!!document.querySelector('[data-testid=object-detail-modal]')})") { !it.optBoolean("panel") && it.optBoolean("detail") }
    }

    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()
    private val records = JSONArray()
    private lateinit var evidence: File
    private var directoryAttempts = 0
    private val unlockAttempts = JSONArray()
    private var notificationPrompts = 0
    private var overlayChecks = false
    private var keyboardChecks = false
    private var accountChecks = false
    private var insetsChecks = false
    private var lifecycleChecks = false
    private var coldChecks = false
    private var previewChecks = false
    private var renderingChecks = false
    private val requestedFontScale get() = InstrumentationRegistry.getArguments().getString("fontScale")?.toDouble()
    private val motionChecks get() = InstrumentationRegistry.getArguments().getString("motionPreferenceChecks") == "true"

    private fun writeReport() {
        val provider = WebView.getCurrentWebViewPackage()
        File(evidence, "report.json").writeText(JSONObject().put("records", records)
            .put("processId", android.os.Process.myPid())
            .put("runtime", JSONObject().put("api", android.os.Build.VERSION.SDK_INT)
                .put("webViewPackage", provider?.packageName).put("webViewVersion", provider?.versionName)
                .put("webViewVersionCode", provider?.longVersionCode))
            .put("actualSafPickerChecks",safPickerChecks)
            .put("directoryAttempts", directoryAttempts).put("unlockAttempts", unlockAttempts)
            .put("notificationPrompts", notificationPrompts).put("motionPreferenceChecks", motionChecks)
            .put("requestedFontScale", requestedFontScale ?: JSONObject.NULL)
            .put("fontMenuBeforeDetail", requestedFontScale != null)
            .put("scenario", if (renderingChecks) "rendering" else if (previewChecks) "previews" else if (coldChecks) "cold" else if (lifecycleChecks) "lifecycle" else if (overlayChecks) "overlays" else if (keyboardChecks) "keyboard" else if (accountChecks) "accounts" else if (insetsChecks) "insets" else "baseline")
            .put("scope", "production UI bootstrap/appearance/lock/password unlock, encrypted preferences, populated home/editor/list/detail/actions and Android system back" +
                if (renderingChecks) ", actual WebGL JS submissions while idle/away/return; CPU submission timing only, not GPU duration, battery or compositor FPS"
                else if (previewChecks) ", real encrypted content-URI fixtures, nonempty attachments/text/image/album/viewer in both themes and real in-flow backup/metadata-save notifications; not system picker"
                else if (overlayChecks) ", real backup reminder inside attachment/history overlays and nested system back"
                else if (keyboardChecks) ", real touch/IME/typing, backup reminder, Save reachability and keyboard-first system back"
                else if (accountChecks) ", two public accounts with distinct saved themes/materials/accents, repeated UI switch, system bars and composed home pixels" +
                    if (coldChecks) ", real process-cold startup/login/password unlock/system theme" else if (lifecycleChecks) ", real system night-mode changes and native Activity/WebView recreation; not process-cold startup" else ""
                else if (insetsChecks) ", physical system-bar safe bounds, home rotation, login IME/Back and actual document reload"
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
      glass:document.documentElement.dataset.androidGlass,reduceMotion:document.documentElement.dataset.userReduceMotion,hidden:document.hidden,createTrigger:window.fe2MenuTrigger,
      home:!!document.querySelector('[data-testid="android-home"]'),
      alerts:[...document.querySelectorAll('[role="alert"]')].map(e=>e.textContent.trim())})"""

    private val home = """(() => {
      const root=document.documentElement, style=getComputedStyle(root);
      const copy=document.querySelector('.android-overview-copy'), art=document.querySelector('.android-liquid-artwork');
      const rect=e=>{if(!e)return null;const r=e.getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height,right:r.right,bottom:r.bottom}};
      const visible=e=>{if(!e)return false;let n=e;while(n){const s=getComputedStyle(n);if(s.display==='none'||s.visibility==='hidden'||Number(s.opacity)===0)return false;n=n.parentElement}return e.getBoundingClientRect().height>0};
      const canvas=art?.querySelector('canvas');
      const navigation=document.querySelector('.android-navigation');
      return JSON.stringify({path:location.pathname,home:!!document.querySelector('[data-testid="android-home"]'),
        userAgent:navigator.userAgent,cssSupport:{backdropFilter:CSS.supports('backdrop-filter','blur(1px)'),
          colorMix:CSS.supports('color','color-mix(in srgb, red 50%, blue)'),dynamicViewport:CSS.supports('height','100dvh')},
        theme:root.dataset.theme,glass:root.dataset.androidGlass,reduceMotion:root.dataset.userReduceMotion,
        background:style.getPropertyValue('--bg-base').trim(),surface:style.getPropertyValue('--bg-elevated').trim(),
        foreground:style.getPropertyValue('--text-primary').trim(),accent:style.getPropertyValue('--accent-primary').trim(),
        count:copy?.querySelector('strong')?.textContent,copyText:copy?.textContent,copyVisible:visible(copy),
        name:document.querySelector('.android-home-intro h2')?.textContent,
        copyColor:copy?getComputedStyle(copy.querySelector('strong')).color:null,copyRect:rect(copy),artRect:rect(art),
        ready:art?.dataset.liquidReady==='true',canvasVisible:visible(canvas),
        canvasWidth:canvas?.width,canvasHeight:canvas?.height,
        viewportWidth:innerWidth,overflow:document.documentElement.scrollWidth>innerWidth,
        navigationBackground:navigation?getComputedStyle(navigation).backgroundColor:null,
        navigationBackdrop:navigation?getComputedStyle(navigation).backdropFilter:null,
        navCount:document.querySelectorAll('.android-navigation a').length});
    })()"""

    private fun capture(scenario: ActivityScenario<MainActivity>, stage: String,
                        dark: Boolean, enhanced: Boolean, count: String = "0") {
        var result = waitFor(scenario, stage, home) {
            it.optBoolean("home") && it.optString("count") == count && it.optBoolean("copyVisible") &&
            it.optString("theme") == (if (dark) "dark" else "light") &&
            it.optString("glass") == (if (enhanced) "enhanced" else "local") &&
            (!enhanced || it.optBoolean("ready"))
        }
        if (requestedFontScale != null) {
            scrollTypographyIntoView(scenario)
            result = waitFor(scenario, "$stage 滚动后", home) {
                it.optBoolean("home") && it.optString("count") == count && it.optBoolean("copyVisible") &&
                it.optString("theme") == (if (dark) "dark" else "light") &&
                it.optString("glass") == (if (enhanced) "enhanced" else "local") &&
                (!enhanced || it.optBoolean("ready"))
            }
        }
        assertTrue(result.getString("name").contains("FE2 public visual test"))
        assertTrue(result.getString("copyText").contains("My vault"))
        assertTrue(result.getString("copyText").contains("Local vault"))
        assertFalse("首页不能横向溢出", result.getBoolean("overflow"))
        assertEquals(4, result.getInt("navCount"))
        val navigationColor = result.getString("navigationBackground")
        val navigationChannels = navigationColor.substringAfter('(').substringBefore(')').split(',')
        assertTrue("导航必须保留主题着色，不能因旧 CSS 语法变为全透明: $navigationColor",
            navigationColor.startsWith("rgb") && (navigationChannels.size == 3 ||
                navigationChannels.size == 4 && navigationChannels[3].trim().toDouble() >= 0.85))
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
        if (requestedFontScale != null) captureTypography(scenario, result)
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
        if (requestedFontScale != null) {
            val controls = result.getJSONObject("typography").getJSONObject("header").getJSONArray("controls")
            for (index in 0 until controls.length()) {
                val control = controls.getJSONObject(index)
                val rect = control.getJSONObject("svg")
                val ink = Regex("\\d+").findAll(control.getString("color")).take(3).map { it.value.toInt() }.toList()
                assertEquals(3, ink.size)
                val x = (location[0] + rect.getDouble("x") * scale).toInt().coerceIn(0, bitmap.width-1)
                val y = (location[1] + rect.getDouble("y") * scale).toInt().coerceIn(0, bitmap.height-1)
                val right = (location[0] + rect.getDouble("right") * scale).toInt().coerceIn(x+1, bitmap.width)
                val bottom = (location[1] + rect.getDouble("bottom") * scale).toInt().coerceIn(y+1, bitmap.height)
                val width = right-x
                val pixels = IntArray(width*(bottom-y))
                bitmap.getPixels(pixels,0,width,x,y,width,bottom-y)
                val painted = pixels.count { pixel ->
                    kotlin.math.abs(android.graphics.Color.red(pixel)-ink[0])<=8 &&
                    kotlin.math.abs(android.graphics.Color.green(pixel)-ink[1])<=8 &&
                    kotlin.math.abs(android.graphics.Color.blue(pixel)-ink[2])<=8
                }
                control.put("inkPixels", painted)
                if (painted < 6) record(JSONObject(result.toString()).put("stage", "failure-typography-header-pixels"))
                assertTrue("放大字号后顶栏图标须在最终合成帧可见: $control", painted>=6)
            }
        }
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

    /** 放大后内容可以滚动，使用实际触屏滑动把目标带入阅读区。 */
    private fun scrollTypographyIntoView(scenario: ActivityScenario<MainActivity>,
                                         targetSelector: String = ".android-overview",
                                         viewportSelector: String = "[data-shell-content]") {
        val geometry = js(scenario, """(() => {
          const card=document.querySelector(${JSONObject.quote(targetSelector)}),main=document.querySelector(${JSONObject.quote(viewportSelector)});
          // 概览在大字号下可能高于视口；只将本次测量的标题与数量带入阅读区。
          // 不要求长内容整张卡片一次塞进视口，也不缩小产品文字。
          const textRects=${JSONObject.quote(targetSelector)}==='.android-overview'?
            [...document.querySelectorAll('.android-overview-copy > p:first-child,.android-overview-copy strong')].map(e=>{
              const range=document.createRange();range.selectNodeContents(e);return range.getBoundingClientRect();}):[];
          const r=textRects.length===2?{top:Math.min(...textRects.map(r=>r.top)),bottom:Math.max(...textRects.map(r=>r.bottom))}:card?.getBoundingClientRect(),
            m=main?.getBoundingClientRect();
          return JSON.stringify({found:!!r&&!!m,cardTop:r?.top,cardBottom:r?.bottom,
            mainTop:m?.top,mainBottom:m?.bottom,width:innerWidth});
        })()""")
        assertTrue("必须有生产滚动区域和目标", geometry.getBoolean("found"))
        // 原生手势起始移动会被 touch slop 消耗，留 24 CSS px 的阅读余量，
        // 最终仍以真实 Range / 裁切祖先判定，不扩大裁切容差。
        val distance = geometry.getDouble("cardBottom") - geometry.getDouble("mainBottom") + 24
        if (distance <= 24) return
        val native = nativeViewport(scenario)
        val scale = native.getDouble("webViewWidthPx") / geometry.getDouble("width")
        val start = geometry.getDouble("mainBottom") - 20
        val end = (start - distance).coerceAtLeast(geometry.getDouble("mainTop") + 20)
        val x = (native.getDouble("webViewScreenX") + geometry.getDouble("width") * 0.25 * scale).toFloat()
        val downTime = SystemClock.uptimeMillis()
        for (step in 0..14) {
            val y = (native.getDouble("webViewScreenY") + (start + (end-start)*minOf(step,12)/12) * scale).toFloat()
            val action = when(step) { 0 -> MotionEvent.ACTION_DOWN; 14 -> MotionEvent.ACTION_UP; else -> MotionEvent.ACTION_MOVE }
            val event = MotionEvent.obtain(downTime, SystemClock.uptimeMillis(), action, x, y, 0)
            instrumentation.sendPointerSync(event); event.recycle()
            Thread.sleep(if (step >= 12) 80 else 30)
        }
        instrumentation.waitForIdleSync()
        Thread.sleep(200)
    }

    /** 只观察系统配置与生产 DOM 的文字几何；不调用 setTextZoom 或修改页面字号。 */
    private fun captureTypography(scenario: ActivityScenario<MainActivity>, result: JSONObject) {
        val sample = js(scenario, """(() => {
          const selectors=['.android-overview-copy > p:first-child','.android-overview-copy strong'];
          const samples=selectors.map(selector=>{
            const e=document.querySelector(selector);if(!e)return {selector,found:false};
            const range=document.createRange();range.selectNodeContents(e);
            const r=range.getBoundingClientRect(),box=e.getBoundingClientRect(),s=getComputedStyle(e);
            // Range 可能高于 line-height；只有实际裁切祖先越界才说明内容被截断。
            const clippers=[];let ancestor=e;
            while(ancestor){const style=getComputedStyle(ancestor),ar=ancestor.getBoundingClientRect();
              const clipX=style.overflowX!=='visible',clipY=style.overflowY!=='visible';
              if(clipX||clipY)clippers.push({tag:ancestor.tagName,classes:ancestor.className,clipX,clipY,
                box:{x:ar.x,y:ar.y,right:ar.right,bottom:ar.bottom}});
              ancestor=ancestor.parentElement;}
            return {selector,found:true,text:e.textContent,fontSize:s.fontSize,lineHeight:s.lineHeight,
              glyph:{x:r.x,y:r.y,width:r.width,height:r.height,right:r.right,bottom:r.bottom},
              box:{x:box.x,y:box.y,right:box.right,bottom:box.bottom},
              clippers,clipped:clippers.some(c=>(c.clipX&&(r.x<c.box.x-1||r.right>c.box.right+1))||
                (c.clipY&&(r.y<c.box.y-1||r.bottom>c.box.bottom+1)))};
          });
          const heading=document.querySelector('.android-appbar h1'),hr=heading?.getBoundingClientRect();
          const actions=document.querySelector('.android-appbar-actions'),ar=actions?.getBoundingClientRect();
          const controls=[...document.querySelectorAll('.android-appbar-actions button')].filter(e=>e.getClientRects().length>0&&
            getComputedStyle(e).visibility!=='hidden'&&!e.closest('[inert]')).map(e=>{
            const r=e.getBoundingClientRect(),hit=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);
            const svg=e.querySelector('svg'),sr=svg?.getBoundingClientRect();
            return {label:e.getAttribute('aria-label'),x:r.x,y:r.y,right:r.right,bottom:r.bottom,
              svg:sr?{x:sr.x,y:sr.y,right:sr.right,bottom:sr.bottom,width:sr.width,height:sr.height}:null,
              color:svg?getComputedStyle(svg).color:null,
              width:r.width,height:r.height,visible:r.x>=0&&r.right<=innerWidth&&r.y>=0&&r.bottom<=innerHeight,
              hittable:hit===e||e.contains(hit)};
          });return JSON.stringify({samples,viewportWidth:innerWidth,viewportHeight:innerHeight,
            header:{headingRight:hr?.right,actionsLeft:ar?.left,controls}});
        })()""")
        scenario.onActivity { activity ->
            val view = requireNotNull(webView(activity.window.decorView))
            sample.put("nativeFontScale", activity.resources.configuration.fontScale.toDouble())
                .put("textZoom", view.settings.textZoom)
        }
        assertEquals("实际 Android 字体配置必须匹配驱动设置", requestedFontScale!!,
            sample.getDouble("nativeFontScale"), 0.001)
        assertTrue("实际 WebView 文字缩放必须有值", sample.getInt("textZoom") > 0)
        result.put("typography", sample)
        val header = sample.getJSONObject("header")
        val controls = header.getJSONArray("controls")
        val headerValid = header.getDouble("headingRight") <= header.getDouble("actionsLeft") + 1 &&
            controls.length() >= 3 && (0 until controls.length()).all {
                val control = controls.getJSONObject(it)
                control.getBoolean("visible") && control.getBoolean("hittable") &&
                    control.getDouble("width") >= 48 && control.getDouble("height") >= 48 &&
                    control.optJSONObject("svg")?.let { svg -> svg.getDouble("width") > 0 && svg.getDouble("height") > 0 } == true
            }
        if (!headerValid) record(JSONObject(result.toString()).put("stage", "failure-typography-header"))
        assertTrue("放大字号时标题不能遮挡或挤出顶栏操作: $header", headerValid)
        val samples = sample.getJSONArray("samples")
        for (index in 0 until samples.length()) {
            val text = samples.getJSONObject(index)
            assertTrue("生产首页文字必须存在", text.getBoolean("found"))
            assertFalse("系统放大后文字不能被自身容器截断: $text", text.getBoolean("clipped"))
            assertTrue("必须测到实际文字 Range", text.getJSONObject("glyph").getDouble("height") > 0)
        }
    }

    private fun captureContent(scenario: ActivityScenario<MainActivity>, stage: String,
                               selector: String, text: String) {
        val checkToast = stage == "object-editor-dark" || (!overlayChecks && !keyboardChecks && !previewChecks && stage == "object-actions-dark") ||
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
            // 旧 WebView 相邻正常流边界可相差约 0.000015 CSS px；仅容忍数值舍入，
            // 不以整像素容差放过真正的覆盖。记录交集尺寸供证据复核。
            const overlapWidth=tr&&br?Math.max(0,Math.min(tr.right,br.right)-Math.max(tr.left,br.left)):0;
            const overlapHeight=tr&&br?Math.max(0,Math.min(tr.bottom,br.bottom)-Math.max(tr.top,br.top)):0;
            const ancestors=[];let n=b;while(n&&ancestors.length<8){const s=getComputedStyle(n),r=n.getBoundingClientRect();
              ancestors.push({tag:n.tagName,classes:n.className,position:s.position,transform:s.transform,filter:s.filter,
                backdrop:s.backdropFilter,height:s.height,top:r.top,bottom:r.bottom});n=n.parentElement;}
            return {label,found:!!b,rect:br?{x:br.x,y:br.y,right:br.right,bottom:br.bottom}:null,ancestors,
              visible:!!br&&br.y>=0&&br.bottom<=innerHeight,
              hittable:!!b&&(hit===b||b.contains(hit)),
              overlapWidth,overlapHeight,overlapTolerance:0.001,
              overlapped:overlapWidth>0.001&&overlapHeight>0.001};
          });
          return JSON.stringify({path:location.pathname,search:location.search,theme:document.documentElement.dataset.theme,
            historyIndex:history.state?.idx,historyLength:history.length,overlayMarker:history.state?.solosoulOverlayLayer===true,
            text:e?.textContent,visible:!!r&&r.height>0&&s.display!=='none'&&s.visibility!=='hidden',
            settled:!!e&&e.getAnimations().every(a=>a.playState!=='running'),
            background:s?.backgroundColor,foreground:s?.color,backdrop:s?.backdropFilter,
            rect:r?{x:r.x,y:r.y,right:r.right,bottom:r.bottom}:null,
            toastCount:document.querySelectorAll('[data-toast-container]').length,
            toastRect:tr?{x:tr.x,y:tr.y,right:tr.right,bottom:tr.bottom,width:tr.width,height:tr.height}:null,
            contentAnimations:e?[...e.getAnimations({subtree:true})].map(a=>({state:a.playState,target:a.effect?.target?.className})):[],
            backupReminder:!!toast&&[...toast.querySelectorAll('button')].some(b=>b.textContent.trim()==='Back Up Now'),
            toastTrace:window.fe2ToastTrace||[],
            toastInFlow:!!toast&&getComputedStyle(toast).position==='static',
            toastInSheet:!!toast?.closest('.android-sheet'),toastInPanel:!!e?.contains(toast),
            toastBackdrop:toast?.closest('[data-macos-glass-backdrop]')?.style.zIndex,
            toastVisible:!!tr&&tr.top>=0&&tr.bottom<=innerHeight,
            toastHittable:!!tr&&toast.contains(document.elementFromPoint(tr.x+tr.width/2,tr.y+tr.height/2)),controls,
            viewportWidth:innerWidth,viewportHeight:innerHeight,overflow:document.documentElement.scrollWidth>innerWidth});
        })()""".trimIndent()
        if (requestedFontScale != null && stage == "object-actions-dark") {
            waitFor(scenario, "$stage 滚动前", expression) { it.optBoolean("visible") && it.optBoolean("settled") }
            scrollTypographyIntoView(scenario, ".android-sheet .android-create-options", ".android-sheet")
        }
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
                if (control.getBoolean("overlapped")) {
                    record(JSONObject(result.toString()).put("stage","failure-content-geometry").put("failedStage",stage))
                }
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
        // 操作随长表单滚动；保持真实提醒和最终命中门槛，不要求常驻屏幕。
        js(scenario, """(() => {const e=[...document.querySelectorAll('button')].find(e=>e.textContent.trim()==='Save');
          e?.scrollIntoView({block:'center'});return JSON.stringify({scrolled:!!e});})()""")
        captureContent(scenario, "object-editor-dark", "[data-testid='object-save-destination']", "Travel")
        if (keyboardChecks) verifyKeyboard(scenario)
        clickText(scenario, "Save")
        waitFor(scenario, "保存后首页数量", home) { it.optBoolean("home") && it.optString("count") == "1" }
        val category = js(scenario, """(() => {const b=[...document.querySelectorAll('.android-category button')].find(e=>e.querySelector('strong')?.textContent==='Travel');b?.click();return JSON.stringify({clicked:!!b})})()""")
        assertTrue(category.getBoolean("clicked"))
        captureContent(scenario, "object-list-dark", "[data-testid='workspace-object-card']", "FE2 public object")
        // 放大字号多一次真实滑动；先验菜单，避免详情截帧消耗同一 8 秒提醒。
        // 提醒计时与门槛不变；普通 baseline 的原有顺序保持不变。
        if (requestedFontScale != null) captureObjectActionsAndBack(scenario)
        clickSelector(scenario, "[data-testid='workspace-object-card'] .android-row-button")
        captureContent(scenario, "object-detail-dark", "[data-testid='object-detail-modal']", "FE2 public country")
        if (overlayChecks) verifyNestedOverlays(scenario)
        if (previewChecks) verifyNonemptyPreviews(scenario, true, true)
        val back = js(scenario, """JSON.stringify({index:history.state?.idx,marker:history.state?.solosoulOverlayLayer===true})""")
        assertTrue("详情必须有可返回的浮层历史标记", back.getInt("index") > 0 && back.getBoolean("marker"))
        // 注入 Android 实际返回键，不直接关闭组件或修改 history/state。
        instrumentation.sendKeyDownUpSync(KeyEvent.KEYCODE_BACK)
        waitFor(scenario, "详情系统返回保留对象页面", """JSON.stringify({path:location.pathname,search:location.search,detail:!!document.querySelector('[data-testid="object-detail-modal"]'),card:!!document.querySelector('[data-testid="workspace-object-card"]')})""") {
            it.optString("path") == "/workspace" && it.optString("search").contains("section=travel") &&
                !it.optBoolean("detail") && it.optBoolean("card")
        }
        if (requestedFontScale == null) captureObjectActionsAndBack(scenario)
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
        if (previewChecks) {
            val travel = js(scenario, """(() => {const b=[...document.querySelectorAll('.android-category button')].find(b=>b.querySelector('strong')?.textContent==='Travel');b?.click();return JSON.stringify({clicked:!!b})})()""")
            assertTrue("点击浅色首页的真实分类入口",travel.getBoolean("clicked"))
            waitFor(scenario, "浅色对象列表真实入口已加载", """(() => {
              const b=document.querySelector('[data-testid="workspace-object-card"] .android-row-button'),r=b?.getBoundingClientRect();
              const hit=r?document.elementFromPoint(r.x+r.width/2,r.y+r.height/2):null;
              return JSON.stringify({path:location.pathname,search:location.search,text:b?.textContent,
                visible:!!r&&r.height>0,hittable:!!b&&(hit===b||b.contains(hit))});
            })()""") {
                it.optString("path") == "/workspace" && it.optString("search").contains("section=travel") &&
                    it.optString("text").contains("FE2 public object") && it.optBoolean("visible") && it.optBoolean("hittable")
            }
            clickSelector(scenario, "[data-testid='workspace-object-card'] .android-row-button")
            waitFor(scenario, "浅色对象详情", page) { it.optString("text").contains("FE2 public country") }
            verifyNonemptyPreviews(scenario, false, false)
        }
    }

    private fun captureObjectActionsAndBack(scenario: ActivityScenario<MainActivity>) {
        clickSelector(scenario, "[data-testid='workspace-object-card'] button[aria-haspopup='dialog']")
        captureContent(scenario, "object-actions-dark", ".android-sheet[role='dialog']", "Attachments")
        instrumentation.sendKeyDownUpSync(KeyEvent.KEYCODE_BACK)
        waitFor(scenario, "操作菜单系统返回保留对象页面", """JSON.stringify({path:location.pathname,search:location.search,sheet:!!document.querySelector('.android-sheet'),card:!!document.querySelector('[data-testid="workspace-object-card"]')})""") {
            it.optString("path") == "/workspace" && it.optString("search").contains("section=travel") &&
                !it.optBoolean("sheet") && it.optBoolean("card")
        }
    }

    /** 公开夹具走真实导入/保存 IPC；不替换读取、解密或 UI，不声称文件选择器已验。 */
    private fun seedPreviewFixtures(scenario: ActivityScenario<MainActivity>) {
        val context = instrumentation.targetContext
        val dir = File(context.cacheDir, "fe2-preview-public").also { check(it.mkdir()) }
        val image = File(dir, "FE2-public-quadrants.png")
        val bitmap = Bitmap.createBitmap(320, 240, Bitmap.Config.ARGB_8888)
        val colors = intArrayOf(0xffe43d30.toInt(), 0xff24b35e.toInt(), 0xff286be0.toInt(), 0xffe9b52e.toInt())
        for (y in 0 until 240) for (x in 0 until 320) bitmap.setPixel(x, y, colors[(if (y < 120) 0 else 2) + (if (x < 160) 0 else 1)])
        image.outputStream().use { assertTrue(bitmap.compress(Bitmap.CompressFormat.PNG, 100, it)) }
        bitmap.recycle()
        val text = File(dir, "FE2-public-notes.txt").also { it.writeText("FE2 public preview text\nShared neutral surfaces\nNo private data\n") }
        if(safPickerChecks) { seedActualSafFixtures(scenario,listOf(text to "text/plain",image to "image/png"));return }
        val fixtures = JSONArray()
        for ((file, mime) in listOf(image to "image/png", text to "text/plain")) {
            val uri = FileProvider.getUriForFile(context, "com.solosoul.app.fileprovider", file)
            fixtures.put(JSONObject().put("uri", uri.toString()).put("name", file.name).put("mime", mime))
        }
        js(scenario, """(() => {window.fe2PreviewSeed={pending:true};(async()=>{
          const invoke=window.__TAURI_INTERNALS__.invoke;
          const accounts=await invoke('vault_list_accounts',{});
          if(accounts.length!==1||accounts[0].name!=='FE2 public visual test')throw Error('not owned synthetic account');
          const objects=await invoke('object_list',{accountId:accounts[0].id,filter:null});
          if(objects.length!==1||objects[0].name!=='FE2 public object')throw Error('not owned synthetic object');
          const objectId=objects[0].id,rows=[];
          for(const f of $fixtures){const id=crypto.randomUUID();
            const imported=await invoke('attachment_import_content_uri',{objectId,attachmentId:id,contentUri:f.uri,fileName:f.name});
            if(imported.displayName!==f.name||imported.sizeBytes<=0)throw Error('wrong imported fixture');
            await invoke('attachment_save',{objectId,meta:{id,objectId,fileName:f.name,mimeType:f.mime,
              sizeBytes:imported.sizeBytes,createdAt:new Date().toISOString(),srcPath:f.uri,vaultPath:imported.vaultPath}});
            rows.push({name:f.name,mime:f.mime,path:imported.vaultPath,size:imported.sizeBytes});}
          const actual=await invoke('attachment_list',{objectId,showDeleted:false});
          if(actual.length!==2)throw Error('wrong saved attachment count');
          window.fe2PreviewSeed={objectId,rows,savedCount:actual.length};
        })().catch(e=>window.fe2PreviewSeed={error:String(e)});return JSON.stringify({started:true});})()""")
        val result = waitFor(scenario, "加密附件夹具", "JSON.stringify(window.fe2PreviewSeed||{})") { it.has("rows") || it.has("error") }
        assertFalse(result.toString(), result.has("error"))
        for (index in 0 until result.getJSONArray("rows").length()) {
            val file = File(result.getJSONArray("rows").getJSONObject(index).getString("path"))
            val magic = ByteArray(4)
            file.inputStream().use { assertEquals(4, it.read(magic)) }
            assertEquals("夹具必须由真实 Vault 加密落盘", "SOLC", String(magic, Charsets.US_ASCII))
        }
        record(JSONObject().put("stage", "preview-fixtures-encrypted").put("savedCount", 2)
            .put("encrypted", true).put("source", "owned private cache via production FileProvider/content URI/import/save IPC"))
    }

    private fun capturePreview(scenario: ActivityScenario<MainActivity>, stage: String, selector: String,
                               dark: Boolean, kind: String) {
        val notificationKind = when {
            stage.startsWith("photo-viewer-") -> "saved"
            !safPickerChecks && dark && !stage.contains("after-back") -> "backup"
            else -> "none"
        }
        val expression = """(() => {
          const e=document.querySelector(${JSONObject.quote(selector)}),r=e?.getBoundingClientRect(),s=e?getComputedStyle(e):null;
          const rect=e=>{const r=e?.getBoundingClientRect();return r?{x:r.x,y:r.y,right:r.right,bottom:r.bottom,width:r.width,height:r.height}:null};
          const visible=e=>{if(!e)return false;for(let n=e;n;n=n.parentElement){const s=getComputedStyle(n);if(s.visibility==='hidden'||s.display==='none'||Number(s.opacity)<.99)return false}const r=e.getBoundingClientRect();return r.width>0&&r.height>0};
          const img=[...(e?.querySelectorAll('img')||[])].find(visible),pre=e?.querySelector('pre'),zoom=e?.querySelector('[data-preview-zoom-controls]');
          const toast=document.querySelector('[data-toast-container]'),tr=toast?.getBoundingClientRect();
          const header=e?.querySelector('[data-preview-titlebar]');
          const luminance=color=>{const values=color.match(/[\d.]+/g)?.slice(0,3).map(Number);if(!values||values.length!==3)throw Error('Invalid preview color');return values.reduce((sum,n,i)=>{const c=n/255;return sum+(c<=.04045?c/12.92:Math.pow((c+.055)/1.055,2.4))*[.2126,.7152,.0722][i]},0)};
          const headerBackground=header?getComputedStyle(header).backgroundColor:null;
          const headerForegrounds=header?[...header.querySelectorAll('button,span')].filter(visible).map(n=>{
            const color=getComputedStyle(n).color,background=luminance(headerBackground),foreground=luminance(color);
            return {label:n.getAttribute('aria-label')||n.title||n.textContent.trim(),rect:rect(n),color,contrast:(Math.max(background,foreground)+.05)/(Math.min(background,foreground)+.05)};
          }):[];
          const controlScope = ${if (kind == "attachments") "e?.firstElementChild" else "e"};
          const controls=[...(controlScope?.querySelectorAll(${JSONObject.quote(if (kind == "attachments") "button" else "[data-preview-titlebar] button,[data-preview-zoom-controls] button")})||[])].filter(b=>!b.disabled&&b.getBoundingClientRect().height>0&&getComputedStyle(b).visibility!=='hidden').map(b=>{
            const r=b.getBoundingClientRect(),hit=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);
            return {label:b.getAttribute('aria-label')||b.title||b.textContent.trim(),rect:rect(b),hittable:hit===b||b.contains(hit),overlapped:!!tr&&tr.left<r.right&&tr.right>r.left&&tr.top<r.bottom&&tr.bottom>r.top};});
          return JSON.stringify({theme:document.documentElement.dataset.theme,path:location.pathname,search:location.search,
            visible:!!r&&r.height>0&&s.visibility!=='hidden',settled:!!e&&e.getAnimations({subtree:true}).every(a=>a.playState!=='running'),rect:rect(e),viewportWidth:innerWidth,viewportHeight:innerHeight,
            overflow:document.documentElement.scrollWidth>innerWidth,controls,headerBackground,headerForegrounds,
            toastCount:document.querySelectorAll('[data-toast-container]').length,toastRect:rect(toast),
            savedNotification:!!toast&&[...toast.children].some(n=>n.firstElementChild?.textContent.trim()==='Saved'),
            backupReminder:!!toast&&[...toast.querySelectorAll('button')].some(b=>b.textContent.trim()==='Back Up Now'),
            toastInPanel:!!e?.contains(toast),toastInFlow:!!toast&&getComputedStyle(toast).position==='static',
            toastVisible:!!tr&&tr.top>=0&&tr.bottom<=innerHeight,
            toastHittable:!!tr&&toast.contains(document.elementFromPoint(tr.x+tr.width/2,tr.y+tr.height/2)),
            text:e?.textContent,preText:pre?.textContent,textRect:rect(pre),textColor:pre?getComputedStyle(pre).color:null,
            imageDecoded:!!img&&img.complete&&img.naturalWidth>0&&img.naturalHeight>0,imageNaturalWidth:img?.naturalWidth,imageNaturalHeight:img?.naturalHeight,imageRect:rect(img),
            imageCount:e?.querySelectorAll('img').length,zoomText:zoom?.textContent,zoomRect:rect(zoom),
            counter:e?.querySelector('[data-testid="photo-viewer-counter"]')?.textContent});})()"""
        var previousImageGeometry: String? = null
        val result = waitFor(scenario, stage, expression) {
            val geometry = it.optJSONObject("imageRect")?.toString()
            val stable = geometry != null && geometry == previousImageGeometry
            previousImageGeometry = geometry
            (kind !in listOf("image","viewer") || stable) && it.optBoolean("visible") && it.optBoolean("settled") && it.optString("theme") == (if (dark) "dark" else "light") &&
                (if (kind == "text") it.optString("preText").contains("FE2 public preview text") else if (kind in listOf("image", "album", "viewer")) it.optBoolean("imageDecoded") else it.optString("text").contains("FE2-public-notes.txt"))
        }
        assertFalse(result.toString(), result.getBoolean("overflow"))
        val native = nativeViewport(scenario)
        val scale = native.getInt("webViewWidthPx") / result.getDouble("viewportWidth")
        val bars = native.getJSONObject("systemBars")
        val controls = result.getJSONArray("controls")
        assertTrue("顶栏返回/关闭操作必须存在", controls.length() >= 2)
        for (index in 0 until controls.length()) {
            val control = controls.getJSONObject(index); val r = control.getJSONObject("rect")
            assertTrue("真实操作不得被遮挡: $control", control.getBoolean("hittable"))
            assertFalse("真实提醒不得盖住预览操作",control.getBoolean("overlapped"))
            assertTrue("操作不得超出安全区: $control", r.getDouble("x") >= 0 && r.getDouble("right") <= result.getDouble("viewportWidth") + 1 &&
                native.getInt("webViewScreenY") + r.getDouble("y") * scale >= bars.getInt("top") - 1 &&
                native.getInt("webViewScreenY") + r.getDouble("bottom") * scale <= native.getInt("screenHeightPx") - bars.getInt("bottom") + 1)
        }
        if (kind == "viewer") assertEquals("1 / 1", result.getString("counter").trim())
        if (kind == "image" || kind == "viewer") assertTrue("图片缩放工具必须有可见比例", result.optString("zoomText").contains("%"))
        val frame = screenshot(scenario)
        // 一次批量读取元素区域像素，避免逐像素 getPixel 的 JNI 往返侵占真实提醒时限。
        // 区域、颜色误差与像素门槛保持原样；不改变通知计时或截帧等待。
        fun countInk(rect: JSONObject, color: String): Int {
            val ink = Regex("\\d+").findAll(color).take(3).map { it.value.toInt() }.toList()
            assertEquals(3, ink.size)
            val left = (native.getInt("webViewScreenX") + rect.getDouble("x") * scale).toInt().coerceIn(0,frame.width-1)
            val top = (native.getInt("webViewScreenY") + rect.getDouble("y") * scale).toInt().coerceIn(0,frame.height-1)
            val right = (native.getInt("webViewScreenX") + rect.getDouble("right") * scale).toInt().coerceIn(left+1,frame.width)
            val bottom = (native.getInt("webViewScreenY") + rect.getDouble("bottom") * scale).toInt().coerceIn(top+1,frame.height)
            val width = right-left
            val pixels = IntArray(width*(bottom-top))
            frame.getPixels(pixels,0,width,left,top,width,bottom-top)
            return pixels.count { pixel ->
                kotlin.math.abs(android.graphics.Color.red(pixel)-ink[0])<=8 &&
                    kotlin.math.abs(android.graphics.Color.green(pixel)-ink[1])<=8 &&
                    kotlin.math.abs(android.graphics.Color.blue(pixel)-ink[2])<=8
            }
        }
        if (kind != "attachments") {
            val foregrounds = result.getJSONArray("headerForegrounds")
            assertTrue("预览标题与按钮必须存在", foregrounds.length() >= 3)
            for (index in 0 until foregrounds.length()) {
                val item = foregrounds.getJSONObject(index)
                assertTrue("预览标题/图标对比不足: $item", item.getDouble("contrast") >= 4.5)
                val count = countInk(item.getJSONObject("rect"),item.getString("color"))
                assertTrue("预览标题/图标须在最终帧实际绘制: $item", count>=6)
                item.put("inkPixels",count)
            }
        }
        if (kind == "text") {
            val count = countInk(result.getJSONObject("textRect"),result.getString("textColor"))
            assertTrue("解密文本必须在最终合成帧中有实际文字像素", count>=20)
            result.put("textInkPixels",count)
        }
        if (kind == "image" || kind == "viewer") {
            val r = result.getJSONObject("imageRect")
            val expected = arrayOf(intArrayOf(228,61,48),intArrayOf(36,179,94),intArrayOf(40,107,224),intArrayOf(233,181,46))
            val pixels = JSONArray()
            for (index in 0..3) {
                val x = (native.getInt("webViewScreenX") + (r.getDouble("x") + r.getDouble("width") * (if (index % 2 == 0) .25 else .75)) * scale).toInt()
                val y = (native.getInt("webViewScreenY") + (r.getDouble("y") + r.getDouble("height") * (if (index < 2) .25 else .75)) * scale).toInt()
                assertTrue("图片采样在实际截图内", x in 0 until frame.width && y in 0 until frame.height)
                val pixel = frame.getPixel(x,y)
                val actual = intArrayOf(android.graphics.Color.red(pixel),android.graphics.Color.green(pixel),android.graphics.Color.blue(pixel))
                pixels.put(JSONArray(actual.toList()))
                if (!(0..2).all { kotlin.math.abs(actual[it]-expected[index][it]) <= 12 }) {
                    record(result.put("stage","failure-preview-probe").put("failedStage",stage).put("sampleX",x).put("sampleY",y).put("sample",JSONArray(actual.toList())))
                }
                assertTrue("解密图片须出现在最终合成帧: ${actual.toList()}", (0..2).all { kotlin.math.abs(actual[it]-expected[index][it]) <= 12 })
            }
            result.put("quadrantPixels", pixels)
        }
        val reminderStillPresent = js(scenario,"JSON.stringify({backup:[...document.querySelectorAll('[data-toast-container] button')].some(b=>b.textContent.trim()==='Back Up Now'),saved:[...document.querySelectorAll('[data-toast-container] > div')].some(n=>n.firstElementChild?.textContent.trim()==='Saved')})")
        val present = notificationKind=="none" || reminderStillPresent.getBoolean(if(notificationKind=="backup") "backup" else "saved")
        if (notificationKind!="none") assertTrue("最终截图时真实通知必须仍存在",present)
        result.put("notificationKind",notificationKind).put("notificationPresentAtCapture",present)
        File(evidence,"$stage.png").outputStream().use { frame.compress(Bitmap.CompressFormat.PNG,100,it) }
        frame.recycle()
        val previewBars = accountBars(scenario)
        assertEquals("预览状态栏图标须与当前顶栏明暗一致", !dark, previewBars.getBoolean("statusBarLight"))
        assertEquals("预览导航栏图标须与安全区底色一致", !dark, previewBars.getBoolean("navigationBarLight"))
        result.put("stage",stage).put("kind",kind).put("nativeViewport",native).put("systemBars",previewBars)
        record(result)
    }

    private fun verifyNonemptyPreviews(scenario: ActivityScenario<MainActivity>, dark: Boolean, seed: Boolean) {
        if (seed) seedPreviewFixtures(scenario)
        val suffix = if (dark) "dark" else "light"
        val panel = "[data-macos-glass-backdrop][style*='z-index: 5100'] > [data-macos-glass='panel']"
        clickSelector(scenario, ".android-object-detail-footer button[aria-label^='Attachments']")
        capturePreview(scenario,"nonempty-attachments-$suffix",panel,dark,"attachments")
        for ((file,kind) in listOf("FE2-public-notes.txt" to "text", "FE2-public-quadrants.png" to "image")) {
            clickSelector(scenario,"button[aria-haspopup='dialog'][aria-label*='$file']")
            clickText(scenario,"Preview")
            capturePreview(scenario,"file-preview-$kind-$suffix","[data-testid='attachment-preview-overlay']",dark,kind)
            clickSelector(scenario,"[data-testid='attachment-preview-overlay'] button[aria-label='Back']")
            waitFor(scenario,"文件返回非空附件", "JSON.stringify({preview:!!document.querySelector('[data-testid=attachment-preview-overlay]')})") { !it.optBoolean("preview") }
        }
        val album = js(scenario, """(() => {const b=[...document.querySelectorAll('button')].find(b=>[...b.querySelectorAll('span')].some(s=>s.textContent.trim()==='Photo Album'));b?.click();return JSON.stringify({clicked:!!b})})()""")
        assertTrue("点击带数量的真实相册入口",album.getBoolean("clicked"))
        capturePreview(scenario,"photo-album-$suffix","[data-testid='photo-album-overlay']",dark,"album")
        clickSelector(scenario,"[data-testid='photo-album-grid'] [role='button'][aria-label='FE2-public-quadrants.png']")
        waitFor(scenario,"懒加载查看器实际就绪", "JSON.stringify({ready:[...document.querySelectorAll('[data-testid=photo-viewer] button')].some(b=>b.title==='Edit Attachment Attributes'),decoded:[...document.querySelectorAll('[data-testid=photo-viewer] img')].some(i=>i.complete&&i.naturalWidth===320)})") { it.optBoolean("ready") && it.optBoolean("decoded") }
        // 原提醒自然到期；真实编辑公开描述，由保存操作产生新通知，不修改计时。
        clickSelector(scenario,"[data-testid='photo-viewer'] button[title='Edit Attachment Attributes']")
        waitFor(scenario,"真实附件属性编辑", "JSON.stringify({editor:!!document.querySelector('[role=dialog] textarea')})") { it.optBoolean("editor") }
        val edited = js(scenario, """(() => {const e=document.querySelector('[role=dialog] textarea');if(!e)return JSON.stringify({edited:false});
          Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype,'value').set.call(e,'FE2 public preview metadata $suffix');
          e.dispatchEvent(new Event('input',{bubbles:true}));return JSON.stringify({edited:true});})()""")
        assertTrue(edited.getBoolean("edited"))
        clickText(scenario,"Save")
        waitFor(scenario,"真实保存后保留查看器", "JSON.stringify({editor:!!document.querySelector('[role=dialog] textarea'),viewer:!!document.querySelector('[data-testid=photo-viewer]'),saved:[...document.querySelectorAll('[data-toast-container] > div')].some(n=>n.firstElementChild?.textContent.trim()==='Saved')})") { !it.optBoolean("editor") && it.optBoolean("viewer") && it.optBoolean("saved") }
        capturePreview(scenario,"photo-viewer-$suffix","[data-testid='photo-viewer']",dark,"viewer")
        instrumentation.sendKeyDownUpSync(KeyEvent.KEYCODE_BACK)
        waitFor(scenario,"查看器系统返回相册", "JSON.stringify({viewer:!!document.querySelector('[data-testid=photo-viewer]'),album:!!document.querySelector('[data-testid=photo-album-overlay]')})") { !it.optBoolean("viewer") && it.optBoolean("album") }
        capturePreview(scenario,"photo-album-after-back-$suffix","[data-testid='photo-album-overlay']",dark,"album")
        instrumentation.sendKeyDownUpSync(KeyEvent.KEYCODE_BACK)
        waitFor(scenario,"相册系统返回附件", "JSON.stringify({album:!!document.querySelector('[data-testid=photo-album-overlay]'),panel:!!document.querySelector(${JSONObject.quote(panel)})})") { !it.optBoolean("album") && it.optBoolean("panel") }
        capturePreview(scenario,"attachments-after-back-$suffix",panel,dark,"attachments")
        instrumentation.sendKeyDownUpSync(KeyEvent.KEYCODE_BACK)
        val returned = waitFor(scenario,"附件系统返回对象详情", "JSON.stringify({path:location.pathname,search:location.search,detail:!!document.querySelector('[data-testid=object-detail-modal]'),panel:!!document.querySelector(${JSONObject.quote(panel)})})") { !it.optBoolean("panel") && it.optBoolean("detail") }
        val bars = accountBars(scenario)
        assertEquals(!dark,bars.getBoolean("statusBarLight"))
        record(returned.put("stage","preview-returned-detail-$suffix").put("theme",if(dark) "dark" else "light").put("systemBars",bars))
    }

    private fun nativeViewport(scenario: ActivityScenario<MainActivity>): JSONObject {
        var result = JSONObject()
        scenario.onActivity { activity ->
            val view = requireNotNull(webView(activity.window.decorView))
            val insets = requireNotNull(ViewCompat.getRootWindowInsets(view))
            val location = IntArray(2)
            view.getLocationOnScreen(location)
            val bars = insets.getInsets(WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout())
            val bounds = if (android.os.Build.VERSION.SDK_INT >= 30) activity.windowManager.currentWindowMetrics.bounds
                else android.graphics.Rect(0, 0, activity.resources.displayMetrics.widthPixels, activity.resources.displayMetrics.heightPixels)
            result = JSONObject().put("imeVisible", insets.isVisible(WindowInsetsCompat.Type.ime()))
                .put("documentStartScriptSupported", androidx.webkit.WebViewFeature.isFeatureSupported(androidx.webkit.WebViewFeature.DOCUMENT_START_SCRIPT))
                .put("imeHeightPx", insets.getInsets(WindowInsetsCompat.Type.ime()).bottom)
                .put("screenHeightPx", activity.window.decorView.rootView.height)
                .put("webViewWidthPx", view.width).put("webViewHeightPx", view.height)
                .put("webViewScreenX", location[0]).put("webViewScreenY", location[1])
                .put("systemBars", JSONObject().put("top",bars.top).put("right",bars.right).put("bottom",bars.bottom).put("left",bars.left))
                .put("windowBounds", JSONObject().put("top",bounds.top).put("right",bounds.right).put("bottom",bounds.bottom).put("left",bounds.left))
        }
        return result
    }

    private fun captureInsets(scenario: ActivityScenario<MainActivity>, stage: String, login: Boolean, ime: Boolean = false): JSONObject {
        val selector = if (login) "[data-login-method-region='password'] input, [data-login-password-submit]"
            else ".android-appbar-leading, .android-appbar-actions button, .android-navigation a, [data-shell-notifications] button"
        val expression = """(() => {const nodes=[...document.querySelectorAll(${JSONObject.quote(selector)})].filter(e=>
          !e.closest('[inert]')&&getComputedStyle(e).visibility!=='hidden'&&getComputedStyle(e).display!=='none'&&e.getClientRects().length>0);
          return JSON.stringify({path:location.pathname,width:innerWidth,height:innerHeight,
            home:!!document.querySelector('[data-testid="android-home"]'),login:!!document.querySelector('[data-login-card]'),
            historyIndex:history.state?.idx,overflow:document.documentElement.scrollWidth>innerWidth,
            glass:document.documentElement.dataset.androidGlass,
            backupReminder:[...document.querySelectorAll('[data-shell-notifications] button')].some(b=>b.textContent.trim()==='Back Up Now'),
            targets:nodes.map(e=>{const r=e.getBoundingClientRect(),h=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);
              return {tag:e.tagName,notification:!!e.closest('[data-shell-notifications]'),rect:{x:r.x,y:r.y,right:r.right,bottom:r.bottom},hittable:e===h||e.contains(h)}})});})()"""
        waitFor(scenario,stage,expression) { it.optBoolean(if (login) "login" else "home") && it.getJSONArray("targets").length() >= (if (login) 2 else 8) }
        // 旋转 / IME 后等待同一生产文档提交绘制，再采其只读几何，避免记录上一帧。
        val bitmap=screenshot(scenario)
        val result=js(scenario,expression)
        val native = nativeViewport(scenario)
        val bars = native.getJSONObject("systemBars")
        val bounds = native.getJSONObject("windowBounds")
        val scale = native.getDouble("webViewWidthPx") / result.getDouble("width")
        val limits = JSONObject().put("left",bounds.getInt("left")+bars.getInt("left"))
            .put("right",bounds.getInt("right")-bars.getInt("right"))
            .put("top",bounds.getInt("top")+bars.getInt("top"))
            .put("bottom",bounds.getInt("bottom")-maxOf(bars.getInt("bottom"),if (ime) native.getInt("imeHeightPx") else 0))
        result.put("stage",stage).put("nativeViewport",native).put("safeBoundsOnScreen",limits)
        File(evidence,"$stage.png").outputStream().use { bitmap.compress(Bitmap.CompressFormat.PNG,100,it) };bitmap.recycle()
        record(result)
        if (stage == "safe-home-landscape") {
            assertTrue("横屏必须包含真实限时备份提醒，不能靠提醒到期跳过通知安全区", result.getBoolean("backupReminder"))
            val notificationTargets = result.getJSONArray("targets")
            assertTrue("横屏必须采集提醒操作和关闭按钮", (0 until notificationTargets.length()).count {
                notificationTargets.getJSONObject(it).optBoolean("notification")
            } >= 2)
        }
        assertEquals("必须采真实 IME 状态",ime,native.getBoolean("imeVisible"))
        assertFalse("安全区适配不能撑宽页面",result.getBoolean("overflow"))
        for (index in 0 until result.getJSONArray("targets").length()) {
            val control=result.getJSONArray("targets").getJSONObject(index);val rect=control.getJSONObject("rect")
            assertTrue("控件不能被遮挡: $stage / $control",control.getBoolean("hittable"))
            assertTrue("控件必须避开实际系统栏、缺口和键盘: $stage / $control / $native",
                native.getDouble("webViewScreenX")+rect.getDouble("x")*scale >= limits.getDouble("left")-1 &&
                native.getDouble("webViewScreenX")+rect.getDouble("right")*scale <= limits.getDouble("right")+1 &&
                native.getDouble("webViewScreenY")+rect.getDouble("y")*scale >= limits.getDouble("top")-1 &&
                native.getDouble("webViewScreenY")+rect.getDouble("bottom")*scale <= limits.getDouble("bottom")+1)
        }
        return result
    }

    private fun unlockPublicVisualAccount(scenario: ActivityScenario<MainActivity>) {
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
    }

    private fun verifyInsets(scenario: ActivityScenario<MainActivity>) {
        // 公开合成账户的首次解锁提醒已在前面的对象场景自然到期。
        // 只重置其加密提醒时间，再经真实锁定/密码解锁触发生产的8秒提醒；
        // 不注入Toast、不修改duration，也不替换通知或登录处理器。
        js(scenario, """(() => {window.fe2InsetsReminder={pending:true};
          const invoke=window.__TAURI_INTERNALS__.invoke;
          invoke('vault_list_accounts',{}).then(async accounts=>{
            if(accounts.length!==1||accounts[0].name!=='FE2 public visual test')throw Error('not isolated public account');
            await invoke('user_data_update_preference',{payload:{accountId:accounts[0].id,preferences:{lastBackupReminderAt:0}}});
            const prefs=await invoke('user_data_get_preferences',{accountId:accounts[0].id});
            if(prefs.lastBackupReminderAt!==0)throw Error('reminder fixture not persisted');
            window.fe2InsetsReminder={persisted:true};
          }).catch(error=>window.fe2InsetsReminder={error:String(error)});
          return JSON.stringify({started:true});})()""")
        val fixture=waitFor(scenario,"仅公开账户提醒夹具落盘","JSON.stringify(window.fe2InsetsReminder||{})") {
            it.optBoolean("persisted") || it.has("error")
        }
        assertFalse("公开提醒夹具失败: $fixture",fixture.has("error"))
        lockToPassword(scenario)
        unlockPublicVisualAccount(scenario)
        waitFor(scenario,"新一轮真实限时提醒","JSON.stringify({present:[...document.querySelectorAll('[data-shell-notifications] button')].some(b=>b.textContent.trim()==='Back Up Now')})") {
            it.optBoolean("present")
        }
        val portrait = captureInsets(scenario,"safe-home-portrait",login=false)
        var original = android.content.pm.ActivityInfo.SCREEN_ORIENTATION_UNSPECIFIED
        scenario.onActivity { activity -> original=activity.requestedOrientation;activity.requestedOrientation=android.content.pm.ActivityInfo.SCREEN_ORIENTATION_LANDSCAPE }
        try {
            waitFor(scenario,"横屏完成","JSON.stringify({landscape:innerWidth>innerHeight})") { it.optBoolean("landscape") }
            captureInsets(scenario,"safe-home-landscape",login=false)
            scenario.onActivity { it.requestedOrientation=android.content.pm.ActivityInfo.SCREEN_ORIENTATION_PORTRAIT }
            waitFor(scenario,"竖屏恢复","JSON.stringify({portrait:innerWidth<innerHeight})") { it.optBoolean("portrait") }
            captureInsets(scenario,"safe-home-after-rotation",login=false)
        } finally { scenario.onActivity { it.requestedOrientation=original } }
        lockToPassword(scenario)
        val login = captureInsets(scenario,"safe-login-portrait",login=true)
        val target=js(scenario,"""(() => {const e=document.querySelector('[data-login-method-region="password"] input');
          e.scrollIntoView({block:'center'});const r=e.getBoundingClientRect();return JSON.stringify({x:r.x+r.width/2,y:r.y+r.height/2});})()""")
        val native=nativeViewport(scenario);val scale=native.getDouble("webViewWidthPx")/login.getDouble("width")
        val x=(native.getDouble("webViewScreenX")+target.getDouble("x")*scale).toFloat()
        val y=(native.getDouble("webViewScreenY")+target.getDouble("y")*scale).toFloat();val downTime=SystemClock.uptimeMillis()
        for (action in listOf(MotionEvent.ACTION_DOWN,MotionEvent.ACTION_UP)) {
            val event=MotionEvent.obtain(downTime,SystemClock.uptimeMillis(),action,x,y,0);instrumentation.sendPointerSync(event);event.recycle()
        }
        waitForIme(scenario,true)
        captureInsets(scenario,"safe-login-keyboard",login=true,ime=true)
        instrumentation.sendKeyDownUpSync(KeyEvent.KEYCODE_BACK)
        waitForIme(scenario,false,restoredHeight=native.getInt("webViewHeightPx"))
        val back=captureInsets(scenario,"safe-login-after-keyboard-back",login=true)
        assertEquals("键盘返回不能离开登录页",login.getInt("historyIndex"),back.getInt("historyIndex"))
        scenario.onActivity { requireNotNull(webView(it.window.decorView)).reload() }
        waitFor(scenario,"新文档登录就绪",page) { it.optBoolean("startupGone") && it.optString("path")=="/login" && it.optJSONArray("buttons")?.toString()?.contains("Unlock") == true }
        val reload=captureInsets(scenario,"safe-login-after-document-reload",login=true)
        assertEquals("重载后仍保留真实窗口高度",portrait.getJSONObject("nativeViewport").getInt("webViewHeightPx"),reload.getJSONObject("nativeViewport").getInt("webViewHeightPx"))
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
                                withReminder: Boolean = true, typedDraft: String? = null,
                                cancel: Boolean = false): JSONObject {
        val result = waitFor(scenario, stage, """(() => {
          const name=document.querySelector('input[aria-label="Object Name"]');
          const save=[...document.querySelectorAll('button')].find(e=>e.textContent.trim()==='Save');
          const cancel=[...document.querySelectorAll('button')].find(e=>e.textContent.trim()==='Cancel');
          const e=${if (input) "name" else if (cancel) "cancel" else "save"},r=e?.getBoundingClientRect(),v=visualViewport;
          const toast=document.querySelector('[data-toast-container]'),tr=toast?.getBoundingClientRect();
          const hit=r?document.elementFromPoint(r.x+r.width/2,r.y+r.height/2):null;
          return JSON.stringify({path:location.pathname,search:location.search,historyIndex:history.state?.idx,
            theme:document.documentElement.dataset.theme,draft:name?.value,focused:document.activeElement===name,
            viewportWidth:innerWidth,viewportHeight:innerHeight,visualHeight:v?.height,visualOffset:v?.offsetTop,
            keyboardInset:getComputedStyle(document.documentElement).getPropertyValue('--android-keyboard-inset'),
            rect:r?{x:r.x,y:r.y,right:r.right,bottom:r.bottom,width:r.width,height:r.height}:null,
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
        if (cancel) {
            assertTrue("取消触控区不能缩小", rect.getDouble("width") >= 48 && rect.getDouble("height") >= 48)
        }
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
        // 新操作栏随正文滚动；分别检查保存和取消，不能以保存可达代替取消。
        js(scenario, """(() => {const e=[...document.querySelectorAll('button')].find(e=>e.textContent.trim()==='Cancel');
          e?.scrollIntoView({block:'center'});return JSON.stringify({scrolled:!!e});})()""")
        captureKeyboard(scenario, "editor-keyboard-cancel-dark", input = false,
            withReminder = false, cancel = true)
        js(scenario, """(() => {const e=[...document.querySelectorAll('button')].find(e=>e.textContent.trim()==='Save');
          e?.scrollIntoView({block:'center'});return JSON.stringify({scrolled:!!e});})()""")
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
        // IME 关闭后系统可能把仍聚焦的输入框带回视口；正文操作需要再次滚动可达。
        js(scenario, """(() => {const e=[...document.querySelectorAll('button')].find(e=>e.textContent.trim()==='Save');
          e?.scrollIntoView({block:'center'});return JSON.stringify({scrolled:!!e});})()""")
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

    private fun lockToPassword(scenario: ActivityScenario<MainActivity>) {
        clickSelector(scenario, "button[aria-label='Lock Vault']")
        waitFor(scenario, "账户切换前锁定", """(() => {const b=document.querySelector('[data-login-password-submit]');
          return JSON.stringify({path:location.pathname,form:!!b,enabled:!!b&&!b.disabled,
            pending:!!document.querySelector('[data-login-method-region="pending"]'),home:!!document.querySelector('[data-testid="android-home"]')});})()""") {
            it.optString("path") == "/login" && it.optBoolean("form") && it.optBoolean("enabled") &&
                !it.optBoolean("pending") && !it.optBoolean("home")
        }
    }

    private fun openAppearance(scenario: ActivityScenario<MainActivity>) {
        clickSelector(scenario, ".android-navigation a[href='/settings']")
        val opened = js(scenario, """(() => {const e=[...document.querySelectorAll('[role="button"]')]
          .find(e=>e.textContent.includes('Theme & Appearance'));e?.click();return JSON.stringify({clicked:!!e});})()""")
        assertTrue("通过真实设置行打开外观", opened.getBoolean("clicked"))
        waitFor(scenario, "账户外观设置就绪", page) { it.optString("path") == "/settings/appearance" }
    }

    private fun chooseValue(scenario: ActivityScenario<MainActivity>, selector: String, value: String) {
        val selected = js(scenario, """(() => {const e=document.querySelector(${JSONObject.quote(selector)});
          const option=e&&[...e.options].find(o=>o.value===${JSONObject.quote(value)});
          if(!option)return JSON.stringify({selected:false});
          Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype,'value').set.call(e,option.value);
          e.dispatchEvent(new Event('change',{bubbles:true}));return JSON.stringify({selected:true});})()""")
        assertTrue("真实选项必须存在: $selector / $value", selected.getBoolean("selected"))
    }

    private fun configureAccountTheme(scenario: ActivityScenario<MainActivity>, dark: Boolean,
                                      scheme: String, accent: String, enhanced: Boolean) {
        openAppearance(scenario)
        clickText(scenario, if (dark) "Dark" else "Light")
        chooseValue(scenario, ".android-scheme-row select[aria-label='${if (dark) "Dark" else "Light"}']", scheme)
        val filled = js(scenario, """(() => {const e=document.querySelector('.android-custom-accent input');
          if(!e)return JSON.stringify({filled:false});
          Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(e,${JSONObject.quote(accent)});
          e.dispatchEvent(new Event('input',{bubbles:true}));return JSON.stringify({filled:true});})()""")
        assertTrue(filled.getBoolean("filled"))
        clickSelector(scenario, ".android-custom-accent button[type='submit']")
        waitFor(scenario, "账户自定义强调色交付", """JSON.stringify({accent:getComputedStyle(document.documentElement).getPropertyValue('--accent-primary').trim(),
          saving:document.querySelector('.android-custom-accent button')?.disabled})""") {
            it.optString("accent") == accent && !it.optBoolean("saving")
        }
        clickText(scenario, if (enhanced) "Enhanced glass" else "Local glass")
        if (motionChecks) {
            // 用真实复选框保存 true / false；不得直接写偏好或注入 Store。
            val selector = "input[type='checkbox'][aria-label='Reduce motion']"
            val state = js(scenario, "JSON.stringify({checked:document.querySelector(${JSONObject.quote(selector)})?.checked})")
            if (state.getBoolean("checked") == dark) {
                clickSelector(scenario, selector)
                waitFor(scenario, "相反动效值先交付", "JSON.stringify({checked:document.querySelector(${JSONObject.quote(selector)})?.checked,reduced:document.documentElement.dataset.userReduceMotion})") {
                    it.getBoolean("checked") == !dark && it.optString("reduced") == (!dark).toString()
                }
            }
            // 即使初始值等于目标，也通过真实切换产生一次显式保存。
            clickSelector(scenario, selector)
            waitFor(scenario, "减少动态效果交付", "JSON.stringify({checked:document.querySelector(${JSONObject.quote(selector)})?.checked,reduced:document.documentElement.dataset.userReduceMotion})") {
                it.getBoolean("checked") == dark && it.optString("reduced") == dark.toString()
            }
        }
        clickSelector(scenario, ".android-navigation a[href='/']")
    }

    private fun readAccountPreferences(scenario: ActivityScenario<MainActivity>, name: String,
                                       expected: JSONObject, accountCount: Int): JSONObject {
        var sampled = JSONObject()
        // UI 已交付仍可能先于偏好落盘；只轮询真实只读 IPC，不替换保存链路。
        repeat(10) {
            js(scenario, """(() => {window.fe2AccountRead={pending:true};
              window.__TAURI_INTERNALS__.invoke('vault_list_accounts',{}).then(accounts=>{
                if(accounts.length!==$accountCount)throw new Error('unexpected account count');
                const account=accounts.find(a=>a.name===${JSONObject.quote(name)});
                if(!account)throw new Error('expected public account missing');
                return window.__TAURI_INTERNALS__.invoke('user_data_get_preferences',{accountId:account.id})
                  .then(prefs=>({accountId:account.id,accountName:account.name,prefs}));
              }).then(value=>window.fe2AccountRead=value,error=>window.fe2AccountRead={error:String(error)});
              return JSON.stringify({started:true});})()""")
            sampled = waitFor(scenario, "账户偏好只读回执", "JSON.stringify(window.fe2AccountRead||{})") { it.has("prefs") || it.has("error") }
            assertFalse("不能绕过账户偏好读取失败: $sampled", sampled.has("error"))
            val prefs = sampled.getJSONObject("prefs")
            if (expected.keys().asSequence().all { key -> prefs.opt(key) == expected.get(key) }) return sampled
            Thread.sleep(100)
        }
        throw AssertionError("账户保存偏好未满足预期: expected=$expected / $sampled")
    }

    private fun accountBars(scenario: ActivityScenario<MainActivity>): JSONObject {
        var result = JSONObject()
        scenario.onActivity { activity ->
            val controller = WindowCompat.getInsetsController(activity.window, activity.window.decorView)
            result = JSONObject().put("statusBarLight", controller.isAppearanceLightStatusBars)
                .put("navigationBarLight", controller.isAppearanceLightNavigationBars)
                .put("nightMode", activity.resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK == Configuration.UI_MODE_NIGHT_YES)
        }
        return result
    }

    private fun captureAccount(scenario: ActivityScenario<MainActivity>, stage: String, name: String,
                               dark: Boolean, enhanced: Boolean, count: String, expectedPrefs: JSONObject,
                               accountCount: Int): JSONObject {
        val background = if (dark) "#1a211d" else "#f6f8fa"
        val surface = if (dark) "#242d28" else "#ffffff"
        val foreground = if (dark) "#d6ddd8" else "#1f2328"
        val expression = """(() => {const state=JSON.parse($home),s=getComputedStyle(document.documentElement);
          const link=document.querySelector('.android-section-heading .android-text-button');
          const rgb=c=>c.startsWith('#')?[1,3,5].map(i=>parseInt(c.slice(i,i+2),16)):c.match(/[\d.]+/g).slice(0,3).map(Number);
          const luminance=c=>rgb(c).reduce((sum,n,i)=>{const v=n/255;return sum+(v<=.04045?v/12.92:((v+.055)/1.055)**2.4)*[.2126,.7152,.0722][i]},0);
          let action=null;if(link){const r=link.getBoundingClientRect(),color=getComputedStyle(link).color;
            const range=document.createRange();range.selectNodeContents(link.firstChild);const t=range.getBoundingClientRect();
            const ink=luminance(color),base=luminance(state.background),hit=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);
            action={color,contrast:(Math.max(ink,base)+.05)/(Math.min(ink,base)+.05),
              hittable:link===hit||link.contains(hit),visible:r.y>=0&&r.bottom<=innerHeight,
              textRect:{x:t.x,y:t.y,right:t.right,bottom:t.bottom},rgb:rgb(color)};}
          return JSON.stringify({...state,primaryInk:s.getPropertyValue('--md-on-primary').trim(),
            action,
            liquidBase:s.getPropertyValue('--android-liquid-base').trim(),
            motionEffect:{navigationTransition:getComputedStyle(document.querySelector('.android-navigation')).transitionDuration,
              systemReduced:matchMedia('(prefers-reduced-motion: reduce)').matches},
            navBackdrop:getComputedStyle(document.querySelector('.android-navigation')).backdropFilter});})()"""
        val result = waitFor(scenario, stage, expression) {
            val bars = accountBars(scenario)
            it.optBoolean("home") && it.optString("name").contains(name) && it.optString("count") == count &&
                it.optString("theme") == (if (dark) "dark" else "light") &&
                it.optString("glass") == (if (enhanced) "enhanced" else "local") &&
                it.optString("background") == background && it.optString("surface") == surface &&
                it.optString("foreground") == foreground && it.optString("accent") == expectedPrefs.getString("customAccentHex") &&
                it.optString("liquidBase") == surface && bars.getBoolean("statusBarLight") == !dark &&
                bars.getBoolean("navigationBarLight") == !dark && it.optBoolean("copyVisible") &&
                (!enhanced || (it.optBoolean("ready") && it.optBoolean("canvasVisible"))) &&
                (!motionChecks || it.optString("reduceMotion") == dark.toString())
        }
        if (motionChecks && dark) {
            val durations = result.getJSONObject("motionEffect").getString("navigationTransition").split(',')
                .map { value -> val text=value.trim(); if(text.endsWith("ms"))text.removeSuffix("ms").toDouble()/1000 else text.removeSuffix("s").toDouble() }
            assertTrue("用户减少动态效果应交付到真实导航 CSS",durations.all { it <= 0.001 })
        }
        assertFalse("账户切换后首页无横向溢出", result.getBoolean("overflow"))
        assertTrue("当前账户正文应可见", result.getBoolean("copyVisible"))
        assertEquals(4, result.getInt("navCount"))
        assertEquals(if (dark) "#ffffff" else "#000000", result.getString("primaryInk"))
        val action = result.getJSONObject("action")
        assertTrue("无填色首页操作文字应对比实际底色至少 4.5", action.getDouble("contrast") >= 4.5)
        assertTrue("首页操作应完整可见且可命中", action.getBoolean("visible") && action.getBoolean("hittable"))
        if (enhanced) {
            assertTrue("当前账户增强画布应可见", result.getBoolean("canvasVisible"))
            assertTrue("当前账户装饰仍需与正文分区", result.getJSONObject("copyRect").getDouble("right") <=
                result.getJSONObject("artRect").getDouble("x") + 1)
        } else assertTrue("局部档位不保留上一账户的画布", result.isNull("artRect"))
        val saved = readAccountPreferences(scenario, name, expectedPrefs, accountCount)
        val bitmap = screenshot(scenario)
        val native = nativeViewport(scenario)
        val scale = native.getDouble("webViewWidthPx") / result.getDouble("viewportWidth")
        val copy = result.getJSONObject("copyRect")
        val left = (native.getDouble("webViewScreenX") + copy.getDouble("x") * scale).toInt().coerceIn(0, bitmap.width-1)
        val right = (native.getDouble("webViewScreenX") + copy.getDouble("right") * scale).toInt().coerceIn(left+1, bitmap.width)
        val top = (native.getDouble("webViewScreenY") + copy.getDouble("y") * scale).toInt().coerceIn(0, bitmap.height-1)
        val bottom = (native.getDouble("webViewScreenY") + copy.getDouble("bottom") * scale).toInt().coerceIn(top+1, bitmap.height)
        val ink = android.graphics.Color.parseColor(foreground)
        var inkPixels = 0
        for (y in top until bottom) for (x in left until right) {
            val pixel = bitmap.getPixel(x,y)
            if (listOf(android.graphics.Color.red(pixel)-android.graphics.Color.red(ink),
                       android.graphics.Color.green(pixel)-android.graphics.Color.green(ink),
                       android.graphics.Color.blue(pixel)-android.graphics.Color.blue(ink)).all { kotlin.math.abs(it) <= 8 }) inkPixels++
        }
        assertTrue("最终合成帧必须有当前主题正文像素", inkPixels >= 20)
        val actionRect = action.getJSONObject("textRect")
        val actionRgb = action.getJSONArray("rgb")
        val actionLeft = (native.getDouble("webViewScreenX") + actionRect.getDouble("x") * scale).toInt().coerceIn(0, bitmap.width-1)
        val actionRight = (native.getDouble("webViewScreenX") + actionRect.getDouble("right") * scale).toInt().coerceIn(actionLeft+1, bitmap.width)
        val actionTop = (native.getDouble("webViewScreenY") + actionRect.getDouble("y") * scale).toInt().coerceIn(0, bitmap.height-1)
        val actionBottom = (native.getDouble("webViewScreenY") + actionRect.getDouble("bottom") * scale).toInt().coerceIn(actionTop+1, bitmap.height)
        var actionInkPixels = 0
        for (y in actionTop until actionBottom) for (x in actionLeft until actionRight) {
            val pixel = bitmap.getPixel(x,y)
            if (listOf(android.graphics.Color.red(pixel)-actionRgb.getInt(0),
                       android.graphics.Color.green(pixel)-actionRgb.getInt(1),
                       android.graphics.Color.blue(pixel)-actionRgb.getInt(2)).all { kotlin.math.abs(it) <= 8 }) actionInkPixels++
        }
        assertTrue("最终合成帧必须有可读的首页操作文字像素", actionInkPixels >= 20)
        action.put("inkPixels", actionInkPixels)
        File(evidence, "$stage.png").outputStream().use { bitmap.compress(Bitmap.CompressFormat.PNG,100,it) }
        bitmap.recycle()
        val keys = expectedPrefs.keys().asSequence().toList()
        val projection = JSONObject()
        keys.forEach { projection.put(it, saved.getJSONObject("prefs").get(it)) }
        result.put("stage",stage).put("accountId",saved.getString("accountId")).put("accountName",name)
            .put("savedPreferences",projection).put("copyInkPixels",inkPixels).put("systemBars",accountBars(scenario))
            .put("systemNightMode",accountBars(scenario).getBoolean("nightMode"))
        record(result)
        return result
    }

    private fun switchAccount(scenario: ActivityScenario<MainActivity>, name: String, password: String) {
        lockToPassword(scenario)
        unlockSelectedAccount(scenario, name, password)
    }

    private fun unlockSelectedAccount(scenario: ActivityScenario<MainActivity>, name: String, password: String) {
        val option = js(scenario, """(() => {const e=document.querySelector('[data-login-card] select');
          return JSON.stringify({value:[...e.options].find(o=>o.textContent.startsWith(${JSONObject.quote(name + " ·")}))?.value,count:e.options.length});})()""")
        assertEquals("选择器必须显示两个实际账户",2,option.getInt("count"))
        chooseValue(scenario,"[data-login-card] select",option.getString("value"))
        waitFor(scenario,"所选账户密码区就绪且已清空","""JSON.stringify({selected:document.querySelector('[data-login-card] select')?.value,
          enabled:!document.querySelector('[data-login-password-submit]')?.disabled,
          input:!!document.querySelector('[data-login-method-region="password"] input'),
          empty:document.querySelector('[data-login-method-region="password"] input')?.value===''})""") {
            it.optString("selected")==option.getString("value") && it.optBoolean("input") && it.optBoolean("enabled") && it.optBoolean("empty")
        }
        val filled = js(scenario,"""(() => {const e=document.querySelector('[data-login-method-region="password"] input');
          Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(e,${JSONObject.quote(password)});
          e.dispatchEvent(new Event('input',{bubbles:true}));return JSON.stringify({filled:true});})()""")
        assertTrue(filled.getBoolean("filled"))
        for (attempt in 1..30) {
            clickSelector(scenario,"[data-login-password-submit]")
            val outcome=waitFor(scenario,"切换账户解锁结果",page) { it.optBoolean("home") || it.optJSONArray("alerts")?.length()?.let { n->n>0 } == true }
            if (outcome.optBoolean("home")) {
                unlockAttempts.put(JSONObject().put("accountName",name).put("attempt",attempt).put("result","home"))
                return
            }
            val alerts=outcome.getJSONArray("alerts")
            unlockAttempts.put(JSONObject().put("accountName",name).put("attempt",attempt).put("alerts",alerts))
            assertEquals(1,alerts.length())
            assertEquals("The vault is under maintenance. Try again later.",alerts.getString(0))
            assertTrue("账户切换持续繁忙不能接受",attempt<30)
            Thread.sleep(2000)
        }
    }

    private fun changeSystemNight(scenario: ActivityScenario<MainActivity>, dark: Boolean) {
        val descriptor = instrumentation.uiAutomation.executeShellCommand("cmd uimode night ${if (dark) "yes" else "no"}")
        ParcelFileDescriptor.AutoCloseInputStream(descriptor).use { it.readBytes() }
        val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(15)
        do {
            var actual = false
            scenario.onActivity { activity ->
                actual = activity.resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK == Configuration.UI_MODE_NIGHT_YES
            }
            if (actual == dark) return
            Thread.sleep(100)
        } while (System.nanoTime() < deadline)
        throw AssertionError("实际系统夜间模式未交付: dark=$dark")
    }

    private fun recreateAndUnlock(scenario: ActivityScenario<MainActivity>, name: String, password: String) {
        var previousActivity: MainActivity? = null
        var previousWebView: WebView? = null
        scenario.onActivity { activity ->
            previousActivity = activity
            previousWebView = requireNotNull(webView(activity.window.decorView))
        }
        scenario.recreate()
        scenario.onActivity { activity ->
            assertNotSame("必须真正重建原生 Activity", previousActivity, activity)
            assertNotSame("必须真正重建 WebView，不接受只重读旧页面", previousWebView,
                requireNotNull(webView(activity.window.decorView)))
        }
        val state = waitFor(scenario, "重建后生产登录或首页", page) {
            it.optBoolean("home") || (it.optString("path") == "/login" &&
                it.optJSONArray("buttons")?.toString()?.contains("Unlock") == true)
        }
        if (!state.optBoolean("home")) unlockSelectedAccount(scenario, name, password)
    }

    private fun verifyAccounts(scenario: ActivityScenario<MainActivity>) {
        val nameA="FE2 public visual test"
        val nameB="FE2 public second account"
        val prefsA=JSONObject().put("theme","dark").put("defaultDarkTheme","forest-night")
            .put("accentColor","custom").put("customAccentHex","#112233").put("androidGlass","enhanced")
        val prefsB=JSONObject().put("theme","light").put("defaultLightTheme","clean-slate")
            .put("accentColor","custom").put("customAccentHex","#ffee00").put("androidGlass","local")
        if (motionChecks) { prefsA.put("reduceMotion",true); prefsB.put("reduceMotion",false) }
        configureAccountTheme(scenario,true,"forest-night","#112233",true)
        val firstA=captureAccount(scenario,"account-a-custom-dark",nameA,true,true,"1",prefsA,1)
        lockToPassword(scenario)
        clickText(scenario,"Create a new account")
        waitFor(scenario,"第二公开账户创建页面",page) { it.optString("path")=="/bootstrap" && it.optJSONArray("buttons")?.toString()?.contains("Create Account")==true }
        val filled=js(scenario,"""(() => {const inputs=[...document.querySelector('form').querySelectorAll('input')];
          if(inputs.length!==4)return JSON.stringify({filled:false});
          ['FE2 public second account','FE2-public-test-password-B-20261007','FE2-public-test-password-B-20261007',''].forEach((value,i)=>{
            Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(inputs[i],value);
            inputs[i].dispatchEvent(new Event('input',{bubbles:true}));
          });return JSON.stringify({filled:true});})()""")
        assertTrue(filled.getBoolean("filled"))
        clickText(scenario,"Create Account")
        waitFor(scenario,"第二账户真实首页",home) { it.optBoolean("home") && it.optString("name").contains(nameB) && it.optString("count")=="0" }
        configureAccountTheme(scenario,false,"clean-slate","#ffee00",false)
        val firstB=captureAccount(scenario,"account-b-custom-light",nameB,false,false,"0",prefsB,2)
        assertNotEquals("两个账户应有独立 ID",firstA.getString("accountId"),firstB.getString("accountId"))
        switchAccount(scenario,nameA,"FE2-public-test-password-20261007")
        val secondA=captureAccount(scenario,"account-a-after-switch",nameA,true,true,"1",prefsA,2)
        switchAccount(scenario,nameB,"FE2-public-test-password-B-20261007")
        val secondB=captureAccount(scenario,"account-b-after-switch",nameB,false,false,"0",prefsB,2)
        switchAccount(scenario,nameA,"FE2-public-test-password-20261007")
        val thirdA=captureAccount(scenario,"account-a-after-second-switch",nameA,true,true,"1",prefsA,2)
        assertEquals(firstA.getString("accountId"),secondA.getString("accountId"))
        assertEquals(firstA.getString("accountId"),thirdA.getString("accountId"))
        assertEquals(firstB.getString("accountId"),secondB.getString("accountId"))
        if (lifecycleChecks) {
            changeSystemNight(scenario, false)
            captureAccount(scenario,"account-a-system-light",nameA,true,true,"1",prefsA,2)
            recreateAndUnlock(scenario,nameA,"FE2-public-test-password-20261007")
            captureAccount(scenario,"account-a-after-activity-recreate",nameA,true,true,"1",prefsA,2)
                .put("activityRecreated",true).put("webViewRecreated",true)
            switchAccount(scenario,nameB,"FE2-public-test-password-B-20261007")
            changeSystemNight(scenario, true)
            captureAccount(scenario,"account-b-system-dark",nameB,false,false,"0",prefsB,2)
            recreateAndUnlock(scenario,nameB,"FE2-public-test-password-B-20261007")
            captureAccount(scenario,"account-b-after-activity-recreate",nameB,false,false,"0",prefsB,2)
                .put("activityRecreated",true).put("webViewRecreated",true)
            writeReport()
        }
        record(JSONObject().put("stage","account-preferences-isolated").put("accountCount",2)
            .put("aRetainsObjectCount",true).put("bRemainsEmpty",true).put("savedPreferencesUnchanged",true))
        if (InstrumentationRegistry.getArguments().getString("prepareColdRestart") == "true") {
            check(!lifecycleChecks)
            // 加密账户偏好可先于登录前镜像落盘；先取得真实只读回执，
            // 再结束进程，不能把未完成的异步镜像误当作已持久化配置。
            for (attempt in 1..30) {
                js(scenario,"""(() => {window.fe2StartupPrefs={pending:true};
                  window.__TAURI_INTERNALS__.invoke('ui_get_preferences',{}).then(
                    prefs=>window.fe2StartupPrefs={prefs},error=>window.fe2StartupPrefs={error:String(error)});
                  return JSON.stringify({started:true});})()""")
                val response = waitFor(scenario,"冷启动前外观镜像回执","JSON.stringify(window.fe2StartupPrefs||{})") {
                    it.has("prefs") || it.has("error")
                }
                assertFalse("读取外观镜像失败不能接受: $response",response.has("error"))
                val durable = response.getJSONObject("prefs")
                if (prefsA.keys().asSequence().all { durable.opt(it) == prefsA.get(it) }) break
                assertTrue("真实外观镜像未落盘: $durable",attempt < 30)
                Thread.sleep(1000)
            }
            records.getJSONObject(records.length()-1).put("startupUiPrefsConfirmed",true)
                .put("startupUiPreferences",prefsA)
            writeReport()
            val marker = File(instrumentation.targetContext.applicationInfo.dataDir, ".fe2-cold-owned.json")
            check(marker.createNewFile()) { "冷启动夹具不能覆盖已有 marker" }
            marker.writeText(JSONObject().put("version",1)
                .put("tag",InstrumentationRegistry.getArguments().getString("evidenceTag"))
                .put("processId",android.os.Process.myPid())
                .put("accounts",JSONObject().put(firstA.getString("accountId"),nameA)
                    .put(firstB.getString("accountId"),nameB)).toString())
            android.system.Os.chmod(marker.absolutePath, 384) // 0600，公开合成夹具仍保持私有权限。
        }
    }

    private fun verifyRenderingWork(scenario: ActivityScenario<MainActivity>) {
        // 只在专用合成账户的测试进程记录实际 drawArrays 调用，不改变生产 renderer。
        // 正向返回样本必须产生绘制，防止未安装探针被误记为“静止零绘制”。
        val installed = js(scenario, """(() => {
          const proto=WebGLRenderingContext.prototype,original=proto.drawArrays;
          const probe={original,phase:'idle',startedAt:performance.now(),draws:[],installed:true};
          window.fe2DrawProbe=probe;
          proto.drawArrays=function(...args){
            if(!this.canvas?.closest('.android-liquid-artwork'))return original.apply(this,args);
            const start=performance.now(),value=original.apply(this,args);
            probe.draws.push({phase:probe.phase,submitMs:performance.now()-start,connected:this.canvas.isConnected,
              width:this.canvas.width,height:this.canvas.height});return value;
          };
          return JSON.stringify({installed:true});
        })()""")
        assertTrue(installed.getBoolean("installed"))
        fun begin(phase: String) {
            js(scenario, """(() => {const p=window.fe2DrawProbe;p.phase=${JSONObject.quote(phase)};
              p.startedAt=performance.now();p.draws=[];return JSON.stringify({started:true})})()""")
        }
        fun sample(phase: String, mounted: Boolean, shouldDraw: Boolean) {
            val expression = """(() => {const p=window.fe2DrawProbe,e=document.querySelector('.android-liquid-artwork'),c=e?.querySelector('canvas');
              return JSON.stringify({path:location.pathname,hookInstalled:p?.installed===true,phase:p?.phase,
                elapsedMs:performance.now()-p.startedAt,timeOrigin:performance.timeOrigin,
                drawCalls:p.draws.length,draws:p.draws,artworkMounted:!!e,ready:e?.dataset.liquidReady==='true',
                canvasWidth:c?.width||0,canvasHeight:c?.height||0,documentHidden:document.hidden,
                measurement:'actual JS WebGL drawArrays submissions; CPU time excludes GPU/compositor work'});})()"""
            val result = waitFor(scenario,"绘制工作量 $phase",expression) { it.optDouble("elapsedMs") >= 1000 }
            record(result.put("stage","rendering-$phase"))
            assertTrue("必须使用已安装的实际调用探针",result.getBoolean("hookInstalled"))
            assertFalse("样本必须处于前景文档",result.getBoolean("documentHidden"))
            assertEquals(mounted,result.getBoolean("artworkMounted"))
            assertEquals(if (mounted) "/" else "/tools",result.getString("path"))
            if (mounted) assertTrue("装饰应已提交可用绘制",result.getBoolean("ready") && result.getInt("canvasWidth") > 0 && result.getInt("canvasHeight") > 0)
            if (shouldDraw) assertTrue("返回必须产生真实绘制正向控制",result.getInt("drawCalls") > 0)
            else assertEquals("静止或离开首页不能持续提交 WebGL 绘制",0,result.getInt("drawCalls"))
        }
        try {
            begin("idle"); sample("idle",mounted=true,shouldDraw=false)
            begin("away"); clickSelector(scenario,".android-navigation a[href='/tools']")
            waitFor(scenario,"离开首页",page) { it.optString("path") == "/tools" }
            sample("away",mounted=false,shouldDraw=false)
            begin("return"); clickSelector(scenario,".android-navigation a[href='/']")
            waitFor(scenario,"增强首页已返回",home) { it.optBoolean("home") && it.optBoolean("ready") }
            sample("return",mounted=true,shouldDraw=true)
            capture(scenario,"rendering-return-ready",dark=false,enhanced=true,count="1")
            begin("return-idle"); sample("return-idle",mounted=true,shouldDraw=false)
        } finally {
            js(scenario,"""(() => {const p=window.fe2DrawProbe;if(p){WebGLRenderingContext.prototype.drawArrays=p.original;p.installed=false;}
              return JSON.stringify({restored:true})})()""")
        }
    }

    @Test fun coldAccountPreferencesSurviveProcessRestart() {
        val arguments = InstrumentationRegistry.getArguments()
        assertEquals("true",arguments.getString("privateDataBackedUp"))
        assertEquals("cold",arguments.getString("scenario"))
        val initialTag = requireNotNull(arguments.getString("initialEvidenceTag"))
        val tag = requireNotNull(arguments.getString("evidenceTag"))
        require(Regex("fe2-home-[a-z0-9-]+").matches(initialTag) && tag == "$initialTag-cold")
        val data = File(instrumentation.targetContext.applicationInfo.dataDir)
        val markerFile = File(data,".fe2-cold-owned.json")
        check(markerFile.isFile && markerFile.canonicalFile == File(data.canonicalFile,".fe2-cold-owned.json"))
        val marker = JSONObject(markerFile.readText())
        assertEquals(1,marker.getInt("version"))
        assertEquals(initialTag,marker.getString("tag"))
        assertNotEquals("必须真正结束旧进程，不能把 Activity 重建称为冷启动",
            marker.getInt("processId"),android.os.Process.myPid())
        assertEquals(arguments.getString("previousProcessId")!!.toInt(),marker.getInt("processId"))
        val expected = marker.getJSONObject("accounts")
        val ids = expected.keys().asSequence().toSet()
        assertEquals(arguments.getString("expectedAccountIds")!!.split(',').toSet(),ids)
        assertEquals(2,ids.size)
        assertEquals(setOf("FE2 public visual test","FE2 public second account"),
            ids.map { expected.getString(it) }.toSet())
        check(ids.all { Regex("acc_[a-f0-9]{16}").matches(it) })
        var manifestFound = false
        val accountDirectories = mutableSetOf<String>()
        for (file in data.walkTopDown()) {
            if (file.name.startsWith("acc_")) {
                check(file.isDirectory && file.name in ids) { "夹具中存在外来账户目录" }
                accountDirectories.add(file.name)
            }
            if (file.name == "accounts.json") {
                val accounts = JSONArray(file.readText())
                assertEquals(2,accounts.length())
                assertEquals(ids,(0 until accounts.length()).map { accounts.getJSONObject(it).getString("id") }.toSet())
                for (i in 0 until accounts.length()) {
                    val account = accounts.getJSONObject(i)
                    assertEquals(expected.getString(account.getString("id")),account.getString("name"))
                }
                manifestFound = true
            }
        }
        check(manifestFound && accountDirectories == ids)
        coldChecks = true
        accountChecks = true
        evidence = File(instrumentation.targetContext.getExternalFilesDir(null),tag).also { check(it.mkdir()) }
        try {
            val scenario = ActivityScenario.launch(MainActivity::class.java)
            val locked = waitFor(scenario,"真实进程冷启动必须进入锁定页且恢复外观","""(() => {
              const state=JSON.parse($page),s=getComputedStyle(document.documentElement);
              return JSON.stringify({...state,theme:document.documentElement.dataset.theme,
                background:s.getPropertyValue('--bg-base').trim(),foreground:s.getPropertyValue('--text-primary').trim(),
                accent:s.getPropertyValue('--accent-primary').trim(),
                accountCount:document.querySelector('[data-login-card] select')?.options.length,
                passwordRequired:!!document.querySelector('[data-login-method-region="password"] input')});})()""") {
                it.optString("path") == "/login" && !it.optBoolean("home") &&
                    it.optJSONArray("buttons")?.toString()?.contains("Unlock") == true &&
                    it.optBoolean("passwordRequired") && it.optInt("accountCount") == 2 &&
                    it.optString("theme") == "dark" && it.optString("glass") == "enhanced" &&
                    it.optString("background") == "#1a211d" && it.optString("foreground") == "#d6ddd8" &&
                    it.optString("accent") == "#112233" &&
                    (!motionChecks || it.optString("reduceMotion") == "true")
            }
            val bitmap = screenshot(scenario)
            File(evidence,"cold-login.png").outputStream().use { bitmap.compress(Bitmap.CompressFormat.PNG,100,it) }
            bitmap.recycle()
            record(locked.put("stage","cold-login").put("previousProcessId",marker.getInt("processId"))
                .put("processId",android.os.Process.myPid()).put("locked",true))
            val prefsA=JSONObject().put("theme","dark").put("defaultDarkTheme","forest-night")
                .put("accentColor","custom").put("customAccentHex","#112233").put("androidGlass","enhanced")
            val prefsB=JSONObject().put("theme","light").put("defaultLightTheme","clean-slate")
                .put("accentColor","custom").put("customAccentHex","#ffee00").put("androidGlass","local")
            if (motionChecks) { prefsA.put("reduceMotion",true); prefsB.put("reduceMotion",false) }
            unlockSelectedAccount(scenario,"FE2 public visual test","FE2-public-test-password-20261007")
            captureAccount(scenario,"account-a-cold-system-light","FE2 public visual test",true,true,"1",prefsA,2)
            changeSystemNight(scenario,true)
            captureAccount(scenario,"account-a-cold-system-dark","FE2 public visual test",true,true,"1",prefsA,2)
            switchAccount(scenario,"FE2 public second account","FE2-public-test-password-B-20261007")
            captureAccount(scenario,"account-b-cold-system-dark","FE2 public second account",false,false,"0",prefsB,2)
            record(JSONObject().put("stage","account-preferences-isolated").put("accountCount",2)
                .put("aRetainsObjectCount",true).put("bRemainsEmpty",true).put("savedPreferencesUnchanged",true))
        } finally {
            writeReport()
        }
    }

    @Test fun realHomePreservesGlassAndCopyAcrossLockUnlock() {
        val arguments = InstrumentationRegistry.getArguments()
        val scenarioName = arguments.getString("scenario") ?: "baseline"
        require(scenarioName == "baseline" || scenarioName == "overlays" || scenarioName == "keyboard" || scenarioName == "accounts" || scenarioName == "insets" || scenarioName == "lifecycle" || scenarioName == "previews" || scenarioName == "rendering")
        renderingChecks = scenarioName == "rendering"
        previewChecks = scenarioName == "previews"
        overlayChecks = scenarioName == "overlays"
        keyboardChecks = scenarioName == "keyboard"
        lifecycleChecks = scenarioName == "lifecycle"
        accountChecks = scenarioName == "accounts" || lifecycleChecks
        insetsChecks = scenarioName == "insets"
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
            unlockPublicVisualAccount(scenario)
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
            if (accountChecks) verifyAccounts(scenario)
            if (insetsChecks) verifyInsets(scenario)
            if (renderingChecks) verifyRenderingWork(scenario)
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
            val cleanup=JSONArray()
            for(uri in ownedSafUris) {
                val deleted=instrumentation.targetContext.contentResolver.delete(uri,null,null)
                cleanup.put(JSONObject().put("deleted",deleted).put("uri",uri.toString()))
                assertEquals("Only test-owned public MediaStore entries cleaned",1,deleted)
            }
            if(safPickerChecks)record(JSONObject().put("stage","saf-public-fixtures-cleaned").put("entries",cleanup).put("count",cleanup.length()))
            writeReport()
        }
    }
}
