# 代码分析修复报告

> 最后更新：2026-07-29 20:05:00
> 当前分支：`main`
> 修复轮次：1（初始分析）

---

## 基线检查结果摘要

| 检查项 | 状态 |
|--------|------|
| `cargo fmt --check` | ✅ 通过（已修复 5 文件格式问题） |
| `cargo clippy -- -D warnings` | ✅ 通过 |
| `npx tsc --noEmit` | ✅ 通过 |
| `npm run lint` (ESLint) | ✅ 通过（0 warning） |
| `cargo test` | ✅ 通过 |
| `npm run test` (Vitest) | ✅ 通过（411 测试） |
| `check_acl_consistency.py` | ✅ 通过（221 命令已登记） |

> 基线阶段已修复 2 项问题：cargo fmt 格式化（5 文件）、OnboardingDialog 异步测试断言。

---

## 问题清单（按优先级 P0 > P1 > P2）

| ID   | 优先级 | 类别       | 文件位置                                                   | 描述                                                                   | 状态      |
|------|--------|------------|------------------------------------------------------------|------------------------------------------------------------------------|-----------|
| P001 | P1     | 死代码     | `tauri/src/components/layout/AppBar.tsx:11,13`            | `@deprecated` 的 `titleBarOffset` prop 已无任何调用方传值，可安全删除   | `[ ]` 待修复 |
| P002 | P1     | 代码重复   | `tauri/src-tauri/src/plugin/host/mod.rs:28` 与 `tauri/crates/solosoul-plugin/src/host.rs:28` | `mod code` 错误码常量块在两个 crate 中完全复制（10 个常量），应提取为共享定义 | `[ ]` 待修复 |
| P003 | P1     | 死代码     | `tauri/src-tauri/src/services/llm_context.rs:17`          | `CachedPrompt.created_at` 字段标记 `#[allow(dead_code)]`，从未用于 TTL 驱逐，应移除或实现过期逻辑 | `[ ]` 待修复 |
| P004 | P1     | 规范       | `tauri/src/pages/auth/LoginPage.tsx:342`                  | 直接使用 `console.error` 而非项目统一 `logger.error`，应替换为 logger 调用 | `[ ]` 待修复 |
| P005 | P2     | 代码重复   | `tauri/src-tauri/src/services/llm_context.rs:351,378`     | `type_display_name` 与 `property_key_to_label` 两个函数逻辑几乎完全相同（驼峰/下划线转标题格式），可合并为一个通用 helper | `[ ]` 待修复 |
| P006 | P2     | 死代码     | `tauri/src-tauri/src/commands/backup.rs:281-285`          | `RestoreManifest` 的 `version`、`created_at`、`profile_count` 字段标记 `#[allow(dead_code)]`，反序列化后从未读取，可精简 | `[ ]` 待修复 |
| P007 | P2     | 规范       | `tauri/src-tauri/src/services/llm_context.rs:83-85`       | `build_section5_plugins()` 硬编码返回"（暂无已安装插件）"，注释说明"intentionally omitted"，应添加 TODO 跟踪或实现 | `[ ]` 待修复 |

## 修复进度

- 已完成：0 / 7
- 当前处理：无

---

## 详细问题描述与修复指引

### P001: AppBar 废弃 prop `titleBarOffset`

**位置**：`tauri/src/components/layout/AppBar.tsx:11`

**现象**：
- `titleBarOffset?: number` 标记为 `@deprecated`，注释说明"由 sidebarPosition 推导"。
- 全仓搜索确认无任何组件传递 `titleBarOffset`（`AppShell.tsx` 仅传 `topBarHeight`，而 `topBarHeight` 在 `AppBar` 组件中也未被解构使用——见下文）。
- `titleBarOffset` 在 `AppBar` 函数参数解构中未被提取，属于纯死代码。

**影响**：接口冗余，可能误导调用方。

**修复方向**：
- 删除 `titleBarOffset?: number` prop 定义。
- 同时检查 `topBarHeight?: number`：虽然 `AppShell.tsx` 传了该 prop，但 `AppBar` 解构中未使用它（`sidebarPosition` 已用于推导布局），也应清理。

---

### P002: 插件错误码常量重复定义

**位置**：
- `tauri/src-tauri/src/plugin/host/mod.rs:28-39`
- `tauri/crates/solosoul-plugin/src/host.rs:28-39`

**现象**：
两个文件各自定义了完全相同的 `mod code` 块，包含 10 个 `i32` 常量（`SUCCESS`、`PERMISSION_DENIED`、`USER_DENIED`、`TTL_EXPIRED`、`BUFFER_TOO_SMALL`、`INVALID_FIELD`、`NETWORK_TIMEOUT`、`VAULT_LOCKED`、`RATE_LIMITED`、`NOT_IMPLEMENTED`）。

两个文件大量引用这些常量（`mod.rs` 内引用 ~20 处，`host.rs` 内引用 ~15 处）。

