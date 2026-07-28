# SoloSoul CLI 终端版本预研报告

> 版本：v2.0
> 日期：2026-06-14
> 依据文档：`/Users/zzc/PycharmProjects/SoloSoul/docs/solosoul_cli_prepare.md`
> 目标读者：SoloSoul 客户端开发团队、架构师、产品经理

---

## 1. 执行摘要

本报告基于 `solosoul_cli_prepare.md` 的产品设想，对 **SoloSoul 终端 CLI 版本** 的开发进行系统性预研，并**以当前 GUI 客户端的完整功能集为基准**，明确 CLI 需要覆盖的能力边界、技术选型与实施路径。核心结论如下：

- **技术可行性高**：现有 Rust 后端（`solosoul-vault`、`solosoul-crypto`、`VaultService`）已完整覆盖账户管理、Vault 加解密、对象 CRUD、模板系统、生物识别、导入导出、备份、插件、LLM、同步等能力，CLI 只需实现交互层与命令路由。
- **推荐技术栈**：Rust 独立二进制 + `ratatui` + `crossterm` + `clap` + `inquire`。以**全屏 TUI 为主**，`inquire` 仅用于临时退出全屏的向导场景。
- **关键前提**：`VaultService` 当前与 `tauri::State`、`tauri::Emitter` 耦合，建议先通过**复制核心逻辑到 `solosoul-core`、保留 Tauri 薄包装层**的方式解耦，预估 2–3 天，对现有 GUI 影响最小。
- **最大风险点**：`VaultService` 解耦的工程复杂度；CLI/GUI 同时访问 Vault 的并发安全；生物识别为**可选增强特性**而非 P0；模板字段的富交互填写体验。
- **推荐实施策略**：新增 **Phase 0（核心库抽取）**，再按「基础框架 → 账户生命周期 → 核心数据操作 → 增强能力 → 发布」推进；初期 `/update` 可不实现，依赖 `cargo install` 或包管理器更新。

---

## 2. 项目背景与目标

### 2.1 背景

SoloSoul（独灵）当前主产品为 **Tauri + React** 跨平台桌面客户端，所有敏感数据本地存储，采用 Argon2id + AES-256-GCM 的零知识架构。随着产品成熟，终端用户需要一个**不依赖图形界面、在命令行中即可完成核心数据操作**的入口，以覆盖以下场景：

- 开发者、运维人员、极客用户的高效操作习惯；
- 远程 SSH / 无 GUI 服务器环境下的账户访问；
- 自动化脚本与批量数据录入的前置能力；
- 与 GUI 客户端形成互补，扩大产品覆盖边界；
- 在资源受限设备（低内存、无显卡）上提供完整数据访问能力；
- 为后续 AI Agent、自动化工作流提供可编程接口的本地入口。

### 2.2 目标

开发一个名为 `solosoul` 的终端可执行文件，支持：

1. 首次启动引导创建账户；
2. 账户登录（主密码、指纹、面容）；
3. 账户内核心操作：创建页面（page）、创建对象（object）、按模板填写字段；
4. 未登录状态可用辅助命令：`/account_list`、`/doctor`、`/update`、`/exit`。

CLI 必须**继承 SoloSoul 的本地优先与零知识原则**：主密码不落盘、敏感数据按需掩码、Vault 超时自动锁定。

### 2.3 与 GUI 版本对齐的范围声明

CLI 不是 GUI 的简化版，而是**同一后端的另一套交互宿主**。以下能力必须与 GUI 对齐：

| 维度 | CLI 对齐要求 |
|------|-------------|
| 数据模型 | 完全复用 `solosoul_vault` 的 `ObjectRecord`、`UserTemplate`、`AttachmentMeta`、`AuditLogEntry` 等类型 |
| 命令语义 | CLI 命令与 Tauri Commands 一一映射，参数/返回值尽量复用 |
| 安全策略 | 主密码不落盘、敏感字段掩码、会话超时锁定、操作审计、敏感度分级完全一致 |
| 插件生态 | 复用同一套 WASM ABI，CLI 宿主补齐终端版 Host Functions |
| 数据互通 | CLI 与 GUI 共享 `~/.solosoul` 数据目录，通过文件锁避免并发冲突 |

---

## 3. 需求梳理

### 3.1 启动与全局命令

| 命令 | 触发条件 | 说明 |
|------|---------|------|
| `solosoul` | 任意 | 启动 CLI。无账户时进入创建流程；有账户时进入登录流程。 |
| `solosoul upgrade` | 显式调用 | 检查并安装新版本。 |
| `solosoul --data-dir <path>` | 显式调用 | 指定数据目录，覆盖 `SOLOSOUL_DATA_DIR` 与默认目录。 |
| `/exit` | 未登录 / 已登录 | 安全退出，清理内存中的 session key。 |

### 3.2 未登录可用命令

| 命令 | 功能 | 后端映射 |
|------|------|---------|
| `/login` | 登录账户 | `auth::login` / `biometric::biometric_unlock` |
| `/account_list` | 列出本地账户 | `vault::vault_list_accounts` |
| `/doctor` | 检查依赖、权限、数据目录健康 | 新增诊断逻辑 |
| `/update` | 检查版本更新 | 新增更新检查逻辑 |
| `/exit` | 退出 | — |

### 3.3 登录后可用命令

#### 数据操作

| 命令 | 功能 | 后端映射 |
|------|------|---------|
| `/newpage [page name]` | 创建页面 | `object::object_create(collection_type="page")` |
| `/newobject [page name]` | 创建对象（进入分步流程） | `object::object_list` → `template::template_list` → `object::object_create` |
| `/list [page name]` | 列出页面或页面内对象 | `object::object_list` |
| `/open <object-id>` | 查看/编辑对象详情 | `object::object_get` / `object::object_update` |
| `/edit <object-id>` | 编辑对象 | `object::object_update` |
| `/delete <object-id>` | 软删除对象或页面 | `object::object_delete` / `object::page_delete` |
| `/search [keyword]` | 搜索页面、对象、字段属性 | `search::search_unified` / `search::search_advanced` |
| `/trash`（或 `/bin`） | 查看回收站 | `object::object_trash_list` |
| `/restore <trash-id>` | 恢复回收站项目 | `object::trash_restore` |
| `/purge <trash-id>` | 彻底删除回收站项目 | `object::trash_permanent_delete` / `object::object_purge` |
| `/history <object-id>` | 查看对象历史快照 | `object::snapshot_list` / `snapshot_get_data` |
| `/rollback <object-id> <snapshot-id>` | 回滚到历史版本 | `object::snapshot_rollback` |
| `/operation_log` | 查看操作日志 | `log::log_get_recent` |
| `/export [path]` | 导出账户数据 | `export_import::export_execute` |
| `/import [path]` | 导入数据 | `export_import::import_execute` / `import_execute_advanced` |

#### 附件系统

| 命令 | 功能 | 后端映射 |
|------|------|---------|
| `/attach <object-id> <file-path>` | 为对象添加附件 | `attachment::attachment_save` / `attachment_copy_to_vault` |
| `/attachments <object-id>` | 列出对象附件 | `attachment::attachment_list` |
| `/attach-rename <object-id> <attachment-id> <new-name>` | 重命名附件 | `attachment::attachment_rename` |
| `/attach-delete <object-id> <attachment-id>` | 软删除附件 | `attachment::attachment_soft_delete` |
| `/attach-restore <object-id> <attachment-id>` | 恢复已删除附件 | `attachment::attachment_restore` |
| `/attach-purge <object-id> <attachment-id>` | 彻底删除附件 | `attachment::attachment_delete` |

#### LLM 与模型

