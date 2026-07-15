package com.solosoul.app

import android.app.Activity
import android.view.Window
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsControllerCompat
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import app.tauri.plugin.Invoke

@InvokeArg
class StatusBarStyleArgs {
    lateinit var style: String
}

/**
 * 状态栏/导航栏图标颜色控制插件。
 *
 * 前端在切换应用主题时调用 `setStyle`，参数为 `"dark"` 或 `"light"`：
 * - `"dark"`：应用使用深色主题，系统栏图标/文字应为浅色（白色）。
 * - `"light"`：应用使用浅色主题，系统栏图标/文字应为深色（黑色）。
 */
@TauriPlugin
class StatusBarPlugin(private val activity: Activity): Plugin(activity) {
    @Command
    fun setStyle(invoke: Invoke) {
        val args = invoke.parseArgs(StatusBarStyleArgs::class.java)
        val window: Window = activity.window
        val rootView = window.decorView.rootView
        val controller = WindowCompat.getInsetsController(window, rootView)
        if (controller == null) {
            android.util.Log.w("SoloSoul", "StatusBarPlugin.setStyle: WindowInsetsController is null")
            invoke.resolve(JSObject())
            return
        }

        // style 为 "dark" 时应用背景深，图标应设为浅色（isAppearanceLightStatusBars = false）。
        // style 为 "light" 时应用背景浅，图标应设为深色（isAppearanceLightStatusBars = true）。
        val isLight = args.style == "light"
        controller.isAppearanceLightStatusBars = isLight
        controller.isAppearanceLightNavigationBars = isLight

        invoke.resolve(JSObject())
    }
}
