package com.solosoul.app

import android.app.Activity
import android.app.KeyguardManager
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.graphics.drawable.ColorDrawable
import android.os.Build
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
 * 前台锁屏：插件基类生命周期钩子 onPause/onStop 时通过
 * KeyguardManager.isKeyguardLocked 判断是否为锁屏（而非仅切后台）。
 *
 * 后台锁屏：灭屏瞬间 keyguard 通常尚未置位，且灭屏后进程很快被冻结，
 * 「灭屏时查 keyguard」是竞态。改为 SCREEN_OFF → USER_PRESENT 事件对：
 * - ACTION_SCREEN_OFF（不区分电源键/超时自动灭屏）：置 pending 标记；
 * - ACTION_USER_PRESENT：keyguard 被解除时系统发出，此刻进程必定解冻。
 *   仅当设备配置了安全锁屏（isKeyguardSecure：PIN/图案/密码/生物识别）
 *   才确认"确实锁过屏"并触发锁定，随后清除 pending。
 *
 * 锁定标记（lockedWhileBackgrounded + 持久化）只在 JS 确认后由
 * dismissLockMask 清除：trigger 可能因 WebView 冻结/渲染进程被回收而丢失，
 * 未确认前每次 onResume 都会重新挂遮罩并补达事件；前端也可通过
 * getLockPending 主动拉取，保证最终必然收敛到「锁定 + 无遮罩」。
 *
 * 检测到锁屏时先给窗口加不透明遮盖（decorView.foreground），
 * 避免 WebView 解冻首帧泄露旧页面内容；前端进入登录页后调用
 * dismissLockMask 撤掉遮盖并清除全部标记。
 */
@TauriPlugin
class LockStatePlugin(private val activity: Activity): Plugin(activity) {

    /**
     * 标记灭屏期间是否检测到锁屏。注意：不在 trigger 时清除，
     * 只有 JS 确认（dismissLockMask）后才清除，防止事件丢失后状态失联。
     */
    private var lockedWhileBackgrounded = false

    /** 锁屏遮盖是否已显示（仅用于 show 幂等；hide 不设门槛，随时可撤）。 */
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

        /**
         * 进程级 receiver 单例标记：receiver 的职责要求其跨 Activity 存活
         * （Activity 销毁/重建后仍需捕获后台灭屏与解锁广播），因此只在
         * 进程内首次 init 时注册到 applicationContext，不随插件实例重复注册。
         */
        private var receiverRegistered = false

        /**
         * 当前存活的插件实例。receiver 收到广播后委托给它处理；
         * Activity/插件实例重建后 init 自动指向新实例，不持有旧 Activity。
         */
        @Volatile
        private var activeInstance: LockStatePlugin? = null

