# 11 — 页面迁移映射与开发优先级

> **前置阅读**：`08_前端技术架构与组件映射.md`、`10_状态管理_Zustand_Store设计.md`
> **Manifesto 对齐**：本地优先（所有页面离线可用）
> **源文档**：`tauri_refactor/页面迁移映射.md`
>
> **[警告] 术语迁移（审批通过）**：UI 中 "分区" → "集合"，"字段" → "属性"，`UnifiedObject` → `Object`。
> 详见文档 17 的术语规范。

---

## 1. 页面映射总览（23 个页面，按优先级）

| # | Flutter 页面 | React 页面 | 路由 | 优先级 | 复杂度 |
|---|-------------|-----------|------|--------|--------|
| 1 | BootstrapPage | `pages/auth/BootstrapPage.tsx` | `/bootstrap` | P0 | 低 |
| 2 | LoginPage | `pages/auth/LoginPage.tsx` | `/login` | P0 | 低 |
| 3 | HomePage | `pages/home/HomePage.tsx` | `/` | P0 | 中 |
| 4 | ObjectWorkspacePage | `pages/workspace/ObjectWorkspacePage.tsx` | `/workspace/:categoryId?` | P0 | 中 |
| 5 | ObjectEditorPage | `pages/editor/ObjectEditorPage.tsx` | `/editor/:objectId?` | P0 | 高 |
| 6 | SearchPage | `pages/search/SearchPage.tsx` | `/search` | P1 | 中 |
| 7 | SettingsPage | `pages/settings/SettingsPage.tsx` | `/settings` | P1 | 低 |
| 8 | SecuritySettingsPage | `pages/settings/SecuritySettingsPage.tsx` | `/settings/security` | P1 | 中 |
| 9 | SensitivitySettingsPage | `pages/settings/SensitivitySettingsPage.tsx` | `/settings/sensitivity` | P1 | 中 |
| 10 | DataManagementPage | `pages/settings/DataManagementPage.tsx` | `/settings/data` | P2 | 低 |
| 11 | ExportImportPage | `pages/settings/ExportImportPage.tsx` | `/settings/export-import` | P2 | 高 |
| 12 | BackupConfigPage | `pages/settings/BackupConfigPage.tsx` | `/settings/backup` | P2 | 中 |
| 13 | TrashPage | `pages/settings/TrashPage.tsx` | `/settings/trash` | P2 | 中 |
| 14 | OperationLogPage | `pages/settings/OperationLogPage.tsx` | `/settings/operation-log` | P2 | 低 |
| 15 | LlmChatPage | `pages/ai/LlmChatPage.tsx` | `/llm-chat` | P3 | 高 |
| 16 | PluginDashboardPage | `pages/ai/PluginDashboardPage.tsx` | `/plugins` | P3 | 高 |
| 17 | ScanConfigPage | `pages/ai/ScanConfigPage.tsx` | 内嵌 | P3 | 中 |
| 18 | SyncPage | `pages/sync/SyncPage.tsx` | `/sync` | P3 | 高 |
| 19 | DebugLogPage | `pages/system/DebugLogPage.tsx` | `/debug-log` | P3 | 低 |
| 20 | AboutPage | `pages/system/AboutPage.tsx` | `/about` | P3 | 低 |

---

## 2. 按优先级的开发批次

### P0（必须先实现）：认证 + 核心数据管理

| 页面 | 依赖 | 关键验收标准 |
|------|------|-------------|
| BootstrapPage | Vault IPC：`auth_bootstrap` | 可创建账户，密码强度检查，确认密码匹配 |
| LoginPage | Vault IPC：`auth_login`, `vault_list_accounts` | 账户列表选择，密码错误提示，解锁后跳转首页 |
| HomePage | Profile IPC：`profile_get` | 显示用户名称，分区摘要卡片，快速操作按钮 |
| ObjectWorkspacePage | UnifiedObject IPC：`unified_object_list` | 分区筛选，搜索框，空状态提示，加载状态 |
| ObjectEditorPage | UnifiedObject IPC：`unified_object_get`, `_create`, `_update` | 新建/编辑对象，分区编辑器，防抖保存，敏感度标签 |

### P1（重要功能）：搜索 + 设置

| 页面 | 关键验收标准 |
|------|-------------|
| SearchPage | 全局搜索跨 Profile 和 Objects，结果按相关性排序，加载态 |
| SettingsPage | 设置分组导航，点击跳转正确的子页面 |
| SecuritySettingsPage | 自动锁定时间选择，生物识别开关，修改密码对话框 |
| SensitivitySettingsPage | 见文档 12（敏感度等级系统重构） |

### P2（辅助功能）：数据管理 + 迁移

| 页面 | 关键验收标准 |
|------|-------------|
| DataManagementPage | 数据概况统计、存储空间、回收站入口 |
| ExportImportPage | 见文档 14（导入导出功能设计） |
| TrashPage | 回收站列表，恢复/永久删除，空状态，批量操作 |
| OperationLogPage | 操作日志时间线，筛选，导出（加密） |

### P3（高级功能）：AI + 同步 + 系统

| 页面 | 关键验收标准 |
|------|-------------|
| LlmChatPage | AI 对话（流式响应）、消息气泡、模型选择、token 统计 |
| PluginDashboardPage | 见文档 16（插件系统迁移设计） |
| SyncPage | 见文档 15（同步功能重构路线图） |
| DebugLogPage | 日志列表、级别筛选、导出 |
| AboutPage | 版本信息、开源许可、外部链接 |

---

## 3. 全局共享组件

| 组件 | 用途 |
|------|------|
| `PasswordVerificationDialog` | 密码验证对话框（通用，所有敏感操作复用） |
| `SensitiveValue` | 敏感数据遮罩（所有字段卡片复用） |
| `AppShell` | 应用外壳（SideNavigation 上下分区 + AppBar + Content），详见文档 09 第 9 节 |
| `GlassCard` | 玻璃卡片（所有页面复用） |

---

## 4. 网络状态处理

所有页面必须在无网络下 100% 可用：
- [正确] 数据全部来自本地 Rust 后端（SQLite/文件系统）
- [正确] 无网络请求的加载态（没有 "Loading..." 因为等待网络）
- [正确] 同步相关页面在无网络时显示"未连接到任何设备"而非报错

---

## 5. 开发顺序依赖

```
P0 页面必须先完成：
  BootstrapPage → LoginPage → HomePage → ObjectWorkspacePage → ObjectEditorPage

P0 完成后，P1/P2/P3 可并行开发：
  ├── SearchPage + SettingsPage（并行）
  ├── ExportImportPage + TrashPage（并行）
  └── LlmChatPage + PluginDashboardPage + SyncPage（并行）
```

---

## 6. 完成标准

- [ ] P0 全部 5 个页面可通过路由访问并完成核心用户旅程
- [ ] 所有页面支持加载态、空状态、错误态三种 UI 状态
- [ ] 所有页面在无网络下正常工作（数据来自本地）
- [ ] 敏感数据字段使用 `SensitiveValue` 组件渲染
- [ ] `PasswordVerificationDialog` 在所有敏感操作中复用（无重复代码）

---

*文档版本：v1.0*
*创建日期：2026-06-05*
*对应开发阶段：Phase 2-3（页面迁移）*
