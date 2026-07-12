//! Wasm 沙箱执行器移动端占位实现。
//!
//! 移动端暂不支持 WASM 插件运行时，因此本模块仅提供与桌面端相同的公共类型签名，
//! 所有实际功能均为空实现，确保 crate 在移动平台可编译。

use crate::{
    ConsentManager, PluginError, PluginEvent, PluginResult, PluginSession, SoloHostFunctions,
};

/// Wasm 沙箱占位。
#[derive(Debug, Clone, Copy)]
pub struct WasmSandbox {
    /// 单次运行燃料上限
    pub fuel_limit: u64,
}

impl Default for WasmSandbox {
    fn default() -> Self {
        Self {
            fuel_limit: 10_000_000_000,
        }
    }
}

/// 占位 Wasm 模块类型。
#[derive(Debug)]
pub struct Module;

impl WasmSandbox {
    /// 创建默认沙箱
    pub fn new() -> Self {
        Self::default()
    }

    /// 编译 Wasm 模块（占位，始终返回错误）。
    pub fn compile(&self, _wasm: &[u8]) -> Result<Module, PluginError> {
        Err(PluginError::ExecutionFailed(
            "移动端暂不支持插件运行时".to_string(),
        ))
    }

    /// 执行 Wasm 模块（占位，始终返回错误）。
    pub fn execute(
        &self,
        _module: &Module,
        _host: SoloHostFunctions,
        _session: &PluginSession,
        _consent_manager: &ConsentManager,
    ) -> Result<PluginResult, PluginError> {
        Err(PluginError::ExecutionFailed(
            "移动端暂不支持插件运行时".to_string(),
        ))
    }
}
