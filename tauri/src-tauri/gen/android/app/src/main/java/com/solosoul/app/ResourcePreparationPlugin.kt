package com.solosoul.app

import android.app.Activity
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

/** Rust 消费者异步等待当前准备结果；等待期间不在主线程做安装或阻塞。 */
@TauriPlugin
class ResourcePreparationPlugin(private val activity: Activity) : Plugin(activity) {
  @Command
  fun waitUntilReady(invoke: Invoke) {
    AndroidResources.prepare(activity)
    AndroidResources.waitUntilComplete { state ->
      val response = JSObject().apply {
        put("status", state.status)
        put("errorCode", state.errorCode)
      }
      activity.runOnUiThread { invoke.resolve(response) }
    }
  }
}
