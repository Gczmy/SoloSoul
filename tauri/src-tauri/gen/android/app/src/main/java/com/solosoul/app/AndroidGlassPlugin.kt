package com.solosoul.app

import android.app.Activity
import android.app.Dialog
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Path
import android.graphics.RectF
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.os.Build
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.view.Window
import android.view.WindowManager
import android.webkit.WebView
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.WindowCompat
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleOwner
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.util.function.Consumer

@InvokeArg
class GlassLabels { var `object`: String = ""; var page: String = ""; var scan: String = "" }
@InvokeArg
class GlassMenuArgs {
    var requestId: String = ""
    var title: String = ""
    var description: String = ""
    var closeLabel: String = ""
    var footer: String = ""
    var labels: GlassLabels = GlassLabels()
    var descriptions: GlassLabels = GlassLabels()
    var dark: Boolean = false
    var reduceMotion: Boolean = false
    var background: String = "#F9F9F3"
    var foreground: String = "#20251F"
    var secondary: String = "#626B62"
    var accent: String = "#405F82"
    var container: String = "#D9E7F8"
}
@InvokeArg
class CloseGlassArgs { var requestId: String = "" }

/** 独立 Dialog 只负责快捷动作；主 Activity/WebView 始终保留原来的不透明背景。 */
@TauriPlugin
class AndroidGlassPlugin(private val activity: Activity) : Plugin(activity) {
    private val wm = activity.getSystemService(WindowManager::class.java)
    private var activeDialog: Dialog? = null
    private var activeArgs: GlassMenuArgs? = null
    private var pendingInvoke: Invoke? = null
    private var windowBackground: GradientDrawable? = null
    private var blurListener: Consumer<Boolean>? = null
    private var destroyed = false
    // close 可能先于异步 show 抵达；保留有限的已取消请求，避免旧菜单重新出现。
    private val cancelled = linkedSetOf<String>()

    private fun dp(value: Int) = (value * activity.resources.displayMetrics.density).toInt()
    private fun supported() = Build.VERSION.SDK_INT >= 31 && wm.isCrossWindowBlurEnabled
    private fun snapshot() = JSObject().apply {
        put("apiLevel", Build.VERSION.SDK_INT)
        put("windowBlur", supported())
        put("webViewVersion", WebView.getCurrentWebViewPackage()?.versionName ?: "")
    }

    override fun load(webView: WebView) {
        if (Build.VERSION.SDK_INT >= 31) {
            val listener = Consumer<Boolean> {
                if (!destroyed) {
                    updateBlur()
                    trigger("capabilities-changed", snapshot())
                }
            }
            blurListener = listener
            wm.addCrossWindowBlurEnabledListener(activity.mainExecutor, listener)
        }
    }

    @Command
    fun capabilities(invoke: Invoke) { invoke.resolve(snapshot()) }

    @Command
    fun showMenu(invoke: Invoke) {
        val args = invoke.parseArgs(GlassMenuArgs::class.java)
        activity.runOnUiThread {
            val resumed = (activity as? LifecycleOwner)?.lifecycle?.currentState
                ?.isAtLeast(Lifecycle.State.RESUMED) ?: activity.hasWindowFocus()
            if (destroyed || activity.isFinishing || activity.isDestroyed || !resumed || cancelled.contains(args.requestId)) {
                resolve(invoke, args.requestId, "cancel")
                return@runOnUiThread
            }
            if (!supported()) {
                resolve(invoke, args.requestId, "unavailable")
                return@runOnUiThread
            }
            finish("cancel")
            val dialog = Dialog(activity, R.style.Theme_SoloSoul_GlassMenu)
            activeDialog = dialog
            activeArgs = args
            pendingInvoke = invoke
            try {
                dialog.requestWindowFeature(Window.FEATURE_NO_TITLE)
                dialog.setCancelable(true)
                dialog.setCanceledOnTouchOutside(true)
                dialog.setContentView(menuContent(args))
                dialog.setOnCancelListener { finish("cancel") }
                dialog.setOnDismissListener { if (activeDialog === dialog) finish("cancel") }
                val window = dialog.window ?: error("Missing dialog window")
                windowBackground = rounded(Color.parseColor(args.background), 30)
                window.setBackgroundDrawable(windowBackground)
                window.decorView.clipToOutline = true
                window.addFlags(WindowManager.LayoutParams.FLAG_DIM_BEHIND or WindowManager.LayoutParams.FLAG_BLUR_BEHIND)
                // Dialog 使用系统默认的 inset 避让；三键导航、分屏 caption 和手势区由系统处理。
                WindowCompat.setDecorFitsSystemWindows(window, true)
                val available = if (Build.VERSION.SDK_INT >= 30) wm.currentWindowMetrics.bounds.width()
                    else activity.resources.displayMetrics.widthPixels
                window.setLayout(minOf(available - dp(24), dp(640)), ViewGroup.LayoutParams.WRAP_CONTENT)
                window.setGravity(Gravity.BOTTOM or Gravity.CENTER_HORIZONTAL)
                window.attributes = window.attributes.apply {
                    y = dp(6)
                    windowAnimations = 0 // 先显示稳定窗口，避免平台动画与模糊首帧重复合成。
                }
                updateBlur()
                // 背景、圆角与模糊在窗口附着之前就准备好，首帧直接采用最终材质。
                dialog.show()
                // 附着后再覆盖 Dialog 主题的系统栏外观，仍在首次绘制之前完成。
                window.addFlags(WindowManager.LayoutParams.FLAG_DRAWS_SYSTEM_BAR_BACKGROUNDS)
                val controller = WindowCompat.getInsetsController(window, window.decorView)
                controller.isAppearanceLightStatusBars = !args.dark
                controller.isAppearanceLightNavigationBars = !args.dark
            } catch (_: Exception) {
                finish("unavailable")
            }
        }
    }

