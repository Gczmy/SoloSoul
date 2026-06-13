use solo_soul::plugin::{
    ConsentManager, FieldResolver, PluginAuditLogger, PluginManifest, PluginNetworkPolicy,
    PluginSessionManager, SoloHostFunctions, WasmSandbox,
};
use solosoul_vault::{ObjectRecord, VaultConfig, VaultStore};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::ipc::Channel;

fn wasm_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("SoloSoul_plugin_market")
        .join("plugins")
        .join("com.solosoul.official.address-fmt")
        .join("plugin.wasm")
}

fn dummy_channel() -> Channel<solo_soul::plugin::PluginEvent> {
    Channel::new(|_| Ok(()))
}

fn open_test_vault(path: &std::path::Path, account_id: &str) -> VaultStore {
    let key = [0u8; 32];
    let config = VaultConfig::new(account_id, path.to_path_buf()).with_data_key(key);
    VaultStore::open(config).expect("打开测试 Vault 失败")
}

fn create_address_record(account_id: &str, idx: usize) -> ObjectRecord {
    let now = chrono::Utc::now().to_rfc3339();
    ObjectRecord {
        id: format!("addr_{}", idx),
        account_id: account_id.to_string(),
        type_id: "address".to_string(),
        section_type: "identity".to_string(),
        name: format!("地址 {}", idx + 1),
        icon_name: "map-pin".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({
            "title": if idx == 0 { "家".to_string() } else { String::new() },
            "street": "长安街1号",
            "city": "北京市",
            "district": "海淀区",
            "state": "",
            "postalCode": "100080",
            "country": "CN"
        }),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: None,
        template_type: None,
        created_at: now.clone(),
        updated_at: now,
        version: 1,
    }
}

#[tokio::test]
async fn test_address_fmt_plugin_reads_vault_fields() {
    let wasm_bytes = std::fs::read(wasm_path()).expect("读取 address-fmt wasm 失败");
    let sandbox = WasmSandbox::new();
    let module = sandbox.compile(&wasm_bytes).expect("编译失败");

    let tmp = tempfile::TempDir::new().unwrap();
    let account_id = "test_account";
    let vault = Arc::new(open_test_vault(tmp.path(), account_id));

    // 写入两条地址记录
    vault
        .save_object(&create_address_record(account_id, 0))
        .expect("保存地址 0 失败");
    // 第二条使用美国地址，验证按国家格式化
    let mut addr2 = create_address_record(account_id, 1);
    addr2.properties = serde_json::json!({
        "title": "公司",
        "street": "1600 Pennsylvania Avenue NW",
        "city": "Washington",
        "district": "",
        "state": "DC",
        "postalCode": "20500",
        "country": "US"
    });
    // 让 created_at 略晚，保证排序稳定
    addr2.created_at = chrono::Utc::now().to_rfc3339();
    vault.save_object(&addr2).expect("保存地址 1 失败");

    let session_manager = PluginSessionManager::new();
    let session = session_manager.create("com.solosoul.official.address-fmt", 300);
    let audit = Arc::new(PluginAuditLogger::default());
    let rate_limiter = Arc::new(solo_soul::plugin::RateLimiter::new(60));
    let consent_manager = Arc::new(ConsentManager::new());

    let permissions = vec![
        "address.count".to_string(),
        "address.title".to_string(),
        "address.street".to_string(),
        "address.district".to_string(),
        "address.city".to_string(),
        "address.state".to_string(),
        "address.postalCode".to_string(),
        "address.country".to_string(),
    ];
    let field_resolver = Arc::new(FieldResolver::with_vault(
        vault,
        account_id.to_string(),
        permissions,
    ));

    let manifest = PluginManifest {
        id: "com.solosoul.official.address-fmt".to_string(),
        name: "Address Formatter".to_string(),
        version: "1.0.4".to_string(),
        description: "地址格式化器".to_string(),
        author: Some("SoloSoul Official".to_string()),
        homepage: None,
        permissions: vec![],
        required_core_version: Some("1.0".to_string()),
        wasm_hash_sha256: None,
        data_ttl_seconds: 60,
        network_policy: PluginNetworkPolicy {
            block_all_outbound: true,
            allowed_domains: vec![],
        },
        require_user_confirmation: false,
        tier: solo_soul::plugin::PluginTier::P0,
        category: "formatter".to_string(),
        params: vec![],
    };

    let host = SoloHostFunctions::new(
        "com.solosoul.official.address-fmt",
        "Address Formatter",
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

    assert_eq!(result.exit_code, 0, "插件应成功退出");
    assert!(
        result
            .logs
            .iter()
            .any(|l| l.message.contains("发现 2 条地址")),
        "日志中应报告两条地址"
    );
    assert!(
        result.results.iter().any(|r| {
            let text = r.0.to_string();
            text.contains("北京市海淀区长安街1号")
        }),
        "结果中应包含中国地址"
    );
    assert!(
        result.results.iter().any(|r| {
            let text = r.0.to_string();
            text.contains("1600 Pennsylvania Avenue NW")
        }),
        "结果中应包含美国地址"
    );
}
