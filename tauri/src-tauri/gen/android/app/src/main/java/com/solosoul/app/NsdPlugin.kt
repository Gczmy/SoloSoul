package com.solosoul.app

import android.app.Activity
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import app.tauri.plugin.Invoke
import org.json.JSONArray
import org.json.JSONObject
import java.net.InetAddress
import java.util.concurrent.ConcurrentHashMap

@InvokeArg
class RegisterServiceArgs {
    var port: Int = 0
    lateinit var nodeId: String
    lateinit var accountId: String
    lateinit var fingerprint: String
}

/**
 * Android NSD（Network Service Discovery）插件。
 *
 * 封装 android.net.nsd.NsdManager，提供以下能力：
 * - startDiscovery：开始发现局域网内 _solosoul._tcp 服务。
 * - stopDiscovery：停止发现。
 * - registerService：注册本地服务，供其他设备发现。
 * - unregisterService：注销本地服务。
 * - getDiscoveredServices：获取当前已发现的服务列表。
 *
 * 服务 TXT 记录包含 node_id、account_id、fingerprint，用于前端过滤和配对。
 */
@TauriPlugin
class NsdPlugin(private val activity: Activity): Plugin(activity) {
    private val nsdManager: NsdManager by lazy {
        activity.getSystemService(Activity.NSD_SERVICE) as NsdManager
    }

    private val discoveredServices = ConcurrentHashMap<String, NsdServiceInfo>()
    private var discoveryListener: NsdManager.DiscoveryListener? = null
    private var registrationListener: NsdManager.RegistrationListener? = null
    private var resolveListener: NsdManager.ResolveListener? = null

    companion object {
        private const val SERVICE_TYPE = "_solosoul._tcp"
    }

    @Command
    fun startDiscovery(invoke: Invoke) {
        try {
            if (discoveryListener != null) {
                invoke.resolve(JSObject())
                return
            }
            discoveredServices.clear()
            discoveryListener = object : NsdManager.DiscoveryListener {
                override fun onDiscoveryStarted(serviceType: String?) {
                    android.util.Log.i("SoloSoul", "NSD discovery started: $serviceType")
                }

                override fun onDiscoveryStopped(serviceType: String?) {
                    android.util.Log.i("SoloSoul", "NSD discovery stopped: $serviceType")
                }

                override fun onServiceFound(serviceInfo: NsdServiceInfo?) {
                    serviceInfo ?: return
                    android.util.Log.i("SoloSoul", "NSD service found: ${serviceInfo.serviceName}")
                    // 只解析 SoloSoul 服务。
                    resolveService(serviceInfo)
                }

                override fun onServiceLost(serviceInfo: NsdServiceInfo?) {
                    serviceInfo ?: return
                    discoveredServices.remove(serviceInfo.serviceName)
                    android.util.Log.i("SoloSoul", "NSD service lost: ${serviceInfo.serviceName}")
                }

                override fun onStartDiscoveryFailed(serviceType: String?, errorCode: Int) {
                    android.util.Log.e("SoloSoul", "NSD start discovery failed: $errorCode")
                }

                override fun onStopDiscoveryFailed(serviceType: String?, errorCode: Int) {
                    android.util.Log.e("SoloSoul", "NSD stop discovery failed: $errorCode")
                }
            }
            nsdManager.discoverServices(SERVICE_TYPE, NsdManager.PROTOCOL_DNS_SD, discoveryListener)
            invoke.resolve(JSObject())
        } catch (e: Exception) {
            invoke.reject("启动 NSD 发现失败: ${e.message}")
        }
    }

    @Command
    fun stopDiscovery(invoke: Invoke) {
        try {
            discoveryListener?.let { listener ->
                try {
                    nsdManager.stopServiceDiscovery(listener)
                } catch (e: Exception) {
                    android.util.Log.w("SoloSoul", "stopServiceDiscovery failed: ${e.message}")
                }
            }
            discoveryListener = null
            discoveredServices.clear()
            invoke.resolve(JSObject())
        } catch (e: Exception) {
            invoke.reject("停止 NSD 发现失败: ${e.message}")
        }
    }

