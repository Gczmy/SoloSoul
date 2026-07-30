# 代码分析修复报告

> 最后更新：2026-07-31 12:00:00
> 当前分支：`main`
> 修复轮次：2（P008 iOS Keychain + Android 清理）

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

> 基线检查已全部通过；本轮针对扩展静态分析中 P0/P1 级别可修复项做了定向修复，剩余 P1/P2 项评估后决定延后处理。

---

## 问题清单（按优先级 P0 > P1 > P2）

| ID   | 优先级 | 类别       | 文件位置 | 描述 | 状态 |
|------|--------|------------|----------|------|------|
| P001 | P0（已缓解） | 安全 | `tauri/crates/solosoul-core/src/ocr/macos_vision.rs` | 外部可编译的临时二进制路径直接作为 `Command` 执行；已加固为确定性私有缓存目录、每次执行前 SHA-256 哈希校验、0o700/0o600 权限 | `[x]` 已修复 |
| P002 | P1 | 安全 | `tauri/crates/solosoul-core/src/vault_service.rs:80-84,105-108` | Windows `icacls` 通过 `format!` 拼接用户名参数，若 `%USERNAME%` 含特殊字符可能导致参数注入 | `[x]` 已修复 |
| P003 | P1 | 稳定性 | `tauri/src-tauri/src/sync/auto_sync.rs:155,172` | 同步事件源 `.unwrap()`，异常时可能导致同步任务 panic | `[x]` 已修复 |
| P004 | P1 | 稳定性 | `tauri/src-tauri/src/sync/device_auto_sync.rs:173,192` | 设备同步事件源 `.unwrap()` | `[x]` 已修复 |
| P005 | P1 | 规范 | `tauri/crates/solosoul-vault/src/storage.rs` | 分批修复 16+ 处静默吞没的 `unwrap_or_default()`/`.ok()` — SQL query_row、JSON 序列化/反序列化、字段解密路径 | `[x]` 已修复 |
| P006 | P1 | 安全/性能 | `tauri/crates/solosoul-vault/src/storage.rs:2846` | 使用 `format!` 动态拼接 IN 子句占位符；已改为 `repeat_n("?")` + `params_from_iter` 参数绑定 | `[x]` 已修复 |
| P007 | P1 | 可维护性 | `tauri/src-tauri/src/services/llm_context.rs:254` | `build_section5_plugins()` 仍为 TODO 占位，AI 助手无法感知已安装插件 | `[x]` 已修复 |
| P008 | P1 | 可维护性 | `tauri/crates/solosoul-core/src/biometric/ios.rs` | iOS Keychain 迁移完成（`IosBiometricStorage`）；Android `platform_storage` 改为 `StubBiometricStorage`（因 Android 实际使用 KeystorePluginHandle）；macOS Keychain 暂缓（需 Apple Developer Program） | `[x]` 已完成 |
| P009 | P2 | 规范 | 全 Rust workspace | Extended Clippy (pedantic + unwrap_used/expect_used) 产生 1179+ 条 warning，测试代码占多数 | `[ ]` 延后 |
| P010 | P2 | 规范 | `tauri/crates/solosoul-vault/src/storage.rs` | 高频内联 SQL（objects 表 22 列列表重复 4 次）提取为模块常量 `OBJECT_COLUMNS`，消除重复 | `[x]` 已修复 |
| P011 | P2 | 性能 | `tauri/src/**/*.{ts,tsx}` | 208 处 `useMemo`/`useCallback` 使用，部分可能属于过早优化 | `[ ]` 延后 |
| P012 | P2 | 安全 | 多处 `unsafe` FFI | `unsafe` 块为平台 FFI 所必需，且已有 SAFETY 注释；建议补充错误边界测试 | `[ ]` 延后 |
| P013 | P2 | 可维护性 | `tauri/src` 导出清单 | 245+ 处导出，建议引入 `knip` 或 `cargo-machete` 自动扫描死代码 | `[ ]` 延后 |

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

- 已完成：9 / 13（P001、P002、P003、P004、P005、P006、P007、P008、P010）
- 延后处理：4 / 13（P009、P011、P012、P013）
- 当前处理：无

---

## 本轮修复详情

### P001: macOS Vision OCR 外部二进制执行风险

