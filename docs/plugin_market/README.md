# SoloSoul 插件系统架构文档

> 版本: v2.0
> 日期: 2026-08-07
> 状态: **已实现**（与 Tauri 客户端代码一致）
> 关联代码: `tauri/crates/solosoul-plugin/`（Rust 宿主）、`tauri/src/pages/ai/PluginDashboardPage.tsx` 与 `tauri/src/components/plugin/`（前端）、`SoloSoul_plugin_market/`（插件市场子模块）

---

## 文档索引

| 序号 | 文件 | 内容 |
|------|------|------|
| 1 | [01-overview.md](01-overview.md) | 架构总览、分层职责与客户端分发链路 |
| 2 | [02-runtime.md](02-runtime.md) | Rust Host 运行时：模块地图、生命周期、数据流、安全机制、ABI 与错误码 |
| 3 | [03-frontend.md](03-frontend.md) | Tauri 前端：插件看板、授权弹窗、事件流与 IPC 命令 |

## 与其他文档的分工

| 文档 | 读者 | 内容定位 |
|------|------|----------|
| 本目录（3 篇） | SoloSoul 客户端开发者 | 客户端侧架构：Rust Host、前端 UI、安全机制 |
| [子模块 README](../../SoloSoul_plugin_market/README.md) | 插件开发者 | 市场结构、manifest.json / registry.json 规范、SDK API、发布流程 |
| [wasm-plugin-development-guide.md](../wasm-plugin-development-guide.md) | 插件开发者 | ABI 规范、错误码、从零开发插件的完整步骤 |

---

## 实现状态速览

| 设计项 | 状态 | 实现位置 |
|--------|------|----------|
| Wasm 沙盒执行（Wasmtime + WASI Preview1） | ✅ | `tauri/crates/solosoul-plugin/src/sandbox.rs` |
| Host Functions（字段/网络/结果/审计） | ✅ | `tauri/crates/solosoul-plugin/src/host.rs` |
| 字段级授权（Consent）+ TTL | ✅ | `consent.rs`、`host.rs`、`session.rs` |
| 字段解析（typed field + contracts + Vault 解析器） | ✅ | `field.rs` |
| 速率限制 / 网络白名单 / 审计日志 | ✅ | `rate_limiter.rs`、`host.rs`、`audit.rs` |
| 注册表拉取（minisign 验签）+ SHA-256 安装校验 | ✅ | `tauri/crates/solosoul-plugin/registry.rs`、`manager.rs` |
| 插件看板 + 授权弹窗（Tauri/React） | ✅ | `tauri/src/pages/ai/`、`tauri/src/components/plugin/` |
| 21 个官方插件 | ✅ | `SoloSoul_plugin_market/plugins/` |
| JIT 即时解密 / 熔断机制 | ❌ 未实施 | 原设计提案，未进入实现（详见 02-runtime.md §7） |
