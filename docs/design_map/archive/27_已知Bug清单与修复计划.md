# 27 — 已知 Bug 清单与修复计划
> **⚠️ 已归档（2026-08-07）**：本文档 9 项 Bug 均为 2026-06 开发期问题，**现已全部修复闭环**。
> 保留仅作历史记录，不再作为修复任务来源。

> **前置阅读**：`11_跨平台材质系统与视觉规范.md`、`12_状态管理_Zustand_Store设计.md`、`10_前端技术架构与组件映射.md`
> **Manifesto 对齐**：最少惊喜 | 安全默认
> **文档定位**：当前 Tauri 重构开发阶段中已发现、待修复的已知 Bug 汇总。每项 Bug 包含详细描述、复现步骤、根因分析、修复方向与验收标准。

---

## Bug 清单总览

| # | 模块 | Bug 摘要 | 严重度 | 状态 |
|---|------|---------|--------|------|
| 1 | UI 组件 | 密码提示按钮浮层文本竖向排列 | 中 | [ ] 待修复 |
| 2 | 设置页面 | 主题/Accent/语言设置点击后无反应 | **高** | [ ] 待修复 |
| 3 | 侧边栏导航 | 上区按钮名称/图标/缺失问题 | 中 | [ ] 待修复 |
| 4 | 页面路由 | 进入默认页面提示 "Vault not unlocked" | **高** | [ ] 待修复 |
| 5 | 侧边栏导航 | 锁定按钮点击无反应 | **高** | [ ] 待修复 |
| 6 | 侧边栏导航 | 下区缺少插件/AI 对话图标 | 中 | [ ] 待修复 |
| 7 | 侧边栏导航 | 自定义页面创建输入框被侧边栏截断 | 中 | [ ] 待修复 |
| 8 | 侧边栏导航 | 加号按钮下移与下方按钮重叠 | 中 | [ ] 待修复 |
| 9 | 页面路由 | 自定义页面点击后刷新主页而非进入页面 | **高** | [ ] 待修复 |

---

## Bug 1：密码提示按钮浮层文本竖向排列

### 1.1 问题描述

`SecurePasswordInput` 组件的**提示按钮（Password Hint）**点击后弹出的 Tooltip / 名称浮层中，提示词文本出现**竖向排列**（每个字符换一行），而非正常的横向文本流。

### 1.2 复现步骤

1. 进入任意包含密码输入框的页面（如登录页、修改密码页）
2. 确保用户已设置密码提示词（或通过调试手动设置 `passwordHint`）
3. 点击输入框右侧的提示按钮（`HelpCircle` 图标）
4. 观察浮层内文本显示方式：文本竖向堆叠，而非横向排列

### 1.3 预期行为 vs 实际行为

| 预期 | 实际 |
|------|------|
| 浮层内文本横向正常排列，自动换行仅在超出 `max-width: 240px` 时生效 | 文本每个字符独占一行，竖向堆叠 |

```
预期：                        实际：
┌──────────────┐             ┌──────────────┐
│ 我的密码提示词 │             │ 我           │
│              │             │ 的           │
└──────────────┘             │ 密           │
                             │ 码           │
                             │ 提           │
                             │ 示           │
                             │ 词           │
                             └──────────────┘
```

### 1.4 根因分析

- **根因 A（高概率）**：浮层 CSS 中 `white-space: nowrap` 与过窄的 `width` / `max-width` 冲突，导致浏览器将每个字符作为独立行处理。
- **根因 B（中概率）**：浮层容器 `width` 被错误地设为固定窄值（如 `width: 1em` 或 `min-width: 0`），而非自适应内容宽度。
- **根因 C（低概率）**：中文字体渲染时 `line-break: anywhere` 或 `word-break: break-all` 被异常应用。

### 1.5 修复方向

检查并修正 `.name-card` 或 Tooltip 组件的 CSS：

```css
/* 当前可能的错误写法 */
.name-card {
  white-space: nowrap;    /* 错误：禁止换行但宽度不足 */
  width: 40px;            /* 错误：固定宽度过窄 */
}

/* 正确写法 */
.name-card {
  white-space: normal;      /* 允许正常换行 */
  word-break: break-word;   /* 长单词/URL 时优雅换行 */
  max-width: 240px;         /* 最大宽度限制 */
  width: max-content;       /* 自适应内容宽度 */
  min-width: 80px;          /* 最小宽度，避免过窄 */
}
```

**检查点**：
- [ ] 确认 `.name-card` 的 `white-space` 值为 `normal` 而非 `nowrap`
- [ ] 确认 `width` 为 `max-content` 或 `auto`，非固定窄值
- [ ] 确认 `max-width: 240px` 已生效
- [ ] 中文环境下测试：短文本（2-3 字）、长文本（20+ 字）、无可用提示词（6 字）均正常显示

### 1.6 验收标准

- [ ] 短提示词（如"生日"）横向正常显示，不换行
- [ ] 长提示词（如"我常用的密码是我的生日加姓名缩写"）在 240px 宽度内自动换行
- [ ] "无可用提示词" 6 个字符横向正常显示
- [ ] 中英文混合提示词显示正常

---

## Bug 2：设置页面主题/Accent/语言设置点击后无反应

### 2.1 问题描述