| 命令 | 功能 | 后端映射 |
|------|------|---------|
| `/model` | 查看和选择当前 LLM 模型 | `llm::llm_get_config` / `llm::llm_set_active_provider` |
| `/llm_config` | 查看/编辑当前 LLM 配置 | `llm::llm_get_config` / `llm::llm_save_provider` |
| `/llm_chat` | 进入 AI 对话模式 | `llm::llm_send_message` / `llm::llm_chat` |
| `/llm_conversations` | 管理对话历史 | `llm::llm_list_conversations` / `llm_*_conversation` |
| `/llm_stats` | 查看 LLM 使用统计 | `llm::llm_get_stats` |
| `/embed_model` | 管理本地 Embedding 模型 | `embed_model::llm_get_embed_models` / `llm_download_embed_model` |

#### 插件系统

| 命令 | 功能 | 后端映射 |
|------|------|---------|
| `/plugin` | 进入插件系统 | `plugin::plugin_list_installed` / `plugin::plugin_run` |
| `/plugin-market` | 浏览插件市场 | `plugin::plugin_list_all` |
| `/plugin-install <id> <version>` | 安装插件 | `plugin::plugin_install` |
| `/plugin-update <id>` | 更新插件 | `plugin::plugin_update` |
| `/plugin-uninstall <id>` | 卸载插件 | `plugin::plugin_uninstall` |
| `/plugin-run <id>` | 运行插件 | `plugin::plugin_run` |
| `/plugin-sessions` | 查看插件会话 | `plugin::plugin_list_sessions` |
| `/plugin-audit` | 查看插件审计日志 | `plugin::plugin_audit_log` |

#### 账户状态与系统

| 命令 | 功能 | 后端映射 |
|------|------|---------|
| `/size`（或 `/status` / `/state`） | 查看账户占用大小、对象数量等统计 | `crypto::get_vault_stats` / `object::object_list` |
| `/backup [path]` | 备份账户数据 | `backup::backup_create` |
| `/backup-restore <backup-id>` | 从备份恢复 | `backup::backup_restore` |
| `/backups` | 列出本地备份 | `backup::backup_list` |
| `/backup-delete <backup-id>` | 删除本地备份 | `backup::backup_delete` |
| `/sync` | 设备同步管理 | `sync::sync_discover` / `sync::sync_with_device` |
| `/ocr <image-path>` | OCR 识别本地图片 | `ocr::ocr_scan_image` |
| `/debug_log` | 查看调试日志 | `log::log_get_recent` / `log::log_export` |
| `/about`（或 `/version`） | 版本信息与条款 | `system::get_app_info` + `docs/` 条款 |

#### 设置与帮助

| 命令 | 功能 | 后端映射 |
|------|------|---------|
| `/setting` | 进入设置页面 | `settings::user_data_get_preferences` / `user_data_update_preference` |
| `/security` | 安全设置（密码、生物识别、回收站保留期） | `vault::change_password` / `biometric::*` / `object::trash_set_retention` |
| `/language [lang]` | 切换界面语言 | `settings::ui_update_preference` |
| `/theme [theme]` | 切换主题（TUI 配色） | `settings::ui_get_preferences` |
| `/help [command]` | 帮助文档与命令说明 | 本地帮助文本 |
| `/logout` | 退出登录 | `auth::logout` / `vault::lock` |

> **命令别名说明**：
> - `/size` / `/status` / `/state` 可互为别名，默认显示账户统计；
> - `/trash` / `/bin` 可互为别名；
> - `/about` / `/version` 可互为别名，`/about` 额外显示条款链接。

### 3.4 创建对象流程拆解

根据 `prepare.md`，`/newobject` 采用三步向导：

1. **选择所属页面**：支持直接输入页面名或从列表选择；页面不存在时给出明确错误。
2. **选择对象模板**：展示该账户下的 `UserTemplate` 列表（含系统模板与用户模板）。
3. **填写字段属性**：根据模板 `properties` 渲染字段卡片，逐项输入后保存。

字段类型（基于现有 `system_templates_*.json` 与 `PropertyType`）：

- `text`：单行文本
- `date`：日期
- `datetime`：日期时间
- `email` / `phone` / `url`：带格式校验的文本
- `number`：数值
- `single_select` / `multi_select`：单选/多选
- `boolean`：是/否
- `textarea` / `markdown`：多行文本（可唤起外部编辑器）
- `attachment` / `image`：附件（CLI 下可支持路径输入，暂不展示富媒体）
- `relation`：指向其他对象的引用（P2）

### 3.5 敏感性与权限

CLI 必须尊重 GUI 统一的 4 级 `sensitivity_level` 分级：

| 级别 | 显示策略 | 编辑验证 |
|------|---------|---------|
| `public` | 明文显示 | 无 |
| `internal` | 默认明文显示 | 无 |
| `sensitive` | 展示时掩码（如 `••••••`），查看前需生物识别/密码验证 | 需密码验证 |
| `critical` | 展示时掩码，编辑前需主密码验证，并记录更严格的审计日志 | 需密码验证 + 操作确认 |

---

## 4. 现有架构可复用性分析

### 4.1 Workspace 结构

当前 `tauri/Cargo.toml` 已定义 workspace：

```toml
[workspace]
members = [
    "src-tauri",
    "crates/solosoul-crypto",
    "crates/solosoul-vault",
    "crates/solosoul-sync",
]
```

新增 `crates/solosoul-cli` 或 `crates/solosoul-tui` 即可无缝接入，共享依赖版本。

### 4.2 可复用模块

| 模块 | 复用方式 | 说明 |
|------|---------|------|
| `solosoul_crypto::kdf` | 直接依赖 | Argon2id 密钥派生、`KdfConfig`、`derive_key`、`generate_salt` |
| `solosoul_crypto::secure` | 直接依赖 | 常量时间比较 `secure_compare` |
| `solosoul_vault::VaultStore` | 直接依赖 | 数据库打开、对象/模板/快照/回收站操作 |
| `solosoul_vault::ObjectRecord` / `UserTemplate` / `TemplateProperty` | 直接依赖 | 数据模型 |
| `services::vault_service::VaultService` | 复制到 `solosoul-core` + Tauri 薄包装 | 核心逻辑复制到新 crate；原 Tauri 版本保留为薄包装层，对 GUI 影响最小 |
| `services::template_service` | 参考/复用 | 默认模板导入、模板检测 |
| `commands::biometric` | 移植 | macOS LocalAuthentication FFI 逻辑可移植到 CLI |

### 4.3 需解耦的 Tauri 依赖

当前 `VaultService`（约 1000+ 行）、`biometric.rs` 与 `tauri::State`、`tauri::Emitter` 强耦合。直接迁移会影响 `src-tauri/src/commands/` 下大量命令，存在并行开发冲突风险。

**推荐解耦方案（最小化对 GUI 影响）**：

1. **新建 `crates/solosoul-core`**：将 `VaultService` 的核心逻辑（账户创建、解锁、锁定、列表、密码验证、session key 管理）复制到该 crate，剥离所有 `tauri::*` 类型；
2. **保留 Tauri 薄包装层**：`src-tauri/src/services/vault_service.rs` 改为对 `solosoul-core::VaultService` 的轻量封装，仅负责：
   - 将 `tauri::State` 注入核心服务；
   - 在 `lock()` 等关键事件时调用 `app_handle.emit("vault-locked", ())`；
3. **事件机制抽象**：核心 crate 通过 callback / signal（如 `Box<dyn Fn(LockEvent)>`）通知上层，Tauri 包装层将其转换为 Tauri 事件；
4. **生物识别命令去 Tauri 化**：将 `commands/biometric.rs` 中的业务逻辑提取为 `solosoul-core::biometric` 模块，`tauri::command` 仅做参数透传；
5. **短期可接受两份代码并存**：通过 feature flag 或内部 re-export 隔离，待 CLI 稳定后再彻底废弃旧版本。

**预估工时**：2–3 天（复制 + 验证 Tauri 包装层行为一致）。

