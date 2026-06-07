# 18 — 帮助文档系统重构

> **前置阅读**：`08_前端技术架构与组件映射.md`
> **Manifesto 对齐**：本地优先 | 最少惊喜
> **源文档**：`tauri_refactor/帮助文档系统重构设计.md`

---

## 1. 核心决策

**保留 Markdown 源文件，用 React 组件增强渲染。**

```
源文件（维护层）                    渲染层（展示层）
assets/docs/guides/                  src/components/guide/
├── index.json    ───────────────→   GuideIndex.tsx（指南目录）
├── objects.md    ───────────────→   GuideRenderer.tsx（富交互渲染）
├── security.md   ───────────────→   GuideSearch.tsx（全文搜索）
└── ...                              GuideCodeBlock.tsx（代码复制）
        ↑
        └──────────────────────→   AI 检索（纯文本提取，注入 LLM 上下文）
```

### 为什么不选其他方案？

| 方案 | 排除理由 |
|------|---------|
| 纯 Markdown 渲染（现有） | 无全文搜索、无交互、样式天花板低 |
| 嵌入 WebView 外部站点 | 违背本地优先（需要网络） |
| 富文本编辑器 | 多人协作不需要，维护成本高 |
| PDF 内嵌 | 不可搜索、不可交互、更新不便 |

---

## 2. 文档源文件结构

```
assets/docs/guides/
├── index.json              # 指南索引（ID、标题、关键词、文件路径）
├── objects.md / objects_zh.md
├── security.md / security_zh.md
├── sync.md / sync_zh.md
├── export_import.md / export_import_zh.md
├── plugins.md / plugins_zh.md
├── trash.md / trash_zh.md
├── ai_chat.md / ai_chat_zh.md
├── PRIVACY_POLICY.md / PRIVACY_POLICY_zh.md
└── TERMS_OF_SERVICE.md / TERMS_OF_SERVICE_zh.md
```

### index.json 格式

```json
{
  "guides": [
    {
      "id": "objects",
      "title": { "zh": "对象管理", "en": "Object Management" },
      "keywords": ["对象", "创建", "编辑", "属性"],
      "files": { "zh": "objects_zh.md", "en": "objects.md" }
    }
  ]
}
```

---

## 3. React 渲染组件

| 组件 | 功能 |
|------|------|
| `GuideIndex` | 指南目录、分类浏览 |
| `GuideRenderer` | Markdown → React 渲染（使用 react-markdown） |
| `GuideSearch` | 全文搜索、关键词高亮 |
| `GuideStepper` | 操作步骤指示器（1→2→3） |
| `GuideCodeBlock` | 代码块 + 一键复制按钮 |

### GuideRenderer 增强能力

```tsx
// 标准 Markdown 元素 → 富交互组件
{
  code: GuideCodeBlock,      // 代码块 → 带复制按钮
  blockquote: GuideTip,      // 引用块 → 提示框（info/warning/tip）
  img: GuideImage,           // 图片 → 点击放大
  table: GuideTable,         // 表格 → 响应式滚动
}
```

### 自定义容器（Markdown 扩展语法）

```markdown
::: stepper 创建护照对象
1. 点击首页的「+ 新建对象」按钮
2. 选择对象类型「护照」
3. 填写护照号码、签发日期等信息
4. 点击「保存」
:::

::: tip
护照号码属于敏感数据，查看时需要进行身份验证。
:::

::: warning
删除对象后可在回收站 30 天内恢复，过期将永久删除。
:::
```

---

## 4. 全文搜索

```typescript
// src/lib/guide-search.ts
export function searchGuides(query: string, locale: string): SearchResult[] {
  // 1. 加载所有指南内容
  // 2. 分词（中文用结巴分词或简单二元分词）
  // 3. 计算 TF-IDF 相关性
  // 4. 返回匹配段落 + 高亮位置
}
```

---

## 5. AI 检索兼容

帮助文档作为 AI 对话的上下文注入：

```rust
// Rust 端
pub fn retrieve_relevant_guide(query: &str, locale: &str) -> Option<String> {
    // 1. 对 query 分词
    // 2. 每个指南得分 = 关键词命中 + 标题命中权重(+3)
    // 3. 返回得分最高的 1 篇
    // 4. 截断至 800 字符（控制 token）
}
```

---

## 6. 从零实现顺序

1. 创建 `assets/docs/guides/` 目录 + 编写 Markdown 源文件
2. 实现 `GuideRenderer`（react-markdown + 自定义组件）
3. 实现 `GuideIndex` + `GuideSearch`
4. 实现自定义容器渲染（stepper、tip、warning）
5. 集成 AI 检索（注入 LLM 上下文）

---

## 7. 完成标准

- [ ] 所有指南 Markdown 源文件编写完成（中英双语）
- [ ] GuideRenderer 正确渲染 Markdown + 自定义容器
- [ ] 全文搜索返回正确结果并高亮匹配
- [ ] 代码块支持一键复制
- [ ] AI 检索返回相关指南（截断 ≤ 800 字符）
- [ ] 帮助文档纯本地可用（无网络依赖）

---

*文档版本：v1.0*
*创建日期：2026-06-05*
*对应开发阶段：Phase 5（帮助文档）*
