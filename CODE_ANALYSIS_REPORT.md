# 代码分析修复报告

> 最后更新：2026-07-30 17:05:00
> 当前分支：`main`
> 修复轮次：1（初始分析）

---

## 基线检查结果摘要

| 检查项 | 状态 |
|--------|------|
| `cargo fmt --check` | ✅ 通过 |
| `cargo clippy -- -D warnings` | ✅ 通过 |
| `npx tsc --noEmit` | ✅ 通过 |
| `npm run lint` (ESLint) | ✅ 通过（0 warning） |
| `cargo test` | ✅ 通过（315+ 测试） |
| `npm run test` (Vitest) | ✅ 通过（414 测试，44 文件） |
| ACL 一致性 | ✅ 通过 |

> 基线检查已全部通过；以下为扩展静态分析与启发式扫描发现的问题清单。本次分析时工作树中尚有 `tauri/src-tauri/gen/schemas/acl-manifests.json` 未提交，将在本报告提交时一并处理。

---

## 问题清单（按优先级 P0 > P1 > P2）

| ID   | 优先级 | 类别       | 文件位置 | 描述 | 状态 |
|------|--------|------------|----------|------|------|
| P001 | P0（已缓解） | 安全 | `tauri/crates/solosoul-core/src/ocr/macos_vision.rs:207,283` | 外部可编译的临时二进制路径直接作为 `Command` 执行；已使用随机临时目录、SHA-256 哈希校验和 0o700 权限缓解，但仍建议进一步收敛 | `[ ]` 待修复 |
| P002 | P1 | 安全 | `tauri/crates/solosoul-core/src/vault_service.rs:80-84,105-108` | Windows `icacls` 通过 `format!` 拼接用户名参数，若 `%USERNAME%` 含特殊字符可能导致参数注入；路径为内部 Vault 目录但仍需防御 | `[ ]` 待修复 |
| P003 | P1 | 稳定性 | `tauri/src-tauri/src/sync/auto_sync.rs:155,172` | 同步事件源 `.unwrap()`，异常时可能导致同步任务 panic | `[ ]` 待修复 |
| P004 | P1 | 稳定性 | `tauri/src-tauri/src/sync/device_auto_sync.rs:173,192` | 同上，设备同步事件源 `.unwrap()` | `[ ]` 待修复 |
| P005 | P1 | 规范 | Rust 生产代码多处 | `unwrap()`/`expect()` 在生产代码中广泛存在，建议逐步替换为 `Result` 传播 | `[ ]` 待修复 |
| P006 | P1 | 安全/性能 | `tauri/crates/solosoul-vault/src/storage.rs:2846` | 使用 `format!` 动态拼接 IN 子句占位符，SQL 计划缓存失效且维护成本高 | `[ ]` 待修复 |
| P007 | P1 | 可维护性 | `tauri/src-tauri/src/services/llm_context.rs:254` | `build_section5_plugins()` 仍为 TODO 占位，AI 助手无法感知已安装插件 | `[ ]` 待修复 |
| P008 | P1 | 可维护性 | `tauri/crates/solosoul-core/src/biometric/mod.rs:163` | TODO：生物识别密钥当前为文件双槽存储，需迁移到 Android Keystore / iOS Keychain | `[ ]` 待修复 |
| P009 | P2 | 规范 | 全 Rust workspace | Extended Clippy (pedantic + unwrap_used/expect_used) 产生 1179+ 条 warning，测试代码占多数 | `[ ]` 待复核 |
| P010 | P2 | 规范 | `tauri/crates/solosoul-vault/src/storage.rs` / `migration.rs` | 大量内联 SQL 字符串，建议集中管理 | `[ ]` 待复核 |
| P011 | P2 | 性能 | `tauri/src/**/*.{ts,tsx}` | 208 处 `useMemo`/`useCallback` 使用，部分可能属于过早优化 | `[ ]` 待复核 |
| P012 | P2 | 安全 | 多处 `unsafe` FFI | `unsafe` 块为平台 FFI 所必需，且已有 SAFETY 注释；建议补充错误边界测试 | `[ ]` 待复核 |
| P013 | P2 | 可维护性 | `tauri/src` 导出清单 | 245+ 处导出，建议引入 `knip` 或 `cargo-machete` 自动扫描死代码 | `[ ]` 待复核 |

---