**建议**：优先完成此项工作，它是 CLI 与 GUI 长期共享内核的基础，也是性价比最高的架构投资。

### 4.4 命令映射总览

CLI 命令与现有 Tauri Commands 的映射关系如下表（按模块）：

| GUI 模块 | Tauri Command | CLI 命令 | 优先级 |
|---------|--------------|---------|--------|
| Auth | `check_has_account` | 启动时自动调用 | P0 |
| Auth | `bootstrap` | 首次启动引导 | P0 |
| Auth | `login` | `/login` | P0 |
| Auth | `logout` | `/logout` | P0 |
| Auth | `verify_password` | 敏感操作前验证 | P0 |
| Auth | `get_current_account` | 状态栏显示 | P0 |
| Vault | `unlock` | `/login` 内部调用 | P0 |
| Vault | `lock` | `/logout`、`/exit`、超时锁定 | P0 |
| Vault | `get_state` | 状态判断 | P0 |
| Vault | `change_password` | `/security password` | P1 |
| Vault | `delete_account` | `/security delete-account` | P1 |
| Vault | `vault_list_accounts` | `/account_list` | P0 |
| Vault | `vault_update_hint` | `/security hint` | P1 |
| Biometric | `biometric_check_availability` | 登录界面动态显示 | P1 |
| Biometric | `biometric_save_credential` | `/security biometric enable` | P1 |
| Biometric | `biometric_unlock` | 登录选项 | P1 |
| Biometric | `biometric_delete_credential` | `/security biometric disable` | P1 |
| Biometric | `biometric_test` | `/security biometric test` | P2 |
| Profile | `profile_save` / `profile_load` / `profile_list` / `profile_delete` | `/profile` 管理 | P2 |
| Profile | `profile_get_section` / `profile_update_field` | `/profile section` / `/profile field` | P2 |
| Object | `object_list` | `/list`、`/search` | P0 |
| Object | `object_get` | `/open` | P0 |
| Object | `object_create` | `/newpage`、`/newobject` | P0 |
| Object | `object_update` | `/edit` | P0 |
| Object | `object_delete` / `object_purge` | `/delete` / `/purge` | P0 |
| Object | `object_restore` / `trash_restore` | `/restore` | P0 |
| Object | `object_trash_list` / `trash_get_detail` | `/trash` | P0 |
| Object | `trash_permanent_delete` | `/purge` | P0 |
| Object | `trash_get_retention` / `trash_set_retention` | `/security trash-retention` | P1 |
| Object | `snapshot_list` / `snapshot_get` / `snapshot_get_data` / `snapshot_rollback` | `/history`、`/rollback` | P1 |
| Template | `template_list` / `template_get` | `/newobject` 内部调用 | P0 |
| Template | `template_create` / `template_update` / `template_delete` / `template_restore` | `/template` 管理 | P1 |
| Template | `template_save_from_object` | `/template save-from <object-id>` | P1 |
| Search | `search_unified` / `search_advanced` | `/search` | P0 |
| Export/Import | `export_get_scope_tree` / `export_estimate_size` / `export_get_attachments` / `export_execute` | `/export` | P1 |
| Export/Import | `import_parse_package` / `import_get_password_hint` / `import_decrypt_preview` / `import_execute` / `import_execute_advanced` | `/import` | P1 |
| Settings | `ui_get_preferences` / `ui_update_preference` | `/theme`、`/language` | P1 |
| Settings | `user_data_get_preferences` / `user_data_update_preference` | `/setting` | P1 |
| LLM | `llm_get_config` / `llm_save_provider` / `llm_set_active_provider` 等 | `/llm_config`、`/model` | P2 |
| LLM | `llm_send_message` / `llm_chat` / `llm_list_conversations` 等 | `/llm_chat` | P2 |
| LLM | `llm_get_stats` / `llm_reset_stats` / `llm_persist_stats` | `/llm_stats` | P2 |
| Embed Model | `llm_get_embed_models` / `llm_download_embed_model` / `llm_delete_embed_model` | `/embed_model` | P2 |
| Plugin | `plugin_list_all` / `plugin_list_installed` / `plugin_install` / `plugin_update` / `plugin_uninstall` | `/plugin-market`、`/plugin` | P2 |
| Plugin | `plugin_run` / `plugin_consent_response` / `plugin_dialog_response` | `/plugin-run` | P2 |
| Plugin | `plugin_list_sessions` / `plugin_audit_log` / `plugin_update_registry` | `/plugin-sessions`、`/plugin-audit` | P2 |
| OCR | `ocr_scan_image` / `ocr_get_supported_languages` | `/ocr` | P2 |
| Attachment | `attachment_list` / `attachment_save` / `attachment_soft_delete` / `attachment_restore` / `attachment_rename` / `attachment_delete` / `attachment_count_batch` / `attachment_copy_to_vault` / `attachment_cleanup_orphans` | `/attach*` 系列 | P1 |
| Backup | `backup_list` / `backup_create` / `backup_restore` / `backup_delete` | `/backup*` 系列 | P1 |
| Sync | `sync_discover` / `sync_get_status` / `sync_enable` / `sync_with_device` / `sync_trust_peer` / `sync_forget_peer` | `/sync` | P2 |
| Log | `log_write` / `log_get_recent` / `log_export` | `/operation_log`、`/debug_log` | P1 |
| System | `get_app_info` / `get_system_locale` | `/about` | P0 |

---

## 5. 技术选型

### 5.1 TUI 框架对比

| 框架 | 类型 | 优点 | 缺点 | 适用场景 |
|------|------|------|------|---------|
| **ratatui** | 全屏 TUI 框架 | 社区活跃、组件丰富、跨平台（crossterm/termion）、文档完善 | 学习曲线略陡，需手动管理状态与事件循环 | 首页、对象字段卡片、列表选择、底部命令输入框 |
| **inquire** | 交互式提示库 | API 简洁（Select/Password/DateSelect/Editor）、快速构建向导 | 非全屏，与 ratatui 全屏切换需管理屏幕状态 | 临时退出全屏的向导步骤（如日期选择、外部编辑器） |
| **dialoguer** | 交互式提示库 | 成熟、与 clap 生态兼容好 | 界面较朴素，定制性弱于 inquire | 简单确认、输入 |
| **cliclack** | 现代提示库 | 类 @clack/prompts 的美观输出 | 较新，生态成熟度低于 inquire | 品牌化命令行体验 |
| **crossterm** | 终端控制底层 | 跨平台输入/输出、清屏、光标、颜色 | 需自行封装高层组件 | 与 ratatui 配合使用；屏幕切换与恢复 |
| **rustyline** | REPL 行编辑 | 提供历史、补全、行编辑 | 与 ratatui 全屏模式有控制台竞争 | 仅用于纯 REPL 模式（非默认） |

### 5.2 推荐方案

**主框架：`ratatui` + `crossterm`**

理由：

- 与 SoloSoul 现有 Rust 技术栈完全吻合；
- 支持全屏、无闪烁差分渲染、键盘/鼠标事件；
- 可构建 `pi-tui` 风格的组件化界面（参考 pi 项目的 `SettingsList`、`SelectList`、`Input`、`Editor`）；
- 生态成熟，可搭配 `tui-input`、`tui-prompts`、`tui-widgets` 加速开发。

**交互模式：以全屏 TUI 为主，临时退出使用 `inquire`**

- 首页、页面/模板列表、字段卡片等复杂布局在 `ratatui` 中绘制；
- 对于 `date` 选择、`single_select` / `multi_select`、外部编辑器（`$EDITOR`）等场景，可临时退出 `ratatui` 全屏，调用 `inquire` 完成输入，再恢复 `ratatui` 界面。此模式可避免在 ratatui 中自研复杂模态组件，降低 MVP 成本。

**命令解析：`clap`**

