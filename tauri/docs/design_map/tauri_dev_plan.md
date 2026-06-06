# Tauri 客户端开发审计与未完成任务清单

> **审计日期:** 2026-06-06
> **最后更新:** 2026-06-06 (P0-2~P1-4 已完成)
> **审计范围:** `tauri/src-tauri/src/` (Rust 后端) + `tauri/src/` (TypeScript 前端)
> **审计方法:** 逐文件代码审查，识别桩代码、硬编码、未使用参数、TODO 标记、占位页面

---

## 一、架构总览与核心设计债

### 1.1 根本架构问题：Profile = Object = Settings

整个 Tauri 后端建立在 `solosoul_vault::Profile` 作为唯一存储单元之上。没有独立的 Object 存储层——对象、设置偏好、配置文件数据全部存储为 Profile。

```
当前实际架构:
  vault
    └── profiles/                    ← 唯一的存储类型
          ├── acc_xxx (账户profile)   ← 存储 preferences (settings)
          ├── __page_profile           ← 存储为 profile
          └── <custom-page-id>         ← 存储为 profile

修复后架构 (P0-1 完成):
  vault
    ├── profiles/                    ← settings + 页面 section 数据
    │     ├── acc_xxx (preferences)
    │     └── __page_* (页面 section 数据)
    └── objects/                     ← 独立的统一对象存储 (新增)
          ├── type_id 分类 (page/collection/note/task)
          ├── parent_id/children_ids 层级关系
          ├── properties JSON (灵活 schema)
          ├── 软删除 (is_deleted + deleted_at)
          └── SQL LIKE 全文搜索
```

**影响范围：** 所有 object_*、search_*、profile_*、settings 命令均受此架构约束。

---

## 二、Rust 后端桩代码清单

### 2.1 object.rs — 对象 CRUD（严重桩）

| 命令 | 状态 | 问题描述 |
|------|------|---------|
| `object_list` | **桩** | 参数 `_account_id` 和 `_filter` 完全未使用（`_` 前缀）。忽略 `collection_type` 过滤、`sensitivity_level` 过滤、`keyword` 搜索。直接 `list_profiles()` 返回所有 profile 作为 object，`collection_type` 硬编码为 `"profile"` |
| `object_get` | **桩** | 通过 `load_profile(&object_id)` 加载，仅支持 profile 类型。无法读取真正的 Object 结构 |
| `object_create` | **桩** | 调用 `Profile::new_with_id` 创建 profile，`properties` 被序列化为 profile.data。没有 ObjectType 验证、没有 property schema 校验、没有 parent/children 关系 |
| `object_update` | **桩** | 直接修改 profile 的 name 和 data。没有字段级更新、没有版本控制、没有冲突检测 |
| `object_delete` | **桩** | 直接 `delete_profile`。没有软删除、没有回收站、没有级联删除子对象 |

**Severity:** CRITICAL — 导致「创建页面后出现 acc_xxx 对象」Bug 的直接原因

### 2.2 search.rs — 搜索（严重桩）

| 命令 | 状态 | 问题描述 |
|------|------|---------|
| `search_unified` | **桩** | 参数 `_account_id` 完全忽略。仅搜索 profile name 字符串匹配，不搜索实际对象数据。relevance 评分简陋（匹配=2.0, 不匹配=0.0） |
| `search_advanced` | **桩** | 参数 `_account_id` 和 `_sensitivity_level` 完全忽略。`collection_type` 过滤仅对 profile name 做简单包含匹配。无全文索引、无语义搜索、无属性级匹配 |

**Severity:** CRITICAL — 搜索结果仅包含 profile 名称，无实际内容搜索能力

### 2.3 sync.rs — 设备同步（完全桩）

