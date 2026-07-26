package com.solosoul.app

import android.app.Activity
import android.app.Application
import android.app.KeyguardManager
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.graphics.drawable.ColorDrawable
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.util.TypedValue
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.JSObject
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin

/**
 * 锁屏状态检测插件。
 *
 * 前台锁屏：在 Activity onPause/onStop 时通过 KeyguardManager.isKeyguardLocked
 * 判断是否为锁屏（而非仅切后台），是则推送 "screen-locked" 事件。
 *
 * 后台锁屏：灭屏瞬间 keyguard 通常尚未置位，且灭屏后进程很快被冻结，
 * 「灭屏时查 keyguard」是竞态。改为 SCREEN_OFF → USER_PRESENT 事件对：
 * - ACTION_SCREEN_OFF（不区分电源键/超时自动灭屏）：置 pending 标记；
 * - ACTION_USER_PRESENT：只有 keyguard 出现并被解除时系统才发出，
 *   且此刻进程必定解冻，是"确实锁过屏"的铁证 → 触发锁定；
 * - ACTION_SCREEN_ON：点亮时 keyguard 未锁（无锁屏密码设备纯灭屏）→
 *   清除 pending，保持"锁屏才锁、纯灭屏不锁"语义。
 *
 * 检测到锁屏时先给窗口加不透明遮盖（decorView.foreground），
 * 避免 WebView 解冻首帧泄露旧页面内容；前端进入登录页后调用
 * dismissLockMask 撤掉遮盖。
 *
 * 事件可能在 WebView 冻结时发出，回到前台后按持久化标记补达。
 */
@TauriPlugin
class LockStatePlugin(private val activity: Activity): Plugin(activity) {

    /**
     * 标记灭屏期间是否检测到锁屏，用于 onResume 时补达事件。
     * WebView 冻结可能导致 trigger 发出的事件在回前台时丢失。
     */
    private var lockedWhileBackgrounded = false

    /** 锁屏遮盖是否已显示（幂等控制）。 */
    private var lockMaskShown = false

    /**
     * 用于进程被系统回收后恢复灭屏锁标记。
     */
    private val prefs by lazy {
        activity.getSharedPreferences("LockStatePlugin", Context.MODE_PRIVATE)
    }

    companion object {
        private const val PREF_LOCKED_WHILE_BACKGROUNDED = "locked_while_backgrounded"
        private const val PREF_SCREEN_OFF_PENDING = "screen_off_pending"
    }

