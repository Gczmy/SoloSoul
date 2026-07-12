//! Wasmtime 沙箱执行器移动端占位实现

/// Wasm 沙箱（移动端占位）。
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

impl WasmSandbox {
    /// 创建默认沙箱
    pub fn new() -> Self {
        Self::default()
    }
}