| 命令 | 状态 | 问题描述 |
|------|------|---------|
| `sync_discover` | **完全桩** | 注释写明 "full mDNS integration requires running a background discovery service"。始终返回空 peer 列表 |
| `sync_get_status` | **完全桩** | 始终返回 `is_discovering: false, connected_peers: [], sync_enabled: false` |
| `sync_enable` | **完全桩** | 参数 `_enable` 未被使用。注释 `TODO: Start/stop background sync daemon`。函数体仅 `Ok(())` |
| `sync_with_device` | **完全桩** | 参数 `_device_id` 未被使用。注释 `TODO: Initiate Noise handshake and CRDT sync`。硬编码返回 `"sync_initiated"` |

**Severity:** CRITICAL（功能不可用）

### 2.4 sensitivity.rs — 敏感级别（部分桩）

| 命令 | 状态 | 问题描述 |
|------|------|---------|
| `sensitivity_update_field` | **半桩** | 密码验证硬编码 `password != "debug"`，注释写明 "real impl would verify against vault"。降级保护逻辑正确，但实际密码校验是假的 |

**Severity:** HIGH — 安全隐患。任何人都可以用密码 "debug" 修改敏感级别

### 2.5 system.rs — 系统信息（半桩）

| 命令 | 状态 | 问题描述 |
|------|------|---------|
| `check_version` | **半桩** | `latestVersion: null, hasUpdate: false` 硬编码，从不检查真实的更新服务器 |

**Severity:** MINOR — 功能不影响核心使用

### 2.6 discovery.rs — mDNS 发现（已完成但有历史问题）

| 命令 | 状态 | 问题描述 |
|------|------|---------|
| `mdns_discover` | **完成** | 功能完整，使用 mdns_sd crate 浏览 `_solosoul._tcp.local.` |
| `mdns_advertise` | **完成** | 功能完整。Git 历史显示曾修复重复注册问题（`580993a`） |

**Severity:** OK — 无待处理问题

### 2.7 已完成的模块

| 模块 | 状态 | 备注 |
|------|------|------|
| `auth.rs` | 完成 | 账户创建/登录/登出/查询 |
| `vault.rs` | 完成 | Vault lock/unlock/change_password/delete/list |
| `crypto.rs` | 完成 | AES-GCM 加解密、Argon2id 密钥派生、安全比较 |
| `fs.rs` | 完成 | 文件加密/解密、ZIP 打包/解包、备份检查 |
| `profile.rs` | 完成 | Profile CRUD + section 读写 |
| `backup.rs` | 完成 | 备份创建/列表/恢复/删除 |
| `export_import.rs` | 完成 | .solosoul 文件导出/导入 |
| `log.rs` | 完成 | 操作日志读写/导出 |
| `settings.rs` | 完成 | 偏好设置读写（已修复 profile 自动创建） |

---

## 三、TypeScript 前端占位页面清单

### 3.1 P3 占位页面（仅展示"开发中"卡片）

| 页面 | 路由 | 代码量 | 状态 |
|------|------|--------|------|
| `LlmChatPage.tsx` | `/llm-chat` | 26 行 | **P3 线框** — 显示 "AI features are under development" |
| `PluginDashboardPage.tsx` | `/plugins` | 26 行 | **P3 线框** — 显示 "Plugin system is under development" |
| `SyncPage.tsx` | `/sync` | 26 行 | **P3 线框** — 显示 "Device sync is under development" |
| `TrashPage.tsx` | `/settings/trash` | 28 行 | **空页面** — 仅显示 "trash_empty" 消息，无列出/恢复/永久删除功能 |

### 3.2 功能不完整页面

