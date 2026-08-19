// Copyright 2019-2024 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

// SoloSoul PATCH（backport tauri upstream #15798「fix(android): use whichever activity in plugin manager」）
//
// 修复：Activity 销毁重建（配置变更/内存回收）后 ActivityResultLauncher 未重新注册，
// 导致 SAF 目录选择器等 startActivityForResult 的结果丢失、插件 invoke 永久挂起
// （安卓端选择保险库外部目录一直"加载中"、无法进入下一步）。
//
// 改动：用集合跟踪全部已创建 Activity；onActivityCreate 仅在尚无当前 Activity 时注册
// launcher；onDestroy 时把 launcher 迁移到下一个可用 Activity（原实现绑定进程内
// 第一个 Activity 实例，销毁重建后结果无人接收）。activity 字段由 lateinit 改为可空。
//
// 上游修复尚未发布（tauri 2.11.5 为最新已发布版），因此以构建时补丁落地（见
// gen/android/build.gradle.kts 的 patchTauriAndroidPluginManager 任务）。
// 升级 tauri 至含 #15798 的版本后应删除本补丁。同步修改见 PluginHandle.kt。

package app.tauri.plugin

import android.content.res.Configuration
import android.content.Context
import android.content.Intent
import android.webkit.WebView
import androidx.activity.result.ActivityResult
import androidx.activity.result.ActivityResultLauncher
import androidx.activity.result.IntentSenderRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import app.tauri.annotation.InvokeArg
import app.tauri.FsUtils
import app.tauri.JniMethod
import app.tauri.Logger
import com.fasterxml.jackson.annotation.JsonAutoDetect
import com.fasterxml.jackson.annotation.PropertyAccessor
import com.fasterxml.jackson.databind.DeserializationFeature
import com.fasterxml.jackson.databind.JsonNode
import com.fasterxml.jackson.databind.ObjectMapper
import com.fasterxml.jackson.databind.module.SimpleModule
import java.lang.reflect.InvocationTargetException

object PluginManager {
  fun interface RequestPermissionsCallback {
    fun onResult(permissions: Map<String, Boolean>)
  }

  fun interface ActivityResultCallback {
    fun onResult(result: ActivityResult)
  }

  // SoloSoul PATCH: 跟踪所有已创建 Activity；当前 Activity 可空（全部销毁时为 null）
  private val activities: HashSet<AppCompatActivity> = HashSet()
  var activity: AppCompatActivity? = null
  private val plugins: HashMap<String, PluginHandle> = HashMap()
  private var startActivityForResultLauncher: ActivityResultLauncher<Intent>? = null
  private var startIntentSenderForResultLauncher: ActivityResultLauncher<IntentSenderRequest>? = null
  private var requestPermissionsLauncher: ActivityResultLauncher<Array<String>>? = null
  private var requestPermissionsCallback: RequestPermissionsCallback? = null
  private var startActivityForResultCallback: ActivityResultCallback? = null
  private var startIntentSenderForResultCallback: ActivityResultCallback? = null
  private var jsonMapper: ObjectMapper = ObjectMapper()
    .disable(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES)
    .enable(DeserializationFeature.FAIL_ON_NULL_FOR_PRIMITIVES)
    .setVisibility(PropertyAccessor.FIELD, JsonAutoDetect.Visibility.ANY)

  init {
    val channelDeserializer = ChannelDeserializer({ channelId, payload ->
      sendChannelData(channelId, payload)
    }, jsonMapper)
    jsonMapper
      .registerModule(SimpleModule().addDeserializer(Channel::class.java, channelDeserializer))
  }

  // SoloSoul PATCH: 记录 Activity；仅当尚无当前 Activity 时才注册 launcher，
  // 避免绑定到第一个（可能已销毁的）Activity 实例
  fun onActivityCreate(activity: AppCompatActivity) {
    activities.add(activity)
    if (this.activity == null) {
      this.activity = activity
      registerResultLaunchers(activity)
    }
  }

  // SoloSoul PATCH: launcher 注册逻辑抽离，供 onActivityCreate / onDestroy 复用
  private fun registerResultLaunchers(activity: AppCompatActivity) {
    startActivityForResultLauncher =
      activity.registerForActivityResult(ActivityResultContracts.StartActivityForResult()
      ) { result ->
        startActivityForResultCallback?.onResult(result)
      }

    startIntentSenderForResultLauncher =
      activity.registerForActivityResult(ActivityResultContracts.StartIntentSenderForResult()
      ) { result ->
        startIntentSenderForResultCallback?.onResult(result)
      }

    requestPermissionsLauncher =
      activity.registerForActivityResult(ActivityResultContracts.RequestMultiplePermissions()
      ) { result ->
        requestPermissionsCallback?.onResult(result)
      }
  }

