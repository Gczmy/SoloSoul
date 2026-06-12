# SoloSoul 需求与 Bug 修复实施计划

> 生成时间：2026-06-12  
> 范围：Tauri + React 客户端（`tauri/` 目录）  
> 计划性质：详细实施报告，含当前状态、修改方案、文件清单、i18n 与验收标准  

---

## 1. 总体说明

### 1.1 技术栈与约束
- 前端：React 19 + TypeScript + Vite + Zustand + i18next  
- 后端：Tauri v2 + Rust 2021  
- 状态持久化分两层：  
  - **明文 UI 偏好**：`~/.solosoul/ui_preferences.json`，可在登录前读取（主题、语言等）。  
  - **加密账户数据**：Vault 解锁后读取（`user_data_get_preferences`）。  
- 敏感数据加密模型：主密码不存储，Salt/验证令牌存 `config.json`。

### 1.2 实施原则
1. **最小侵入**：复用现有 `PAGE_ICON_MAP`、`Button`、`Card`、`Dialog`、审计日志体系。  
2. **单一来源**：图标、模板、敏感度继续以现有中心文件为唯一来源。  
3. **登录前可用数据走 `ui_preferences.json`**：主题、语言、窗口大小等非敏感项。  
4. **操作日志统一走 `log_structured`**：动作类型、实体类型、详情三段式。  
5. **i18n 同步更新 zh-CN / en-US**。

---

## 2. 需求与 Bug 逐项实施计划

### 2.1 侧边栏下方功能按钮客制化（3 个可变位置）

#### 当前状态
- `src/components/layout/useNavigationItems.ts` 中 `secondaryItems` 写死 5 项：锁定、搜索、插件、AI 对话、设置。  
- `SideNavigation.tsx` 的 `navSecondary` 按数组顺序渲染，无“固定底部”逻辑。  
- 所有页面路由已存在：`/plugins`、`/llm-chat`、`/search`、`/settings/trash`、`/help`、`/settings/templates`、`/settings/export-import`。

#### 修改方案
1. **新增设置项**：`AppSettings.sidebarBottomActions: [string, string, string]`，表示从上到下的 3 个可变按钮 ID。  
   - 默认值：`['search', 'plugins', 'ai_chat']`。  
   - 候选 ID：`plugins`、`ai_chat`、`search`、`trash`、`help`、`templates`、`import_export`。  
2. **固定顺序**：可变 3 项始终渲染在 `navSecondary` 上半区；`lock` 次之；`settings` 永远在最底部。  
3. **渲染改造**：  
   - `useNavigationItems.ts` 拆分 `secondaryItems` 为：  
     - `CUSTOMIZABLE_ACTIONS` 定义（ID → iconKey/labelKey/onClick 工厂）。  
     - `useBoundNavActions()` 读取 `settings.sidebarBottomActions` 并按顺序绑定动作。  
   - `SideNavigation.tsx`：`navSecondary` 显式分三段渲染：3 个可变按钮 → 锁定 → 设置。  
   - `TopFunctionBar.tsx`（顶部/底部侧边栏模式）同步改造，保持行为一致。  
4. **新增设置 UI**：在 `AppearanceSettingsPage.tsx` 增加“侧边栏按钮”卡片，使用拖拽或下拉选择 3 个位置。  
5. **图标补充**：`src/lib/pageIcons.ts` 的 `PAGE_ICON_MAP` 补全 trash/help/templates/import_export 图标。

#### 涉及文件
- `src/stores/settingsStore.ts`（类型 + 默认值）  
- `src/components/layout/useNavigationItems.ts`  
- `src/components/layout/SideNavigation.tsx`  
- `src/components/layout/TopFunctionBar.tsx`  
- `src/pages/settings/AppearanceSettingsPage.tsx`  
- `src/lib/pageIcons.ts`  
- `src/locales/zh-CN/navigation.json`、`src/locales/en-US/navigation.json`  
- `src/locales/zh-CN/settings.json`、`src/locales/en-US/settings.json`

#### i18n 新增
```json
{
  "navigation": {
    "trash": "回收站",
    "help": "帮助文档",
    "templates": "模板管理",
    "import_export": "导入与导出"
  },
  "settings": {
    "sidebar_bottom_actions": "侧边栏功能按钮",
    "sidebar_slot_1": "第一位",
    "sidebar_slot_2": "第二位",
    "sidebar_slot_3": "第三位"
  }
}
```

