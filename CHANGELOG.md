# Changelog

All notable changes to SoloSoul are documented in this file.

## [Unreleased]

## [2.5.6] - 2026-06-27

### Fixed

- **回收站对象详情卡片"原位置"显示** — 自定义页面的 UUID 改为显示页面名称（使用 resolveCollectionLabel + useSettingsStore 解析），同时修复国际化。
- **回收站页面时间筛选切换闪烁** — 移除加载覆盖层（Loader2），旧数据在加载期间保持不变，新数据到达后无缝替换。
- **切换页面历史/附件 badge 数字闪烁** — 移除 snapshot counts 0 初始化，加载期间 badge 不显示，数据到达后统一显示实际数字。
- **快照区域横线调整** — 折叠时 toggle 下方显示横线，展开时由最后一个字段的 borderBottom 自然作底部边界，避免双线重叠。
- **ConfirmDialog 事件冒泡** — 修复覆盖层点击 e.stopPropagation()，防止点击 dismiss 时意外关闭附件卡片。
- **附件文件名 .* 后缀 bug** — 移除 save() 对话框的 filters 配置，避免系统对话框自动给文件名追加 .*。
- **附件拖拽穿透与事件重复** — 修复拖拽上传时多层元素穿透、计数刷新不及时、Tauri 重复事件去重问题。

### Changed

- **回收站恢复按钮无边框** — variant="secondary" + accent 边框 -> variant="tertiary"（无边框，hover 浅色底）。
- **回收站底部按钮右对齐** — 恢复和永久删除按钮容器添加 justifyContent: flex-end，垂直位置不变。
- **danger-outline 按钮样式强化** — 常态显示浅红底 + 亮红文字，hover 时更深背景。
- **提取共享 DeleteButton 组件** — 附件管理批量删除等场景统一使用，减少重复代码。
- **全站字体大小统一为语义化 token** — 新增 --text-body、--text-caption、--text-badge 等语义字号 token。
- **全站页面布局标准化** — 新增 PageContainer 共享容器组件、CardGrid 卡片网格组件与 tokens.css 布局/排版 token。
- **附件 UI 统一** — TrashPage 附件标签按钮改为 pill 样式；附件图标和格式标签颜色统一为 var(--text-tertiary)。
- **全站卡片 gap 统一** — 使用 --card-grid-gap token 布局。
- **ConfirmDialog 字体/按钮对齐** — 标题/正文字号与对象删除确认框对齐。

### Added

- **附件下载功能** — AttachmentViewer 和 GlobalAttachmentManager 支持单个附件下载（系统 save 对话框）和批量下载（目录选择器）。新增 i18n key。
- **附件批量选择 UI** — 提取 SelectCheckbox 组件，支持 indeterminate 状态，批量工具栏始终可见。

### i18n

- **81 条新 i18n key** — 包括 fs_is_dir、文件夹拖拽过滤、cannot_open_file 等。
- **HistoryPage 和 GlobalAttachmentManager 国际化** — Toast 消息全面支持中英文。

### Chores

- 版本号同步升级到 2.5.6.


## [2.5.5] - 2026-06-24

### Fixed

- **附件预览无法打开文件** — 新增 Rust 命令 `attachment_open`，绕过 `tauri-plugin-shell` 默认仅允许 `http(s)/mailto/tel` 的 open 校验，直接从 Vault attachments 目录调用系统默认应用打开非图片附件，并校验路径必须在 Vault 存储范围内。
- **图片附件预览无法滚动和缩放** — `AttachmentPreviewOverlay` 改为可滚动容器，支持按原始尺寸滚动浏览；新增底部缩放工具栏（放大 / 缩小 / 适应窗口）以及 Ctrl/Cmd + 滚轮缩放。
- **地址格式化器国家徽章国际化** — `PluginResultPanel` 将插件返回的 `DEFAULT` 国家代码识别为“未检测到国家”，中文显示 `默认`，英文显示 `Default`。

### Changed

- **全站页面布局标准化** — 新增 `PageContainer` 共享容器组件、`CardGrid` 卡片网格组件与 `tokens.css` 布局/排版 token（`--page-max-width*`、`--section-gap`、`--card-grid-*`、`--text-page-title` 等）。首页保持 720px 作为内容/列表页标准，设置/回收站/搜索统一 600px，编辑器/同步/LLM 配置统一 560px，表单/设置详情页统一 480px。20+ 页面从写死 `maxWidth` 改为引用 token，以后改一处即可统一全站宽度、间距与字号。
- **附件管理页面字体统一** — `GlobalAttachmentManager` 内所有写死 `fontSize`（10/11/12/13px）替换为排版 token（`--text-2xs`、`--text-xs`、`--text-body-sm`），与全站 token 保持一致。
- **全站字体大小统一为语义化 token** — 新增 `--text-body`、`--text-caption`、`--text-badge` 等语义字号 token；通过 codemod 将 125 个文件、约 591 处写死 `fontSize` 像素值按“首页层次”映射为 token（24→`--text-xl`、20→`--text-page-title`、18→`--text-md`、16→`--text-section-title`、15→`--text-card-title`、14→`--text-body`、13→`--text-body-sm`、12→`--text-caption`、11/10/9→`--text-badge`）。

### Added

- **附件下载功能** — AttachmentViewer 和 GlobalAttachmentManager 支持单个附件下载（系统 `save` 对话框）和批量下载（目录选择器），点击后复制 Vault 文件到用户选择的目标路径。新增 i18n key：`download_result`、`download_failed`、`batch_download_result`、`select_download_directory`。
- **附件下载文件名 bug 修复** — 移除 `save()` 对话框的 `filters` 配置，避免系统对话框自动给文件名追加 `.*` 后缀。

### Fixed

- **附件文件名 `.*` 后缀 bug** — 移除 `save()` 对话框的 `filters: [{name:'All Files', extensions:['*']}]`，避免系统对话框自动给文件名追加 `.*`。
- **附件拖拽穿透与事件重复** — 修复拖拽上传时多层元素穿透、计数刷新不及时、Tauri 重复事件去重问题。
- **回收站附件按钮顺序** — 附件卡片回收站批量工具栏恢复按钮与永久删除按钮对调。
- **回收站附件删除线** — 软删除附件行仅文件名有删除线，图标、大小日期、格式标签正常显示。
- **代码审查修复** — useDragToAttach 错误处理、清除过期 console.error。

### Changed

- **附件 UI 统一** — TrashPage 附件标签按钮改为 pill 样式；附件图标和格式标签颜色统一为 `var(--text-tertiary)`，消除 PDF 红色、图片强调色的差异。

### i18n

- **81 条新 i18n key** — 包括 `fs_is_dir`、文件夹拖拽过滤、`cannot_open_file` 等。
- **HistoryPage 和 GlobalAttachmentManager 国际化** — Toast 消息全面支持中英文。
- **`dialog_subtitle` 修复** — 从空字符串改为正确的占位符文本。

### Chores

- 版本号同步升级到 2.5.5。
- `cargo fmt` 格式化及代码清理。


## [2.5.4] - 2026-06-24

### Added

- **附件批量操作** — 附件管理页面支持批量选择、批量删除与恢复，抽离 `useBatchSelect`/`useAttachmentPageSort` hooks，支持附件重命名。
- **插件自适应 Key 截断** — 插件结果区 Key 不再硬编码 3em max-width，改为根据内容自适应截断，避免长 Key 过早换行。
- **插件国家徽章本地化** — 地址格式化器结果中的国家徽章根据界面 locale 显示本地化名称（中文环境显示「中国」，英文显示代码「CN」）。
- **侧边栏插件结果 UI 统一** — `QuickRunningInfo` 改用 `PluginResultPanel`，与插件页面共享同一套结果渲染组件。
- **OCR MRZ 模板匹配 Strategy C** — 新增 MRZ 模板匹配策略 C，改进 `split_mrz_lines` 方法，提升 MRZ 识别准确率。

### Fixed

- **附件/快照徽章溢出** — 数量超过 99 时显示 `99+`，避免徽章宽度溢出。
- **插件 UI 修复集** — 修复徽章布局（已安装始终可见、Completed 使用绿色）、Key/Value 截断与换行、侧边栏结果区 UI 与插件页面对齐、日志默认展开、Toast 重复防止等 10+ 项 UI 问题。
- **ESLint 问题修复** — 解决 E001-E007 共 7 个 ESLint 警告。
- **Clippy 警告修复** — 解决 P009-P010 共 11 个 OCR 代码中的 clippy 警告。

### Changed

- **版本号统一** — 全平台版本号同步升级到 `2.5.4`。

### Refactored

- **插件共享可折叠组件提取** — 提取统一的可折叠日志/结果组件，消除插件页面与侧边栏的重复实现。
- **OCR 函数重命名** — `locate_mrz_region_flutter` 重命名为 `locate_mrz_region`，清除 Flutter 遗留引用。

### Chores

- 移除遗留的 `CODE_ANALYSIS_REPORT.md`。
- 更新过时注释，清理旧死代码。

## [2.5.3] - 2026-06-21

### Added

- **侧边栏功能按钮区折叠展开（SecondaryActionBar）** — 垂直模式下 hover 折叠区域箭头展开/离开自动折叠，支持 200px 可滚动按钮区，隐藏滚动条。展开过渡期间 `pointer-events: none` 防止 tooltip 闪烁。
- **侧边栏功能按钮区折叠展开（TopFunctionBar）** — 水平模式（top/bottom）同样支持 hover 展开/折叠，使用 ChevronRight 箭头指示方向，按钮区水平滚动。
- **Lock/Settings 按钮固定在折叠区外部** — 锁定账户和设置按钮始终固定在侧边栏底部（垂直模式）或功能条右侧（水平模式），不受折叠影响。
- **Zustand store 持久化展开/滚动状态** — `useSidebarHoverStore` 跨页面导航持久化 `isHovering`、滚动条位置（`verticalScrollTop`/`horizontalScrollLeft`），消除路由切换时状态丢失。
- **`document.documentElement` mouseleave 兜底** — 鼠标离开窗口时自动折叠功能区域。

### Fixed

- **折叠区域箭头方向** — 水平模式（top/bottom）箭头方向翻转：折叠时显示 ChevronLeft（←），展开时显示 ChevronRight（→），符合用户直觉。
- **底部（bottom）侧边栏不显示水平折叠** — `AppShell.tsx` 将 `{isTop ? <TopFunctionBar /> : <SideNavigation />}` 改为 `{isHorizontal ? ...}`，bottom 模式正确渲染 TopFunctionBar 并添加 `paddingBottom`。
- **底部 bar 位置错误** — TopFunctionBar CSS `position: fixed; top: 0` 覆盖了 flex 布局，新增动态 `bottom: 0` + `borderTop`。
- **展开后按钮无法点击** — `onTransitionEnd` 在 React 重渲染时可能不触发导致 `pointer-events: none` 永久生效，改为 `useEffect` + `setTimeout`（200ms）兜底。
- **卡片/页面按钮导航后折叠** — 关闭卡片或点击页面按钮后，如果鼠标仍在功能区域内保持展开。
- **跨页面导航滚动条闪烁** — `useLayoutEffect` 替代 `useEffect` 消除 `scrollTop`/`scrollLeft` 恢复时的视觉闪烁。
- **Settings 按钮无高亮状态** — 添加 `isActive={location.pathname.startsWith('/settings')}`。
- **LoginPage 生物识别闪烁** — 移除早期返回 bare `<div>`，始终渲染完整卡片布局，bio check 期间显示 Loader2 加载动画。
- **展开过渡期间 name card 闪烁** — 按钮容器在过渡期间 `pointer-events: none` 防止 tooltip 随移动按钮闪烁。

### Changed

- **版本号统一** — 全平台版本号同步升级到 `2.5.3`。
- **按钮顺序调整** — 功能按钮区顺序为：搜索→回收站→模板管理→插件→OCR→导入导出→帮助文档→AI 对话。
- **底部按钮自定义配置移除** — `sidebar_bottom_actions` 相关配置项删除，按钮改为固定 8 个功能按钮 + Lock + Settings。

## [2.5.2] - 2026-06-21

### Added

- **侧边栏插件快捷面板** — 侧边栏点击插件按钮弹出浮动卡片，支持「全部/已安装/运行中」三个标签页，内联安装/卸载/运行操作，日志和结果内联展示，关闭卡片后后台运行不受影响。
- **侧边栏按钮卡片/页面模式切换** — 为 OCR 扫描、插件、AI 对话、搜索 4 个按钮提供可配置的卡片/页面两种模式，用户可在「主题与外观」设置中自由切换，默认均为卡片模式。

### Fixed

- **Auth 页面错误条件顺序** — 修复 BootstrapPage 和 LoginPage 中后端返回密码长度错误时被错误归类为「密码不正确」的问题，'8 characters' 检查现在优先于 'password' 通用检查。
- **插件注册表/安装降级修复** — 修复注册表更新和安装回退场景下的 2 个 bug。

### Changed

- **版本号统一** — 全平台版本号同步升级到 2.5.2。

## [2.5.1] - 2026-06-20

### Added

- **插件注册表后台自动刷新** — 应用启动时后台自动调用 `update_registry`，带 1 小时速率限制，失败不阻塞启动。
- **插件手动刷新按钮** — 插件仪表盘页新增手动刷新注册表按钮，带加载旋转动画。
- **远程插件安装** — `install_from_registry` 现在从远程下载最新的 manifest + WASM，使插件更新与软件版本真正分离。
- **注册表缓存分离** — `registry.json` 写入可写应用数据目录（`~/.solosoul/`），而非只读资源目录。
- **地址格式化器国家徽章** — 插件结果中每条地址左侧显示国家徽章（中文环境显示国家名如「中国」，英文显示代码如「CN」）。
- **图标分类国际化** — 侧边栏新建页面、重命名页面、新建模板的图标选择器分类名支持中英文切换。

### Changed

- **版本号统一** — 全平台版本号同步升级到 `2.5.1`。
- **已安装插件存储分离** — WASM 文件存储路径从 `PluginStore` 独立为应用数据目录，bundled 资源仅作离线回退。
- **默认侧边栏按钮** — 新用户默认侧边栏底部按钮调整为「搜索」「模板管理」「帮助文档」；检测到旧默认值自动迁移。
- **地址格式化器 v1.0.5** — 日志与结果支持国际化（通过 `locale` 参数）。