  fun onNewIntent(intent: Intent) {
    for (plugin in plugins.values) {
      plugin.instance.onNewIntent(intent)
    }
  }

  fun onPause() {
    for (plugin in plugins.values) {
      plugin.instance.onPause()
    }
  }

  fun onResume() {
    for (plugin in plugins.values) {
      plugin.instance.onResume()
    }
  }

  fun onRestart(activity: AppCompatActivity) {
    for (plugin in plugins.values) {
      plugin.instance.triggerOnRestart(activity)
    }
  }

  fun onStop() {
    for (plugin in plugins.values) {
      plugin.instance.onStop()
    }
  }

  // SoloSoul PATCH: Activity 销毁时从集合移除；若它正是当前 Activity，把 launcher
  // 迁移到下一个可用 Activity；无可用则清空，等待下次 onCreate 重新注册
  fun onDestroy(activity: AppCompatActivity) {
    activities.remove(activity)
    val nextActivity = activities.firstOrNull()
    if (nextActivity != null) {
      if (this.activity == activity) {
        this.activity = nextActivity
        registerResultLaunchers(nextActivity)
      }
    } else {
      this.activity = null
      this.startActivityForResultLauncher = null
      this.startIntentSenderForResultLauncher = null
      this.requestPermissionsLauncher = null
    }
  }

  fun onConfigurationChanged(newConfig: Configuration) {
    for (plugin in plugins.values) {
      plugin.instance.onConfigurationChanged(newConfig)
    }
  }

  fun startActivityForResult(intent: Intent, callback: ActivityResultCallback) {
    startActivityForResultCallback = callback
    startActivityForResultLauncher!!.launch(intent)
  }

  fun startIntentSenderForResult(intent: IntentSenderRequest, callback: ActivityResultCallback) {
    startIntentSenderForResultCallback = callback
    startIntentSenderForResultLauncher!!.launch(intent)
  }

  fun requestPermissions(
    permissionStrings: Array<String>,
    callback: RequestPermissionsCallback
  ) {
    requestPermissionsCallback = callback
    requestPermissionsLauncher!!.launch(permissionStrings)
  }

  @JniMethod
  fun onWebViewCreated(webView: WebView) {
    for ((_, plugin) in plugins) {
      if (!plugin.loaded) {
        plugin.load(webView)
      }
    }
  }

  @JniMethod
  fun load(webView: WebView?, name: String, plugin: Plugin, config: String) {
    val handle = PluginHandle(this, name, plugin, config, jsonMapper)
    plugins[name] = handle
    if (webView != null) {
      plugin.load(webView)
    }
  }

  @JniMethod
  fun runCommand(id: Int, pluginId: String, command: String, data: String) {
    val successId = 0L
    val errorId = 1L
    val invoke = Invoke(id.toLong(), command, successId, errorId, { fn, result ->
      var success: String? = null
      var error: String? = null
      if (fn == successId) {
        success = result
      } else {
        error = result
      }
      handlePluginResponse(id, success, error)
    }, data, jsonMapper)

    dispatchPluginMessage(invoke, pluginId)
  }

  private fun dispatchPluginMessage(invoke: Invoke, pluginId: String) {
    Logger.verbose(
      Logger.tags("Plugin"),
      "Tauri plugin: pluginId: $pluginId, command: ${invoke.command}"
    )

    try {
      val plugin = plugins[pluginId]
      if (plugin == null) {
        invoke.reject("Plugin $pluginId not initialized")
      } else {
        plugins[pluginId]?.invoke(invoke)
      }
    } catch (e: Exception) {
      var exception: Throwable = e
      if (exception.message?.isEmpty() != false) {
        if (e is InvocationTargetException) {
          exception = e.targetException
        }
      }
      invoke.reject(if (exception.message?.isEmpty() != false) { exception.toString() } else { exception.message })
    }
  }

  fun<T> loadConfig(context: Context, plugin: String, cls: Class<T>): T {
    val tauriConfigJson = FsUtils.readAsset(context.assets, "tauri.conf.json")
    val mapper = ObjectMapper()
      .configure(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, false)
    val config = mapper.readValue(tauriConfigJson, Config::class.java)
    return mapper.readValue(config.plugins[plugin].toString(), cls)
  }

  private external fun handlePluginResponse(id: Int, success: String?, error: String?)
  private external fun sendChannelData(id: Long, data: String)
}

@InvokeArg
internal class Config {
  lateinit var plugins: Map<String, JsonNode>
}
