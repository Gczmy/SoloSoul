# SoloSoul Tauri 迁移计划

> 本文档描述将 SoloSoul 从现有 Flutter/Go/Rust 技术栈全面迁移至 Tauri + Rust 的路线图与实施策略。
>
> **目标**：统一后端语言、消除 FFI 开销、提升安全边界、降低跨平台维护成本。

---

## 1. 迁移动机与架构愿景

当前 SoloSoul 主项目采用 **Flutter（Dart UI）+ Rust（原生核心，通过 flutter_rust_bridge FFI 调用）** 的两层技术栈。虽然职责清晰，但跨语言调用带来了以下长期成本：

| 痛点 | 影响 |
|------|------|
| FFI 性能损耗 | Dart ↔ Rust 加密调用存在序列化与上下文切换开销 |
| 代码冗余 | `flutter/native/src/` 中的 Rust 代码与 FFI 绑定层耦合紧密，难以独立复用 |
| 维护复杂度 | 需要同时维护 Dart 与 Rust 两套语言环境、CI/CD 及平台适配 |
| 安全边界模糊 | 敏感数据在 Dart 与 Rust FFI 之间流转，增加了攻击面 |

**Tauri 统一后的架构愿景**：

```
┌─────────────────────────────────────────┐
│              前端进程 (WebView)           │
│   React / Vue / Svelte — 纯粹 UI 渲染   │
│         无业务逻辑，无敏感数据处理         │
└─────────────────┬───────────────────────┘
                  │  IPC (invoke / emit)
┌─────────────────▼───────────────────────┐
│              后端进程 (Rust)              │
│   ┌──────────────┐  ┌──────────────┐   │
│   │  加密核心      │  │  业务逻辑     │   │
│   │ Argon2id     │  │ 账户/Vault   │   │
│   │ AES-256-GCM  │  │ 同步/索引    │   │
│   │ zeroize      │  │ 插件沙盒     │   │
│   └──────────────┘  └──────────────┘   │
│         单一 Rust 二进制，统一编译         │
└─────────────────────────────────────────┘
```

- **前端**：仅负责 UI 展示与用户输入传递，禁止直接访问系统 API。
- **后端**：持有所有业务逻辑，加密、索引、文件读写等关键操作全部在 Rust 中完成。
- **IPC**：通过 `tauri::command` 与 `emit` 实现前后端通信，安全且高效。

---

## 2. 整体迁移策略：分步实施，控制风险

迁移采用稳健的三步走计划，预估总工期 **9–17 周**。

### 第 1 步：搭环境与建壳（1–2 周）

**目标**：跑通跨平台构建流程，建立可运行的 Tauri 空壳项目。

- [ ] 使用 `create-tauri-app` 初始化 Tauri v2 项目。
- [ ] 选择前端框架（推荐 **React** 或 **Solid.js**，兼顾生态与性能）。
- [ ] 熟悉 `src-tauri/` 目录结构（`Cargo.toml`、`tauri.conf.json`、`capabilities/`、`src/main.rs`）。
- [ ] 在 **Windows、macOS、Linux** 三大桌面平台成功编译并运行空壳。
- [ ] 配置 `tauri-build` 与前端框架的联调热重载（`tauri dev`）。
- [ ] 建立新的 GitHub Actions CI 流水线，覆盖三大桌面平台的构建验证。

**交付物**：
- 可独立运行的 Tauri 空壳项目（位于新仓库或现有仓库的 `tauri/` 目录）。
- 跨平台 CI 构建流水线通过。

### 第 2 步：迁移后端与实现 IPC（3–5 周）

**目标**：将 SoloSoul 的核心业务逻辑统一迁移至 Rust 后端，并通过 IPC 暴露给前端。

