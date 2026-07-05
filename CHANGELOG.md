# Changelog

## [2.5.11] - 2026-07-05

### Fixed

- **侧边栏悬停展开逻辑修复（水平模式）** — 水平模式下功能区与 AddPageButton 之间的 4px flex gap 导致鼠标穿过间隙时功能区误折叠。改用 hoverZone 外层包裹，彻底消除 gap 问题。
- **AI 对话卡片弹出位置修复** — 侧边栏左/右模式时 AI 对话卡片因本地 state 与 hook 内部 state 分离导致定位计算未触发，卡片始终渲染在回退位置。修复后卡片与按钮顶部对齐，并正确附加滚动/缩放监听器。
- **水平模式添加页面卡片右侧溢出修复** — 当侧边栏水平放置且功能区折叠时 "+" 按钮靠近窗口右边缘，弹出卡片右半部分超出视口。添加右侧溢出检测，自动左移卡片保证完整显示。

### Changed

- **恢复自定义 DatePicker + DropdownSelect** — 此前原生 `<select>`/`<input type=date>` 在 Tauri WebView 中存在兼容性问题（白屏、闪动），回退至自定义组件。

### Chores

- 版本号同步升级到 2.5.11。


## [2.5.10] - 2026-07-05

### Added

- **模板字段级契约角色绑定** — 字段级 `contractField` + `contractBindings`，插件安装时自动注册合约绑定。
- **expiry-guardian 插件重写** — typed contract + 自定义 UI，Passport/Visa/ID Card/银行卡系统模板绑定。
- **导入 KeepBoth 冲突策略** — 重复对象自动附加本地化后缀保留双方数据。
- **导入三级树预览** — 「页面 → 对象 → 附件」树形结构预览 + 附件级选择过滤。
- **导入内容哈希去重** — 按模板 properties 哈希自动去重。
- **导出全选/取消全选** — 导出区批量选择操作。
- **窗口状态持久化** — 使用 `plugin-window-state` 替代自定义 hook。

### Changed

- **Dialog → 原生 `<dialog>`** — 移除 93 行自定义实现。
- **ExpandableSection → 原生 `<details>/<summary>`** — 移除 137 行自定义折叠组件。
- **DatePicker/DropdownSelect → 原生 `input[type=date]`** — 移除 509 行自定义日期选择器。
- **合并 `rag.rs` + 删除废弃 `db/` 模块** — 减少 605 行死代码。
- **统一 AES 错误类型** — 用 `CipherError` 枚举替换 `String`。
- **Windows 导出密码校验移除** — 不再强制 8 位复杂度。

### Fixed

- **P0.5 架构审计 85 项全部完成** — 移除 ~3,120 行代码，减少 9 个依赖。
- **6 个 ESLint `no-explicit-any` 错误清零** — OcrPage/OcrSettingsPage/PluginResultPanel 全面修复。
- **2 个 React Hooks 顺序回归** — Dialog/PluginBadge early return 修复。
- **插件契约反序列化兼容** — `label`/`displayName` 支持 String 或 Object 格式。
- **导入创建初始 Snapshot** — 修复历史记录版本号显示问题。

### Refactored

- **P0.5 Phase 1-4 完成** — 死代码清理、CLI 去重、`solosoul-core` 下沉、`FieldResolver` 插件兼容性提升。
- **P2 前端重构** — `useCancellable` → `AbortController` 迁移，死代码内联。

### Chores

- 版本号同步升级到 2.5.10。
- 60 个 commit 自 v2.5.9。
- expiry-guardian 子模块更新。

## [2.5.9] - 2026-07-01

### Added

- **SafeMarkdown 安全组件** — 统一配置 XSS 防护，替换 ReactMarkdown 调用。

### Changed

- **PIN 审计日志独立** — 查看关键信息时记录 `critical_field_pin`，entityType 归入认证。
- **Windows Hello 日志一致** — 补齐审计日志映射与 `writeCriticalAccessLog` 分支。
- **对话框宽度** — PasswordVerificationDialog 缩窄至 360px，与登录卡片一致。

### Performance

- **PIN 解锁提速 70%** — KDF 参数统一后开发模式从 ~1472ms 降至 ~445ms。

### Fixed

