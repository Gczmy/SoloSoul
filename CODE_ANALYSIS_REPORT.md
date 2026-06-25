# 代码分析修复报告

> 最后更新：2026-06-25 20:48:40
> 当前分支：`master`
> 修复轮次：1（初始分析）

---

## 执行摘要

本次审查依据 `docs/review_code_process.md` 对全库进行了扫描，已删除旧报告并重新生成。当前代码库存在 **编译/格式化阻塞问题** 以及大量架构、性能、安全与规范类债务。所有问题均标记为 `[ ]` 待修复，等待进一步指令后再进入修复阶段。

**关键阻塞项（P0）**：
- CLI 项目因 `ObjectSummary` 结构体缺少新增字段 `property_labels` 无法编译/测试。
- Tauri 主项目 Clippy 严格模式下存在 5 个错误，导致 `cargo clippy -- -D warnings` 失败。
- Tauri 与 CLI 均存在 `cargo fmt --check` 不一致。
- 自动更新配置（端点 / 公钥）疑似错误，将导致发布版本更新失效。

**检查基线结果**：

| 检查项 | 命令 | 结果 | 备注 |
|--------|------|------|------|
| Tauri Rust 格式化 | `cd tauri && cargo fmt --check` | ❌ 失败 | 多处 diff，主要位于 `export_import/`、`object/mod.rs`、`attachment.rs`、`storage.rs` |
| Tauri Rust Clippy | `cd tauri && cargo clippy -- -D warnings` | ❌ 失败 | 5 个错误：`needless_borrows_for_generic_args`、`collapsible_if`、`map_flatten`、`needless_borrow` |
| Tauri Rust 测试 | `cd tauri && cargo test` | ✅ 通过 | 513 个测试通过；1 个 `unused variable: langs` 编译警告 |
| TypeScript 类型检查 | `cd tauri && npx tsc --noEmit` | ✅ 通过 | 无错误 |
| ESLint | `cd tauri && npm run lint` | ⚠️ 警告 | 2 条 `no-unused-vars` warning |
| 前端单元测试 | `cd tauri && npm run test` | ✅ 通过 | 372 个测试通过 |
| CLI 格式化 | `cd solosoul_cli && cargo fmt --check` | ❌ 失败 | 多处 diff，主要位于 `screens/help.rs`、`commands/attachment.rs`、`app.rs` |
| CLI Clippy | `cd solosoul_cli && cargo clippy -- -D warnings` | ✅ 通过 | 无错误 |
| CLI 单元测试 | `cd solosoul_cli && cargo test` | ❌ 失败 | 编译错误：`missing field property_labels` |

---

## 问题清单（按优先级 P0 > P1 > P2）

