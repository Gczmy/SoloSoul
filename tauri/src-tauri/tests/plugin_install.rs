use solo_soul::plugin::PluginManager;
use tempfile::TempDir;

#[tokio::test]
async fn test_install_plugin_from_market() {
    // 市场资源只读；安装、卸载及审计记录均留在本测试的临时目录。
    let data_dir = TempDir::new().expect("创建临时数据目录失败");
    let manager = PluginManager::new_with_dirs(
        solo_soul::plugin::paths::default_market_dir(),
        data_dir.path().to_path_buf(),
    )
    .expect("创建 PluginManager 失败");

    // 任选一个 P0 官方插件
    let plugin_id = "com.solosoul.official.phone-fmt";
    let entry = manager.list_all(None).expect("列出市场插件失败");
    let info = entry
        .iter()
        .find(|i| i.plugin_id == plugin_id)
        .expect("找不到插件");
    let version = info
        .registry_entry
        .latest_version
        .clone()
        .expect("无最新版本");

    // 先卸载，确保测试可重复
    let _ = manager.uninstall(plugin_id);

    let result = manager.install_from_registry(plugin_id, &version).await;
    let result = result.expect("安装失败");
    assert_eq!(result.plugin_id, plugin_id);

    // 安装后应出现在已安装列表
    let installed = manager.list_installed().expect("列出已安装插件失败");
    assert!(installed.iter().any(|m| m.id == plugin_id));

    // 同时检查落盘位置与卸载行为，防止测试误用默认用户目录。
    let plugin_dir = data_dir.path().join("plugins").join(plugin_id);
    let current: serde_json::Value = serde_json::from_slice(
        &std::fs::read(plugin_dir.join("current.json")).expect("缺少安装发布指针"),
    )
    .expect("发布指针格式错误");
    assert_eq!(current["schemaVersion"], 1);
    let generation = current["generation"].as_str().expect("缺少版本目录");
    assert!(generation.starts_with("v-"));
    assert!(generation[2..]
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric()));
    let version_dir = plugin_dir.join("versions").join(generation);
    assert!(version_dir.join("manifest.json").is_file());
    assert!(version_dir.join("plugin.wasm").is_file());

    // 经实际运行入口校验同代清单和 WASM，不能只检查目录或指针存在。
    let store = solosoul_plugin::PluginStore::new_with_data_dir(data_dir.path().to_path_buf())
        .expect("创建测试 Store 失败");
    let (manifest, wasm) = store.load_plugin(plugin_id).expect("安装包无法成对加载");
    assert_eq!(manifest.id, result.plugin_id);
    assert_eq!(manifest.version, result.version);
    assert!(!wasm.is_empty());
    assert_eq!(current["wasmSize"].as_u64(), Some(wasm.len() as u64));
    assert_eq!(
        std::fs::read(version_dir.join("plugin.wasm")).unwrap(),
        wasm
    );
    let stored_manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(version_dir.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(stored_manifest, serde_json::to_value(&manifest).unwrap());
    manager.uninstall(plugin_id).expect("卸载插件失败");
    assert!(!plugin_dir.exists());
    assert!(manager.list_installed().unwrap().is_empty());
    drop(manager);
    data_dir.close().expect("清理测试数据失败");
}