#### 验收标准
- 锁定账户按钮倒数第二，设置按钮永远在最下方。  
- 3 个可变位置可在设置页修改，保存后即时生效。  
- AI 对话按钮在 `/llm-chat` 外点击仍弹出快速聊天卡片；搜索按钮仍弹出 `SearchPopover`。  
- 顶部/底部模式与左侧/右侧模式行为一致。

---

### 2.2 LLM 使用统计页面“重置统计”按钮深色模式显示优化

#### 当前状态
- `src/pages/ai/LlmStatsPage.tsx:158-166` 使用 `variant="secondary"` 并硬编码 `color: '#e74c3c'; borderColor: '#e74c3c'`。  
- `Button.module.css:31-39` 的 `.secondary` 使用固定 `rgba(255,255,255,0.2)` 背景，深色模式下发灰、hover 几乎消失。

#### 修改方案
- 将重置按钮改为 `variant="danger"`，移除硬编码红色 inline style；危险操作语义一致，且 `.danger` 有独立背景色。  
- 同时修复 `.secondary` 的背景为 `var(--bg-subtle)`、`hover` 为 `var(--bg-hover)`，使其在深色模式下可辨。

#### 涉及文件
- `src/pages/ai/LlmStatsPage.tsx`  
- `src/components/ui/Button.module.css`

#### 验收标准
- 深色模式下重置统计按钮清晰可见、hover 状态明显。  
- 浅色模式外观不劣化。

---

### 2.3 窗口大小加密保存并在登录前加载

#### 当前状态
- 窗口大小存储在加密账户偏好中（`user_data_update_preference`），登录后通过 `restoreWindowSize(accountId)` 恢复。  
- 主题/外观存储在明文 `ui_preferences.json`，可在 `main.tsx` 登录前加载。  
- **关键矛盾**：“加密保存”与“登录前加载”在技术上是冲突的：加密账户数据必须 Vault 解锁后才能读取。

#### 推荐方案（方案 A）
**将窗口大小视为非敏感 UI 偏好，迁移到 `ui_preferences.json`**，与主题/language 同一套机制。  
1. `src-tauri/src/commands/settings.rs` 的 `ui_get_preferences` / `ui_update_preference` 增加 `windowSize` 字段。  
2. `src/hooks/useWindowSize.ts`：  
   - `restoreWindowSize()` 不再依赖 `accountId`，改为读取 `ui_get_preferences`。  
   - `useWindowSize()` 监听 resize，调用 `ui_update_preference` 保存。  
3. `src/main.tsx`：在 `loadUiPreferences()` 后调用 `restoreWindowSize()`，确保登录前应用。  
4. 同步 localStorage 缓存，避免 IPC 延迟。

#### 涉及文件
- `src-tauri/src/commands/settings.rs`  
- `src/hooks/useWindowSize.ts`  
- `src/main.tsx`  
- `src/App.tsx`（移除登录后 `restoreWindowSize`）  
- `src/stores/settingsStore.ts`（可选：若需回写 UI 偏好）

#### 验收标准
- 登录页面出现前窗口已恢复为上次关闭时大小。  
- 调整窗口大小后 500ms 自动保存，下次启动生效。  
- 数据持久在 `~/.solosoul/ui_preferences.json`。

---

### 2.4 登录页面按钮加入浮动动画和边框高亮

#### 当前状态
- 登录页 `LoginPage.tsx` 使用原生 `<button>`：生物识别解锁大按钮、“使用密码登录”文本按钮、“使用生物识别”文本按钮。  
- 密码提交使用 `Button` 组件（带背景色）；密码输入框内有眼睛/提示按钮。

#### 修改方案
- 仅对以下登录按钮增加动画与高亮：  
  1. 生物识别解锁大按钮（指纹图标 + “使用 Touch ID 解锁 SoloSoul”）。  
  2. “使用密码登录”切换按钮。  
  3. “使用 Touch ID”切换按钮。  
- 不动：  
  - `SecurePasswordInput` 内部的眼睛/提示按钮。  
  - 带背景色的密码“解锁”提交 `Button`。  
- 新增 CSS 类（可放 `src/styles/global.css` 或独立 `LoginPage.module.css`）：  
  - `.login-float-button`：`@keyframes float`（轻微上下浮动）。  
  - `.login-glow-button`：hover 时 `box-shadow` 边框高亮，使用 `var(--accent-primary)`。  
