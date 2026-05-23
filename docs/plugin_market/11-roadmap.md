## 12. 实施 Roadmap

| 阶段 | 任务 | 涉及文件 | 预计工作量 |
|------|------|----------|----------|
| **Phase 1** | 完善插件市场基础设施 | `SoloSoul_plugin_market/` | 2 天 |
| | - 编写 Rust SDK | `SDK/rust/src/lib.rs` | |
| | - 定义 JSON Schema（含 plugin_api_version） | `SDK/schema/manifest.schema.json` | |
| | - 创建示例插件 | `examples/hello_world/` | |
| | - 配置 CI/CD 推送到 CDN | `.github/workflows/plugin_release.yml` | |
| **Phase 2** | 打通 Rust Host + PluginStore | `flutter/native/src/plugin/` | 3 天 |
| | - 实现 PluginStore（独立目录读写） | `store.rs` | |
| | - 实现 ConsentChannel（tokio mpsc） | `host.rs` | |
| | - 实现字段映射层（UnifiedObject） | `host.rs` + `field_map.rs` | |
| | - 实现 RateLimiter + 网络白名单 | `host.rs` | |
| | - 实现 Store 级 TTL 隔离 | `sandbox.rs` | |
| | - 补充审计日志输出 | 新增 `audit.rs` | |
| **Phase 3** | 实现 Flutter 安装器 + 授权弹窗 | `flutter/lib/core/services/` | 3 天 |
| | - PluginRegistryService（远程/缓存） | `plugin_registry_service.dart` | |
| | - PluginInstallerService（安装/更新/卸载） | `plugin_installer_service.dart` | |
| | - PluginConsentDialog（带敏感度色标） | `widgets/plugin_consent_dialog.dart` | |
| | - PluginDashboardPage | `pages/plugin_dashboard_page.dart` | |
| | - PluginSessionProvider | `providers/plugin_session_provider.dart` | |
| **Phase 4** | 端到端集成测试 | `flutter/integration_test/` | 2 天 |
| | - 测试市场下载 -> 安装 -> 权限请求 -> 数据返回 | `plugin_e2e_test.dart` | |
| | - 测试卸载后数据隔离 | | |
| | - 测试 TTL 到期 Store 销毁 | | |
| | - 测试 Rate Limiting | | |
| **Phase 5** | 官方插件开发（SlotGo） | `SoloSoul_plugin_market/plugins/slotgo/` | 3 天 |