| ID | 优先级 | 类别 | 文件位置 | 描述 | 状态 |
|----|--------|------|----------|------|------|
| P0-001 | P0 | 编译错误 | `solosoul_cli/src/commands/vault_write.rs:1180` | `ObjectSummary` 初始化缺少新增字段 `property_labels`，CLI 无法编译/测试 | `[ ]` 待修复 |
| P0-002 | P0 | 静态分析 | `tauri/src-tauri/src/commands/attachment.rs:585`<br>`tauri/src-tauri/src/commands/export_import/export.rs:22,228`<br>`tauri/src-tauri/src/commands/export_import/import.rs:470,473` | Clippy 严格模式下 5 个错误，导致 CI `rust-check` 失败 | `[ ]` 待修复 |
| P0-003 | P0 | 代码规范 | `tauri/crates/solosoul-vault/src/storage.rs:2010`<br>`tauri/src-tauri/src/commands/attachment.rs:566-585`<br>`tauri/src-tauri/src/commands/export_import/export.rs:27,52`<br>`tauri/src-tauri/src/commands/export_import/import.rs:253`<br>`tauri/src-tauri/src/commands/object/mod.rs:149-508` | `cargo fmt --check` 在 Tauri workspace 中报告多处格式不一致 | `[ ]` 待修复 |
| P0-004 | P0 | 代码规范 | `solosoul_cli/src/app.rs:850`<br>`solosoul_cli/src/commands/attachment.rs:182`<br>`solosoul_cli/src/screens/help.rs:65-94` | `cargo fmt --check` 在 CLI workspace 中报告多处格式不一致 | `[ ]` 待修复 |
| P0-005 | P0 | 配置错误 | `tauri/src-tauri/tauri.conf.json:84`<br>`SoloSoul-Releases/latest.json:8,12` | 自动更新 endpoint 指向 `github.com/Gczmy/SoloSoul`，但项目远程仓库为 `Gczmy/SoloSoul_code`，会导致 404 | `[ ]` 待修复 |
| P0-006 | P0 | 安全/配置 | `tauri/src-tauri/tauri.conf.json:86` | updater `pubkey` 疑似为 `.pub` 文件整体内容的 base64，而非 minisign 公钥行 `RW...` 的 base64，签名校验会失败 | `[ ]` 待修复 |
| P0-007 | P0 | 架构/规范 | `tauri/src/pages/settings/GlobalAttachmentManager.tsx:151` | 组件共 1482 行，主函数 1234 行非注释代码，职责严重过载 | `[ ]` 待修复 |
| P0-008 | P0 | 架构/规范 | `tauri/src/components/object/AttachmentViewer.tsx:100` | 组件共 1238 行，主函数 1202 行，与 GlobalAttachmentManager 大量重复 | `[ ]` 待修复 |
| P0-009 | P0 | 性能 | `tauri/src/pages/workspace/ObjectWorkspacePage.tsx:256-300` | 搜索输入变化时触发 `snapshot_count_batch` / `attachment_count_batch`，造成高频 IPC 抖动 | `[ ]` 待修复 |
| P0-010 | P0 | 性能/架构 | `tauri/src/pages/workspace/ObjectWorkspacePage.tsx:137-156, 324-359` | 直接操作 `e.currentTarget.style.background`，绕开 React 状态管理，易导致样式不一致与重排 | `[ ]` 待修复 |
| P0-011 | P0 | 架构/重复代码 | `tauri/src/pages/ai/LlmChatPage/useLlmChat.ts:90`<br>`tauri/src/components/layout/AiQuickChatPopover.tsx:65` | 两套 LLM 聊天逻辑重复实现超过 300 行 | `[ ]` 待修复 |
| P0-012 | P0 | 潜在漏洞 | `tauri/src/stores/pluginStore.ts:208,218,240` | 插件事件 JSON 数据直接 `JSON.parse` 后裸 `as` 断言，无 schema 校验 | `[ ]` 待修复 |
| P0-013 | P0 | Bug | `tauri/src/hooks/useExportEstimate.ts:32-74` | `useEffect` 依赖数组遗漏 `scope`，并禁用 `exhaustive-deps`，导致估算状态陈旧 | `[ ]` 待修复 |
| P1-001 | P1 | 安全架构 | `tauri/crates/solosoul-crypto/src/kdf.rs:13-34`<br>`tauri/crates/solosoul-core/src/vault_service.rs:278,384,479,547`<br>`tauri/crates/solosoul-core/src/auth.rs:17,64` | 文档承诺的 `SOLOSOUL_SECURE=1` 切换未实现，代码硬编码 `KdfConfig::balanced()`（16 MiB / 3 iter） | `[ ]` 待修复 |
| P1-002 | P1 | 安全架构 | `tauri/crates/solosoul-core/src/vault_service.rs:36-44` | Windows 下 `set_private_dir` / `set_private_file` 为空操作，不设置 ACL | `[ ]` 待修复 |
| P1-003 | P1 | 安全架构 | `tauri/crates/solosoul-core/src/biometric/legacy.rs:21` | 非测试分支存在硬编码 32 字节密钥 `BIO_FILE_KEY_SECRET` | `[ ]` 待修复 |
| P1-004 | P1 | CI/构建 | `.github/workflows/ci_cd.yml` | 缺少 `ci_cd.yml` 文档承诺的 build-macos、build-windows、release Job | `[ ]` 待修复 |
| P1-005 | P1 | CI/构建 | `.github/workflows/ci_cd.yml` | `ci_cd.yml` 未包含 CLI 检查，而 `pr_check.yml` 已包含 | `[ ]` 待修复 |
| P1-006 | P1 | CI/构建 | `.github/workflows/plugin_release.yml:5,19,21,39-105` | Action 名称错误、target 应为 `wasm32-wasip1`、触发分支与默认分支不一致、CI 内重复生成 registry.json | `[ ]` 待修复 |
| P1-007 | P1 | 配置 | `tauri/src-tauri/tauri.conf.json:2` | `$schema` 指向非官方仓库，版本与依赖不一致 | `[ ]` 待修复 |
| P1-008 | P1 | 配置 | `tauri/package.json:23` | `@tauri-apps/cli` 被放在 `dependencies`，应移到 `devDependencies` | `[ ]` 待修复 |
| P1-009 | P1 | 架构/重复代码 | `tauri/src-tauri/src/plugin/`<br>`tauri/crates/solosoul-plugin/src/` | 两套插件运行时实现并行维护，功能高度重叠 | `[ ]` 待修复 |
| P1-010 | P1 | 测试覆盖 | 多处（见详细描述） | 插件安全、LLM 核心、导入导出、CLI 关键命令等模块缺少单元测试 | `[ ]` 待修复 |
| P1-011 | P1 | 依赖 | `tauri/Cargo.toml:35`<br>`solosoul_cli/Cargo.toml:57` | Tauri workspace 使用 `thiserror 1.x`，CLI 使用 `thiserror 2.x`，存在 major 版本冲突 | `[ ]` 待修复 |
| P1-012 | P1 | 测试策略 | `tauri/package.json:13`<br>`sdk/js/package.json:15` | `npm test` 使用 `--passWithNoTests`，掩盖测试缺失 | `[ ]` 待修复 |
| P1-013 | P1 | 代码规范 | `tauri/src/pages/workspace/ObjectWorkspacePage.tsx:35` | 组件主函数 653 行，职责混杂 | `[ ]` 待修复 |
| P1-014 | P1 | 代码规范 | `tauri/src/pages/settings/SecuritySettingsPage.tsx:15` | 组件 655 行，包含密码/生物识别/回收站/删除账户等多个模块 | `[ ]` 待修复 |
| P1-015 | P1 | 代码规范 | `tauri/src/pages/editor/ObjectEditorPage.tsx:26` | 组件 617 行，模板匹配/表单/校验/保存逻辑集中 | `[ ]` 待修复 |
| P1-016 | P1 | 代码规范 | `tauri/src/pages/settings/ExportImportPage.tsx:25` | 组件 606 行，`togglePage` 嵌套 7 层 setState 回调 | `[ ]` 待修复 |
| P1-017 | P1 | Bug/架构 | `tauri/src/pages/editor/ObjectEditorPage.tsx:144-205` | 依赖全局 `currentObject`，快速切换对象时存在竞态与旧数据残留风险 | `[ ]` 待修复 |
| P1-018 | P1 | 架构 | `tauri/src/components/object/ObjectDetailModal.tsx:140-154` | 详情弹窗同时读取/写入全局 `currentObject`，易造成状态污染 | `[ ]` 待修复 |
| P1-019 | P1 | 安全/架构 | `tauri/src/stores/pluginStore.ts:349-353` | `runningPlugins`（含日志、结果、consent 请求）被持久化到 localStorage 明文 | `[ ]` 待修复 |
| P1-020 | P1 | 错误处理 | `tauri/src/stores/settingsStore.ts:333-357` | 窗口大小同步调用 `invoke` 未 `await`，错误静默丢失 | `[ ]` 待修复 |
| P1-021 | P1 | 错误处理 | `tauri/src/stores/settingsStore.ts:149-256` | 多个 `try/catch` 空捕获，localStorage/IP C 损坏时无提示 | `[ ]` 待修复 |
| P1-022 | P1 | 死代码 | `tauri/src/pages/editor/ObjectEditorPage.tsx:493`<br>`tauri/src/pages/workspace/WorkspaceObjectCard.tsx:87` | ESLint 报告 2 条未使用变量/函数 warning | `[ ]` 待修复 |
| P1-023 | P1 | 性能 | `tauri/src-tauri/src/commands/export_import/export.rs:256,262`<br>`tauri/src-tauri/src/commands/export_import/import.rs:234-235`<br>`tauri/src-tauri/src/commands/export_import/helpers.rs:119-130` | 导出/导入 `payload` 一次性序列化/加密/读取到内存，大 Vault 可能 OOM | `[ ]` 待修复 |
| P1-024 | P1 | 性能 | `tauri/src-tauri/src/commands/export_import/export.rs:247-251`<br>`tauri/src-tauri/src/commands/export_import/import.rs:132-141`<br>`tauri/src-tauri/src/commands/attachment.rs:403-555` | 导出/导入冲突检测、附件树构建在循环中逐个 `vault.load_object(id)`，造成 N+1 查询 | `[ ]` 待修复 |
| P1-025 | P1 | 安全策略 | `tauri/src-tauri/src/plugin/registry.rs:87-95` | 远程注册表更新在 `SOLOSOUL_REGISTRY_PUBKEY` 未设置时直接返回 Ok，Release 构建可能跳过签名验证 | `[ ]` 待修复 |
| P2-001 | P2 | 代码规范 | 多处（见详细描述） | Clippy 在两个 workspace 共报告约 30+ 处 `redundant_clone`、`needless_borrow`、`map_flatten`、`collapsible_if` 等风格警告 | `[ ]` 待修复 |
| P2-002 | P2 | 代码规范 | 多处（见详细描述） | 大量函数超过 100 行，部分超过 300/600 行，影响可读性 | `[ ]` 待修复 |
| P2-003 | P2 | 安全/文档 | `tauri/src-tauri/src/lib.rs:71-73`<br>`tauri/src-tauri/src/commands/window.rs:27,41,43,59-66`<br>`tauri/src-tauri/src/commands/system.rs:25`<br>`tauri/crates/solosoul-core/src/biometric/...` | 多处 `unsafe` FFI 调用缺少 `// SAFETY:` 注释 | `[ ]` 待修复 |
| P2-004 | P2 | 配置 | `tauri/crates/solosoul-plugin/Cargo.toml:3` | crate 版本 `0.1.0` 未接入 workspace `2.5.5` | `[ ]` 待修复 |
| P2-005 | P2 | 配置 | `tauri/crates/solosoul-vault/Cargo.toml:1-6` | 缺少 `license.workspace = true` 与 `repository.workspace = true` | `[ ]` 待修复 |
| P2-006 | P2 | 文档一致性 | `tauri/Cargo.toml:16`<br>`README.md:193` | Cargo 声明 MIT，README 声明 Private/All Rights Reserved，口径冲突 | `[ ]` 待修复 |
| P2-007 | P2 | 文档一致性 | `AGENTS.md:99,128,433` | AGENTS.md 引用了已不存在的文件路径（`core/SensitivityManager`、`docs/TODO.md`、`commands/unified_object.rs`） | `[ ]` 待修复 |
| P2-008 | P2 | 文档一致性 | `README.md:155,164`<br>`AGENTS.md:22,304-309` | OCR 版本（PP-OCRv4 vs v6）、敏感度分级（3 级 vs 6 级）描述不一致 | `[ ]` 待修复 |
| P2-009 | P2 | CI/构建 | `.github/workflows/ci_cd.yml:67`<br>`.github/workflows/pr_check.yml:57,127` | `rust-cache` 的 `workspaces` 设为 `tauri/src-tauri` 而非 workspace 根 `tauri` | `[ ]` 待修复 |
| P2-010 | P2 | 安全架构 | `tauri/crates/solosoul-core/src/vault_service.rs:283-293,388-400,566-577`<br>`tauri/crates/solosoul-core/src/auth.rs:21-38` | 验证令牌使用低强度 Argon2（8 MiB / 1 iter）对已派生主密钥再次派生，建议改用 HKDF-HMAC-SHA256 | `[ ]` 待修复 |
| P2-011 | P2 | 依赖 | `tauri/Cargo.toml:56` | `ort = "2.0.0-rc.12"` 为候选版本，API/稳定性存在风险 | `[ ]` 待修复 |
| P2-012 | P2 | 潜在漏洞 | `tauri/src-tauri/src/commands/attachment.rs:311-335` | `attachment_copy_to_vault` 直接使用用户传入 `src_path` 读取任意文件，未校验源路径授权 | `[ ]` 待修复 |
| P2-013 | P2 | 潜在漏洞 | `tauri/crates/solosoul-vault/src/profile.rs:139-143` | `#[serde(untagged)]` enum 反序列化 `TypeOrEntryType`，存在歧义解析风险 | `[ ]` 待修复 |
| P2-014 | P2 | 潜在漏洞 | `tauri/crates/solosoul-core/src/ocr/macos_vision.rs:177,222` | 运行时调用 `swiftc` 编译并执行临时二进制，临时目录若被篡改可能执行恶意代码 | `[ ]` 待修复 |
| P2-015 | P2 | 死代码 | `tauri/src-tauri/src/services/llm_context.rs:16-17`<br>`tauri/crates/solosoul-sync/src/manager.rs:62-66` | `CachedPrompt.created_at` 未读取；`PeerSession` 定义后未使用 | `[ ]` 待修复 |
| P2-016 | P2 | 代码规范 | `tauri/src-tauri/src/commands/ocr.rs:560,562,578` | `langs` 未使用；两处 `vec![...]` 可用数组替代 | `[ ]` 待修复 |
| P2-017 | P2 | 架构 | `tauri/src-tauri/src/lib.rs:160` | 使用 `std::mem::forget(guard)` 故意泄漏 tracing non-blocking writer guard | `[ ]` 待修复 |
| P2-018 | P2 | 死代码/设计债 | `solosoul_cli/src/commands/backup.rs:43`<br>`tauri/src-tauri/src/commands/backup.rs:275` | `RestoreManifest` 字段被 `#[allow(dead_code)]` 屏蔽，字段未被业务逻辑消费 | `[ ]` 待修复 |
| P2-019 | P2 | 性能 | `tauri/src/pages/settings/GlobalAttachmentManager.tsx:575-582` | 统计信息对同一数据重复 4 次 reduce | `[ ]` 待修复 |
| P2-020 | P2 | 性能 | `tauri/src/pages/settings/GlobalAttachmentManager.tsx:1157`<br>`tauri/src/components/object/AttachmentViewer.tsx:677` | 附件列表全量渲染，大数量时存在渲染/内存瓶颈 | `[ ]` 待修复 |
| P2-021 | P2 | 性能 | `tauri/src/pages/workspace/ObjectWorkspacePage.tsx:502-525` | 对象卡片列表全量渲染，每个卡片触发独立计数/字段解析 | `[ ]` 待修复 |
| P2-022 | P2 | 性能 | `tauri/src/components/llm/ChatMessageList.tsx:66` | 聊天消息全量渲染，长对话累积 DOM 节点 | `[ ]` 待修复 |
| P2-023 | P2 | 性能 | `tauri/src/pages/settings/TemplateManagerPage.tsx:426-505` | 模板卡片全量渲染，内部重复 `templates.find` | `[ ]` 待修复 |
| P2-024 | P2 | 性能 | `tauri/src/components/object/ObjectDetailModal.tsx:452-597` | 字段列表未 memo，每次渲染重建 | `[ ]` 待修复 |
| P2-025 | P2 | 代码规范 | `tauri/src/pages/settings/GlobalAttachmentManager.tsx:444` | 下载路径直接字符串拼接 `dirPath + "/" + fileName`，未处理特殊字符或路径遍历 | `[ ]` 待修复 |