| 页面 | 缺失功能 | Severity |
|------|---------|----------|
| `ObjectWorkspacePage.tsx` | 使用 `object_list` (桩) → 显示所有 profile（包括 acc_xxx 账户 profile），不区分 object 类型 | HIGH |
| `ObjectEditorPage.tsx` | 编辑已有对象时不从 store 加载当前数据；仅支持 3 种硬编码模板 (passport/bank/identity)；无自定义字段 | MAJOR |
| `SearchPage.tsx` | 依赖 `search_unified` (桩) → 仅搜索 profile name | MAJOR |
| `HomePage.tsx` | 快速入口卡片链接到 workspace section filter，但 workspace 显示的是 profile 而非真正的 section 数据 | MAJOR |
| `SecuritySettingsPage.tsx` | 两处 TODO: "update password_hint via separate IPC after backend support" 和 "call dedicated update_hint IPC once backend supports it" | MINOR |
| `SensitivitySettingsPage.tsx` | 密码验证提交后无明显反馈；reason 字段可选 | MINOR |
| `ExportImportPage.tsx` | 导入/导出功能依赖 profile 列表（`export_get_scope_tree` 返回 profiles） | MINOR |
| `AppearanceSettingsPage.tsx` | 功能基本完整 | OK |
| `DataManagementPage.tsx` | 功能基本完整（链接到各设置页） | OK |
| `LoginPage.tsx` / `BootstrapPage.tsx` | 功能完整 | OK |
| `AboutPage.tsx` | 功能完整 | OK |
| `DebugLogPage.tsx` | 功能完整 | OK |
| `OperationLogPage.tsx` | 功能完整 | OK |
| `BackupConfigPage.tsx` | 功能完整 | OK |

---

## 四、未完成任务按优先级排序

### P0 — 阻塞发布（数据完整性/安全）

| # | 任务 | 涉及文件 | 工作量估计 | 状态 |
|---|------|---------|-----------|------|
| P0-1 | **实现真正的 Object 存储层** — 将 object CRUD 与 profile 分离，支持 ObjectType、property schema、parent/children 关系 | `commands/object.rs`, `solosoul_vault` | 3-5 天 | ✅ 已完成 |
| P0-2 | **修复 object_list 过滤逻辑** — 实现 collection_type/sensitivity_level/keyword 过滤，排除系统内部 profile（`__page_*`、`acc_*`） | `commands/object.rs` | 0.5 天 | ✅ 已完成 |
| P0-3 | **修复搜索功能** — 搜索实际对象数据而不仅是 profile name，支持全文检索和属性匹配 | `commands/search.rs` | 1-2 天 | ✅ 已完成 |
| P0-4 | **修复 sensitivity 密码验证** — 将硬编码 `"debug"` 替换为真实的 vault 密码校验 | `commands/sensitivity.rs` | 0.5 天 | ✅ 已完成 |

### P1 — 核心功能缺失

| # | 任务 | 涉及文件 | 工作量估计 | 状态 |
|---|------|---------|-----------|------|
| P1-1 | **实现回收站功能** — 软删除、回收站列表、恢复、永久删除、自动清理 | `TrashPage.tsx`, `commands/object.rs` | 1-2 天 | ✅ 已完成 |
| P1-2 | **完善对象编辑器** — 加载已有对象、动态字段、自定义属性类型、敏感字段处理 | `ObjectEditorPage.tsx` | 1-2 天 | ⏳ 待实施 |
| P1-3 | **实现设备同步** — Noise 握手、CRDT 同步协议、冲突解决 | `commands/sync.rs`, `SyncPage.tsx` | 5-10 天 | ⏳ 待实施 |
| P1-4 | **首页快速入口对接真实数据** — HomePage 卡片显示实际 section 数据而非 profile | `HomePage.tsx`, `ObjectWorkspacePage.tsx` | 1 天 | ✅ 已完成 |

### P2 — 次要功能

| # | 任务 | 涉及文件 | 工作量估计 |
|---|------|---------|-----------|
| P2-1 | **AI 对话页面** — LLM 聊天集成、会话管理 | `LlmChatPage.tsx` | 3-5 天 |
| P2-2 | **插件系统** — 插件加载/卸载/管理界面 | `PluginDashboardPage.tsx` | 5-10 天 |
| P2-3 | **密码提示后端支持** — 实现 `update_hint` IPC + 前端 UI | `SecuritySettingsPage.tsx`, `commands/auth.rs` | 0.5 天 |
| P2-4 | **版本检查** — 连接更新服务器检查最新版本 | `commands/system.rs` | 0.5 天 |