`设置 → 通用` 页面中，以下设置项点击后**没有任何反应**，设置未生效：
- **主题切换**（亮色/暗色/跟随系统）
- **Accent 颜色**（ocean/amber/forest/rose/custom）
- **语言切换**（中文/英文）

点击后界面无变化，刷新页面后恢复默认状态，设置未被持久化。

### 2.2 复现步骤

1. 解锁 Vault 后进入设置页面（`设置 → 通用`）
2. 点击主题选项（如从"亮色"切换到"暗色"）
3. 观察：页面主题未变化，CSS 变量未更新
4. 点击 Accent 颜色选项（如从 ocean 切换到 amber）
5. 观察：按钮/指示条颜色未变化
6. 点击语言选项（如从中文切换到英文）
7. 观察：界面文本未切换
8. 刷新页面，所有设置恢复默认值

### 2.3 预期行为 vs 实际行为

| 设置项 | 预期 | 实际 |
|--------|------|------|
| 主题 | 点击后立即切换亮/暗模式，CSS 变量即时更新 | 点击后无反应，主题不变 |
| Accent | 点击后主色调立即变化 | 点击后无反应，颜色不变 |
| 语言 | 点击后 `i18next.changeLanguage()` 即时切换全应用文本 | 点击后无反应，语言不变 |

### 2.4 根因分析

**链路分析**（前端点击 → Store 更新 → IPC → Rust Vault → 持久化 → 即时生效）：

```
用户点击设置项
    ↓
[前端] 触发 onChange / onClick
    ↓
[前端] 调用 settingsStore.updateSetting(key, value)
    ↓ ←── 断点 1：Store 是否正确更新？
[前端] settingsStore 内部状态已更新？
    ↓ ←── 断点 2：IPC 命令是否正确调用？
[前端] await commands.userDataUpdatePreference(key, value)
    ↓ ←── 断点 3：IPC 命令是否实现？Rust 是否报错？
[后端] Rust Vault 接收请求并写入 ui.enc
    ↓ ←── 断点 4：写入是否成功？
[前端] IPC 返回成功 → 应用即时生效
    ↓ ←── 断点 5：即时生效逻辑是否执行？
界面更新
```

**已确认根因（启动时序问题）**：

问题本质：**前端启动时序错误**——`loadSettings` 是异步操作，但主题应用逻辑没有等待其完成。

```
启动时间线（修复前）：
  1. App 挂载 → applyTheme(DEFAULT: system, ocean) ← 默认值生效
  2. 用户登录 → loadSettings() 从磁盘读取保存值
  3. settings store 更新为保存值（如 theme: dark, accent: amber）
  4. ❌ applyTheme 没有再被调用 → 界面仍是默认值
  5. 用户看到：主题没变（还是 system）

启动时间线（修复后）：
  1. App 挂载 → applyTheme(DEFAULT)
  2. 用户登录 → loadSettings().then(() => {
  3.   ✅ applyTheme(SAVED: dark, amber) ← 保存值生效
  4. })
  5. 用户看到：正确的保存主题
```

> **注意**：数据一直持久化在磁盘（`~/.solosoul/`），dev 和 production 都一样。问题纯粹是前端启动时序——`loadSettings` 是异步的，之前没有等它完成就跳过了主题重新应用。

**深层根因（架构层面）**：

主题、语言等 UI 偏好存储在 Vault 内（`preferences.enc`），导致**登录/解锁页面无法读取用户设置**——Vault 未解锁时这些偏好不可访问。这是 Bug 2 反复出现的根本原因：即使修复了启动时序，登录页仍然会显示默认主题而非用户设置的主题。

```
深层问题链：
  1. 用户设置暗色主题 + 中文语言
  2. 这些偏好加密存储于 Vault
  3. 用户重新打开应用 → 登录页
  4. Vault 未解锁 → 无法读取偏好
  5. 登录页显示默认主题（亮色）+ 默认语言（英文）
  6. 用户登录后 → Vault 解锁 → 加载偏好 → 主题切换
  7. 但登录页的体验已经割裂了
```

**架构修复方案**（见文档 15 §4.1、文档 12 §5）：
- 非敏感 UI 偏好（主题/主题色/语言/窗口大小/侧边栏状态）移出 Vault，存储于明文 `ui_preferences.json`
- 敏感偏好（自动锁定/生物识别/密码提示词）继续存储于 Vault 内的 `preferences.enc`
- 登录页可直接读取 `ui_preferences.json`，无需解锁 Vault

**其他可能根因**：

| 断点 | 根因描述 | 验证方法 |
|------|---------|---------|
| 断点 1 | `settingsStore` 未正确绑定到 React 组件，或 Immer 更新未触发重渲染 | 在 `updateSetting` 中打印 `get().settings` 前后值 |
| 断点 2 | `commands.userDataUpdatePreference` 方法名拼写错误，或 IPC 命令未注册 | 检查 `src/lib/ipc.ts` 中命令定义与 Rust `src/commands/` 中命令注册是否一致 |
| 断点 3 | Rust 后端 `user_data_update_preference` 命令未实现，或 `ui.enc` 文件写入失败 | 检查 Rust 编译日志，确认命令存在且无 panic |
| 断点 4 | Rust Vault 写入加密文件时权限不足或路径错误 | 检查 `~/.solosoul/` 目录权限和 `ui.enc` 文件是否存在 |
| 断点 5 | 主题/语言切换后的即时生效回调未执行（如 `i18next.changeLanguage()` 未调用） | 在 `updateSetting` 中 `key === 'language'` 分支添加断点 |