### Fixed

- **模板详情卡片图标** — `TemplateDetailModal`、`SampleTemplateDetail`、`TemplateEditor` 的字段类型图标统一使用 `FieldTypeIcon` 组件（Lucide 图标），与对象编辑器保持一致。
- **PluginCard 国际化** — 日志级别、JSON/Markdown 标签、复制按钮支持中英文。
- **pluginStore Toast 错误处理** — 插件运行失败时显示红色错误 Toast。
- **BootstrapPage 密码提示** — 两次密码不一致时显示错误提示。
- **BootstrapPage 密码不匹配** — 修复 BootstrapPage 中密码验证不一致的问题。

## [2.5.0] - 2026-06-20

### Added

- **插件系统正式开放** — 插件系统对发布版本开放，已内置"地址格式化器"插件，支持10个国家/地区的地址规范格式化输出。更多插件将在后续版本中发布。
- **首页插件快捷入口** — 在首页「快速入口」区域新增插件卡片，一键跳转插件管理页面。
- **插件结果区折叠展开** — 插件结果区与日志区统一支持折叠/展开，默认展开，方便管理多插件输出。
- **Toast 国际化支持** — 插件运行完成的 Toast 通知支持中英文切换。
- **复制按钮反馈优化** — 所有复制按钮（日志、JSON、Markdown、逐条地址）统一添加主题色边框与柔光效果反馈。

### Changed

- **版本号统一** — 全平台版本号同步升级到 `2.5.0`。
- **插件 UI 对齐** — 「运行中」tab 的结果卡片样式与「已安装」tab 统一，移除旧式分离输出卡片。
- **发布版本插件过滤** — 生产环境仅显示地址格式化器，开发/调试模式始终显示全部插件。
- **Sidebar 底部按钮 i18n** — 英文设置下"Position N"改为"Button N"，侧边栏底部"Plugins"选项正确显示国际化文本。

### Fixed

- **PluginGatePage 阻挡** — 移除了生产环境下的"插件系统开发中"门禁页，插件页面始终可用。
- **「全部」tab 运行插件跳转** — 在「全部」tab 点击运行后自动切换到「已安装」tab，避免出现旧样式结果卡片。
- **导航翻译缺失** — 补充 `navigation:plugins` 翻译键，侧边栏底部按钮选择器中的 Plugins 选项正确显示中英文。

## [2.4.1] - 2026-06-19

### Added

- **插件模板系统 Stage 3 — v17 幂等性 + 部分 DB 状态测试** — 为 Stage 1 落地的 v17 migration 在 `solosoul-vault/src/migration.rs::tests` 内新增两条验收测试:
  - `test_migration_v17_idempotent_run_twice` —— 两次顺序执行 `run_migrations()` 后断言 `schema_migrations.version = 17` 仅 1 行,证实 gate `current < 17` 在第二次调用中正确 skip;且两表 `contract_type_id` 均 `notnull = 0` 且 `(dflt_value IS NULL OR dflt_value = '')` (Option B 合约 — NULL 与空串都是「无合约」合法表述)。
  - `test_migration_v17_partial_state` —— 4 个阶梯 mod 内 for-loop 覆盖 `(user_templates.contract_type_id, objects.contract_type_id)` 四种「有/无」组合: 双列均无 (fresh install), 仅 `user_templates` 有, 仅 `objects` 有, 双列已有 (no-op path)。每个阶梯结尾都验证两表都有列 + `schema_migrations.version=17` 仅 1 行 + `data_version == 17`。
  - 新增 `setup_v16_partial_state(has_utpl_ctid, has_objects_ctid) -> (Connection, TempDir)` 辅助,内嵌 `HELPERS_PARTIAL_V16_SQL` 常量与 `/*UTPL_CTID*/` / `/*OBJECTS_CTID*/` 站位标记,以 `.replace()` 条件投送 `contract_type_id TEXT,` 行为。

### Changed

- **版本号统一** — 全平台版本号同步升级到 `2.4.1`。
- **大组件拆分 Round 2 (P056–P059)** — 拆分 4 个大型前端组件为独立子组件，降低维护成本：
  - `SideNavigation.tsx` (1026 行) → `PrimaryNavZone` / `SecondaryActionBar` / `RenameableNavButton` / `AddPageButton`
  - `AiQuickChatPopover.tsx` (1009 行) → `ChatMessageList` / `ChatInputBar` / `ConversationHistory` / `UnconfiguredHint`
  - `LlmConfigPage.tsx` (929 行) → `AiFeaturesCard` / `SystemPromptCard` / `ProviderManagerPanel` / `LocalEmbeddingsPanel` / `KnowledgeBaseCard` / `RiskAcceptanceDialog`
  - `OcrQuickScanPopover.tsx` (789 行) → `OcrPopoverHeader` / `OcrHistoryTrashDropdown` / `OcrScanControls` / `OcrResultPanel`
- **大组件拆分 Round 1 (P052–P055)** — 拆分 `TrashPage`、`LlmChatPage`、`TemplateManagerPage`、`ExportImportPage` 为独立子组件与自定义 hook。
- **代码审计 P060–P062** — 修复 `lib.rs:491` `.expect()` 为优雅退出；提取 `TemplateTypeSelect`、`TemplatePageSelect`；提取 `useExportEstimate` hook 替代复杂 `useMemo`。`CODE_ANALYSIS_REPORT.md` 全部 20 项 P2 问题已修复。
- **代码格式化清理** — 对 `migration.rs` 中 Stage 3 测试区块与 Stage 1 落库时 `lib.rs` 中 `#[serde(...)]` 属性的 rustfmt 漂移进行规范化 (`cargo fmt --all`)。

### Fixed

- **`cargo clippy --workspace --all-targets -- -D warnings` 警告** — 修复 `migration.rs::tests::setup_v16_partial_state` 中 `let mut conn = Connection::open(...)` 的 `unused_mut` 警告 (`Connection::open` / `execute_batch` / `set_schema_version` 均只需 `&self`)。

## [2.3.3] - 2026-06-18

### Added

- **插件模板系统 Stage 1** — 新增插件兼容模板 Schema 与数据库 v17 migration，为插件市场模板化能力奠定基础。
- **Windows 静默自动更新** — 启用 Windows 端静默下载与安装更新，减少用户手动干预。
- **CLI 设置阶段重构** — `/setting` 命令从原始卡片替换为 `SettingsMenu`，提升终端设置体验。
- **CLI 扩展命令** — 新增 `/sync`、`/ocr`、`/embed_model` 命令；CLI 支持 `--ocr scan --mrz` 等参数，并补齐 CLI Release 产物构建。
- **模板库过滤优化** — 过滤时保持卡片高度稳定，并将可见卡片优先排列。
- **OCR 历史记录交互** — 点击 OCR 历史条目可直接加载扫描结果；补充 `active_model_series` 国际化并显示模型系列名称。
- **侧边栏搜索激活态** — 搜索弹窗打开时，侧边栏搜索导航按钮显示为激活状态。

### Changed

- **日志降噪** — 在 `npm run tauri dev` 期间静默 ONNX Runtime（`ort` crate）的 INFO 级别日志 spam。
- **代码格式化清理** — 对 `llm/service.rs`、`cipher.rs` 等文件进行 rustfmt 规范化整理。

## [2.3.2] - 2026-06-17

### Added

- **首页快捷入口** — 为回收站、OCR、搜索、AI 聊天添加快速访问卡片，优化高频功能入口。
- **自动更新下载进度** — 更新横幅显示下载进度，支持后台下载更新。
- **macOS 更新包生成** — Release 构建脚本生成 `.app.tar.gz` 供 Tauri updater 使用，替代 DMG 作为更新载体。
- **统一签名脚本** — 新增 `docs/sign_artifacts.sh`，在 macOS 上为 Windows `.exe` 和 macOS `.app.tar.gz` 统一生成 updater 签名，避免在 Windows 构建机暴露私钥。
- **PDFium 自动下载** — Release 构建脚本在缺失 PDFium 动态库时自动下载对应平台库。

### Changed

- **帮助入口调整** — 将帮助卡片从数据区移至快捷入口区。
- **快捷入口位置** — 将快捷入口区域调整到数据区下方，优化信息层级。
- **Release 签名策略** — `.sig` 签名文件不再上传到 GitHub Release，仅嵌入 `latest.json`。

### Fixed

- **帮助页面国际化** — 修正帮助页面标题的 i18n key。
- **快捷入口返回行为** — 修复快捷访问目标页面的返回按钮无法正确回到首页的问题。
- **代码审计 P048-P051** — 修复 `llm/service.rs` 中的 `.expect()`、`decrypt_chunked_from_bytes` 中的 `.unwrap()`、`OcrQuickScanPopover` 死代码，以及前端 `react-hooks/exhaustive-deps` 警告；使用 `editingTemplateId` memo 避免 `TemplateManagerPage` 过度重渲染。
- **更新后重启** — 注册 `tauri-plugin-process` 并授予 `process:default` ACL，允许更新完成后自动重启应用。
- **macOS 更新包元数据** — 禁用 `tar` 生成 `._*` AppleDouble 文件，避免 Tauri updater 解压报错。
- **PDFium 下载路径** — 修正 Windows DLL 的下载源路径。

## [2.3.1] - 2026-06-17

### Added

- **OCR PDF/MRZ 扩展** — 支持扫描/解析 PDF 文档与机读区（MRZ），识别结果可导入为对象。
- **自动更新签名集成** — Release 构建脚本自动生成 `.sig` 签名文件，并生成 Tauri updater 所需的 `latest.json`。
- **本地模型文件前置检查** — macOS/Windows 构建脚本在编译前检查 OCR 与 Embedding 模型是否完整，避免打包失败。

### Changed

- **国际化补全** — NSIS 安装程序欢迎/完成页、OCR 模型选项、帮助文档标题支持多语言。
- **发布脚本增强** — 自动检测 `~/.tauri/secret.key`，按版本号过滤产物生成 `latest.json`，补充签名密钥管理文档。

### Fixed

- **OCR 侧边栏快速扫描** — 修复快速扫描卡片阻塞性 bug 与相关体验问题。
- **NSIS 模板语法** — 修复内联 i18n 注释中的 Handlebars 转义问题，避免安装包构建异常。
- **签名路径** — 修复 macOS/Windows 构建脚本在 `tauri` 子目录中调用 `tauri signer` 时相对路径错误的 bug。

## [2.3.0] - 2026-06-17

### Added

- **本地 OCR 引擎（PP-OCRv6）** — 集成 ONNX Runtime 本地文字识别，支持 tiny/small/medium 三档模型；扫描图片/附件后可导入为对象，并补充测试、文档与审计日志。
- **OCR 首次启动静默安装** — 新账户首次启动时自动在后台下载并安装 small 模型，避免用户首次扫描时无模型可用。
- **macOS 生物识别改进** — 支持 Touch ID 凭证迁移到 Keychain UserPresence；修改主密码后自动更新生物识别凭证，无需重新启用。
- **SoloSoul CLI Phase 5 插件系统** — 终端完整支持插件生命周期：`/plugin_install`、`/plugin_update`、`/plugin_uninstall`、`/plugin_run`（支持 `key=value` 运行时参数，后台线程异步运行 Wasm 插件）、`/plugin_search`、`/plugin_sessions`，以及插件列表内联过滤、批量快捷键与详情屏幕。
- **SoloSoul CLI Phase 5 LLM 能力** — 终端新增 LLM 配置、对话、聊天与统计命令，核心能力下沉到 `solosoul-core`。
- **生物识别实现规范** — 新增 `docs/biometric-spec.md`，记录 macOS Touch ID 与 Windows Hello 实现约束及回退策略。
- **同步服务跨二进制复用** — 将 `SyncService` 移入 `solosoul-sync` crate，供 GUI 与 CLI 共享。

### Changed

- **版本号统一** — 全平台版本号同步升级到 `2.3.0`。
- **扫描页模型描述** — 模型系列改为变量管理，描述不再硬编码模型名，便于后续扩展。
- **OCR 无模型提示** — 未安装模型时触发扫描，改为弹出国际化提示引导用户下载。
- **macOS 标题栏与 UI 修复** — 优化安全警告、国际化文案、NSIS 资源与 macOS 标题栏行为。
- **Updater ACL 权限** — 授予 `updater:default` ACL 权限，允许客户端正常下载并安装更新。

### Fixed

- **生物识别 Touch ID 解锁失败** — 修复修改密码或锁定后 Touch ID 无法解锁的问题。
- **生物识别 Keychain 回退** — 开发模式下 Keychain `-34018` 或 `not_found` 时自动回退到本地文件存储，避免反复弹出钥匙串输入框。
- **生物识别凭证迁移** — 修复旧账户生物识别迁移与关闭失败、本地凭证文件误删等问题。
- **生物识别取消提示** — 修复生物识别测试取消时错误使用国际化消息的问题。
- **Windows 构建失败** — 将 `macos_keychain` 模块限制为仅 macOS 编译，修复 Windows 构建时找不到 `core_foundation` / `security_framework` 的问题。
- **NSIS i18n.nsh 打包失败** — 将 `i18n.nsh` 内容内联到 `installer.nsi`，避免 Windows 构建时因未复制该文件导致 `could not find: "i18n.nsh"` 错误。

## [2.2.2] - 2026-06-16

### Added

- **Windows 启动崩溃排查基础设施** — 增加文件日志、panic 捕获、启动前检查（WebView2 / 数据目录 / 资源目录），便于定位安装后双击闪退。
- **插件市场缺失优雅降级** — 当打包资源中 `SoloSoul_plugin_market` 缺失时不再 panic，记录警告后继续启动。
- **SoloSoul CLI Phase 4 M1+M2 增强能力** — 终端 UI 与命令体系进一步增强。
- **代码审计清零** — `CODE_ANALYSIS_REPORT.md` 全部 47 项问题已修复或确认关闭。

### Changed

- **版本号统一** — 全平台版本号同步升级到 `2.2.2`。
- **大组件拆分** — `LlmChatPage` 提取 `ChatMessageBubble`，`SideNavigation` 提取 `AiQuickChatPopover` 独立文件。
- **共享工具函数** — `formatTimestamp` / `formatRelative` 提取到 `src/lib/time.ts`，复制反馈时长与 debounce 延迟提取到 `src/lib/constants.ts`。

### Fixed

