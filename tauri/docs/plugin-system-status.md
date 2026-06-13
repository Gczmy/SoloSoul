# SoloSoul 插件系统实现状态

> 更新于 2026-06-13

## 已完成内容

### Phase 1：Rust 插件宿主核心

- **沙箱执行**：基于 `wasmtime 45.0.1` + `wasmtime-wasi` Preview1，仅继承 stdio，启用 Fuel 限制。
- **Host Functions**：在 `env` 模块暴露与 `SoloSoul_plugin_market/SDK/rust` 一致的 ABI：
  - `solosoul_request_field`
  - `solosoul_post_data`
  - `solosoul_log`
  - `solosoul_get_timestamp`
  - `solosoul_get_data_structure_tree`
  - `solosoul_result`
  - `solosoul_show_dialog`
  - 扩展：`solosoul_get_param`、`solosoul_get_locale`、`solosoul_request_consent`、`solosoul_sleep`
- **安装/更新/卸载**：从本地 `SoloSoul_plugin_market` 读取注册表与 `plugin.wasm`，校验 SHA-256 与应用版本兼容性。
- **会话与授权**：会话 TTL、授权请求通道（Phase 2 占位自动返回空字符串）。
- **审计日志**：同步内存审计，支持最近 N 条查询。
- **集成测试**：`tests/plugin_sandbox.rs` 成功运行 `hello_world` 插件。

### Phase 2：前端插件市场与 Dashboard

- **页面**：`PluginDashboardPage` 支持「全部 / 已安装 / 运行中 / 日志」四 Tab。
- **组件**：`PluginCard`、`PluginConsentDialog`、`PluginResultPanel`。
- **状态管理**：`pluginStore`（Zustand）处理市场列表、已安装列表、运行中插件输出与 Consent 请求。
- **国际化**：新增 `plugin` namespace（zh-CN / en-US）。
- **路由**：`/plugins` 已在 `App.tsx` 注册，设置页已添加入口。

### Phase 3：测试与调优

- Rust 新增 `matches_domain`、版本兼容性、SHA-256 单元测试。
- 前端新增 `PluginResultPanel.test.tsx`。
- `npm run check-all` 与 `cargo test` 全量通过。

## 已知限制

- `request_field` 当前为占位实现，直接返回空字符串；真实 Vault 数据查询将在后续接入 `VaultStore` 后实现。
- `post_data`、`show_dialog`、`get_data_structure_tree` 当前返回未实现错误码。
- 市场注册表与插件目录路径使用编译期 `CARGO_MANIFEST_DIR`；Release 分发时需要改为基于 `app.path().resource_dir()` 的资源目录。
- 官方插件 P0-P4 分批启用需要注册表新增 `tier` 或 `category` 字段，当前未实现。
