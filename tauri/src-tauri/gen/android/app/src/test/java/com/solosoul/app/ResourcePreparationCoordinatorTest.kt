package com.solosoul.app

import org.junit.Assert.*
import org.junit.Test
import java.util.concurrent.Executor

class ResourcePreparationCoordinatorTest {
  private class ControlledExecutor : Executor {
    val tasks = mutableListOf<Runnable>()
    override fun execute(command: Runnable) { tasks.add(command) }
    fun finish() { tasks.removeAt(0).run() }
  }

  @Test fun slowPreparationReturnsImmediatelyAndRecreationSharesOneTask() {
    val executor = ControlledExecutor()
    val coordinator = ResourcePreparationCoordinator(executor)
    var installs = 0
    val install = { installs++; ResourceInstallResult(false, 3, "v1") }
    coordinator.start(install)
    coordinator.start(install)
    assertEquals("preparing", coordinator.snapshot().status)
    assertEquals(0, installs)
    assertEquals(1, executor.tasks.size)
    val received = mutableListOf<String>()
    coordinator.waitUntilComplete { received.add(it.status) }
    assertTrue(received.isEmpty())
    executor.finish()
    assertEquals(1, installs)
    assertEquals(listOf("ready"), received)
    coordinator.start(install)
    assertTrue(executor.tasks.isEmpty())
  }

  @Test fun failureIsDeliveredAndExplicitRetryHasNewBarrier() {
    val executor = ControlledExecutor()
    val coordinator = ResourcePreparationCoordinator(executor)
    coordinator.start({ throw java.io.IOException("injected") })
    val received = mutableListOf<ResourcePreparationSnapshot>()
    coordinator.waitUntilComplete { received.add(it) }
    executor.finish()
    assertEquals("error", received.single().status)
    assertEquals("resource_install_failed", received.single().errorCode)
    coordinator.start({ error("不能隐式重试") })
    assertTrue(executor.tasks.isEmpty())
    coordinator.start({ ResourceInstallResult(false, 1, "v2") }, retry = true)
    coordinator.waitUntilComplete { received.add(it) }
    assertEquals(1, received.size)
    executor.finish()
    assertEquals(listOf("error", "ready"), received.map { it.status })
  }

  @Test fun destroyedObserverIsRemovedButWaitersStillReceiveResult() {
    val executor = ControlledExecutor()
    val coordinator = ResourcePreparationCoordinator(executor)
    val visible = mutableListOf<String>()
    val remove = coordinator.observe { visible.add(it.status) }
    coordinator.start({ ResourceInstallResult(true, 0, "v1") })
    remove()
    var result: ResourcePreparationSnapshot? = null
    coordinator.waitUntilComplete { result = it }
    executor.finish()
    assertEquals(listOf("idle", "preparing"), visible)
    assertEquals("ready", result?.status)
    var cached: ResourcePreparationSnapshot? = null
    coordinator.waitUntilComplete { cached = it }
    assertEquals(result, cached)
  }
}
