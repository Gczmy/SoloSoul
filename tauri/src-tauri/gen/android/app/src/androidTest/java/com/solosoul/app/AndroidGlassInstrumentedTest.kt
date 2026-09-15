package com.solosoul.app

import android.app.Dialog
import android.graphics.Bitmap
import android.os.Build
import android.view.View
import android.view.ViewGroup
import android.view.WindowManager
import android.widget.TextView
import androidx.lifecycle.Lifecycle
import androidx.test.core.app.ActivityScenario
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.tauri.plugin.Invoke
import com.fasterxml.jackson.databind.ObjectMapper
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Assume.assumeTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.util.concurrent.CopyOnWriteArrayList

@RunWith(AndroidJUnit4::class)
class AndroidGlassInstrumentedTest {
    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()
    private fun args(id: String) = JSONObject().apply {
        put("requestId",id); put("title","New record"); put("description","Choose a way to record it")
        put("closeLabel","Close"); put("footer","Local vault · Fictional test")
        put("labels",JSONObject().put("object","New object").put("page","New page").put("scan","Scan"))
        put("descriptions",JSONObject().put("object","From a template").put("page","Organize your records").put("scan","Import a photo"))
        put("background","#F9F9F3");put("foreground","#20251F");put("secondary","#626B62");put("accent","#405F82");put("container","#D9E7F8")
    }.toString()
    private fun invoke(command: String, json: String, responses: MutableList<JSONObject>) = Invoke(
        1,command,1,2,{ callback,data -> assertEquals(1L,callback); responses.add(JSONObject(data)) },json,ObjectMapper()
    )
    private fun dialog(plugin: AndroidGlassPlugin): Dialog? {
        val field=AndroidGlassPlugin::class.java.getDeclaredField("activeDialog");field.isAccessible=true
        return field.get(plugin) as? Dialog
    }
    private fun backgroundAlpha(plugin: AndroidGlassPlugin): Int {
        // DecorView 可包装系统的 BackgroundBlurDrawable；验证我们实际设置的色底。
        val field = AndroidGlassPlugin::class.java.getDeclaredField("windowBackground")
        field.isAccessible = true
        return (field.get(plugin) as android.graphics.drawable.GradientDrawable).alpha
    }
    private fun findText(view: View,label: String): TextView? {
        if(view is TextView && view.text.toString()==label)return view
        if(view is ViewGroup) for(i in 0 until view.childCount)findText(view.getChildAt(i),label)?.let{return it}
        return null
    }
    private fun screenshot(name: String) {
        // 状态断言后等待系统 dim/blur 合成稳定，避免截图混入上一帧材质。
        instrumentation.uiAutomation.waitForIdle(350, 3000)
        val bitmap=instrumentation.uiAutomation.takeScreenshot()
        File(instrumentation.targetContext.getExternalFilesDir(null),name).outputStream().use { bitmap.compress(Bitmap.CompressFormat.PNG,100,it) }
        bitmap.recycle()
    }
    private fun requireBlur(activity: GlassRegressionActivity) {
        assumeTrue(Build.VERSION.SDK_INT>=31 && activity.getSystemService(WindowManager::class.java).isCrossWindowBlurEnabled)
    }
    private fun waitForMenu(scenario: ActivityScenario<GlassRegressionActivity>) {
        val deadline = System.currentTimeMillis() + 3000
        var focused = false
        while (!focused && System.currentTimeMillis() < deadline) {
            scenario.onActivity { focused = dialog(it.glass)?.window?.decorView?.hasWindowFocus() == true }
            if (!focused) Thread.sleep(30)
        }
        assertTrue("Menu must own window focus before interaction", focused)
        instrumentation.uiAutomation.waitForIdle(100, 3000)
    }