- 为可定制按钮添加 CSS 模块，避免全局污染。

#### 涉及文件
- `src/pages/auth/LoginPage.tsx`  
- 新增 `src/pages/auth/LoginPage.module.css`

#### 验收标准
- 指定按钮有持续轻微浮动动画和 hover 边框发光。  
- 输入框按钮、密码提交按钮样式不变。  
- 动画不过度，不影响可访问性。

---

### 2.5 侧边栏滚动 UI 一致性（以 top/bottom 为基准）

#### 当前状态
- `SideNavigation.module.css` 中左侧/右侧模式使用默认滚动条（`overflow: auto`）。  
- 顶部/底部模式使用隐藏滚动条、hover 显示的风格。

#### 修改方案
- 统一左侧/右侧 `navPrimary` 的滚动条样式为与顶部/底部一致：  
  - 默认隐藏滚动条（`scrollbar-width: none; -ms-overflow-style: none; ::-webkit-scrollbar { display: none; }`）。  
  - hover 时显示，支持平滑滚动。  
- 同步检查 `TopFunctionBar.module.css` 的 `.scrollablePages`，作为基准样式。

#### 涉及文件
- `src/components/layout/SideNavigation.module.css`  
- `src/components/layout/TopFunctionBar.module.css`（基准参考，可能微调）

#### 验收标准
- 左侧/右侧侧边栏的可滚动区域在默认状态下无滚动条，hover/滚动时出现。  
- 顶部/底部模式外观不变。

---

### 2.6 密码登录操作加入操作日志与调试日志

#### 当前状态
- `src-tauri/src/commands/auth.rs:login` 仅调用 `svc.unlock(...)`，成功后不写日志。  
- 生物识别解锁在 `biometric.rs` 中写入 `biometric_unlock` 日志。

#### 修改方案
- 在 `login` 命令成功解锁后，通过 `vault.log_structured` 写入：  
  - `action_type`: `login`  
  - `entity_type`: `auth`  
  - `entity_id`: `account_id`  
  - `details`: `method=password location=login_page action=unlock`  
- 同步前端 `OperationLogPage.tsx` 的 `ALL_ENTITY_TYPES` 无需新增（已有 `auth` 或加入 `auth`）。

#### 涉及文件
- `src-tauri/src/commands/auth.rs`  
- `src/locales/zh-CN/settings.json`、`src/locales/en-US/settings.json`  
- `src/pages/settings/OperationLogPage.tsx`（如 `auth` 不在 `ALL_ENTITY_TYPES` 则加入）

#### i18n 新增
```json
{
  "settings": {
    "log": {
      "action": { "login": "密码登录" },
      "entity": { "auth": "认证" },
      "detail": { "login": "{{location}} — {{action}}（{{method}}）" }
    }
  }
}
```

#### 验收标准
- 使用密码登录后，操作日志出现“密码登录 认证：{accountId} 登录页面 — 解锁（password）”。

---

### 2.7 生物识别解锁记录显示具体类型（Touch ID / Face ID）

#### 当前状态
- `biometric_check_availability` 在 macOS 返回 `touchId`；目前 `biometric_unlock` 命令只记录 `biometric_unlock`。  
- 用户看到“生物识别解锁生物识别 2026/6/11 19:51:32 登录页面 — 解锁”。

#### 修改方案（推荐）
- 区分动作类型：  
  - Touch ID 解锁 → `touch_id_unlock`  
  - Face ID 解锁 → `face_id_unlock`  
- 前端 `LoginPage.tsx` 调用 `biometric_unlock` 时传入 `biometryType` 原始值（`touchId` / `faceId`）。  
- 后端 `biometric_unlock` 接收 `biometry_type` 参数，根据类型写入不同 `action_type`。  
- 同步 `biometric_saved` / `biometric_deleted` 也可区分类型（可选）。

#### 涉及文件
- `src-tauri/src/commands/biometric.rs`  
- `src/pages/auth/LoginPage.tsx`  
- `src/pages/settings/SecuritySettingsPage.tsx`（如相关）  
- `src/locales/zh-CN/settings.json`、`src/locales/en-US/settings.json`

#### i18n 新增
```json
{
  "settings": {
    "log": {
      "action": {
        "touch_id_unlock": "Touch ID 解锁",
        "face_id_unlock": "Face ID 解锁"
      }
    }
  }
}
```