    @Command
    fun closeMenu(invoke: Invoke) {
        val args = invoke.parseArgs(CloseGlassArgs::class.java)
        activity.runOnUiThread {
            cancelled.add(args.requestId)
            if (cancelled.size > 64) cancelled.remove(cancelled.first())
            if (activeArgs?.requestId == args.requestId) finish("cancel")
            invoke.resolve(JSObject())
        }
    }

    private fun updateBlur() {
        val args = activeArgs ?: return
        val window = activeDialog?.window ?: return
        val enabled = supported()
        windowBackground?.alpha = if (enabled) (255 * if (args.dark) .80 else .76).toInt() else 255
        window.setDimAmount(if (enabled) .12f else .40f)
        if (Build.VERSION.SDK_INT >= 31) {
            window.setBackgroundBlurRadius(if (enabled) dp(24) else 0)
            window.attributes = window.attributes.apply { blurBehindRadius = if (enabled) dp(4) else 0 }
        }
    }

    private fun resolve(invoke: Invoke, requestId: String, action: String) {
        invoke.resolve(JSObject().apply { put("requestId", requestId); put("action", action) })
    }

    private fun finish(action: String) {
        val invoke = pendingInvoke
        val id = activeArgs?.requestId
        val dialog = activeDialog
        // 先清空所有权，dismiss 回调不重复 resolve，也不能关闭下一次请求。
        pendingInvoke = null; activeArgs = null; activeDialog = null; windowBackground = null
        dialog?.setOnDismissListener(null)
        dialog?.setOnCancelListener(null)
        // Activity 已销毁/窗口已被系统移除时也要完成待处理请求。
        runCatching { dialog?.dismiss() }
        if (invoke != null && id != null) resolve(invoke, id, action)
    }

    override fun onPause() { finish("cancel") }
    override fun onStop() { finish("cancel") }
    override fun onDestroy(activity: AppCompatActivity) {
        destroyed = true
        finish("cancel")
        if (Build.VERSION.SDK_INT >= 31) blurListener?.let { wm.removeCrossWindowBlurEnabledListener(it) }
        blurListener = null
        cancelled.clear()
    }

    private fun rounded(color: Int, radius: Int) = GradientDrawable().apply {
        shape = GradientDrawable.RECTANGLE
        cornerRadius = dp(radius).toFloat()
        setColor(color)
    }

    private fun text(value: String, size: Float, color: Int, bold: Boolean = false) = TextView(activity).apply {
        this.text = value; textSize = size; setTextColor(color)
        if (bold) typeface = Typeface.create("sans-serif-medium", Typeface.NORMAL)
    }

