# L5 UI 组件与设计系统层

> **层级定位**：可复用的 UI 构建块，不持有业务逻辑，通过参数和回调与上层（L4 状态层 / L6 页面层）交互。所有组件遵循 Liquid Glass 设计语言，提供统一的视觉和交互体验。
>
> **核心原则**：组件纯函数化（相同输入 → 相同输出）；敏感数据通过专用组件处理（禁止各页面自行实现掩码）；所有组件支持国际化文本注入；响应式适配桌面/移动端。
>
> **Tauri 迁移方向**：用 Web 技术栈（HTML/CSS/JS）重写所有组件，复刻 Liquid Glass 玻璃质感。可用 CSS backdrop-filter + 自定义 Shader 实现玻璃效果。

---

## 目录

- [L5.1 设计系统基础](#l51-设计系统基础)
- [L5.2 布局组件](#l52-布局组件)
- [L5.3 数据展示组件](#l53-数据展示组件)
- [L5.4 敏感数据组件](#l54-敏感数据组件)
- [L5.5 表单与输入组件](#l55-表单与输入组件)
- [L5.6 对话框组件](#l56-对话框组件)
- [L5.7 首页专用组件](#l57-首页专用组件)
- [L5.8 LLM 专用组件](#l58-llm-专用组件)
- [L5.9 扫描专用组件](#l59-扫描专用组件)
- [L5.10 设置/数据管理专用组件](#l510-设置数据管理专用组件)
- [Liquid Glass 复刻方案](#liquid-glass-复刻方案)
- [从零开始实现顺序](#从零开始实现顺序)

---

## L5.1 设计系统基础

### 1.1 Liquid Glass 包装器

**当前**：`liquid_glass_widgets` 插件

```dart
// 全局启用
LiquidGlassWidgets.wrap(
  adaptiveQuality: true,
  child: GlassTheme(
    data: AppTheme.glassThemeData,
    child: MaterialApp(...),
  ),
)
```

**核心能力**：
- `GlassBackdropScope` — 全局玻璃质感背景
- `GlassAdaptiveScope` — 根据设备性能自适应渲染质量
- `GlassTheme` — 集中配置玻璃参数（模糊半径、折射率、光泽度）

**Tauri 复刻方案**：
```css
/* CSS 实现玻璃质感 */
.glass {
  background: rgba(255, 255, 255, 0.15);
  backdrop-filter: blur(20px) saturate(180%);
  -webkit-backdrop-filter: blur(20px) saturate(180%);
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 16px;
  box-shadow: 
    0 4px 30px rgba(0, 0, 0, 0.1),
    inset 0 1px 0 rgba(255, 255, 255, 0.2);
}

.glass-dark {
  background: rgba(0, 0, 0, 0.3);
  border: 1px solid rgba(255, 255, 255, 0.1);
}
```

**进阶方案**：WebGL Shader 实现更真实的物理折射

### 1.2 Material 3 主题

**当前**：`lib/presentation/theme/app_theme.dart`

```dart
ThemeData lightTheme = ThemeData(
  useMaterial3: true,
  colorScheme: ColorScheme.fromSeed(seedColor: primaryColor),
  // ...
);

ThemeData darkTheme = ThemeData(
  useMaterial3: true,
  brightness: Brightness.dark,
  colorScheme: ColorScheme.fromSeed(seedColor: primaryColor, brightness: Brightness.dark),
);
```

**Tauri 复刻方案**：
- CSS 变量定义 Design Tokens
- 明暗模式通过 `prefers-color-scheme` 或手动切换 `data-theme` 属性

```css
:root {
  --primary: #6366f1;
  --primary-container: #e0e7ff;
  --surface: #ffffff;
  --surface-glass: rgba(255, 255, 255, 0.15);
  --on-surface: #1f2937;
  --outline: rgba(0, 0, 0, 0.12);
}

[data-theme="dark"] {
  --primary: #818cf8;
  --primary-container: #312e81;
  --surface: #0f172a;
  --surface-glass: rgba(0, 0, 0, 0.3);
  --on-surface: #f1f5f9;
  --outline: rgba(255, 255, 255, 0.12);
}
```

### 1.3 SnackBar 系统

**当前**：`showOverlaySnackBar(context, message, type)`

```dart
enum SnackBarType { success, error, warning, info }
```

**Tauri 复刻方案**：
- 自定义 Toast 组件（CSS 动画）
- 或 `react-hot-toast` / `sonner` 等库，自定义样式匹配 Liquid Glass

### 1.4 图标解析器

**当前**：`lib/presentation/utils/icon_resolver.dart`

- 字符串图标名（如 `"passport"`, `"credit_card"`）→ `IconData`
- 支持 Material Icons 和自定义 SVG

**Tauri 复刻方案**：
- `lucide-react` 或 `heroicons`，图标名映射表
- 自定义 SVG 图标组件

---

## L5.2 布局组件

### 2.1 ScaffoldWithSidebar

**当前**：`lib/presentation/widgets/scaffold_with_sidebar.dart`

| 特性 | 描述 | Tauri 实现 |
|------|------|-----------|
| **桌面端** | 左侧固定侧边栏（宽度 260px）+ 右侧内容区 | CSS Grid：`grid-template-columns: 260px 1fr` |
| **移动端** | 底部导航栏 / Drawer | `@media (max-width: 768px)` 切换布局 |
| **响应式断点** | 768px | Tailwind: `md:` 或 CSS Media Query |

### 2.2 AppSidebar

**当前**：`lib/presentation/widgets/app_sidebar.dart`

**子组件**：
- `SidebarHeader` — 账户信息 + 账户切换按钮
- `PageTreeTile` — 可展开/折叠的页面树节点
- `NavTile` — 导航项（图标 + 标签 + 选中状态）
- `AddPageInput` — 内联添加新页面输入框

**数据结构**：
```typescript
interface SidebarProps {
  pages: PageNode[];
  selectedPageId: string | null;
  onSelectPage: (id: string) => void;
  onAddPage: (name: string, parentId?: string) => void;
  onReorderPages: (orderedIds: string[]) => void;
}

interface PageNode {
  id: string;
  name: string;
  icon: string;
  children?: PageNode[];
  isExpanded?: boolean;
}
```

---

## L5.3 数据展示组件

### 3.1 ObjectCard（对象卡片）

**当前**：`lib/presentation/widgets/object_card/`（8 个子组件）

**子组件清单**：
- `ObjectCardHeader` — 图标 + 名称 + 操作菜单
- `ObjectCardEditField` — 内联字段编辑
- `ObjectCardEditModeWidget` — 编辑模式容器
- `ObjectCardItemTileWidget` — 列表项瓷砖
- `ObjectCardNewItemForm` — 添加新项表单
- `ObjectCardPropertiesList` — 属性列表展示
- `ObjectCardHistorySection` — 字段历史展开区

**接口设计**：
```typescript
interface ObjectCardProps {
  object: UnifiedObject;
  isEditing: boolean;
  onEdit: () => void;
  onSave: (updates: Partial<UnifiedObject>) => void;
  onDelete: () => void;
  onAddAttachment: (file: File) => void;
  onViewHistory: (fieldKey: string) => void;
}
```

### 3.2 ObjectTile（对象列表项）

```typescript
interface ObjectTileProps {
  object: UnifiedObject;
  isSelected: boolean;
  onTap: () => void;
  onLongPress?: () => void;
  trailing?: React.ReactNode;
}
```

### 3.3 SectionCard（分区卡片）

```typescript
interface SectionCardProps {
  title: string;
  icon: string;
  children: React.ReactNode;
  actions?: React.ReactNode;
  isCollapsible?: boolean;
}
```

### 3.4 其他数据展示组件

| 组件 | 功能 | 复杂度 |
|------|------|--------|
| `EntryCardWidget` | 条目卡片（列表展示） | 中 |
| `UniversalEntryCard` | 通用条目卡片 | 中 |
| `DynamicSectionCard` | 动态分区（按类型渲染） | 高 |
| `SectionRendererRegistry` | 分区渲染器注册表 | 高 |
| `PredefinedObjectSection` | 预置对象分区 | 低 |

---

## L5.4 敏感数据组件

> **核心规则**：所有敏感数据展示必须通过以下专用组件，禁止在各页面自行实现掩码逻辑。

### 4.1 SensitivityTag（敏感度标签）

```typescript
interface SensitivityTagProps {
  level: SensitivityLevel;
  size?: 'small' | 'medium';
}

// 颜色映射
const levelColors = {
  public:      { bg: 'transparent', text: '#6b7280', border: '#6b7280' },
  internal:    { bg: '#f3f4f6', text: '#4b5563', border: '#d1d5db' },
  private:     { bg: '#fef3c7', text: '#92400e', border: '#fbbf24' },
  sensitive:   { bg: '#fee2e2', text: '#991b1b', border: '#f87171' },
  restricted:  { bg: '#fce7f3', text: '#9d174d', border: '#f472b6' },
  critical:    { bg: '#f3e8ff', text: '#6b21a8', border: '#c084fc' },
};
```

### 4.2 SensitiveValueWidget（敏感值展示）

```typescript
interface SensitiveValueWidgetProps {
  value: string;
  level: SensitivityLevel;
  isRevealed: boolean;      // 是否已揭示
  onReveal: () => void;     // 点击揭示
  onHide: () => void;       // 点击隐藏
}

// 行为：
// - public / internal: 直接展示明文
// - private: 默认掩码（•••），点击揭示 5 秒后自动隐藏
// - sensitive / restricted / critical: 必须验证密码后才能揭示
```

### 4.3 SensitivityBlurredWidget（模糊遮罩）

```typescript
interface SensitivityBlurredWidgetProps {
  children: React.ReactNode;
  level: SensitivityLevel;
  isAuthorized: boolean;
}

// CSS 实现
.blur-mask {
  filter: blur(8px);
  user-select: none;
  transition: filter 0.3s ease;
}
.blur-mask.revealed {
  filter: none;
}
```

---

## L5.5 表单与输入组件

### 5.1 IconPicker（图标选择器）

```typescript
interface IconPickerProps {
  selectedIcon: string | null;
  onSelect: (iconName: string) => void;
  categories?: IconCategory[];
}

// 分类：证件、财务、旅行、个人、工具、其他
// 每类一个图标网格
// 支持搜索过滤
```

### 5.2 SemanticTypePicker（语义类型选择器）

```typescript
interface SemanticTypePickerProps {
  selectedType: string | null;
  onSelect: (typeId: string) => void;
}

// 层级结构：
// person.{name, birth_date, gender, nationality}
// pet.{name, species, breed}
// document.{number, issue_date, expiry_date}
// ...
```

### 5.3 其他表单组件

| 组件 | 功能 | 特殊要求 |
|------|------|---------|
| `DatePickerFormField` | 日期选择 | 支持多种日期格式、本地化 |
| `CharacterCounter` | 字符计数 | 实时统计、超限提示 |
| `ResponsiveLabelField` | 标签-字段响应式布局 | 桌面端左右排列，移动端上下排列 |
| `FormFieldDef` | 动态表单字段 | 根据 PropertyDefinition 渲染对应控件 |
| `CategorizedIconGrid` | 分类图标网格 | 支持拖拽、搜索 |

---

## L5.6 对话框组件

### 6.1 PasswordVerificationDialog（密码验证对话框）

> **重要**：所有需要密码验证的场景必须统一使用此对话框，禁止复制对话框代码。

```typescript
interface PasswordVerificationDialogProps {
  title?: string;
  message?: string;
  onVerify: (password: string) => Promise<boolean>;
  onCancel: () => void;
}

// 行为：
// - 显示密码输入框（可显隐切换）
// - 验证中显示加载指示器
// - 错误时显示红色提示
// - 验证成功后自动关闭，返回 true
```

### 6.2 其他对话框

| 组件 | 功能 | 触发场景 |
|------|------|---------|
| `ChangePasswordDialog` | 修改密码 | 设置页 |
| `LockVaultDialog` | 锁定确认 | 菜单栏/设置 |
| `AddSectionDialog` | 添加分区 | 对象编辑器 |
| `FolderPickerDialog` | 文件夹选择 | 扫描配置 |
| `FieldHistoryDialog` | 字段历史 | 对象卡片 |
| `PluginConsentDialog` | 插件授权 | 插件执行 |
| `PluginDetailDialog` | 插件详情 | 插件市场 |
| `PluginAccessReviewDialog` | 权限审查 | 插件设置 |
| `ImportPreviewDialog` | 导入预览 | 导入流程 |
| `PptxPreviewDialog` | PPTX 预览 | 附件列表 |
| `LegalDocumentSheet` | 法律文档 | 首次使用 |
| `AttachmentListSheet` | 附件列表 | 对象编辑器 |

---

## L5.7 首页专用组件

### 7.1 PageEditor（页面编辑器）

```typescript
interface PageEditorProps {
  pageId: string | null;
  onClose: () => void;
}

// 功能：
// - 展示页面树（可拖拽排序）
// - 添加/删除/重命名页面
// - 拖拽调整层级（父子关系）
```

### 7.2 其他首页组件

| 组件 | 功能 |
|------|------|
| `QuickActionTile` | 快捷操作入口（可自定义） |
| `AddButton` | 圆形添加按钮（带动画） |
| `AddQuickActionDialog` | 添加快捷操作 |
| `SecurityItem` | 即将过期安全项提醒 |
| `DeleteBadge` | 删除标记（动画） |
| `DashedPlaceholder` | 虚线占位（引导添加） |

---

## L5.8 LLM 专用组件

### 8.1 LlmChatPanel（聊天面板）

```typescript
interface LlmChatPanelProps {
  messages: LlmMessage[];
  isLoading: boolean;
  onSendMessage: (content: string) => void;
  onStopGeneration: () => void;
}

// 功能：
// - 消息列表（自动滚动到底部）
// - 输入框（支持多行、快捷键发送）
// - 停止生成按钮（流式响应时显示）
// - Markdown 渲染（代码块高亮）
```

### 8.2 LlmChatBubble（聊天消息气泡）

```typescript
interface LlmChatBubbleProps {
  message: LlmMessage;
  isUser: boolean;
  isLoading?: boolean;
}

// 样式：
// - 用户消息：右侧，主色背景
// - AI 消息：左侧，玻璃质感背景
// - Markdown 渲染 + 代码复制按钮
```

### 8.3 ChatSessionSidebar（会话侧边栏）

```typescript
interface ChatSessionSidebarProps {
  sessions: ChatSession[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onNewSession: () => void;
  onDeleteSession: (id: string) => void;
  onRenameSession: (id: string, name: string) => void;
}
```

---

## L5.9 扫描专用组件

| 组件 | 功能 |
|------|------|
| `ScanDocumentButton` | 扫描文档按钮（带动画） |
| `ScanProgressBanner` | 扫描进度横幅（顶部固定） |
| `OcrScannerSheet` | OCR 扫描底部弹窗 |
| `OcrScannerResultCard` | OCR 结果卡片（字段+置信度） |
| `OcrScannerActionButton` | OCR 操作按钮（填入/忽略） |
| `OcrScannerLlmSection` | LLM 提取选项区域 |
| `ExtractedFieldsPreview` | 提取字段预览（可编辑） |
| `MrzPreviewCard` | MRZ 解析结果卡片 |

---

## L5.10 设置/数据管理专用组件

| 组件 | 功能 | 位置 |
|------|------|------|
| `SettingsTile` | 设置项瓷砖（标题+副标题+尾部控件） | `settings/settings_tile.dart` |
| `BiometricSettingsWidget` | 生物识别开关+状态 | `biometric_settings_widget.dart` |
| `VaultInfoCard` | Vault 统计信息卡片 | `data_management/vault_info_card.dart` |
| `BackupSection` / `RestoreSection` | 备份/恢复操作区 | `data_management/` |
| `BackupProgressIndicator` | 备份进度（环形+百分比） | `data_management/backup_progress_indicator.dart` |
| `TrashFilterSection` | 回收站过滤控件 | `trash/trash_filter_section.dart` |
| `UnifiedObjectTrashCard` | 回收站对象卡片（含恢复/删除按钮） | `trash/unified_object_trash_card.dart` |
| `OperationLogFilterSection` | 操作日志过滤（类型+日期） | `operation_log_filter_section.dart` |
| `OperationTile` | 单条操作日志展示 | `operation_tile.dart` |
| `SearchFilters` | 搜索过滤面板 | `search_filters.dart` |
| `SearchResultTile` | 搜索结果项 | `search_result_tile.dart` |
| `SearchEmptyState` | 搜索空状态 | `search_empty_state.dart` |
| `HeaderActionButtons` | 页面头部操作按钮组 | `header_action_buttons.dart` |

---

## Liquid Glass 复刻方案

### CSS 实现（基础版）

```css
/* 玻璃卡片 */
.glass-card {
  background: linear-gradient(
    135deg,
    rgba(255, 255, 255, 0.1) 0%,
    rgba(255, 255, 255, 0.05) 100%
  );
  backdrop-filter: blur(20px) saturate(180%);
  -webkit-backdrop-filter: blur(20px) saturate(180%);
  border-radius: 20px;
  border: 1px solid rgba(255, 255, 255, 0.18);
  box-shadow: 
    0 8px 32px 0 rgba(31, 38, 135, 0.15),
    inset 0 1px 0 rgba(255, 255, 255, 0.2);
}

/* 玻璃 AppBar */
.glass-appbar {
  background: rgba(255, 255, 255, 0.7);
  backdrop-filter: blur(20px);
  border-bottom: 1px solid rgba(0, 0, 0, 0.05);
}

/* 暗色模式 */
[data-theme="dark"] .glass-card {
  background: linear-gradient(
    135deg,
    rgba(0, 0, 0, 0.3) 0%,
    rgba(0, 0, 0, 0.2) 100%
  );
  border: 1px solid rgba(255, 255, 255, 0.1);
}
```

### WebGL Shader 实现（进阶版）

```glsl
// 物理折射模拟
// 需要 Three.js / WebGL 环境
// 参考 liquid_glass_widgets 的 shader 实现
```

### 组件库选型建议

| 方案 | 优点 | 缺点 |
|------|------|------|
| **纯 CSS** | 简单、性能好、无需额外依赖 | 效果有限，无物理折射 |
| **CSS + SVG 滤镜** | 效果稍好 | 性能一般，兼容性差 |
| **WebGL (Three.js)** | 效果最接近原生 | 复杂、性能开销大 |
| **使用现有玻璃 UI 库** | 如 `react-glassmorphism` | 定制性有限 |

**推荐**：先以 CSS `backdrop-filter` 实现基础效果，后续迭代加入 WebGL 增强。

---

## 从零开始实现顺序

1. **设计系统基础**
   - CSS 变量定义（颜色、间距、圆角、阴影）
   - 明暗主题切换
   - 玻璃质感基础类 `.glass`, `.glass-card`, `.glass-appbar`

2. **布局组件**
   - `ScaffoldWithSidebar`（响应式布局骨架）
   - `AppSidebar`（页面树 + 导航）

3. **敏感数据组件**（优先，安全相关）
   - `SensitivityTag`
   - `SensitiveValueWidget`
   - `SensitivityBlurredWidget`

4. **表单组件**
   - `IconPicker`
   - `SemanticTypePicker`
   - `DatePicker`
   - 动态表单字段渲染器

5. **数据展示组件**
   - `ObjectCard`（最复杂，拆分多个子组件）
   - `ObjectTile`
   - `SectionCard`

6. **对话框组件**
   - `PasswordVerificationDialog`（最高优先级）
   - 其他对话框

7. **页面专用组件**
   - 首页：`PageEditor`, `QuickActionTile`, `SecurityItem`
   - LLM：`LlmChatPanel`, `LlmChatBubble`, `ChatSessionSidebar`
   - 扫描：`OcrScannerSheet`, `ScanProgressBanner`
   - 设置：`SettingsTile`, `VaultInfoCard`

---

*文档版本：v1.0*  
*创建日期：2026-06-04*  
*对应层级：L5*
