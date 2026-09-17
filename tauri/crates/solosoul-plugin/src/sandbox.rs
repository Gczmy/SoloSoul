//! Wasmtime 沙箱执行器
//!
//! 提供编译、燃料限制与 WASI Preview1 支持，隔离插件运行环境。

use super::{
    ConsentManager, PluginError, PluginEvent, PluginResult, PluginSession, SoloHostFunctions,
    SoloHostState,
};
use std::sync::Arc;
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

    /// 编译 Wasm 模块（进程级缓存：以字节 SHA-256 为键，同版本重复运行免重复编译）。
    ///
    /// P022: WASM 编译（Cranelift JIT / Pulley）是每次运行最贵步骤。
    /// `Module` 为 Send+Sync，编译产物可跨线程安全共享；锁仅在查/插时持有，
    /// 编译在锁外执行避免并发首编译互相阻塞。
    pub fn compile(&self, wasm: &[u8]) -> Result<Arc<Module>, PluginError> {
        use sha2::{Digest, Sha256};
        static MODULE_CACHE: std::sync::Mutex<
            Option<std::collections::HashMap<String, Arc<Module>>>,
        > = std::sync::Mutex::new(None);

        let key = hex::encode(Sha256::digest(wasm));
        {
            let guard = MODULE_CACHE.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(hit) = guard.as_ref().and_then(|m| m.get(&key)) {
                return Ok(hit.clone());
            }
        }

        let mut config = Config::new();
        config.consume_fuel(true);
        // 移动端强制使用 Pulley 解释器目标，避免 Cranelift JIT 在 Android/iOS 上的 native signal / mmap 兼容性问题
        #[cfg(any(target_os = "android", target_os = "ios"))]
        {
            config.target("pulley64").map_err(|e| {
                PluginError::ExecutionFailed(format!("设置 Pulley target 失败: {}", e))
            })?;
        }
        let engine =
            Engine::new(&config).map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
        let module = Arc::new(Module::new(&engine, wasm).map_err(PluginError::from)?);

        let mut guard = MODULE_CACHE.lock().unwrap_or_else(|e| e.into_inner());
        let cache = guard.get_or_insert_with(std::collections::HashMap::new);
        cache.entry(key).or_insert_with(|| module.clone());
        Ok(module)
    }

    /// 执行 Wasm 模块
    pub fn execute(
        &self,
        module: &Module,
        mut host: SoloHostFunctions,
        session: &PluginSession,
        _consent_manager: &ConsentManager,
    ) -> Result<PluginResult, PluginError> {
        if host.plugin_id != session.plugin_id || host.session_id != session.id {
            return Err(PluginError::ExecutionFailed("插件与会话不匹配".into()));
        }
        host.field_resolver = Arc::new((*host.field_resolver).clone().with_session(session));
        host.channel = Arc::new(super::event::SessionEventSink {
            resolver: host.field_resolver.clone(),
            inner: host.channel.clone(),
        });
        host.ensure_live()?;
        let engine = module.engine().clone();
        let mut linker = Linker::<SoloHostState>::new(&engine);

        // 注册 SoloSoul 自定义 Host Functions
        super::register_host_functions(&mut linker)?;

        // 注册 WASI Preview1（stdio 默认空槽——插件 stdout/stderr 写入静默丢弃，
        // 不继承宿主终端，杜绝向宿主日志注入伪造内容的注入面，见 P208）
        wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |s: &mut SoloHostState| &mut s.wasi)
            .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

        // 在将 host 移入 Store 前克隆必要信息，以便 panic 时仍能发送错误事件
        let plugin_id = host.plugin_id.clone();
        let plugin_name = host.plugin_name.clone();
        let channel = host.channel.clone();

        // 不调用 inherit_stdio：WasiCtxBuilder 默认将 stdin/stdout/stderr 接为
        // tokio::io::empty() 黑洞——插件写 stdout/stderr 静默丢弃而非进入宿主终端。
        // 插件输出应走 Host Functions 的 log/result 通道（受 Consent 约束）。
        let wasi = wasmtime_wasi::WasiCtx::builder().build_p1();
        let state = SoloHostState { wasi, host };
        let mut store = Store::new(&engine, state);
        // 覆盖所有自定义及 WASI 宿主入口，并在阻塞调用返回后再次检查。
        // 即便插件继续计算，下一次数据/网络/输出访问及最终结果均会被拒绝。
        store.call_hook(|context, _| {
            context
                .data()
                .host
                .ensure_live()
                .map_err(|e| wasmtime::Error::msg(e.to_string()))
        });
        store
            .set_fuel(self.fuel_limit)
            .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

        let instance = linker
            .instantiate(&mut store, module)
            .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

        let run = instance
            .get_typed_func::<(), i32>(&mut store, "run")
            .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

        let exit_code = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run.call(&mut store, ())
        })) {
            Ok(Ok(code)) => code,
            Ok(Err(e)) => {
                let host = store.into_data().host;
                let _ = host.channel.send(PluginEvent::error(
                    &host.plugin_id,
                    format!("Wasm trap: {}", e),
                ));
                return Err(PluginError::ExecutionFailed(e.to_string()));
            }
            Err(_) => {
                let _ = channel.send(PluginEvent::error(
                    &plugin_id,
                    format!("插件 {} 运行时 panic", plugin_name),
                ));
                return Err(PluginError::ExecutionFailed(
                    "插件运行时 panic，已被沙箱捕获".to_string(),
                ));
            }
        };

        let remaining = store.get_fuel().unwrap_or(self.fuel_limit);
        let fuel_consumed = self.fuel_limit.saturating_sub(remaining);

        let host = store.into_data().host;
        host.ensure_live()?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        event::PluginEventSink, FieldResolver, PluginAuditLogger, PluginManifest,
        PluginSessionManager, RateLimiter,
    };
    use std::sync::Mutex;

    #[derive(Default)]
    struct Events {
        values: Mutex<Vec<PluginEvent>>,
        lock_on_log: Option<Arc<solosoul_vault::VaultStore>>,
    }

    impl PluginEventSink for Events {
        fn send(&self, event: PluginEvent) -> Result<(), String> {
            if event.event_type == "log" {
                if let Some(vault) = &self.lock_on_log {
                    vault.lock();
                }
            }
            self.values.lock().unwrap().push(event);
            Ok(())
        }
    }

    fn host(
        session: &PluginSession,
        resolver: FieldResolver,
        events: Arc<Events>,
    ) -> SoloHostFunctions {
        let manifest: PluginManifest = serde_json::from_value(serde_json::json!({
            "id": "test", "name": "Test", "version": "1.0.0", "description": "test"
        }))
        .unwrap();
        SoloHostFunctions::new(
            "test",
            "Test",
            &session.id,
            manifest,
            Default::default(),
            Arc::new(PluginAuditLogger::default()),
            Arc::new(RateLimiter::new(100)),
            Arc::new(ConsentManager::new()),
            Arc::new(resolver),
            events,
        )
    }

    const LOG_THEN_RESULT: &[u8] = br#"(module
        (import "env" "solosoul_log" (func $log (param i32 i32 i32 i32)))
        (import "env" "solosoul_result" (func $result (param i32 i32) (result i32)))
        (memory (export "memory") 1)
        (data (i32.const 0) "info") (data (i32.const 4) "start") (data (i32.const 9) "{}")
        (func (export "run") (result i32)
            i32.const 0 i32.const 4 i32.const 4 i32.const 5 call $log
            i32.const 9 i32.const 2 call $result drop i32.const 0))"#;

    #[test]
    fn wasm_rejects_expired_and_mismatched_sessions_before_execution() {
        let sandbox = WasmSandbox::new();
        let module = sandbox.compile(LOG_THEN_RESULT).unwrap();
        let events = Arc::new(Events::default());
        let mut session = PluginSessionManager::new().create("test", 0);
        assert!(sandbox
            .execute(
                &module,
                host(&session, FieldResolver::new(), events.clone()),
                &session,
                &ConsentManager::new()
            )
            .is_err());
        session.expires_at = chrono::Utc::now().timestamp_millis() + 60_000;
        let h = host(&session, FieldResolver::new(), events.clone());
        session.plugin_id = "another-plugin".into();
        assert!(sandbox
            .execute(&module, h, &session, &ConsentManager::new())
            .is_err());
        assert!(events.values.lock().unwrap().is_empty());
    }

    #[test]
    fn wasm_lock_during_host_call_blocks_later_result_and_completion() {
        let dir = tempfile::TempDir::new().unwrap();
        let vault = Arc::new(
            solosoul_vault::VaultStore::open(
                solosoul_vault::VaultConfig::new("test", dir.path().to_path_buf())
                    .with_data_key([7; 32]),
            )
            .unwrap(),
        );
        let events = Arc::new(Events {
            lock_on_log: Some(vault.clone()),
            ..Default::default()
        });
        let resolver = FieldResolver::with_vault(vault, "test".into(), vec!["*".into()]);
        let sandbox = WasmSandbox::new();
        let module = sandbox.compile(LOG_THEN_RESULT).unwrap();
        let session = PluginSessionManager::new().create("test", 60);
        assert!(sandbox
            .execute(
                &module,
                host(&session, resolver.clone(), events.clone()),
                &session,
                &ConsentManager::new()
            )
            .is_err());
        assert_eq!(events.values.lock().unwrap().len(), 1);
        assert_eq!(events.values.lock().unwrap()[0].event_type, "log");
        let guarded = crate::event::SessionEventSink {
            resolver: Arc::new(resolver),
            inner: events.clone(),
        };
        assert!(guarded.send(PluginEvent::result("private-result")).is_err());
        assert_eq!(events.values.lock().unwrap().len(), 1);
    }

    #[test]
    fn wasm_expiry_during_blocking_host_call_traps_on_return() {
        let sandbox = WasmSandbox::new();
        let module = sandbox
            .compile(
                br#"(module
            (import "env" "solosoul_sleep" (func $sleep (param i64) (result i32)))
            (func (export "run") (result i32) i64.const 1000 call $sleep drop i32.const 0))"#,
            )
            .unwrap();
        let events = Arc::new(Events::default());
        let mut session = PluginSessionManager::new().create("test", 60);
        let h = host(&session, FieldResolver::new(), events.clone());
        session.expires_at = chrono::Utc::now().timestamp_millis() + 400;
        let start = std::time::Instant::now();
        assert!(sandbox
            .execute(&module, h, &session, &ConsentManager::new())
            .is_err());
        assert!(start.elapsed() >= std::time::Duration::from_millis(400));
        assert!(events.values.lock().unwrap().is_empty());
    }
}