- **PIN 关键字段展开** — PIN 解锁后关键字段被遮挡问题修复。
- **Windows COM 错误** — `ensure_mta()` 类型转换修复。
- **代码审计 P001-P005** — cargo fmt 9 文件、ESLint 清零、Prettier 178 文件、XOR key 注释、SafeMarkdown 加固。

### Chores

- 版本号同步升级到 2.5.9。

## [2.5.8] - 2026-07-01

### Added

- **Windows Hello 生物识别** — 新增 Windows Hello 支持。
- **PIN 快速解锁** — PIN 码快速解锁 vault，强因子认证后自动重置锁定。
- **新手引导增强** — 多步引导式帮助文档（PageGuide）与拖拽上传教程。

### Changed

- **登录优先级** — Face ID > Touch ID > Windows Hello > PIN > 主密码自动选择。
- **导出密码简化** — 任何非空密码均可导出，不再限制复杂度。
- **PinInput 重写** — 隐藏 input + 纯视觉盒子架构，消除光标偏移。

### Fixed

- **登录页面闪烁** — 全面消除锁定后登录页面闪烁问题。
- **PIN 竞态 & 导航卡死** — 修复并发 panic 与 vault 锁定后卡在连接界面。
- **白屏崩溃** — 移除 settings.json 残留注释导致的崩溃。
- **macOS 生物识别检测** — 修复 biometric 可用性检测逻辑。
- **Windows COM 初始化** — 修复 `BiometricError` 类型转换错误。

### Chores

- 版本号同步升级到 2.5.8。

## [2.5.7] - 2026-06-29

### Added

- **Attachment Watermark Plugin** — New plugin for adding text watermarks to images and PDFs, with customizable text, font, color, opacity, angle, positioning (center/corners/tile), output directory, and live preview.
- **Sensitive Field Redaction in Search** — Search results now redact field values marked as sensitive/critical, with triple-strategy protection: object-level, field-level (property_labels), and template-level fallback.
- **Template Name Search** — Searching by template name now includes all objects using that template in results, with match annotations.
- **Search Result Caching** — 30-second in-memory cache to reduce redundant search requests.
- **Help Search Enhancement** — Full-text content matching via pre-built search index, improved result ordering.
- **Settings Search Page Entry** — Added search page shortcut in settings system group.
- **Onboarding Animation** — Hover animations on skip/back buttons in tutorial cards.

### Security

- Enhanced attachment path validation, OCR scan path hardening, Windows icacls username validation
- Attachment rename character sanitization, conversation message limit (500)
- Multiple P0/P1 security fixes across attachment download, export paths, and state management

### Fixed

- 6 search bugs resolved (system page name translation, collection type filtering, critical object exclusion, etc.)
- Watermark plugin permission fixes (fs.copy_file, shell open scope, workspace path resolution)
- P2xx fixes: React hooks deps, localStorage consolidation, CLI import double-decryption, password change callback flattening
- PDF iframe preview restored via data URL
- ConfirmDialog i18n for cancel/confirm buttons
- Export/import template snapshot isolation with deterministic hashing
- Various code quality fixes: clippy, ESLint, dead code removal

### Performance

- LLM streaming output optimization (batch emit_typing_effect to 20-char chunks)
- Attachment N+1 query elimination via load_objects_batch VaultStore API
- RAG batch embedding improvements

### Changed

- Watermark plugin rewritten from custom PDF processing to pdfium-render + PDFium
- Full i18n support for watermark plugin (zh-CN/en-US)

## [2.5.6] - 2026-06-27

### Security

- **Streaming Encrypted Export/Import (P1-023/024)** — Large file encryption/decryption changed to chunked streaming, reducing memory usage and side-channel risk.
- **HKDF-SHA256 Password Verification (P2-010)** — Verification hash migrated from lightweight Argon2id to HKDF-SHA256, improving performance while maintaining security.
- **Windows ACL File Permissions (P1-002)** — Vault data directory permissions restricted via icacls.
- **Hardcoded Key Removal (P1-003)** — Removed hardcoded BIO_FILE_KEY_SECRET, replaced with key derivation.
- **Path Traversal Hardening (P2-012)** — User-controlled path validation in OCR and attachment processing.
- **OCR Swift Security Hardening (P2-014)** — Security fixes in OCR Swift code.
- **tauri.conf.json Schema Fix (P1-007)** — Schema points to official tauri-apps/tauri.
- **Plugin Runtime Double-Write Elimination (P1-009)** — Merged src-tauri/plugin/ into solosoul-plugin crate.
- **Global currentObject Singleton Replaced (P1-017/018)** — Per-objectId cache replaces global singleton.
- **Configurable KDF Parameters** — SOLOSOUL_SECURE=1 env var for production KDF parameters (64 MiB/3 iter).

