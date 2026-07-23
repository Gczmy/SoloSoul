package com.solosoul.app

import android.app.Activity
import android.app.Application
import android.app.KeyguardManager
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.Build
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
     * 用于进程被系统回收后恢复灭屏锁标记。
     */
    private val prefs by lazy {
        activity.getSharedPreferences("LockStatePlugin", Context.MODE_PRIVATE)
    }

    companion object {
        private const val PREF_LOCKED_WHILE_BACKGROUNDED = "locked_while_backgrounded"
    }

    /**
     * 监听屏幕熄灭广播，捕获应用在后台时用户按电源键锁屏的场景。
     */
    private val screenOffReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            if (intent.action != Intent.ACTION_SCREEN_OFF) return
            checkKeyguardAndTrigger()
            // KeyguardManager 状态可能略滞后于屏幕熄灭广播，延迟再试一次
            Handler(Looper.getMainLooper()).postDelayed({ checkKeyguardAndTrigger() }, 300)
        }
    }

    /**
     * 保存 Application 生命周期回调引用，用于在插件销毁时反注册，避免内存泄漏。
     */
    private var lifecycleCallbacks: Application.ActivityLifecycleCallbacks? = null

    /**
     * 检查 keyguard 状态，若锁屏则 trigger screen-locked 事件。
     * 在 onPause/onStopped 和 postDelayed 重试中调用。
     */
    private fun checkKeyguardAndTrigger() {
        try {
            val km = activity.getSystemService(Context.KEYGUARD_SERVICE) as? KeyguardManager
            if (km?.isKeyguardLocked == true) {
                lockedWhileBackgrounded = true
                persistLockedFlag(true)
                val payload = JSObject()
                payload.put("locked", true)
                trigger("screen-locked", payload)
            }
        } catch (e: Exception) {
            android.util.Log.e("SoloSoul", "LockStatePlugin checkKeyguardAndTrigger failed", e)
        }
    }

    /**
     * 持久化锁屏标记，用于进程恢复后仍能触发锁定。
     */
    private fun persistLockedFlag(value: Boolean) {
        prefs.edit().putBoolean(PREF_LOCKED_WHILE_BACKGROUNDED, value).apply()
    }

    /**
     * 读取持久化的锁屏标记。
     */
    private fun restoreLockedFlag(): Boolean {
        return prefs.getBoolean(PREF_LOCKED_WHILE_BACKGROUNDED, false)
    }

    /**
     * 触发 screen-locked 事件，并清理锁屏标记。
     */
    private fun triggerScreenLocked() {
        lockedWhileBackgrounded = false
        persistLockedFlag(false)
        val payload = JSObject()
        payload.put("locked", true)
        trigger("screen-locked", payload)
    }

    init {
        // 注册屏幕熄灭广播监听，捕获应用在后台时用户按电源键锁屏的场景。
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            activity.registerReceiver(
                screenOffReceiver,
                IntentFilter(Intent.ACTION_SCREEN_OFF),
                Context.RECEIVER_NOT_EXPORTED,
            )
        } else {
            activity.registerReceiver(screenOffReceiver, IntentFilter(Intent.ACTION_SCREEN_OFF))
        }

        // 注册 Application 生命周期回调，监听宿主 Activity 的生命周期事件。
        lifecycleCallbacks = object : Application.ActivityLifecycleCallbacks {
            override fun onActivityPaused(act: Activity) {
                // onPause 时同步检查一次（按电源键灭屏时 keyguard 可能已置位）
                checkKeyguardAndTrigger()
                // keyguard 可能在 onPause 之后才置位，延迟 500ms 再查一次
                Handler(Looper.getMainLooper()).postDelayed({ checkKeyguardAndTrigger() }, 500)
            }
            override fun onActivityResumed(act: Activity) {
                // 灭屏期间 WebView 冻结可能导致 trigger 事件丢失，
                // 回前台时补达一次。doLock 有 lockInitiated 幂等保护。
                // 同时读取持久化标记，防止进程回收后成员变量丢失。
                if (lockedWhileBackgrounded || restoreLockedFlag()) {
                    triggerScreenLocked()
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
        }
        (activity.applicationContext as Application).registerActivityLifecycleCallbacks(lifecycleCallbacks)
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
