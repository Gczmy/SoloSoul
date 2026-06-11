# Session TODO — Bug 修复与功能优化

> 创建时间: 2026-06-09
> 状态跟踪: 进行中

---

## Bug 修复

### [ ] Bug 1: LLM 配置和 LLM 使用统计页面国际化
- **问题**: LLM 配置页面（`LLMConfigPage`）和 LLM 使用统计页面（`LLMUsageStatsPage`）中的文本为硬编码中文/英文，未走 i18n 系统。
- **目标**: 将所有硬编码文本提取到国际化资源文件中，支持中英文切换。
- **涉及文件**:
  - `tauri/src/pages/settings/LLMConfigPage.tsx`
  - `tauri/src/pages/settings/LLMUsageStatsPage.tsx`
  - `tauri/public/locales/zh/settings.json`
  - `tauri/public/locales/en/settings.json`

### [ ] Bug 2: LLM 使用统计数据持久化（加密保存到保险库）
- **问题**: LLM 使用统计页面的数据目前似乎仅在内存中，页面刷新后数据丢失。
- **目标**: 将 LLM 使用统计数据视为用户重要数据，加密后持久化保存到 Vault（SQLite）。
- **涉及文件**:
  - `tauri/src-tauri/src/commands/llm.rs`（或相关 LLM 命令模块）
  - `tauri/src-tauri/src/db/`（数据库迁移）
  - `tauri/src/pages/settings/LLMUsageStatsPage.tsx`
  - `tauri/src/stores/`（状态管理）

### [ ] Bug 3: 软删除页面有时不显示在回收站
- **问题**: 软删除自定义页面后，有概率该页面不会出现在回收站中。锁定账户重新登录后，侧边栏又能看到这个页面，说明实际上没被软删除，只是在侧边栏消失了。
- **分析方向**:
  1. `removeCustomPage` 调用是否成功？是否处理了异步错误？
  2. `page_delete` IPC 命令是否正确执行了软删除？
  3. `loadCustomPages` 的 `includeDeleted: true` 是否生效？
  4. 前端状态更新后是否触发了重新渲染？Zustand 订阅是否正常？
- **涉及文件**:
  - `tauri/src/stores/settingsStore.ts`
  - `tauri/src-tauri/src/commands/page.rs`
  - `tauri/src/components/layout/SideNavigation.tsx`

---

## 功能开发

### [ ] 功能 1: 主题与外观页面加入侧边栏位置配置
- **需求**: 侧边栏目前只能在左边，允许用户配置显示在上边、下边、左边（默认）、右边。
- **布局规则**:
  - 上/下边时：按钮横向排布
  - 上方按钮（首页、身份、财务、职业、旅行、自定义页面、加号）在左侧
  - 下方按钮（设置、主题、锁定等）在右侧
- **目标**: 用户选择后立即生效。
- **涉及文件**:
  - `tauri/src/pages/settings/AppearancePage.tsx`
  - `tauri/src/components/layout/SideNavigation.tsx`
  - `tauri/src/stores/settingsStore.ts`（添加 `sidebarPosition` 配置）
  - `tauri/src/App.tsx` 或布局组件

### [ ] 功能 2: 系统标题栏视觉一致 + 内部搜索栏（类似 Warp）
- **需求**: 像 Warp 终端一样，将系统的关闭/最小化/缩放按钮行融入应用主题，并增加软件内部功能如搜索栏。
- **技术方案**: Tauri 支持 `data-tauri-drag-region` 和自定义标题栏（`titleBarStyle: 'Overlay'` 或完全自定义）。
- **涉及文件**:
  - `tauri/src-tauri/tauri.conf.json`（标题栏配置）
  - `tauri/src/App.tsx`
  - `tauri/src/components/layout/`（新标题栏组件）

### [ ] 功能 3: 生物识别锁定时避免密码页面闪烁
- **问题**: 开启 Touch ID 后，锁定账户时会先显示密码输入页面，再快速跳转到生物识别登录页面，导致闪烁。
- **目标**: 如果开启了生物识别功能，锁定账户后直接跳转到生物识别登录页面。
- **涉及文件**:
  - `tauri/src/App.tsx`（路由/状态判断）
  - `tauri/src/pages/auth/LockScreenPage.tsx`（或相关锁定页面）

### [ ] 功能 4: 生物识别解锁后直接跳转首页避免闪烁
- **问题**: 使用生物识别登录时，从"验证中"跳到"使用 Touch ID 解锁 SoloSoul"，再跳转到首页，闪烁明显。
- **目标**: 识别通过后直接跳转到账户首页，避免中间状态页面。
- **涉及文件**:
  - `tauri/src/pages/auth/`（生物识别相关页面）
  - `tauri/src/App.tsx`

---

## UI/UX 优化

### [ ] 优化 1: 回收站搜索栏 placeholder
- **问题**: 回收站搜索栏没有输入时显示 "搜索日志…"
- **目标**: 改为 "搜索项目…"
- **涉及文件**:
  - `tauri/src/pages/workspace/TrashPage.tsx`
  - 国际化资源文件

### [ ] 优化 2: 回收站卡片悬停浮动动画和高亮
- **需求**: 鼠标悬停时卡片有浮动动画（轻微上移 + 阴影增强）和高亮效果。
- **涉及文件**:
  - `tauri/src/pages/workspace/TrashPage.tsx`（卡片样式）
  - `tauri/src/components/`（可能提取为共享组件）

### [ ] 优化 3: 回收站卡片整卡可点击
- **问题**: 回收站项目卡片部分区域点击后没反应。
- **目标**: 整个卡片区域都可点击进入详情。
- **涉及文件**:
  - `tauri/src/pages/workspace/TrashPage.tsx`

---

## 完成标准

- [ ] 所有 Bug 修复完成并通过测试
- [ ] 所有功能开发完成并编译通过
- [ ] 所有 UI 优化完成
- [ ] 每次完成一项任务后更新本 TODO 并推送代码
- [ ] 全部完成后撰写总结报告