---

## 修复进度

- 已完成：**0 / 44**
- 当前处理：无

---

## 详细问题描述与修复指引

### P0-001 CLI 编译失败：`ObjectSummary` 缺少 `property_labels`

**位置**：`solosoul_cli/src/commands/vault_write.rs:1180`

**影响**：`solosoul_cli` 无法通过 `cargo test` / `cargo clippy --all-targets`，阻塞 CLI 相关 CI。

**复现**：
```bash
cd solosoul_cli
cargo test
```

**建议修复**：
```rust
let child_summary = ObjectSummary {
    id: child.id,
    name: child.name,
    // ... 其他字段
    property_labels: None, // 或从 child 属性中提取
};
```

---

### P0-002 Tauri Clippy 严格模式失败

**位置**：
- `tauri/src-tauri/src/commands/attachment.rs:585`：`needless_borrows_for_generic_args`
- `tauri/src-tauri/src/commands/export_import/export.rs:22`：`collapsible_if`
- `tauri/src-tauri/src/commands/export_import/export.rs:228`：`map_flatten`
- `tauri/src-tauri/src/commands/export_import/import.rs:470`：`needless_borrow`
- `tauri/src-tauri/src/commands/export_import/import.rs:473`：`needless_borrows_for_generic_args`

**影响**：PR / Push CI 的 `rust-check` 步骤失败。

