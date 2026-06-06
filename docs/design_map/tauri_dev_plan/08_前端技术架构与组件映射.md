# 08 — 前端技术架构与组件映射

> **前置阅读**：`01_技术选型确认与架构决策.md`、`03_项目顶层结构规划.md`
> **Manifesto 对齐**：依赖最小化 | 最少惊喜
> **源文档**：`tauri_refactor/前端框架与UI设计系统.md`（技术栈 + 组件部分）

---

## 1. 技术栈确认

| 层 | 选型 | 版本 | 安装 |
|----|------|------|------|
| UI 框架 | React | 19.x | `npm install react react-dom` |
| 构建工具 | Vite | 6.x | `npm install -D vite @vitejs/plugin-react` |
| 语言 | TypeScript | 5.7.x | `npm install -D typescript` |
| 样式 | CSS Modules + 全局 CSS | — | 无需安装 |
| 状态管理 | Zustand | 5.x | `npm install zustand` |
| 路由 | React Router | 7.x | `npm install react-router-dom` |
| 图标 | Lucide React | latest | `npm install lucide-react` |
| 动画 | Framer Motion | 11.x | `npm install framer-motion` |
| 表单 | React Hook Form + Zod | 7.x / 3.x | `npm install react-hook-form zod @hookform/resolvers` |
| 日期 | date-fns | latest | `npm install date-fns` |
| 国际化 | i18next + react-i18next | latest | `npm install i18next react-i18next i18next-browser-languagedetector` |

### [错误] 明确不使用的工具

| 工具 | 不使用原因 |
|------|-----------|
| Tailwind CSS | 团队无经验；与 Liquid Glass 复杂效果不兼容 |
| styled-components/emotion | 运行时开销；调试困难 |
| Redux | 过于冗长；Zustand 更轻量（详见文档 10） |
| Next.js | Tauri 不需要 SSR |
| shadcn/ui | 样式与 Liquid Glass 不匹配 |

---

## 2. Flutter Widget → React Component 映射

### 2.1 基础组件

| Flutter Widget | React 实现 | 方式 |
|---------------|-----------|------|
| `Container` | `<div>` | CSS |
| `Column` | `display: flex; flex-direction: column` | CSS |
| `Row` | `display: flex; flex-direction: row` | CSS |
| `Stack` | `position: relative/absolute` | CSS |
| `ListView` | `<div>` + `overflow: auto` | CSS |
| `GridView` | `display: grid` | CSS |
| `Expanded` | `flex: 1` | CSS |
| `TextField` | `<input>` / `<textarea>` | 原生 + CSS |
| `TextFormField` | React Hook Form + `<input>` | 库 + 原生 |
| `Checkbox` | `<input type="checkbox">` + CSS | 原生 |
| `Switch` | `<input type="checkbox">` + CSS 切换样式 | 原生 |
| `Dialog` | 自定义 Dialog 组件 | 自定义 + Framer Motion |
| `BottomSheet` | 自定义 Sheet 组件 | 自定义 + Framer Motion spring |
| `SnackBar` | Toast 组件 | 自定义或 sonner |
| `CircularProgressIndicator` | SVG / CSS animation | 自定义 |
| `Icon` | `<svg>` / Lucide React | Lucide |

### 2.2 SoloSoul 特定组件

| Flutter Widget | React Component |
|---------------|----------------|
| `LiquidGlassCard` | `GlassCard`（CSS backdrop-filter） |
| `SensitiveValueWidget` | `SensitiveValue`（状态控制可见性） |
| `SectionCard` | `SectionCard` |
| `PasswordVerificationDialog` | `PasswordVerificationDialog` |
| `SecurePasswordInput` | `SecurePasswordInput`（含揭示按钮 + 提示按钮，详见文档 09 第 8 节） |
| `FieldEditorSheet` | `FieldEditorSheet` |
| `OperationLogEntryTile` | `OperationLogEntryTile` |
| `TrashItemCard` | `TrashItemCard` |
| `DiscoveredDeviceCard` | `DiscoveredDeviceCard` |

---

## 3. 路由系统