- **Windows 安装后闪退** — 修复在部分 Windows 环境下安装包安装成功但双击启动即崩溃的问题。
- **代码审计剩余 16 项** — 完成 P004、P019-P022、P029、P030、P033、P034、P036、P041-P046。
- **原生对话框替换** — 替换 4 处原生 `alert()` / `confirm()` 为项目统一的 `ConfirmDialog`。
- **生产日志清理** — 移除 `App.tsx` onboarding 路径的 `console.warn`。
- **Clippy 警告** — 修复 `lib.rs` 中的 `redundant_closure` 警告。

## [2.2.1] - 2026-06-15

### Added

- **SoloSoul CLI Phase 0–5 完整交付** — 独立终端 TUI 客户端达到与 GUI 一致的核心能力：账户创建/登录/自动锁定、Vault 只读与写入命令、对象向导、附件/备份/导入导出/设置/安全子命令。
- **CLI 核心库抽取** — 新增 `solosoul-core` crate，迁移 `VaultService`、模板、生物识别、密码验证核心逻辑，供 GUI 与 CLI 共享。
- **CLI 终端 UI** — 基于 `ratatui` 0.30.1 实现全屏 TUI，包含命令输入框、屏幕渲染、自动补全、字段编辑器、模态提示与密码输入框。
- **CLI 命令体系** — 支持 `/unlock`、`/lock`、`/list`、`/open`、`/size`、`/search`、`/history`、`/rollback`、`/newpage`、`/newobject`、`/edit`、`/delete`、`/trash`、`/restore`、`/purge`、`/operation_log`、`/export_log`、`/attach`、`/backup`、`/export`、`/import`、`/language`、`/theme`、`/setting`、`/security`、`/debug_log`、`/about`、`/help`。
- **CLI 品牌视觉** — 登录页/首页采用暖色仪表盘风格，新增 Codebuff 风格 Unicode Logo、FIGlet banner、扫光动画与 GUI 品牌蓝色主题。

### Changed

- **CLI 登录体验** — 账号名/选择器始终置顶，生物识别/密码选项改为可点击卡片，禁用 emoji 并加入鼠标悬停动画。
- **CLI 命令补全** — 根据当前阶段动态过滤命令，未解锁时隐藏登录后命令，向导内仅提供安全命令。
- **CLI 日志与锁定** — 日志写入 `{data_dir}/logs/cli.log`，5 分钟无操作自动锁定 Vault，模态打开期间暂停锁定计时。

### Fixed

- **GUI 窗口大小恢复** — 修复权限问题与同步缓存覆盖导致的启动后尺寸无法恢复。
- **GUI 关键字段日志** — 修复关键字段操作日志中的对象名、页面与属性名记录。
- **GUI 侧边栏滚动** — 修复各模式下滚动条显示与图标偏移。
- **GUI 搜索闪烁** — 修复搜索弹窗与结果高亮的闪烁问题。
- **GUI 主题悬停色** — 统一深色/浅色模式下的悬停颜色。

## [2.2.0] - 2026-06-14

### Added

- **端到端设备同步** — 实现 HLC / Noise XX / CRDT 的本地设备同步 P2/P3 完整链路，并在设置页增加「设备同步」入口。
- **插件系统 Phase 1-4** — 恢复并实现完整插件生命周期：真实 Vault 字段读取、网络代理、阻塞式 Consent、结果导出为 JSON / Markdown、在线注册表更新与 Minisign 签名验证、插件运行状态持久化与 Wasm 崩溃隔离。
- **插件开发资源** — 新增 JS / Python SDK 占位目录与 Wasm 插件开发指南。
- **模板管理增强** — 模板页面新增筛选与搜索功能；系统示例模板拆分为中文 / 英文独立子页面。
- **帮助文档** — 重写并更名帮助文档中的「同步」章节为「设备同步」。

### Changed

- **Windows 安装包品牌化** — NSIS 安装程序 UI 与品牌视觉一致，资源生成脚本对缺失 Pillow 更健壮。
- **macOS 应用标识** — 修正 macOS bundle 配置键，正确设置 Dock 名称与应用名。
- **登录页交互** — 账号名/选择器始终显示在所有登录方式上方；生物识别按钮动效与悬停样式统一。
- **搜索与布局** — 搜索弹窗自适应、结果卡片布局与高亮优化；主内容区增加 `scrollbar-gutter: stable` 防止滚动条抖动。
- **侧边栏滚动条** — 各模式侧边栏滚动条完全隐藏，避免 Windows 下图标偏移。

### Fixed

- **代码审计 P0/P1 问题** — 落地 R001–R032 与 F001–F032 中可修复的安全与稳定性问题。
- **Vault 加密** — 敏感字段使用 AES-256-GCM 加密存储；VaultService IO 移出异步锁路径，大文件加解密改为流式处理以限制内存占用。
- **生物识别安全** — 生物识别主密钥从混淆文件迁移至系统 Keychain。
- **路径遍历防护** — 清理附件导入/导出、备份文件名、嵌入模型 ID 等用户可控路径。
- **账户创建竞态** — 序列化 `create_account` 并拒绝重复 active ID 的对象创建。
- **登录性能** — 密码登录 KDF 与账户列表查询移至阻塞线程池。
- **异步竞态与内存泄漏** — 引入 `useCancellable` 保护关键 effect，清理剩余 copy/search timeout 监听器。

### Security

- 修复审计报告中的敏感数据明文落盘、硬编码 XOR 混淆、路径遍历、命令越权访问等问题。
- 限制 `fs` 类命令可访问路径，禁止目录穿越。
- 备份与附件操作改为精确 ID 匹配，避免前缀匹配误删/误读。

## [2.1.0] - 2026-06-12

### Added

- **新手引导** — 首次启动时显示可跳过的新手教程卡片，完成状态持久化到 `ui_preferences.json`；若已存在账号则自动跳过。
- **应用内更新检测** — About 页面自动检测 GitHub Releases 新版本并显示更新横幅，支持一键下载安装。
- **NSIS 安装程序品牌化** — Windows 安装包支持语言选择（简体中文/英文），并显示自定义欢迎与完成文案。
- **模板示例库** — 模板管理页新增 8 个系统推荐模板示例，一键以此模板创建对象。
- **关键字段默认敏感度** — 系统推荐模板中的身份证、护照、银行卡等关键字段默认标记为 `critical`。
- **图标系统扩展** — 侧边栏新建页面卡片可直接选择图标，支持更多图标选项。
- **审计日志增强** — 密码登录、Touch ID 解锁、Face ID 解锁以及关键字段查看均写入操作日志，关键字段日志包含对象名、所属页面和属性名。
- **窗口大小持久化** — 窗口尺寸同时保存到明文 UI 偏好（登录前可用）和加密账户偏好（账户级隔离），启动时自动恢复。

### Changed

- **搜索结果卡片** — 敏感度标签按 `公开/内部/敏感/关键` 顺序排列，命中的字段名/值使用主题色高亮。
- **搜索交互** — 搜索结果点击对象后打开详情模态框（而非编辑器），支持附件查看。
- **生物识别实现** — macOS 生物识别改用 objc2 FFI，移除运行时 `swiftc` 编译依赖，不再要求安装 Xcode Command Line Tools。
- **Windows 标题栏** — Windows 系统标题栏颜色跟随应用主题。
- **侧边栏底部按钮** — 支持自定义 3 个底部快捷按钮位置。
- **登录页视觉** — 按钮加入浮动动画与边框高亮。
- **安全设置** — 自动锁定时长绑定状态并加入说明，新增主密码警告文案。
- **操作日志** — 页面创建日志国际化并按类型区分颜色，生物识别解锁显示具体类型（Touch ID / Face ID）。

### Fixed

- **主内容区纯色背景** — 修复页面变成纯色内容、仅显示侧边栏和 AppBar 的问题。
- **侧边栏滚动条** — 左/右/上/下各模式均完全隐藏滚动条，避免 Windows 图标偏移。
- **滚动条占位** — 主内容区增加 `scrollbar-gutter: stable`，防止滚动条出现/消失时内容左右偏移。
- **窗口大小恢复** — 修复启动时磁盘旧值覆盖同步缓存导致无法恢复上次尺寸的问题。
- **NSIS 卸载流程** — 维护模式卸载成功后直接退出安装程序，不再重新进入安装流程。
- **macOS 关闭按钮** — 修复 `onCloseRequested` 监听器导致交通灯关闭按钮无法关闭窗口的问题。
- **Windows 编译** — 修复 `windows` crate 版本对齐与 HWND 类型转换问题。
- **LLM 统计** — 修复重置统计按钮在深色模式下的显示。
- **启动页面** — Bootstrap 密码提示词同步修复。

## [2.0.1] - 2026-06-12

### Fixed

- **macOS Bootstrap Page Input (v2)** — Password fields on first-run wizard no longer blocked by WKWebView password manager. Replaced `<input type="password">` with `type="text"` + CSS `-webkit-text-security: disc` to bypass WebKit keyboard interception on forms with multiple password fields. Also applied to login page password input.
- **Password Change Command** — Fixed `Command vault_change_password not found` error on Settings page. Frontend was calling `vault_change_password` but Rust command was registered as `change_password`.
- **Password Hint After Password Change** — When updating password + hint simultaneously, the hint update now uses the new password (instead of the old one, which failed because the password had already changed).

## [2.0.0] - 2026-06-12

### Changed (Major)

- **Complete Rewrite: Flutter → Tauri v2** — SoloSoul now uses a Rust/Tauri backend with React/TypeScript frontend, providing better performance, smaller binary size (~50MB vs ~200MB), and more native feel. The Flutter codebase in the `flutter/` directory is no longer maintained.
- **Crate Name** — `solosoul-app` → `solo_soul` (Rust snake_case convention).
- **Bundle Identifier** — Updated to `com.solosoul.app`.
- **Package Manager** — Flutter → npm (Node.js ≥ 22, React 19, Vite, TypeScript strict mode).
- **State Management** — Riverpod → Zustand.
- **UI Framework** — Flutter Widgets → React + CSS Modules (no Tailwind).
- **Layout** — Removed custom title bar, search changed to popover, support top/bottom function bars.
- **Theme System** — Full CSS custom properties theme engine with 20+ preset themes, accent color customization, and light/dark modes.

### Added

- **App Icon** — Custom shield icon with embedded S letter rendered via SVG path, not relying on system fonts.
- **Object CRUD** — Complete object workspace with card-based display, property editing, inline add/remove.
- **Template System** — User-defined object templates with 8+ field types (text, number, date, checkbox, select, multi-select, URL, email, phone).
- **Select/Multi-Select Field Options** — Template editor now supports defining options for select-type fields with an options editor overlay.
- **Field Format Hints** — Show format examples (e.g. `https://example.com`, `name@example.com`) below field names in the object editor.
- **URL Validation Relaxation** — Auto-prepend `https://` before validation to reduce user friction.
- **History System** — Object snapshot-based history with rollback, diff summary, and version browser.
- **Trash/Recycle Bin** — Soft-delete, restore, permanent delete, conflict detection on restore, batch operations.
- **Search** — Full-text search across objects with category/tag/type filtering.
- **AI Chat** — Multi-provider LLM support (OpenAI, Anthropic, Ollama, etc.) with streaming chat, conversation management, and AI-powered object creation.
- **Attachment System** — Upload/download/preview (images, PDF, text), rename, soft-delete, restore, permanent delete, vault encryption.
- **OCR Scanning** — Image-based OCR with MRZ parsing for passports/IDs.
- **Local Scan** — Filesystem scanning and indexing for local file search.
- **Template Manager** — Full template CRUD with field options editing and property reordering.
- **Export/Import** — Object export with encryption, tag filtering, attachment inclusion.
- **Password Hint** — Welcome page now includes a password hint input field with clear/undo support.
- **Biometric Unlock** — Touch ID / Face ID support.
- **Privacy Policy & Terms of Service** — Bilingual (EN/ZH) documents with language-based auto-routing.
- **Search Index** — Built-in user guide search with 2685 indexed words across 23 guides.
- **Operation Log** — Structured audit logging with full i18n support, all CRUD actions tracked.
- **Orphaned Attachment Cleanup** — One-click cleanup of unreferenced attachment disk files.
- **Vault Stats Dashboard** — Pie chart visualization of storage breakdown (objects, attachments, metadata).

### Fixed

- **macOS Bootstrap Page Input** — Password fields on first-run wizard no longer blocked by WKWebView password manager. Replaced `<input type="password">` with `type="text"` + CSS `-webkit-text-security: disc` to bypass WebKit keyboard interception on forms with multiple password fields.
- **Windows Language Detection** — Use `GetUserDefaultUILanguage` Win32 API to correctly detect display language (was always showing English on first launch). Final 5-layer detection chain: Rust eval → localStorage → IPC get_system_locale → navigator.language → eval override.
- **Windows Resource Paths** — Fixed `resource_path()` resolution for docs on Windows (was incorrectly using macOS `../Resources` path).
- **Windows User Data Path** — Use `%USERPROFILE%` instead of `$HOME` for data storage.
- **Windows Password Reveal Overlap** — Hidden native WebView2 password reveal button via CSS (`::-ms-reveal`) that overlapped the custom eye icon.
- **Multi-Select Order** — Field values now display in template option order, not selection order.
- **Multi-Select Display** — Fixed multi-select values not showing in cards, detail modals, and history panels.
- **Page Restore** — Restoring a deleted page from trash now immediately refreshes the sidebar via `loadCustomPages()`.
- **Theme Sync** — Selecting a card theme now automatically syncs the light/dark appearance option.
- **Password Validation** — Proper error message for short passwords (was incorrectly showing "incorrect password").
- **Password Error Matching** — Added detection for backend "8 characters" message to show localized `password_too_short`.
- **Validation i18n** — URL/email/phone/date/number validation errors no longer display raw localization keys.
- **Toast Messages** — Improved error toast from redundant "验证失败: 验证失败" to descriptive messages.
- **About Page** — Platform shows OS name only (no architecture); logo uses consistent brand gradient.
- **Privacy Policy & Terms Links** — Fixed 404 links to correct docs path, added bilingual documents.
- **Login Theme Flash** — Inline script + CSS dark color fallback prevents white flash on dark-themed first load.
- **Object Editor Data Pollution** — Switching objects no longer shows stale previous object data.
- **Object Editor Jump Flash** — Loading placeholder prevents blank form flash during data fetch.
- **Attachment Preview CSP** — Added `frame-src 'self' data:` for PDF iframe preview.
- **Attachment Double-Save Prevention** — StopPropagation on preview close to prevent accidental card dismissal.
- **Attachment Preview on Windows** — File path + data URI dual-path fallback for preview across platforms.
- **Vault Stats Field Names** — Fixed camelCase/snake_case mismatch between Rust backend and TypeScript frontend.
- **Orphaned Attachment Stats** — `get_vault_stats` now only counts referenced attachments.
- **Restore Conflict Detection** — Fixed to check same-name + same-page instead of ID existence.
- **Restored Suffix i18n** — Now generated in Rust backend per user's language.
- **Trash Detail Display** — All property keys use i18n, internal fields like `__attachments` filtered out.
- **History Snapshot Layout** — Fixed layout jumps during page transitions, unified badge styling.
- **Diff Summary** — 'Created' now shows i18n "初始版本" instead of raw text.
- **Crate `--bundles app` only** — Avoids generating unnecessary AppImage on macOS.
- **Sidebar Rename** — Double-click to rename custom pages.
- **Theme Persistence** — IPC-driven theme loading before React render prevents theme flicker.
- **FAB → Inline Add Button** — Replaced floating "+" with dashed-border placeholder cards.
- **Attachment Preview** — Data URI + file path dual-path strategy bypasses CSP limitations.
- **Auth Page Redirects** — Fixed closure capture bug using `getState()` pattern.
- **Set Vault Path Index Loading** — Fixed missing `loadIndex()` call on account switch.
- **Account Deletion Lock** — Delete now properly locks and redirects to login.
- **Change Password Flow** — Wrong current password returns proper error instead of hanging.

