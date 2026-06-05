# ADR-002: Liquid Glass Shader 实现方案

> **状态**: 已采纳 ✅  
> **决策日期**: 2026-06-04  
> **决策人**: SoloSoul 架构组  
> **影响范围**: UI 视觉效果、渲染性能、包体积、开发复杂度

---

## 背景

SoloSoul 的 UI 设计语言采用 **iOS 26 Liquid Glass（液态玻璃）** 风格，这是当前 Flutter 客户端的核心视觉特征。迁移到 Tauri + Web 技术栈后，需要在前端重新实现这一视觉效果。

当前 Flutter 实现依赖 `liquid_glass_widgets: ^0.10.6` 包（36 个组件），该包内部使用 Flutter CustomPainter + Shader 实现玻璃质感。

## Liquid Glass 视觉特征定义

从 iOS 26 设计语言中提取的核心特征：

| 特征 | 描述 | 当前 Flutter 实现 |
|------|------|----------------|
| **透光性** | 背景内容模糊透过表面 | `BackdropFilter` + `ImageFilter.blur` |
| **光泽感** | 表面有微妙的高光反射 | `RadialGradient` + `BoxShadow` |
| **厚度感** | 边缘有折射/深度感 | `Border` + 多层阴影 |
| **动态光效** | 随滚动/移动产生光影变化 | `CustomPainter` + 动画 |
| **色彩中性** | 不自带颜色，完全依赖背景 | `Color` with opacity |
| **圆角** | 大圆角（16px+），柔和边缘 | `BorderRadius.circular(16)` |

## 候选实现方案

### 方案 A: CSS backdrop-filter（推荐 - 基础组件）

**技术**: 纯 CSS，无 JavaScript/Shader

```css
.liquid-glass {
  /* 核心透光 */
  background: rgba(255, 255, 255, 0.15);
  backdrop-filter: blur(20px) saturate(180%);
  -webkit-backdrop-filter: blur(20px) saturate(180%);
  
  /* 光泽边框 */
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 16px;
  
  /* 厚度阴影 */
  box-shadow: 
    0 4px 30px rgba(0, 0, 0, 0.1),
    inset 0 1px 0 rgba(255, 255, 255, 0.2);
}

/* 深色模式变体 */
.liquid-glass-dark {
  background: rgba(0, 0, 0, 0.25);
  border: 1px solid rgba(255, 255, 255, 0.1);
  box-shadow: 
    0 4px 30px rgba(0, 0, 0, 0.3),
    inset 0 1px 0 rgba(255, 255, 255, 0.05);
}

/* 悬浮状态增强 */
.liquid-glass:hover {
  background: rgba(255, 255, 255, 0.2);
  box-shadow: 
    0 8px 40px rgba(0, 0, 0, 0.15),
    inset 0 1px 0 rgba(255, 255, 255, 0.3);
}
```

**优势**:
- **性能最优**: GPU 加速，零 JavaScript 开销
- **实现最简单**: 纯 CSS，无额外依赖
- **浏览器支持好**: Safari/Chrome/Edge 均支持（Firefox 需 flag）
- **与 Tauri 完美兼容**: Tauri 使用系统 WebView（macOS Safari/WebKit，Windows Edge/WebView2，Linux WebKitGTK），均支持 `backdrop-filter`

**劣势**:
- **效果相对简单**: 无法实现复杂的动态光影、折射模拟
- **无厚度深度感**: 边缘折射效果有限
- **光效静态**: 无法随内容滚动产生动态高光

**适用场景**: 所有基础组件（卡片、弹窗、导航栏、按钮）

---

### 方案 B: WebGL Fragment Shader（推荐 - 高级组件）

**技术**: raw WebGL / Three.js / glsl-canvas

```glsl
// 简化版 Liquid Glass Fragment Shader
precision mediump float;

uniform vec2 u_resolution;
uniform vec2 u_mouse;
uniform float u_time;
uniform sampler2D u_background;

varying vec2 v_uv;

// 噪声函数
float hash(vec2 p) {
  return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float noise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  f = f * f * (3.0 - 2.0 * f);
  
  float a = hash(i);
  float b = hash(i + vec2(1.0, 0.0));
  float c = hash(i + vec2(0.0, 1.0));
  float d = hash(i + vec2(1.0, 1.0));
  
  return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

void main() {
  vec2 uv = v_uv;
  
  // 1. 背景模糊采样（多层采样模拟高斯模糊）
  vec4 bgColor = vec4(0.0);
  float totalWeight = 0.0;
  
  for(float x = -3.0; x <= 3.0; x += 1.0) {
    for(float y = -3.0; y <= 3.0; y += 1.0) {
      vec2 offset = vec2(x, y) * 0.003;
      float weight = exp(-(x*x + y*y) / 8.0);
      bgColor += texture2D(u_background, uv + offset) * weight;
      totalWeight += weight;
    }
  }
  bgColor /= totalWeight;
  
  // 2. 玻璃基底颜色
  vec3 glassBase = mix(
    vec3(1.0, 1.0, 1.0),
    bgColor.rgb,
    0.7  // 透光率
  );
  
  // 3. 高光反射（模拟光源）
  vec2 lightPos = vec2(0.3, 0.3) + vec2(sin(u_time * 0.5) * 0.1, cos(u_time * 0.3) * 0.1);
  float lightDist = length(uv - lightPos);
  float highlight = exp(-lightDist * lightDist * 8.0) * 0.3;
  
  // 4. 边缘厚度（菲涅尔效应）
  vec2 edgeDist = abs(uv - 0.5) * 2.0;
  float edgeFactor = max(edgeDist.x, edgeDist.y);
  float fresnel = pow(edgeFactor, 3.0) * 0.15;
  
  // 5. 内部微光纹理
  float microShine = noise(uv * 30.0 + u_time * 0.1) * 0.03;
  
  // 合成
  vec3 finalColor = glassBase + vec3(highlight) + vec3(fresnel) + vec3(microShine);
  float alpha = 0.85 + fresnel * 0.3;
  
  gl_FragColor = vec4(finalColor, alpha);
}
```