**影响**：
- 修改错误码时需同步两处，易遗漏导致不一致。
- 违反 DRY 原则。

**修复方向**：
- 在 `solosoul-plugin` crate 中定义公开的 `pub mod code` 常量块。
- `tauri/src-tauri/src/plugin/host/mod.rs` 中 `use solosoul_plugin::code`（或对应路径）替代本地 `mod code`。
- 注意：需确认 crate 依赖方向——`tauri` 的 `src-tauri` 已依赖 `solosoul-plugin` crate，可安全复用。

---

### P003: `CachedPrompt.created_at` 未使用的死字段

**位置**：`tauri/src-tauri/src/services/llm_context.rs:17`

**现象**：
```rust
struct CachedPrompt {
    static_prompt: String,
    #[allow(dead_code)]
    created_at: Instant,
}
```
`created_at` 在 `build_context()` 中赋值为 `Instant::now()`，但从未被读取。缓存无 TTL 驱逐逻辑——缓存仅在 `clear_cache()` 被显式调用时清空。

**影响**：
- `#[allow(dead_code)]` 掩盖了潜在的设计意图缺失。
- 缓存可能无限增长（每个 `account_id + public_data_version` 组合新增一条）。

**修复方向**：
- 方案 A（推荐）：移除 `created_at` 字段及 `Instant` import，简化结构体。
- 方案 B：实现 TTL 驱逐——在缓存命中时检查 `created_at.elapsed() > TTL`，过期则重建。但这需引入过期时间常量，改动更大。

---

### P004: LoginPage 直接使用 `console.error`

**位置**：`tauri/src/pages/auth/LoginPage.tsx:342`

**现象**：
```typescript
.catch((err) => console.error('[LoginPage] backup reminder check failed:', err));
```
项目已有统一的 `logger` 模块（`tauri/src/lib/logger.ts`），`logger.error` 始终输出到 `console.error`，且后续可统一接入后端 log_write IPC。

**影响**：违反项目日志规范（AGENTS.md 要求统一使用 logger）。

**修复方向**：
- 将 `console.error(...)` 替换为 `logger.error(...)`，并添加 `import { logger } from '@/lib/logger'`。
- 注意：`authStore.ts` 中类似位置已使用 `logger.warn`，保持一致。

---

### P005: `type_display_name` 与 `property_key_to_label` 重复逻辑

**位置**：`tauri/src-tauri/src/services/llm_context.rs:351-403`

**现象**：
两个函数都执行相同的"驼峰/下划线 → 标题格式"转换：
1. 在大写字母前插入空格
2. 将下划线替换为空格
3. 按空格分词后首字母大写

唯一差异：`type_display_name` 多了一步 `strip_prefix("__preset_")`。

**影响**：代码重复，维护时需同步修改。

**修复方向**：
- 提取通用函数 `fn to_title_case(key: &str) -> String`，包含空格插入 + 分词大写逻辑。
- `type_display_name` 调用 `to_title_case` 并前置 `strip_prefix`。
- `property_key_to_label` 直接调用 `to_title_case`。

---

### P006: `RestoreManifest` 未使用的反序列化字段

**位置**：`tauri/src-tauri/src/commands/backup.rs:281-285`

**现象**：
```rust
#[allow(dead_code)]
struct RestoreManifest {
    version: String,
    created_at: String,
    profile_count: usize,
    profiles: Vec<RestoreProfileEntry>,
}
```
反序列化后仅读取 `manifest.profiles`，`version`、`created_at`、`profile_count` 从未使用。

**影响**：`#[allow(dead_code)]` 掩盖未使用字段。

**修复方向**：
- 方案 A：移除未使用字段（`version`、`created_at`、`profile_count`），仅保留 `profiles`。serde 反序列化会自动忽略 JSON 中多余的字段。
- 方案 B：在 `profile_count` 与 `profiles.len()` 不一致时添加校验告警。但这属于增强而非修复。

---

### P007: `build_section5_plugins()` 硬编码占位

**位置**：`tauri/src-tauri/src/services/llm_context.rs:83-85`

**现象**：
```rust
fn build_section5_plugins() -> String {
    // Plugin context is intentionally omitted until the installed plugin list
    // is exposed to the LLM context service.
    "（暂无已安装插件）".to_string()
}
```
该函数始终返回固定占位文本。注释说明是"intentionally omitted"，但缺少 TODO/FIXME 标记来跟踪后续实现。

**影响**：LLM 系统提示词中插件信息始终缺失，降低了 AI 助手对用户环境的感知能力。

**修复方向**：
- 添加 `// TODO: 查询已安装插件列表并注入 Section 5` 标记，便于后续跟踪。
- 或实现插件列表查询（需接入 `plugin_list_installed` 逻辑），但这属于功能增强，超出代码审查范围。

---

*报告生成时间：2026-07-29 20:05:00*
