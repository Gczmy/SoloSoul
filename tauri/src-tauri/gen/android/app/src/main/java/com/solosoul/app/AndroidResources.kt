package com.solosoul.app

import android.content.Context
import android.os.Trace
import java.util.concurrent.Executors

/** 不持有 Activity；重建只重新订阅同一进程任务。 */
object AndroidResources {
  @Volatile private var coordinator = ResourcePreparationCoordinator(
    Executors.newSingleThreadExecutor { task -> Thread(task, "SoloSoul-resource-install").apply { isDaemon = true } }
  )
  @Volatile private var testInstall: (() -> ResourceInstallResult)? = null

  fun prepare(context: Context, retry: Boolean = false) {
    val app = context.applicationContext
    coordinator.start({
      Trace.beginSection("SoloSoul.resources.install")
      try {
        testInstall?.invoke() ?: MainActivity.extractAssetsToDataDir(app.assets, app.dataDir)
      } finally { Trace.endSection() }
    }, retry)
  }
  fun snapshot() = coordinator.snapshot()
  fun observe(listener: (ResourcePreparationSnapshot) -> Unit) = coordinator.observe(listener)
  fun waitUntilComplete(listener: (ResourcePreparationSnapshot) -> Unit) = coordinator.waitUntilComplete(listener)

  /** 仅 Debug 专用 instrumentation 注入；Release 不接受测试安装器。 */
  internal fun overrideForTest(value: ResourcePreparationCoordinator, install: (() -> ResourceInstallResult)?) {
    check(BuildConfig.DEBUG)
    coordinator = value
    testInstall = install
  }
}