- `clap`：处理 `solosoul upgrade`、全局 flag（如 `--data-dir`）、子命令路由；
- 进入交互模式后，命令由 `ratatui` 底部自绘输入框解析，无需 `rustyline`。

### 5.3 命令解析

建议采用 **全屏 TUI + 底部命令输入框** 模式：

```
+------------------------------------------------+
| ... 内容区域 ...                                |
+------------------------------------------------+
| > /newobject 身份 ___________________________ |
+------------------------------------------------+
```

- 命令输入框自绘在 `ratatui` 界面底部，支持方向键翻阅历史、Tab 补全命令；
- 历史记录由 CLI 状态机维护，不依赖 `rustyline`；
- 若未来需要纯 REPL 模式（非全屏），可再引入 `rustyline` 作为可选实现。

---

## 6. CLI 架构设计

### 6.1 高层模块划分

```
crates/solosoul-cli/
├── Cargo.toml
├── src/
│   ├── main.rs              # 入口：解析参数、启动 App
│   ├── app.rs               # 全局状态机 (AppState)
│   ├── cli.rs               # clap 命令定义
│   ├── repl.rs              # REPL 循环
│   ├── tui.rs               # ratatui 启动/退出/事件循环
│   ├── events.rs            # 键盘事件映射
│   ├── screens/             # 各界面 Screen
│   │   ├── welcome.rs       # 首次启动欢迎 + 创建账户
│   │   ├── login.rs         # 登录选择
│   │   ├── home.rs          # 登录后首页
│   │   ├── new_page.rs
│   │   ├── new_object/
│   │   │   ├── select_page.rs
│   │   │   ├── select_template.rs
│   │   │   └── fill_fields.rs
│   │   ├── object_detail.rs
│   │   ├── trash.rs
│   │   ├── search.rs
│   │   ├── history.rs
│   │   ├── settings.rs
│   │   ├── plugin.rs
│   │   ├── llm_chat.rs
│   │   └── about.rs
│   ├── commands/            # 命令执行器
│   │   ├── auth.rs
│   │   ├── page.rs
│   │   ├── object.rs
│   │   ├── template.rs
│   │   ├── search.rs
│   │   ├── trash.rs
│   │   ├── export_import.rs
│   │   ├── settings.rs
│   │   ├── llm.rs
│   │   ├── plugin.rs
│   │   ├── sync.rs
│   │   ├── backup.rs
│   │   ├── doctor.rs
│   │   └── update.rs
│   ├── widgets/             # 可复用 TUI 组件
│   │   ├── account_list.rs
│   │   ├── page_list.rs
│   │   ├── template_card.rs
│   │   ├── field_editor.rs
│   │   ├── sensitive_input.rs
│   │   ├── command_input.rs # 底部命令输入框（含历史）
│   │   ├── object_table.rs
│   │   ├── attachment_list.rs
│   │   └── markdown_view.rs
│   └── strings.rs           # 终端文本常量（初期硬编码中文，后续按需国际化）
```

### 6.2 状态机

```rust
pub enum AppPhase {
    /// 无账户：引导创建
    Onboarding,
    /// 有账户但未登录
    Locked,
    /// 已登录：主界面
    Home { account_id: String },
    /// 创建对象向导
    CreatingObject { step: ObjectCreationStep },
    /// 对象详情/编辑
    ObjectDetail { object_id: String, mode: DetailMode },
    /// 回收站
    Trash,
    /// 搜索结果
    Search { query: String },
    /// 设置
    Settings { section: SettingsSection },
    /// LLM 对话
    LlmChat,
    /// 插件市场/已安装
    Plugin { sub_view: PluginSubView },
}

pub struct App {
    phase: AppPhase,
    vault_service: Arc<VaultService>,
    config: CliConfig,
    // ratatui 状态
    terminal: Terminal<CrosstermBackend<Stdout>>,
    // 命令历史
    command_history: Vec<String>,
    history_index: Option<usize>,
    // 会话空闲计时
    last_activity: Instant,
}
```

### 6.3 与 VaultService 的集成

CLI 启动时初始化 `VaultService::new()`，数据目录沿用现有规则：

- 优先 `SOLOSOUL_DATA_DIR`；
- 否则 `~/.solosoul`（Unix）或 `%USERPROFILE%\.solosoul`（Windows）。

登录成功后，`VaultService::unlock()` 会打开 `vault.db`，后续对象/模板操作直接通过 `get_vault_store()` 访问。

### 6.4 仓库组织决策

**决策：CLI 代码初期放在当前 `SoloSoul` 仓库内，不新建独立仓库。**

#### 目录位置

CLI 代码将位于当前仓库根目录下的 `solosoul_cli/` 文件夹中，与 `tauri/`、`docs/`、`sdk/` 等目录并列：

```
SoloSoul/
├── tauri/                       # 现有 Tauri + Rust workspace
│   ├── src-tauri/
│   ├── crates/
│   │   ├── solosoul-crypto/
│   │   ├── solosoul-vault/
│   │   ├── solosoul-sync/
│   │   └── solosoul-core/       # 解耦后的共享核心库
│   └── ...
├── solosoul_cli/                # CLI 代码目录
│   ├── Cargo.toml
│   └── src/
├── docs/
├── sdk/
└── ...
```

#### 决策理由

| 维度 | 当前仓库内 | 新建独立仓库 |
|------|-----------|-------------|
| 核心库复用 | 直接本地依赖 `crates/solosoul-core`，零版本同步成本 | 需通过 git 依赖或 crates.io，跨仓库协调成本高 |
| 开发效率 | 一个 PR 可同时修改核心库与 CLI，便于快速迭代 | 核心库改动需先合并、发布，CLI 才能跟进 |
| 当前阶段适配 | `VaultService` 尚未完全解耦，Monorepo 最利于推进 Phase 0 | 需先完成解耦并稳定发布，否则 CLI 开发受阻 |
| CI/CD | 复用现有 GitHub Actions，统一检查、测试、发布 | 需新建整套 CI 流水线 |
| 版本一致性 | CLI 与 GUI 版本号同步，用户认知一致 | 需额外维护版本映射 |
| 子模块复杂度 | 无新增 submodule | 增加 `SoloSoul_plugin_market` 之外的第二个子模块，维护负担更大 |

#### 未来拆分条件

当出现以下情况时，再考虑将 `solosoul_cli` 拆分为独立 GitHub 仓库：

1. `solosoul-core` 已稳定并发布到 crates.io 或作为独立 git crate 可用；
2. CLI 用户群体与 GUI 用户群体明显分离，独立 issue/PR 更有利于社区治理；
3. CLI 需要独立的 release 节奏；
4. 仓库体积和 CI 时间成为明显瓶颈。

> **说明**：若未来需要拆分，可使用 `git subtree split` 等工具从当前仓库历史中无损提取 CLI 目录到独立仓库。

### 6.5 CLI 与 GUI 插件系统兼容性

SoloSoul 当前插件系统基于 **WASM（`wasm32-wasip1`）+ `wasmtime`**，插件通过统一的 Host Functions ABI 与核心交互。因此 CLI 与 GUI 的插件**不一定需要开发两个版本**。

#### 6.5.1 当前插件 ABI 概述

插件依赖的 Host Functions 包括：

- `solosoul_request_field`：读取用户字段（需 Consent 授权）
- `solosoul_post_data` / `solosoul_http_request`：代理网络请求
- `solosoul_log` / `solosoul_get_timestamp`：日志与时间戳
- `solosoul_show_dialog`：通用对话框（单选/多选/输入）
- `solosoul_result`：返回结构化结果（`text` / `key_value` / `table` / `markdown`）

由于插件逻辑本身不依赖 React 或 Tauri，**理论上同一个 `.wasm` 文件可以在 GUI 和 CLI 两个宿主中运行**，只要 CLI 宿主实现了相同的 ABI。

#### 6.5.2 GUI vs CLI 能力对照