**建议修复**：按 Clippy 提示简化；可尝试 `cargo clippy --fix`。

---

### P0-003 / P0-004 格式化不一致

**影响**：`cargo fmt --check` 失败，CI 阻塞。

**建议修复**：分别执行：
```bash
cd tauri && cargo fmt
cd solosoul_cli && cargo fmt
```

---

### P0-005 自动更新端点与仓库不一致

**位置**：`tauri/src-tauri/tauri.conf.json:84`、`SoloSoul-Releases/latest.json:8,12`

**影响**：Release 版本用户无法通过 updater 获取新版本，会收到 404。

**建议修复**：统一发布仓库地址；若发布仓库就是 `Gczmy/SoloSoul_code`，则同步修改 `tauri.conf.json` 与 `latest.json`。

---

### P0-006 自动更新公钥格式疑似错误

**位置**：`tauri/src-tauri/tauri.conf.json:86`

**影响**：即使端点正确，签名验证也会失败，更新被阻止。

**建议修复**：将 `pubkey` 设置为 `.pub` 文件中 `RW...` 那一行内容的 base64 编码。

---

### P0-007 / P0-008 超大单体附件组件

**位置**：
- `tauri/src/pages/settings/GlobalAttachmentManager.tsx:151`（1482 行）
- `tauri/src/components/object/AttachmentViewer.tsx:100`（1238 行）

