# SoloSoul 插件系统架构设计文档

> 版本: v1.2
> 日期: 2026-05-22
> 状态: 设计稿（待实施）
> 关联目录: `flutter/native/src/plugin/`, `SoloSoul_plugin_market/`, `core/api/plugin.go`

---

## 文档索引

本文档按模块切分为以下子文件，建议按顺序阅读：

| 序号 | 文件 | 内容 |
|------|------|------|
| 1 | [01-overview.md](01-overview.md) | 设计目标、架构总览与分层职责 |
| 2 | [02-market-structure.md](02-market-structure.md) | 插件市场目录结构、registry.json / manifest.json 规范 |
| 3 | [03-plugin-sdk.md](03-plugin-sdk.md) | 插件开发 SDK（Rust）：Host Functions 绑定、ABI、示例 |
| 4 | [04-rust-host.md](04-rust-host.md) | Rust Host 侧实现：PluginStore、SoloHostFunctions、沙盒执行、TTL 管理 |
| 5 | [05-field-mapping.md](05-field-mapping.md) | 字段路径到 UnifiedObject 的映射层 |
| 6 | [06-flutter-ui.md](06-flutter-ui.md) | Flutter 侧实现：授权弹窗、FRB 接口、Service 封装 |
| 7 | [07-plugin-dashboard.md](07-plugin-dashboard.md) | 插件看板页面设计（PluginDashboardPage）：布局、状态机、交互流程 |
| 8 | [08-data-flow.md](08-data-flow.md) | 权限与数据流完整流程 |
| 9 | [09-market-integration.md](09-market-integration.md) | 与 SoloSoul_plugin_market 的集成：CI/CD 发布流程 |
| 10 | [10-security.md](10-security.md) | 安全机制清单、Debug/Release 差异 |
| 11 | [11-roadmap.md](11-roadmap.md) | 五阶段实施 Roadmap |
| 12 | [12-code-integration.md](12-code-integration.md) | 与现有代码的衔接点 |
| 13 | [13-lifecycle.md](13-lifecycle.md) | 插件生命周期管理：安装、更新、卸载（与主软件分离） |
| 14 | [14-advanced-security.md](14-advanced-security.md) | 高级安全机制：JIT 即时解密、熔断机制 |
| 15 | [15-appendix.md](15-appendix.md) | 附录：Host Functions ABI 规范、错误码总表、版本兼容性检查流程 |

---

*本文档应与 `docs/TODO.md` 中 **P4: 插件系统** 条目同步更新。*