### P3 — 增强/优化

| # | 任务 | 涉及文件 | 工作量估计 |
|---|------|---------|-----------|
| P3-1 | **操作日志 UI 改进** — 高级过滤、导出格式选择 | `OperationLogPage.tsx` | 1 天 |
| P3-2 | **数据导入/导出 UX** — 进度条、选择性导出、预览 | `ExportImportPage.tsx` | 1 天 |
| P3-3 | **搜索 UX 改进** — 高级搜索面板、结果高亮、搜索历史 | `SearchPage.tsx` | 1 天 |

---

## 五、代码质量备注

### 5.1 已知 Bug（本次审计中修复）

| Bug | 文件 | 修复状态 |
|-----|------|---------|
| 侧边栏新建页面选中错误 | `SideNavigation.tsx` | ✅ 已修复 |
| 侧边栏多页面溢出无滚动条 | `SideNavigation.module.css` | ✅ 已修复 |
| 悬停名称卡片被 overflow 裁剪 | `SideNavigation.tsx` + `.css` | ✅ 已修复 |
| 自定义页面 lock/login 后消失 | `settings.rs` + `settingsStore.ts` | ✅ 已修复 |
| 加号按钮缺少悬停卡片 | `SideNavigation.tsx` | ✅ 已修复 |

### 5.2 图标系统

| 模块 | 状态 |
|------|------|
| `lib/pageIcons.ts` (SSOT) | ✅ 已完成 — `PAGE_ICON_MAP` + `CUSTOM_ICON_MAP` |
| 侧边栏图标迁移 | ✅ 已完成 |
| 主页图标迁移 | ✅ 已完成 |
| 工作区图标迁移 | ✅ 已完成 |
| 搜索页图标迁移 | ✅ 已完成 |
| 自定义页面图标选择器 | ✅ 已完成 |

### 5.3 未使用的 Rust 文件属性

- `commands/mod.rs:1` — `#![allow(unused_imports)]` 表明存在未使用的导入
- `commands/crypto.rs:1` — `#![allow(unused_variables)]` 表明该文件曾有计划但实际已实现

---

## 六、文件大小统计

| 类别 | 文件数 | 总行数 |
|------|--------|--------|
| Rust 命令模块 | 15 | ~1,750 |
| TypeScript 页面 | 20 | ~1,800 |
| TypeScript Stores | 8 | ~570 |
| TypeScript 组件/Hooks/Lib | 10 | ~600 |
| CSS 模块 | 1 | ~230 |
| **合计** | **54** | **~4,950** |

---

## 七、已完成修复详情 (2026-06-06)

### P0-1: Object 存储层实现

**架构变更：** 新增独立的 `objects` 表，与 `profiles` 表完全分离。

**`crates/solosoul-vault/src/storage.rs`:**
- 新增 `objects` SQLite 表（16 列 + 4 个索引：account_id, parent_id, type_id, is_deleted）
- 表字段：id, account_id, type_id, name, icon_name, parent_id, children_ids (JSON), properties (JSON), property_labels (JSON), sensitivity_level, is_deleted, deleted_at, created_at, updated_at, version
- 新增 6 个 CRUD 方法：`save_object`, `load_object`, `list_objects`, `delete_object`（支持软/硬删除）, `restore_object`, `search_objects`

**`crates/solosoul-vault/src/lib.rs`:**
- 新增 `ObjectRecord` struct — 完整的对象数据模型（含 parent/children 关系、properties JSON、软删除标记）
- 新增 `ObjectSummary` struct — 轻量列表摘要