#### 验收标准
- Touch ID 解锁日志显示“Touch ID 解锁”。  
- Face ID 解锁日志显示“Face ID 解锁”（未来平台扩展时直接生效）。

---

### 2.8 编辑对象页面验证失败提示更精准

#### 当前状态
- `ObjectEditorPage.tsx:235-237`：`onError(t('editor:validation_failed'), t('common:save'))`。  
- 提示条显示“保存失败：字段类型填写错误，请检查后重试”或中文“验证失败：验证失败”。  
- 字段级错误已存在，但 toast 文案泛化。

#### 修改方案
- 新增 i18n key：`editor:save_failed_validation` → “保存失败：部分字段类型填写错误，请检查后重试”。  
- 更新 toast 调用：  
  ```ts
  onError(t('editor:save_failed_validation'), t('common:save'));
  ```  
- 如需要更友好，可在 toast message 中附带错误字段数量（例如“保存失败：3 个字段填写有误”）。

#### 涉及文件
- `src/pages/editor/ObjectEditorPage.tsx`  
- `src/locales/zh-CN/editor.json`、`src/locales/en-US/editor.json`

#### i18n 新增/修改
```json
{
  "editor": {
    "save_failed_validation": "保存失败：部分字段类型填写错误，请检查后重试",
    "save_failed_validation_n": "保存失败：{{count}} 个字段填写有误"
  }
}
```

#### 验收标准
- 链接/邮箱字段验证失败时，toast 显示为“保存失败：部分字段类型填写错误，请检查后重试”。  
- 字段级内联错误（“请输入有效的链接地址”等）保留。

---

### 2.9 Windows 系统标题栏跟随主题颜色

#### 当前状态
- `src-tauri/tauri.conf.json` 中 `decorations: true`、`titleBarStyle: "Transparent"`。  
- `src-tauri/src/commands/window.rs` 的 `set_titlebar_color` 仅 macOS 实现，Windows 分支为空操作。

#### 修改方案
- 在 Windows 下调用 DWM API：`DwmSetWindowAttribute(DWMWA_CAPTION_COLOR)` 设置标题栏颜色。  
- 需要：  
  - `Cargo.toml` 增加 `windows` crate（或 Tauri 已暴露的 HWND 能力）。  
  - `window.rs` 中获取窗口 HWND，调用 DWM。  
- 颜色来源：`theme.ts` 的 `syncTitleBarColor` 已根据当前 scheme 的 `--bg-base` 计算 RGB，无需前端改动。

#### 涉及文件
- `src-tauri/src/commands/window.rs`  
- `src-tauri/Cargo.toml`  
- `src/lib/theme.ts`（无需大改，确保 Windows 也调用 `set_titlebar_color`）

#### 验收标准
- Windows 下标题栏颜色随主题深浅/强调色变化。  
- macOS 标题栏行为不变。

---

### 2.10 操作日志中“创建页面”条目国际化与类型颜色区分

#### 当前状态
- 创建页面时调用 `object_create`，`entity_type` 为 `object`，details 为 `section=page`。  
- `OperationLogPage.tsx` 中实体类型显示统一颜色（action 颜色），无按 entity 类型区分。

#### 修改方案
1. **后端区分页面创建**：  
   - 在 `src-tauri/src/commands/object.rs` 创建对象时，若 `collection_type == "page"`，使用 `entity_type: "page"` 而不是 `"object"`。  
   - 或新增 `action_type: "page_create"` 与 `entity_type: "page"`。  
2. **前端颜色区分**：  
   - 在 `OperationLogPage.tsx` 为 `page` 实体类型增加独立颜色（如紫色/蓝色），与 `object` 的绿色区分。  
   - 实体类型 badge 颜色从 action 颜色改为 entity 颜色。  
3. **i18n**：  
   - 已有 `settings:log.entity.page` 可翻译为“页面”。

#### 涉及文件
- `src-tauri/src/commands/object.rs`  
- `src/pages/settings/OperationLogPage.tsx`  
- `src/locales/zh-CN/settings.json`、`src/locales/en-US/settings.json`

#### 验收标准
- 创建页面日志显示“创建页面 页面：123”。  
- 页面类型使用与“对象”不同的颜色徽章。  
- 对象类型仍显示“创建对象 对象：123”并保持绿色。

---

### 2.11 欢迎界面与安全设置页加入主密码警告