### Architecture

- **§4 Layered Encryption** — `ui_preferences.json` stored in plaintext; theme/language loaded before React render to prevent flash.
- **§5 Snapshot System** — `object_snapshots` table with auto-save on modify, version browser, rollback.
- **§24 Flat Model** — Removed collection layer; tags replace hierarchical grouping.
- **§25 Section/Type Separation** — `section_type` column separates UI grouping from `type_id` semantics.
- **§26 Multi-Provider LLM** — 5 preset providers + custom endpoint, API key separation, connection test, risk disclosure.
- **§6 Attachment System** — Full spec: create/rename/soft-delete/permanent-delete/preview/restore with dedicated trash tab.

### i18n

- **Complete i18n Foundation** — i18next + react-i18next with en-US/zh-CN locales across all pages (editor, auth, settings, navigation, layout, object workspace, AI chat, etc.).
- **Object field keys** — All template field keys and validation messages localized.
- **Operation log** — Actions, entities, performed-by, conflict status, section names all localized.
- **Trash page** — Time labels, type labels, expiration text, relative times all localized.
- **History** — Version labels, triggers, loading/no-history states all localized.
- **Attachment cards** — All buttons, confirmations, empty states, preview errors localized.
- **Template editor** — Field types, option editor, delete confirmations all localized.
- **User guides** — 23 system guides in both languages.
- **Bootstrap page** — Password hints, validation rules, account names all localized.
- **Settings** — All tabs, descriptions, backup labels, security settings fully localized.
- **Layout** — Sidebar, search popover, app shell, theme selector all localized.
- **About page** — Version info, platform, legal links all localized.

### Chores

- MSI bundle removed from build targets; Windows generates only NSIS (.exe).
- Cleaned up all language detection debug logging, retained production-ready 5-layer detection.
- iOS, Android, tvOS build targets removed from Cargo workspace (Tauri desktop-only).
- Removed `create-dmg` AppleScript dependency documentation (fallback to `hdiutil` fine).

## [1.8.0] - 2026-06-03

### Fixed

- **Windows PDF Preview** — Migrated from `pdfx` to `pdfrx` (^2.4.3), fixing PDF preview on Windows. `pdfx` was incompatible with CMake 4.x due to deprecated `DownloadProject.cmake` syntax.
- **Cross-Platform PDF Engine Unification** — Both macOS and Windows now use the same PDF rendering engine (PDFium via `pdfrx`), ensuring consistent behavior.
- **i18n Hardcoded English** — Replaced 20+ hardcoded English strings in biometric settings with localized ARB keys (`biometricTypeNotAvailable`, `biometricTypeAuthFailed`, etc.).
- **Ollama Exception Localization** — `modelNotFound` errors now display localized messages instead of raw English.
- **Windows CJK Font Fallback** — Added `_cjkFontFallback` (`Microsoft YaHei`, `PingFang SC`, etc.) to `AppTheme`, ensuring Chinese characters render with correct glyphs on Windows instead of falling back to Japanese fonts.
- **Settings Version Icon** — Windows platform icon in version sheet was incorrectly showing `phone_android`; now correctly shows `desktop_windows`.
- **Recent Device Platform Detection** — `getDeviceName()` now appends `(Windows)` / `(Linux)` to hostnames that don't contain platform keywords, so `getDevicePlatformLabel()` can correctly identify them in the recent devices list.

### Added

- **Login Page Account Short ID** — Account list items now display a short ID prefix (e.g. `acc_550e84`) next to the account name, allowing users to distinguish accounts with identical names copied from different machines.
- **Password Input Page Account Short ID** — The account header in the password input section also shows the short ID for quick verification before entering the master password.

### Changed

- **PDF Rendering Service** — Rewrote `PdfRenderService` using `pdfrx` API. Pages are rendered to BGRA then converted to PNG for OCR consumption.
- **Attachment PDF Preview** — Replaced custom `PdfController` + `PdfView` + navigation bar with `pdfrx`'s built-in `PdfViewer.file()` widget, which includes zoom, scroll, and page navigation out of the box.
- **Build Scripts** — Added `dart run pdfrx:remove_wasm_modules` to all build scripts (DMG, Windows ZIP) to strip unused Web WASM assets and reduce bundle size.
- **Windows CMake** — Removed `pdfx` auto-removal hacks from `windows/CMakeLists.txt`; `local_auth_windows` removal is retained.

## [1.7.1] - 2026-06-03

### Added

- **Windows Release Packaging**
  - `build_windows_zip.ps1` / `build_windows_zip.sh` — PowerShell and Bash scripts for Windows ZIP packaging, auto-reading version from `pubspec.yaml`.
  - CI/CD three-stage pipeline — `build-macos` + `build-windows` → unified `release` job uploading both `.dmg` and `.zip` to GitHub Releases.

- **Windows Compilation Hardening**
  - CMake auto-fix removes incompatible `pdfx` and `local_auth_windows` plugins from generated registrants.
  - `Cargo.toml` conditional dependency: `ort` excluded on Windows; OCR module uses stub implementation on Windows.
  - Enforced 900×600 minimum window size on Windows via Win32 `WM_GETMINMAXINFO`.
  - C++ source comments switched to English to prevent VS 2026 C4819 encoding errors under `/WX`.

### Fixed

- **Plugin Routing** — `backRoute` corrected from `'/'` to `AppRoutes.home`; missing `AppRoutes` import added.
- **Backup Manifest** — Ensures sidecar directory exists before writing `manifest.json`.
- **PDF Render on Windows** — `PdfRenderService` now returns `null` safely on Windows instead of crashing.
- **Rust Dependency** — Removed duplicate `rustls-tls` key in `reqwest` features.

### Changed

- **Build Scripts** — `build_dmg.sh`, `build_windows_zip.sh`, and `build_windows_zip.ps1` now auto-read version from `pubspec.yaml`; manual version argument is optional.
- **Operation Log** — Extended coverage to attachment, backup, and plugin lifecycle operations.
- **Localization** — Backup list summary text fully internationalized.
- **Settings UI** — Removed help & guides section (redundant with home quick-actions).

### Docs

- Updated Windows known issues in `docs/TODO.md`.
- Rewrote trash/recycle bin guides (Chinese & English), clarifying distinction between general trash and AI conversation trash.
- Fixed sensitivity level naming in object guides.

### Internal

- `.gitattributes` added to enforce LF for shell scripts.
- Removed redundant AttachmentPool "already exists" debug log.

## [1.7.0] - 2026-06-02

### Added

- **Attachment Storage & Statistics**
  - Data management page now correctly counts and displays attachment size alongside vault data size.
  - `AttachmentStorageService` gained `getTotalAttachmentSize`, `getAttachmentCount`, `getAttachmentFileIds`, and `cleanupPartialFiles` APIs.

- **Attachment Sync Support**
  - Rust `SyncEngine` extended with a dedicated attachment sync phase: manifest exchange → serial chunked transfer → temp file `.solo.part` + checksum rename.
  - `StateVectorRequest/Response` added `supports_attachments` backward-compatible flag.
  - Dart `SyncService` and `SyncPage` adapted to report attachment transfer progress.

- **Backup with Attachments**
  - `BackupService` rewritten to use a sidecar directory `{backupFile}.backup.attachments/` for each backup.
  - `createBackup` copies account attachments to the sidecar; `restoreBackup` restores them; `deleteBackup` cleans up the sidecar.

- **Attachment Reference Pool**
  - New `AttachmentPoolService` manages a global attachment pool (`attachments_pool/`) where each `fileId` is stored only once.
  - Backups share pool files via `manifest.json` references instead of duplicating attachments.
  - Deleting a backup triggers lazy reference counting: scans all remaining manifests and removes unreferenced pool files.
  - Old-format sidecar directories (containing raw `.solo` files) are automatically migrated to the pool + manifest format on restore.
  - Backup list UI now shows "X backups · Attachment Pool Y" with an explanatory subtitle about shared storage.

- **Attachment Management & PPTX Preview**
  - Full attachment lifecycle: upload, download, preview (PDF via `pdfx`, images with zoom/pan), soft-delete, restore, and permanent deletion.
  - PPTX thumbnail extraction for quick preview.

- **Windows Rust FFI Support**
  - Added Windows platform compilation and linking support for the Rust FFI native library.

- **AI Context & Chat Enhancements**
  - AI context now injects installed plugin information for richer responses.
  - Multi-conversation windows, trash/recycle bin for deleted conversations, and message timestamps in LLM chat.

- **User Guides System**
  - New plugin feature guide documents.
  - Home page quick-actions section now includes help & guides entry.
  - Settings page guide items include descriptive subtitles.
  - Notion-style rendering for guide documents via `flutter_markdown_plus`.

- **Test Infrastructure**
  - Added 34 new test files; test coverage increased from 15.9% to 23.5% (+7.6%).
  - New `attachment_pool_service_test.dart` (9 tests) and `backup_service_manifest_test.dart` (9 tests).

### Fixed

- **Code Security & Quality (P001-P011)**
  - Eliminated ~166 force unwraps (`!`) across 34 files.
  - Fixed 3 `use_build_context_synchronously` warnings.
  - Added path injection hardening for `Process.run` calls.
  - Replaced bare `catch (e)` with typed `on Exception catch (e)`.
  - Fixed memory leaks in `llm_chat_session_provider` (`StreamSubscription`/`Timer` cleanup).

- **P014** — Fixed `dart:convert` import in `plugin_card.dart`.
- **P007 Supplement** — Replaced `ValueChanged` with native `Function` types.

### Refactored

- **P007-B** — Extracted UI dependencies from `attachment_upload_service.dart` into the Presentation layer.
- **P007-A** — Decoupled `IconData` dependency from `unified_object_service.dart`.
- **P013** — `solosould` daemon migrated to structured logging (`log/slog`).
- **P015** — Replaced all `map[string]interface{}` in `server.go` with concrete structs.
- **P012** — Batch fixed code quality issues across test files.

### Docs

- Generated final code analysis report (`CODE_ANALYSIS_REPORT_FINAL.md`) — all 15 issues resolved.
- Added deferred items explanation (`CODE_ANALYSIS_DEFERRED_ITEMS.md`).
- Updated code analysis report with final status.

## [1.6.8] - 2026-05-31

### Added

- **Attachment Soft-Delete & Trash** (`dabb7e4`)
  - Added `isDeleted` + `deletedAt` fields to `Attachment` model with full `copyWith`/`toJson`/`fromJson` support.
  - `UnifiedObjectNotifier` gained `softDeleteAttachment`, `restoreAttachment`, and `permanentlyDeleteAttachment` methods.
  - `AttachmentListSheet` UI rebuilt to support 3-state lifecycle: active / soft-deleted (visible in trash tab) / permanently removed.
  - `updateObject` diff logic automatically filters out soft-deleted attachments from the active list while preserving them for restoration.

- **Attachment Download Service** (`dabb7e4`, `cc2c666`)
  - New `AttachmentDownloadService` class encapsulates decryption → file write → platform share sheet flow.
  - Downloads target a configurable directory (default: system `Downloads`). Settings page added `settings_page_download_section.dart` with custom path picker and "Restore Default" button.
  - Filename collision handling: `report.pdf` → `report (1).pdf` → `report (2).pdf` via `_getUniqueFilePath`.
  - Write-permission validation before every download; on macOS sandbox path invalidation, auto-fallback to default Downloads with a SnackBar notification.
  - Debounce guard (`_activeDownloads` `Set<String>`) prevents concurrent duplicate downloads from rapid button taps.

- **Attachment Preview Enhancement** (`dabb7e4`)
  - PDF preview powered by `pdfx` package with page navigation and zoom gestures.
  - Image preview supports full-screen zoom, pan, and tap-to-dismiss via `InteractiveViewer` + `GestureDetector`.
  - Preview dialog close button repositioned to top-right for consistency with macOS window chrome.

- **Inline Add Section Placeholder** (`78b1a4f`)
  - New reusable `AddSectionPlaceholder` widget: dashed border, centered "+ Add Section" text, tap gesture.
  - Replaces the FAB "+" button in `ObjectWorkspacePage` and `ObjectCategoryPage`.
  - Appears at the end of every list (pages, root objects, default category pages), eliminating empty-state dead ends.
  - AppBar "+" icon removed; all add flows now route through the inline placeholder for visual consistency.

- **Attachment Upload Service Extraction** (`cc2c666`)
  - New `AttachmentUploadService` unifies the file-pick → sensitivity-check → encrypt → store flow.
  - Replaces duplicated upload logic in `ObjectCardItemTile` and `EntryCardWidget` (~100 lines removed per widget).
  - Single call site: `AttachmentUploadService.upload(context, objectId, fileName, bytes, sensitivity)`.