| 能力 | GUI 宿主 | CLI 宿主 | 兼容性 |
|------|---------|---------|--------|
| `request_field` 读字段 | React 弹窗授权 | 终端提示授权 | ✅ 统一 ABI |
| `post_data` / `http_request` | 后台执行 | 后台执行 | ✅ 统一 ABI |
| `log` / `get_timestamp` | 直接可用 | 直接可用 | ✅ 统一 ABI |
| `result_text` / `result_key_value` / `result_table` | React 渲染 | 终端文本/表格渲染 | ⚠️ 需结果渲染层适配 |
| `result_markdown` | React Markdown 组件 | 终端简化 Markdown 或纯文本降级 | ⚠️ 需降级处理 |
| `show_dialog`（单选/多选/输入） | GUI 对话框 | `inquire` 终端对话框 | ⚠️ 需宿主提供终端实现 |
| 图片/图表/富媒体结果 | GUI 展示 | 终端无法展示 | ❌ GUI-only |

#### 6.5.3 推荐策略：一份核心逻辑，两套宿主实现

**不要为 CLI 单独建立一套插件生态**。正确做法是：

1. **统一插件 ABI**：CLI 宿主补齐所有 Host Functions 的终端版本；
2. **结果渲染层分别适配**：
   - GUI：React 组件根据 `result.type` 渲染；
   - CLI：`ratatui` widgets 根据 `result.type` 渲染，不支持的类型降级为文本提示；
3. **插件 Manifest 声明兼容范围**：

```json
{
  "id": "com.solosoul.plugin.passport-expiry-check",
  "name": "护照过期检查",
  "supported_interfaces": ["gui", "cli"],
  "cli_compatible": true
}
```

4. **官方插件优先双端兼容**：纯数据处理、网络请求、简单对话框类插件应只维护一个 WASM 版本；
5. **GUI-only 插件明确声明**：图片、图表、拖拽交互、截图 OCR 等插件声明 `"supported_interfaces": ["gui"]`，CLI 市场不展示。

#### 6.5.4 需要单独开发 CLI 版本的场景

| 场景 | 原因 | 示例 |
|------|------|------|
| 强依赖 GUI 对话框 | 终端无法还原复杂弹窗 | 带图片预览的选择器 |
| 返回富媒体结果 | 终端无法渲染 | 图表、图片画廊、地图 |
| 需要拖拽/点击交互 | 终端无鼠标坐标输入 | 截图标注插件 |
| 工作流在终端下完全不可用 | 业务逻辑依赖 GUI | OCR 截图识别插件 |

#### 6.5.5 对 CLI 宿主的具体要求

为了让 GUI 插件在 CLI 下良好运行，CLI 宿主需要实现：

- **终端对话框**：将 `show_dialog` 映射为 `inquire::Select` / `MultiSelect` / `Text`，必要时临时退出 `ratatui` 全屏；
- **结果渲染器**：
  - `text` / `key_value`：直接文本输出；
  - `table`：`ratatui` 表格组件或 ASCII 表格；
  - `markdown`：简化渲染（标题、列表、加粗）或纯文本降级；
- **授权提示**：字段级 Consent 在终端中以列表形式展示，用户输入 `Y/n` 确认。

#### 6.5.6 结论

| 插件类型 | 是否需要两个版本 |
|---------|----------------|
| 纯数据处理/网络请求型官方插件 | ❌ 不需要，一份 WASM 跑双端 |
| 需要简单对话框的官方插件 | ❌ 不需要，统一 ABI + 终端对话框实现 |
| 返回 Markdown/表格的官方插件 | ❌ 不需要，结果渲染层分别适配 |
| 图片/图表/拖拽交互型插件 | ✅ 需要，或声明为 GUI-only |
| 第三方插件 | 由开发者声明 `cli_compatible`，不强制 |

**一句话：插件系统应坚持“一份核心逻辑 + 两套宿主实现（GUI/CLI）+ 统一 WASM ABI”。不要为了 CLI 把插件生态劈成两半。**

---

## 7. 关键流程详细设计

### 7.1 首次启动 / 账户创建

```
+----------------------------------+
|  欢迎使用 SoloSoul CLI           |
|  本地优先 · 零知识 · 你的数据你做主 |
+----------------------------------+

尚未发现本地账户，请先创建：

[1/4] 账户名：________________
[2/4] 主密码：________________
[3/4] 再次输入主密码：________
[4/4] 密码提示词（可选）：______

⚠️  主密码不会被保存，无法找回。
    丢失主密码将导致所有数据永久不可访问。

确认创建？[Y/n]
```

实现要点：

- 账户名校验：非空、不重复（调用 `VaultService` 已有逻辑）；
- 主密码校验：长度 ≥ 8，两次输入一致；
- 创建成功后立即解锁 Vault，并调用 `seed_default_templates()` 导入默认模板；
- 进入首页前显示安全提示。
- 对齐 GUI `auth::bootstrap(account_name, password, locale, password_hint)`。

### 7.2 登录流程

```
+----------------------------------+
|  选择账户登录                     |
+----------------------------------+

  [ ] account_1
  [✓] account_2
  [ ] account_3

选择登录方式：
  [✓] 主密码登录
  [ ] 指纹识别 (Touch ID)
  [ ] 面容识别 (Face ID)   [仅当已开启时显示]
```

实现要点：

- 账户列表来自 `vault_service.list_accounts()`；
- 生物识别选项仅当 `biometric_enabled && is_configured(account_id)` 时显示；
- 主密码登录调用 `auth::login(account_id, password)`；
- 指纹/面容登录调用现有 `biometric_unlock()` 的移植版本；
- 登录失败给出明确错误，不泄露账户是否存在。
- 连续失败 3 次后隐藏生物识别选项（与 GUI 对齐）。

### 7.3 首页（Home）

```
+------------------------------------------------+
| SoloSoul · account_2              [🔒 锁定]   |
+------------------------------------------------+
| 欢迎回来！                                      |
|                                                 |
| 数据操作：                                      |
|   /newpage [页面名]   创建页面                  |
|   /newobject [页面名] 创建对象                  |
|   /list [页面名]      列出对象                  |
|   /open <id>          查看对象                  |
|   /edit <id>          编辑对象                  |
|   /delete <id>        删除对象                  |
|   /search [关键词]    搜索                      |
|   /trash              回收站                    |
|   /history <id>       历史记录                  |
|   /export [路径]      导出                      |
|   /import [路径]      导入                      |
|                                                 |
| 附件：                                          |
|   /attach <id> <path> 添加附件                  |
|   /attachments <id>   列出附件                  |
|                                                 |
| 智能与插件：                                    |
|   /model              LLM 模型选择              |
|   /llm_config         LLM 配置                  |
|   /llm_chat           AI 对话                   |
|   /plugin             插件系统                  |
|                                                 |
| 系统与状态：                                    |
|   /size               账户统计                  |
|   /backup [路径]      备份                      |
|   /backup-restore <id> 恢复备份                 |
|   /sync               设备同步                  |
|   /ocr <path>         OCR 识别                  |
|   /operation_log      操作日志                  |
|   /debug_log          调试日志                  |
|   /about              版本与条款                |
|                                                 |
| 设置与帮助：                                    |
|   /setting            设置                     |
|   /security           安全设置                  |
|   /language [语言]    切换语言                  |
|   /theme [主题]       切换主题                  |
|   /help [命令]        帮助                      |
|   /logout             退出登录                  |
|   /exit               退出软件                  |
+------------------------------------------------+
| > ____________________________________________ |
+------------------------------------------------+
```

### 7.4 /newpage 流程