**影响**：难以测试、review、维护；两套实现重复。

**建议修复**：
- 提取共用组件：`AttachmentTree`、`AttachmentRow`、`BatchActionBar`、`AttachmentDialogs`、`AttachmentPreview`。
- 提取共用 hook：`useAttachmentSelection`、`useAttachmentOperations`。
- 两套 UI 复用同一套逻辑层。

---

### P0-009 搜索过滤触发高频 Rust Command 调用

**位置**：`tauri/src/pages/workspace/ObjectWorkspacePage.tsx:256-300`

**影响**：每次按键都触发 `snapshot_count_batch` / `attachment_count_batch`，大对象时 IPC 抖动明显。

**建议修复**：对 `visibleObjects` 变化后的批量计数请求做 debounce（200-300ms），或改为按需/分页加载计数。

---

### P0-010 直接 DOM 样式操作

**位置**：`tauri/src/pages/workspace/ObjectWorkspacePage.tsx:137-156, 324-359`

**影响**：样式状态与 React 状态不一致，增加重排开销。

**建议修复**：使用 CSS Modules + `data-active` / className 切换；或封装可复用 `HoverButton`。

---

### P0-011 LLM 快速聊天重复实现

**位置**：`tauri/src/pages/ai/LlmChatPage/useLlmChat.ts:90`、`tauri/src/components/layout/AiQuickChatPopover.tsx:65`