**修复内容：**
- 将随机 `tempfile::TempDir`（通过 `std::mem::forget` 泄漏）替换为确定性系统缓存目录 `~/Library/Caches/com.solosoul.app/vision_cli`。
- 缓存目录权限强制 `0o700`，Swift 源码 `0o600`，编译产物 `0o700`。
- 仅在源码内容发生变化时才重新写入源文件，避免无意义地触发重新编译。
- 编译后计算并持久化 SHA-256 哈希；每次返回二进制路径前强制重新校验哈希，防止 TOCTOU 篡改。
- 测试环境使用每个测试进程独立的临时目录（`#[cfg(test)]`），避免污染用户缓存并解决沙箱/链接器限制。

**相关文件：**
- `tauri/crates/solosoul-core/src/ocr/macos_vision.rs`
- `tauri/crates/solosoul-core/Cargo.toml`（新增 `dirs` 依赖）

---

### P002: Windows 权限命令参数拼接

**修复内容：**
- 将 `icacls` 命令的构建从 `.args([...])` 改为链式 `.arg(...)`，使路径、`/grant`、授权字符串各自作为独立参数传入。
- 保留 `sanitize_windows_username` 白名单校验，避免用户名中的特殊字符被 shell/命令解析器误解。

**相关文件：**
- `tauri/crates/solosoul-core/src/vault_service.rs`

---

### P003 / P004: 同步事件源 `.unwrap()`

**修复内容：**
- 在 `auto_sync.rs` 与 `device_auto_sync.rs` 的事件接收循环中，将 `event.unwrap().source()` 重构为：
  ```rust
  event = rx.recv() => match event {
      Some(ref inner) => match *inner {
          SyncEvent::Immediate | SyncEvent::Background => {
              state = AutoSyncState::Running(inner.source());
          }
          SyncEvent::Debounce => { ... }
      },
      None => break,
  }
  ```
- 因事件枚举均为 unit variant，`match *inner` 不会移动任何数据，调用 `inner.source()` 仍然合法。

**相关文件：**
- `tauri/src-tauri/src/sync/auto_sync.rs`
- `tauri/src-tauri/src/sync/device_auto_sync.rs`

---

### P006: 动态 IN 子句 SQL 拼接

**修复内容：**
- 移除索引化占位符 `?1, ?2, ...` 的手动构造。
- 使用 `std::iter::repeat_n("?", n).collect::<Vec<_>>().join(",")` 生成占位符列表。
- 使用 `rusqlite::params_from_iter(object_ids.iter())` 绑定参数，消除字符串拼接注入风险并提升可读性。

**相关文件：**
- `tauri/crates/solosoul-vault/src/storage.rs`

---

### P007: AI 助手插件上下文注入

**修复内容：**
- `build_context()` 新增 `plugins: &[PluginManifest]` 参数，在 `unified_chat.rs` 调用方从 `PluginManager::list_installed()` 获取已安装插件列表并传入。
- `build_section5_plugins()` 不再是 TODO 占位，实际格式化每个已安装插件的名称、版本、描述和可选的作者信息。
- 若插件列表为空仍返回 `（暂无已安装插件）`，保持向后兼容。
- 插件列表获取失败时静默降级为空列表，不阻塞对话。

**相关文件：**
- `tauri/src-tauri/src/services/llm_context.rs`
- `tauri/src-tauri/src/commands/llm/unified_chat.rs`

---

### P010: 高频内联 SQL 提取为模块常量

**修复内容：**
- `objects` 表的 22 列字段列表在 `storage.rs` 中重复 4 处（`list_object_changes_since`、`load_object`、`load_objects_batch`、`search_objects`），每处长约 300 字节。
- 提取为模块级常量 `OBJECT_COLUMNS`，统一引用的同时也提升了未来 schema 变更时的可维护性（只需修改常量定义一处）。

**相关文件：**
- `tauri/crates/solosoul-vault/src/storage.rs`

---

## 延后处理项说明

### P005: 生产代码中 `unwrap()` / `expect()` 广泛存在（已修复）

**分两批修复 `storage.rs` 中 16+ 处静默吞没错误模式：**

#### 第一批：`query_row` 的 `.ok()` 错误遮蔽（4 处）
- `load_profile()`、`load_object()`、`get_trash_item()`、`load_snapshot()` 中 `rusqlite::Error` 被 `.ok()` 静默吞没，数据库错误被掩盖为"记录不存在"。
- 全部改为 `.optional().map_err(|e| format!("...: {}", e))?;`，保留 `None` 语义的同时向上传播真实错误。

