# SoloSoul 插件系统实现状态

> 更新于 2026-06-13（最近一次提交 `2c87580`）

## 已完成内容

### Phase 1：Rust 插件宿主核心

- **沙箱执行**：基于 `wasmtime 45.0.1` + `wasmtime-wasi` Preview1，仅继承 stdio，启用 Fuel 限制。
- **Host Functions**：在 `env` 模块暴露与 `SoloSoul_plugin_market/SDK/rust` 一致的 ABI：
  - `solosoul_request_field` —— 真实 Vault 字段解析（支持 `typeId.count`、`typeId[index].prop`、`typeId.prop`）
  - `solosoul_post_data` —— 带域名白名单的同步 HTTP POST 代理
  - `solosoul_http_request` / `solosoul_http_poll` / `solosoul_http_read` / `solosoul_http_close` —— 异步 HTTP 轮询 ABI，支持 GET/POST/PUT/PATCH/DELETE
  - `solosoul_log`
  - `solosoul_get_timestamp`
  - `solosoul_get_data_structure_tree` —— 返回用户模板元数据 + 对象数量
  - `solosoul_result`
  - `solosoul_show_dialog` —— 通用阻塞对话框（alert / confirm / input / radio_list / checkbox_list）
  - `solosoul_get_param`
  - `solosoul_get_locale`
  - `solosoul_request_consent` —— 字段级授权请求，已结合 Vault Schema 返回真实字段标签与敏感度
  - `solosoul_sleep`
- **安装/更新/卸载**：从本地 `SoloSoul_plugin_market` 读取注册表与 `plugin.wasm`，校验 SHA-256 与应用版本兼容性。
- **Release 资源路径**：`SoloSoul_plugin_market` 优先从 Tauri 资源目录读取，开发模式回退源码路径。
- **会话与授权**：会话 TTL、`ConsentManager` 阻塞等待用户响应。
- **审计日志**：持久化到 `~/.solosoul/plugin_audit.jsonl`，保留最近 2000 条，文件权限 `0600`。
- **在线注册表更新**：`plugin_update_registry` 从远程拉取 `registry.json` + `.minisig`，使用 Minisign 校验签名后原子写入本地市场目录。
- **官方插件分批启用**：`RegistryEntry` / `PluginManifest` 增加 `tier`、`category` 与 `params`；前端 Dashboard 默认启用 P0/P1。
- **集成测试**：`tests/plugin_sandbox.rs` 成功运行 `hello_world` 插件；新增 `plugin_address_fmt.rs` 等 Rust 单元测试。

### Phase 2：前端插件市场与 Dashboard

- **页面**：`PluginDashboardPage` 支持「全部 / 已安装 / 运行中 / 日志」四 Tab，带 tier chips 与默认 P0/P1 启用。
- **组件**：`PluginCard`、`PluginConsentDialog`、`PluginResultPanel`、新增 `PluginDialog`、`PluginRunParamsDialog`。
- **结果导出**：`PluginResultPanel` 每个结果卡片支持一键复制为 JSON / Markdown，纯前端实现，不依赖后端权限。
- **运行中插件持久化**：`pluginStore` 使用 `zustand/persist` 将 `runningPlugins` 落盘到 `localStorage`，刷新页面后可恢复日志、结果与运行状态。
- **状态管理**：`pluginStore`（Zustand）处理市场列表、已安装列表、运行中插件输出、Consent 请求、Dialog 请求。
- **国际化**：新增 `plugin` namespace（zh-CN / en-US）。
- **路由**：`/plugins` 已在 `App.tsx` 注册，设置页已添加入口。

### Phase 3：测试与调优

- Rust 新增 `matches_domain`、版本兼容性、SHA-256、字段解析、数据结构树单元测试。
- 前端新增 `PluginResultPanel.test.tsx`。
- 新增 Playwright E2E：`e2e/plugin-lifecycle.spec.ts` 覆盖安装、运行、对话框响应、结果渲染。
- `npm run check-all`、`cargo test`、`npm run test:e2e` 全量通过。

## 剩余任务与建议

| # | 任务 | 说明 | 优先级 |
|---|------|------|--------|
| 1 | ~~`post_data` 异步化与响应通道~~ | ✅ 新增异步 HTTP 轮询 ABI：`solosoul_http_request` / `http_poll` / `http_read` / `http_close`；同步 `post_data` 保留兼容。 | P2 |
| 2 | ~~插件结果导出~~ | ✅ `PluginResultPanel` 每个结果卡片支持复制为 JSON / Markdown。 | P2 |
| 3 | ~~运行中插件列表持久化/恢复~~ | ✅ `pluginStore` 通过 `zustand/persist` 持久化 `runningPlugins` 到 `localStorage`，刷新后可恢复。 | P3 |
| 4 | ~~`plugin_update_registry` 在线更新~~ | ✅ 从 `SOLOSOUL_REGISTRY_URL` 拉取注册表，Minisign 验证签名后原子写入本地 `registry.json`。 | P3 |
| 4 | **`plugin_update_registry` 在线更新** | 当前 registry 为本地静态文件；后续可实现从远程 URL 拉取最新 registry 并校验签名。 | P3 |
| 5 | **插件市场子模块 CI 集成** | 主项目 CI 中验证 `SoloSoul_plugin_market/registry.json` 与子模块指针一致性。 | P3 |
| 6 | **官方 P2/P3/P4 插件** | 当前仅 P0/P1 默认启用；需要实际填充 P2–P4 官方插件并完善权限审核。 | P4 |
| 7 | **Wasm 插件崩溃隔离** | 单个插件 Fuel 耗尽或 panic 时，确保不影响宿主与其他插件运行。 | P4 |
| 8 | **文档与 SDK 示例** | 补充 JS/Python SDK 占位实现与 Wasm 插件开发示例。 | P4 |

## 已知限制

- `plugin:event|listen` 等事件机制已通过前端 `Channel` 支持；纯 WebDriver/Playwright 测试需 mock Tauri 内部 IPC。
- 插件网络策略 `network_policy` 当前在 manifest 中可选；若缺失则默认放行，生产环境建议强制显式声明。
- E2E 测试基于 Vite dev server + IPC mock，未覆盖真实 Tauri 打包后的 Webview 行为。