### 2.5 修复方向

**前端检查清单**：

```tsx
// settingsStore.ts — 检查 updateSetting 实现
updateSetting: async (key, value) => {
  const oldValue = get().settings[key];
  set((state) => { state.settings[key] = value; });

  // 断点 1：确认状态已更新
  console.log('[settingsStore] updating', key, 'from', oldValue, 'to', value);

  try {
    // 断点 2：确认 IPC 命令正确调用
    await commands.userDataUpdatePreference(key, value);

    // 断点 5：确认即时生效逻辑执行
    if (key === 'language') {
      console.log('[settingsStore] changing language to', value);
      await i18next.changeLanguage(value);
    }
    if (key === 'theme') {
      console.log('[settingsStore] applying theme', value);
      applyTheme(value); // 检查此函数是否存在且正确
    }
  } catch (err) {
    console.error('[settingsStore] IPC failed:', err);
    set((state) => { state.settings[key] = oldValue; });
    throw err;
  }
},
```

**后端检查清单**（Rust）：

```rust
// src/commands/user_data.rs — 检查命令实现
#[tauri::command]
pub async fn user_data_update_preference(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), String> {
    // 断点 3：确认命令被调用
    println!("[Rust] update_preference: key={}, value={}", key, value);

    let mut prefs = state.preferences.lock().map_err(|e| e.to_string())?;
    prefs.insert(key.clone(), value);

    // 断点 4：确认加密写入成功
    prefs.save_encrypted().map_err(|e| e.to_string())?;
    Ok(())
}
```

**Theme 应用函数检查**：

```typescript
// src/lib/theme.ts — 检查 applyTheme 是否正确更新 CSS 变量
export function applyTheme(theme: ThemeConfig) {
  const root = document.documentElement;
  root.style.setProperty('--accent-primary', theme.accentColor === 'custom'
    ? theme.customAccentHex
    : ACCENT_MAP[theme.accentColor]
  );
  // 检查：是否遗漏了暗色/亮色模式的 body class 切换？
  document.body.classList.toggle('dark', theme.preset === 'warm-stone-dark');
}
```

### 2.6 验收标准

- [ ] 点击主题选项后，页面亮/暗模式即时切换，无闪烁
- [ ] 点击 Accent 颜色后，按钮/指示条/焦点环颜色即时变化
- [ ] 点击语言选项后，全应用文本即时切换为对应语言，无需刷新
- [ ] 刷新页面后，设置保持上次选择（从 Rust Vault 加密偏好中读取）
- [ ] Vault 锁定后重新解锁，设置仍然保持

---

## Bug 3：侧边栏上区按钮名称/图标/缺失问题

### 3.1 问题描述

侧边栏 **Primary Nav（上区）** 存在以下 3 个子问题：

#### 3.1.1 名称不一致

| 按钮 | 悬停名称卡片显示 | 预期显示 |
|------|-----------------|---------|
| 主页（IconHome） | "identity" | "主页"（zh-CN）/ "Home"（en-US） |
| 个人资料（IconProfile） | "profile" | "个人资料"（zh-CN）/ "Profile"（en-US） |

> **根因**：`navigation.json` 词条中 `home` 对应的值为 "identity"（或代码中硬编码了 "identity"），而非 "主页"。个人资料使用了英文 "profile" 小写，未走国际化词条。

#### 3.1.2 缺少 Professional 按钮

上区未渲染 **职业（Professional）** 按钮。当前仅显示：主页、个人资料、旅行、财务。

> **根因**：`NavPrimary` 组件中遗漏了 `<NavButton icon={<IconProfessional>} ... />` 的渲染。

#### 3.1.3 图标不一致

同一页面在**侧边栏**、**主页列表**、**进入页面后的页面标题/图标**三个位置使用了**不同的图标组件**。

| 页面 | 侧边栏图标 | 主页列表图标 | 页面内图标 |
|------|-----------|-------------|-----------|
| 旅行 | `IconTravel` | `IconPlane`（假设） | `IconMap`（假设） |
| 财务 | `IconFinancial` | `IconWallet`（假设） | `IconDollar`（假设） |

> **根因**：没有统一的图标映射表，各页面自行选择图标，导致视觉不一致。

### 3.2 修复方向

#### 3.2.1 名称不一致修复

检查 `src/locales/zh-CN/navigation.json` 和 `src/locales/en-US/navigation.json`：

```json
// zh-CN/navigation.json
{
  "navigation": {
    "sidebar": {
      "home": "主页",          // 修复：原值可能为 "identity"
      "profile": "个人资料",    // 修复：原值可能为 "profile"（英文硬编码）
      "travel": "旅行",
      "financial": "财务",
      "professional": "职业"
    }
  }
}

// en-US/navigation.json
{
  "navigation": {
    "sidebar": {
      "home": "Home",          // 修复
      "profile": "Profile",    // 修复
      "travel": "Travel",
      "financial": "Financial",
      "professional": "Professional"
    }
  }
}
```

检查代码中是否有硬编码：

```tsx
// 错误：硬编码
<NavButton icon={<IconHome />} label="identity" to="/" />

// 错误：直接 import 图标，绕过唯一映射表
<NavButton icon={<IconHome />} label={t('navigation:sidebar.home')} to="/" />

// 正确：通过唯一映射表引用图标 + 国际化
<NavButton icon={<PAGE_ICON_MAP.home />} label={t('navigation:sidebar.home')} to="/" />
```