- [ ] **合并加密核心**：
  - 将 `flutter/native/src/crypto/` 中的 Argon2id + AES-256-GCM 逻辑提取为独立 Rust crate `solosoul-crypto`，解耦 FFI 绑定。
  - 引入 [`zeroize`](https://docs.rs/zeroize) crate，对所有密钥/密码实现自动/显式内存清零。
  - 配置 `lto = true`，利用链接时优化减少冗余副本。
- [ ] **迁移 Vault 存储**：
  - 将现有 Vault 存储逻辑（位于 `flutter/native/src/vault/`）重写为更清晰的 Rust 模块（基于 `rusqlite` + AES-GCM 应用层加密）。
  - 保持现有 Vault 数据格式兼容。
- [ ] **迁移账户与业务逻辑**：
  - 将 Dart 端的账户管理、会话管理、插件授权等逻辑下沉至 Rust。
  - 将 Profile 数据模型与验证器用 Rust 实现（借助 `serde` + `validator`）。
- [ ] **实现 IPC 接口**：
  - 使用 `#[tauri::command]` 宏将后端函数暴露为 IPC 命令。
  - 前端通过 `invoke('command_name', payload)` 调用后端。
  - 后端通过 `app.emit('event_name', payload)` 主动推送事件（如自动锁定通知、同步进度）。
- [ ] **权限配置**：
  - 在 `src-tauri/capabilities/` 中为每个 IPC 命令配置最小权限，遵循最小特权原则。
  - 所有涉及文件系统、系统托盘、剪贴板的命令必须显式授权。

**交付物**：
- 完整的 Rust 后端 crate，包含加密、Vault、账户、业务逻辑。
- 前后端 IPC 接口文档（命令列表、参数、返回值、事件名）。
- 单元测试覆盖 Rust 核心业务逻辑（`cargo test`）。

### 第 3 步：重构前端 UI（5–10 周）

**目标**：将现有 Flutter UI 完全重写为 Web 前端，整合加密与性能优化。

- [ ] **页面级重写**：
  - 逐一复刻现有 13 个 Flutter 页面（login, home, object_workspace, object_editor, profile, travel, financial, professional, settings, security_settings, sensitivity_settings, operation_log, trash）。
  - 保持原有 Liquid Glass 设计语言，或在前端框架中复刻玻璃质感 UI。
- [ ] **组件迁移**：
  - 将 `SensitiveValueWidget`、`SensitivityBlurredWidget`、`SensitivityTag` 等敏感数据组件用 React/Vue/Svelte 重写。
  - 统一密码验证对话框，禁止多处复制。
- [ ] **状态管理**：
  - 使用前端框架原生状态管理（如 Zustand / Pinia / Svelte Store）替代 `flutter_riverpod`。
  - 敏感状态（如会话令牌）仅缓存在内存，禁止写入 `localStorage`。
- [ ] **性能优化**：
  - 前端启用代码分割、Tree Shaking、图片压缩、gzip/brotli。
  - Rust 后端直接调用加密逻辑，无 FFI 开销，性能天然优于旧架构。
  - 配置 `Cargo.toml` Release 优化：`opt-level = "z"`, `strip = true`, `lto = true`。
- [ ] **移动端评估**：
  - Tauri v2 已支持 iOS/Android，可作为桌面端之外的第二目标。
  - 若需 React Native 移动端，Rust 核心逻辑可通过 `uniffi-react-native` 共享，逻辑零重复。

**交付物**：
- 功能完整的前端 UI，与现有 Flutter 版本功能对等。
- 优化后的 Release 二进制（桌面端 < 30MB 基础，启动 < 0.5s）。
- 端到端测试覆盖核心用户流程。

---

## 3. 关键技术决策

### 3.1 前端框架选择

| 框架 | 优势 | 劣势 | 建议 |
|------|------|------|------|
| **React** | 生态最丰富，团队熟悉度高 | 包体积略大 | 团队已熟悉，优先推荐 |
| **Solid.js** | 性能极佳，无虚拟 DOM，包体小 | 生态相对年轻 | 对性能敏感时的备选 |
| **Svelte** | 编译时优化，代码量最少 | 大型项目经验少 | 轻量项目的优雅选择 |

**当前建议**：优先 React，利用现有 `web/` 目录的部分组件资产，降低重写成本。

### 3.2 IPC 通信规范

```rust
// 后端：tauri/src/commands.rs
#[tauri::command]
async fn vault_unlock(
    state: tauri::State<'_, AppState>,
    password: String,
) -> Result<VaultStatus, VaultError> {
    // 所有敏感操作在 Rust 后端执行
    let key = derive_key(&password)?;
    state.vault.unlock(key).await
}
```

```typescript
// 前端：调用示例
import { invoke } from '@tauri-apps/api/core';

const status = await invoke('vault_unlock', { password: input });
```

**安全规则**：
1. 所有 `invoke` 调用必须在 `capabilities/*.json` 中显式声明。
2. 禁止将密钥、密码等敏感数据通过 IPC 从后端推送到前端（除非是加密后的密文）。
3. 前端绝不处理明文敏感数据。

### 3.3 内存安全：zeroize

Rust 后端所有持有敏感数据的类型必须实现 `Zeroize`：

```rust
use zeroize::{Zeroize, Zeroizing};

// 显式清零
let mut secret = vec![1, 2, 3, 4];
secret.zeroize(); // 内存被安全覆盖

// 自动清零（离开作用域时）
let password = Zeroizing::new(String::from("user_input"));
```

同时配合 `lto = true`，让编译器消除不必要的中间副本。

---

## 4. Tauri 跨平台能力评估

### 4.1 桌面端

| 平台 | 发布格式 | 说明 |
|------|---------|------|
| macOS | `.dmg`, `.app` | 原生体验，支持签名与公证 |
| Windows | `.msi`, `.exe` | 支持 NSIS 安装器 |
| Linux | `.deb`, `.rpm`, `.AppImage` | 覆盖主流发行版 |

**构建建议**：利用 GitHub Actions 的 `tauri-action`，在构建矩阵中并行编译，避免单一机器交叉编译的复杂性与可靠性问题。

### 4.2 移动端（Tauri v2）

Tauri v2 已原生支持 iOS 与 Android，可作为桌面之后的扩展目标：
- 共享同一套 Rust 后端逻辑。
- 前端使用移动适配的 Web 前端。
- 若选择 React Native，Rust 核心可通过 `uniffi-react-native` 桥接，逻辑完全复用。

---

## 5. 安全最佳实践清单

- [ ] **CSP（内容安全策略）**：在 `tauri.conf.json` 中配置严格 CSP，禁止内联脚本。
- [ ] **输入验证**：所有 IPC 入参在后端进行严格校验，不信任前端。
- [ ] **最小权限**：`capabilities/*.json` 中仅暴露必要命令，禁止通配符。
- [ ] **自动锁定**：复用现有逻辑，App 进入后台/非活跃状态时启动倒计时，超时锁定 Vault。
- [ ] **调试安全**：Release 构建剥离调试符号（`strip = true`），禁止前端 DevTools。
- [ ] **安全更新**：复用系统 WebView 的自动安全补丁路径，无需单独更新浏览器内核。

---

## 6. 性能与体积优化目标

| 指标 | 当前 Flutter | 目标 Tauri | 优化手段 |
|------|-------------|-----------|---------|
| 启动时间 | ~2–3s | **< 0.5s** | 原生二进制 + 系统 WebView |
| 基础内存占用 | ~80–120MB | **< 30MB** | 无 Chromium 内核，共享系统 WebView |
| Release 包体 | ~50–100MB | **3–5MB 基础** | `opt-level = "z"`, `strip = true`, `lto = true` |
| 加密调用延迟 | FFI + 序列化 | **直接调用** | Rust 后端内部直接调用，无跨语言开销 |

---

## 7. 风险与应对

| 风险 | 影响 | 应对措施 |
|------|------|---------|
| 重写 UI 工作量大 | 高 | 分步实施，先完成后端与 IPC，UI 按页面逐步迁移；保留旧 Flutter 代码作为参考 |
| Vault 数据格式不兼容 | 高 | 设计一次性 Rust 迁移脚本，在新 Tauri 首次启动时自动转换 |
| 移动端 Tauri 成熟度 | 中 | 桌面端优先交付，移动端作为第二阶段；保留 React Native + uniffi 作为备选 |
| 团队学习成本 | 中 | Tauri 文档完善，Rust 团队已有基础；第 1 步重点熟悉脚手架与 IPC |
| 第三方插件依赖 | 低 | 评估现有 `local_auth`（生物识别）、`flutter_secure_storage` 的 Tauri 替代方案 |

---

## 8. 迁移后的项目结构预览

```
SoloSoul/
├── tauri/                      # Tauri 主项目
│   ├── src-tauri/
│   │   ├── Cargo.toml          # Rust 依赖与编译优化配置
│   │   ├── tauri.conf.json     # Tauri 应用配置
│   │   ├── capabilities/       # IPC 权限声明
│   │   └── src/
│   │       ├── main.rs         # 应用入口
│   │       ├── commands.rs     # IPC 命令封装
│   │       ├── crypto/         # Argon2id + AES-256-GCM（统一后的唯一实现）
│   │       ├── vault/          # 加密存储（原 Go + Rust 合并）
│   │       ├── account/        # 账户与会话管理
│   │       ├── schema/         # Profile 数据模型与验证
│   │       └── plugin/         # Wasmtime 插件沙盒
│   ├── src/                    # 前端源码（React / Vue / Svelte）
│   ├── public/
│   └── package.json
├── rust-core/                  # 共享 Rust 核心（供 Tauri 桌面 + 移动端复用）
│   └── ...
├── docs/
│   └── design_map/
│       └── TAURI_MIGRATION_PLAN.md   # 本文档
└── .github/workflows/
    └── tauri-ci.yml            # 跨平台构建与发布
```

---

## 9. 总结

迁移到 Tauri 是一次让 SoloSoul **脱胎换骨** 的技术决策：

| 优势类别 | 具体体现 |
|---------|---------|
| ⚡ 性能与资源 | 启动快（<0.5s）、内存省（<30MB）、包体小（3–5MB） |
| 🛡️ 安全模型 | 前端沙箱化、最小权限 IPC、Rust 内存安全、zeroize 自动清零 |
| 🔧 开发体验 | 语言统一（Rust 统一后端与加密核心）、工具强大（Tauri CLI） |
| 🌍 跨平台 | 桌面全支持（Win/macOS/Linux），移动端原生支持（Tauri v2） |
| 🔗 可维护性 | 消除 FFI 与 Go 中间层，单一 Rust 后端，降低长期维护成本 |

这不仅解决了多语言栈的性能与维护开销，更利用 Rust 生态强化了零知识安全边界，为 SoloSoul 的未来扩展奠定了坚实基础。

---

*文档版本：v1.0*  
*创建日期：2026-06-04*  
*状态：计划中*
