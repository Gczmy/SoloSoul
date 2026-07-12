//! 插件 Host Functions 移动端占位实现
//!
//! 移动端暂不提供 Wasmtime 沙箱，因此相关类型与注册函数均为空壳，
//! 仅保证 `event.rs` 等共享模块的编译与类型签名一致。

use crate::plugin::PluginError;

/// 传递给 Wasm Store 的状态（移动端占位）。
pub struct SoloHostState;

/// 自定义 Host Functions 数据（移动端占位）。
#[allow(clippy::module_name_repetitions)]
pub struct SoloHostFunctions;

/// 注册 SoloSoul 自定义 Host Functions（移动端无操作）。
pub fn register_host_functions<T>(_linker: &mut T) -> Result<(), PluginError> {
    Ok(())
}

/// 被 `event.rs` 依赖的时间戳辅助函数。
pub mod memory {
    pub fn now_millis() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64
    }
}