### Fixed

- **Account Duplicate-Name Logic Bug** (`d319bd0`) — `SecureAccountStorage.createAccount` now rejects accounts with existing names when `accountId == null`. Previously all existing accounts were treated as "stale" and silently deleted, causing data loss.
- **JSON Serialization Key Mismatch** (`54cf246`) — Added `@JsonKey(name: '__propertyLabels')` and `@JsonKey(name: '__semanticTypes')` to `UnifiedObject` to align `json_serializable`-generated `fromJson()` with hand-written `toMap()`.
- **Orphan File Prevention** (`cc2c666`) — `permanentlyDeleteAttachment` throws `StateError` when `accountId` is null, preventing encrypted file orphans on disk.
- **macOS Sandbox Download Fallback** (`cc2c666`) — When the user-configured download directory becomes unreachable (sandbox token expiration after restart), the service automatically falls back to `~/Downloads` instead of crashing.
- **TypeId Migration Test Sync** (`9c40ee9`) — Updated 6 test files to match the `profile_*` → `__preset_*` typeId migration, fixing 14 pre-existing failures.

### Refactored

- **Plugin Event Handler Decomposition** (`b919a84`) — Split `_onRun()` (285 lines, 7-level nesting) in `plugin_dashboard_page.dart` into `_PluginRunSession` state class + 6 handler methods (`_handleDialogConsent`, `_handleFieldConsent`, `_handlePluginResult`, `_handlePluginLog`, `_handlePluginCompleted`, `_handlePluginError`) + `_showRunResult`.
- **Object Editor Validation Extraction** (`b919a84`) — Split `_saveObject()` into `_validateSaveInput()` (early-return validation) and `_buildProperties()` + `_PropertyBuildResult` (property map construction).
- **Password Dialog Deduplication** (`e0fc878`) — Extracted `_PasswordDialogBaseMixin` containing shared `TextEditingController`, `FocusNode`, error state, `initState`/`dispose`, and focus/text-change handlers. Two dialog variants now only implement `_verify()` and `build()`.
- **OCR Processing Pipeline** (`0b0f0ca`) — Extracted `_processOcrBytes()` unifying OCR → MRZ → field extraction. `_pickImage` and `_pickDocument` now only acquire bytes and delegate to the shared pipeline. ~80 lines of duplicate code removed.
- **Default Structure Factory** (`e5e12e7`) — Extracted `_buildPage()` / `_buildSection()` factory methods in `unified_object_notifier.dart` for reuse between `_createDefaultStructure` and `_migrateDefaultSectionSchemas`.
- **LLM Config Deduplication** (`cfb668e`) — Extracted `_activeProfile()` helper in `llm_config_service`, eliminating repeated `ref.read(currentProfileProvider)` lookups across 5 getters.
- **Icon Lookup Optimization** (`cfb668e`) — Replaced 114-line `switch` in `unified_object_service.getIconFromName` with a `Map<String, String>` constant table. ~70 lines removed.
- **Parallel I/O** (`b75808e`) — `updateObject`, `permanentlyDeleteObject`, and `permanentlyDeleteMultiple` now delete attachments in parallel via `Future.wait` instead of sequential `await`.
- **O(n) → O(1) Lookups** (`b75808e`) — `semantic_type_registry.getType` pre-builds a `Map<String, ObjectTypeDefinition>`; `recommend()` uses a `Set<String>` for constant-time contains checks.
- **Dead Code & Proxy Removal** (`b75808e`) — Removed unused `locale` variable in `plugin_dashboard_page`, unreachable `case 'map'` branch, and proxy methods `_logSectionForTypeId` / `_typeColor` (inlined at call sites).
- **Context Safety** (`b75808e`) — Fixed 3 `use_build_context_synchronously` info warnings in `plugin_dashboard_page.dart` by adding `if (!context.mounted) break;` at the top of the `await for` loop.

### Test Infrastructure

- **Injectable FFI Wrappers** (`d319bd0`) — Added `_saltGenerator` / `_keyDeriver` function pointers to `SecureAccountStorage` with `setFfiWrappersForTest()` API. All direct `frb.frbGenerateSalt` / `frb.frbDeriveKey` calls replaced with injectable wrappers.
- **Full Test Suite Green** — All 902 unit tests pass (0 failures). Previously 16 tests failed due to stale `typeId` formats and missing Rust FFI initialization.

## [1.6.7] - 2026-05-31

### Fixed

- **Force Unwrap Elimination** — Eliminated ~166 `!` force unwraps across 34 files. Replaced with local variable null-checks, pattern matching (`case final x?`), and `whereType<T>()` to prevent runtime crashes.
- **Context Safety** — Fixed 3 `use_build_context_synchronously` warnings in `plugin_dashboard_page.dart` by adding `mounted` checks before using `BuildContext` after async gaps.
- **Path Injection Hardening** — Added `_isSafePath()` validation to `Process.run` calls in `content_parser_service.dart` to prevent command injection via malicious file paths.
- **Typed Exception Catching** — Replaced 4 bare `catch (e)` clauses with `on Exception catch (e)` to avoid swallowing `Error` subclasses.
- **Memory Leak Fix** — Added `onDispose` cleanup for `StreamSubscription` and `Timer` in `llm_chat_session_provider.dart` to prevent leaks on provider disposal.

### Refactored

- **RadioGroup Migration** — Migrated deprecated `RadioListTile.groupValue` / `onChanged` to `RadioGroup` ancestor pattern in `plugin_radio_list_dialog.dart` and `plugin_sensitivity_override_dialog.dart`. Eliminates all 4 dart analyzer info warnings.
- **Widget Extraction** — Split 4 oversized build methods (>200 lines each) into focused sub-methods:
  - `_onRun()` (307 lines) → `_prepareInitialParams()` + `_showExecutionResult()`
  - `app_sidebar build()` (237 lines) → `_buildSidebarContent` / `_buildPagesSection` / `_buildBottomActions` / `_buildResizeHandle`
  - `_PluginResultDialog build()` (224 lines) → `_buildTitle` / `_buildLogsSection` / `_buildResultsSection` / `_buildErrorBanner`
  - `trash_card build()` (214 lines) → `_buildHeader` / `_buildActionBar`

### Code Quality

- **Static Analysis Zero-Warnings** — `dart analyze` now reports 0 errors, 0 warnings, 0 info across the entire codebase.
- **Dead Code Removal** — Removed unused imports, variables, and parameters across 5 files.

## [1.6.6] - 2026-05-25

### Added

- **Sidebar Plugin Entry** — Added Plugins shortcut to the app sidebar above the Lock Vault button for quick access to the plugin dashboard.

### Fixed

- **Identity Section Title Bug** — Fixed all identity-related sections (Passport, Travel, Education, Employment, Skill, Language, Article) showing empty or incorrect titles. `ObjectTypeRegistry` now uses `Title` as the title key for all predefined types. `ObjectCard._template` deduplicates redundant `Title` fields when a custom title key exists, and `ObjectEditorPage` correctly recognizes type-specific title keys.
- **Missing Title in Predefined Sections** — `DynamicSectionCard` now falls back to `ObjectTypeRegistry.buildPropertiesFromType()` when a section's properties lack a `Title`. All 8 predefined types include `Title` in their schema.
- **Title Sensitivity Tag** — `ObjectCardEditField` now displays the `SensitivityTag` (e.g., "公开") for title fields, not just regular fields.
- **Skill/Language Schema** — Removed redundant `name` field from `professional_skill` and `professional_language` templates; now use `Title` + `level`/`proficiency` only.
- **HardwareKeyboard Error Spam** — `main.dart` now silently filters out `HardwareKeyboard` assertion errors on macOS, preventing debug console noise.
- **Address Formatter Error Messages** — Removed `[address-fmt]` prefix from user-facing error logs in the address formatting plugin.

## [1.6.5] - 2026-05-25

### Added

- **Plugin Runtime v2** — Semantic type system, access review dialog, and address formatter plugin. Plugins now declare semantic types (e.g., `pet.name`) and request field-level consent before execution.
- **Plugin Market Expansion** — 20 plugins available via GitHub-as-market with multi-source support and CDN fallback:
  - TOTP Generator, Emergency Card, Address Formatter, MRZ Encoder
  - Expiry Guardian, ID Validator, Phone Formatter, Contact Exporter
  - Packing List, Calendar Events, Digital Will, Identity Timeline
  - Form Prefiller, Tax Profile, Data Completeness, Namecard Generator
  - Doc Checklist, Travel Footprint, Resume Builder
- **Plugin Home Quick Action** — Added Plugins shortcut to home page quick actions grid.
- **Schema propertyLabels** — Storage key, display label, and semantic type are now three independent layers. `propertyLabels` map allows editable field display names without changing storage keys.
- **Drag-to-Reorder Fields** — Section editor supports drag-to-reorder property fields via `ReorderableListView`. Order persists via `propertyOrder` on `UnifiedObject`.
- **Parent Page Selector** — Section editor now includes a "所属页面" dropdown to move sections between pages. Sections are prepended to the front of the new page's children.
- **Xcode Auto-Build Rust** — Debug builds now auto-compile `cargo build --features sandbox` when the native library is missing, eliminating manual Rust build steps.

### Fixed

- **Section Move-Delete Cascade** — `updateObject` no longer silently overwrites `parentId` with `null` when `parentId` is not explicitly passed. Fixed by introducing `UnifiedObject.kNullSentinel` in `_service.updateObject` and `notifier.updateObject`. Also fixed `restoreObject` unable to clear `deletedAt`.
- **Account Delete-Recreate Conflict** — `SecureAccountStorage.deleteAccount` now verifies removal and retries on stale data. `createAccount` auto-cleans stale Keychain records before creation. `AccountManager.deleteAccount` no longer silently ignores Keychain failures.
- **ObjectCard UI Polish Cycle** — Collapse threshold reduced from 3 to 1 items; persistent full-width "add item" button at bottom; edit/delete buttons grouped next to section name with add button pushed to far right.
- **Title Key Normalization** — All built-in type definitions now use `id: 'Title'` (capitalized). Legacy `'title'` is normalized at load time and in card templates to prevent duplication.
- **Title Order in Items** — Title is now included in `propertyOrder` so it stays at the top of item cards instead of falling to the bottom.
- **Empty Key Save Fallback** — New fields with empty `key` now use `displayLabel` as the fallback property key on save.
- **Editor Drag Handle** — `ReorderableDragStartListener` now uses the ReorderableListView's local index instead of the global `_propertyFields` index, fixing "non-visible item" exceptions.
- **Localized Field Display** — Restored localized display labels in the object editor schema view.
- **Registry Offline Fallback** — Bundled `registry.json` ships with the app so plugin market works on first launch without internet.
- **P001–P018 Code Quality** — Comprehensive static-analysis cleanup: replaced deprecated `withOpacity` with `withValues()`, added `on Exception` to catch clauses, removed dead code, extracted oversized widgets, fixed unawaited futures, and resolved 160+ analyzer warnings across 41 files.

### Refactored

- **Widget Extraction** — Extracted `_buildAccountTile` and glass settings methods from `app_sidebar` to reduce nesting and file size.
- **Git Cleanup** — Removed 33.7MB prebuilt `libsolosoul_core.a` from git tracking; updated `.gitignore` for Flutter ephemeral artifacts.

### Tests

- Added `PluginManager`, MRZ parser, and Vault path tests.
- Fixed failing tests from second wave test coverage push.

## [1.6.4] - 2026-05-23

### Added

- **Icon Library Expansion to 96 Icons** — `icon_picker_sheet.dart` now supports 96 icons across 12 categories (travel, finance, identity, education, technology, health, media, objects, nature, symbols, arrows, misc). Added search filtering, categorized grid display, and collapsible filter bar.
- **Default Page Custom Sections** — Profile/Travel/Financial/Professional pages now support adding custom sections via "+" button alongside the sensitivity mode button. Reuses `AddSectionDialog` and `custom_sections_widget.dart`. New sections appear at the bottom and support rename, delete, and property editing.
- **Article Section Template** — New predefined template for articles and notes with Title, Author, Source, URL, and Content fields.
- **Password Hint-Only Changes** — `change_password_dialog.dart` now supports updating only the password hint without changing the master password. Added `updatePasswordHintOnly` flow in `rust_vault_service.dart`.
- **Global Backoff Protection** — Password verification dialog now enforces a 30-second global cooldown after 5 failed attempts. Backoff state is persisted via `SharedPreferences` and survives dialog close/reopen across the entire app.
- **Unified Password Verification Dialog** — Extracted shared `password_verification_dialog.dart` with `showPasswordVerificationDialog()` API. Replaced duplicated password dialogs on search page, settings page, and all protected pages.

### Fixed

- **macOS Hot Restart Compilation Error** — `packages/liquid_glass_widgets/lib/src/renderer/shaders.dart` had `const String _shadersRoot = !kIsWeb && isTestEnvironment ? ...` which failed on IO builds because `isTestEnvironment` is `final` (not `const`) in `_env_io.dart`. Changed `_shadersRoot` and `ShaderKeys` fields to `final` with `ignore: prefer_const_declarations` to prevent analyzer false-positives.
- **Default Page Alignment (Phase 3)** — Custom sections now visually align with predefined sections on Profile/Travel/Financial/Professional pages. Removed hardcoded padding differences.
- **Old Account Auto-Migration** — Accounts created before schema v2 now automatically get missing default pages and sections (Identity, Contact, Address, ID Card, Passport, Visa, etc.) on next unlock.
- **macOS Sandbox Data Isolation** — Fixed `path_provider` data directory resolution for sandboxed release builds. Data now correctly stores in `~/Library/Containers/...` when sandbox is enabled.
- **URL Property Type** — `UrlProperty` now has proper validation regex and displays clickable links in `ObjectCard`.
- **Sidebar Alignment** — Fixed vertical alignment of sidebar items with icons of varying widths. Added `IntrinsicWidth` wrapper for consistent label positioning.
- **Filter Bar Collapse** — Filter sections on operation log, search, and trash pages now properly collapse/expand without layout jumps.
- **Delete Account Flow** — Improved confirmation dialog with warning text and 3-second delay before allowing deletion.
- **New Account Default Pages** — Fixed missing default pages when creating a brand new account after app reinstall.

### Refactored

