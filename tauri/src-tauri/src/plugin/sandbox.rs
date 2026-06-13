//! Wasmtime 沙箱执行器
//!
//! 提供编译、燃料限制与 WASI Preview1 支持，隔离插件运行环境。

use super::{
    ConsentManager, PluginError, PluginEvent, PluginResult, PluginSession, SoloHostFunctions,
    SoloHostState,
};
use wasmtime::{Config, Engine, Linker, Module, Store};

/// Wasm 沙箱
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

    /// 编译 Wasm 模块
    pub fn compile(&self, wasm: &[u8]) -> Result<Module, PluginError> {
        let mut config = Config::new();
        config.consume_fuel(true);
        let engine =
            Engine::new(&config).map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
        Module::new(&engine, wasm).map_err(PluginError::from)
    }

    /// 执行 Wasm 模块
    pub fn execute(
        &self,
        module: &Module,
        host: SoloHostFunctions,
        _session: &PluginSession,
        _consent_manager: &ConsentManager,
    ) -> Result<PluginResult, PluginError> {
        let engine = module.engine().clone();
        let mut linker = Linker::<SoloHostState>::new(&engine);

        // 注册 SoloSoul 自定义 Host Functions
        super::register_host_functions(&mut linker)?;

        // 注册 WASI Preview1（仅继承 stdio）
        wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |s: &mut SoloHostState| &mut s.wasi)
            .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

        let wasi = wasmtime_wasi::WasiCtx::builder().inherit_stdio().build_p1();
        let state = SoloHostState { wasi, host };
        let mut store = Store::new(&engine, state);
        store
            .set_fuel(self.fuel_limit)
            .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

        let instance = linker
            .instantiate(&mut store, module)
            .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

        let run = instance
            .get_typed_func::<(), i32>(&mut store, "run")
            .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

        let exit_code = match run.call(&mut store, ()) {
            Ok(code) => code,
            Err(e) => {
                let host = store.into_data().host;
                let _ = host.channel.send(PluginEvent::error(
                    &host.plugin_id,
                    format!("Wasm trap: {}", e),
                ));
                return Err(PluginError::ExecutionFailed(e.to_string()));
            }
        };

        let remaining = store.get_fuel().unwrap_or(self.fuel_limit);
        let fuel_consumed = self.fuel_limit.saturating_sub(remaining);

        let host = store.into_data().host;
        let logs = host.take_logs();
        let results = host.take_results();

        let _ = host.channel.send(PluginEvent::completed(
            &host.plugin_id,
            exit_code,
            fuel_consumed,
        ));

        Ok(PluginResult {
            exit_code,
            logs,
            results,
            fuel_consumed,
        })
    }
}