#### 3.2.2 缺少 Professional 按钮修复

在 `NavPrimary` 中补充：

```tsx
<NavPrimary>
  {/* 修复：统一使用 PAGE_ICON_MAP 引用图标 */}
  <NavButton icon={<PAGE_ICON_MAP.home />} label={t('navigation:sidebar.home')} to="/" />
  <NavButton icon={<PAGE_ICON_MAP.profile />} label={t('navigation:sidebar.profile')} to="/profile" />
  <NavButton icon={<PAGE_ICON_MAP.travel />} label={t('navigation:sidebar.travel')} to="/workspace/travel" />
  <NavButton icon={<PAGE_ICON_MAP.financial />} label={t('navigation:sidebar.financial')} to="/workspace/financial" />
  {/* 修复：补充 Professional 按钮 */}
  <NavButton icon={<PAGE_ICON_MAP.professional />} label={t('navigation:sidebar.professional')} to="/workspace/professional" />
  {customPages.map(...)}
  <AddPageButton onCreate={handleCreateCustomPage} />
</NavPrimary>
```

#### 3.2.3 图标不一致修复

建立统一的页面图标映射表：

```typescript
// src/lib/pageIcons.ts
export const PAGE_ICON_MAP: Record<string, React.ComponentType> = {
  home: IconHome,
  profile: IconProfile,
  travel: IconTravel,
  financial: IconFinancial,
  professional: IconProfessional,
  custom: IconDocument,
};

// 所有位置统一使用此映射
import { PAGE_ICON_MAP } from '@/lib/pageIcons';

// 侧边栏
<NavButton icon={<PAGE_ICON_MAP.home />} ... />

// 主页列表
<PageCard icon={<PAGE_ICON_MAP.travel />} ... />

// 页面内标题
<PageHeader icon={<PAGE_ICON_MAP.financial />} ... />
```

### 3.3 验收标准

- [ ] 主页悬停名称卡片显示 "主页" / "Home"
- [ ] 个人资料悬停名称卡片显示 "个人资料" / "Profile"
- [ ] 侧边栏上区包含 5 个默认按钮：主页、个人资料、旅行、财务、职业
- [ ] 侧边栏、主页列表、页面内三个位置的同一页面使用相同图标
- [ ] 所有按钮名称通过 `t('navigation:sidebar.xxx')` 读取，无硬编码

---

## Bug 4：进入默认页面提示 "Vault not unlocked"

### 4.1 问题描述

点击侧边栏任意默认页面按钮（主页、个人资料、旅行等）后，页面内容区域显示 **"Vault not unlocked"**，而非正常页面内容。此时 Vault 实际上已解锁（用户已成功登录并看到主界面）。

### 4.2 复现步骤

1. 启动应用，输入密码解锁 Vault
2. 成功进入主页，正常显示内容
3. 点击侧边栏"旅行"按钮
4. 页面跳转后显示 "Vault not unlocked"，而非旅行页面内容

### 4.3 根因分析

**可能根因 A（高概率）**：页面级路由守卫在页面切换时错误地检查了 Vault 状态。

```tsx
// 可能的错误逻辑
function TravelPage() {
  const { isUnlocked } = useVaultStore();
  // 错误：页面切换时 isUnlocked 可能为 false（状态未同步）
  if (!isUnlocked) return <div>Vault not unlocked</div>;
  return <TravelContent />;
}
```

**可能根因 B（中概率）**：`vaultStore` 在路由切换时被意外重置，或 `clearOnVaultLock` 被错误触发。

**可能根因 C（中概率）**：路由守卫（`ProtectedRoute`）在嵌套路由切换时重新检查认证状态，但检查逻辑存在竞态条件。

**可能根因 D（低概率）**：IPC 命令（如 `vault_get_status`）在页面切换时被调用，但返回了过期的锁定状态。

### 4.4 修复方向

**检查 1：vaultStore 状态持久性**

```tsx
// vaultStore.ts — 确认 Vault 状态在路由切换时不被重置
export const useVaultStore = create<VaultState>()(
  immer((set, get) => ({
    isUnlocked: false,  // 初始状态
    
    unlock: async (password) => {
      const result = await commands.vaultUnlock(password);
      if (result.success) {
        set({ isUnlocked: true });  // 解锁后设为 true
      }
    },
    
    // 检查：是否有其他地方 set({ isUnlocked: false })？
    // 例如：Vault 锁定后应由后端事件触发，而非页面切换触发
  }))
);
```

**检查 2：路由守卫逻辑**

```tsx
// ProtectedRoute.tsx — 确认检查方式
function ProtectedRoute() {
  const { isUnlocked } = useVaultStore();
  
  // 错误：直接读取可能为 false 的初始状态
  if (!isUnlocked) return <Navigate to="/login" />;
  
  // 正确：等待 Vault 状态确认后再渲染
  const [isReady, setIsReady] = useState(false);
  useEffect(() => {
    commands.vaultGetStatus().then((status) => {
      setIsReady(true);
      if (!status.unlocked) {
        // 重定向到登录
      }
    });
  }, []);
  
  if (!isReady) return <Loading />;
  return <Outlet />;
}
```

**检查 3：页面组件内部检查**

