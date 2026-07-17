package com.solosoul.app

import android.Manifest
import android.app.Activity
import android.content.pm.PackageManager
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.net.wifi.WifiManager
import android.os.Build
import android.os.Handler
import android.os.Looper
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
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

    private val wifiManager: WifiManager by lazy {
        activity.applicationContext.getSystemService(Activity.WIFI_SERVICE) as WifiManager
    }

    private val discoveredServices = ConcurrentHashMap<String, NsdServiceInfo>()
    private var discoveryListener: NsdManager.DiscoveryListener? = null
    private var registrationListener: NsdManager.RegistrationListener? = null
    private var multicastLock: WifiManager.MulticastLock? = null

    private var permissionGranted: Boolean? = null
    private val permissionHandler = Handler(Looper.getMainLooper())
    private var permissionRunnable: Runnable? = null

    companion object {
        private const val SERVICE_TYPE = "_solosoul._tcp"
        private const val PERMISSION_REQUEST_CODE = 1001
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
            acquireMulticastLock()
            nsdManager.discoverServices(SERVICE_TYPE, NsdManager.PROTOCOL_DNS_SD, discoveryListener)
            invoke.resolve(JSObject())
        } catch (e: Exception) {
            releaseMulticastLock()
            discoveryListener = null
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
            releaseMulticastLock()
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

    @Command
    fun requestPermissions(invoke: Invoke) {
        try {
            val permission = getRequiredPermission()
            if (permission == null) {
                invoke.resolve(JSObject())
                return
            }

            // 缓存命中且确实仍被授予时快速返回；若用户从系统设置撤销权限，则重新请求。
            permissionGranted?.let {
                if (it && ContextCompat.checkSelfPermission(activity, permission) == PackageManager.PERMISSION_GRANTED) {
                    invoke.resolve(JSObject())
                    return
                }
            }

            if (ContextCompat.checkSelfPermission(activity, permission) == PackageManager.PERMISSION_GRANTED) {
                permissionGranted = true
                invoke.resolve(JSObject())
                return
            }

            ActivityCompat.requestPermissions(
                activity,
                arrayOf(permission),
                PERMISSION_REQUEST_CODE
            )

            // 权限请求是异步的，通过轮询等待用户响应（最多 10 秒）。
            permissionRunnable?.let { permissionHandler.removeCallbacks(it) }
            val deadline = System.currentTimeMillis() + 10_000
            val runnable = object : Runnable {
                override fun run() {
                    if (ContextCompat.checkSelfPermission(activity, permission) == PackageManager.PERMISSION_GRANTED) {
                        permissionGranted = true
                        permissionRunnable = null
                        invoke.resolve(JSObject())
                    } else if (System.currentTimeMillis() >= deadline) {
                        permissionGranted = false
                        permissionRunnable = null
                        invoke.reject("Permission denied: $permission")
                    } else {
                        permissionHandler.postDelayed(this, 200)
                    }
                }
            }
            permissionRunnable = runnable
            permissionHandler.postDelayed(runnable, 200)
        } catch (e: Exception) {
            invoke.reject("请求 NSD 权限失败: ${e.message}")
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        permissionRunnable?.let { permissionHandler.removeCallbacks(it) }
        // 清理 NSD 发现与注册，避免插件销毁后泄漏监听器和 MulticastLock
        discoveryListener?.let { listener ->
            try {
                nsdManager.stopServiceDiscovery(listener)
            } catch (e: Exception) {
                android.util.Log.w("SoloSoul", "stopServiceDiscovery in onDestroy failed: ${e.message}")
            }
        }
        discoveryListener = null
        unregisterServiceInternal()
        releaseMulticastLock()
    }

    /**
     * 返回当前 Android 版本发现 NSD 服务所需的运行时权限。
     * - API 33+：NEARBY_WIFI_DEVICES
     * - API <= 32：ACCESS_FINE_LOCATION
     */
    private fun getRequiredPermission(): String? {
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            Manifest.permission.NEARBY_WIFI_DEVICES
        } else {
            Manifest.permission.ACCESS_FINE_LOCATION
        }
    }

    private fun acquireMulticastLock() {
        try {
            if (multicastLock == null) {
                multicastLock = wifiManager.createMulticastLock("SoloSoulNsdLock").apply {
                    setReferenceCounted(false)
                }
            }
            multicastLock?.takeIf { !it.isHeld }?.acquire()
        } catch (e: Exception) {
            android.util.Log.w("SoloSoul", "acquireMulticastLock failed: ${e.message}")
        }
    }

    private fun releaseMulticastLock() {
        try {
            multicastLock?.takeIf { it.isHeld }?.release()
        } catch (e: Exception) {
            android.util.Log.w("SoloSoul", "releaseMulticastLock failed: ${e.message}")
        }
    }

    private fun resolveService(serviceInfo: NsdServiceInfo) {
        val listener = object : NsdManager.ResolveListener {
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
            nsdManager.resolveService(serviceInfo, listener)
        } catch (e: Exception) {
            android.util.Log.w("SoloSoul", "resolveService failed: ${e.message}")
        }
    }
}
