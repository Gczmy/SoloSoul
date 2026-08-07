# 1. 架构总览与分发链路

> 与 `tauri/crates/solosoul-plugin/`（Rust 宿主）及 `tauri/src/`（Tauri 前端）实现一致。

## 1.1 设计目标

SoloSoul 插件系统为个人数字孪生引擎提供可扩展能力，核心原则：

1. **本地优先、安全隔离**：插件编译为 WebAssembly（`wasm32-wasip1`），在客户端内置的 Wasmtime 沙盒中运行，所有数据处理在用户本机完成。
2. **字段级授权**：插件通过声明式清单（`required_fields` / `optional_fields`）请求字段，敏感字段经用户逐项确认（Consent）后由 Host 代理返回，支持数据 TTL 自动失效与会话级撤销。
3. **零知识架构延续**：插件永不直接接触 Vault 数据库或主密钥，所有数据交互必须经过 Rust Host Functions 代理。
4. **零服务器分发**：插件市场以公开 GitHub 仓库（子模块）存在，本地生成 `registry.json` 索引后 push 即发布，客户端经 CDN / Raw 拉取。
5. **生命周期分离**：插件拥有独立的安装目录与更新周期，不随 App 二进制打包，卸载不影响 Vault 数据。

## 1.2 系统架构

```
┌──────────────────────────────  SoloSoul 客户端（Tauri: React + Rust）  ─────────────────────────────┐
│                                                                                                      │
│  前端（React + TypeScript）                                                                          │
│  ├─ 插件看板 PluginDashboardPage（tauri/src/pages/ai/）                                               │
│  ├─ 授权弹窗 / 参数弹窗 / 结果面板（tauri/src/components/plugin/）                                    │
│  └─ 插件状态 store（tauri/src/stores/pluginStore.ts）                                                 │
│        │  Tauri IPC（plugin_list_all / plugin_install / plugin_run / …）                              │
│        ▼                                                                                             │
│  Rust 宿主 solosoul-plugin crate（tauri/crates/solosoul-plugin/）                                     │
│  ├─ PluginManager      安装 / 更新 / 卸载 / 运行编排                                                    │
│  ├─ PluginRegistry     注册表加载与远程更新（minisign 验签）                                           │
│  ├─ WasmSandbox        Wasmtime 沙盒（WASI Preview1 + 燃料上限 + 编译缓存）                            │
│  ├─ SoloHostFunctions  Host Functions（字段 / 网络 / 结果 / 审计）                                    │
│  ├─ FieldResolver      字段路径解析（typed field + contracts + Vault）                                │
│  ├─ ConsentManager / RateLimiter / AuditLogger / SessionManager                                       │
│  └─ PluginStore        插件安装目录（{data_dir}/plugins/）                                            │
│                                                                                                      │
│  数据：Vault（加密存储）—— 插件只能经 Host Functions 访问，无法直接触达                                  │
└──────────────────────────────────────────────────────────────────────────────────────────────────────┘
        │ ① 拉取注册表 registry.json（minisign 验签）
        │ ② 下载 plugin.wasm（SHA-256 校验）
        ▼
┌───────────────────────────────────  SoloSoul_plugin_market（子模块）  ───────────────────────────────┐
│  plugins/{plugin_id}/    插件源码 + manifest.json + plugin.wasm（提交即发布）                          │
│  registry.json           插件索引（scripts/generate_registry.py 本地生成）                            │
│  SDK/rust                Rust SDK（solosoul-plugin-sdk）                                              │
└──────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

## 1.3 分层职责

| 层级 | 组件 | 职责 |
|------|------|------|
| **插件市场** | `SoloSoul_plugin_market/`（子模块） | 插件源码、SDK、`registry.json` 索引。仅作为分发源存在，不参与运行时 |
| **Tauri 前端** | `tauri/src/pages/ai/`、`tauri/src/components/plugin/`、`tauri/src/stores/pluginStore.ts` | 看板、授权/参数弹窗、结果展示、运行状态管理 |
| **Rust Host** | `tauri/crates/solosoul-plugin/` | Wasmtime 沙盒、Host Functions、字段解析、Consent、TTL、速率限制、审计 |
| **Vault** | `tauri/crates/solosoul-vault/` | 加密数据存储，仅经 Host 代理暴露给插件 |

## 1.4 客户端消费链路

1. **市场目录定位**（`paths.rs`）：Release 构建时插件市场作为 Tauri 资源打包（`资源目录/SoloSoul_plugin_market/`）；开发模式（`debug_assertions`）回退到源码相对路径。
2. **注册表**：启动时从 `https://plugins.solosoul.app/registry.json` 拉取（可用 `SOLOSOUL_REGISTRY_URL` 覆盖），用 `SOLOSOUL_REGISTRY_PUBKEY` 对应私钥的 **minisign 签名**校验完整性；未配置公钥或拉取失败时，回落使用随应用打包的 bundled 注册表（`registry.rs::update_from_remote`）。
3. **插件二进制**：按注册表条目的 `download_url`（jsDelivr CDN）下载，失败回退 `raw_url`（GitHub Raw）；再失败则尝试 `install_bundled_fallback`（随应用分发的本地副本）。
4. **安装校验**：下载完成后计算 SHA-256 并与注册表记录的 `sha256` 强制比对，不一致即拒绝安装（`manager.rs::install_from_registry`）。
5. **兼容性检查**：`min_app_version ≤ 当前版本 ≤ max_app_version`（`version.rs::is_version_compatible`）。
6. **运行**：Wasmtime 沙盒执行——WASI Preview1、单次运行 100 亿燃料上限、stdio 静默丢弃；桌面端 Cranelift JIT，Android/iOS 自动切换 Pulley 解释器；同一 wasm 编译产物以 SHA-256 为键进程级缓存。

## 1.5 数据目录

| 路径 | 说明 |
|------|------|
| `{data_dir}/plugins/{plugin_id}/` | 已安装插件（`manifest.json` + `plugin.wasm`），由 `PluginStore` 管理 |
| `{data_dir}/plugins/registry.json` | 远程注册表缓存（可写，替换 bundled 只读副本） |
| `{data_dir}/plugin_audit.log` | 插件审计日志（`audit.rs`） |

> 无 `installed.json` 索引文件——已安装插件列表通过扫描 `plugins/` 目录实时获取（`store.rs::installed_manifests`）。