- **Code Quality Audit (Round 1)** — Comprehensive static analysis and automated fix cycle:
  - Fixed 4 P0 test compilation errors: `llm_query_enhancer_test.dart` (dead file removed), `local_search_service_test.dart` (wrong class reference), `property_value_utils_test.dart` (missing import), `sensitivity_tag_test.dart` (removed `getSensitivityLabel` restored)
  - Fixed P1 warnings: unused variables/imports in `scan_import_service.dart`, `sensitivity_settings_page.dart`, `predefined_object_section.dart`
  - Fixed P1 potential bugs: `use_build_context_synchronously` in `llm_config_page.dart`, `unawaited_futures` in `account_style_provider.dart`
  - Fixed P1 deprecated API usages: `dangling_library_doc_comments` in `mrz_date_utils.dart`, missing `fake_async` dependency
  - Bulk P2 fixes via `dart fix --apply`: 160 fixes across 41 files (`prefer_const_constructors`, `prefer_const_declarations`, `no_leading_underscores_for_local_identifiers`, `unnecessary_import`, `unnecessary_to_list_in_spreads`, etc.)
  - Fixed `build_dmg.sh` entitlements path: `Runner/Release.entitlements` → `macos/Runner/Release.entitlements`
- **Filter Sections Unification** — Extracted shared `FilterSection` widget pattern across operation log, search, and trash pages. Eliminated duplicated filter logic.

### Internal

- Added `currentObjects` public getter to `UnifiedObjectNotifier` to avoid external access to protected `state` property.
- Restored `getSensitivityLabel(SensitivityLevel)` top-level helper in `sensitivity_tag.dart` for test compatibility.
- Generated `CODE_ANALYSIS_REPORT.md` and `CODE_ANALYSIS_REPORT_FINAL.md` documenting 20 identified issues and 17 resolutions.

## [1.6.3] - 2026-05-10

### Added

- **Custom Sections on Default Pages** — Added "+" button to each default page's right side (alongside sensitivity mode button), reusing `AddSectionDialog` to add custom sections to Profile/Travel/Financial/Professional pages. New sections appear at the bottom of the page and support all standard section operations (rename, delete, add/edit properties).
  - `custom_sections_widget.dart`: New shared widget wrapping `ObjectCard` list with add/edit/delete controls
  - `add_section_dialog.dart`: New dialog for naming and creating a custom section on any page
  - Updated all 4 default pages: `profile_page.dart`, `travel_page.dart`, `financial_page.dart`, `professional_page.dart`
- **OCR Scan: Save Original File** — `saveOriginalFile` checkbox in scan document sheet allows saving the scanned image as an encrypted attachment via `saveAttachment()`. Attachment is encrypted with vault key and stored in `UnifiedObject.attachments` map.
  - `scan_document_button.dart`: Added checkbox for save-original-file with translated label
  - `object_card_fields_sheet.dart`: Attachment UI displays saved filename with open/open-location actions
  - `unified_object_model.dart`: Added `attachments` field (`Map<String, String>`) to `UnifiedObject`
  - `base_models.dart`: Added `attachments` persistence field to `UnifiedObjectData`
  - `operation_logger.dart`: Added `logCustomSection()` for property-level audit logging
- **MRZ Scan Section Selector** — When importing from MRZ scan, user can override the default section via dropdown menu in the preview dialog. Validated against existing page sections and dynamically created sections.
  - `mrz_preview_card.dart`: Section selector dropdown + validation
  - `predefined_object_section_helpers.dart`: Added `findSectionByName` + `suggestSectionForType` for smart section routing
  - `entry_card_widget.dart`: Added `currentPageSections` parameter support
  - `travel_page.dart`: Passes page sections to MRZ preview
- **Operation Log i18n** — All action labels, time labels, device labels, and section names in operation log page now localized:
  - Filter: `'Action:'` → `'${l10n.operationLabelAction}:'`, `'Device:'` → `'${l10n.operationLabelDevice}:'`
  - Filter chips: `'macOS'` → `l10n.operationPlatformMacos`, `'iOS'` → `l10n.operationPlatformIos`
  - Tile badges: `_actionLabel` (hardcoded 'Created'/'Updated'/'Deleted'/'Restored'/'Purged') → `_actionLabel(l10n)` using `l10n.operationAction*`
  - Time labels: `_formatTime` (hardcoded 'Just now'/'Xm ago'/'Xh ago'/'Xd ago') → reuses `l10n.trashJustNow`/`trashMinutesAgo`/`trashHoursAgo`/`trashDaysAgo`
  - Device tags: `_getDeviceLabel` (hardcoded 'macOS'/'iOS'/'Android'/'Windows'/'Linux'/'Web') → `l10n.operationPlatform*`
  - Section display: `entry.section.toUpperCase()` → `_sectionLabel(l10n)` mapping via `logSection*` l10n keys
  - Detail dialog: All labels (`_actionLabel`, `_sectionLabel`, `_getDeviceLabel`) use l10n
- **New ARB Keys**: Added 23 new keys to both `app_en.arb` and `app_zh.arb`:
  - `operationPlatformMacos`, `operationPlatformIos`, `operationPlatformWindows`, `operationPlatformLinux`
  - `logSectionIdentity` through `logSectionCustom` (19 section labels)

### Fixed

- **MRZ Visa Routing + Double-Save Guard** — `import_result_page.dart` now correctly routes visa items to the travel page section instead of creating orphan objects. Added prevention for multiple MRZ scans of the same document path creating duplicate entries.
- **Section Deletion Notification** — Fixed snackbar disappearing after section soft-delete. Root cause: ObjectCard gets removed from widget tree after `deleteObject` updates provider state, making `context.mounted == false`. Fix: capture `Overlay.of(context)` before any `await`, pass via `forOverlay` parameter to `showOverlaySnackBar`, and add `BuildContext?` / `OverlayState?` dual-path API in `app_theme.dart`.
- **Trash Children Sorting** — `deletedChildrenProvider` now sorts by `deletedAt` descending (matching `trashRootDeletedObjectsProvider` pattern), ensuring newest deletions appear first.
- **Custom Section Title i18n** — Added `'Title'` case to `translateFieldLabel` switch in `format_field_label.dart` (case-sensitive: custom sections use capitalized `'Title'` key).
- **Trash Detail Dialog Empty Properties** — Removed `l10n.commonEmpty` text for empty property values in `unified_object_trash_card.dart`; labels and sensitivity tags remain visible.
- **ObjectCardPropertiesList i18n** — Changed from `formatFieldLabel(key)` (algorithmic Title Case) to `translateFieldLabel(key, l10n)` (i18n-aware) with proper imports.

## [1.6.2] - 2026-05-10

### Added