### Fixed

- Recycle bin object detail "original location" display now shows page names instead of UUIDs
- Page switching history/attachment badge flickering fixed
- ObjectDetailModal/HistoryViewer/ExportSection flickering fixed with lazy loading + fade-in
- Attachment preview: non-image files, image scrolling/zoom fixed
- Copy button ghost background eliminated
- Attachment drag-and-drop penetration and event duplication fixed
- Template deletion resilience: property_labels, fields, templateName cleanup prevents crashes
- ESLint warnings cleared (P1-022)
- Clippy/Fmt fixes (P0-002/003/004)
- Frontend performance optimizations (P2-019~025)
- Search debounce and OCR document fixes (P0-009/P2-008/P2-011)

### Changed

- ObjectWorkspacePage refactored into useWorkspacePasswordGuard, ConfirmDeleteDialog, WorkspaceCategoryTabs
- P1 component extraction: BadgeIconButton, DeleteButton, SelectCheckbox; eliminated all miniBtn/pgBtn legacy usage
- Global button style unification: danger-outline variant, shared DeleteButton component
- Global font size unified to semantic tokens (--text-body, --text-caption, --text-badge, etc.)
- Global page layout standardization with PageContainer, CardGrid, tokens.css
- Sidebar attachment manager integration, export/import tab UI optimization
- Plugin copy button unified with theme-colored border and glow animation

### Added

- Attachment download: single and batch download via system save dialog
- Attachment batch selection UI with SelectCheckbox component
- CI/CD workflow: CLI checks, macOS/Windows builds, Release draft job

### i18n

- 81 new i18n keys including fs_is_dir, folder drag filtering, attachment downloads
- HistoryPage and GlobalAttachmentManager full i18n support
- Address formatter country badge localization
- Plugin copy button text i18n

## [2.5.5] - 2026-06-24

### Fixed

- **Attachment Preview File Open** — New Rust command `attachment_open` bypasses shell plugin URL restrictions.
- **Image Preview Scroll/Zoom** — AttachmentPreviewOverlay now scrollable with zoom toolbar.
- **Address Formatter Country Badge i18n** — DEFAULT country code shows localized "默认"/"Default".

### Changed

- Global page layout standardization with PageContainer, CardGrid, tokens.css
- Attachment management page font unified to typography tokens
- 125 files, ~591 hardcoded fontSize values mapped to semantic tokens

### Added

- Attachment download: single file and batch via system save dialog
- Attachment save dialog filter removed (fixes automatic .* suffix bug)

## [2.5.4] - 2026-06-24

### Added

- Attachment batch operations: multi-select, batch delete/restore
- Plugin adaptive key truncation
- Plugin country badge localization
- Sidebar plugin result UI unification with PluginResultPanel
- OCR MRZ template matching Strategy C

### Fixed

- Attachment/badge overflow: 99+ display for large counts
- 10+ plugin UI fixes (badge layout, key/value truncation, log defaults, Toast dedup)
- ESLint warnings (E001-E007) and Clippy warnings (P009-P010)

### Refactored

- Shared collapsible plugin component extraction
- OCR function rename: `locate_mrz_region_flutter` → `locate_mrz_region`

## [2.5.3] - 2026-06-21

### Added

- Sidebar function button area collapse/expand (SecondaryActionBar, TopFunctionBar)
- Lock/Settings buttons fixed outside collapsed area
- Zustand store persistence for expanded/scroll state
- Mouseleave fallback for auto-collapse

### Fixed

- Collapse arrow direction in horizontal mode
- Bottom sidebar horizontal collapse display
- Expanded button clickability (onTransitionEnd fallback via useEffect + setTimeout)
- Cross-page navigation scroll flicker (useLayoutEffect)
- LoginPage biometric flicker

