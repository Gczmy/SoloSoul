package com.solosoul.app

import android.app.Activity
import android.app.KeyguardManager
import android.content.Context
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.JSObject
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin

/**
 * 锁屏状态检测插件。
 *
 * 通过 KeyguardManager.isKeyguardLocked 判断当前设备是否处于锁屏状态。
 * 前端在 `visibilitychange` 隐藏时调用，用于区分「系统锁屏」与「仅切后台」。
 */
@TauriPlugin
class LockStatePlugin(private val activity: Activity): Plugin(activity) {
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
