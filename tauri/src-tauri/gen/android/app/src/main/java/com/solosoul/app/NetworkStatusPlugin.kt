package com.solosoul.app

import android.content.Context
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.JSObject
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin

/**
 * 网络状态检测插件（B-01 · 云同步 Wi-Fi only）。
 *
 * 供云同步调度器在「仅 Wi-Fi 同步」开启时判断当前是否处于 Wi-Fi/以太网：
 * - Wi-Fi 与以太网视为可同步（均不计费）；
 * - 蜂窝/无连接返回 onWifi=false，云同步跳过本轮（省流量，避免计费流量消耗）。
 *
 * 权限：ACCESS_NETWORK_STATE（AndroidManifest 已声明，无需动态申请）。
 */
@TauriPlugin
class NetworkStatusPlugin(private val activity: Activity) : Plugin(activity) {

    /** 当前是否处于适合大流量同步的网络（Wi-Fi / 以太网）。 */
    @Command
    fun isOnWifi(invoke: Invoke) {
        val ret = JSObject()
        try {
            val cm = activity.getSystemService(Context.CONNECTIVITY_SERVICE) as? ConnectivityManager
            if (cm == null) {
                ret.put("onWifi", false)
                ret.put("connected", false)
            } else {
                val network = cm.activeNetwork
                val caps = network?.let { cm.getNetworkCapabilities(it) }
                val connected = caps != null && caps.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
                // VPN 下 hasTransport(WIFI) 可能失真（VPN transport 优先），故同时看底层
                // 是否存在非计费传输：WIFI / ETHERNET 任一即视为可同步。
                val unmetered = caps != null && (
                    caps.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) ||
                        caps.hasTransport(NetworkCapabilities.TRANSPORT_ETHERNET)
                    )
                ret.put("onWifi", connected && unmetered)
                ret.put("connected", connected)
            }
            ret.put("error", null)
        } catch (e: Exception) {
            // 检测失败按「不可同步」处理：宁可不同步，也不在蜂窝下偷跑大流量
            ret.put("onWifi", false)
            ret.put("connected", false)
            ret.put("error", e.message)
        }
        invoke.resolve(ret)
    }

    companion object {
        private const val TAG = "NetworkStatusPlugin"
    }
}
