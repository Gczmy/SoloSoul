# SoloSoul 插件系统实现状态

> 更新于 2026-06-13（最近一次提交 `2ddfa1c`）

## 已完成内容

### Phase 1：Rust 插件宿主核心

- **沙箱执行**：基于 `wasmtime 45.0.1` + `wasmtime-wasi` Preview1，仅继承 stdio，启用 Fuel 限制。
- **Host Functions**：在 `env` 模块暴露与 `SoloSoul_plugin_market/SDK/rust` 一致的 ABI：
  - `solosoul_request_field` —— 真实 Vault 字段解析（支持 `typeId.count`、`typeId[index].prop`、`typeId.prop`）
  - `solosoul_post_data` —— 带域名白名单的同步 HTTP POST 代理
  - `solosoul_log`
  - `solosoul_get_timestamp`
  - `solosoul_get_data_structure_tree` —— 返回用户模板元数据 + 对象数量
  - `solosoul_result`
  - `solosoul_show_dialog` —— 通用阻塞对话框（alert / confirm / input / radio_list / checkbox_list）
  - `solosoul_get_param`
  - `solosoul_get_locale`
  - `solosoul_request_consent` —— 字段级授权请求
  - `solosoul_sleep`
- **安装/更新/卸载**：从本地 `SoloSoul_plugin_market` 读取注册表与 `plugin.wasm`，校验 SHA-256 与应用版本兼容性。
- **Release 资源路径**：`SoloSoul_plugin_market` 优先从 Tauri 资源目录读取，开发模式回退源码路径。
- **会话与授权**：会话 TTL、`ConsentManager` 阻塞等待用户响应。
- **审计日志**：持久化到 `~/.solosoul/plugin_audit.jsonl`，保留最近 2000 条，文件权限 `0600`。
- **官方插件分批启用**：`RegistryEntry` / `PluginManifest` 增加 `tier` 与 `category`；前端 Dashboard 默认启用 P0/P1。
- **集成测试**：`tests/plugin_sandbox.rs` 成功运行 `hello_world` 插件；新增 `plugin_address_fmt.rs` 等 Rust 单元测试。

### Phase 2：前端插件市场与 Dashboard

- **页面**：`PluginDashboardPage` 支持「全部 / 已安装 / 运行中 / 日志」四 Tab，带 tier chips 与默认 P0/P1 启用。
- **组件**：`PluginCard`、`PluginConsentDialog`、`PluginResultPanel`、新增 `PluginDialog`。
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
| 1 | **`request_consent` 敏感度与字段标签** | 当前 `request_consent` 固定 `sensitivity_level="sensitive"`、fieldLabel 等于 fieldId；应结合 Schema 元数据返回真实敏感度与可读标签。 | P1 |
| 2 | **`post_data` 异步化与响应通道** | 当前 `post_data` 在 `spawn_blocking` 线程中同步阻塞 HTTP 请求；建议改为 async 并通过 Channel 返回，避免阻塞 Wasm 执行线程。 | P2 |
| 3 | **插件运行参数 UI** | `plugin_run` 的 `params` 目前只能传空对象；Dashboard 可支持用户输入插件参数（基于 manifest 声明的参数 Schema）。 | P2 |
| 4 | **运行中插件列表持久化/恢复** | 当前运行状态仅在前端内存；刷新页面后丢失。可考虑会话保存或运行日志回放。 | P3 |
| 5 | **插件结果导出** | `PluginResultPanel` 的结果可复制/导出为 JSON/Markdown。 | P3 |
| 6 | **`plugin_update_registry` 在线更新** | 当前 registry 为本地静态文件；后续可实现从远程 URL 拉取最新 registry 并校验签名。 | P3 |
| 7 | **插件市场子模块 CI 集成** | 主项目 CI 中验证 `SoloSoul_plugin_market/registry.json` 与子模块指针一致性。 | P3 |
| 8 | **官方 P2/P3/P4 插件** | 当前仅 P0/P1 默认启用；需要实际填充 P2–P4 官方插件并完善权限审核。 | P4 |
| 9 | **Wasm 插件崩溃隔离** | 单个插件 Fuel 耗尽或 panic 时，确保不影响宿主与其他插件运行。 | P4 |
| 10 | **文档与 SDK 示例** | 补充 JS/Python SDK 占位实现与 Wasm 插件开发示例。 | P4 |

## 已知限制

- `plugin:event|listen` 等事件机制已通过前端 `Channel` 支持；纯 WebDriver/Playwright 测试需 mock Tauri 内部 IPC。
- 插件网络策略 `network_policy` 当前在 manifest 中可选；若缺失则默认放行，生产环境建议强制显式声明。
- E2E 测试基于 Vite dev server + IPC mock，未覆盖真实 Tauri 打包后的 Webview 行为。