**React 集成（使用 @react-three/fiber）**:
```tsx
import { Canvas } from '@react-three/fiber';

function LiquidGlassCard({ children }) {
  return (
    <div className="relative">
      <Canvas className="absolute inset-0 -z-10">
        <LiquidGlassShader />
      </Canvas>
      <div className="relative z-10">{children}</div>
    </div>
  );
}
```

**优势**:
- **效果最逼真**: 可实现真实的折射、动态光影、厚度感
- **完全可控**: 每个像素都可编程
- **动态光效**: 可响应鼠标移动、滚动、时间变化

**劣势**:
- **性能开销大**: WebGL 上下文占用 GPU 资源，大量 Shader 组件会拖慢性能
- **实现复杂**: 需要 GLSL 知识，调试困难
- **电池消耗**: 持续运行的 Shader 动画增加功耗
- **包体积**: Three.js ~150KB+（gzip）
- **Tauri 兼容性**: 需测试各平台 WebView 的 WebGL 支持

**适用场景**: 仅用于关键视觉组件（Hero 区域、主卡片、设置面板背景），**不要**用于普通列表项、按钮等高频组件

---

### 方案 C: CSS + SVG 滤镜混合

**技术**: CSS `backdrop-filter` + SVG `<filter>`

```xml
<svg width="0" height="0">
  <filter id="liquid-glass-filter">
    <!-- 高斯模糊 -->
    <feGaussianBlur in="SourceGraphic" stdDeviation="10" result="blur" />
    
    <!-- 颜色矩阵增强饱和度 -->
    <feColorMatrix in="blur" type="saturate" values="1.8" result="saturated" />
    
    <!-- 高光合成 -->
    <feSpecularLighting in="saturated" surfaceScale="5" specularConstant="0.75" 
                        specularExponent="20" lighting-color="#ffffff" result="specular">
      <fePointLight x="50" y="50" z="200" />
    </feSpecularLighting>
    
    <!-- 合成 -->
    <feComposite in="specular" in2="saturated" operator="in" result="composite" />
    <feBlend in="composite" in2="saturated" mode="screen" />
  </filter>
</svg>
```

```css
.liquid-glass-svg {
  filter: url(#liquid-glass-filter);
  backdrop-filter: blur(10px);
}
```

**优势**:
- **无需 JavaScript**: 纯 CSS + SVG
- **效果优于纯 CSS**: SVG 滤镜提供更多图像处理能力
- ** declarative**: 声明式定义，易于维护

**劣势**:
- **性能差**: SVG 滤镜在复杂场景下性能远不如 GPU 加速的 `backdrop-filter`
- **浏览器支持不一致**: 各浏览器 SVG 滤镜实现有差异
- **动态效果困难**: 无法实现响应鼠标/滚动的动态效果

**结论**: 不采用。性能问题明显，优势不足以弥补。

---

### 方案 D: 预渲染图片 + CSS 蒙版

**技术**: 设计工具（Figma/Sketch）生成玻璃效果图片 + CSS mask

**优势**:
- **性能最好**: 纯图片渲染
- **效果最稳定**: 所见即所得

**劣势**:
- **无法响应背景**: 玻璃效果需要看到背景内容，预渲染图片无法做到
- **无法动态调整**: 尺寸、颜色、透明度都无法动态变化
- **包体积大**: 需要多种尺寸、多种颜色的图片资源

**结论**: 完全不采用。与 Liquid Glass 的核心特征（透光性）冲突。

---

## 决策：分层实现策略

**采用方案 A（CSS）为主 + 方案 B（WebGL Shader）为辅的分层策略**。

### 分层定义