    /**
     * 屏幕状态广播监听。注册在 applicationContext 上，Activity 销毁后仍有效。
     */
    private val screenStateReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            when (intent.action) {
                Intent.ACTION_SCREEN_OFF -> {
                    setPendingFlag(true)
                    // 部分 ROM 灭屏即锁，keyguard 可能已置位，同步 + 延迟各查一次
                    checkKeyguardAndTrigger()
                    Handler(Looper.getMainLooper()).postDelayed({ checkKeyguardAndTrigger() }, 300)
                }
                Intent.ACTION_USER_PRESENT -> {
                    // keyguard 被解除才发出此广播，本身即"确实锁过屏"的铁证，
                    // 无需再查 keyguard（此刻它已复位为 false）
                    if (getPendingFlag()) {
                        setPendingFlag(false)
                        markLockedAndTrigger()
                    }
                }
                Intent.ACTION_SCREEN_ON -> {
                    // 无锁屏密码设备纯灭屏：点亮时 keyguard 未锁，清除挂起避免误锁
                    if (getPendingFlag() && !isKeyguardLocked()) {
                        setPendingFlag(false)
                    }
                }
            }
        }
    }

    /**
     * 保存 Application 生命周期回调引用，用于在插件销毁时反注册，避免内存泄漏。
     */
    private var lifecycleCallbacks: Application.ActivityLifecycleCallbacks? = null

    private fun isKeyguardLocked(): Boolean {
        val km = activity.getSystemService(Context.KEYGUARD_SERVICE) as? KeyguardManager
        return km?.isKeyguardLocked == true
    }

    /**
     * 检查 keyguard 状态，若锁屏则进入锁定分支。
     * 在 onPause/onStopped、SCREEN_OFF 同步与 postDelayed 重试中调用。
     */
    private fun checkKeyguardAndTrigger() {
        try {
            if (isKeyguardLocked()) {
                markLockedAndTrigger()
            }
        } catch (e: Exception) {
            android.util.Log.e("SoloSoul", "LockStatePlugin checkKeyguardAndTrigger failed", e)
        }
    }

    /**
     * 锁定分支：先上遮盖（防 WebView 解冻首帧泄露旧内容），
     * 再置标记并 trigger screen-locked 事件。
     */
    private fun markLockedAndTrigger() {
        showLockMask()
        lockedWhileBackgrounded = true
        persistLockedFlag(true)
        val payload = JSObject()
        payload.put("locked", true)
        trigger("screen-locked", payload)
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
     * 灭屏挂起标记：SCREEN_OFF 置位，USER_PRESENT 确认后清除，
     * SCREEN_ON 时 keyguard 未锁（无锁屏设备）也会清除。
     */
    private fun setPendingFlag(value: Boolean) {
        prefs.edit().putBoolean(PREF_SCREEN_OFF_PENDING, value).apply()
    }

    private fun getPendingFlag(): Boolean {
        return prefs.getBoolean(PREF_SCREEN_OFF_PENDING, false)
    }

    /**
     * 触发 screen-locked 事件，并清理锁屏标记。
     */
    private fun triggerScreenLocked() {
        showLockMask()
        lockedWhileBackgrounded = false
        persistLockedFlag(false)
        val payload = JSObject()
        payload.put("locked", true)
        trigger("screen-locked", payload)
    }

    /** 在主线程执行（已在主线程则直接执行，保证 onResume 中遮盖早于首帧绘制）。 */
    private fun onUi(block: () -> Unit) {
        if (Looper.myLooper() == Looper.getMainLooper()) {
            block()
        } else {
            activity.runOnUiThread(block)
        }
    }

    /**
     * 给窗口加不透明遮盖：取用主题的 windowBackground（与启动画面一致，
     * 明暗主题自适应），盖住 WebView 解冻后首帧可能泄露的旧页面内容。
     */
    private fun showLockMask() {
        if (lockMaskShown) return
        try {
            onUi {
                val a = activity.theme.obtainStyledAttributes(
                    intArrayOf(android.R.attr.windowBackground)
                )
                var bg = a.getDrawable(0)
                a.recycle()
                if (bg == null) {
                    // 主题未显式设置 windowBackground 时退化为 colorBackground 纯色
                    val tv = TypedValue()
                    activity.theme.resolveAttribute(android.R.attr.colorBackground, tv, true)
                    bg = ColorDrawable(tv.data)
                }
                activity.window.decorView.foreground = bg
                lockMaskShown = true
            }
        } catch (e: Exception) {
            android.util.Log.e("SoloSoul", "LockStatePlugin showLockMask failed", e)
        }
    }

    /** 撤掉锁屏遮盖（前端进入登录页后调用）。 */
    private fun hideLockMask() {
        if (!lockMaskShown) return
        try {
            onUi {
                activity.window.decorView.foreground = null
                lockMaskShown = false
            }
        } catch (e: Exception) {
            android.util.Log.e("SoloSoul", "LockStatePlugin hideLockMask failed", e)
        }
    }

    init {
        // 注册屏幕状态广播监听（applicationContext，Activity 销毁后仍有效）。
        // 必须使用 RECEIVER_EXPORTED：这些系统广播在 Android 13+ 用
        // NOT_EXPORTED 会收不到；系统广播第三方无法伪造，导出安全。
        val filter = IntentFilter(Intent.ACTION_SCREEN_OFF).apply {
            addAction(Intent.ACTION_SCREEN_ON)
            addAction(Intent.ACTION_USER_PRESENT)
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            activity.applicationContext.registerReceiver(
                screenStateReceiver,
                filter,
                Context.RECEIVER_EXPORTED,
            )
        } else {
            activity.applicationContext.registerReceiver(screenStateReceiver, filter)
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
            val result = JSObject()
            result.put("locked", isKeyguardLocked())
            invoke.resolve(result)
        } catch (e: Exception) {
            android.util.Log.e("SoloSoul", "LockStatePlugin.isScreenLocked failed", e)
            invoke.reject("无法获取锁屏状态")
        }
    }

    @Command
    fun dismissLockMask(invoke: Invoke) {
        try {
            hideLockMask()
            invoke.resolve(JSObject())
        } catch (e: Exception) {
            android.util.Log.e("SoloSoul", "LockStatePlugin.dismissLockMask failed", e)
            invoke.reject("dismissLockMask failed: ${e.message}")
        }
    }
}