删除页面组件内部的冗余 Vault 状态检查，统一由路由守卫处理：

```tsx
// 错误：TravelPage 内部重复检查
function TravelPage() {
  const { isUnlocked } = useVaultStore();
  if (!isUnlocked) return <div>Vault not unlocked</div>; // 删除此行
  return <TravelContent />;
}

// 正确：仅由 ProtectedRoute 负责权限检查
function TravelPage() {
  return <TravelContent />;
}
```

### 4.5 验收标准

- [ ] 解锁 Vault 后，点击任意默认页面按钮正常进入页面内容
- [ ] 页面切换过程中不出现 "Vault not unlocked" 提示
- [ ] Vault 真正锁定后（如超时自动锁定），点击页面按钮正确重定向到登录页
- [ ] 刷新页面后，若 Vault 仍解锁，直接显示目标页面内容

---

## Bug 5：锁定按钮点击无反应

### 5.1 问题描述

侧边栏下区**锁定按钮（IconLock）**点击后无任何反应：
- Vault 未被锁定
- 页面未重定向到登录页
- 无 Toast 提示
- 无网络请求或 IPC 调用

### 5.2 复现步骤

1. 解锁 Vault 后进入主界面
2. 点击侧边栏下区锁定按钮（锁形图标）
3. 观察：无任何反应，Vault 仍处于解锁状态

### 5.3 根因分析

**根因 A（高概率）**：`<NavButton>` 的 `action` prop 未正确绑定事件处理函数。

```tsx
// 可能的错误：action 仅为字符串标识，未映射到实际函数
<NavButton icon={<IconLock />} label="锁定" action="lockVault" />
// action="lockVault" 只是字符串，未触发任何 IPC 调用
```

**根因 B（中概率）**：`lockVault` IPC 命令未在 Rust 后端实现。

**根因 C（中概率）**：事件处理函数存在但未正确传递给 `onClick`。

### 5.4 修复方向

**检查 1：NavButton 事件绑定**

```tsx
// NavButton.tsx — 确认 onClick 正确传递
interface NavButtonProps {
  icon: React.ReactNode;
  label: string;
  to?: string;        // 页面路由
  action?: () => void; // 动作回调
}

function NavButton({ icon, label, to, action }: NavButtonProps) {
  const navigate = useNavigate();
  
  const handleClick = () => {
    if (to) {
      navigate(to);
    }
    if (action) {
      action();  // 检查：action 是否被正确调用？
    }
  };
  
  return (
    <button onClick={handleClick} aria-label={label}>
      {icon}
    </button>
  );
}
```

**检查 2：锁定动作实现**

```tsx
// SideNavigation.tsx — 确认锁定动作正确绑定
function SideNavigation() {
  const { lock } = useVaultStore();
  const navigate = useNavigate();
  
  const handleLock = async () => {
    console.log('[SideNavigation] locking vault...');  // 断点
    await lock();  // 调用 vaultStore.lock()
    navigate('/login');  // 锁定后重定向到登录页
  };
  
  return (
    <NavSecondary>
      <NavButton icon={<IconLock />} label={t('navigation:sidebar.lock')} action={handleLock} />
      {/* ... */}
    </NavSecondary>
  );
}
```

**检查 3：vaultStore.lock() 实现**

```tsx
// vaultStore.ts — 确认 lock 方法存在且调用 IPC
lock: async () => {
  console.log('[vaultStore] calling vaultLock IPC...');  // 断点
  await commands.vaultLock();  // 检查：IPC 命令是否存在？
  set({ isUnlocked: false, account: null });
  // 触发全局 Vault 锁定事件，通知各 Store 清空敏感数据
  useSettingsStore.getState().clearOnVaultLock();
  useProfileStore.getState().clearOnVaultLock();
}
```

**检查 4：Rust 后端 `vault_lock` 命令**

```rust
#[tauri::command]
pub async fn vault_lock(state: State<'_, AppState>) -> Result<(), String> {
    println!("[Rust] vault_lock called");  // 断点
    
    let mut vault = state.vault.lock().map_err(|e| e.to_string())?;
    vault.lock();  // 擦除内存中的派生密钥
    
    Ok(())
}
```

### 5.5 验收标准

- [ ] 点击锁定按钮后，Vault 正确锁定（内存中密钥被擦除）
- [ ] 点击锁定按钮后，页面重定向到登录页（`/login`）
- [ ] 锁定后，settingsStore 和 profileStore 中的敏感数据被清空
- [ ] 锁定后，侧边栏上区除"锁定"外其他按钮无法点击（或点击后提示需解锁）
- [ ] 锁定后刷新页面，显示登录页而非主界面

---

## Bug 6：侧边栏下区缺少插件/AI 对话图标

### 6.1 问题描述

侧边栏 **Secondary Nav（下区）** 未渲染以下按钮：
- **插件（IconPlugin）**
- **AI 对话（IconChat）**

当前下区仅显示：锁定、搜索、设置。

### 6.2 复现步骤

1. 解锁 Vault 后查看侧边栏下区
2. 观察：仅显示锁定、搜索、设置三个按钮
3. 预期应显示：锁定、搜索、**插件**、**AI 对话**、设置

### 6.3 根因分析

**根因（确定）**：`NavSecondary` 组件中遗漏了 `<NavButton icon={<IconPlugin>} ... />` 和 `<NavButton icon={<IconChat>} ... />` 的渲染代码。