### 3.1 路由表

```tsx
// src/App.tsx
<Routes>
  {/* 引导与认证 */}
  <Route path="/bootstrap" element={<BootstrapPage />} />
  <Route path="/login" element={<LoginPage />} />

  {/* 受保护路由 */}
  <Route element={<ProtectedRoute />}>
    <Route path="/" element={<HomePage />} />
    <Route path="/workspace/:categoryId?" element={<ObjectWorkspacePage />} />
    <Route path="/editor/:objectId?" element={<ObjectEditorPage />} />
    <Route path="/search" element={<SearchPage />} />
    <Route path="/settings" element={<SettingsPage />} />
    <Route path="/settings/security" element={<SecuritySettingsPage />} />
    <Route path="/settings/sensitivity" element={<SensitivitySettingsPage />} />
    <Route path="/settings/data" element={<DataManagementPage />} />
    <Route path="/settings/export-import" element={<ExportImportPage />} />
    <Route path="/settings/trash" element={<TrashPage />} />
    <Route path="/plugins" element={<PluginDashboardPage />} />
    <Route path="/llm-chat" element={<LlmChatPage />} />
    <Route path="/sync" element={<SyncPage />} />
    <Route path="/about" element={<AboutPage />} />
  </Route>

  <Route path="*" element={<Navigate to="/" replace />} />
</Routes>
```

### 3.2 路由参数

```tsx
// Flutter: Navigator.pushNamed(context, '/editor', arguments: {'objectId': 'xxx'})
// React:  <Link to="/editor/xxx" />  或  navigate('/editor/xxx')

// 获取参数
const { objectId } = useParams<{ objectId: string }>();
// objectId 为 undefined → 新建对象；有值 → 编辑对象
```

---

## 4. 表单系统

```tsx
// Flutter Form → React Hook Form + Zod
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
  const { register, handleSubmit, formState: { errors } } = useForm<FormData>({
    resolver: zodResolver(schema),
  });
  // ...
}
```

---

## 5. 防抖保存

```typescript
// src/hooks/useDebouncedSave.ts
export function useDebouncedSave<T>(
  saveFn: (data: T) => Promise<void>,
  delay: number = 500
) {
  const timeoutRef = useRef<ReturnType<typeof setTimeout>>();

  const debouncedSave = useCallback((data: T) => {
    if (timeoutRef.current) clearTimeout(timeoutRef.current);
    timeoutRef.current = setTimeout(() => saveFn(data), delay);
  }, [saveFn, delay]);

  // 组件卸载时立即保存
  useEffect(() => () => { if (timeoutRef.current) clearTimeout(timeoutRef.current); }, []);

  return { debouncedSave };
}
```

---

## 6. 从零实现顺序

| 阶段 | 内容 | 工作量 |
|------|------|--------|
| P0 | Vite + React + TS 项目搭建；Design Tokens；基础组件（Button, Input, Card） | 1-2 天 |
| P0 | 布局组件（AppShell, AppBar, SideNavigation）+ 路由 | 1 天 |
| P1 | Liquid Glass 组件（GlassCard, GlassPanel） | 1 天 |
| P1 | 表单系统 + Zod 校验 + 防抖保存 | 0.5 天 |
| P1 | 敏感遮罩组件（SensitiveValue） | 0.5 天 |
| P2 | Framer Motion 页面过渡 + 微交互 | 1 天 |

---

## 7. 完成标准

- [ ] `npm run dev` 可启动并显示完整路由页面
- [ ] 所有 Flutter 基础 Widget 有 React 对应实现
- [ ] 路由包含全部页面，支持参数传递
- [ ] 表单支持 Zod 校验 + 防抖保存
- [ ] 组件测试覆盖 Button, Input, Card, Dialog
- [ ] `SecurePasswordInput` 实现揭示按钮与提示按钮，失焦自动遮蔽
- [ ] `SideNavigation` 实现上下分区（Primary/Secondary）与 Warp 风格悬停名称卡片

---

*文档版本：v1.2*
*创建日期：2026-06-05*
*对应开发阶段：Phase 2（前端基础）*