```rust
// 伪代码
async fn handle_new_page(name: Option<String>) -> Result<()> {
    let name = match name {
        Some(n) => n,
        None => inquire::Text::new("请输入页面名").prompt()?,
    };
    // 检查重复：list_objects filter collection_type="page"
    let exists = vault.list_objects(account_id, Some("page"), None, Some(&name), false, false)?;
    if !exists.is_empty() { return Err("页面名已存在".into()); }

    let input = CreateObjectInput {
        account_id: account_id.clone(),
        name,
        collection_type: "page".into(),
        properties: json!({}),
        parent_id: None,
        icon_name: Some("folder".into()),
        template_id: None,
        template_type: None,
        id: None,
    };
    object_create(input).await?;
    println!("页面创建成功");
    Ok(())
}
```

### 7.5 /newobject 流程

#### 步骤 1：选择页面

```
你正在创建对象流程中，请选择该对象的所属页面：

  [ ] 身份
  [ ] 旅行
  [✓] 财务
  [ ] 职业
  [ ] 自定义页面1
  [ ] 返回
```

- 若命令带 `[page name]`，直接校验是否存在；
- 若不存在，提示「页面名不存在，请输入正确的页面名」；
- 若用户选择「进入页面列表选择」，展示所有 `collection_type=page` 的对象。

#### 步骤 2：选择模板

```
请选择对象模板：

  [✓] 银行卡
  [ ] 护照
  [ ] 身份证
  [ ] 学历证书
  [ ] 工作履历
  [ ] 空白对象
  [ ] 返回
```

- 模板来自 `template_list()`；
- 空白对象对应无模板，仅输入对象名。

#### 步骤 3：字段填写

```
+------------------------------------------------+
| 创建对象：新银行卡                              |
+------------------------------------------------+
| 字段 1/5                                        |
|                                                 |
| [文本] 银行名称 [公开]                          |
| 中国工商银行                                    |
|                                                 |
| [↑/↓] 切换字段  [Enter] 编辑  [Tab] 保存        |
+------------------------------------------------+
```

字段渲染规则：

- 左侧显示字段类型图标 + 名称 + 敏感度标签；
- 右侧显示当前值：
  - `public` / `internal`：明文显示；
  - `sensitive` / `critical`：掩码显示（如 `••••••`），编辑前要求重新输入主密码；
- 特殊字段编辑方式：
  - `text` / `email` / `phone` / `url` / `number`：在 ratatui 中直接编辑；
  - `date`：临时退出 ratatui 全屏，调用 `inquire::DateSelect` 选择日期，完成后清屏并恢复 ratatui；
  - `datetime`：临时退出全屏，分别选择日期和时间；
  - `single_select` / `multi_select`：临时退出全屏，调用 `inquire::Select` / `MultiSelect`；
  - `textarea` / `markdown`：临时退出全屏，调用 `inquire::Editor` 唤起 `$EDITOR`；
  - `boolean`：在 ratatui 中按空格切换；
  - `attachment` / `image`：仅支持输入本地文件路径，不做预览。

**屏幕切换实现要点**：

1. 编辑前调用 `terminal.leave_alternate_screen()?` 退出 ratatui 备用屏幕；
2. 执行 `inquire` 提示或外部编辑器；
3. 完成后清屏并重新进入备用屏幕 `terminal.enter_alternate_screen()?`；
4. 触发一次全量重绘，避免残影。

保存时构造 `CreateObjectInput` 并调用 `object_create`，成功后返回首页。

### 7.6 /open 与 /edit 对象详情

CLI 对象详情页需展示：

- 对象名称、所属页面、模板类型、标签；
- 属性列表（含敏感度掩码）；
- 附件数量与列表入口；
- 历史记录入口；
- 操作按钮：编辑、删除、添加附件、查看历史。

编辑模式支持：

- 增量更新：仅发送修改的属性；
- 每次更新自动创建快照；
- `critical` / `sensitive` 属性编辑前要求主密码验证。

### 7.7 /trash 回收站

回收站 CLI 界面需支持：

- 列表展示被删除的页面和对象（含名称、类型、删除时间、剩余保留期）；
- 时间筛选：`all` / `1d` / `3d` / `7d` / `30d` / `half_year`；
- 类型筛选：`all` / `page` / `object`；
- 搜索：按 `name_snapshot` 过滤；
- 恢复与彻底删除；
- 批量操作。

命令示例：

```
/trash                    # 进入回收站视图
/trash --filter 7d        # 只看 7 天内删除的
/trash --type object      # 只看对象
/restore <trash-id>       # 恢复指定项目
/purge <trash-id>         # 彻底删除指定项目
```

### 7.8 /search 搜索

CLI 搜索需支持：

- 关键词搜索对象名称、字段名、字段值；
- 结果分类：页面 / 对象；
- 高亮匹配字段；
- 支持进入对象详情。

```
/search 护照
> 找到 3 个结果：
  [对象] 护照 · 张三        匹配：字段 "姓名" = "张三"
  [对象] 签证 · 日本        匹配：字段 "护照号" = "E12345678"
  [页面] 旅行               匹配：页面名
```

### 7.9 /export 与 /import

导出命令：

```
/export --pages travel,financial --output ~/backup.solosoul
/export --objects obj_abc,obj_def --include-attachments --output ~/backup.solosoul
/export --full --output ~/full_backup.solosoul
```

导入命令：

```
/import ~/backup.solosoul
/import ~/backup.solosoul --preview
/import ~/backup.solosoul --strategy merge
```

对齐 GUI `ExportImportPage` 的三级树形结构：页面 → 对象 → 附件。

### 7.10 /backup 与 /backup-restore

备份命令：

```
/backup --name manual_2026_06_14
/backups
/backup-restore <backup-id>
/backup-delete <backup-id>
```

备份使用 `backup::backup_create`，恢复使用 `backup::backup_restore`，删除使用 `backup::backup_delete`。

### 7.11 /sync 设备同步

同步命令：

```
/sync --discover            # 发现附近设备
/sync --status              # 查看同步状态
/sync --enable true         # 启用/禁用同步
/sync --with <device-id>    # 与指定设备同步
/sync --trust <peer-id>     # 信任/取消信任对端
/sync --forget <peer-id>    # 忘记对端
```

### 7.12 /llm_chat AI 对话

CLI LLM 对话采用类似 `kimi-code` 的底部输入框 + 上方消息区域布局：

```
+------------------------------------------------+
|  系统：你好，我是 SoloSoul AI 助手。            |
|  用户：帮我找一下即将过期的证件。               |
|  助手：找到 2 个即将过期的证件...               |
+------------------------------------------------+
| > 输入消息...                                  |
+------------------------------------------------+
```

支持：

- 多轮对话；
- 对话历史管理（`/llm_conversations`）；
- 本地 Embedding 检索（`guide_search`）。

### 7.13 /plugin 插件系统

CLI 插件界面：

```
+------------------------------------------------+
| 插件市场                          [已安装] [市场] |
+------------------------------------------------+
| [✓] 护照过期检查        v1.2.0   [运行] [更新]  |
| [ ] 银行账单解析器      v0.9.0   [安装]         |
+------------------------------------------------+
```

运行插件时：

- 终端对话框替代 GUI 弹窗；
- 结果按类型渲染为文本/表格/简化 Markdown；
- Consent 授权以 `Y/n` 列表形式展示。

---

## 8. 安全与隐私

### 8.1 主密码输入

- 使用 `inquire::Password` 或 `crossterm::event` 实现隐藏输入；
- 密码字符串使用 `Zeroizing<String>` 包装，确保离开作用域后内存清零；
- 登录后立即从栈上清除明文副本。

### 8.2 敏感字段显示

- 统一封装 `SensitiveInput` / `SensitiveDisplay` 组件；
- 掩码逻辑与 GUI 端 `SensitiveValueWidget` 对齐；
- `critical` 字段编辑前要求主密码重验证，调用 `auth::verify_password()`。

### 8.3 会话与锁定