### 6.4 修复方向

在 `NavSecondary` 中补充缺失的按钮：

```tsx
<NavSecondary>
  <NavButton icon={<IconLock />} label={t('navigation:sidebar.lock')} action={handleLock} />
  <NavButton icon={<IconSearch />} label={t('navigation:sidebar.search')} to="/search" />
  {/* 修复：补充插件按钮 */}
  <NavButton icon={<IconPlugin />} label={t('navigation:sidebar.plugins')} to="/plugins" />
  {/* 修复：补充 AI 对话按钮 */}
  <NavButton icon={<IconChat />} label={t('navigation:sidebar.ai_chat')} to="/llm-chat" />
  <NavButton icon={<IconSettings />} label={t('navigation:sidebar.settings')} to="/settings" />
</NavSecondary>
```

同时确认 `navigation.json` 中已包含对应词条：

```json
{
  "navigation": {
    "sidebar": {
      "lock": "锁定",
      "search": "搜索",
      "plugins": "插件",
      "ai_chat": "AI 对话",
      "settings": "设置"
    }
  }
}
```

### 6.5 验收标准

- [ ] 侧边栏下区包含 5 个按钮：锁定、搜索、插件、AI 对话、设置
- [ ] 按钮顺序从上到下：锁定 → 搜索 → 插件 → AI 对话 → 设置
- [ ] 悬停时名称卡片正确显示对应名称
- [ ] 点击插件按钮跳转至 `/plugins`
- [ ] 点击 AI 对话按钮跳转至 `/llm-chat`

---

## Bug 7：自定义页面创建输入框被侧边栏截断

### 7.1 问题描述

点击侧边栏"+"按钮创建自定义页面时，输入框被**限制在 64px 宽的侧边栏内部**，导致输入框宽度严重不足，用户几乎无法看到输入内容。

```
当前（错误）：                预期（正确）：
┌─ SideNavigation ─┐         ┌─ SideNavigation ─┐     ┌──────────┐
│ [职业]            │         │ [职业]            │     │ [输入框..]│ ← 超出侧边栏
│ [Icon] [输]       │         │ [Icon] [输入框..] │     └──────────┘
│ [+]               │         │ [+]               │
└──────────────────┘         └──────────────────┘
     ↑ 被截断                      ↑ 正常显示
```

### 7.2 根因分析

- **根因**：输入框作为侧边栏（`width: 64px`）的子元素渲染，受父容器宽度限制。侧边栏可能设置了 `overflow: hidden`，导致子元素无法超出边界。
- **设计约束**：侧边栏本身保持 64px 固定宽度，但创建流程是**临时状态**（持续数秒），允许输入框临时超出侧边栏右边界。

### 7.3 修复方向

**方案 A（推荐）：使用 `position: fixed` 或 Portal 将输入框提升到侧边栏外部**

```tsx
function AddPageButton() {
  const [isCreating, setIsCreating] = useState(false);
  const buttonRef = useRef<HTMLButtonElement>(null);

  return (
    <div style={{ position: 'relative' }}>
      <button ref={buttonRef} onClick={() => setIsCreating(true)}>+</button>
      {isCreating && (
        // 使用 Portal 将输入框渲染到 body，脱离侧边栏宽度限制
        <CreatePagePopover
          anchorEl={buttonRef.current}
          onConfirm={handleConfirm}
          onCancel={handleCancel}
        />
      )}
    </div>
  );
}

// CreatePagePopover 使用 anchorEl 定位，position: fixed，z-index 高于侧边栏
// 输入框宽度：160px，左侧对齐到"+"按钮右侧（侧边栏右边界）
```

**方案 B：调整侧边栏 z-index 和 overflow**

```css
.side-navigation {
  overflow: visible; /* 临时允许子元素超出 */
  z-index: 100;
}

.create-page-input {
  position: absolute;
  left: calc(100% + 8px); /* 侧边栏右边界外 8px */
  top: 50%;
  transform: translateY(-50%);
  width: 160px;
  z-index: 101;
}
```

> 注意：方案 B 可能导致侧边栏其他内容也溢出，建议优先使用方案 A（Portal）。

### 7.4 验收标准

- [ ] 输入框宽度至少 160px，可完整显示"页面名称"占位符文本
- [ ] 输入框位于侧边栏右边界外侧，不被截断
- [ ] 输入框与"+"按钮垂直居中对齐
- [ ] 创建完成或取消后，输入框消失，侧边栏恢复正常状态

---

## Bug 8：加号按钮下移与下方按钮重叠

### 8.1 问题描述

当侧边栏上区已存在一个或多个自定义页面时，点击"+"按钮后，**"+"按钮下移的距离过大**，导致与下方的 Secondary Nav 按钮（如锁定、搜索）**重叠**。

```
正常状态：                    创建状态（错误）：
┌──────────────┐             ┌──────────────┐
│ [主页]        │             │ [主页]        │
│ [个人资料]    │             │ [个人资料]    │
│ [自定义1]     │             │ [自定义1]     │
│ [+]           │      →      │ [Icon] [输入] │
│              │             │ [+]           │ ← 下移过多，与锁定重叠
│ ──分隔──     │             │ [锁定]        │ ← 被"+"覆盖
│ [锁定]        │             │ [搜索]        │
│ [搜索]        │             └──────────────┘
└──────────────┘
```