## 已识别但属设计如此 / 误报项

| ID | 类别 | 文件位置 | 说明 |
|----|------|----------|------|
| FP001 | 日志 | `tauri/src/lib/logger.ts` | `console.warn` / `console.error` 仅在 logger 工具内部使用，用于开发模式输出，非误报但属设计如此 |
| FP002 | TODO | `tauri/src-tauri/src/services/llm_context.rs:254` | 已作为 P007 记录 |
| FP003 | TODO | `tauri/crates/solosoul-core/src/biometric/mod.rs:163` | 已作为 P008 记录 |
| FP004 | 测试 unwrap | 大量 `#[cfg(test)]` 模块 | 测试代码中 `.unwrap()` / `.expect()` 属于惯用写法，可接受；生产代码仍需收敛 |

---

## 修复进度

- 已完成：0 / 13
- 当前处理：无

---

## 详细问题描述与修复指引

### P001: macOS Vision OCR 外部二进制执行风险

**位置：**
- `tauri/crates/solosoul-core/src/ocr/macos_vision.rs:207`（`Command::new("swiftc")`）
- `tauri/crates/solosoul-core/src/ocr/macos_vision.rs:283`（`Command::new(&binary_path)`）

**影响：**
- 临时目录虽随机化，但仍存在符号链接劫持或 TOCTOU 风险。
- 若缓存校验被绕过，可能执行被篡改的二进制。

**修复建议：**
1. 使用 `std::fs::canonicalize` 解析 `binary_path` 并验证其位于预期临时目录下。
2. 编译后、执行前校验二进制哈希与源码哈希一致。
3. 考虑将 `swiftc` 调用改为调用系统默认 `swiftc` 时验证其路径。

---

### P002: Windows 权限命令参数拼接

**位置：**
- `tauri/crates/solosoul-core/src/vault_service.rs:80-84`
- `tauri/crates/solosoul-core/src/vault_service.rs:105-108`

**影响：**
- `icacls` 的 `/grant` 参数通过 `format!` 拼接 `%USERNAME%`；若用户名包含特殊字符，可能改变命令语义。
- 当前代码已对用户名做白名单校验，但使用 `Command::args` 逐个参数传递更安全。

**修复建议：**
1. 将 `path_str.as_ref()` 和 `format!("{username}:F")` 作为独立 `arg` 传入。
2. 保留并强化 `sanitize_windows_username`。

---

### P003 / P004: 同步事件源 `.unwrap()`

**位置：**
- `tauri/src-tauri/src/sync/auto_sync.rs:155,172`
- `tauri/src-tauri/src/sync/device_auto_sync.rs:173,192`

**影响：**
- 事件 channel 在关闭或异常时 `unwrap()` 会导致同步任务 panic。

**修复建议：**
```rust
if let Some(source) = event { state = ...; } else { break; }
```

---

### P005: 生产代码中 `unwrap()` / `expect()` 广泛存在

**位置：**
- `tauri/src-tauri/src/**/*.rs`（约 188 处，含测试）
- `tauri/crates/**/*.rs`（约 180 处，含测试）

**说明：**
- 测试代码可保留。
- 生产代码中应优先处理：`local_embed.rs` 缓存锁、`objects.rs` 附件清理、`plugin/manager.rs` 与 `plugin/host.rs` 的 WASM 生命周期、`solosoul-vault/src/storage.rs` 数据库操作等。

**修复建议：**
1. 分模块启用 `#![deny(clippy::unwrap_used)]`（仅作用于 src，排除 tests）。
2. 用 `?` 或 `match` 替换生产路径中的 `unwrap()`/`expect()`。

---

### P006: 动态 IN 子句 SQL 拼接

**位置：**
- `tauri/crates/solosoul-vault/src/storage.rs:2846`

**代码片段：**
```rust
let sql = format!("SELECT object_id, COUNT(*) FROM object_snapshots WHERE object_id IN ({}) GROUP BY object_id", placeholders.join(","));
```

**影响：**
- 占位符列表长度可变时，每次查询生成不同 SQL 字符串，导致 SQLite 计划缓存失效。
- 虽然使用参数绑定，但拼接本身增加维护成本。

**修复建议：**
1. 使用 `rusqlite::params_from_iter` 绑定参数。
2. 若 IN 列表长度可变，考虑使用临时表或一次性参数绑定。

---

