package com.solosoul.app

import android.graphics.Bitmap
import android.graphics.Color
import android.graphics.pdf.PdfRenderer
import android.os.Bundle
import android.os.ParcelFileDescriptor
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsControllerCompat
import androidx.core.view.doOnLayout
import java.io.File
import java.io.FileOutputStream
import java.io.IOException

/**
 * 原生 PDF 预览 Activity。
 *
 * 使用 Android PdfRenderer 将 PDF 页面渲染为 Bitmap，避免依赖外部 PDF 阅读器。
 * 支持上一页/下一页导航，并确保系统状态栏可见且图标为浅色。
 */
class PdfPreviewActivity : AppCompatActivity() {

    private lateinit var pageImage: ImageView
    private lateinit var titleView: TextView
    private lateinit var pageInfoView: TextView
    private lateinit var prevButton: ImageButton
    private lateinit var nextButton: ImageButton

    private var pdfRenderer: PdfRenderer? = null
    private var currentPage: PdfRenderer.Page? = null
    private var parcelFileDescriptor: ParcelFileDescriptor? = null

    private var pageCount: Int = 0
    private var currentIndex: Int = 0
    private var tempPdfFile: File? = null

    companion object {
        const val EXTRA_PATH = "pdf_path"
        const val EXTRA_TITLE = "pdf_title"
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_pdf_preview)

        // 状态栏与导航栏保持可见，图标/文字设为浅色（深色背景）。
        window.statusBarColor = Color.parseColor("#1F1C18")
        window.navigationBarColor = Color.parseColor("#1F1C18")
        val controller = WindowCompat.getInsetsController(window, window.decorView)
        controller.isAppearanceLightStatusBars = false
        controller.isAppearanceLightNavigationBars = false

        pageImage = findViewById(R.id.pdfPageImage)
        titleView = findViewById(R.id.pdfTitle)
        pageInfoView = findViewById(R.id.pdfPageInfo)
        prevButton = findViewById(R.id.pdfPrevButton)
        nextButton = findViewById(R.id.pdfNextButton)
        val closeButton: ImageButton = findViewById(R.id.pdfCloseButton)

        closeButton.setOnClickListener { finish() }
        prevButton.setOnClickListener { showPage(currentIndex - 1) }
        nextButton.setOnClickListener { showPage(currentIndex + 1) }

        val path = intent.getStringExtra(EXTRA_PATH)
        val title = intent.getStringExtra(EXTRA_TITLE)
        if (path.isNullOrEmpty()) {
            finish()
            return
        }
        titleView.text = title ?: File(path).name

        if (!preparePdfFile(path)) {
            finish()
            return
        }
        openRenderer()
        // 关键：必须等首次布局完成后才能读取 pageImage.width。
        // 若在 onCreate 中立即 showPage(0)，此时 width==0，会渲染出 1×1 位图，
        // fitCenter 拉伸后显示为黑色块（PdfRenderer 经典时序 bug）。
        pageImage.doOnLayout {
            showPage(0)
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        closeRenderer()
        tempPdfFile?.let {
            try {
                it.delete()
            } catch (_: Exception) {
                // 忽略清理失败
            }
        }
    }

    /**
     * Vault 文件位于应用私有数据根目录（可能不在 filesDir 下），
     * PdfRenderer 需要可直接访问的本地文件描述符。这里先把文件复制到
     * filesDir 下的临时目录，再打开渲染器；退出时自动清理临时文件。
     */
    private fun preparePdfFile(originalPath: String): Boolean {
        return try {
            val srcFile = File(originalPath)
            if (!srcFile.exists()) {
                return false
            }
            val tempDir = File(filesDir, "pdf_preview_temp").apply { mkdirs() }
            val tempFile = File(tempDir, "${System.currentTimeMillis()}_${srcFile.name}")
            FileOutputStream(tempFile).use { output ->
                srcFile.inputStream().use { input ->
                    input.copyTo(output)
                }
            }
            tempPdfFile = tempFile
            true
        } catch (e: IOException) {
            android.util.Log.e("SoloSoul", "preparePdfFile failed: ${e.message}", e)
            false
        }
    }

    private fun openRenderer() {
        val file = tempPdfFile ?: return
        try {
            parcelFileDescriptor = ParcelFileDescriptor.open(file, ParcelFileDescriptor.MODE_READ_ONLY)
            pdfRenderer = PdfRenderer(parcelFileDescriptor!!)
            pageCount = pdfRenderer!!.pageCount
        } catch (e: Exception) {
            android.util.Log.e("SoloSoul", "openRenderer failed: ${e.message}", e)
            finish()
        }
    }

    private fun closeRenderer() {
        try {
            currentPage?.close()
            currentPage = null
            pdfRenderer?.close()
            pdfRenderer = null
            parcelFileDescriptor?.close()
            parcelFileDescriptor = null
        } catch (_: Exception) {
            // 忽略关闭失败
        }
    }

    private fun showPage(index: Int) {
        val renderer = pdfRenderer ?: return
        if (index < 0 || index >= pageCount) return

        currentPage?.close()
        currentPage = null

        val page = renderer.openPage(index)
        currentPage = page
        currentIndex = index

        // 按视图宽度等比缩放，避免内存浪费。
        // 兜底：极端时序下 width 可能仍为 0（如布局未完成），回退到屏幕宽度，
        // 保证位图尺寸有效，绝不渲染 1×1 位图。
        val viewWidth = if (pageImage.width > 0) pageImage.width else resources.displayMetrics.widthPixels
        val scale = viewWidth.toFloat() / page.width.coerceAtLeast(1)
        val bitmapWidth = (page.width * scale).toInt().coerceAtLeast(1)
        val bitmapHeight = (page.height * scale).toInt().coerceAtLeast(1)

        val bitmap = Bitmap.createBitmap(bitmapWidth, bitmapHeight, Bitmap.Config.ARGB_8888)
        page.render(bitmap, null, null, PdfRenderer.Page.RENDER_MODE_FOR_DISPLAY)
        pageImage.setImageBitmap(bitmap)

        updateControls()
    }

    private fun updateControls() {
        pageInfoView.text = "${currentIndex + 1} / $pageCount"
        prevButton.isEnabled = currentIndex > 0
        nextButton.isEnabled = currentIndex < pageCount - 1
        prevButton.alpha = if (prevButton.isEnabled) 1.0f else 0.4f
        nextButton.alpha = if (nextButton.isEnabled) 1.0f else 0.4f
    }
}
