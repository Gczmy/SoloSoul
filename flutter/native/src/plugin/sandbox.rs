//! Wasm sandbox - Isolated plugin execution environment
//!
//! 每个插件运行在独立的 wasmtime::Store 中，敏感数据仅在 Store 存活期间可访问。
//! TTL 到期后整个 Store 被 drop，Wasm 内存（含可能的敏感数据副本）彻底销毁。

use std::collections::HashSet;
use std::sync::Arc;
use wasmtime::{Config, Engine, Linker, Module, Store};

use super::host::{AuditAction, AuditEntry, ConsentChannel, RateLimiter, SoloHostFunctions};
use super::manifest::PluginManifest;


/// 沙盒执行结果
#[derive(Debug)]
pub struct PluginResult {
    pub exit_code: i32,
}

/// 沙盒执行错误
#[derive(Debug)]
pub enum PluginError {
    /// Wasm 编译失败
    CompileFailed(String),
    /// 实例化失败
    InstantiationFailed(String),
    /// 插件执行时崩溃（Trap）
    ExecutionFailed(String),
    /// 插件主入口 `run` 不存在
    MissingRunFunction,
    /// Host functions 注册失败
    HostFunctionRegistrationFailed(String),
    /// Fuel 耗尽（防死循环）
    FuelExhausted,
    /// Session 已过期
    SessionExpired,
}

/// Wasm 沙盒管理器
pub struct WasmSandbox {
    engine: Engine,
}

impl WasmSandbox {
    /// 创建新的沙盒引擎
    pub fn new() -> Result<Self, PluginError> {
        let mut config = Config::new();
        config.consume_fuel(true);
        // wasm_multi_memory 用于支持多内存提案（某些 WASI 应用需要）
        config.wasm_multi_memory(true);

        let engine = Engine::new(&config)
            .map_err(|e| PluginError::CompileFailed(format!("Failed to create engine: {}", e)))?;

        Ok(Self { engine })
    }

    /// 从 wasm 字节码编译模块
    pub fn compile_module(&self, wasm_bytes: &[u8]) -> Result<Module, PluginError> {
        Module::new(&self.engine, wasm_bytes)
            .map_err(|e| PluginError::CompileFailed(format!("Failed to compile module: {}", e)))
    }

    /// 执行插件，返回结果并自动清理敏感内存
    ///
    /// # 安全保证
    /// - Store 级隔离：每个插件拥有独立的 `wasmtime::Store`
    /// - TTL 到期：整个 Store 被 drop，Wasm 内存清零
    /// - Fuel 限制：防止死循环和无限计算
    /// - Trap 捕获：插件 panic 不会崩溃主程序
    pub fn execute(
        &self,
        module: &Module,
        plugin_id: &str,
        plugin_name: &str,
        session_id: &str,
        manifest: &PluginManifest,
        consent_channel: &ConsentChannel,
        audit_tx: tokio::sync::mpsc::Sender<AuditEntry>,
        log_tx: tokio::sync::mpsc::Sender<(String, String)>,
        result_tx: tokio::sync::mpsc::Sender<String>,
        rate_limiter: Arc<RateLimiter>,
        ttl_seconds: u64,
        pre_approved_fields: HashSet<String>,
    ) -> Result<PluginResult, PluginError> {

        let host = SoloHostFunctions::new(
            plugin_id,
            plugin_name,
            session_id,
            manifest.clone(),
            consent_channel.tx.clone(),
            audit_tx.clone(),
            log_tx,
            result_tx,
            rate_limiter,
            ttl_seconds,
            pre_approved_fields,
        );

        let mut linker = Linker::new(&self.engine);

        // 注册所有 SoloSoul Host Functions
        SoloHostFunctions::register(&mut linker)
            .map_err(PluginError::HostFunctionRegistrationFailed)?;

        // WASI Preview 1 基础环境（标准输入输出，无文件系统访问）
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |host: &mut SoloHostFunctions| {
            &mut host.wasi
        })
        .map_err(|e| PluginError::HostFunctionRegistrationFailed(e.to_string()))?;

        let mut store = Store::new(&self.engine, host);

        // Fuel 限制（Release: 10M, Debug: 100M）
        let fuel = if cfg!(debug_assertions) {
            100_000_000
        } else {
            10_000_000
        };
        store.set_fuel(fuel).map_err(|e| {
            PluginError::HostFunctionRegistrationFailed(format!("Failed to set fuel: {}", e))
        })?;

        let instance = linker
            .instantiate(&mut store, module)
            .map_err(|e| PluginError::InstantiationFailed(e.to_string()))?;

        let run = instance
            .get_typed_func::<(), i32>(&mut store, "run")
            .map_err(|_| PluginError::MissingRunFunction)?;

        // 捕获 Wasm Trap（插件 panic / 内存越界 / 除零等）
        let result = match run.call(&mut store, ()) {
            Ok(code) => Ok(PluginResult { exit_code: code }),
            Err(trap) => {
                let _ = audit_tx.try_send(AuditEntry {
                    plugin_id: plugin_id.to_string(),
                    session_id: session_id.to_string(),
                    timestamp: std::time::Instant::now(),
                    action: AuditAction::PluginCrashed {
                        reason: trap.to_string(),
                    },
                });
                Err(PluginError::ExecutionFailed(trap.to_string()))
            }
        };

        // Store 离开作用域后被 drop，所有 Wasm 内存（含可能的敏感数据副本）清零
        drop(store);

        result
    }

    /// 校验 wasm 字节码的 SHA-256 哈希
    pub fn verify_hash(wasm_bytes: &[u8], expected_hash: &str) -> Result<(), PluginError> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(wasm_bytes);
        let computed_hash = format!("{:x}", hasher.finalize());
        if computed_hash != expected_hash {
            return Err(PluginError::CompileFailed(
                "SHA-256 hash verification failed".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for WasmSandbox {
    fn default() -> Self {
        Self::new().expect("Failed to create default sandbox")
    }
}