### 8.2 根因分析

- **根因**：当前动画仅将"+"按钮向下平移固定距离（如 52px），未考虑上区实际剩余空间。当上区已有多个自定义页面时，"+"按钮下方空间不足，平移后直接覆盖 Secondary Nav 按钮。
- **设计约束**：侧边栏总高度固定，上区 + 下区 + 弹性分隔共享空间。创建流程需要临时"挤出"空间，而非简单平移按钮。

### 8.3 修复方向

**核心原则**：创建自定义页面时，上区整体**临时压缩**，为输入行腾出空间。"+"按钮下移距离受剩余空间限制，最大不超过可用空间。

**方案 A（推荐）：仅上移自定义页面，默认页面固定**

```
创建前：                      创建时（方案 A）：
┌──────────────┐             ┌──────────────┐
│ [主页]        │             │ [主页]        │ ← 默认页面固定不动
│ [个人资料]    │             │ [个人资料]    │
│ [自定义1]     │             │ [自定义1]     │ ← 自定义页面上移（被截断）
│ [自定义2]     │             │ [自定义2]     │ ← 自定义页面上移（被截断）
│ [+]           │      →      │ [Icon] [输入] │ ← 新行插入
│              │             │ [+]           │ ← 下移有限距离
│ ──分隔──     │             │ ──分隔──      │
│ [锁定]        │             │ [锁定]        │ ← 不被覆盖
└──────────────┘             └──────────────┘
```

- 默认页面（主页、个人资料等）位置固定
- 已有自定义页面整体上移，超出侧边栏顶部的可被截断
- "+"按钮下移距离 = 剩余可用空间（不超过 52px）
- 弹性分隔区自动压缩

**方案 B：默认页面和自定义页面一起上移**

```
创建前：                      创建时（方案 B）：
┌──────────────┐             ┌──────────────┐
│ [主页]        │             │ [主页]        │ ← 整体上移（被截断）
│ [个人资料]    │             │ [个人资料]    │ ← 整体上移（被截断）
│ [自定义1]     │             │ [自定义1]     │ ← 整体上移（被截断）
│ [+]           │      →      │ [Icon] [输入] │ ← 新行插入
│              │             │ [+]           │
│ ──分隔──     │             │ ──分隔──      │
│ [锁定]        │             │ [锁定]        │
└──────────────┘             └──────────────┘
```

- 上区所有按钮（默认 + 自定义）作为一个整体向上平移
- 超出侧边栏顶部的按钮可被截断
- 视觉上更统一，但默认页面被截断可能让用户困惑

**实现要求**：

```tsx
// 两种方案均需实现，通过 feature flag 切换
const SIDEBAR_CREATE_STRATEGY: 'A' | 'B' = 'A'; // 默认方案 A

// 方案 A 核心逻辑
function getCreateRowTransform(
  defaultPageCount: number,
  customPageCount: number,
  navHeight: number
) {
  const rowHeight = 52; // 48px 按钮 + 4px 间距
  const totalUpperHeight = (defaultPageCount + customPageCount + 1) * rowHeight;
  const availableSpace = navHeight - totalUpperHeight - SECONDARY_MIN_HEIGHT;
  
  // "+"按钮下移距离不超过可用空间
  const shiftDown = Math.min(rowHeight, Math.max(0, availableSpace));
  
  // 自定义页面上移距离 = 需要腾出的空间
  const customShiftUp = rowHeight - shiftDown;
  
  return { shiftDown, customShiftUp };
}
```

> **决策**：先实现方案 A 和方案 B，提交后通过实际效果对比选择最终方案。评估标准：
> - 默认页面是否保持可见（方案 A 优）
> - 动画是否流畅自然
> - 创建完成后恢复是否顺滑

### 8.4 验收标准

- [ ] "+"按钮下移后不与 Secondary Nav 任何按钮重叠
- [ ] 上区按钮（默认或自定义）上移时，超出侧边栏顶部可被截断
- [ ] 创建完成或取消后，上区按钮动画恢复到正常位置
- [ ] 无论已有 0 个还是 N 个自定义页面，创建流程均正常工作
- [ ] 方案 A 和方案 B 均已实现，可通过配置切换

---

## Bug 9：自定义页面点击后刷新主页而非进入页面

### 9.1 问题描述

点击侧边栏上区已创建的自定义页面按钮后，页面**刷新并显示主页内容**，而非进入对应的自定义页面。URL 可能未变化，或变化后又立即回退到 `/`。

### 9.2 复现步骤

1. 点击侧边栏"+"按钮创建自定义页面，命名为"我的项目"
2. 侧边栏出现新的自定义页面按钮（`IconDocument` + "我的项目"）
3. 点击"我的项目"按钮
4. 观察：页面闪烁后显示主页内容，URL 可能为 `/` 或短暂变为 `/workspace/custom/xxx` 后回退

### 9.3 根因分析

**根因 A（高概率）**：自定义页面的 `to` 属性或 `onClick` 处理逻辑错误，触发了页面刷新而非客户端路由导航。

```tsx
// 可能的错误：使用 <a href> 或 <form> 导致页面刷新
<NavButton
  icon={<IconDocument />}
  label="我的项目"
  to={`/workspace/custom/${page.id}`}
  // 错误：NavButton 内部可能使用了 <a href={to}> 而非 React Router 的 <Link>
/>

// 或错误：onClick 中使用了 window.location.href
const handleClick = () => {
  window.location.href = `/workspace/custom/${page.id}`; // 全页刷新！
};
```