#### 当前状态
- `BootstrapPage.tsx` 创建账户时只有密码规则提示，无主密码遗失警告。  
- `SecuritySettingsPage.tsx` 修改密码区域无警告。

#### 修改方案
- **BootstrapPage.tsx**：在“创建账户”按钮上方增加红色/橙色警告文本：  
  > “请妥善保管您的主密码，如果遗失将无法解锁保险库，所有数据将无法找回！”  
- **SecuritySettingsPage.tsx**：在“修改密码”卡片顶部增加同一段警告文本。  
- 使用 `var(--accent-danger)` 或橙色，增加 `ShieldAlert` / `AlertTriangle` 图标。

#### 涉及文件
- `src/pages/auth/BootstrapPage.tsx`  
- `src/pages/settings/SecuritySettingsPage.tsx`  
- `src/locales/zh-CN/auth.json`、`src/locales/en-US/auth.json`  
- `src/locales/zh-CN/settings.json`、`src/locales/en-US/settings.json`

#### i18n 新增
```json
{
  "auth": { "master_password_warning": "请妥善保管您的主密码，如果遗失将无法解锁保险库，所有数据将无法找回！" },
  "settings": { "master_password_warning": "请妥善保管您的主密码，如果遗失将无法解锁保险库，所有数据将无法找回！" }
}
```

#### 验收标准
- 两个页面均显示醒目警告文案。  
- 文案支持中英文切换。

---

### 2.12 安全设置页自动锁定项加入功能说明

#### 当前状态
- `SecuritySettingsPage.tsx` 的自动锁定 select 未绑定状态（非受控组件），且无说明文本。

#### 修改方案
1. 将 select 改为受控，绑定 `settings.autoLockTimeoutMinutes`，`onChange` 调用 `updateSetting`。  
2. 在标题下方增加说明文本：  
   > “设置多长时间无操作后自动锁定保险库并擦除内存中的敏感状态。”  
3. 选项值对齐：`1` / `5` / `15` / `30` / `0`（0=从不）。

#### 涉及文件
- `src/pages/settings/SecuritySettingsPage.tsx`  
- `src/locales/zh-CN/settings.json`、`src/locales/en-US/settings.json`

#### i18n 新增
```json
{
  "settings": {
    "auto_lock_description": "设置多长时间无操作后自动锁定保险库并擦除内存中的敏感状态。"
  }
}
```

#### 验收标准
- 自动锁定下拉框正确反映当前设置。  
- 修改后保存到账户偏好并即时生效。  
- 说明文本清晰可见。

---

### 2.13 模板管理页面加入模板示例按钮与 8 个示例

#### 当前状态
- `TemplateManagerPage.tsx` 显示用户模板列表，支持新建/编辑/删除。  
- 系统模板种子在 `src-tauri/resources/system_templates_*.json`，创建账户时导入。  
- `TemplatePreview.tsx` 组件存在但似乎未被使用。

#### 修改方案
1. **右上角新增“模板示例”按钮**。  
2. 点击后弹出卡片/Dialog，展示 8 个示例模板卡片：  
   - 身份信息、身份证、护照、签证、银行账户、银行卡、教育信息、工作信息。  
3. 每个示例卡片显示：模板名字、所属页面、字段数量、包含的敏感度等级（`SensitivityBadges`）。  
4. 点击示例卡片弹出详情卡片，展示具体字段信息（字段名、类型、敏感度）。  
5. 数据定义：  
   - 在 `src/lib/sampleTemplates.ts` 中硬编码 8 个示例模板（结构同 `UserTemplate`）。  
   - 或复用系统模板 JSON（推荐：复用 `system_templates_zh.json` / `system_templates_en.json` 的数据）。  
6. 可选增强：提供“以此模板创建”按钮，直接生成用户模板（降低用户创建成本）。

#### 涉及文件
- `src/pages/settings/TemplateManagerPage.tsx`  
- 新增 `src/components/template/SampleTemplateGallery.tsx`  
- 新增 `src/components/template/SampleTemplateDetail.tsx`  
- 新增/复用 `src/lib/sampleTemplates.ts`  
- `src/locales/zh-CN/settings.json`、`src/locales/en-US/settings.json`

#### 验收标准
- 模板管理页右上角有“模板示例”按钮。  
- 弹出 8 个示例卡片，信息完整。  
- 点击卡片可看字段详情。  
- UI 风格与现有模板卡片一致。

---

