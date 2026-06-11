# Session TODO — Bug 修复与功能优化

> 创建时间: 2026-06-09
> 状态跟踪: 进行中

---

## Bug 修复

### [x] Bug 1: LLM 配置和 LLM 使用统计页面国际化
- **问题**: LLM 配置页面（`LlmConfigPage`）和 LLM 使用统计页面（`LlmStatsPage`）中的文本为硬编码中文/英文，未走 i18n 系统。
- **修复**: 将所有硬编码文本提取到国际化资源文件 `settings.json`（zh-CN / en-US），并修改组件使用 `t()` 调用。LLM 子组件（`ModelInfoCard`, `StatsGrid`, `AccountStatsCard`, `TokenBreakdownCard`, `DailySparklineCard`, `ModelUsageCard`）通过 `TFunction` prop 接收翻译函数。
- **涉及文件**: `tauri/src/pages/ai/LlmConfigPage.tsx`, `tauri/src/pages/ai/LlmStatsPage.tsx`, `tauri/src/components/llm/*.tsx`, `tauri/src/locales/*/settings.json`

### [x] Bug 2: LLM 使用统计数据持久化（加密保存到保险库）
- **问题**: LLM 使用统计数据仅在内存中维护（`STATS_MAP`），应用重启/锁定后丢失。
- **修复**: `save_stats_to_vault` / `load_stats_from_vault` 已实现但未调用。在 `llm_send_message_stream` 和 `llm_chat` 命令中，`record_usage` 之后立即调用 `save_stats_to_vault` 将内存统计持久化到 Vault（Profile preferences 中的 `llmUsageStats` 字段）。
- **涉及文件**: `tauri/src-tauri/src/commands/llm.rs`

### [x] Bug 3: 软删除页面有时不显示在回收站
- **问题**: 软删除自定义页面后，页面从侧边栏消失但回收站里没有；锁定重新登录后页面又出现。
- **根因分析**: `addCustomPage` 前端使用 `crypto.randomUUID()` 生成 id，但 `object_create` 后端也生成新的 `Uuid::new_v4()`，导致前端 id 与数据库 id 不一致。`page_delete` 使用前端 UUID 查找对象时找不到记录，软删除未执行。
- **修复**: `CreateObjectInput` 添加可选 `id` 字段；`object_create` 优先使用客户端提供的 id；`addCustomPage` 将前端 id 传入后端。
- **涉及文件**: `tauri/src-tauri/src/commands/object.rs`, `tauri/src/stores/settingsStore.ts`

---

## 功能开发

### [x] 功能 1: 主题与外观页面加入侧边栏位置配置
- **需求**: 侧边栏目前只能在左边，允许用户配置显示在上边、下边、左边（默认）、右边。
- **实现**:
  - `AppSettings` 添加 `sidebarPosition: 'left' | 'right' | 'top' | 'bottom'`
  - `AppearanceSettingsPage` 添加 4 格网格选择器（带图标）
  - `SideNavigation` 根据位置动态调整 `flex-direction`、`width/height`、border、按钮排列
  - `AppShell` 根据位置调整 `flex-direction`（`row`/`row-reverse`/`column`/`column-reverse`）
  - 上/下时：primary 按钮在左，secondary 按钮在右，横向排布
  - 国际化：`settings:sidebar_position`, `settings:sidebar_left/right/top/bottom`
- **涉及文件**: `tauri/src/stores/settingsStore.ts`, `tauri/src/pages/settings/AppearanceSettingsPage.tsx`, `tauri/src/components/layout/SideNavigation.tsx`, `tauri/src/components/layout/AppShell.tsx`, `tauri/src/locales/*/settings.json`

### [x] 功能 2: 系统标题栏视觉一致 + 内部搜索栏（类似 Warp）
- **需求**: 将系统的关闭/最小化/缩放按钮融入应用主题，并增加内部搜索栏。
- **实现**:
  - `tauri.conf.json`: `titleBarStyle: "Transparent"` + `transparent: true`
  - macOS 上自动检测 `navigator.platform`，添加 28px 标题栏偏移
  - `AppBar` 顶部增加 `data-tauri-drag-region` 支持拖拽；添加搜索图标按钮（跳转到 `/search`）
  - `SideNavigation` 和 `AppShell` 在 macOS 上自动调整 `padding-top` 为交通灯按钮留出空间
- **涉及文件**: `tauri/src-tauri/tauri.conf.json`, `tauri/src/components/layout/AppBar.tsx`, `tauri/src/components/layout/AppShell.tsx`, `tauri/src/components/layout/SideNavigation.tsx`

### [x] 功能 3: 生物识别锁定时避免密码页面闪烁
- **问题**: 开启 Touch ID 后，锁定账户时会先显示密码输入页面，再快速跳转到生物识别登录页面，导致闪烁。
- **根因**: `LoginPage` 初始渲染时 `bioAvailable` 为 `false`，`!bioAvailable` 为 `true`，密码表单先显示；随后 bio 检查完成切换到生物识别 UI。
- **修复**: 添加 `bioChecked` 状态，在生物识别可用性检查完成前显示与背景同色的占位符（不渲染任何登录 UI），检查完成后一次性渲染正确界面。
- **涉及文件**: `tauri/src/pages/auth/LoginPage.tsx`

### [x] 功能 4: 生物识别解锁后直接跳转首页避免闪烁
- **问题**: 使用生物识别登录时，从"验证中"跳到"使用 Touch ID 解锁 SoloSoul"，再跳转到首页，闪烁明显。
- **修复**: `handleBiometricUnlock` 成功解锁并设置 `isAuthenticated` 后，立即调用 `navigate('/')`，而不是等待 `useEffect` 检测到 `isAuthenticated` 变化后再导航。
- **涉及文件**: `tauri/src/pages/auth/LoginPage.tsx`

---

## UI/UX 优化

### [x] 优化 1: 回收站搜索栏 placeholder
- **修复**: `TrashPage` 搜索栏 placeholder 从 `settings:search_logs` 改为 `settings:search_trash`。
- **国际化**: zh-CN: "搜索项目...", en-US: "Search items..."
- **涉及文件**: `tauri/src/pages/settings/TrashPage.tsx`, `tauri/src/locales/*/settings.json`

### [x] 优化 2: 回收站卡片悬停浮动动画和高亮
- **修复**: `Card` 组件添加 `style` prop 支持。TrashPage 卡片设置 `interactive` + `style={{ cursor: 'pointer', transition: 'transform 0.15s ease, box-shadow 0.15s ease' }}`，`Card.module.css` 中 `.interactive` 已有悬停阴影增强效果。
- **涉及文件**: `tauri/src/components/ui/Card.tsx`, `tauri/src/pages/settings/TrashPage.tsx`

### [x] 优化 3: 回收站卡片整卡可点击
- **修复**: 将 `onClick={() => openDetail(item.id)}` 从内部 div 移到 `Card` 上；按钮和 checkbox 的点击事件调用 `e.stopPropagation()` 防止触发卡片点击。
- **涉及文件**: `tauri/src/pages/settings/TrashPage.tsx`

---

## 完成状态

- [x] 所有 Bug 修复完成并编译通过
- [x] 所有功能开发完成并编译通过
- [x] 所有 UI 优化完成并编译通过
- [x] 每次完成一项任务后推送代码（共 6 次提交）
- [x] 全部完成后撰写总结报告
