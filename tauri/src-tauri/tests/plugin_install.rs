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
    assert!(result.is_ok(), "安装失败: {:?}", result.err());

    // 安装后应出现在已安装列表
    let installed = manager.list_installed().expect("列出已安装插件失败");
    assert!(installed.iter().any(|m| m.id == plugin_id));

    // 同时检查落盘位置与卸载行为，防止测试误用默认用户目录。
    let plugin_dir = data_dir.path().join("plugins").join(plugin_id);
    assert!(plugin_dir.join("manifest.json").is_file());
    assert!(plugin_dir.join("plugin.wasm").is_file());
    manager.uninstall(plugin_id).expect("卸载插件失败");
    assert!(!plugin_dir.exists());
    assert!(manager.list_installed().unwrap().is_empty());
    drop(manager);
    data_dir.close().expect("清理测试数据失败");
}