### 2.14 软件内检测新版本并更新

#### 当前状态
- `AboutPage.tsx` 使用自定义 GitHub Releases API 检查版本并手动下载安装包。  
- 无 `tauri-plugin-updater`。

#### 方案选择（推荐方案 A：增强当前自定义更新）
- 启动时自动调用 `check_version`。  
- 有更新时显示非侵入式提示条（如顶部 banner 或 Toast）。  
- 点击后进入 `AboutPage` 下载，下载完成后提示打开安装包。  
- 增加“跳过此版本”/“稍后提醒”。

#### 涉及文件
- `src/pages/system/AboutPage.tsx`  
- `src-tauri/src/commands/system.rs`  
- `src/App.tsx`（启动时检查更新）  
- `src/components/ui/UpdateBanner.tsx`（新增）  
- `src/locales/zh-CN/common.json`、`src/locales/en-US/common.json`

#### 验收标准
- 启动后自动检测新版本。  
- 有更新时给出可操作的更新提示。  
- 下载过程显示进度。

---

### 2.15 优化 NSIS 安装过程（语言选项 + UI）

#### 当前状态
- `tauri.conf.json` 中 `bundle.targets` 包含 `nsis`，使用 Tauri v2 默认模板。  
- 无自定义 NSIS 脚本或语言文件。

#### 修改方案
1. 创建 `src-tauri/bundles/nsis/` 目录，添加自定义 NSIS 模板：  
   - `main.nsi` 或 `installer.nsi`  
   - `languages/` 中/英文语言文件  
2. 在 `tauri.conf.json` 的 `bundle` 中配置 `nsis`：  
   ```json
   "nsis": {
     "template": "./bundles/nsis/installer.nsi",
     "languages": ["SimpChinese", "English"],
     "displayLanguageSelector": true,
     "installerIcon": "icons/icon.ico",
     "sidebarImage": "...",
     "headerImage": "..."
   }
   ```  
3. 自定义安装界面：  
   - 品牌色标题栏/侧边图。  
   - 简洁的许可/安装路径/完成页面。  
   - 中文/英文语言选择器。

#### 涉及文件
- 新增 `src-tauri/bundles/nsis/installer.nsi`  
- 新增 `src-tauri/bundles/nsis/languages/SimpChinese.nsh`  
- 新增 `src-tauri/bundles/nsis/languages/English.nsh`  
- `src-tauri/tauri.conf.json`

#### 验收标准
- Windows 安装程序启动时可选中文/英文。  
- 安装界面体现 SoloSoul 品牌风格。  
- 安装流程无功能退化。

---

### 2.16 扩展图标系统

#### 当前状态
- `PAGE_ICON_MAP` 有 13 个系统图标；`CUSTOM_ICON_MAP` 有 16 个用户可选图标。  
- 字段类型图标 `FieldTypeIcon.tsx` 已覆盖 12 种类型。

#### 修改方案
1. **系统图标扩展**：为插件、AI、搜索、回收站、帮助、模板、导入导出等新增/补充图标。  
2. **用户可选图标扩展**：在 `CUSTOM_ICON_MAP` 中增加 20-30 个常用 Lucide 图标，分类：  
   - 安全：Shield、Key、Lock、Unlock  
   - 财务：CreditCard、Wallet、Coins、Receipt  
   - 旅行：Plane、MapPin、Compass、Hotel  
   - 工作：Building、User、Users、Briefcase  
   - 生活：Smartphone、Wifi、Sun、Moon、Cloud  
3. 图标选择器网格从 5 列自适应到 6 列，以容纳更多图标。

#### 涉及文件
- `src/lib/pageIcons.ts`  
- `src/components/layout/SideNavigation.tsx`（图标选择器网格）  
- `src/components/ui/FieldTypeIcon.tsx`（可选扩展）

#### 验收标准
- 新增图标可在侧边栏、自定义页面、模板等位置使用。  
- 图标选择器显示正常，不溢出。

---

### 2.17 侧边栏新建页面卡片直接支持选择图标

#### 当前状态
- `AddPageButton` 组件（`SideNavigation.tsx`）新建页面时已有图标选择器。  
- 可能存在的体验问题：图标选择不够明显或默认图标单一。

#### 修改方案
- 确认现有图标选择器可用，优化 UX：  
  - 将图标选择按钮更醒目（显示当前选中图标 + 下拉网格）。  
  - 默认图标保持 `document`。  
