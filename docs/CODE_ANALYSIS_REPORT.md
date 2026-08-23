# 代码分析修复报告（B 组新代码专项轮）

> 最后更新：2026-08-23 17:45:00
> 当前分支：`main`
> 修复轮次：1（B-01~B-06 新增代码专项审查）

## 审查范围

- B-01~B-06 全部新增/重构代码：
  - `src-tauri/src/sync/cloud_auto_sync.rs`（多轮文件重建后的完整性复核）
  - `crates/solosoul-core/src/cloud_sync/webdav.rs`（put_stream/upload_if_match 重构）
  - `commands/export_import/import.rs` + `commands/recovery.rs`（guard 参数化重构）
  - `network_status_plugin.rs` + `NetworkStatusPlugin.kt`
  - 前端 `CloudSyncPage.tsx` / `useExportImportPage.tsx`

## 基线检查

fmt/clippy/tsc/ESLint/Vitest/Rust workspace(993)/E2E(9) 全绿；函数重复定义检查通过。

## 问题清单

| ID    | 优先级 | 类别     | 文件位置                                              | 描述                                                                                     | 状态        |
|-------|--------|----------|-------------------------------------------------------|------------------------------------------------------------------------------------------|-------------|
| N-101 | P1     | 行为回归 | `src-tauri/src/commands/recovery.rs`                  | 恢复主机导入完成后丢失 `auto_sync.trigger_debounce()` 触发——原内部实现无条件调用，重构后仅 advanced 调用方补了触发 | `[ ]` 待修复 |
| N-102 | P2     | 行为偏差 | `commands/export_import/import.rs:272`                | advanced 导入的 debounce 触发移到了结果判定之前：导入失败也会触发自动同步（原为成功才触发） | `[ ]` 待修复 |
| N-003 | P2     | 性能     | `crates/solosoul-core/src/cloud_sync/webdav.rs:141-149` | `upload` 先做 `ensure_dir` 后委托 `put_stream`，而 put_stream 内部再做一次——每次上传多一轮 PROPFIND/MKCOL 网络往返（快照上传热路径） | `[x]` 已修复 |
| N-104 | P3     | i18n     | `src-tauri/src/sync/cloud_auto_sync.rs:756`           | `auto_import_one` 的导入 locale 硬编码 `"zh-CN"`，应使用系统默认 locale                    | `[ ]` 待修复 |

## 详细说明与修复方案

### N-101（P1 · 行为回归）：恢复流程丢失自动同步触发

- **原行为**：`import_execute_internal` 内部末尾无条件 `state.auto_sync.trigger_debounce()`
  → 恢复主机等全部导入路径成功后都会触发 SAF 自动同步。
- **现状**：重构后触发移至 `import_execute_advanced` 调用方，recovery.rs 未补 → 恢复完成后
  SAF 云盘回灌不再被触发。
- **方案**：recovery.rs 在 `import_result = import_execute_internal(...)` 成功后补
  `state.auto_sync.trigger_debounce();`。

### N-102（P2 · 行为偏差）：失败也触发 debounce

- **现状**：advanced 调用方在 `.await` 后立即 `trigger_debounce()` 再返回 result；
  原实现仅在成功路径触发。
- **方案**：改为 `let result = ...?; state.auto_sync.trigger_debounce(); Ok(result)`
  （与 N-101 同一提交：同一根因「触发改造」的两半）。

### N-003（P2 · 性能）：双重 ensure_dir

- **方案**：删除 `upload` 内的前置 ensure_dir 段（put_stream 已统一处理），
  upload_if_match 经 put_stream 同样受益。每轮云同步快照上传省 2 次 RTT。

### N-104（P3）：locale 硬编码

- **方案**：改用 `crate::commands::export_import::default_locale()`（与 recovery 流程一致）。

## 修复进度

- 已完成：0 / 4
- 当前处理：无