## [2.5.2] - 2026-06-21

### Added

- Sidebar plugin quick panel with All/Installed/Running tabs
- Sidebar button card/page mode switching (OCR, Plugins, AI Chat, Search)

### Fixed

- Auth error condition ordering (password length check priority)
- Plugin registry/install downgrade fixes

## [2.5.1] - 2026-06-20

### Added

- Plugin registry background auto-refresh with 1-hour rate limit
- Manual registry refresh button
- Remote plugin installation
- Registry cache separated to writable app data directory
- Address formatter country badges
- Icon category i18n

## [2.5.0] - 2026-06-20

### Added

- Plugin system officially opened with built-in address formatter plugin
- Home page quick plugin entry
- Plugin result/log section collapse/expand
- Toast i18n for plugin notifications
- Copy button feedback with border glow

## [2.4.1] - 2026-06-19

### Added

- Plugin template system Stage 3: v17 migration idempotency + partial DB state tests

### Changed

- Large component split Round 2 (P056-P059): SideNavigation, AiQuickChatPopover, LlmConfigPage, OcrQuickScanPopover
- Large component split Round 1 (P052-P055): TrashPage, LlmChatPage, TemplateManagerPage, ExportImportPage
- Code audit P060-P062 fixes

## [2.3.3] - 2026-06-18

### Added

- Plugin template system Stage 1: schema + v17 migration
- Windows silent auto-update
- CLI settings phase: /setting command → SettingsMenu
- CLI extension commands: /sync, /ocr, /embed_model

## [2.3.2] - 2026-06-17

### Added

- Home page quick action cards (trash, OCR, search, AI chat)
- Auto-update download progress bar
- macOS updater archive generation (.app.tar.gz)
- Unified signing script (docs/sign_artifacts.sh)
- PDFium auto-download

## [2.3.1] - 2026-06-17

### Added

- OCR PDF/MRZ extension: scan PDF documents and machine-readable zones
- Auto-update signature integration
- Local model file pre-check in build scripts

## [2.3.0] - 2026-06-17

### Added

- Local OCR engine (PP-OCRv6) with tiny/small/medium models
- OCR first-run silent model install
- macOS biometric improvements with Keychain UserPresence
- SoloSoul CLI Phase 5: plugin system and LLM capabilities
- Sync service cross-binary reuse

## [2.2.2] - 2026-06-16

### Added

- Windows crash investigation infrastructure (file logging, panic capture, pre-flight checks)
- Plugin market graceful degradation
- SoloSoul CLI Phase 4 M1+M2 enhancements
- Code audit zero: all 47 items resolved

## [2.2.1] - 2026-06-15

### Added

- SoloSoul CLI Phase 0-5 full delivery: account management, Vault read/write, attachments, backup, import/export, settings, security
- CLI core library extraction into solosoul-core crate
- CLI TUI with ratatui 0.30.1
- CLI command system with 28+ commands

## [2.2.0] - 2026-06-14

### Added

- End-to-end device sync (HLC / Noise XX / CRDT)
- Plugin system Phase 1-4: lifecycle, field reading, consent, result export, registry updates
- Template management: filtering and search
- Help docs: device sync section rewritten

## [2.1.0] - 2026-06-12

### Added

- New user onboarding tutorial
- In-app update detection
- NSIS branded installer
- Template example library (8 system templates)
- Critical field default sensitivity
- Extended icon system
- Audit log: login, biometric, critical field access
- Window size persistence

## [2.0.1] - 2026-06-12

### Fixed

- macOS Bootstrap Page input (WebKit password manager bypass)
- Password change command routing
- Password hint update after password change

## [2.0.0] - 2026-06-12

### Changed (Major)

- **Complete Rewrite: Flutter → Tauri v2** — Rust/Tauri backend with React/TypeScript frontend, better performance, smaller binary (~50MB vs ~200MB)

### Added

- Object CRUD, Template System, History System, Trash/Recycle Bin
- Search, AI Chat, Attachment System, OCR, Local Scan
- Template Manager, Export/Import, Biometric Unlock
- Privacy Policy & Terms of Service
- Operation Log, Vault Stats Dashboard

### i18n

- Complete i18n foundation with en-US/zh-CN locales across all pages