#### 第二批：JSON 序列化/反序列化与解密路径的静默吞没（12+ 处）
- **`save_object()`**（4 处）：`serde_json::to_string` 对 `children_ids`、`properties`、`property_labels`、`tags_json` 的序列化失败被 `.unwrap_or_default()` 静默吞没，导致空数据写入 DB → 改为 `map_err(...)?` 传播。
- **同步函数**（3 处）：`list_object_changes_since` / `list_user_template_changes_since` / `list_trash_changes_since` 中 `serde_json::to_value` 序列化失败被静默吞没 → 传播。
- **`get_sync_record_data()`**（4 处）：4 个 match 分支中 `to_value` 失败 → 传播。
- **`list_object_attachment_ids()`**（1 处）：`serde_json::from_str` 反序列化失败 → 传播。
- **`list_objects()`**（5+ 处）：`decrypt_text_field` 解密失败及后续 `serde_json::from_str` 反序列化失败全部从 `.unwrap_or_default()` 改为传播 `rusqlite::Error::FromSqlConversionFailure`。

**相关文件：** `tauri/crates/solosoul-vault/src/storage.rs`

### P007: AI 助手插件上下文缺失（已修复）
- **修复内容：**
  - `build_context()` 新增 `plugins: &[PluginManifest]` 参数，在 `unified_chat.rs` 调用方从 `PluginManager::list_installed()` 获取已安装插件列表并传入。
  - `build_section5_plugins()` 不再是 TODO 占位，实际格式化每个已安装插件的名称、版本、描述和可选的作者信息。
  - 若插件列表为空仍返回 `（暂无已安装插件）`，保持向后兼容。
  - 插件列表获取失败时静默降级为空列表，不阻塞对话。
- **相关文件：** `tauri/src-tauri/src/services/llm_context.rs`、`tauri/src-tauri/src/commands/llm/unified_chat.rs`

### P008: 生物识别密钥迁移（已完成）

**iOS Keychain 迁移：**
- 新建 `crates/solosoul-core/src/biometric/ios.rs`，实现 `IosBiometricStorage`（基于 iOS Security.framework Keychain + `kSecAccessControlUserPresence`）。
- 更新 `mod.rs` 中 `platform_storage()` 返回 `IosBiometricStorage`。
- 更新 `commands/biometric.rs` 中 3 处 iOS 分支从 `FileBiometricStorage` → `IosBiometricStorage`。
- 在 `Cargo.toml` 中为 iOS 添加 `security-framework`、`core-foundation` 等依赖。
- 安全提升：文件权限 0o600 → 硬件 Keychain + `kSecAccessControlUserPresence`（Face ID / Touch ID）。

**Android platform_storage 清理：**
- Android 的 `platform_storage()` 从 `FileBiometricStorage` 改为 `StubBiometricStorage`。
- Android 实际凭证操作由 `commands/biometric.rs` 中的 `KeystorePluginHandle` 处理，不经过 `BiometricManager`。

**macOS Keychain：** 暂缓（需 Apple Developer Program 99 USD/年）。

### P009–P013: 扩展 Clippy、SQL 集中化、React 性能、unsafe FFI、死代码扫描
- **原因：** 均为代码质量与可维护性项，不引入安全或稳定性风险。Extended Clippy 1179+ 条 warning 中测试代码占多数，需要分阶段收敛；SQL 集中化、React useMemo 审计、unsafe FFI 测试补充、死代码扫描均需单独投入。
- **建议后续动作：**
  - P009：在 CI 中先以 `--no-fail` 模式生成 warning 趋势报告，再分批修复。
  - P010：将高频 SQL 提取到模块常量，逐步替换内联字符串。
  - P011：对长列表引入虚拟滚动，对简单计算优先内联。
  - P012：为每个 unsafe 块补充错误路径单元测试。
  - P013：引入 `knip`/`cargo-machete` 以报告模式运行，确认死代码后再删除。

---

## 验证结果

本轮修复后重新执行：

```bash
cd tauri
cargo fmt --check        # ✅ 通过
cargo clippy -- -D warnings # ✅ 通过
npm run test             # ✅ 414 tests passed
cargo test               # ✅ all passed
```

> 注：`tauri/src-tauri/gen/schemas/acl-manifests.json` 在本次修复中未发生变化，无需提交。

---

## 提交说明

- 本次提交包含上述 P001/P002/P003/P004/P006 修复及相关报告更新。

---

## 后续步骤

1. **P009 CI 中建立 Extended Clippy 趋势报告**：以 `--no-fail` 模式运行，生成 warning 趋势。
2. **P011–P013 代码质量项**：分阶段在后续迭代中处理。
3. **macOS Keychain（P008 遗留）**：等待 Apple Developer Program 付费后激活 `macos_keychain.rs`。

---

*报告生成时间：2026-07-31 01:00:00*
*修复轮次：2*
