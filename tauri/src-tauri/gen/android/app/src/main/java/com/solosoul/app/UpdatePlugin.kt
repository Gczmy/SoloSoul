package com.solosoul.app

import android.app.Activity
import android.content.Intent
import android.net.Uri
import androidx.core.content.FileProvider
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.io.File

/**
 * Android 应用内更新插件。
 *
 * 负责将 Rust 后端下载到缓存目录的 APK 文件，通过 FileProvider 暴露给系统
 * 安装器，启动 `ACTION_VIEW` Intent 触发安装流程。
 *
 * 对应 Rust 命令 `android_install_apk`。
 */
@TauriPlugin
class UpdatePlugin(private val activity: Activity): Plugin(activity) {

    /**
     * 启动系统包安装器安装指定路径的 APK。
     *
     * @param invoke 包含参数 `filePath`（APK 文件的绝对路径）。
     */
    @Command
    fun installApk(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(InstallApkArgs::class.java)
            val apkFile = File(args.filePath)

            if (!apkFile.exists()) {
                invoke.reject("APK 文件不存在: ${args.filePath}")
                return
            }

            val authority = "${activity.packageName}.fileprovider"
            val apkUri: Uri = FileProvider.getUriForFile(activity, authority, apkFile)

            val intent = Intent(Intent.ACTION_VIEW).apply {
                setDataAndType(apkUri, "application/vnd.android.package-archive")
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                // Android 14+ 需要此项以允许系统安装器访问 URI
                addFlags(Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION)
            }

            // 预检：确认系统有应用可处理此 Intent
            if (intent.resolveActivity(activity.packageManager) == null) {
                invoke.reject("NO_PACKAGE_INSTALLER: 未找到包安装器")
                return
            }

            activity.startActivity(intent)
            invoke.resolve(JSObject().apply {
                put("success", true)
            })
        } catch (e: Exception) {
            android.util.Log.e("SoloSoul", "installApk failed: ${e.message}", e)
            invoke.reject("INSTALL_APK_FAILED: ${e.message}")
        }
    }
}

/**
 * installApk 命令的参数。
 */
class InstallApkArgs {
    lateinit var filePath: String
}