- **Section Template Browser** — 15 predefined section templates (passport, visa, bank account China/UK/US, ID card, driver's license, contact, education, employment, skills, languages, awards, identity, task) with localized names/descriptions
- **Trash Empty Placeholder** — Trash card shows "Title: (empty)" in gray when name is blank; detail dialog shows "(empty)" in gray italic for blank property values
- **Operation Log Windows/Linux Filter** — Added Windows and Linux device platform filter chips
- **I18n for Section Templates** — All 15 template names, descriptions, and field keys fully localized in English and Chinese
- **Field Key Translation** — Added snake_case variants for all translated field keys (full_name, given_name, date_of_birth, etc.) to support section template field display
- **Debug Logger Buffering** — Logger now always records sanitized entries to a circular buffer from app start; activation prints buffered logs for pre-bug capture

### Fixed

- **Per-Section Schema Independence** — Items now read schema properties from their parent section instead of a shared `ObjectTypeDefinition`, preventing cross-section property leakage. Each new section starts with only `Title`. New sections no longer inherit properties from previously-edited sections
- **Section Editor Deprecated Toggle Removed** — Deleting a property in a section editor truly removes it (no deprecated toggle); deprecated toggle only appears in the item editor for properties removed from parent schema
- **Section Delete/Re-create** — Removing then re-creating a property in a section no longer triggers "duplicate property" error
- **Settings Page Account Count i18n** — Hardcoded English replaced with `l10n.settingsAccountCount`
- **Settings Page Data Management i18n** — Backup summaries, special backup labels localized
- **Settings Page Current Account i18n** — Operation descriptions, device names, platform labels fully localized
- **Settings Page Version Display** — Platform now shows actual OS name (macOS/Windows/Android/iOS) instead of "Unknown"
- **Device Name Resolution** — Now uses actual system hostname (`Platform.localHostname`) so multiple Macs are distinguishable; adds platform label prefix ([macOS], [iOS], etc.) to device names
- **Template Field i18n** — ObjectCard add-item form shows localized field labels instead of raw camelCase keys
- **Redundant Title Fields Removed** — Removed duplicate Title property field in section editors
- **Duplicate ARB Keys** — Removed duplicate ARB keys causing gen-l10n warnings
- **Redundant Label Above Field Input** — Removed duplicate localized label above template field input in item editor
- **Privacy Mode Timeout** — Fixed privacy mode not timing out correctly
- **Sidebar Child Page Drag** — Fixed drag-and-drop glitch when reordering sidebar child pages
- **Verbose Metadata Update Logs** — Removed verbose debug logs from metadata update operations

### Changed

- **Debug Logger Always Recording** — Logger switched from gated (only records when activated) to always-recording with circular buffer; activation prints buffered entries live
- **liquid_glass_widgets Vendored** — Moved from pub.dev dependency to local `packages/liquid_glass_widgets` path dependency; removed verbose init logs

## [1.6.1] - 2026-05-09

### Added

- **Trash Filter Section** — Collapsible filter panel with time-based filters (10 days, 1 day, 6 hours, 1 hour) and type filters (Page, Section, Item)
- **Trash Hierarchy Expansion** — Pages show "Show sections", sections show "Show items" with nested expand/collapse
- **Trash Card Color Coding** — Icon backgrounds colored by type (blue=page, green=section, orange=item)

### Fixed

- **Delete Dialog i18n** — Hardcoded English replaced with localized strings in delete confirmation dialogs
- **Delete Snackbars** — Changed from `ScaffoldMessenger` to `showOverlaySnackBar` for consistent styling
- **MediaQuery Error** — Fixed null check error in `showOverlaySnackBar` when called from dialog context
- **Item Type Filter** — Fixed "Item" filter not matching predefined section items (travel_passport, profile_identity, etc.)
- **Trash Item Labels** — Removed hardcoded "collection" text from child rows, proper type labels shown
- **Double "前" Bug** — Fixed Chinese localization "在 X 前前删除" to "X 前删除"

### Changes

- **Sub-page Creation Disabled** — Add button now directly shows "Add section" dialog, bypassing sub-page creation to avoid bugs
- **Parent Dropdown Hidden** — Parent selector removed from page editor to prevent accidental parent changes

## [1.5.1] - 2026-05-08

### Fixed

- **Password Hint Persistence** — Account creation now saves hint to Rust vault via `updatePasswordHint` in both normal and fallback paths. Previously the hint was only in Keychain; if Keychain was unavailable, the hint was lost on re-login.
- **Display Card Field Labels** — Added `toFormattedStringLocalized(l10n)` to `FormattableEntry` mixin using `translateFieldLabel`. Updated all 13 `formatAllFields` callbacks across travel, profile, financial, and professional pages.
- **File Picker in Release Builds** — Added `com.apple.security.files.user-selected.read-write` and `com.apple.security.files.bookmarks.app-scope` to DMG signing entitlements (v1.5.0).

## [1.5.0] - 2026-05-08

### Fixed

- **File Picker in Release Builds** — Added `com.apple.security.files.user-selected.read-write` and `com.apple.security.files.bookmarks.app-scope` to `build_dmg.sh` entitlements template. These were missing, preventing `file_picker` and `image_picker` from working in release DMG builds.

## [1.4.9] - 2026-05-08

### Fixed

- **LLM Config Page** — Fixed redirecting to home page in release mode (removed from `debugOnlyRoutes` guard)
- **Local Search & Scan Routes** — Opened to production builds

## [1.4.8] - 2026-05-08

### Added

- **Auto Language Detection** — OS locale auto-detection on first launch (Chinese OS → zh, all others → en)
- **Version Auto-Injection** — Version auto-injected into DMG builds, with update notification linking to GitHub Releases
- **935 ARB Keys** — Comprehensive i18n completion across all pages and widgets with 0 hardcoded strings remaining

### Fixed

- Date picker localization, password dialog width consistency, search empty state colors, untranslated "Vault" references

## [1.4.7] - 2026-05-06

### Added

- **Liquid Glass UI Overhaul** — Complete cross-platform UI redesign using liquid glass material design. All 20+ protected pages, AppBars, cards, buttons, dialogs, and sidebar now use glass-morphism effects with Notion+Anytype bright color palette
- **Login UI Refresh** — Redesigned login page with gradient background, decorative orbs, vertical centering, and hover effects on all interactive elements
- **Back Navigation** — SoloGlassAppBar now supports `backRoute` for proper back button behavior on deep-linked pages

### Fixed

- **Sensitivity Lock Enforcement** — Locking sensitive access now simultaneously enforces data masking and collapses all expanded history records
- **Sidebar Rename Bug** — Editing a custom page name no longer persists when navigating away; double-tap renamed to long-press for faster click response
- **Sidebar Drag Performance** — Cached descendant lookups during drag-and-drop and simplified drag placeholder to reduce jank
- **LLM Stats Persistence** — Skips LLM usage statistics persistence when vault is locked to prevent errors
- **Object Editor Character Counter** — Fixed character counter showing literal text instead of actual number
- **History Timestamp Alignment** — Full timestamps in history records are now right-aligned for consistency

### Refactored

- **Sensitivity Model Consolidation** — `sensitivity_models.dart` moved to `core/models/` for cleaner architecture
- **Scan Service Refactoring** — `local_search_service` now uses `FieldRegistry` as the single source of truth for field sensitivity levels

## [1.4.6] - 2026-05-06

### Added

- **LLM Integration (P0)** — Full AI chat interface with streaming responses, smart field mapping for object creation, and encrypted usage statistics persistence. Supports multiple providers with per-model configuration, usage tracking, and sparkline chart visualization in settings
- **Local Search Import (P0)** — Import objects from local search results with automatic schema field mapping, batch validation, and CancelToken support to interrupt underlying scan I/O
- **Multi-Device Sync** — FRB-based sync engine featuring CRDT data structures for conflict-free replicated data types, Noise protocol encryption for handshake security, and TCP transport layer. Includes dedicated sync UI for device pairing and connection management
- **Rust Core Engine (Phases 1-5)** — Complete migration from Dart crypto fallback to unified Rust implementation:
  - Phase 1: Unified encryption layer with Argon2id + AES-256-GCM
  - Phase 2: Unified account management with UUID account IDs
  - Phase 3: Eliminated Dart fallback, added KdfPreset configuration
  - Phase 4: Typed FRB bindings replacing JSON relay pattern
  - Phase 5: CRDT sync engine with Noise encryption and TCP transport
- **Anytype-Inspired macOS Features** — Redesigned object workspace and editor interactions following Anytype UX patterns for spatial navigation and relation editing
- **Full Operation Recording** — All user CRUD actions are logged via `OperationLogService` with before/after property snapshots, sensitivity levels, and proper `logSectionForTypeId` mapping for complete audit trails
- **Comprehensive Test Coverage (Phases 1-11)** — Added extensive unit tests for LLM service, local import, auth notifiers, vault operations, sync engine, and widget behavior
- **Structured Sensitive Debug Logging (P028)** — `DebugLogger` now tags sensitive data with structured sensitivity levels for safer diagnostic output
- **Sync UI** — New settings section for managing multi-device synchronization with real-time connection status

### Fixed

- **AI Privacy Protection (P001)** — Smart mapping now blocks critical and restricted sensitivity data from being transmitted to cloud LLM APIs, preventing privacy leakage
- **Startup Black Screen (P0)** — Fixed native library loading race condition caused by incorrect `dlopen` path resolution on macOS app launch
- **Unlock Flow Hangs** — Restructured async unlock sequence to prevent UI hangs caused by `verify_hash` encoding mismatches between old and new account formats. Added automatic `verify_hash` repair for corrupted Keychain entries
- **Account Switch Security** — Password verification is now mandatory when switching accounts from settings, preventing unauthorized access via cached session tokens
- **Password Dialog Error Icon Color** — When verifying identity for critical/sensitive fields, entering an incorrect password now correctly turns the hint (`help_outline`) and visibility toggle (`visibility_outlined`/`visibility_off_outlined`) icons red to match the error text, instead of leaving them white/default
- **Security Audit (S001-S015)** — Path traversal validation on profile IDs, minimum PBKDF2 iteration enforcement, secure key material wipe after account creation, backup file permissions (0600), debug mode security hardening, and constant-time comparison migrated to Rust FFI
- **Performance Audit (PF001-PF010)** — Batch delete for empty trash, O(n²) elimination in list operations, save/log debounce, TextEditingController leak fixes, and timer no-op prevention
- **Code Quality Audit (D001-D011, P001-P055)** — Dead code cleanup, concrete exception types in catch clauses, mounted guards, duplicate code extraction, and removed empty setState calls
- **LLM Stability (P002-P007, P016)** — Proper `http.Client` disposal, input field clearing, stream controller leak fixes, print-to-SoloLog migration, and max 5-file limit for AI mapping to prevent request storms
- **LLM Type Safety (P004, P008-P011, P015)** — Debounced stream rebuilds, type-safe API key handling, proper comment alignment, and API key clearing on model switch
- **Import Integrity** — Ensures imported objects contain all schema-defined fields with correct defaults; prevents stats loss when Vault is locked at startup

### Refactored

- **Widget Extraction (P010-P015, P023-P025, P034-P039, P043, P057-P059)** — Extracted 26+ widget classes from 8 oversized files:
  - Settings page: 427 → 35 lines
  - Profile page: 327 → ~50 lines
  - Editor header, bottom save bar, property field rows, contact forms as standalone widgets
- **LLM Service Modularization (P020)** — Split monolithic `llm_service.dart` into focused files: `llm_config_service.dart`, `llm_chat_service.dart`, `llm_mapping_service.dart`, `llm_stats_service.dart`
- **Shared Utilities (Phase 0)** — Extracted `_verifyPassword`, `_postLoginSetup`, dialog overlay helpers, and page templates to eliminate duplication
- **Login Flow (P045)** — Unified `_handleUnlock` and `_handleCreateAccount` post-login setup into shared `_postLoginSetup`
- **Settings Dialogs (P031-P032)** — Extracted `_DebugActivationDialog` reducing dialog builder from 196 → 58 lines

## [1.4.5] - 2026-04-30

### Added

- **Operation Log Search** — Added search bar to operation log page filtering by description, section, and action; shows live result count and supports clear action
- **Trash Property Snapshot** — Purge actions in trash page now capture full property values and sensitivity levels via `OperationLogger.logCustomSection()` for complete audit trail
- **Object Card Title Key Config** — `ObjectCard` now accepts `titlePropertyKey` parameter (default `'Title'`) to support schemas using different title fields (e.g., `fullName` in Identity). Title input controller initialization and save logic now reference this configurable key instead of hard-coded `'Title'`

### Fixed

- **Label Formatting Consistency** — Extracted shared `formatFieldLabel()` utility in `presentation/utils/format_field_label.dart`; applied to `FieldHistoryView` and `OperationTile` so history records and operation log property snapshots display human-readable labels like "Given Name" instead of raw camelCase keys like "givenName", matching the display card formatting

## [1.4.4] - 2026-04-30

### Fixed

- **Trash Purge Snackbar Silent Failure** — `_confirmPurgeUnifiedObject()` previously accepted a `BuildContext` parameter from `ListView.itemBuilder`, which becomes unmounted after `permanentlyDeleteObject()` removes the item from the list. `showOverlaySnackBar()` checks `context.mounted` and silently returns if false. Removed the `BuildContext` parameter from both `_confirmPurgeUnifiedObject()` and `_confirmRestoreUnifiedObject()`; methods now use `_TrashPageState`'s stable `this.context`. Also removed the ineffective `WidgetsBinding.instance.addPostFrameCallback` workaround
- **Trash Card Action Overflow** — Reduced button padding from `EdgeInsets.symmetric(horizontal: 12)` to `6` and wrapped timestamp text in `Flexible` to prevent 13px overflow on medium-width screens
- **Trash Button Alignment** — Replaced `Flexible + Spacer` with `Expanded` so action buttons occupy the full card width consistently
- **Trash Responsive Actions** — Added `LayoutBuilder` with 420px threshold: narrow screens show icon-only buttons with tooltips; wide screens show labeled text buttons
- **Trash History Button State** — Empty history now shows gray icon with "0" count and tap-to-show "No history available" tooltip; non-empty history shows purple icon with count badge
- **Trash Detail/History Dialogs** — Added Details dialog showing fields + sensitivity tags + deletion time; added History dialog reusing `FieldHistoryDialog` with proper field prefix mapping
- **Trash Operation Logging** — Purge, restore, and empty-trash actions now write to `OperationLogService` via `_logSectionForTypeId()` mapping
- **Trash "Untitled" Display** — `PredefinedObjectSection` name resolution now includes `fullName` key in the lookup list
- **flutter_animate Crash** — Removed `.animate().fadeIn()` from `_UnifiedObjectTrashCard` which caused `FractionalTranslation` hit-test assertion during widget removal
- **Operation Log Sensitivity Colors** — Filter chips, `OperationTile`, and detail dialog now use `SensitivityTag` colors: Critical=red.shade900, Internal=blue, Public=green, Sensitive=orange
- **Identity Operation Logging** — `PredefinedObjectSection.onSave()` and `onDidDelete()` now log create/update/delete actions; undo restore also logs
- **Object Editor Sensitivity Dropdown** — `PopupMenuButton` child now shows `Row(SensitivityTag + Icon(Icons.keyboard_arrow_down))` for clearer affordance

## [1.4.3] - 2026-04-29

### Fixed

- **Vault Initialization Race (Android/Windows)** — `NativeVaultService._initialize()` now stores the async init future in `_initFuture`; all Android/Windows public async methods (`createAccountAsync`, `unlockVaultAsync`, `unlockVaultWithKeyAsync`, `deleteAccountAsync`, `listAccountsAsync`, `getAccountConfigAsync`) await `_ensureInitialized()` before accessing `_fallbackSecureStorage` or `_profilesDir`. Sync fallback `_androidRequest()` now returns `{'success': false, 'error': 'Vault not initialized'}` if called before initialization completes, preventing null-dereference crashes
- **Property Editor Controller Leaks** — `_TextEditor`, `_NumberEditor`, `_RelationEditor`, `_UrlEditor` in `property_editor_factory.dart` were `StatelessWidget`s creating `TextEditingController` on every `build()` without disposal. Converted all four to `StatefulWidget` with `dispose()` calling `controller.dispose()`
- **Object Editor Fire-and-Forget** — `_saveObject()` in `object_editor_page.dart` was `void _saveObject() async`, meaning exceptions from internal `await`s were silently swallowed. Changed to `Future<void> _saveObject() async` with full `try/catch`, logging errors via `DebugLogger` and showing `SnackBar` to user on failure
- **Mounted Checks** — Added `if (mounted)` guards after all `await showModalBottomSheet` / `await showDialog` / `await saveObject()` calls in `object_editor_page.dart` to prevent `setState()` on disposed widgets
- **Search Result Tile Rebuild Logic** — `SearchResultTile` previously watched `accountStyleProvider.select((s) => s.value?.displayMode)`, which does not change when a field is revealed. Changed to watch `accountStyleProvider.select((s) => s.value?.revealedFields)` and `isSensitiveAccessGrantedProvider`, ensuring tiles rebuild correctly when users click "Reveal"

### Performance

- **Sensitivity Settings Cache** — `SensitivitySettingsPage._buildSettingsView()` previously re-sorted all fields (O(n log n)) and performed 4 sensitivity-level filters + search filter on every rebuild. Added `_getEffectiveFields()` and `_getFilteredSections()` to `_SensitivitySettingsPageState` with memoization via `_cachedEffectiveFields`, `_cachedSections`, `_cachedRegistryHash`, `_cachedAccountStyleHash`, and `_cachedSearchQuery`
- **Trash Provider Aggregation** — `TrashPage._buildTrashContent()` previously called `ref.watch(effectiveSensitivityProvider(fieldId))` inside a `for` loop over 12 item types on every rebuild, causing 12 individual provider watches. Added `trashItemSensitivityMapProvider` in `sensitivity_provider.dart` which aggregates all 12 sensitivities into a single `Map<String, SensitivityLevel>`; `trash_page.dart` now watches only this one provider. Also added `_getFilteredTrash()` to cache filtered deleted items/unified objects by search query
- **Predefined Object Section Cache** — `PredefinedObjectSection` was a `ConsumerWidget` that rebuilt `fieldDefs` (schema property → `FormFieldDef` mapping with `FieldRegistry` sensitivity lookup) on every provider change. Converted to `ConsumerStatefulWidget` with `_cachedFieldDefs` and `_cachedTypeDef`, eliminating redundant O(m × n) registry traversals
- **Effective Field Level Select** — `effectiveFieldLevelProvider` in `sensitivity_based_visibility_widget.dart` previously watched the entire `accountStyleProvider` AsyncValue. Narrowed to `accountStyleProvider.select((s) => s.value)`, reducing rebuilds when only the AsyncValue wrapper state (loading/error) changes

### Changed

- **Test Warnings** — `biometric_credential_service_test.dart`: replaced deprecated `setMockMethodCallHandler` with `TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger.setMockMethodCallHandler`; removed unused `dart:typed_data` import. `profile_data_test.dart` and `profile_provider_test.dart`: added `const` to `ProfileData()` constructor calls

## [1.4.2] - 2026-04-29

### Fixed

- **Trash Emptying Completeness** — `TrashManager._calculateEmptyTrash()` and `getDeletedItems()` now correctly include `awards` in the professional section
- **Memory Leaks** — Fixed TextEditingController leaks in `ObjectEditorPage` (property field removal) and `ObjectCard` (dummy controller created on every build)
- **Null Safety** — Replaced unsafe `!` operators in `unified_object_provider.dart` children lookups with defensive `whereType<UnifiedObject>()`
- **Error Visibility** — Added error logging to 5 previously silent catch blocks in `native_vault_service.dart` and `native_crypto_service.dart`
- **State Consistency** — `purgeOldDeletedItems()` now returns a new immutable `ProfileData` via `copyWith` instead of mutating the input parameter
- **Operation Log Reliability** — `OperationLogService.addEntry()` now `await`s disk persistence before notifying listeners, preventing log loss on crash
- **Unlock Robustness** — `_handleUnlock()` in login page now wraps `unlockVault()` in try-catch to ensure loading spinner resets on unexpected errors
- **ProfileSectionEditor Safety** — `_markDeletedProfile` and `_markRestoredProfile` now use `identity.copyWith()` instead of fragile manual field reconstruction

### Performance

- **Provider Select Optimization** — `home_page.dart`, `object_editor_page.dart`, and `AccountsVersion` provider now use `.select()` for precise state watching, reducing unnecessary rebuilds

### Removed

- **Dead Code Cleanup** — Removed `UnifiedObjectDataExtension`, `verifyPasswordForRestrictedField`, `animatedSection` helper, `ProfileFieldHistories` typedef, and stale lint suppressions

## [1.4.1] - 2026-04-29

### Fixed

- **Critical Privacy Fix: Cross-Account Data Leakage** — Fixed a severe vulnerability where creating a new account after locking a previous account could display the previous account's data:
  - `main.dart`: Auth state transitions to `locked` now trigger `_wipeSensitiveState()` for both manual and auto-lock paths, ensuring all in-memory sensitive state is cleared
  - `_wipeSensitiveState()`: Now includes `unifiedObjectProvider.reset()` to clear UnifiedObject data previously missed
  - `UnifiedObjectNotifier`: `loadFromProfile()` now resets state to empty when `profile == null` (new account without data)
  - `UnifiedObjectNotifier`: `ref.listen(profileNotifierProvider)` now resets state when profile is cleared to `null` (lock/account switch)

### Performance

- **Isolate Offload for Search** — `SearchProvider._performSearch()` now offloads string matching across all identity/travel/financial/professional/unified fields to a background isolate via `Isolate.run()`, eliminating main-thread jank during search
- **Isolate Offload for Data I/O** — JSON encode/decode and `ProfileData.fromJson()` offloaded to isolates for:
  - `ProfileStorageService.loadProfile()` / `saveProfile()`
  - `BackupService.createBackup()` / `createSpecialBackup()` / `restoreBackup()`
- **Lazy List Rendering** — `AppSidebar`, `TrashPage`, and `ObjectCard` lists converted from eager `ListView` (spread operator) to `ListView.builder`, eliminating pre-build of off-screen children
- **Fine-Grained Object Card Rebuilds** — `_ObjectCardItemTile` now uses `fieldHistoriesProvider.select((h) => h.getHistory(item.id, 'unified'))` so each tile rebuilds independently only when its own history changes

### Fixed

- **Operation Log Live Updates** — `OperationLogProvider` architecture fixed from dead `NotifierProvider<OperationLogServiceNotifier, OperationLogService>` to a version-counter notifier, so entries correctly respond to `addEntry` / `refreshFromDisk` / `clearEntries`

### Changed

- **Widget Decomposition (Code Quality)** — Extracted widgets from god files to improve maintainability (no user-facing changes):
  - `settings_page.dart` (1957 → 1105 lines): `CurrentAccountSheet`, `AllAccountsSheet`, `VersionSheet`, `DebugLogSheet`
  - `profile_storage_service.dart` (3500 → 1865 lines): all 22 model classes moved to `core/models/profile_data.dart` with `@JsonSerializable` codegen
  - `home_page.dart` (1253 → 756 lines): `PageEditor`, `IconPicker`, `DashedPlaceholder`
  - `object_card.dart`: `_ObjectCardHeader`, `_ObjectCardPropertiesList`, `_ObjectCardHistorySection`

## [1.4.0] - 2026-04-29

### Added

- **Startup Data Integrity Validation** — `ProfileStorageService.loadProfile()` now runs `_validateAndRepairProfile()` immediately after migration. Automatically repairs:
  - Duplicate `UnifiedObject` IDs (keeps first occurrence)
  - Invalid `childrenIds` references (removes IDs pointing to non-existent objects)
  - Invalid `parentId` references (sets to `null` if parent no longer exists)
  - Repairs are persisted automatically so they don't re-occur on next load
- **Complete Trash Purge Coverage** — `purgeOldDeletedItemsIfNeeded()` and `purgeOldDeletedItems()` now cover all legacy sections:
  - `travel.travelHistory`
  - `professional.skills`, `professional.languages`, `professional.awards`
  - `identity.idCards`, `identity.addresses`, `identity.contact.entries`
  - `unifiedObjects.objects`
- **Field History Orphan Cleanup** — `FieldHistoryService.cleanupOrphanHistories()` removes history entries for permanently deleted items. Wired into `ProfilePersistenceService.loadProfile()` to run automatically on startup.
- **ProfileData.collectAllItemIds()** — Centralized method collecting all item IDs across legacy sections and unified objects, used for cross-section integrity checks.

### Fixed

- **`_calculateEmptyTrash()` completeness** — Now includes `professional.awards` and `unifiedObjects.objects` in permanent deletion.
- **FormHistory unbounded growth** — History entries for deleted items no longer accumulate indefinitely.

---

## [1.3.0] - 2026-04-29

### Added

- **Encrypted Backup & Restore** (`BackupService`) — Full-screen Data Management page. All backups are encrypted with the vault's AES-256-GCM key. Regular backups auto-rotate (max 5) with version-timestamp filenames.
- **Special Backups** — Up to 5 user-named backups stored outside the rotation cycle. Support rename, restore, and delete. Can be created from current state or promoted from any regular backup.
- **Auto-Backup on Unlock** — `AuthNotifier` fires non-blocking backup creation after every successful vault unlock.
- **Auto-Backup on App Upgrade** — `AppVersionTracker` detects version changes and triggers a versioned backup on the first unlock after upgrade.
- **Backup Recovery Prompt** — `LoginPage` detects empty vault + existing backups and offers a restore dialog before creating default items.
- **Account Data Isolation** — `UnifiedObjectNotifier.loadFromProfile()` now resets state to empty when the new account's `unifiedObjects` is null, preventing old account data from leaking into the new account.
- **Default Page Deletion Protection** — `deleteObject()` blocks soft-deletion of `DefaultPageIds` (profile/travel/financial/professional).
- **Default Page Sidebar Filtering** — `AppSidebar` custom pages list now excludes the four built-in default pages.
- **Operation Notification Overlay** — Backup actions use `OperationNotification.show()` (top-floating overlay) instead of `ScaffoldMessenger` SnackBar.

### Changed

- **Data Management** — Moved from BottomSheet (`settings_page.dart`) to standalone page (`data_management_page.dart`) with `AppBar`, `RefreshIndicator`, and full-screen layout.
- **Restore Backup Order** — `restoreBackup()` now reads the target backup file into memory *before* calling `createBackup()` for the protective backup, preventing cleanup from deleting the file being restored.

### Fixed

- **Restore Oldest Backup Failed** — When 5 regular backups existed, the protective backup's cleanup would delete the oldest backup before it could be read. Fixed by reordering read-before-protect.
- **Date Masking Leak** — `_maskedValue()` threshold was 8 chars, causing `1997-08-19` (10 chars) to show `1997••••••••8-19`. Threshold raised to 12 chars for full masking of dates and short IDs.
- **Object Workspace Pop Crash** — `build()` auto-navigate and `_deleteCurrentObject()` both called `context.pop()`, causing double-pop `GoError`. Removed pop from delete, let build handle navigation.
- **Migration `StateError` Crash** — `_migrateProfileDataToUnified._sens()` used `firstWhere` with `on Exception catch`, but `StateError` (from missing field in `FieldRegistry.defaultFields`) is an `Error`, not `Exception`. Fixed to `catch (_)` and added `FormFieldRegistry.getField()` fallback.
- **Account Switch Data Leak** — New account login would display previous account's custom pages because `UnifiedObjectNotifier` state was never cleared when `profile.unifiedObjects == null`.

---

## [1.2.0] - 2026-04-27

### Added
- **Unified Object Model** - Everything is a `UnifiedObject` with `parentId`/`childrenIds` tree structure. Replaces legacy `FlexibleSection`/`FlexibleItem` models.
- **Persistent Sidebar (`AppSidebar`)** - Drag-resizable (180–400px), collapse/expand, with tree-structured custom pages (expand/collapse for nested sub-pages)
- **Object Workspace Page** - Page-centric UI showing children as cards with inline property editing; non-page children shown as list tiles
- **Object Editor Page** - Generic editor for creating/editing any `UnifiedObject` with icon picker, type selection (create-only), and parent assignment
- **Icon Picker Sheet** - Shared bottom-sheet component for selecting from 26 predefined Material icons
- **Lock Vault Confirmation Dialog** - Unified confirmation dialog before locking, with cancel/confirm buttons and tap-outside-to-dismiss
- **Data Size Display** - Settings page Account section now shows total vault data size (B/KB/MB/GB)
- **Property Editor Factory** - Type-aware inline property editors (text, number, date, checkbox, select, multi-select, relation, URL)
- **CHANGELOG.md** - Version history documentation
- **Rust FFI for Argon2id** (`crypto-argon2/`) - High-performance Argon2id key derivation using Rust SIMD optimizations for Apple Silicon
- **Change Password API** - `POST /api/auth/password` endpoint with full data re-encryption
- **Shared Header Component** - Left sidebar with navigation, account badge, and lock button shared across all auth pages
- **Request Timeout** - 10-second timeout on all API client requests to prevent hanging

### Changed
- **Schema Version** - Bumped to v3; `ProfileData` now uses `unifiedObjects` field for all object storage
- **Home Page** - Simplified to main dashboard only; quick actions fixed at 90×90; inline page editor moved to object workspace
- **Custom Page Trash** - Page-type children no longer display in parent workspace (hierarchy visible only in sidebar tree)
- **Object Editor Save** - Save button moved from AppBar to bottom-centered outline button
- **Multi-account Session Persistence** - `sessionToken` and `currentAccount` are now persisted to localStorage
- **Session Validation** - Fixed closure capture bug in auth page redirects using `getState()` pattern
- **Settings Page** - Change Master Password section is now collapsed by default
- **Dashboard** - Simplified toolbar, account info moved to shared Header
- **Vault ChangePassword** - Now properly re-encrypts all profile data with new key

### Fixed
- **Data Persistence on Login** - `UnifiedObjectNotifier` now auto-loads from `ProfileData` via `ref.listen` and explicit `loadFromProfile()` calls on login
- **Lock Button Stucking** - Added try-finally in settings page to ensure button state is restored on error
- **Change Password Flow** - Wrong current password now returns proper error message instead of hanging
- **SetVaultPath Index Loading** - Fixed missing `loadIndex()` call when switching account vaults
- **Account Deletion Lock** - Delete account now properly locks and redirects to login

### Security
- **Argon2id Performance** - Rust SIMD implementation provides ~3x speedup on Apple M-series chips
- **Password Change Re-encryption** - All vault data is now properly re-encrypted when password changes

---

## [1.1.0] - 2026-04-24

### Added

- **Riverpod 3.0 Upgrade** — Upgraded from Riverpod 2.6.1 to 3.0.3
- **Disable Debug Mode Button** — Added power button in debug log sheet to exit debug mode

### Bug Fixes

- Fixed address save not persisting new entries (missing list update logic)
- Fixed soft delete confirmation dialog not showing (alreadyConfirmed flag was wrong)
- Fixed debug mode being lost on page navigation (provider now uses keepAlive)
- Fixed debug mode password dialog dismissing on outside tap (barrierDismissible: false)

### Technical

- StateNotifier → Notifier migration (4 classes)
- ChangeNotifierProvider → NotifierProvider migration
- Auto-retry disabled in ProviderScope
- All generated code regenerated for Riverpod 3.0 compatibility

---

## [1.0.0] - 2026-04-24

### Added

- **macOS DMG Installer** — Official v1.0.0 release with drag-and-drop installation
- **Debug Mode** — Hidden debug log sheet (tap version 5 times to reveal) with colored log levels
- **Improved Keychain Handling** — Better fallback mechanism for non-notarized distribution
- **Biometric Authentication** — Face ID unlock support with password verification fallback
- **Debug Logger Colors** — Color-coded log levels (INFO: cyan, WARN: yellow, ERROR: red, DEBUG: gray)

### Bug Fixes

- Fixed macOS Keychain probe false-negative issues
- Fixed debug log copy button functionality
- Fixed biometric toggle requiring password verification
- Fixed password dialog ghost overlay when cancelled
- Fixed duplicate hint button in message boxes

### Build

- Non-notarized distribution support (sandbox + identity signing disabled)
- DMG packaging with create-dmg tool

---

## [1.0.0-pre.1] - 2026-04-22

### Added

- **Flutter macOS Application** — Native macOS client with full feature set
- **Zero-Knowledge Security Architecture** — Master password never stored
- **Rust FFI Crypto Core** — High-performance Argon2id + AES-256-GCM via native FFI
- **Profile Management** — Identity, travel, financial, and professional data
- **OCR Scanning** — Auto-extract data from passports, IDs, and visas
- **Four-Tier Sensitivity System** — Public / Private / Sensitive / Critical
- **Operation History** — Full audit trail of all changes including sensitivity settings
- **Multi-Account Support** — Each account with independently encrypted storage
- **Local Storage Only** — All data in `~/.solosoul/`, no cloud sync

### Features

- **Profile Editor** — Intuitive tab-based interface for managing all profile data
- **Travel Module** — Passports, visas, travel history management
- **Financial Module** — Bank accounts, card information
- **Professional Module** — Education, employment, skills, languages
- **Sensitivity Settings** — Per-field sensitivity level configuration
- **Operation Log Page** — Searchable history of all profile and settings changes
- **Password Verification Dialog** — Re-authentication for sensitive operations

### Security

- **Argon2id Key Derivation** — Memory-hard KDF (64MB, 3 iterations)
- **AES-256-GCM Encryption** — Military-grade symmetric encryption
- **Secure Memory Handling** — Sensitive values zeroed after use
- **24-Hour Session Tokens** — Automatic expiry for plugin sessions
- **Plugin Consent System** — Per-field authorization for third-party access

### Technical Stack

| Component | Technology |
|-----------|------------|
| Frontend | Flutter, Riverpod |
| Crypto Core | Rust, Argon2id, AES-256-GCM |
| Backend | Go, Gin |
| Storage | Local encrypted files |

### Known Issues

- macOS only (other platforms coming soon)
- Touch ID not yet functional

---

## [0.1.0] - 2026-04-09

### Added
- Core crypto: Argon2id KDF, AES-256-GCM encryption, secure memory
- Vault storage system with file-based implementation
- CLI tool: init, unlock, lock, status, profile commands
- gRPC API server for vault operations
- Plugin management system with consent flow
- OCR module with MRZ parsing and PaddleOCR adapter
- Next.js web UI with login, dashboard, profile editor, vault, OCR, plugins, settings pages
- Multi-account support with independent vault directories
- Comprehensive test suite

[Unreleased]: https://github.com/Gczmy/SoloSoul/compare/v2.5.6...HEAD
[2.5.6]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.5.6
[2.5.5]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.5.5
[2.5.4]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.5.4
[2.5.3]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.5.3
[2.5.2]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.5.2
[2.5.1]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.5.1
[2.5.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.5.0
[2.4.1]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.4.1
[2.3.3]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.3.3
[2.3.2]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.3.2
[2.3.1]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.3.1
[2.3.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.3.0
[2.2.2]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.2.2
[2.2.1]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.2.1
[2.2.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.2.0
[2.1.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.1.0
[2.0.1]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.0.1
[2.0.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v2.0.0
[1.8.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.8.0
[1.7.1]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.7.1
[1.7.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.7.0
[1.6.6]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.6.6
[1.6.5]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.6.5
[1.6.4]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.6.4
[1.6.3]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.6.3
[1.6.2]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.6.2
[1.6.1]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.6.1
[1.5.1]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.5.1
[1.5.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.5.0
[1.4.9]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.4.9
[1.4.8]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.4.8
[1.4.7]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.4.7
[1.4.6]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.4.6
[1.4.5]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.4.5
[1.4.4]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.4.4
[1.4.3]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.4.3
[1.4.2]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.4.2
[1.4.1]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.4.1
[1.4.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.4.0
[1.3.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.3.0
[1.2.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.2.0
[1.1.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.1.0
[1.0.0-pre.1]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.0.0-pre.1
[1.0.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.0.0
[0.1.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v0.1.0