**影响**：后续修改极易遗漏，导致两端行为不一致。

**建议修复**：`AiQuickChatPopover` 复用 `useLlmChat`，仅处理悬浮弹窗 UI 状态。

---

### P0-012 插件事件 JSON 解析依赖类型断言

**位置**：`tauri/src/stores/pluginStore.ts:208,218,240`

**影响**：恶意/异常插件事件可导致下游逻辑错误。

**建议修复**：所有 `JSON.parse` 结果先经过 Zod schema 校验，禁止裸 `as`。

---

### P0-013 导出大小估算 Hook 依赖缺失

**位置**：`tauri/src/hooks/useExportEstimate.ts:32-74`

**影响**：当 `totalSelected` 不变但 `scope` 变化时，估算结果不更新。

**建议修复**：将 `scope` 加入依赖数组，保留 `scopeKey` 比较以避免 Set 引用变化导致重复请求。

---

### P1-001 KDF 参数实现与文档/安全承诺不一致

**位置**：`tauri/crates/solosoul-crypto/src/kdf.rs`、`tauri/crates/solosoul-core/src/vault_service.rs`、`auth.rs`

**影响**：文档承诺默认 8 MiB / 2 iter、生产 64 MiB / 3 iter，但代码统一使用 16 MiB / 3 iter，且未实现环境变量切换。

**建议修复**：
- 在账户创建/解锁/改密处根据 `std::env::var("SOLOSOUL_SECURE")` 选择 `KdfConfig`。
- 生产参数达到 64 MiB / 3 iter / parallelism 4。
- 同步更新文档。