**根因 B（中概率）**：自定义页面的路由配置未正确注册，导致 React Router 匹配不到 `/workspace/custom/:id`，回退到默认路由 `/`。

```tsx
// 可能的错误：路由表中缺少自定义页面路由
<Routes>
  <Route path="/" element={<HomePage />} />
  <Route path="/profile" element={<ProfilePage />} />
  <Route path="/workspace/travel" element={<TravelPage />} />
  {/* 错误：缺少 /workspace/custom/:id 路由 */}
  <Route path="*" element={<Navigate to="/" />} /> {/* 回退到主页 */}
</Routes>
```

**根因 C（中概率）**：自定义页面组件内部存在错误（如未定义、导入失败），React Router 的错误边界捕获后重定向到主页。

**根因 D（低概率）**：`vaultStore` 或 `authStore` 在路由切换时触发了重新验证，导致路由守卫将用户重定向到主页。

### 9.4 修复方向

**检查 1：NavButton 路由导航方式**

```tsx
// NavButton.tsx — 确认使用 React Router 的 <Link> 或 useNavigate
import { useNavigate, Link } from 'react-router-dom';

function NavButton({ icon, label, to, action }: NavButtonProps) {
  const navigate = useNavigate();

  if (to) {
    // 正确：使用 onClick + navigate（客户端路由，无刷新）
    return (
      <button
        onClick={() => navigate(to)}
        aria-label={label}
      >
        {icon}
      </button>
    );
    
    // 或正确：使用 <Link>（客户端路由）
    return (
      <Link to={to} aria-label={label}>
        {icon}
      </Link>
    );
  }
  
  // action 按钮...
}
```

**检查 2：路由表配置**

```tsx
// App.tsx — 确认自定义页面路由已注册
<Routes>
  <Route path="/" element={<HomePage />} />
  <Route path="/profile" element={<ProfilePage />} />
  <Route path="/workspace/travel" element={<TravelPage />} />
  <Route path="/workspace/financial" element={<FinancialPage />} />
  <Route path="/workspace/professional" element={<ProfessionalPage />} />
  {/* 修复：补充自定义页面路由 */}
  <Route path="/workspace/custom/:pageId" element={<CustomPage />} />
  <Route path="*" element={<Navigate to="/" />} />
</Routes>
```

**检查 3：CustomPage 组件实现**

```tsx
// pages/workspace/CustomPage.tsx
import { useParams } from 'react-router-dom';

function CustomPage() {
  const { pageId } = useParams<{ pageId: string }>();
  
  // 从 store 中获取自定义页面数据
  const page = useCustomPageStore((state) => state.getPage(pageId));
  
  if (!page) {
    return <div>页面不存在</div>; // 不要重定向到主页
  }
  
  return (
    <div>
      <PageHeader icon={<IconDocument />} title={page.name} />
      {/* 自定义页面内容 */}
    </div>
  );
}
```

**检查 4：路由守卫**

确认 `ProtectedRoute` 不会因为自定义页面的路径模式（`/workspace/custom/:id`）而误判为未授权：

```tsx
// 错误：路由守卫硬编码了允许的路径列表
const ALLOWED_PATHS = ['/', '/profile', '/workspace/travel', '/workspace/financial'];
if (!ALLOWED_PATHS.includes(currentPath)) {
  return <Navigate to="/" />; // 自定义页面被误判！
}

// 正确：路由守卫只检查 Vault 解锁状态，不限制路径
function ProtectedRoute() {
  const { isUnlocked } = useVaultStore();
  if (!isUnlocked) return <Navigate to="/login" />;
  return <Outlet />;
}
```

### 9.5 验收标准

- [ ] 点击自定义页面按钮后，URL 正确变为 `/workspace/custom/{id}`
- [ ] 页面无刷新，使用客户端路由导航
- [ ] 页面内容显示对应的自定义页面（非主页内容）
- [ ] 页面标题/图标与自定义页面名称一致
- [ ] 浏览器后退按钮可正确返回上一页面
- [ ] 刷新页面后仍保持在自定义页面（路由持久化）

---

## 修复优先级建议

| 优先级 | Bug | 理由 |
|--------|-----|------|
| **P0** | Bug 4（Vault not unlocked） | 阻断性 Bug，用户无法进入任何页面 |
| **P0** | Bug 5（锁定按钮无反应） | 安全功能失效，用户无法主动锁定 Vault |
| **P0** | Bug 9（自定义页面路由失效） | 自定义页面功能完全不可用 |
| **P1** | Bug 2（设置无反应） | 核心用户体验功能失效 |
| **P1** | Bug 6（缺少图标） | 功能缺失，用户无法访问插件/AI 页面 |
| **P1** | Bug 8（加号按钮重叠） | 自定义页面创建流程异常，影响核心交互 |
| **P2** | Bug 7（输入框被截断） | 创建流程可用但体验差 |
| **P2** | Bug 3（名称/图标/缺失） | 体验问题，不影响核心功能 |
| **P2** | Bug 1（文本竖向排列） | 视觉问题，不影响功能可用性 |

---

*文档版本：v1.0*
*创建日期：2026-06-05*
*对应开发阶段：Phase 2–3（前端基础 + 安全与数据保护）*