- 登录成功后保持 `VaultService` 解锁状态；
- 监听终端焦点/键盘空闲：
  - 5 分钟无操作自动调用 `vault_service.lock()`；
  - `Ctrl+L` 手动锁定；
  - `/logout` 显式登出。
- 进程退出前必须调用 `lock()` 并 `zeroize` session key。

### 8.4 CLI/GUI 并发访问控制

若 CLI 与 GUI 同时运行并访问同一 `~/.solosoul` 目录，SQLite 写入可能冲突。建议在 `VaultService::unlock()` 时获取进程级文件锁：

- 锁文件：`~/.solosoul/.lock`；
- 使用 `fs2` crate 的 `File::lock_exclusive()` 获取排他锁；
- 若锁已被占用，提示用户「Vault 正被其他程序使用，请关闭后再试」；
- GUI 端也需同步实现相同机制，确保双端互斥。

```rust
use fs2::FileExt;
let lock_file = std::fs::OpenOptions::new()
    .write(true)
    .create(true)
    .open(base_path.join(".lock"))?;
lock_file.try_lock_exclusive()
    .map_err(|_| "Vault 正被其他进程使用")?;
```

### 8.5 数据目录权限

沿用现有 `VaultService` 逻辑：

- 目录 `0o700`；
- 文件 `0o600`；
- Windows 下通过 ACL 或保留默认权限（当前实现为 no-op）。

### 8.6 操作审计

所有创建/更新/删除操作调用 `vault.log_structured()`，与 GUI 保持一致，便于后续审计与同步。

---

## 9. 跨平台与生物识别

### 9.1 支持平台

| 平台 | 优先级 | 说明 |
|------|--------|------|
| macOS | P0 | 主战场，Touch ID / Face ID 已有 FFI 实现 |
| Windows | P1 | 大量用户，Windows Hello 需新增集成 |
| Linux | P2 | 可通过密码登录，生物识别暂不支持 |

### 9.2 生物识别实现思路

当前 `commands/biometric.rs` 已实现：

- macOS LocalAuthentication FFI（`objc2` + `block2`）；
- 主密钥通过 keyring 或混淆文件存储；
- 解锁时触发系统生物识别，成功后读取密钥并调用 `unlock_with_session_key()`。

CLI 复用路径：

1. 将生物识别逻辑从 `tauri::command` 抽成普通函数，放入 `solosoul-core`；
2. 在 CLI 登录界面检测平台与配置，动态显示生物识别选项；
3. macOS 直接复用现有 `objc2` + `block2` FFI；
4. Windows 增加 `windows::Security::Credentials::UI::UserConsentVerifier` 调用（中等工作量）；
5. Linux 暂不支持生物识别，仅密码登录。

**验证路径**：

- 在 Phase 0 或 Phase 1 编写一个独立 Rust 程序，验证 macOS LocalAuthentication FFI 在非 Tauri 环境下可正常弹出系统对话框（如 `sudo` 指纹认证类似）。
- Windows Hello 在终端下的弹窗行为需单独 POC；若验证成本高，MVP 可仅支持密码登录。

**定位**：CLI 生物识别为**可选增强特性，非 P0**。MVP 以主密码登录为主，macOS Touch ID / Face ID 可在 Phase 4 作为增强加入。

---

## 10. 依赖与构建

### 10.1 新增 crate 建议

```toml
[package]
name = "solosoul-cli"
version.workspace = true
edition.workspace = true

[dependencies]
# 内部 crate
solosoul-crypto = { path = "../solosoul-crypto" }
solosoul-vault = { path = "../solosoul-vault" }
# 假设 VaultService 已被抽到 solosoul-core
solosoul-core = { path = "../solosoul-core" }

# CLI / TUI
clap = { version = "4.5", features = ["derive"] }
ratatui = "0.29"
crossterm = "0.29"
inquire = { version = "0.9", features = ["date", "editor"] }
color-eyre = "0.6"         # 友好的彩色错误输出
fs2 = "0.4"                # 进程级文件锁

# 可选：纯 REPL 模式才需要
# rustyline = "14"

# 已有 workspace 依赖（可统一版本）
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
chrono = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
```

### 10.2 Workspace 调整

```toml
[workspace]
members = [
    "src-tauri",
    "crates/solosoul-crypto",
    "crates/solosoul-vault",
    "crates/solosoul-sync",
    "crates/solosoul-core",    # 新增：共享核心逻辑
    "crates/solosoul-cli",     # 新增
]
```

### 10.3 构建产物

- 二进制名：`solosoul`
- Release 路径：`target/release/solosoul`
- 分发方式：
  - Homebrew tap（macOS）
  - `cargo install solosoul-cli`
  - 与 Tauri 客户端一起打包到 `src-tauri/target/release/bundle`

---

## 11. 参考开源项目分析

### 11.1 pi-tui（原 @mariozechner/pi-tui，现 @earendil-works/pi-tui）

- **定位**：TypeScript 编写的最小化终端 UI 框架，服务于 AI 编码代理 Pi。
- **可借鉴点**：
  - 差分渲染 + 同步输出（CSI 2026）实现无闪烁更新；
  - 组件化架构（`Component` 接口：`render(width) -> string[]`）；
  - 内置 `SettingsList`、`SelectList`、`Input`、`Editor`、`Markdown` 组件；
  - 底部输入框 + 消息列表的聊天式布局。
- **对 SoloSoul 的启示**：首页、对象字段卡片、设置面板均可参考其组件化思想。

### 11.2 kimi-code / Codex / OpenCode / CodeWhale

- **共同点**：AI 编码助手的终端界面，均以「底部输入框 + 上方会话/输出区域」为核心。
- **可借鉴点**：
  - REPL 风格的斜杠命令；
  - 文件/路径自动补全；
  - 状态栏显示当前项目、模型、token 使用情况。
- **对 SoloSoul 的启示**：
  - CLI 首页可采用类似的「命令输入框 + 操作日志/提示区域」；
  - `/newobject` 的分步向导可借鉴这些工具的渐进式披露交互。

### 11.3 awesome-tuis

- **定位**：TUI 项目精选列表。
- **用途**：作为后续 UI 灵感库，关注文件管理器（如 `ranger`、`yazi`）、数据库客户端（如 `lazygit`）的列表/表单交互。

---

## 12. 风险与待确认事项

| 序号 | 风险/问题 | 级别 | 影响 | 建议 |
|------|----------|------|------|------|
| R1 | `VaultService` 解耦工作量被低估 | 高 | 影响 CLI 与 GUI 并行开发 | 先复制核心逻辑到 `solosoul-core`，保留 Tauri 薄包装层；预估 2–3 天 |
| R2 | 生物识别在终端环境中无法弹出系统对话框 | 中 | 影响登录体验 | 生物识别为可选特性；MVP 仅密码登录；macOS 先做 POC 验证 |
| R3 | CLI 与 GUI 同时写入导致数据损坏 | 中 | 数据安全风险 | 使用 `fs2` 在 `~/.solosoul/.lock` 实现进程级排他锁，GUI 同步实现 |
| R4 | 模板字段的复杂交互（日期、多选、附件）在 TUI 中体验差 | 中 | 影响对象创建效率 | text/date/select/textarea 优先；日期/多选临时退出全屏用 `inquire`；附件仅路径输入 |
| R5 | 升级机制复杂，引入安全隐患 | 低 | 增加发布与维护成本 | 初期不实现 `/update`，依赖 `cargo install` 或包管理器；Phase 4 再评估 |
| R6 | 国际化文本与 GUI 不共享 | 低 | 增加二进制体积 | 初期硬编码中文提示，后续按需引入轻量 i18n |
| R7 | 主密码在终端输入时被其他进程窥探 | 低 | 密码泄露风险 | 使用 `crossterm` 隐藏输入，依赖操作系统安全 |
| R8 | GUI 插件在 CLI 下无法运行或体验差 | 中 | 影响插件生态统一 | CLI 宿主补齐所有 Host Functions 终端实现；插件 Manifest 声明 `cli_compatible`；富媒体插件声明 GUI-only |
| R9 | LLM 流式输出在 TUI 中渲染复杂 | 中 | 影响 AI 对话体验 | 采用增量文本更新 + 滚动缓冲区；Phase 3 后实现 |
| R10 | 附件预览在终端无法展示 | 中 | 影响附件管理体验 | 提供文件路径/元数据查看；调用系统默认程序打开 |
| R11 | OCR 截图插件在 CLI 下不可用 | 低 | 限制部分插件场景 | 声明为 GUI-only；CLI OCR 仅支持本地图片路径 |

