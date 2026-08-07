//! 自动同步调度内核（P017）。
//!
//! `auto_sync.rs`（SAF 自动同步）与 `device_auto_sync.rs`（设备间自动同步）原先各维护
//! 一份约 90 行逐行重复的 Idle/Scheduled/Running 三态 select 循环 + 重试退避 +
//! test spawn 分支，仅「事件分类 / 防抖与周期来源 / enabled 门控」不同。此处收敛为
//! 单一泛型内核，差异由调用方通过 `SchedulerEvent` 关联类型与 `periodic_enabled`
//! 闭包声明。

use futures::future::BoxFuture;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

/// 调度事件契约：实现方声明事件如何分类、防抖/周期来源是什么。
pub(crate) trait SchedulerEvent: Send + 'static {
    /// 触发来源类型（对应各调用方的 `SyncSource`/`DeviceSyncSource`）。
    type Source: Copy + Send + 'static;

    /// 是否为「立即执行」事件（取消防抖直接运行）。
    fn is_immediate(&self) -> bool;

    /// 事件对应的来源（用于立即执行的 Running 态来源）。
    fn source(&self) -> Self::Source;

    /// 防抖事件对应的来源。
    fn debounce_source() -> Self::Source;

    /// 周期兜底对应的来源。
    fn periodic_source() -> Self::Source;
}

/// 调度动作契约：一次同步动作的执行体。
pub(crate) trait SchedulerAction: Send + Sync + 'static {
    /// 触发来源类型（须与事件契约一致）。
    type Source: Copy + Send + 'static;

    fn run(&self, source: Self::Source) -> BoxFuture<'static, Result<(), String>>;
}

/// 调度内核配置（对应 `AutoSyncConfig`/`DeviceAutoSyncConfig` 的公共字段）。
#[derive(Clone)]
pub(crate) struct SchedulerConfig {
    pub debounce_delay: Duration,
    pub periodic_interval: Duration,
    pub max_retries: u32,
    pub retry_delay: Duration,
}

enum SchedulerState<S> {
    Idle,
    Scheduled(S, tokio::time::Instant),
    Running(S),
}

/// 启动调度内核。`periodic_enabled` 控制周期 tick 是否允许触发同步
/// （SAF 恒为 true；设备端读取 AtomicBool 运行时开关）。
pub(crate) fn spawn_scheduler<E, A, F>(
    mut rx: mpsc::Receiver<E>,
    action: Arc<A>,
    config: SchedulerConfig,
    periodic_enabled: F,
) where
    E: SchedulerEvent,
    A: SchedulerAction<Source = E::Source> + ?Sized,
    F: Fn() -> bool + Send + 'static,
{
    let fut = async move {
        let mut state = SchedulerState::<E::Source>::Idle;
        let mut interval = tokio::time::interval(config.periodic_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // tokio::time::interval 第一次 tick 会立即触发；
        // 先消耗掉这次 tick，避免启动时误触发一次同步。
        interval.tick().await;
        let mut retry_count: u32 = 0;

        loop {
            match state {
                SchedulerState::Idle => {
                    tokio::select! {
                        event = rx.recv() => match event {
                            Some(inner) => {
                                if inner.is_immediate() {
                                    state = SchedulerState::Running(inner.source());
                                } else {
                                    let deadline =
                                        tokio::time::Instant::now() + config.debounce_delay;
                                    state =
                                        SchedulerState::Scheduled(E::debounce_source(), deadline);
                                }
                            }
                            None => break,
                        },
                        _ = interval.tick() => {
                            if periodic_enabled() {
                                state = SchedulerState::Running(E::periodic_source());
                            }
                        }
                    }
                }
                SchedulerState::Scheduled(source, d) => {
                    tokio::select! {
                        event = rx.recv() => match event {
                            Some(inner) => {
                                if inner.is_immediate() {
                                    state = SchedulerState::Running(inner.source());
                                } else {
                                    let new_deadline =
                                        tokio::time::Instant::now() + config.debounce_delay;
                                    state = SchedulerState::Scheduled(source, new_deadline);
                                }
                            }
                            None => break,
                        },
                        _ = tokio::time::sleep_until(d) => {
                            state = SchedulerState::Running(source);
                        }
                        _ = interval.tick() => {
                            // Already scheduled, nothing to do.
                        }
                    }
                }
                SchedulerState::Running(source) => {
                    let result = action.run(source).await;
                    match result {
                        Ok(()) => {
                            retry_count = 0;
                            state = SchedulerState::Idle;
                        }
                        Err(_) => {
                            if retry_count < config.max_retries {
                                retry_count += 1;
                                let exponent = (retry_count - 1).min(10);
                                let backoff = config.retry_delay * 2u32.pow(exponent);
                                tokio::time::sleep(backoff).await;
                                state = SchedulerState::Running(source);
                            } else {
                                retry_count = 0;
                                state = SchedulerState::Idle;
                            }
                        }
                    }
                }
            }
        }
    };

    // 生产环境使用 tauri 的全局 async runtime，确保在无 Tokio 上下文（如
    // Android 主线程）中也能成功 spawn。测试环境下使用 tokio::spawn 即可，
    // 因为 #[tokio::test] 已建立运行时上下文。
    #[cfg(test)]
    tokio::spawn(fut);
    #[cfg(not(test))]
    tauri::async_runtime::spawn(fut);
}