---

### P1-002 Windows 数据目录权限缺失

**位置**：`tauri/crates/solosoul-core/src/vault_service.rs:36-44`

**影响**：Windows 上 Vault 数据目录对其他用户/进程开放，违背隐私优先承诺。

**建议修复**：使用 `windows-sys` / `winapi` 设置目录 ACL 为当前用户独占，或引入 ` directories` + `winapi` 显式设置。

---

### P1-003 生物识别遗留文件存储硬编码密钥

**位置**：`tauri/crates/solosoul-core/src/biometric/legacy.rs:21`

**影响**：若遗留文件存储在非测试环境使用，主密钥以可预测密钥加密。

**建议修复**：删除非测试分支硬编码密钥；遗留存储仅用于测试 mock，生产强制使用平台 Keychain/Secure Enclave。

---

### P1-004 / P1-005 CI 配置缺失

**位置**：`.github/workflows/ci_cd.yml`

**影响**：master push 后缺少 Release 构建与 CLI 检查。

**建议修复**：补充 build-macos / build-windows / release Job，并加入 cli-check。

---

### P1-006 plugin_release.yml 配置错误

**位置**：`.github/workflows/plugin_release.yml`

**影响**：插件发布 workflow 可能无法触发或执行失败。

**建议修复**：
- `dtolnay/rust-action@stable` → `dtolnay/rust-toolchain@stable`
- `wasm32-wasi` → `wasm32-wasip1`
- 触发分支与默认分支对齐
- 将 CI 内生成 registry.json 改为验证一致性

---

### P1-007 / P1-008 package.json / tauri.conf.json 配置问题

**位置**：`tauri/src-tauri/tauri.conf.json:2`、`tauri/package.json:23`

**建议修复**：
- `$schema` 改为 Tauri 官方 schema。
- `@tauri-apps/cli` 移到 `devDependencies`。

---

### P1-009 插件运行时双写

**位置**：`tauri/src-tauri/src/plugin/`、`tauri/crates/solosoul-plugin/src/`

**影响**：同一逻辑双点维护，安全修复容易遗漏。

**建议修复**：以 `solosoul-plugin` crate 为唯一实现，`src-tauri` 仅做薄封装。

---

### P1-010 关键模块缺少测试

**位置**：插件安全、LLM 核心、导入导出、CLI 关键命令等。

**影响**：高危路径回归风险高。

**建议修复**：优先覆盖插件 consent、sandbox、export/import 的核心错误路径与权限边界。

---

### P1-011 thiserror 版本冲突

**位置**：`tauri/Cargo.toml:35`、`solosoul_cli/Cargo.toml:57`

**影响**：可能产生类型不匹配，CLI 集成共享 crate 时易出现编译或 trait 实现问题。

**建议修复**：CLI 统一使用 workspace 的 `thiserror = "1.0"`。

---

### P1-012 `--passWithNoTests` 掩盖测试缺失

**位置**：`tauri/package.json:13`、`sdk/js/package.json:15`

**影响**：测试文件误删或全部被跳过时 CI 仍通过。

**建议修复**：移除全局 `--passWithNoTests`；SDK 占位目录补充最小占位测试。

---

### P1-013 ~ P1-018 前端大组件与状态管理问题

**建议修复**：
- 按业务拆分子组件与 hook。
- `ObjectEditorPage` 改为基于局部 `getObject` 回填，不依赖全局 `currentObject`。
- `ObjectDetailModal` 仅使用局部 `fetchedObj`。

---

### P1-019 PluginStore 持久化敏感运行状态

**位置**：`tauri/src/stores/pluginStore.ts:349-353`

**建议修复**：`partialize` 中移除 `runningPlugins`，仅持久化安装列表；运行状态保留在内存。

---

### P1-020 / P1-021 settingsStore 错误处理缺失

**位置**：`tauri/src/stores/settingsStore.ts`

**建议修复**：
- `await` invoke 并在 catch 中记录日志 / toast。
- 区分预期异常（文件不存在）与非预期异常。

---

### P1-022 ESLint 未使用变量

**位置**：`ObjectEditorPage.tsx:493`、`WorkspaceObjectCard.tsx:87`

**建议修复**：删除未使用变量，或将 `getFieldDef` 投入使用。