---

## 13. 推荐实施路径

### Phase 0：核心库抽取（2–3 天）🔴 关键前置

1. 新建 `crates/solosoul-core`；
2. 将 `VaultService` 核心逻辑（不含 `tauri::*`）复制到 `solosoul-core`；
3. 将 `commands/biometric.rs` 的业务逻辑提取到 `solosoul-core::biometric`；
4. 调整 `src-tauri/src/services/vault_service.rs` 为薄包装层，保留 `tauri::State` 注入与事件发射；
5. 运行 Tauri 端现有测试，确保行为一致。

> **说明**：此阶段独立于 CLI，可显著降低后续并行开发冲突。若时间紧迫，也可先复制简化版到 CLI crate，但建议尽早统一。

### Phase 1：基础架构（2–3 周）

1. 新建 `crates/solosoul-cli`，依赖 `solosoul-core`；
2. 实现 `solosoul` 入口、`clap` 参数解析、全屏 `ratatui` 启动/退出；
3. 实现底部自绘命令输入框与历史记录；
4. 实现 `/exit`、`/account_list`、`/doctor`；
5. 实现 `~/.solosoul/.lock` 进程锁。

### Phase 2：账户生命周期（2 周）

1. 首次启动引导创建账户（主密码强度校验、二次确认、密码提示词存储）；
2. 主密码登录（密码错误不泄露账户存在性）；
3. `/logout`、自动锁定、`Ctrl+L` 手动锁定；
4. 敏感输入组件、密码提示词显示；
5. macOS Touch ID / Face ID POC 验证（可选，不阻塞 MVP）。

### Phase 3：核心数据操作（3 周）

1. `/newpage`；
2. `/newobject` 三步向导；
3. `/list`、`/open`、`/edit`、`/delete`；
4. `/search` 搜索；
5. `/trash` 回收站查看与恢复/彻底删除；
6. 模板列表、字段编辑器；
7. 敏感度标签与掩码显示；
8. `/history`、`/rollback`；
9. `/export` / `/import` 数据导出导入。

### Phase 4：增强能力（3–4 周）

1. macOS Touch ID / Face ID 登录（若 POC 通过）；
2. 附件系统 `/attach*` 系列；
3. `/backup` / `/backup-restore` 备份与恢复；
4. `/model` / `/llm_config` / `/llm_chat` LLM 模型、配置与对话；
5. `/embed_model` 本地 Embedding 模型管理；
6. `/plugin` 插件系统接入（列表、运行、结果渲染）；
7. `/size` / `/status` 账户统计信息；
8. `/operation_log` / `/debug_log` 日志查看；
9. `/sync` 设备同步；
10. `/ocr` 本地图片 OCR；
11. `/setting` / `/security` / `/language` / `/theme` / `/about` / `/help` 系统与帮助命令；
12. `/doctor` 依赖检查；
13. `/update` 自更新（可选，需单独评估复杂度）。

### Phase 5：发布与文档（1–2 周）

1. CI 构建 `solosoul` 二进制；
2. 安装文档、快捷键文档；
3. 提供 `cargo install solosoul-cli` 与 Homebrew tap（macOS）两种分发方式；
4. 与 Tauri 客户端一起发布 Release。

---

## 14. 结论

SoloSoul CLI 终端版本具备**高可行性与明确的实现路径**。其最大优势在于可直接复用现有 Rust 后端的安全与存储能力，避免重复造轮子。CLI 不是 GUI 的降级版本，而是**同一核心上的另一套交互宿主**，必须在与 GUI 对齐的数据模型、安全策略、插件 ABI 基础上构建。

建议立即启动 **Phase 0（核心库抽取）**，通过「复制核心逻辑到 `solosoul-core`、保留 Tauri 薄包装层」的方式最小化对 GUI 的影响。在交互层面，建议以 **全屏 `ratatui` 为主界面**，底部自绘命令输入框；对于日期选择、单选/多选、外部编辑器等场景，采用「临时退出全屏 + `inquire`」的低成本方案。

生物识别为可选增强特性，MVP 以主密码登录为主；`/update` 自更新可延后实现。CLI 的推出将显著扩展 SoloSoul 的使用场景，进一步巩固其「本地优先、隐私优先」的产品定位。

---

## 附录 A：关键文件索引

| 文件 | 说明 |
|------|------|
| `tauri/Cargo.toml` | Workspace 定义 |
| `crates/solosoul-core/` | 共享核心逻辑（VaultService、生物识别） |
| `tauri/src-tauri/src/services/vault_service.rs` | 账户与 Vault 生命周期（Tauri 薄包装层） |
| `tauri/src-tauri/src/commands/auth.rs` | 登录/注册 Tauri Commands |
| `tauri/src-tauri/src/commands/biometric.rs` | 生物识别实现 |
| `tauri/src-tauri/src/commands/object.rs` | 对象 CRUD、回收站、快照 |
| `tauri/src-tauri/src/commands/template.rs` | 模板系统 |
| `tauri/src-tauri/src/commands/search.rs` | 搜索 |
| `tauri/src-tauri/src/commands/export_import.rs` | 导入导出 |
| `tauri/src-tauri/src/commands/attachment.rs` | 附件系统 |
| `tauri/src-tauri/src/commands/backup.rs` | 备份恢复 |
| `tauri/src-tauri/src/commands/sync.rs` | 设备同步 |
| `tauri/src-tauri/src/commands/plugin.rs` | 插件系统 Tauri Commands |
| `tauri/src-tauri/src/commands/llm.rs` | LLM 配置与对话 |
| `tauri/src-tauri/src/commands/ocr.rs` | OCR 识别 |
| `tauri/src-tauri/src/commands/log.rs` | 操作日志 |
| `tauri/src-tauri/src/commands/settings.rs` | 用户偏好设置 |
| `tauri/src-tauri/src/commands/system.rs` | 系统信息 |
| `tauri/src-tauri/src/plugin/` | 插件宿主与管理器（wasmtime） |
| `SoloSoul_plugin_market/SDK/rust/src/lib.rs` | 插件 Rust SDK 与 Host Functions ABI |
| `docs/wasm-plugin-development-guide.md` | WASM 插件开发指南 |
| `docs/design_map/09_对象规范.md` | 对象数据模型规范 |
| `docs/design_map/14_敏感度等级系统重构.md` | 敏感度系统规范 |
| `docs/design_map/16_回收站功能规范.md` | 回收站规范 |
| `docs/design_map/17_导入导出功能设计.md` | 导入导出规范 |
| `docs/design_map/21_生物识别功能规范.md` | 生物识别规范 |

## 附录 B：术语表

| 术语 | 说明 |
|------|------|
| Vault | 加密数据存储，每个账户对应一个 `vault.db` |
| Session Key | 主密码派生的 32 字节数据加密密钥，内存中临时保存 |
| Section / Page | 对象所属的分类页面，如「身份」「旅行」「财务」 |
| Template | 对象模板，定义字段集合与类型 |
| Sensitivity Level | 敏感度分级：public / internal / sensitive / critical |
| Host Functions | 插件 ABI，WASM 插件调用宿主的接口 |
| Consent | 插件读取用户字段前的显式授权 |
