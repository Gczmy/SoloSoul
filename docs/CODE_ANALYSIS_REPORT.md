# 代码分析修复报告

> 最后更新：2026-08-22 23:45:00
> 当前分支：`main`
> 修复轮次：1（全新初始分析，未沿用旧报告；上一轮审计见 tag `code-audit-passed-20260822`）

## 分析范围与方法

- **静态分析**：`cargo fmt --check`、`cargo clippy --workspace -- -D warnings`、`npx tsc --noEmit`、`npm run lint` 全部通过。
- **深度启发式扫描**（本轮新增维度）：
  - 非测试代码 `unwrap()/expect()/panic!` 全量审查（修正测试模块排除逻辑后人工核实全部候选）；
  - 前端空 `catch(() => {})` 与 Rust `let _ =` 吞结果审查；
  - `@ts-ignore` / `as any` / 空 catch 块扫描；
  - Tauri 安全配置审查（CSP、capabilities、shell open scope、fs scope）；
  - **Zustand 敏感状态锁定清理链路审计**（对照项目 P004/P005 内存卫生策略逐 store 核查）；
  - npm audit（0 漏洞）；
  - 大列表渲染与 memo 化复查。
- **已排除的误报**：
  - `search/commands.rs:165` 的 `max_by().unwrap()` → `else` 分支仅在 `field_matches` 非空时进入，必返回 `Some`，不会 panic；
  - 其余非测试 `unwrap/expect` 命中均为编译期定长切片转换（如 `digest[..8].try_into()`）、构建脚本常量读取、平台 FFI，均安全；
  - `let _ = std::fs::remove_file(...)` 等 → 均为 best-effort 临时资源清理惯例；
  - `create_schema_tables`（162 行）/`register_core_commands`（144 行）→ 声明式 DDL / Command 注册表，行数长但结构简单，重构无收益且有风险，设计如此。

## 问题清单（按优先级 P0 > P1 > P2）

| ID    | 优先级 | 类别           | 文件位置                                                              | 描述                                                                                             | 状态        |
|-------|--------|----------------|-----------------------------------------------------------------------|--------------------------------------------------------------------------------------------------|-------------|
| A-001 | P1     | 安全·内存卫生  | `tauri/src/stores/templateStore.ts` + `tauri/src/App/AppRoutes.tsx:248` | 锁定 Vault 后模板解密数据残留内存：templateStore 无任何清理方法且未订阅 vault-locked 清理链路      | `[x]` 已修复 |
| A-002 | P2     | 安全·内存卫生  | `tauri/src/App/AppRoutes.tsx:248-263`                                  | syncStore（设备元数据：地址/指纹）、llmStatsStore（账号用量统计）未在 vault-locked 时清理          | `[ ]` 待修复 |

## 详细问题描述与修复指引

### A-001（P1 · 安全·内存卫生）：锁定后模板解密数据残留内存

- **位置**：`tauri/src/stores/templateStore.ts`（全文件无 clear/reset）；`tauri/src/App/AppRoutes.tsx:246-272`（vault-locked 处理器未调用 templateStore 清理）。
- **现象**：`templateStore.templates` 通过 `invoke('template_list')` 从加密 Vault 解密载入，包含模板名称与自定义字段定义（字段名本身可能是敏感个人信息，如「身份证号」「银行卡号」）。用户锁定 Vault 后该数组仍驻留前端内存。
- **影响**：违反项目自身已确立的内存卫生策略（AppRoutes.tsx 内 P004/P005 注释：「锁定后立即清理回收站解密摘要与搜索明文缓存，避免解密数据残留在内存」）。同一策略下 objectStore/settingsStore/trashStore/ocrScanStore/llmStore/profileStore 均已覆盖，唯独 templateStore 遗漏。
- **修复方案**：
  1. 为 `templateStore` 新增 `clearOnVaultLock` action（重置 `templates: []`、清 error/loading）；
  2. 在 `AppRoutes.tsx` 的 `vault-locked` 监听器中调用之（与其他 clearOnVaultLock 调用并列）。
- **验证**：`npx tsc --noEmit`、`npm run lint`、`npm run test`。

### A-002（P2 · 安全·内存卫生）：syncStore / llmStatsStore 未纳入锁定清理

- **位置**：`tauri/src/App/AppRoutes.tsx:246-272`。
- **现象**：`syncStore` 持有设备同步元数据（peer 名称、地址、指纹、受信状态），`llmStatsStore` 持有账号级 LLM 用量统计（已有 `clear()` 方法但未被调用）。二者均为从 Vault 解密派生的账号数据，锁定后未清理。
- **影响**：与 A-001 同源但敏感度更低（元数据而非内容），故列为 P2。
- **修复方案**：在 `vault-locked` 监听器中追加调用 `useSyncStore.getState().clearOnVaultLock()`（若 store 无该方法则新增最小实现）与 `useLlmStatsStore.getState().clear()`。
- **验证**：同 A-001。

## 修复进度

- 已完成：1 / 2
- 当前处理：无

#### 修复说明 A-001

- `templateStore.ts`：新增 `clearOnVaultLock` action（重置 `templates`/`isLoading`/`error`）。
- `AppRoutes.tsx`：在 `vault-locked` 监听器中追加调用（与其他 store 清理并列，附注释标注 A-001）。
- 验证：`npx tsc --noEmit` 通过、`npm run lint` 零警告、Vitest 108 文件 / 928 测试全绿。