        private val screenStateReceiver = object : BroadcastReceiver() {
            override fun onReceive(context: Context, intent: Intent) {
                val plugin = activeInstance ?: return
                when (intent.action) {
                    Intent.ACTION_SCREEN_OFF -> plugin.onScreenOff()
                    Intent.ACTION_USER_PRESENT -> plugin.onUserPresent()
                }
            }
        }
    }

    init {
        activeInstance = this
        // 进程级单例注册。必须使用 RECEIVER_EXPORTED：这些系统广播在
        // Android 13+ 用 NOT_EXPORTED 会收不到；系统广播第三方无法伪造，导出安全。
        if (!receiverRegistered) {
            val filter = IntentFilter(Intent.ACTION_SCREEN_OFF).apply {
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
            receiverRegistered = true
        }
    }

    private fun onScreenOff() {
        setPendingFlag(true)
        // 部分 ROM 灭屏即锁，keyguard 可能已置位，同步 + 延迟各查一次
        checkKeyguardAndTrigger()
        Handler(Looper.getMainLooper()).postDelayed({
            if (activity.isFinishing || activity.isDestroyed) return@postDelayed
            checkKeyguardAndTrigger()
        }, 300)
    }

    private fun onUserPresent() {
        // keyguard 被解除才发出此广播，此刻进程必定解冻。
        // 以 isKeyguardSecure（设备配置了安全锁屏）判定"确实锁过屏"，
        // 无安全锁屏设备的纯灭屏不触发锁定。
        if (getPendingFlag()) {
            setPendingFlag(false)
            if (isKeyguardSecure()) {
                markLockedAndTrigger()
            }
        }
    }

    // ── 插件基类生命周期钩子（Tauri 自动派发，无需自行注册，无泄漏） ──

    override fun onPause() {
        // onPause 时同步检查一次（按电源键灭屏时 keyguard 可能已置位）
        checkKeyguardAndTrigger()
        // keyguard 可能在 onPause 之后才置位，延迟 500ms 再查一次
        Handler(Looper.getMainLooper()).postDelayed({
            if (activity.isFinishing || activity.isDestroyed) return@postDelayed
            checkKeyguardAndTrigger()
        }, 500)
    }

    override fun onStop() {
        // onStop 作为 onPause 的补充检测点
        checkKeyguardAndTrigger()
    }

    override fun onResume() {
        // 灭屏期间 WebView 冻结/渲染进程被回收都可能导致 trigger 丢失，
        // 只要标记未被 JS 确认清除，回前台就补达事件。
        // 遮罩立即挂上（赶在首帧前）；trigger 延迟少量时间等 WebView
        // 恢复 JS 处理，过早发出会丢失（前端另有回前台拉取兜底）。
        if (lockedWhileBackgrounded || restoreLockedFlag()) {
            showLockMask()
            Handler(Looper.getMainLooper()).postDelayed({
                if (activity.isFinishing || activity.isDestroyed) return@postDelayed
                if (lockedWhileBackgrounded || restoreLockedFlag()) {
                    markLockedAndTrigger(force = true)
                }
            }, 400)
        }
    }

    private fun keyguardManager(): KeyguardManager? =
        activity.getSystemService(Context.KEYGUARD_SERVICE) as? KeyguardManager

    private fun isKeyguardLocked(): Boolean = keyguardManager()?.isKeyguardLocked == true

    private fun isKeyguardSecure(): Boolean = keyguardManager()?.isKeyguardSecure == true

    /**
     * 检查 keyguard 状态，若锁屏则进入锁定分支。
     * 在 onPause/onStop、SCREEN_OFF 同步与 postDelayed 重试中调用。
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
     *
     * 检测路径幂等：force=false 时若标记已置位说明已触发过，
     * 直接短路，避免 SCREEN_OFF/onPause/onStop 多检测点重复发事件；
     * onResume 补达显式 force=true（"JS 确认制"的补达语义依赖它）。
     * 标记不清除——等 JS 通过 dismissLockMask 确认。
     */
    private fun markLockedAndTrigger(force: Boolean = false) {
        if (!force && (lockedWhileBackgrounded || restoreLockedFlag())) return
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
     * 灭屏挂起标记：SCREEN_OFF 置位，USER_PRESENT 确认后清除。
     */
    private fun setPendingFlag(value: Boolean) {
        prefs.edit().putBoolean(PREF_SCREEN_OFF_PENDING, value).apply()
    }

    private fun getPendingFlag(): Boolean {
        return prefs.getBoolean(PREF_SCREEN_OFF_PENDING, false)
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

    /**
     * 撤掉锁屏遮盖。不设实例态门槛：foreground = null 幂等，
     * 即使遮盖由已重建的旧插件实例挂上也能撤掉。
     */
    private fun hideLockMask() {
        try {
            onUi {
                activity.window.decorView.foreground = null
                lockMaskShown = false
            }
        } catch (e: Exception) {
            android.util.Log.e("SoloSoul", "LockStatePlugin hideLockMask failed", e)
        }
    }

    /**
     * 查询是否有未被 JS 确认的锁屏挂起标记。
     * 前端启动/认证后主动拉取，闭合「事件已丢失但标记仍在」的环路。
     */
    @Command
    fun getLockPending(invoke: Invoke) {
        try {
            val result = JSObject()
            result.put("pending", lockedWhileBackgrounded || restoreLockedFlag())
            invoke.resolve(result)
        } catch (e: Exception) {
            android.util.Log.e("SoloSoul", "LockStatePlugin.getLockPending failed", e)
            invoke.reject("getLockPending failed: ${e.message}")
        }
    }

    /**
     * JS 确认点：撤掉锁屏遮盖并清除全部锁屏/挂起标记。
     * 前端完成锁定进入登录页后调用。
     */
    @Command
    fun dismissLockMask(invoke: Invoke) {
        try {
            hideLockMask()
            lockedWhileBackgrounded = false
            persistLockedFlag(false)
            setPendingFlag(false)
            invoke.resolve(JSObject())
        } catch (e: Exception) {
            android.util.Log.e("SoloSoul", "LockStatePlugin.dismissLockMask failed", e)
            invoke.reject("dismissLockMask failed: ${e.message}")
        }
    }
}