**`commands/object.rs`:** 完全重写，全部命令使用新的 object 存储层：
- `object_list` → `vault.list_objects(account_id, type_id, parent_id, include_deleted)`
- `object_get` → `vault.load_object(id)`
- `object_create` → 创建 `ObjectRecord` + 自动更新父对象 children_ids
- `object_update` → 原地更新 properties/name/sensitivity
- `object_delete` → `vault.delete_object(id, soft=true)`
- `object_trash_list` → `vault.list_objects(..., include_deleted=true)` + 过滤 is_deleted
- `object_restore` → `vault.restore_object(id)`
- `object_purge` → `vault.delete_object(id, soft=false)`

**`CreateObjectInput` 新增字段：** `parentId`, `iconName`（可选）

**`commands/search.rs`:** 改用 `vault.search_objects()` — SQL LIKE 搜索 name + properties

**向后兼容：** 所有 13 个 vault 单元测试通过。profiles 表未修改。

### P0-2: object_list 过滤修复
- `commands/object.rs`: 重写 `object_list`，新增 `classify_profile` 逻辑
  - 排除 `acc_*` 系统 profiles（settings 容器）
  - 排除含 `preferences` 键的非页面 profiles
  - 从 profile JSON 中读取真正的 `collectionType`/`typeId`
  - 支持 `collection_type`、`keyword` 过滤
  - 支持软删除标记过滤（`__deleted`）

### P0-4: sensitivity 密码验证
- `commands/sensitivity.rs`: 新增 `verify_vault_password` 函数
  - 使用 Argon2id KDF（与 `vault.unlock()` 相同的算法和参数）
  - 遍历所有账户验证密码（salt + verify_hash）
  - 移除硬编码 `"debug"` 密码

### P0-3: 搜索功能增强
- `commands/search.rs`: 完全重写 `search_advanced` 和 `search_unified`
  - 新增 `search_data_for_field_matches` — 递归搜索 JSON 数据
  - 字段级匹配：返回 `matched_field` 和 `matched_value`
  - 分层 relevance 评分（名称=2.0，字段名匹配=2.5，值精确匹配=5.0）
  - 过滤系统 profiles（同 P0-2 逻辑）
  - `search_unified` 委托给 `search_advanced`

### P1-1: 回收站功能
- `commands/object.rs`:
  - `object_delete` → 软删除（设置 `__deleted: true, __deletedAt`）
  - 新增 `object_trash_list` — 列出所有软删除对象
  - 新增 `object_restore` — 恢复（移除 `__deleted` 标记）
  - 新增 `object_purge` — 物理删除
- `objectStore.ts`: 新增 `trashObjects`、`loadTrashObjects`、`restoreObject`、`purgeObject`
- `TrashPage.tsx`: 完整的回收站 UI — 列表、恢复按钮、永久删除按钮
- `lib.rs`: 注册 3 个新命令
- i18n: 新增 `common:delete_permanently` (zh-CN + en-US)

### P1-4: 工作区显示优化
- `ObjectWorkspacePage.tsx`:
  - 过滤 `collectionType === 'page'` 的对象（从侧边栏导航）
  - 显示 `collectionType` 标签徽章
  - 移除未使用的 `FileText` 直接导入

### Bug 修复（侧边栏）
- `SideNavigation.tsx`:
  - 选中状态修复：`isWorkspaceSectionActive` 排除 `/workspace/custom/`
  - 名称卡片：Portal 到 `document.body`（不被 overflow 裁剪）
  - 加号按钮：新增悬停名称卡片
  - 图标：迁移到 `PAGE_ICON_MAP` / `CUSTOM_ICON_MAP`（SSOT）
  - 创建页面时：图标选择器弹窗（5 列网格）
  - 侧边栏滚动：`flex: 1; overflow-y: auto`
- `SideNavigation.module.css`: scroll + portal 名称卡片 CSS
- `settingsStore.ts`: `addCustomPage` 修复回滚逻辑；`CustomPage.icon` → `iconId`
- `settings.rs`: 自动创建 profile 当不存在时
- `pageIcons.ts`: 新增 SSOT 图标映射

---

*文档维护者: Claude Code 审计*
*下次审计建议: P0-1 完成后重新审查 object 层实现*
