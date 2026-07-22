package com.solosoul.app

import android.app.Activity
import android.app.Application
import android.app.KeyguardManager
import android.content.Context
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.JSObject
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin

/**
 * 锁屏状态检测插件。
 *
 * 在 Activity onPause 时通过 KeyguardManager.isKeyguardLocked 同步判断
 * 当前设备是否处于锁屏状态。如果是（按电源键灭屏），通过 trigger 向 JS
 * 推送 "screen-locked" 事件；否则（仅切后台）不推送——交由前端根据
 * autoLockOnBackground 开关决定。
 *
 * 事件可能在 WebView 冻结时发出，回到前台后补达，时序上都能触发锁定。
 */
@TauriPlugin
class LockStatePlugin(private val activity: Activity): Plugin(activity) {

    /**
     * 标记灭屏期间是否检测到锁屏，用于 onResume 时补达事件。
     * WebView 冻结可能导致 trigger 发出的事件在回前台时丢失。
     */
    private var lockedWhileBackgrounded = false

    /**
     * 检查 keyguard 状态，若锁屏则 trigger screen-locked 事件。
     * 在 onPause/onStopped 和 postDelayed 重试中调用。
     */
    private fun checkKeyguardAndTrigger() {
        try {
            val km = activity.getSystemService(Context.KEYGUARD_SERVICE) as? KeyguardManager
            if (km?.isKeyguardLocked == true) {
                lockedWhileBackgrounded = true
                val payload = JSObject()
                payload.put("locked", true)
                trigger("screen-locked", payload)
            }
        } catch (e: Exception) {
            android.util.Log.e("SoloSoul", "LockStatePlugin checkKeyguardAndTrigger failed", e)
        }
    }

    init {
        // 注册 Application 生命周期回调，监听宿主 Activity 的生命周期事件。
        (activity.applicationContext as Application).registerActivityLifecycleCallbacks(
            object : Application.ActivityLifecycleCallbacks {
                override fun onActivityPaused(act: Activity) {
                    // onPause 时同步检查一次（按电源键灭屏时 keyguard 可能已置位）
                    checkKeyguardAndTrigger()
                    // keyguard 可能在 onPause 之后才置位，延迟 500ms 再查一次
                    Handler(Looper.getMainLooper()).postDelayed({ checkKeyguardAndTrigger() }, 500)
                }
                override fun onActivityResumed(act: Activity) {
                    // 灭屏期间 WebView 冻结可能导致 trigger 事件丢失，
                    // 回前台时补达一次。doLock 有 lockInitiated 幂等保护。
                    if (lockedWhileBackgrounded) {
                        lockedWhileBackgrounded = false
                        val payload = JSObject()
                        payload.put("locked", true)
                        trigger("screen-locked", payload)
                    }
                }
                override fun onActivityCreated(act: Activity, savedInstanceState: Bundle?) {}
                override fun onActivityStarted(act: Activity) {}
                override fun onActivityStopped(act: Activity) {
                    // onStopped 作为 onPause 的补充检测点
                    checkKeyguardAndTrigger()
                }
                override fun onActivitySaveInstanceState(act: Activity, outState: Bundle) {}
                override fun onActivityDestroyed(act: Activity) {}
            },
        )
    }

    @Command
    fun isScreenLocked(invoke: Invoke) {
        try {
            val keyguardManager = activity.getSystemService(Context.KEYGUARD_SERVICE) as? KeyguardManager
            val locked = keyguardManager?.isKeyguardLocked ?: false
            val result = JSObject()
            result.put("locked", locked)
            invoke.resolve(result)
        } catch (e: Exception) {
            android.util.Log.e("SoloSoul", "LockStatePlugin.isScreenLocked failed", e)
            invoke.reject("无法获取锁屏状态")
        }
    }
}
