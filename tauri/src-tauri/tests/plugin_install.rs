use solo_soul::plugin::PluginManager;

#[tokio::test]
async fn test_install_plugin_from_market() {
    let manager = PluginManager::new().expect("创建 PluginManager 失败");

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
}