### P007: AI 助手插件上下文缺失

**位置：**
- `tauri/src-tauri/src/services/llm_context.rs:254`

**修复建议：**
1. 在 `build_static_prompt` 流程中调用 `plugin_manager.list_installed()`。
2. 将插件名称、版本、授权状态注入 prompt Section 5。
3. 仅注入已授权且非敏感的插件元数据。

---

### P008: 生物识别密钥存储迁移

**位置：**
- `tauri/crates/solosoul-core/src/biometric/mod.rs:163`

**修复建议：**
1. Android 端迁移到 `AndroidKeyStore`。
2. iOS 端迁移到 `Keychain`。
3. 提供降级兼容：读取旧文件双槽密钥并迁移到新存储。

---

### P009: Extended Clippy Warning 收敛

**位置：**
- 全 Rust workspace

**说明：**
- 1179+ 条 warning 中，pedantic 规则占大多数，unwrap_used/expect_used 次之。
- 建议分阶段修复：先处理生产代码 unwrap，再处理 pedantic style warning。

---

### P010: SQL 语句集中管理

**位置：**
- `tauri/crates/solosoul-vault/src/storage.rs`
- `tauri/crates/solosoul-vault/src/migration.rs`

**修复建议：**
1. 将高频 SQL 提取为模块级常量。
2. 使用 typed query builder 或宏减少内联 SQL。

---

### P011: React 性能复核

**位置：**
- `tauri/src` 多处（208 处 useMemo/useCallback）

**说明：**
- 大量 `useMemo`/`useCallback` 增加心智负担，部分简单计算可直接内联。
- 长列表渲染缺少虚拟滚动。

**修复建议：**
1. 对长列表引入 `react-window` 或虚拟滚动。
2. 遵循“先内联、后优化”原则。

---

### P012: `unsafe` FFI 边界审计

**位置：**
- `tauri/crates/solosoul-core/src/biometric/windows.rs`
- `tauri/crates/solosoul-core/src/biometric/macos_keychain.rs`
- `tauri/crates/solosoul-core/src/biometric/mod.rs`
- `tauri/src-tauri/src/commands/window.rs`
- `tauri/src-tauri/src/commands/system.rs`

**说明：**
- 现有 `SAFETY` 注释较完整，但缺少错误边界测试。
- macOS keychain 在 `ERR_SEC_MISSING_ENTITLEMENT` 等路径下需确认无 CF 对象泄漏。

**修复建议：**
1. 为每个 unsafe 块添加单元测试覆盖错误路径。
2. 将 FFI 操作集中到最小 unsafe 模块，外层使用纯 Rust API 包装。

---

### P013: 死代码自动扫描

**建议：**
1. 前端引入 `knip` 或 `ts-prune`。
2. Rust 引入 `cargo-machete`。
3. 先以报告模式运行，不自动删除。

---

### P014: 生成的 ACL manifest 待提交

**说明：**
- `tauri/src-tauri/gen/schemas/acl-manifests.json` 已变更，需与权限清单一起提交。

---

## 修复优先级建议

1. **P001、P002、P003、P004、P006**：涉及安全与稳定性，建议优先处理。
2. **P005、P007、P008**：涉及代码健壮性与核心功能，建议次优先。
3. **P009、P010、P011、P012、P013**：代码质量与可维护性，可分批修复/复核。

---

## 提交说明

- 本报告提交时，将同时提交工作树中已生成的 `tauri/src-tauri/gen/schemas/acl-manifests.json`，以保证权限清单与构建产物一致。

---

## 上下文说明

- 本报告生成前，近期已修复并推送的问题包括：
  - `SyncStatus` / `SyncPeer` 序列化 key 从 snake_case 修正为 camelCase。
  - `sync_generate_qr_payload` 命令 ACL 权限缺失修复。
  - 设备同步页面 QR 配对按钮改为图标按钮。
- 上述修复已合并到 `main`，因此本次初始报告未将上述问题重复列出。

---

## 后续步骤

- 按优先级逐个修复问题，每个问题单独提交。
- 修复后重新运行 `npm run check-all` 与 `cargo clippy -- -W clippy::unwrap_used -W clippy::expect_used`。
- 所有问题修复完毕后，生成最终复审报告。

---

*报告生成时间：2026-07-30 17:05:00*
*修复轮次：1（初始分析）*