    private fun menuContent(args: GlassMenuArgs): View {
        val foreground = Color.parseColor(args.foreground)
        val secondary = Color.parseColor(args.secondary)
        val accent = Color.parseColor(args.accent)
        val column = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), dp(12), dp(20), dp(20))
        }
        column.addView(View(activity).apply { background = rounded(secondary, 4); alpha = .4f },
            LinearLayout.LayoutParams(dp(32), dp(4)).apply { gravity = Gravity.CENTER_HORIZONTAL; bottomMargin = dp(8) })
        val header = LinearLayout(activity).apply { gravity = Gravity.CENTER_VERTICAL }
        header.addView(text(args.title, 22f, foreground, true), LinearLayout.LayoutParams(0, -2, 1f))
        header.addView(GlyphView("close", foreground).apply {
            contentDescription = args.closeLabel
            isFocusable = true; isClickable = true
            setOnClickListener { finish("cancel") }
        }, LinearLayout.LayoutParams(dp(48), dp(48)))
        column.addView(header)
        column.addView(text(args.description, 13f, secondary), LinearLayout.LayoutParams(-1, -2).apply { bottomMargin = dp(14) })
        val labels = listOf(args.labels.`object`, args.labels.page, args.labels.scan)
        val descriptions = listOf(args.descriptions.`object`, args.descriptions.page, args.descriptions.scan)
        listOf("object", "page", "scan").forEachIndexed { index, action ->
            val row = LinearLayout(activity).apply {
                gravity = Gravity.CENTER_VERTICAL
                minimumHeight = dp(76)
                setPadding(dp(12), dp(14), dp(12), dp(14))
                background = rounded(Color.parseColor(if (index == 0) args.container else args.background), 18)
                isClickable = true; isFocusable = true
                contentDescription = "${labels[index]}. ${descriptions[index]}"
                setOnClickListener { finish(action) }
            }
            row.addView(GlyphView(action, accent).apply { importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO },
                LinearLayout.LayoutParams(dp(36), dp(36)).apply { marginEnd = dp(12) })
            val copy = LinearLayout(activity).apply { orientation = LinearLayout.VERTICAL; importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO_HIDE_DESCENDANTS }
            copy.addView(text(labels[index], 16f, foreground, true))
            copy.addView(text(descriptions[index], 12f, secondary), LinearLayout.LayoutParams(-1, -2).apply { topMargin = dp(5) })
            row.addView(copy, LinearLayout.LayoutParams(0, -2, 1f))
            column.addView(row, LinearLayout.LayoutParams(-1, -2).apply { bottomMargin = dp(6) })
        }
        column.addView(text(args.footer, 12f, secondary), LinearLayout.LayoutParams(-1, -2).apply { topMargin = dp(12) })
        return ScrollView(activity).apply {
            isFillViewport = false
            addView(column)
            // 字体放大/横屏时菜单可滚动，始终给上方系统栏留空间。
            layoutParams = ViewGroup.LayoutParams(-1, -2)
        }
    }

    /** 固定几何的线形图标，不依赖系统字体中的字符图案。 */
    private inner class GlyphView(private val glyph: String, color: Int) : View(activity) {
        private val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply { this.color = color; style = Paint.Style.STROKE; strokeWidth = 1.7f; strokeCap = Paint.Cap.ROUND; strokeJoin = Paint.Join.ROUND }
        override fun onDraw(canvas: Canvas) {
            super.onDraw(canvas)
            canvas.save(); canvas.translate((width - dp(24)) / 2f, (height - dp(24)) / 2f); canvas.scale(dp(24) / 24f, dp(24) / 24f)
            val path = Path()
            when (glyph) {
                "close" -> { path.moveTo(6f,6f); path.lineTo(18f,18f); path.moveTo(18f,6f); path.lineTo(6f,18f) }
                "object" -> { path.moveTo(14f,3f); path.lineTo(5f,3f); path.lineTo(5f,21f); path.lineTo(19f,21f); path.lineTo(19f,8f); path.close(); path.moveTo(14f,3f); path.lineTo(14f,9f); path.lineTo(19f,9f); path.moveTo(8f,13f);path.lineTo(15f,13f);path.moveTo(8f,17f);path.lineTo(13f,17f) }
                "page" -> { path.moveTo(3f,7f);path.lineTo(3f,4f);path.lineTo(10f,4f);path.lineTo(13f,7f);path.lineTo(21f,7f);path.lineTo(21f,20f);path.lineTo(3f,20f);path.close();path.moveTo(12f,11f);path.lineTo(12f,17f);path.moveTo(9f,14f);path.lineTo(15f,14f) }
                else -> { for ((x,y) in listOf(3f to 3f,16f to 3f,3f to 16f,16f to 16f)) canvas.drawRoundRect(RectF(x,y,x+5,y+5),1f,1f,paint); path.moveTo(3f,12f);path.lineTo(21f,12f) }
            }
            canvas.drawPath(path,paint);canvas.restore()
        }
    }
}