    @Command
    fun registerService(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(RegisterServiceArgs::class.java)
            unregisterServiceInternal()

            val serviceInfo = NsdServiceInfo().apply {
                serviceName = args.nodeId
                serviceType = SERVICE_TYPE
                port = args.port
                setAttribute("node_id", args.nodeId)
                setAttribute("account_id", args.accountId)
                setAttribute("fingerprint", args.fingerprint)
            }

            registrationListener = object : NsdManager.RegistrationListener {
                override fun onServiceRegistered(serviceInfo: NsdServiceInfo?) {
                    android.util.Log.i("SoloSoul", "NSD service registered: ${serviceInfo?.serviceName}")
                }

                override fun onRegistrationFailed(serviceInfo: NsdServiceInfo?, errorCode: Int) {
                    android.util.Log.e("SoloSoul", "NSD registration failed: $errorCode")
                }

                override fun onServiceUnregistered(serviceInfo: NsdServiceInfo?) {
                    android.util.Log.i("SoloSoul", "NSD service unregistered: ${serviceInfo?.serviceName}")
                }

                override fun onUnregistrationFailed(serviceInfo: NsdServiceInfo?, errorCode: Int) {
                    android.util.Log.e("SoloSoul", "NSD unregistration failed: $errorCode")
                }
            }

            nsdManager.registerService(serviceInfo, NsdManager.PROTOCOL_DNS_SD, registrationListener)
            invoke.resolve(JSObject())
        } catch (e: Exception) {
            invoke.reject("注册 NSD 服务失败: ${e.message}")
        }
    }

    @Command
    fun unregisterService(invoke: Invoke) {
        try {
            unregisterServiceInternal()
            invoke.resolve(JSObject())
        } catch (e: Exception) {
            invoke.reject("注销 NSD 服务失败: ${e.message}")
        }
    }

    private fun unregisterServiceInternal() {
        registrationListener?.let { listener ->
            try {
                nsdManager.unregisterService(listener)
            } catch (e: Exception) {
                android.util.Log.w("SoloSoul", "unregisterService failed: ${e.message}")
            }
        }
        registrationListener = null
    }

    @Command
    fun getDiscoveredServices(invoke: Invoke) {
        try {
            val array = JSONArray()
            for ((_, info) in discoveredServices) {
                val host = info.host?.hostAddress ?: ""
                val port = info.port
                val txt = info.attributes
                val nodeId = txt?.get("node_id")?.let { String(it) } ?: ""
                val accountId = txt?.get("account_id")?.let { String(it) } ?: ""
                val fingerprint = txt?.get("fingerprint")?.let { String(it) } ?: ""

                val obj = JSONObject().apply {
                    put("nodeId", nodeId)
                    put("accountId", accountId)
                    put("fingerprint", fingerprint)
                    put("host", host)
                    put("port", port)
                }
                array.put(obj)
            }
            val result = JSObject()
            result.put("services", array)
            invoke.resolve(result)
        } catch (e: Exception) {
            invoke.reject("获取 NSD 服务列表失败: ${e.message}")
        }
    }

    private fun resolveService(serviceInfo: NsdServiceInfo) {
        resolveListener = object : NsdManager.ResolveListener {
            override fun onResolveFailed(serviceInfo: NsdServiceInfo?, errorCode: Int) {
                android.util.Log.w("SoloSoul", "NSD resolve failed: $errorCode")
            }

            override fun onServiceResolved(resolvedInfo: NsdServiceInfo?) {
                resolvedInfo ?: return
                discoveredServices[resolvedInfo.serviceName] = resolvedInfo
                android.util.Log.i("SoloSoul", "NSD service resolved: ${resolvedInfo.serviceName} -> ${resolvedInfo.host}:${resolvedInfo.port}")
            }
        }
        try {
            nsdManager.resolveService(serviceInfo, resolveListener)
        } catch (e: Exception) {
            android.util.Log.w("SoloSoul", "resolveService failed: ${e.message}")
        }
    }
}
