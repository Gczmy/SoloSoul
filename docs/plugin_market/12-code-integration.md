## 13. 与现有代码的衔接点

| 现有文件 | 当前状态 | 需要修改 |
|----------|----------|----------|
| `flutter/native/src/plugin/mod.rs` | 已定义模块结构 | 启用 `sandbox` feature |
| `flutter/native/src/plugin/sandbox.rs` | Wasmtime 基础框架 | 补充 `execute()` + Store 级 TTL |
| `flutter/native/src/plugin/host.rs` | 全为 stub | 重写为完整 Host Functions + Consent + RateLimiting |
| `flutter/native/src/plugin/manifest.rs` | manifest 解析 | 复用 `field_matches` 到 Host，补充 `network_policy` 序列化 |
| `core/api/plugin.go` | PluginManager | 同步补充 `data_ttl_seconds` + `network_policy` + `plugin_api_version` 字段到 Go struct |
| `flutter/lib/core/services/` | 无 Plugin 相关 Service | 新增 `plugin_registry_service.dart` + `plugin_installer_service.dart` + `plugin_service.dart` |
| `SoloSoul_plugin_market/` | 仅有 README | 构建完整 SDK + 示例插件 + CI/CD 配置 |
| `.github/workflows/` | 无插件发布流水线 | 新增 `plugin_release.yml` |