    @Test fun nativeMenuSelectsExactlyOnceAndPauseCancels() {
        ActivityScenario.launch(GlassRegressionActivity::class.java).use { scenario ->
            val result=CopyOnWriteArrayList<JSONObject>()
            scenario.onActivity { activity -> requireBlur(activity); activity.glass.showMenu(invoke("showMenu",args("first"),result)) }
            waitForMenu(scenario)
            scenario.onActivity { activity ->
                val window=dialog(activity.glass)!!.window!!
                assertTrue(dialog(activity.glass)!!.isShowing)
                assertTrue(window.attributes.blurBehindRadius>0)
                assertTrue(backgroundAlpha(activity.glass)<255)
                assertTrue(activity.lifecycle.currentState.isAtLeast(Lifecycle.State.RESUMED))
            }
            screenshot("glass-native-enabled.png")
            scenario.onActivity { activity ->
                val label=findText(dialog(activity.glass)!!.window!!.decorView,"New object")!!
                (label.parent.parent as View).performClick()
                assertNull(dialog(activity.glass))
            }
            assertEquals(1,result.size);assertEquals("object",result[0].getString("action"))
            scenario.onActivity { it.glass.showMenu(invoke("showMenu",args("second"),result)) }
            waitForMenu(scenario)
            scenario.moveToState(Lifecycle.State.CREATED)
            assertEquals(2,result.size);assertEquals("cancel",result[1].getString("action"))
            scenario.moveToState(Lifecycle.State.RESUMED)
            scenario.onActivity { assertNull(dialog(it.glass)) }
        }
    }

    @Test fun cancelledRequestCannotReopenAndBackCancelDoesNotNavigate() {
        ActivityScenario.launch(GlassRegressionActivity::class.java).use { scenario ->
            val result=CopyOnWriteArrayList<JSONObject>()
            scenario.onActivity { activity ->
                requireBlur(activity)
                activity.glass.closeMenu(invoke("closeMenu","{\"requestId\":\"old\"}",CopyOnWriteArrayList()))
                activity.glass.showMenu(invoke("showMenu",args("old"),result))
                assertNull(dialog(activity.glass))
                assertEquals("cancel",result.single().getString("action"))
                activity.glass.showMenu(invoke("showMenu",args("new"),result))
            }
            waitForMenu(scenario)
            // 使用公开的 Instrumentation API，避免旧 Espresso 对 InputManager 隐藏 API 的依赖。
            instrumentation.sendKeyDownUpSync(android.view.KeyEvent.KEYCODE_BACK)
            instrumentation.waitForIdleSync()
            assertEquals(2,result.size)
            assertEquals("cancel",result[1].getString("action"))
            scenario.onActivity { assertNull(dialog(it.glass)); assertFalse(it.isFinishing) }
        }
    }

    @Test fun systemDisablingBlurKeepsExistingMenuReadable() {
        ActivityScenario.launch(GlassRegressionActivity::class.java).use { scenario ->
            val result=CopyOnWriteArrayList<JSONObject>()
            scenario.onActivity { activity -> requireBlur(activity); activity.glass.showMenu(invoke("showMenu",args("toggle"),result)) }
            waitForMenu(scenario)
            val resolver = instrumentation.targetContext.contentResolver
            val setting = "disable_window_blurs"
            val previous = android.provider.Settings.Global.getString(resolver, setting)
            instrumentation.uiAutomation.adoptShellPermissionIdentity(android.Manifest.permission.WRITE_SECURE_SETTINGS)
            try {
                assertTrue(android.provider.Settings.Global.putInt(resolver, setting, 1))
                // WindowManager 回调异步到主线程，按实际状态等待，避免固定帧数假设。
                val deadline=System.currentTimeMillis()+3000
                var opaque=false
                while(!opaque && System.currentTimeMillis()<deadline) {
                    instrumentation.waitForIdleSync()
                    scenario.onActivity { opaque=backgroundAlpha(it.glass)==255 }
                    if(!opaque) Thread.sleep(30)
                }
                var state = ""
                scenario.onActivity { state = "enabled=${it.getSystemService(WindowManager::class.java).isCrossWindowBlurEnabled}, alpha=${backgroundAlpha(it.glass)}" }
                assertTrue(state, opaque)
                scenario.onActivity { activity ->
                    assertTrue(dialog(activity.glass)!!.isShowing)
                    assertFalse(activity.getSystemService(WindowManager::class.java).isCrossWindowBlurEnabled)
                    assertEquals(0,dialog(activity.glass)!!.window!!.attributes.blurBehindRadius)
                }
                screenshot("glass-native-disabled.png")
            } finally {
                android.provider.Settings.Global.putString(resolver, setting, previous)
                instrumentation.uiAutomation.dropShellPermissionIdentity()
            }
        }
    }
}