- 如需求是“增加更多默认图标选择”，则结合 2.16 扩展 `CUSTOM_ICON_MAP`。

#### 涉及文件
- `src/components/layout/SideNavigation.tsx`（AddPageButton 区域）  
- `src/lib/pageIcons.ts`

#### 验收标准
- 点击新建页面按钮后，用户可直接在弹窗内选择图标。  
- 创建后侧边栏显示所选图标。

---

### 2.18 系统推荐对象模板的敏感度等级调整

#### 当前状态
- `system_templates_zh.json` / `system_templates_en.json` 中：  
  - 身份证号：`sensitive`  
  - 护照号：`sensitive`  
  - 签证号码：`sensitive`  
  - 银行账号：`sensitive`  
  - 银行卡号：`sensitive`

#### 修改方案
- 将高敏感身份/金融标识符从 `sensitive` 调整为 `critical`：  
  - 身份证号 → `critical`  
  - 护照号 → `critical`  
  - 签证号码 → `critical`  
  - 银行账号 → `critical`  
  - 银行卡号 → `critical`  
- 邮箱、电话保持 `internal` 或 `sensitive`（根据主流认知，建议邮箱 `internal`，电话 `sensitive`）。  
- 中文/英文模板 JSON 同步修改。

#### 涉及文件
- `src-tauri/resources/system_templates_zh.json`  
- `src-tauri/resources/system_templates_en.json`

#### 验收标准
- 新建对象时，身份证/护照/签证/银行相关字段的敏感度徽章显示为“关键”。  
- 现有用户已导入的模板不会自动变更（属于预期，种子只在创建账户时导入一次）。

---

### 2.19 macOS BootstrapPage 密码提示词登录后修改密码提示仍显示“无可用提示词”

#### 当前状态
- `BootstrapPage.tsx` 创建账户时传入 `passwordHint`。  
- `LoginPage.tsx` 从 `selectedAccount.passwordHint` 读取提示。  
- `SecuritySettingsPage.tsx` 修改提示后调用 `vault_update_hint` 并刷新 `listAccounts`。  
- 可能原因：  
  1. `listAccounts` 返回的 `passwordHint` 来自缓存的 `accounts.json` 而非重新读取 `config.json`。  
  2. `SecurePasswordInput` 的 `key` 未随 hint 更新而重新挂载。  
  3. Windows 与 macOS 共用同一逻辑，若 macOS 复现则 Windows 也可能复现。

#### 修改方案
1. **后端**：`list_accounts` 命令每次读取 `config.json` 的最新 `password_hint`，不依赖内存缓存。  
2. **前端**：确保 `SecurePasswordInput` 的 `key` 或 `hint` prop 在 hint 变更后触发重新渲染。  
3. 在 `SecuritySettingsPage.tsx` 修改提示成功后，强制重新 `listAccounts()` 并更新 `currentAccount`。  
4. 跨平台验证：同时检查 Windows 是否相同问题。

#### 涉及文件
- `src-tauri/src/commands/auth.rs`（`list_accounts` 或相关）  
- `src-tauri/src/services/vault_service.rs`（`update_password_hint`）  
- `src/stores/authStore.ts`  
- `src/pages/settings/SecuritySettingsPage.tsx`  
- `src/components/forms/PasswordInput.tsx`

#### 验收标准
- BootstrapPage 创建时设置提示词，登录后修改密码页面当前密码提示按钮显示最新提示词。  
- Windows 与 macOS 行为一致。

---

### 2.20 首次打开软件显示帮助指南引导（新手教程卡片）

#### 当前状态
- 无首次启动引导。

#### 修改方案
1. 在 `ui_preferences.json` 中增加 `hasSeenOnboarding: boolean`。  
2. `App.tsx` 或独立 `OnboardingGuard` 在应用启动后检测：  
   - 若未看过引导，显示 `OnboardingDialog`。  
3. 引导卡片类型：  
   - 欢迎页  
   - 创建第一个对象  
   - 使用模板  
   - 设置安全选项  
   - 完成  
4. 提供“跳过”和“不再显示”。

#### 涉及文件
- 新增 `src/components/onboarding/OnboardingDialog.tsx`  
- `src-tauri/src/commands/settings.rs`（`ui_get_preferences` / `ui_update_preference`）  
- `src/App.tsx`  
- `src/locales/zh-CN/common.json`、`src/locales/en-US/common.json`

