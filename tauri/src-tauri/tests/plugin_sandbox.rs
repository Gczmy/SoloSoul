use solo_soul::plugin::{
    ConsentManager, FieldResolver, PluginAuditLogger, PluginManager, PluginSessionManager,
    SoloHostFunctions, WasmSandbox,
};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::ipc::Channel;

fn wasm_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("SoloSoul_plugin_market")
        .join("examples")
        .join("hello_world")
        .join("target")
        .join("wasm32-wasip1")
        .join("release")
        .join("hello_world.wasm")
}

fn dummy_channel() -> Channel<solo_soul::plugin::PluginEvent> {
    // 测试中使用一个无需接收者的 Channel；Tauri Channel 在测试中可直接 new。
    Channel::new(|_| Ok(()))
}

#[tokio::test]
async fn test_hello_world_plugin_runs() {
    let wasm_bytes = std::fs::read(wasm_path()).expect("读取 hello_world.wasm 失败");
    let sandbox = WasmSandbox::new();
    let module = sandbox.compile(&wasm_bytes).expect("编译失败");

    let session_manager = PluginSessionManager::new();
    let session = session_manager.create("com.solosoul.official.hello-world", 300);
    let audit = Arc::new(PluginAuditLogger::new());
    let rate_limiter = Arc::new(solo_soul::plugin::RateLimiter::new(60));
    let consent_manager = Arc::new(ConsentManager::new());
    let field_resolver = Arc::new(FieldResolver::new());
    let manifest = solo_soul::plugin::PluginManifest {
        id: "com.solosoul.official.hello-world".to_string(),
        name: "Hello World".to_string(),
        version: "1.0.0".to_string(),
        description: "Test plugin".to_string(),
        author: None,
        homepage: None,
        permissions: vec![],
        required_core_version: None,
        wasm_hash_sha256: None,
        data_ttl_seconds: 300,
    };

    let host = SoloHostFunctions::new(
        "com.solosoul.official.hello-world",
        "Hello World",
        &session.id,
        manifest,
        HashMap::new(),
        audit,
        rate_limiter,
        consent_manager,
        field_resolver,
        dummy_channel(),
    );

    let result = sandbox
        .execute(&module, host, &session, &ConsentManager::new())
        .expect("执行失败");

    assert_eq!(result.exit_code, 0);
    assert!(
        result.logs.iter().any(|l| l.message.contains("Hello")),
        "日志中应包含 Hello"
    );
}

#[test]
fn test_plugin_manager_new() {
    let manager = PluginManager::new();
    assert!(manager.is_ok());
}
