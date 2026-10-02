package com.solosoul.app

import java.util.concurrent.Executor

data class ResourcePreparationSnapshot(
  val status: String,
  val errorCode: String? = null,
  val version: String? = null,
  val writtenFiles: Int? = null
)

/** 进程级准备状态；安装只在 executor 执行，观察与等待不占用 UI 线程。 */
class ResourcePreparationCoordinator(private val executor: Executor) {
  private var state = ResourcePreparationSnapshot("idle")
  private val observers = linkedSetOf<(ResourcePreparationSnapshot) -> Unit>()
  private val waiters = mutableListOf<(ResourcePreparationSnapshot) -> Unit>()

  @Synchronized fun snapshot(): ResourcePreparationSnapshot = state

  fun start(install: () -> ResourceInstallResult, retry: Boolean = false) {
    synchronized(this) {
      if (state.status == "preparing" || state.status == "ready") return
      if (state.status == "error" && !retry) return
      state = ResourcePreparationSnapshot("preparing")
    }
    publish(snapshot())
    try {
      executor.execute {
        val next = try {
          val result = install()
          ResourcePreparationSnapshot("ready", version = result.version, writtenFiles = result.writtenFiles)
        } catch (_: Exception) {
          ResourcePreparationSnapshot("error", errorCode = "resource_install_failed")
        }
        synchronized(this) { state = next }
        publish(next)
      }
    } catch (_: java.util.concurrent.RejectedExecutionException) {
      val failed = ResourcePreparationSnapshot("error", errorCode = "resource_executor_unavailable")
      synchronized(this) { state = failed }
      publish(failed)
    }
  }

  fun observe(listener: (ResourcePreparationSnapshot) -> Unit): () -> Unit {
    val current = synchronized(this) { observers.add(listener); state }
    listener(current)
    return { synchronized(this) { observers.remove(listener) } }
  }

  fun waitUntilComplete(listener: (ResourcePreparationSnapshot) -> Unit) {
    val complete = synchronized(this) {
      if (state.status == "ready" || state.status == "error") state
      else { waiters.add(listener); null }
    }
    if (complete != null) listener(complete)
  }

  private fun publish(value: ResourcePreparationSnapshot) {
    val listeners = synchronized(this) {
      val all = observers.toList() + if (value.status == "ready" || value.status == "error") {
        waiters.toList().also { waiters.clear() }
      } else emptyList()
      all
    }
    listeners.forEach { it(value) }
  }
}