```
┌─────────────────────────────────────────────┐
│  层级 3: 沉浸式 Shader（WebGL）              │
│  使用场景: 启动页背景、设置面板 Hero、关于页  │
│  组件数: < 5 个                              │
│  性能预算: 每个页面最多 1 个                 │
├─────────────────────────────────────────────┤
│  层级 2: 增强 CSS（backdrop-filter + 动画）  │
│  使用场景: 卡片、弹窗、导航栏、侧边栏         │
│  组件数: ~20 个                              │
│  性能预算: 滚动流畅 60fps                    │
├─────────────────────────────────────────────┤
│  层级 1: 基础 CSS（纯色/渐变 fallback）      │
│  使用场景: 按钮、列表项、输入框               │
│  组件数: ~40 个                              │
│  性能预算: 无感知开销                        │
└─────────────────────────────────────────────┘
```

### 降级策略

```typescript
// 性能检测 + 自动降级
function detectPerformanceTier(): 'shader' | 'enhanced' | 'basic' {
  // 检测 GPU 能力
  const canvas = document.createElement('canvas');
  const gl = canvas.getContext('webgl2');
  if (!gl) return 'basic';
  
  // 检测渲染器
  const debugInfo = gl.getExtension('WEBGL_debug_renderer_info');
  if (debugInfo) {
    const renderer = gl.getParameter(debugInfo.UNMASKED_RENDERER_WEBGL);
    // 集成显卡降级
    if (renderer.includes('Intel')) return 'enhanced';
  }
  
  // 默认增强
  return 'enhanced';
}
```

### 具体实现清单

| 组件 | 层级 | 实现方案 | 备注 |
|------|------|---------|------|
| `AppBar` | 层级 2 | CSS backdrop-filter + 底部 border | 固定在顶部 |
| `SideNavigation` | 层级 2 | CSS backdrop-filter | 固定在侧边 |
| `SoloGlassCard` | 层级 2 | CSS backdrop-filter + 悬浮增强 | 通用卡片 |
| `Modal/Dialog` | 层级 2 | CSS backdrop-filter + overlay blur | 全局遮罩 |
| `SectionCard` | 层级 2 | CSS backdrop-filter | 分区卡片 |
| `Button` | 层级 1 | 纯色 + 渐变 fallback | 不需要玻璃 |
| `TextField` | 层级 1 | 纯色 + border | 不需要玻璃 |
| `ListTile` | 层级 1 | 纯色 + 悬浮背景 | 不需要玻璃 |
| `SplashPage` 背景 | 层级 3 | WebGL Shader | 启动页沉浸式 |
| `AboutPage` 背景 | 层级 3 | WebGL Shader | 关于页品牌展示 |
| `SettingsPage` Hero | 层级 3 | WebGL Shader | 设置页头部 |
| `HomePage` 欢迎区 | 层级 2 | CSS backdrop-filter | 首页欢迎卡片 |

---

## CSS 设计令牌（Design Tokens）

```css
/* styles/tokens.css */
:root {
  /* Liquid Glass - Light Mode */
  --glass-bg: rgba(255, 255, 255, 0.15);
  --glass-bg-hover: rgba(255, 255, 255, 0.2);
  --glass-bg-active: rgba(255, 255, 255, 0.1);
  --glass-border: rgba(255, 255, 255, 0.2);
  --glass-border-strong: rgba(255, 255, 255, 0.3);
  --glass-shadow: 0 4px 30px rgba(0, 0, 0, 0.1);
  --glass-shadow-hover: 0 8px 40px rgba(0, 0, 0, 0.15);
  --glass-blur: blur(20px) saturate(180%);
  --glass-radius-sm: 12px;
  --glass-radius-md: 16px;
  --glass-radius-lg: 24px;
  
  /* Liquid Glass - Dark Mode */
  --glass-dark-bg: rgba(0, 0, 0, 0.25);
  --glass-dark-bg-hover: rgba(0, 0, 0, 0.3);
  --glass-dark-border: rgba(255, 255, 255, 0.1);
  --glass-dark-shadow: 0 4px 30px rgba(0, 0, 0, 0.3);
}

@media (prefers-reduced-motion: reduce) {
  /* 减弱动画偏好 */
  .liquid-glass {
    transition: none !important;
    animation: none !important;
  }
}
```

---

## 性能预算

| 指标 | 目标值 | 测量方法 |
|------|--------|---------|
| 首次渲染 | < 100ms | Lighthouse FCP |
| 滚动帧率 | 60fps | Chrome DevTools Performance |
| GPU 内存 | < 200MB | Chrome Task Manager |
| Shader 组件最大数量 | 每页 1-2 个 | 代码审查 |
| CSS backdrop-filter 最大数量 | 视口内 < 10 个 | 运行时检测 |

---

## 相关文档

- `L5_UI组件与设计系统层.md` — 完整组件清单与规范
- `tauri_refactor/前端框架与UI迁移.md` — 具体组件迁移方案
- `ADR-001-前端框架选型分析.md` — React 19 选型决策

---

*文档版本：v1.0*  
*创建日期：2026-06-04*