#### 验收标准
- 首次安装启动后显示新手教程卡片。  
- 完成后写入 `hasSeenOnboarding`，下次不再显示。  
- 可随时在帮助/设置中重新打开。

---

### 2.21 Workspace 新建对象类型下新增“新建对象类型”按钮跳转模板管理

#### 当前状态
- `ObjectEditorPage.tsx` 新建对象时显示当前 section 可用模板按钮组。  
- 无可用模板时仅显示文本链接“前往模板管理新建”。

#### 修改方案
1. 在对象类型选择区域始终显示一个“管理模板”按钮（图标 + 文字）。  
2. 点击后 `navigate('/settings/templates')`。  
3. 保持现有无模板时的文本提示。

#### 涉及文件
- `src/pages/editor/ObjectEditorPage.tsx`  
- `src/locales/zh-CN/editor.json`、`src/locales/en-US/editor.json`

#### i18n 新增
```json
{
  "editor": {
    "manage_templates": "管理模板"
  }
}
```

#### 验收标准
- 新建对象页面的对象类型区域始终可见“管理模板”按钮。  
- 点击跳转到模板管理页面。

---

## 3. 实施阶段与优先级

| 阶段 | 内容 | 预计工时 |
|------|------|----------|
| 阶段 1（高优先级） | 2.2, 2.4, 2.5, 2.8, 2.9, 2.11, 2.12, 2.19 | 4-6 小时 |
| 阶段 2（中优先级） | 2.6, 2.7, 2.10 | 2-3 小时 |
| 阶段 3（中优先级） | 2.1, 2.16, 2.17, 2.21 | 3-4 小时 |
| 阶段 4（中优先级） | 2.13, 2.18 | 2-3 小时 |
| 阶段 5（低优先级，大工作量） | 2.3, 2.14, 2.15, 2.20 | 6-10 小时 |

> 总计工时约 **17-26 小时**，建议按阶段逐步交付。

---

## 4. 风险与待确认事项

| 风险 | 说明 | 建议 |
|------|------|------|
| 窗口大小加密 vs 登录前加载 | 加密账户数据无法登录前读取 | 已选择方案 A（迁移到 `ui_preferences.json`） |
| 模板示例数量 | 当前系统模板已有 8 个（含银行卡），用户原列 7 个 | 按 8 个模板实施：身份信息、身份证、护照、签证、银行账户、银行卡、教育信息、工作信息 |
| 应用内更新机制 | 当前自定义实现 vs 官方 updater | 短期增强自定义实现；长期迁移官方插件 |
| NSIS 自定义模板 | Tauri v2 的 NSIS 自定义能力需验证 | 先查阅 Tauri v2 文档确认 `bundle.nsis.template` 支持 |
| Windows DWM 标题栏 | 需要引入 Windows crate 或 raw FFI | 开发者在 Windows 环境测试 |
| 图标扩展数量 | 过多图标会降低选择器可用性 | 分组或搜索过滤（可选） |

---

## 5. 验收清单（总览）

- [ ] 侧边栏可变 3 按钮 + 锁定/设置固定。  
- [ ] LLM 重置按钮深色模式清晰可见。  
- [ ] 窗口大小登录前恢复（按推荐方案 A）。  
- [ ] 登录页指定按钮有浮动/高亮动画。  
- [ ] 左侧/右侧侧边栏滚动条与 top/bottom 一致。  
- [ ] 密码登录写入操作日志。  
- [ ] 生物识别日志显示 Touch ID / Face ID。  
- [ ] 编辑对象保存验证失败提示准确。  
- [ ] Windows 标题栏颜色随主题变化。  
- [ ] 创建页面日志显示“页面”并使用独立颜色。  
- [ ] BootstrapPage 与 SecuritySettingsPage 显示主密码警告。  
- [ ] 自动锁定可配置且有说明。  
- [ ] 模板管理页可查看 8 个示例模板详情。  
- [ ] 应用启动检测新版本并提示更新。  
- [ ] Windows 安装程序支持中英文与自定义 UI。  
- [ ] 图标系统扩展，新建页面支持图标选择。  
- [ ] 系统推荐模板关键字段敏感度为 critical。  
- [ ] BootstrapPage 提示词修改后登录页正确显示。  
- [ ] 首次启动显示新手教程卡片。  
- [ ] Workspace 新建对象页可跳转模板管理。

---

**文档结束**