---

### P1-023 / P1-024 导出/导入性能问题

**位置**：`tauri/src-tauri/src/commands/export_import/`

**建议修复**：
- payload 使用流式/分块加密写入与读取。
- 增加 `load_objects_batch` 接口，一次性批量查询对象与附件。

---

### P1-025 远程注册表更新跳过签名验证

**位置**：`tauri/src-tauri/src/plugin/registry.rs:87-95`

**建议修复**：Release 构建将公钥内嵌到二进制；未配置公钥时视为失败（fail-closed）。

---

### P2-001 ~ P2-003 Clippy 风格警告、过长函数、unsafe 注释

**建议修复**：
- 运行 `cargo clippy --fix` 批量处理风格警告。
- 将过长函数按业务步骤拆分为私有 helper。
- 为每个 `unsafe` 块补充 `// SAFETY:` 注释。

---

### P2-004 ~ P2-011 配置与文档一致性问题

**建议修复**：
- `solosoul-plugin` 接入 workspace 版本。
- `solosoul-vault` 补齐 license / repository。
- 统一 Cargo 与 README 许可证声明。
- 修正 AGENTS.md / README.md 中的过时路径与技术细节。
- `rust-cache` 的 `workspaces` 改为 `tauri`。
- 评估验证令牌改用 HKDF-SHA256。
- 评估 `ort` 升级至正式版。

---

### P2-012 ~ P2-014 潜在安全漏洞

**建议修复**：
- `attachment_copy_to_vault` 校验 `src_path` 位于授权目录内。
- `TypeOrEntryType` 改用 `#[serde(tag = ...)]` 或自定义反序列化。
- OCR macOS Vision 临时二进制做签名/哈希校验，或改为 bundled helper。

---

### P2-015 ~ P2-018 死代码与资源泄漏

**建议修复**：
- 删除未使用字段/类型，或实现对应逻辑。
- OCR `langs` 加 `_` 前缀或删除。
- `std::mem::forget(guard)` 改为意图更明确的 `Box::leak` / `OnceLock`。
- 确认 `RestoreManifest` 字段是否消费，否则删除。

---

### P2-019 ~ P2-025 前端性能与规范问题

**建议修复**：
- 合并重复 reduce 计算到单个 `useMemo`。
- 大列表引入虚拟滚动或分页。
- 对字段列表、卡片、消息使用 `React.memo` / `useMemo`。
- 路径拼接使用 Tauri `join` API。
- 统一错误消息处理，避免直接展示原始错误对象。

---

## 推荐修复顺序

1. **立即处理 P0-001 ~ P0-004**：恢复编译与 CI 基线。
2. **随后处理 P0-005 / P0-006**：修复自动更新配置，避免发布即失效。
3. **并行处理 P0-007 / P0-008 / P0-009 / P0-010 / P0-011 / P0-012 / P0-013**：前端架构与性能债务。
4. **进入 P1 阶段**：安全参数、Windows 权限、CI、插件双写、测试覆盖。
5. **最后处理 P2**：Clippy 风格警告、文档一致性、配置统一、性能优化。

---

## 附录：静态分析命令输出摘要

### Tauri `cargo clippy -- -D warnings`（失败）
```
error: needless_borrows_for_generic_args at attachment.rs:585
error: collapsible_if at export.rs:22
error: map_flatten at export.rs:228
error: needless_borrow at import.rs:470
error: needless_borrows_for_generic_args at import.rs:473
```

### Tauri `cargo fmt --check`（失败）
- `crates/solosoul-vault/src/storage.rs:2010`
- `src-tauri/src/commands/attachment.rs:566-585`
- `src-tauri/src/commands/export_import/export.rs:27,52`
- `src-tauri/src/commands/export_import/import.rs:253`
- `src-tauri/src/commands/object/mod.rs:149-508`

### CLI `cargo test`（失败）
```
error[E0063]: missing field `property_labels` in initializer of `ObjectSummary`
  --> src/commands/vault_write.rs:1180:33
```

### 前端 ESLint（警告）
```
ObjectEditorPage.tsx:493  'val' is defined but never used
WorkspaceObjectCard.tsx:87  'getFieldDef' is assigned a value but never used
```

---

*本报告生成后未执行任何代码修复，等待进一步指令。*
