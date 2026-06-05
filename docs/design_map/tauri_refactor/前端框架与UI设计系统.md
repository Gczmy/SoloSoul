# 前端框架与 UI 设计系统

> **文档定位**: SoloSoul Tauri 重构后的前端技术栈、跨平台 UI 设计规范、组件实现方案与迁移路线图。
>
> **阅读对象**: 前端开发者、UI 开发者、架构师。
>
> **适用范围**: Tauri + Rust 重构后的 SoloSoul 桌面端（macOS / Windows / Linux）与移动端（iOS / Android）

---

## 目录

- [1. 设计灵感与核心理念](#1-设计灵感与核心理念)
- [2. 跨平台材质系统架构](#2-跨平台材质系统架构)
- [3. 各平台详细设计](#3-各平台详细设计)
- [4. 全局设计系统](#4-全局设计系统)
- [5. 组件设计规范](#5-组件设计规范)
- [6. 动效与交互](#6-动效与交互)
- [7. 暗色模式](#7-暗色模式)
- [8. 可访问性](#8-可访问性)
- [9. 技术栈确认](#9-技术栈确认)
- [10. Flutter Widget → React Component 映射](#10-flutter-widget--react-component-映射)
- [11. Liquid Glass 组件实现](#11-liquid-glass-组件实现)
- [12. 布局系统迁移](#12-布局系统迁移)
- [13. 路由系统迁移](#13-路由系统迁移)
- [14. 表单系统迁移](#14-表单系统迁移)
- [15. 前端架构与技术实现路线图](#15-前端架构与技术实现路线图)
- [16. 设计验证清单](#16-设计验证清单)
- [17. 参考资源](#17-参考资源)
- [18. 变更日志](#18-变更日志)

---

## 1. 设计灵感与核心理念

### 1.1 灵感来源

| 产品 | 我们学习的特质 | 在 SoloSoul 中的体现 |
|------|--------------|---------------------|
| **Notion** | 极简留白、原子化 Block、清晰的排版层级 | 对象工作区采用无干扰编辑，内容即界面；卡片式信息架构 |
| **Anytype** | 温暖的明亮基调、有机的图形语言、对象图谱的秩序感 | 全局采用暖白底色 + 低饱和度强调色；对象关系可视化 |
| **Warp** | 温暖的配色哲学（琥珀/暖灰）、细腻的微交互、人性化的命令行体验 | 侧边栏/工具栏使用暖灰色调；操作反馈带弹性动效；零知识安全产品的"温度感" |
| **Apple Liquid Glass** | 材质的真实光学特性——折射、厚度、高光、边缘 | 在支持的平台启用液态玻璃材质；不支持的平台以"丝滑质感"回退 |
| **Windows Mica** | 将桌面背景与窗口内容自然融合，降低视觉疲劳 | Windows 平台使用 Mica + 内容层级模糊，替代纯纯色窗口 |

### 1.2 核心理念：温暖的数字堡垒

SoloSoul 管理的是用户最私密的数字资产——护照、银行卡、身份凭证。界面必须传递**安全感**与**温度感**的平衡：

- **安全≠冰冷**：传统密码管理器使用深灰/暗黑基调配荧光绿，像银行金库般冰冷。SoloSoul 使用**暖白 + 低饱和蓝灰**，像一本精心装帧的个人手账。
- **隐私≠隐藏**：敏感信息使用分级遮罩（毛玻璃模糊 + 渐变蒙层），而不是粗暴的黑块。遮罩本身也是设计的一部分。
- **专业≠复杂**：操作路径不超过三步；常用功能（查看护照、复制卡号）一键触达。

### 1.3 三个设计原则

```
1. 内容优先，材质服务于内容
   —— 玻璃/云母效果只能增强信息层级，不能干扰阅读

2. 平台原生，体验统一
   —— 各平台使用各自的材质语言（液态玻璃 / Mica / 丝滑），但交互逻辑、信息架构完全一致

3. 降级优雅，从不破碎
   —— 不支持新材质的平台自动获得精心设计的回退方案，用户无感知
```

---

## 2. 跨平台材质系统架构

### 2.1 材质决策矩阵

| 平台 | 系统版本 | 首选材质 | 回退材质 | 技术实现 |
|------|---------|---------|---------|---------|
| **macOS** | 15+ (Sequoia) | Liquid Glass | Acrylic 模糊 | CSS `backdrop-filter` + WebGL 着色器 / Tauri 原生 API |
| **macOS** | 14 及以下 | Acrylic 亚克力 | 纯色 + 投影 | CSS `backdrop-filter: blur()` |
| **iOS** | 26+ | Liquid Glass | 标准模糊 (UIBlurEffect) | CSS `backdrop-filter` + `-webkit-backdrop-filter` |
| **iOS** | 25 及以下 | 标准模糊 | 纯色层级 | CSS 模糊或预渲染毛玻璃纹理 |
| **Windows** | 11 (22H2+) | Mica Alt | Mica | Tauri `mica` / `micaDark` / `acrylic` 窗口效果 |
| **Windows** | 10 | Acrylic | 纯色 + 投影 | CSS `backdrop-filter` 或系统 Acrylic API |
| **Linux** | 不限 | 自定义模糊 | 纯色 + 细微边框 | CSS `backdrop-filter`（视 compositor 支持） |
| **Android** | 14+ | 原生模糊 (Android 14 backdrops) | 纯色层级 | CSS / Canvas 模糊回退 |
| **Web (浏览器)** | 现代浏览器 | CSS 模糊 | 纯色 + 细微透明度 | CSS `backdrop-filter` + `@supports` 检测 |

### 2.2 材质层定义

我们将界面抽象为三层材质栈：

```
┌─────────────────────────────────────────────┐
│  Layer 3: Content Glass（内容玻璃层）        │  ← 卡片、弹窗、浮层 — 最高模糊 + 折射高光
│  blur(40px) + saturate(180%) + 内发光        │
├─────────────────────────────────────────────┤
│  Layer 2: Toolbar Glass（工具栏玻璃层）      │  ← 侧边栏、顶部栏 — 中等模糊
│  blur(20px) + 细微边框                       │
├─────────────────────────────────────────────┤
│  Layer 1: Base Surface（基底材质层）         │  ← 窗口背景 — 平台决定
│  macOS=液态玻璃 / Windows=Mica / 回退=纯色   │
└─────────────────────────────────────────────┘
```

### 2.3 运行时材质探测与切换

```typescript
// 伪代码：Tauri 主进程通过 OS API 探测能力，传递给前端
interface PlatformMaterialCapabilities {
  platform: 'macos' | 'ios' | 'windows' | 'linux' | 'android' | 'web';
  osVersion: string;           // "15.2", "11.22621", etc.
  supportsLiquidGlass: boolean; // macOS 15+, iOS 26+
  supportsMica: boolean;       // Windows 11 22H2+
  supportsAcrylic: boolean;    // Windows 10+, macOS 任意
  supportsBackdropBlur: boolean; // CSS backdrop-filter 可用
  prefersReducedTransparency: boolean; // 系统"减少透明度"设置
  prefersReducedMotion: boolean;       // 系统"减弱动态效果"设置
}

// 材质选择器
function resolveMaterialLayer(
  layer: 'base' | 'toolbar' | 'content',
  caps: PlatformMaterialCapabilities
): MaterialDefinition {
  if (caps.prefersReducedTransparency) {
    return SOLID_FALLBACK[layer]; // 纯色回退
  }
  
  switch (caps.platform) {
    case 'macos':
      if (caps.supportsLiquidGlass) return LIQUID_GLASS[layer];
      return ACRYLIC[layer];
    case 'windows':
      if (caps.supportsMica && layer === 'base') return MICA_BASE;
      if (caps.supportsAcrylic) return ACRYLIC[layer];
      return SOLID_FALLBACK[layer];
    case 'ios':
      if (caps.supportsLiquidGlass) return LIQUID_GLASS_MOBILE[layer];
      return BLUR_MOBILE[layer];
    default:
      return caps.supportsBackdropBlur ? CSS_BLUR[layer] : SOLID_FALLBACK[layer];
  }
}
```

---

## 3. 各平台详细设计

### 3.1 macOS — Liquid Glass

#### 视觉定义

Apple Liquid Glass 的核心视觉特征：

- **光学厚度**：玻璃有实体感，边缘呈现折射造成的光学变形
- **环境反射**：高光不是固定的，而是响应内容/环境
- **景深模糊**：后方内容被高度模糊并提亮（类似 frosted glass 但更深）
- **圆润形体**：所有玻璃表面使用大圆角（16px-24px），边缘有微妙的厚度光带

#### 实现策略

**方案 A：CSS + WebGL 着色器（推荐用于 Tauri）**

```css
/* 基础液态玻璃效果（高级模式） */
.liquid-glass {
  background: linear-gradient(
    135deg,
    rgba(255, 255, 255, 0.4) 0%,
    rgba(255, 255, 255, 0.1) 100%
  );
  backdrop-filter: blur(40px) saturate(180%) brightness(1.1);
  -webkit-backdrop-filter: blur(40px) saturate(180%) brightness(1.1);
  border: 1px solid rgba(255, 255, 255, 0.3);
  border-radius: 20px;
  box-shadow: 
    inset 0 1px 1px rgba(255, 255, 255, 0.4),
    inset 0 -1px 1px rgba(0, 0, 0, 0.05),
    0 8px 32px rgba(0, 0, 0, 0.08);
}

/* 液态玻璃高光边缘 */
.liquid-glass::before {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: inherit;
  padding: 1.5px; /* 光学厚度 */
  background: linear-gradient(
    180deg,
    rgba(255, 255, 255, 0.6) 0%,
    rgba(255, 255, 255, 0.1) 40%,
    transparent 60%,
    rgba(255, 255, 255, 0.15) 100%
  );
  -webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
  mask-composite: exclude;
  pointer-events: none;
}
```

**方案 B：Tauri 原生窗口材质（实验性）**

Tauri v2 支持通过插件或原生代码设置窗口背景材质：

```rust
// 在 Tauri 主进程中，通过 objc 调用 NSWindow 的 backgroundMaterial
// macOS 15+ 可使用 .sheet / .contentBackground / .windowBackground 等材质
#[cfg(target_os = "macos")]
fn apply_liquid_glass(window: &tauri::WebviewWindow) {
    // 通过 cocoa crate 设置 NSVisualEffectView
    // material: NSVisualEffectMaterial::Sheet (或自定义)
    // state: NSVisualEffectState::FollowsWindowActiveState
}
```

> 注：Tauri 目前对 macOS 原生材质的支持有限，推荐以 **CSS 方案为主**，在 Webview 内实现液态玻璃效果。当 Tauri 官方支持增强后，可逐步迁移到原生 API。

#### macOS 回退（14 及以下）

```css
/* Acrylic 亚克力 — macOS 14 及以下 */
.acrylic {
  background: rgba(250, 250, 250, 0.75);
  backdrop-filter: blur(25px) saturate(150%);
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 12px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.06);
}
```

**回退触发条件**：`osVersion < "15.0"` 或 `prefersReducedTransparency = true`

---

### 3.2 iOS — Liquid Glass Mobile

#### 与 macOS 的差异

- **更轻量**：移动设备 GPU 受限，模糊半径减半（20px → 10px）
- **更紧凑**：圆角更小（12px-16px），减少屏幕占用
- **触控优化**：玻璃面板在触摸时产生微妙的按压形变（scale 0.98 + 阴影加深）
- **底部安全区**：底部导航栏自动适配 Home 指示器安全区

#### 实现

```css
/* iOS 液态玻璃 — 轻量模式 */
.liquid-glass-ios {
  background: rgba(255, 255, 255, 0.55);
  backdrop-filter: blur(20px) saturate(160%);
  -webkit-backdrop-filter: blur(20px) saturate(160%);
  border-radius: 16px;
  border: 0.5px solid rgba(255, 255, 255, 0.25);
  /* 移动端不使用外阴影，使用内发光区分层级 */
  box-shadow: inset 0 0 0 0.5px rgba(255, 255, 255, 0.3);
}

/* 底部导航栏 — 带 Home 指示器适配 */
.ios-bottom-bar {
  padding-bottom: env(safe-area-inset-bottom, 20px);
  background: rgba(255, 255, 255, 0.7);
  backdrop-filter: blur(20px);
  border-top: 0.5px solid rgba(0, 0, 0, 0.08);
}
```

#### iOS 回退（15 及以下）

```css
/* iOS 15 标准模糊 */
.ios-blur-fallback {
  background: rgba(250, 250, 250, 0.85);
  backdrop-filter: blur(10px);
  border-radius: 12px;
}

/* 若 backdrop-filter 不支持（iOS 12 及以下） */
@supports not (backdrop-filter: blur(10px)) {
  .ios-blur-fallback {
    background: rgba(245, 245, 245, 0.98);
    border: 1px solid rgba(0, 0, 0, 0.06);
  }
}
```

---

### 3.3 Windows — Mica & Mica Alt

#### Mica 材质特性

- **桌面融合**：窗口背景直接采样桌面壁纸，与应用色调混合
- **无性能损耗**：由 DWM（桌面窗口管理器）直接合成，不占用应用 GPU
- **层次区分**：Mica（浅色/深色）用于主窗口背景，Mica Alt 用于浮动面板/侧边栏
- **自动切换**：跟随系统亮/暗主题自动调整

#### Tauri 实现

Tauri v2 原生支持 Windows 材质：

```rust
// Tauri 主进程 — 设置窗口材质
tauri::Builder::default()
    .setup(|app| {
        let window = app.get_webview_window("main").unwrap();
        
        #[cfg(target_os = "windows")]
        {
            // 需要 tauri-plugin-window-state 或手动调用 windows-rs API
            // 设置 Mica 背景
            window.set_decorations(true)?;
            // 通过 webview2-com 或 windows crate 设置 DWMWA_USE_HOST_BACKDROP_BRUSH
            // 值: DWM_SYSTEMBACKDROP_TYPE::DWMSBT_MAINWINDOW (Mica)
            //      DWM_SYSTEMBACKDROP_TYPE::DWMSBT_TABBEDWINDOW (Mica Alt)
        }
        
        Ok(())
    })
```

**前端配合**：当窗口使用 Mica 时，前端 `<body>` 背景应设为透明，让材质透上来：

```css
/* Windows Mica 模式 — 前端透明背景 */
.windows-mica body {
  background: transparent !important;
}

/* 内容层级使用半透明卡片 */
.windows-mica .content-card {
  background: rgba(255, 255, 255, 0.6); /* 亮色模式 */
  backdrop-filter: blur(8px); /* 轻度模糊，叠加在 Mica 上 */
  border: 1px solid rgba(0, 0, 0, 0.06);
  border-radius: 8px; /* Windows 11 圆角风格 */
}

/* Windows 暗色模式 Mica */
@media (prefers-color-scheme: dark) {
  .windows-mica .content-card {
    background: rgba(40, 40, 40, 0.5);
    border-color: rgba(255, 255, 255, 0.08);
  }
}
```

#### Windows 10 回退

Windows 10 不支持 Mica，使用 Acrylic（有噪声纹理的模糊）：

```css
.windows-acrylic {
  background: rgba(243, 243, 243, 0.9);
  backdrop-filter: blur(30px);
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 4px; /* Windows 10 更锐利的角 */
}
```

---

### 3.4 Linux — 自适应模糊

Linux 桌面环境碎片化（GNOME / KDE / XFCE 等），没有统一的材质 API。

#### 策略

1. **探测 compositor**：通过 Tauri 主进程检测是否支持透明/模糊窗口
2. **GNOME / KDE**：使用 CSS `backdrop-filter`，效果接近 macOS Acrylic
3. **无 compositor**：纯毛玻璃纹理 SVG 背景回退

```css
/* Linux 自适应 */
.linux-glass {
  background: rgba(250, 250, 250, 0.85);
  backdrop-filter: blur(20px) saturate(140%);
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 12px;
}

/* 无模糊支持时的回退 */
.linux-solid {
  background: #fafafa;
  border: 1px solid #e8e8e8;
  border-radius: 8px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.04);
}
```

---

### 3.5 Android — 平台模糊

Android 14 引入了 `android.graphics.RenderEffect`，支持 backdrop blur。

在 Tauri / WebView 中：

```css
/* Android 14+ */
.android-glass {
  background: rgba(255, 255, 255, 0.7);
  backdrop-filter: blur(16px);
  border-radius: 16px;
  /* 使用 Material You 动态取色的边框 */
  border: 1px solid var(--android-system-outline, rgba(0, 0, 0, 0.08));
}

/* Android 13 及以下回退 */
.android-solid {
  background: #ffffff;
  border: 1px solid #eeeeee;
  border-radius: 12px;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.06);
}
```

---

## 4. 全局设计系统

### 4.1 色彩系统 — "Warm Stone" 暖石调

受 Warp 温暖命令行和 Anytype 有机色调启发，SoloSoul 使用**暖灰基底 + 低饱和强调色**。

#### 明亮模式

| Token | 色值 | 用途 |
|-------|------|------|
| `--bg-base` | `#FAFAF8` | 全局画布背景（比纯白更温暖的象牙白） |
| `--bg-elevated` | `#FFFFFF` | 卡片、弹窗、浮层（纯白，与基底形成微妙对比） |
| `--bg-toolbar` | `rgba(250, 250, 248, 0.8)` | 侧边栏/顶部栏（半透明，透出基底） |
| `--text-primary` | `#1A1A1A` | 主标题、正文 |
| `--text-secondary` | `#6B6B6B` | 辅助说明、时间戳 |
| `--text-tertiary` | `#9E9E9E` | 占位符、禁用态 |
| `--accent-primary` | `#5B7C99` | 主要强调色（低饱和蓝灰，传达信任与冷静） |
| `--accent-hover` | `#4A6A85` | 悬停态 |
| `--accent-warm` | `#C4925C` | 温暖强调色（琥珀调，用于关键操作、高亮） |
| `--border-subtle` | `#EBEAE6` | 分割线、卡片边框 |
| `--border-focus` | `#5B7C99` | 聚焦态边框 |
| `--shadow-sm` | `rgba(0, 0, 0, 0.04)` | 轻微投影 |
| `--shadow-md` | `rgba(0, 0, 0, 0.08)` | 中等投影 |
| `--shadow-lg` | `rgba(0, 0, 0, 0.12)` | 弹窗/浮层投影 |

#### 暗色模式

| Token | 色值 | 用途 |
|-------|------|------|
| `--bg-base` | `#1C1C1E` | 全局画布（深灰，非纯黑，减少 OLED 频闪） |
| `--bg-elevated` | `#2C2C2E` | 卡片、弹窗 |
| `--bg-toolbar` | `rgba(28, 28, 30, 0.85)` | 侧边栏 |
| `--text-primary` | `#F5F5F5` | 主文字 |
| `--text-secondary` | `#A0A0A0` | 辅助文字 |
| `--accent-primary` | `#7BA3C4` | 暗色模式下稍亮的蓝灰 |
| `--accent-warm` | `#D4A76A` | 暗色模式琥珀色 |
| `--border-subtle` | `#3A3A3C` | 分割线 |

#### 敏感度色彩编码

沿用 SoloSoul 4 级敏感度分级，每个级别有固定的视觉标识：

| 级别 | 背景色 | 文字/图标色 | 遮罩强度 | 验证要求 |
|------|--------|------------|---------|---------|
| `public` | 透明 | `--text-secondary` | 无 | 无需验证 |
| `internal` | `#F5F5F0` | `#7A7A6A` | 轻微模糊 (blur 2px)，点击展开 | 无需验证 |
| `sensitive` | `#F5F0E8` | `#C4925C` | 中度模糊 (blur 8px) + 渐变蒙层 | 需生物识别/密码 |
| `critical` | `#E8E0D8` | `#A07040` | 重度模糊 (blur 16px) + 深色蒙层 + 🔒 图标 | 需密码 + 操作确认 |

> **注**：早期设计文档中曾使用 6 级（含 `private` / `restricted`），但为降低用户认知负担、保持与现有代码一致，已简化为 4 级。`private` 并入 `internal`，`restricted` 并入 `sensitive`。
>
> 暗色模式下，遮罩使用反向逻辑：敏感内容以**降低亮度/增加噪点**的方式呈现，而非增加亮度。

### 4.2 排版系统

受 Notion 排版层级启发：

| 层级 | 字号 | 字重 | 行高 | 字距 | 用途 |
|------|------|------|------|------|------|
| Display | 32px | 700 | 1.2 | -0.02em | 应用启动页、空状态标题 |
| H1 | 24px | 600 | 1.3 | -0.01em | 页面标题（如"护照详情"） |
| H2 | 18px | 600 | 1.4 | 0 | 区块标题（如"基本信息"） |
| H3 | 15px | 600 | 1.4 | 0.01em | 卡片标题、列表分组头 |
| Body | 14px | 400 | 1.6 | 0.01em | 正文、描述 |
| Label | 12px | 500 | 1.4 | 0.02em | 表单标签、徽章文字 |
| Caption | 11px | 400 | 1.4 | 0.02em | 时间戳、元信息、辅助说明 |

#### 字体栈

```css
/* 跨平台字体回退 */
--font-sans: -apple-system, BlinkMacSystemFont, "SF Pro Text", "Segoe UI", 
             "Noto Sans SC", "PingFang SC", "Hiragino Sans GB", 
             "Microsoft YaHei", sans-serif;
--font-mono: "SF Mono", "Cascadia Code", "Fira Code", "Noto Sans Mono", monospace;
```

### 4.3 间距系统 — 8px 基线

```
4px   — 极紧凑（图标内边距、小徽章）
8px   — 紧凑（按钮内边距、列表项间隙）
12px  — 默认（卡片内边距、表单字段间距）
16px  — 舒适（区块间距、弹窗内边距）
24px  — 宽松（页面边距、模块分隔）
32px  — 极宽松（大模块间距、空状态）
48px  — 章节分隔（工作区切换）
```

### 4.4 圆角系统

```
4px   — 小标签、徽章、输入框
8px   — 按钮、小卡片、列表项
12px  — 标准卡片、弹窗
16px  — 大卡片、模态框、悬浮面板
24px  — 液态玻璃面板（macOS/iOS 高级材质）
999px — 胶囊按钮、圆形头像
```

---

## 5. 组件设计规范

### 5.1 导航架构

SoloSoul 采用**三栏可折叠布局**（桌面端）：

```
┌─────────┬───────────────┬─────────────────────────────┐
│ 窄边栏   │ 对象列表       │ 内容工作区                   │
│ 48px    │ 260px         │ 弹性宽度                      │
│ (图标)  │ (可折叠)      │                             │
├─────────┼───────────────┼─────────────────────────────┤
│ 🏠      │ 护照           │ ┌─────────────────────────┐ │
│ 📄      │ 身份证         │ │ 护照详情                 │ │
│ 💳      │ 银行卡         │ │ ┌─────┐                 │ │
│ ⚙️      │ ...           │ │ │ 照片 │ 姓名: 张三       │ │
│         │               │ │ └─────┘ 国籍: 中国        │ │
│         │               │ │         ...              │ │
│         │               │ └─────────────────────────┘ │
└─────────┴───────────────┴─────────────────────────────┘
```

- **窄边栏**：常驻，平台图标导航（主页、文档、金融、旅行、设置）
- **对象列表**：可折叠（点击汉堡菜单），显示当前分类下的对象列表
- **内容工作区**：核心编辑/查看区域，无干扰设计

#### 移动端

单栏堆叠，底部 Tab 导航：

```
┌─────────────────┐
│ 内容区域         │
│                 │
│                 │
├─────────────────┤
│ 🏠  📄  💳  ⚙️ │
└─────────────────┘
```

### 5.2 核心组件材质映射

| 组件 | macOS | iOS | Windows | Linux/Android |
|------|-------|-----|---------|--------------|
| **窗口背景** | Liquid Glass | 系统模糊 | Mica | 纯白/浅灰 |
| **侧边栏** | Glass blur(30px) | Blur(15px) | Mica Alt | Blur(20px) / 纯色 |
| **内容卡片** | Glass blur(40px) + 高光 | Blur(10px) | 纯白 90% + 轻度阴影 | 纯白 + 边框 |
| **弹窗/模态框** | Glass blur(50px) + 折射 | 系统 sheet | Acrylic + 阴影 | 纯白 + 阴影 |
| **按钮（主）** | 半透明 + 悬停发光 | 实色 + 按压 | 实色 | 实色 |
| **按钮（次）** | 玻璃质感 | 轻微模糊 | 边框 + 背景 | 边框 |
| **输入框** | 玻璃底 + 聚焦高亮 | 系统输入风格 | 边框 + 聚焦 | 边框 + 聚焦 |
| **敏感遮罩** | 玻璃模糊 + 渐变 | 模糊 + 渐变 | 纯色蒙层 + 图标 | 纯色蒙层 + 图标 |
| **Toast 通知** | 玻璃浮层 | 系统通知风格 | 顶部横幅 | 底部 Snackbar |

### 5.3 敏感数据展示 — 分级遮罩组件

```tsx
interface SensitivityOverlayProps {
  level: 'public' | 'internal' | 'sensitive' | 'critical';
  children: React.ReactNode;
  onUnlock?: () => Promise<boolean>; // 返回 true 则临时揭示
}

// 视觉表现：
// public — 无遮罩，正常显示
// internal — 轻微模糊，点击后展开
// sensitive — 中度模糊 + 半透明蒙层，需生物识别/密码
// critical — 重度模糊 + 深色蒙层 + 锁图标，需密码 + 操作确认
```

#### CSS 实现

```css
.sensitivity-internal {
  filter: blur(2px);
  transition: filter 0.3s ease;
  cursor: pointer;
}
.sensitivity-internal:hover {
  filter: blur(1px);
}
.sensitivity-internal.revealed {
  filter: none;
}

.sensitivity-sensitive {
  position: relative;
  filter: blur(8px);
}
.sensitivity-sensitive::after {
  content: '';
  position: absolute;
  inset: 0;
  background: linear-gradient(135deg, rgba(250,250,248,0.3), rgba(250,250,248,0.1));
  border-radius: inherit;
  backdrop-filter: blur(4px);
}
.sensitivity-sensitive.revealed {
  filter: none;
}
.sensitivity-sensitive.revealed::after {
  display: none;
}

.sensitivity-critical {
  position: relative;
  filter: blur(16px) brightness(0.9);
}
.sensitivity-critical::after {
  content: '🔒';
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 24px;
  background: rgba(250, 250, 248, 0.6);
  backdrop-filter: blur(8px);
  border-radius: inherit;
}
```

### 5.4 按钮体系

```
┌────────────────────────────────────────────────────────────┐
│ Primary (主按钮)                                           │
│ 背景: --accent-primary  文字: #FFFFFF  圆角: 8px           │
│ 悬停: --accent-hover    按下: scale(0.97)                  │
│                                                            │
│ ┌─────────────────────────┐                                │
│ │     保存修改            │                                │
│ └─────────────────────────┘                                │
├────────────────────────────────────────────────────────────┤
│ Secondary (次按钮 / 玻璃按钮)                               │
│ 背景: rgba(255,255,255,0.2)  边框: 1px solid rgba(0,0,0,0.08)│
│ 悬停: 背景加深 + 边框加粗                                  │
│                                                            │
│ ┌─────────────────────────┐                                │
│ │     取消                │                                │
│ └─────────────────────────┘                                │
├────────────────────────────────────────────────────────────┤
│ Tertiary (文字按钮)                                        │
│ 背景: transparent  文字: --accent-primary                   │
│ 悬停: 背景 rgba(91,124,153,0.08)                           │
│                                                            │
│ 查看详情 →                                                 │
└────────────────────────────────────────────────────────────┘
```

---

## 6. 动效与交互

### 6.1 动效原则

受 Warp 细腻交互启发：

1. **有目的的动效**：每个动画都服务于用户理解（页面切换表示导航层级，微动效表示状态变化）
2. **物理质感**：使用弹簧曲线（spring easing）而非线性，像真实物体一样有弹性
3. **克制**：自动播放的动画不超过 300ms；避免闪烁、旋转等干扰性动画
4. **尊重系统偏好**：`prefers-reduced-motion` 时所有动画降为 0ms 或简单淡入淡出

### 6.2 平台差异化动效

| 场景 | macOS/iOS | Windows | Linux/Android |
|------|-----------|---------|--------------|
| 页面切换 | 滑动 + 淡入（0.35s spring） | 淡入淡出（0.2s ease） | 淡入淡出（0.2s ease） |
| 卡片展开 | 高度展开 + 阴影加深 | 高度展开 | 高度展开 |
| 按钮按下 | scale(0.97) + 阴影收缩 | scale(0.98) | scale(0.98) |
| 弹窗出现 | 从底部滑入（iOS）/ 缩放淡入（macOS） | 淡入 + 轻微缩放 | 淡入 |
| 侧边栏展开 | 弹性展开，内容跟随 | 线性展开 | 线性展开 |
| 敏感内容揭示 | 模糊渐隐（0.4s） | 直接切换 | 直接切换 |
| Toast 出现 | 从顶部滑入，停留 3s | 从顶部滑入 | 从底部滑入 |

### 6.3 关键交互时序

```css
/* 统一缓动函数 */
--ease-spring: cubic-bezier(0.34, 1.56, 0.64, 1); /* 弹性 */
--ease-smooth: cubic-bezier(0.4, 0, 0.2, 1);      /* Material 标准 */
--ease-decelerate: cubic-bezier(0, 0, 0.2, 1);    /* 减速 */

/* 时长 */
--duration-instant: 0ms;
--duration-fast: 100ms;    /* 悬停、聚焦 */
--duration-normal: 200ms;  /* 按钮按下、小状态切换 */
--duration-slow: 300ms;    /* 页面切换、弹窗 */
--duration-reveal: 400ms;  /* 敏感内容揭示 */
```

---

## 7. 暗色模式

### 7.1 暗色模式策略

- **跟随系统**：默认使用 `prefers-color-scheme` 自动切换
- **手动覆盖**：设置中提供"明亮 / 暗色 / 跟随系统"三选项
- **材质适配**：
  - macOS Liquid Glass → 暗色液态玻璃（更深的折射，冷调高光）
  - Windows Mica → Mica Dark（自动由 DWM 处理）
  - 回退材质 → 暗色纯色 + 降低的透明度

### 7.2 暗色材质示例

```css
/* 暗色液态玻璃 */
.liquid-glass-dark {
  background: linear-gradient(
    135deg,
    rgba(60, 60, 65, 0.5) 0%,
    rgba(40, 40, 45, 0.3) 100%
  );
  backdrop-filter: blur(40px) saturate(140%) brightness(0.9);
  border: 1px solid rgba(255, 255, 255, 0.1);
  box-shadow: 
    inset 0 1px 1px rgba(255, 255, 255, 0.1),
    0 8px 32px rgba(0, 0, 0, 0.3);
}
```

---

## 8. 可访问性

### 8.1 强制要求

| 要求 | 实现方式 |
|------|---------|
| 键盘完全可操作 | Tab 导航、Enter/Space 激活、Esc 关闭弹窗 |
| 屏幕阅读器支持 | ARIA label、role="dialog"、live region |
| 对比度 | 正文对比度 ≥ 4.5:1，大号文字 ≥ 3:1 |
| 焦点可见 | 所有交互元素有 2px `--accent-primary` 焦点环 |
| 动画降级 | `prefers-reduced-motion` 时禁用弹簧动画 |
| 透明降級 | `prefers-reduced-transparency` 时所有玻璃变纯色 |
| 触控目标 | 移动端按钮最小 44×44px，桌面端 32×32px |

### 8.2 减少透明度模式

当用户开启系统"减少透明度"：

```css
@media (prefers-reduced-transparency: reduce) {
  .liquid-glass,
  .acrylic,
  .mica-overlay {
    background: var(--bg-elevated) !important;
    backdrop-filter: none !important;
    border: 1px solid var(--border-subtle) !important;
    box-shadow: 0 2px 8px var(--shadow-sm) !important;
  }
}
```

---

## 9. 技术栈确认

### 9.1 最终选型

| 层 | 选型 | 版本 | 安装命令 |
|----|------|------|---------|
| UI 框架 | React | 19.x | `npm install react react-dom` |
| 构建工具 | Vite | 6.x | `npm install -D vite @vitejs/plugin-react` |
| 语言 | TypeScript | 5.7.x | `npm install -D typescript @types/react @types/react-dom` |
| 样式 | CSS Modules + 全局 CSS | — | 无需安装 |
| 状态管理 | Zustand | 5.x | `npm install zustand` |
| 路由 | React Router | 7.x | `npm install react-router-dom` |
| 图标 | Lucide React | latest | `npm install lucide-react` |
| 动画 | Framer Motion | 11.x | `npm install framer-motion` |
| 表格 | TanStack Table | 8.x | `npm install @tanstack/react-table` |
| Markdown | react-markdown | latest | `npm install react-markdown` |
| 日期 | date-fns | latest | `npm install date-fns` |
| 表单 | React Hook Form | 7.x | `npm install react-hook-form` |
| 校验 | Zod | 3.x | `npm install zod` |
| WebGL | 可选 Three.js | latest | `npm install three @types/three @react-three/fiber` |

### 9.2 不使用以下工具

| 工具 | 不使用原因 |
|------|-----------|
| Tailwind CSS | 与 Liquid Glass 复杂效果不兼容；团队无经验；现有代码为纯 CSS |
| styled-components / emotion | 运行时开销；与 CSS Modules 重复；调试困难 |
| Redux | 过于冗长；Zustand 更轻量；SoloSoul 不需要时间旅行调试 |
| Next.js | Tauri 不需要 SSR；Next.js 增加复杂度 |
| shadcn/ui | 样式与 Liquid Glass 不匹配；需要大量自定义 |

---

## 10. Flutter Widget → React Component 映射

### 10.1 基础 Widget 映射

| Flutter Widget | React Equivalent | 实现方式 | 备注 |
|---------------|-----------------|----------|------|
| `Scaffold` | `AppShell` | 自定义组件 | 页面骨架：AppBar + Body |
| `AppBar` | `AppBar` | 自定义 + CSS | Glass 效果 |
| `Container` | `div` | CSS | 通用容器 |
| `Column` | `display: flex; flex-direction: column` | CSS | 垂直布局 |
| `Row` | `display: flex; flex-direction: row` | CSS | 水平布局 |
| `Stack` | `position: relative/absolute` | CSS | 层叠布局 |
| `ListView` | 原生 `div` + overflow | CSS | 滚动列表 |
| `ListView.separated` | `div` + `gap` | CSS | 带分隔线的列表 |
| `GridView` | `display: grid` | CSS | 网格布局 |
| `SingleChildScrollView` | `overflow: auto` | CSS | 单内容滚动 |
| `Expanded` | `flex: 1` | CSS | 占据剩余空间 |
| `SizedBox` | `width/height` | CSS | 固定尺寸 |
| `Padding` | `padding` | CSS | 内边距 |
| `Margin` | `margin` | CSS | 外边距 |
| `Center` | `display: flex; justify-content: center; align-items: center` | CSS | 居中 |
| `Align` | `align-items` / `justify-content` | CSS | 对齐 |
| `Text` | `span` / `p` / `h1-h6` | 原生 | 文本 |
| `RichText` | 自行拼接 | 自定义 | 富文本 |
| `SelectableText` | `user-select: text` | CSS | 可选文本 |
| `TextField` | `<input>` / `<textarea>` | 原生 + CSS | 输入框 |
| `TextFormField` | React Hook Form + `<input>` | 库 + 原生 | 表单输入 |
| `Checkbox` | `<input type="checkbox">` | 原生 + CSS | 复选框 |
| `Switch` | `<input type="checkbox">` + CSS | 原生 + CSS | 开关 |
| `Slider` | `<input type="range">` | 原生 + CSS | 滑块 |
| `Radio` | `<input type="radio">` | 原生 + CSS | 单选 |
| `DropdownButton` | 自定义或 `<select>` | 自定义 | 下拉选择 |
| `PopupMenuButton` | 自定义 Popover | 自定义 | 弹出菜单 |
| `Dialog` | 自定义 Dialog 组件 | 自定义 + CSS | 对话框 |
| `BottomSheet` | 自定义 Sheet 组件 | 自定义 + Framer Motion | 底部弹窗 |
| `SnackBar` | Toast 组件 | 自定义或 sonner | 提示条 |
| `CircularProgressIndicator` | SVG / CSS animation | 自定义 | 圆形进度 |
| `LinearProgressIndicator` | `<progress>` / CSS | 原生 + CSS | 线性进度 |
| `Icon` | `<svg>` / Lucide | Lucide React | 图标 |
| `Image` | `<img>` | 原生 | 图片 |
| `Card` | `GlassCard` | 自定义 | 卡片 |

### 10.2 复杂 Widget 映射

| Flutter Widget | React Implementation | 说明 |
|---------------|---------------------|------|
| `LiquidGlassCard` | `GlassCard` (自定义) | CSS backdrop-filter |
| `LiquidGlassAppBar` | `AppBar` + glass CSS | 同 GlassCard |
| `LiquidGlassButton` | `Button` + glass CSS | 同 GlassCard |
| `SensitiveValueWidget` | `SensitiveValue` (自定义) | 状态控制可见性 |
| `SensitivityBlurredWidget` | `SensitiveValue` | 同 Flutter |
| `SectionCard` | `SectionCard` (自定义) | 分区卡片 |
| `CategoryChip` | `CategoryChip` (自定义) | 分类标签 |
| `TravelMap` | `TravelMap` (SVG/Canvas) | 旅行地图 |
| `ProfessionalTimeline` | `ProfessionalTimeline` (自定义) | 职业时间线 |
| `CustomCalendar` | `CustomCalendar` (自定义) | 自定义日历 |
| `SyncProgressDialog` | `SyncProgressDialog` (自定义) | 同步进度 |
| `PasswordVerificationDialog` | `PasswordVerificationDialog` | 密码验证 |
| `FieldEditorSheet` | `FieldEditorSheet` (自定义) | 字段编辑器 |
| `SectionEditorSheet` | `SectionEditorSheet` | 分区编辑器 |
| `LlmMessageBubble` | `LlmMessageBubble` | LLM 消息气泡 |
| `OcrScannerSheet` | `OcrScannerSheet` | OCR 扫描弹窗 |
| `ScanPreviewItem` | `ScanPreviewItem` | 扫描预览项 |
| `DiscoveredDeviceCard` | `DiscoveredDeviceCard` | 发现设备卡片 |
| `SectionSummaryCard` | `SectionSummaryCard` | 分区摘要 |
| `OperationLogEntryTile` | `OperationLogEntryTile` | 操作日志项 |
| `TrashItemCard` | `TrashItemCard` | 回收站项 |

---

## 11. Liquid Glass 组件实现

### 11.1 GlassCard（基础玻璃卡片）

```tsx
// src/components/liquid-glass/GlassCard.tsx
import styles from './GlassCard.module.css';
import { clsx } from 'clsx';

interface GlassCardProps {
  children: React.ReactNode;
  className?: string;
  variant?: 'default' | 'elevated' | 'subtle';
  interactive?: boolean;
}

export function GlassCard({
  children,
  className,
  variant = 'default',
  interactive = false,
}: GlassCardProps) {
  return (
    <div
      className={clsx(
        styles.glassCard,
        styles[variant],
        interactive && styles.interactive,
        className
      )}
    >
      {children}
    </div>
  );
}
```

```css
/* src/components/liquid-glass/GlassCard.module.css */
.glassCard {
  background: var(--glass-bg);
  backdrop-filter: var(--glass-blur);
  -webkit-backdrop-filter: var(--glass-blur);
  border: 1px solid var(--glass-border);
  border-radius: var(--glass-radius-md);
  box-shadow: var(--glass-shadow);
  transition: all 0.3s ease;
}

.elevated {
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.12);
}

.subtle {
  background: rgba(255, 255, 255, 0.08);
  border-color: rgba(255, 255, 255, 0.1);
}

.interactive:hover {
  background: var(--glass-bg-hover);
  box-shadow: var(--glass-shadow-hover);
  transform: translateY(-1px);
}

.interactive:active {
  background: var(--glass-bg-active);
  transform: translateY(0);
}

/* 深色模式 */
@media (prefers-color-scheme: dark) {
  .glassCard {
    background: var(--glass-dark-bg);
    border-color: var(--glass-dark-border);
    box-shadow: var(--glass-dark-shadow);
  }
}
```

### 11.2 GlassPanel（沉浸式 Shader 背景）

```tsx
// src/components/liquid-glass/GlassShader.tsx
import { useRef, useEffect } from 'react';
import styles from './GlassShader.module.css';

interface GlassShaderProps {
  className?: string;
}

export function GlassShader({ className }: GlassShaderProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const gl = canvas.getContext('webgl2');
    if (!gl) {
      // 降级：显示纯色背景
      canvas.style.background = 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)';
      return;
    }

    // WebGL Shader 初始化
    // ...（完整 Shader 代码见 ADR-002）

    return () => {
      // 清理 WebGL 上下文
      gl.getExtension('WEBGL_lose_context')?.loseContext();
    };
  }, []);

  return (
    <canvas
      ref={canvasRef}
      className={`${styles.glassShader} ${className || ''}`}
    />
  );
}
```

```css
/* src/components/liquid-glass/GlassShader.module.css */
.glassShader {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  z-index: -1;
}
```

### 11.3 AppBar（玻璃导航栏）

```tsx
// src/components/layout/AppBar.tsx
import styles from './AppBar.module.css';
import { GlassCard } from '../liquid-glass/GlassCard';

interface AppBarProps {
  title: string;
  actions?: React.ReactNode;
  onBack?: () => void;
}

export function AppBar({ title, actions, onBack }: AppBarProps) {
  return (
    <GlassCard className={styles.appBar} variant="subtle">
      <div className={styles.left}>
        {onBack && (
          <button className={styles.backButton} onClick={onBack}>
            <ArrowLeft size={20} />
          </button>
        )}
        <h1 className={styles.title}>{title}</h1>
      </div>
      <div className={styles.actions}>{actions}</div>
    </GlassCard>
  );
}
```

```css
/* src/components/layout/AppBar.module.css */
.appBar {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  height: 56px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px;
  z-index: 100;
  border-radius: 0;
  border-bottom: 1px solid var(--glass-border);
}

.left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.title {
  font-size: 18px;
  font-weight: 600;
  margin: 0;
}

.backButton {
  background: none;
  border: none;
  color: inherit;
  cursor: pointer;
  padding: 8px;
  border-radius: 8px;
  transition: background 0.2s;
}

.backButton:hover {
  background: rgba(255, 255, 255, 0.1);
}
```

---

## 12. 布局系统迁移

### 12.1 Flutter → CSS Flexbox/Grid 映射

```
Flutter Column(children: [A, B, C]) 
  → <div style="display: flex; flex-direction: column;"><A/><B/><C/></div>

Flutter Row(children: [A, Expanded(child: B), C])
  → <div style="display: flex;"><A/><div style="flex: 1;"><B/></div><C/></div>

Flutter Stack(children: [A, Positioned(top: 0, child: B)])
  → <div style="position: relative;"><A/><div style="position: absolute; top: 0;"><B/></div></div>

Flutter GridView.count(crossAxisCount: 2)
  → <div style="display: grid; grid-template-columns: repeat(2, 1fr);">...</div>
```

### 12.2 AppShell（应用外壳）

```tsx
// src/components/layout/AppShell.tsx
import styles from './AppShell.module.css';
import { SideNavigation } from './SideNavigation';
import { AppBar } from './AppBar';

interface AppShellProps {
  children: React.ReactNode;
  title: string;
  actions?: React.ReactNode;
}

export function AppShell({ children, title, actions }: AppShellProps) {
  return (
    <div className={styles.appShell}>
      <SideNavigation />
      <div className={styles.main}>
        <AppBar title={title} actions={actions} />
        <main className={styles.content}>{children}</main>
      </div>
    </div>
  );
}
```

```css
/* src/components/layout/AppShell.module.css */
.appShell {
  display: flex;
  height: 100vh;
  overflow: hidden;
}

.main {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.content {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
  padding-top: calc(56px + 16px); /* AppBar height + padding */
}
```

### 12.3 响应式断点

```css
/* styles/tokens.css */
:root {
  --breakpoint-mobile: 480px;
  --breakpoint-tablet: 768px;
  --breakpoint-desktop: 1024px;
  --breakpoint-wide: 1440px;
}
```

```tsx
// src/hooks/useMediaQuery.ts
import { useState, useEffect } from 'react';

export function useMediaQuery(query: string): boolean {
  const [matches, setMatches] = useState(false);

  useEffect(() => {
    const media = window.matchMedia(query);
    setMatches(media.matches);
    const listener = (e: MediaQueryListEvent) => setMatches(e.matches);
    media.addEventListener('change', listener);
    return () => media.removeEventListener('change', listener);
  }, [query]);

  return matches;
}

// 预设断点
export const useIsMobile = () => useMediaQuery('(max-width: 480px)');
export const useIsTablet = () => useMediaQuery('(max-width: 768px)');
export const useIsDesktop = () => useMediaQuery('(min-width: 1024px)');
```

---

## 13. 路由系统迁移

### 13.1 Flutter Navigator → React Router

```tsx
// src/App.tsx
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { useAuthStore } from './stores/authStore';

// 页面导入
import { BootstrapPage } from './pages/auth/BootstrapPage';
import { LoginPage } from './pages/auth/LoginPage';
import { HomePage } from './pages/home/HomePage';
import { ObjectWorkspacePage } from './pages/workspace/ObjectWorkspacePage';
import { ObjectEditorPage } from './pages/editor/ObjectEditorPage';
import { SearchPage } from './pages/search/SearchPage';
import { SettingsPage } from './pages/settings/SettingsPage';
import { SecuritySettingsPage } from './pages/settings/SecuritySettingsPage';
import { SensitivitySettingsPage } from './pages/settings/SensitivitySettingsPage';
import { DataManagementPage } from './pages/settings/DataManagementPage';
import { ExportImportPage } from './pages/settings/ExportImportPage';
import { BackupConfigPage } from './pages/settings/BackupConfigPage';
import { TrashPage } from './pages/settings/TrashPage';
import { OperationLogPage } from './pages/settings/OperationLogPage';
import { LlmChatPage } from './pages/ai/LlmChatPage';
import { PluginDashboardPage } from './pages/ai/PluginDashboardPage';
import { SyncPage } from './pages/sync/SyncPage';
import { DebugLogPage } from './pages/system/DebugLogPage';
import { AboutPage } from './pages/system/AboutPage';

function App() {
  const { isAuthenticated, hasAccount } = useAuthStore();

  return (
    <BrowserRouter>
      <Routes>
        {/* 引导流程 */}
        <Route path="/bootstrap" element={<BootstrapPage />} />
        
        {/* 认证 */}
        <Route path="/login" element={<LoginPage />} />
        
        {/* 受保护路由 */}
        <Route element={<ProtectedRoute isAuthenticated={isAuthenticated} />}>
          <Route path="/" element={<HomePage />} />
          <Route path="/home" element={<Navigate to="/" replace />} />
          
          {/* 对象管理 */}
          <Route path="/workspace/:categoryId?" element={<ObjectWorkspacePage />} />
          <Route path="/editor/:objectId?" element={<ObjectEditorPage />} />
          
          {/* 全局搜索 */}
          <Route path="/search" element={<SearchPage />} />
          
          {/* 设置 */}
          <Route path="/settings" element={<SettingsPage />} />
          <Route path="/settings/security" element={<SecuritySettingsPage />} />
          <Route path="/settings/sensitivity" element={<SensitivitySettingsPage />} />
          <Route path="/settings/data" element={<DataManagementPage />} />
          <Route path="/settings/export-import" element={<ExportImportPage />} />
          <Route path="/settings/backup" element={<BackupConfigPage />} />
          <Route path="/settings/trash" element={<TrashPage />} />
          <Route path="/settings/operation-log" element={<OperationLogPage />} />
          
          {/* AI */}
          <Route path="/llm-chat" element={<LlmChatPage />} />
          <Route path="/plugins" element={<PluginDashboardPage />} />
          
          {/* 同步 */}
          <Route path="/sync" element={<SyncPage />} />
          
          {/* 系统 */}
          <Route path="/debug-log" element={<DebugLogPage />} />
          <Route path="/about" element={<AboutPage />} />
        </Route>
        
        {/* 404 */}
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </BrowserRouter>
  );
}

// 受保护路由组件
function ProtectedRoute({ isAuthenticated, children }: { isAuthenticated: boolean; children: React.ReactNode }) {
  if (!isAuthenticated) {
    return <Navigate to="/login" replace />;
  }
  return children;
}

export default App;
```

### 13.2 路由参数映射

```tsx
// Flutter: Navigator.pushNamed(context, '/editor', arguments: {'objectId': 'xxx'})
// React: <Link to="/editor/xxx" /> 或 navigate('/editor/xxx')

// 获取参数
import { useParams } from 'react-router-dom';

function ObjectEditorPage() {
  const { objectId } = useParams<{ objectId: string }>();
  // objectId 可能为 undefined（新建对象）
}
```

---

## 14. 表单系统迁移

### 14.1 Flutter Form → React Hook Form + Zod

```tsx
// Flutter:
// Form(
//   key: _formKey,
//   child: Column(
//     children: [
//       TextFormField(validator: ...),
//       TextFormField(validator: ...),
//     ],
//   ),
// )

// React:
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';

const schema = z.object({
  fullName: z.string().min(1, '姓名不能为空'),
  dateOfBirth: z.string().regex(/^\d{4}-\d{2}-\d{2}$/, '日期格式为 YYYY-MM-DD'),
  email: z.string().email('邮箱格式不正确').optional(),
});

type FormData = z.infer<typeof schema>;

function IdentityForm() {
  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<FormData>({
    resolver: zodResolver(schema),
  });

  const onSubmit = (data: FormData) => {
    console.log(data);
  };

  return (
    <form onSubmit={handleSubmit(onSubmit)}>
      <div>
        <label>姓名</label>
        <input {...register('fullName')} />
        {errors.fullName && <span>{errors.fullName.message}</span>}
      </div>
      
      <div>
        <label>出生日期</label>
        <input type="date" {...register('dateOfBirth')} />
        {errors.dateOfBirth && <span>{errors.dateOfBirth.message}</span>}
      </div>
      
      <div>
        <label>邮箱</label>
        <input type="email" {...register('email')} />
        {errors.email && <span>{errors.email.message}</span>}
      </div>
      
      <button type="submit">保存</button>
    </form>
  );
}
```

### 14.2 防抖保存

```tsx
// src/hooks/useDebouncedSave.ts
import { useCallback, useRef } from 'react';

export function useDebouncedSave<T>(
  saveFn: (data: T) => Promise<void>,
  delay: number = 500
) {
  const timeoutRef = useRef<ReturnType<typeof setTimeout>>();

  const debouncedSave = useCallback(
    (data: T) => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
      timeoutRef.current = setTimeout(() => {
        saveFn(data);
      }, delay);
    },
    [saveFn, delay]
  );

  const immediateSave = useCallback(
    (data: T) => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
      saveFn(data);
    },
    [saveFn]
  );

  return { debouncedSave, immediateSave };
}
```

---

## 15. 前端架构与技术实现路线图

### 15.1 前端目录架构

```
src/ui/
├── design-system/
│   ├── tokens/              # CSS 变量定义
│   │   ├── colors.css       # 明亮/暗色色彩 token
│   │   ├── typography.css   # 字体、字号、行高
│   │   ├── spacing.css      # 间距、圆角、阴影
│   │   └── motion.css       # 动画时长、缓动
│   ├── materials/           # 跨平台材质组件
│   │   ├── GlassPanel.tsx   # 通用玻璃面板（自动适配平台）
│   │   ├── MicaWindow.tsx   # Windows Mica 窗口包装
│   │   ├── AcrylicPanel.tsx # Acrylic 回退面板
│   │   └── SolidPanel.tsx   # 纯色回退面板
│   ├── components/          # 基础组件
│   │   ├── Button/
│   │   ├── Input/
│   │   ├── Card/
│   │   ├── Dialog/
│   │   ├── Sidebar/
│   │   └── SensitivityOverlay/
│   └── hooks/
│       ├── usePlatform.ts   # 获取平台能力
│       ├── useMaterial.ts   # 获取当前材质定义
│       └── useTheme.ts      # 主题切换
```

### 15.2 Tauri 主进程配合

```rust
// crates/solosoul-tauri/src/platform.rs
pub struct PlatformCapabilities {
    pub supports_liquid_glass: bool,
    pub supports_mica: bool,
    pub supports_acrylic: bool,
    pub prefers_reduced_transparency: bool,
    pub prefers_reduced_motion: bool,
}

impl PlatformCapabilities {
    pub fn detect() -> Self {
        #[cfg(target_os = "macos")]
        { /* 检测 macOS 版本 */ }
        #[cfg(target_os = "windows")]
        { /* 检测 Windows 版本 + DWM 能力 */ }
        // ...
    }
}

// 通过 Tauri command 暴露给前端
#[tauri::command]
fn get_platform_capabilities() -> PlatformCapabilities {
    PlatformCapabilities::detect()
}
```

### 15.3 动画与过渡实现

使用 Framer Motion 实现 Flutter 风格的物理动画：

```tsx
// 页面过渡
import { motion, AnimatePresence } from 'framer-motion';

function PageTransition({ children }: { children: React.ReactNode }) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -20 }}
      transition={{ duration: 0.3 }}
    >
      {children}
    </motion.div>
  );
}

// 列表项动画
function AnimatedList({ items }: { items: Item[] }) {
  return (
    <AnimatePresence>
      {items.map((item) => (
        <motion.div
          key={item.id}
          initial={{ opacity: 0, height: 0 }}
          animate={{ opacity: 1, height: 'auto' }}
          exit={{ opacity: 0, height: 0 }}
          transition={{ duration: 0.2 }}
        >
          {item.content}
        </motion.div>
      ))}
    </AnimatePresence>
  );
}

// 底部弹窗（BottomSheet）
function BottomSheet({ isOpen, onClose, children }: BottomSheetProps) {
  return (
    <AnimatePresence>
      {isOpen && (
        <>
          <motion.div
            className="overlay"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            onClick={onClose}
          />
          <motion.div
            className="sheet"
            initial={{ y: '100%' }}
            animate={{ y: 0 }}
            exit={{ y: '100%' }}
            transition={{ type: 'spring', damping: 25, stiffness: 300 }}
          >
            {children}
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );
}
```

### 15.4 从零实现顺序

| 阶段 | 内容 | 目标平台 |
|------|------|---------|
| **P0** | 搭建 Vite + React + TypeScript 项目；配置 Tauri 集成；实现 Design Tokens（OKLCH 色彩、间距、字体）；实现基础组件（Button、Input、Card）；实现布局组件（AppShell、AppBar、SideNavigation）；配置路由（React Router）与状态管理（Zustand） | 全部 |
| **P1** | 实现 Liquid Glass 组件（GlassCard、GlassPanel）；实现敏感遮罩组件；实现表单系统（React Hook Form + Zod）；迁移页面（按用户旅程 J1→J7）；CSS backdrop-filter 模糊（Acrylic/标准模糊） | macOS 14-, Windows 10, Linux, Android |
| **P2** | Windows Mica 原生集成（Tauri 主进程 DWM API）；Mica Alt 侧边栏适配 | Windows 11 |
| **P3** | Liquid Glass CSS 高级效果（高光边缘、折射）；WebGL Shader 背景（可选）；Framer Motion 页面过渡与微交互 | macOS 15+, iOS 26+ |
| **P4** | WebGL 着色器增强（动态反射、景深）；性能分级（标准/高级/自适应）；可访问性全面审计 | 高端 macOS / iOS |

---

## 16. 设计验证清单

### 16.1 每平台验收标准

- [ ] **macOS 15+**：窗口背景有液态玻璃质感，卡片有厚度光带，交互有弹簧动效
- [ ] **macOS 14**：Acrylic 模糊自然，无性能问题，与原生应用视觉协调
- [ ] **Windows 11**：Mica 材质正确采样桌面背景，暗色模式切换无缝
- [ ] **Windows 10**：Acrylic 回退清晰，无视觉破碎
- [ ] **iOS 26+**：液态玻璃与系统 UI 风格一致，底部安全区正确适配
- [ ] **iOS 25-**：标准模糊不突兀，触控反馈灵敏
- [ ] **Android 14+**：模糊效果流畅，Material You 动态取色可选
- [ ] **Linux (GNOME/KDE)**：自适应模糊或纯色回退均可用
- [ ] **全部平台**：开启"减少透明度"后，所有玻璃效果优雅退化为纯色
- [ ] **全部平台**：开启"减弱动态效果"后，无自动播放动画

### 16.2 对比度检查

使用 WebAIM 对比度检查器验证：
- 明亮模式正文 `#1A1A1A` on `#FAFAF8` = **15.8:1** ✅
- 暗色模式正文 `#F5F5F5` on `#1C1C1E` = **14.9:1** ✅
- 辅助文字 `#6B6B6B` on `#FAFAF8` = **5.4:1** ✅
- 强调按钮文字 `#FFFFFF` on `#5B7C99` = **4.8:1** ✅

### 16.3 功能验证清单

- [ ] 所有 Flutter 复杂 Widget 均有 React 对应实现
- [ ] 路由系统完整覆盖现有所有页面
- [ ] 表单验证与防抖保存正常工作
- [ ] 敏感内容遮罩六级分级正常运作
- [ ] 暗色模式全平台切换无闪烁
- [ ] 响应式布局在三段断点下正常
- [ ] 键盘导航完整（Tab、Enter、Esc）
- [ ] 屏幕阅读器可正确朗读所有交互元素

---

## 17. 参考资源

| 资源 | 链接 | 用途 |
|------|------|------|
| Apple Liquid Glass Guidelines | developer.apple.com/design/human-interface-guidelines | 液态玻璃视觉参考 |
| Windows Mica Material | learn.microsoft.com/windows/apps/design/style/mica | Mica 实现文档 |
| Warp Terminal Design | warp.dev | 温暖配色与微交互灵感 |
| Notion Design Blog | notion.so/blog | 极简排版与信息架构 |
| Anytype Design Principles | anytype.io | 对象图谱与温暖基调 |
| Material Design 3 | m3.material.io | 组件规范与可访问性基准 |
| React 官方文档 | react.dev | UI 框架参考 |
| Framer Motion | framer.com/motion | 动画库文档 |
| Zustand 文档 | github.com/pmndrs/zustand | 状态管理参考 |
| Tauri v2 文档 | v2.tauri.app | 桌面端框架参考 |

---

## 18. 变更日志

| 日期 | 版本 | 变更 |
|------|------|------|
| 2026-06-05 | v1.0 | 合并《前端框架与UI迁移》与《跨平台UI设计系统》：整合设计理念、材质系统、技术栈、组件映射、代码实现、迁移路线图于单一文档 |
