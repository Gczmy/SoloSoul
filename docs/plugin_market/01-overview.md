## 1. 设计目标

为 SoloSoul 构建一套**本地优先、安全隔离、字段级授权**的插件系统，满足以下核心诉求：

1. **插件可请求用户数据**：插件通过声明式清单请求特定字段，经用户逐字段确认后由 Host 代理返回。
2. **用户掌握绝对权限**：每次敏感字段访问均需显式弹窗确认，支持会话级授权撤销与 TTL 自动失效。
3. **与 SoloSoul_plugin_market 无缝集成**：插件市场以 Git Submodule 形式存在，官方插件经 SHA-256 白名单校验后可从市场动态下载安装。
4. **零知识架构延续**：插件永不直接接触 Vault 数据库或 Master Key，所有数据交互必须经过 Rust Host Functions 代理。
5. **插件与主软件生命周期分离**：插件不随 App 二进制打包，拥有独立的安装目录和更新周期，卸载插件不影响主软件核心数据。

---

## 2. 架构总览与分层职责

### 2.1 系统架构图

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    SoloSoul_plugin_market (Git Submodule)               │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ GitHub Releases / CDN 分发                                       │   │
│  │  ├── registry.json        (市场白名单索引)                        │   │
│  │  └── com.solosoul.slotgo/                                       │   │
│  │       ├── manifest.json                                          │   │
│  │       └── plugin.wasm                                            │   │
│  └─────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                      构建时推送      │      运行时拉取
                      (CI/CD)        │      (App 启动/用户操作)
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         SoloSoul App (Flutter)                          │
│                                                                         │
│   ┌─────────────────┐      ┌─────────────────┐      ┌──────────────┐   │
│   │ PluginRegistry  │      │ PluginInstaller │      │ PluginStore  │   │
│   │ Service         │◄────►│ Service         │◄────►│ (Rust Host)  │   │
│   │                 │      │                 │      │              │   │
│   │ - 远程拉取       │      │ - 下载 wasm     │      │ - 加载 wasm  │   │
│   │   registry.json │      │ - SHA-256 校验  │      │ - 执行沙盒   │   │
│   │ - 本地缓存       │      │ - 解压到独立目录 │      │ - 管理 Session│   │
│   │ - 版本兼容性检查 │      │ - 更新 installed│      │              │   │
│   └─────────────────┘      └─────────────────┘      └──────────────┘   │
│                                                                         │
│   数据来源：                                                            │
│   1. 远程 CDN (https://plugins.solosoul.dev/registry.json)             │
│   2. 本地缓存 (~/.solosoul/plugins/registry.json)                      │
│   3. 本地文件 (用户手动导入 .wasm + manifest.json)                      │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         用户本地文件系统                                │
│                                                                         │
│   ~/.solosoul/                          # 主软件数据目录                 │
│   ├── acc_{accountId}/                  # Vault + 配置（已有）           │
│   │   ├── config.json                                                   │
│   │   ├── vault.db                                                      │
│   │   └── settings.json                                                 │
│   │                                                                     │
│   └── plugins/                          # 【插件独立目录】               │
│       ├── registry.json                 # 市场注册表本地缓存             │
│       ├── installed.json                # 本地已安装插件索引             │
│       │                                                                 │
│       └── com.solosoul.slotgo/          # 每个插件独立子目录             │
│           ├── manifest.json                                             │
│           ├── plugin.wasm                                               │
│           ├── config.json               # 插件私有配置                   │
│           └── cache/                    # 插件运行时缓存                 │
│                                                                         │
│   关键原则：plugins/ 与 acc_{accountId}/ 同级，互不侵入                  │
│   卸载主软件时，plugins/ 可选择保留或删除（由安装器决定）                │
└─────────────────────────────────────────────────────────────────────────┘
```

### 2.2 分层职责

| 层级 | 组件 | 职责 |
|------|------|------|
| **插件市场** | `SoloSoul_plugin_market/` | 插件源码、SDK、registry 白名单。**仅作为 Git Submodule + CDN 源存在**，不直接参与运行时。 |
| **Flutter UI** | `lib/presentation/pages/plugin_*.dart` | 插件市场页面、授权弹窗、会话管理、审计日志展示。 |
| **Flutter Service** | `lib/core/services/plugin_*.dart` | `PluginRegistryService`（远程注册表）、`PluginInstallerService`（下载/安装/卸载）、`PluginService`（调用 Rust FFI、监听 Consent Stream）。 |
| **Rust Host** | `native/src/plugin/` | `PluginStore`（独立目录读写）、Wasmtime 沙盒、Host Functions 实现、字段映射、Vault 解密代理、TTL 管理、Rate Limiting。 |
| **Go 后端** | `core/api/plugin.go` | **仅用于 Web UI / CLI 场景**。Flutter 客户端的插件系统完全由 Rust Host 主导，Go 层的 Session/Consent 作为历史兼容保留。 |

> **关键决策**：
> 1. 插件**不随 App 二进制打包**，拥有独立的安装目录 `~/.solosoul/plugins/`。
> 2. Flutter 端插件系统不经过 Go 后端。Rust Host 直接管理 Session、Consent、字段映射和 Vault 访问。
> 3. 插件市场通过 CDN 分发，App 启动时拉取 registry.json 缓存到本地。
