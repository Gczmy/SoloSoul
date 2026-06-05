# J5 AI 与自动化旅程

> **旅程定位**：用户利用 AI 能力自动化处理数据的完整流程，包括 OCR 识别文档、LLM 智能对话、插件扩展、本地文件扫描导入。这是 SoloSoul 的"智能化"体现，让数据录入从手动输入转向自动提取。
>
> **核心目标**：减少用户手动输入的工作量，通过 OCR + LLM 从文档/图片中自动提取结构化数据；通过插件扩展应用能力；通过本地扫描发现分散在各处的个人信息。
>
> **涉及页面**：OCR Scanner → LLM Chat → Plugin Dashboard → Scan Flow
>
> **状态依赖**：L3 OCR 服务、L3 LLM 服务、L3 插件服务、L3 扫描服务、L4 对应状态

---

## 目录

- [旅程流程图](#旅程流程图)
- [模块 1：OCR 扫描与文档提取](#模块-1ocr-扫描与文档提取)
- [模块 2：LLM 智能对话](#模块-2llm-智能对话)
- [模块 3：插件市场与执行](#模块-3插件市场与执行)
- [模块 4：本地文件扫描与导入](#模块-4本地文件扫描与导入)
- [AI 映射流程详解](#ai-映射流程详解)
- [异常处理](#异常处理)
- [从零实现顺序](#从零实现顺序)

---

## 旅程流程图

```
┌─────────────────┐
│   用户场景      │
└────────┬────────┘
         │
    ┌────┴────┬────────────┬────────────┐
    ▼         ▼            ▼            ▼
┌───────┐ ┌────────┐ ┌──────────┐ ┌──────────┐
│ OCR   │ │ LLM    │ │ 插件     │ │ 本地扫描  │
│ 扫描  │ │ 对话   │ │ 市场     │ │ 导入     │
└───┬───┘ └───┬────┘ └────┬─────┘ └────┬─────┘
    │         │           │            │
    ▼         ▼           ▼            ▼
┌─────────────────────────────────────────────┐
│            自动填入 ObjectEditor             │
│         或直接创建 UnifiedObject             │
└─────────────────────────────────────────────┘
```

**典型用户路径**：
1. **拍照录入护照**：对象编辑器 → OCR 扫描 → 拍摄护照 → 自动提取字段 → 确认填入
2. **AI 问答**：LLM Chat → "我的护照什么时候过期？" → AI 查询数据 → 回答
3. **安装插件**：Plugin Dashboard → 安装"税务计算器" → 运行 → 授权字段 → 获取结果
4. **扫描本地文件**：Scan Config → 扫描文档文件夹 → AI 映射到字段 → 批量导入

---

## 模块 1：OCR 扫描与文档提取

### 1.1 入口点

OCR 扫描有两个入口：
- **对象编辑器内**：编辑对象时点击 `🔍 OCR 扫描` 按钮
- **独立 OCR 页面**：Home → OCR（如有独立入口）

### 1.2 扫描流程（对象编辑器内）

```
用户在 ObjectEditor 中点击 "OCR 扫描"
    │
    ▼
打开文件选择器 / 相机
    │
    ▼
选择图片后显示 OcrScannerSheet（底部弹窗）
    │
    ├──→ 显示识别进度
    ├──→ 调用 Rust OCR 引擎
    │       │
    │       ├──→ MRZ 识别（护照/身份证）
    │       │       └──→ 解析 TD1/TD2/TD3 格式
    │       │
    │       └──→ 通用 OCR（文本块）
    │
    ▼
显示识别结果
    │
    ├──→ 简单字段：直接映射到对象属性
    │
    └──→ 复杂字段：显示 [使用 AI 智能提取] 按钮
            │
            └──→ 调用 LLM 分析文本内容
                    │
                    └──→ 返回结构化字段映射
    │
    ▼
用户确认字段映射
    │
    ▼
自动填入 ObjectEditor 的对应字段
```

### 1.3 OcrScannerSheet UI

```
┌─────────────────────────────────────────┐
│  ┌─ OcrScannerSheet ─────────────────┐  │
│  │                                    │  │
│  │  [护照图片预览]                     │  │
│  │                                    │  │
│  │  ── 识别结果 ─────────────────────  │  │
│  │                                    │  │
│  │  MRZ 识别:                          │  │
│  │  姓名: ZHANG SAN                    │  │
│  │  护照号: E12345678                  │  │
│  │  国籍: CHN                          │  │
│  │  出生日期: 1990-01-01               │  │
│  │  有效期: 2025-06-01                 │  │
│  │                                    │  │
│  │  置信度: 95% ✅                     │  │
│  │                                    │  │
│  │  [使用 AI 智能提取]                 │  │
│  │                                    │  │
│  │  ── AI 提取结果 ──────────────────  │  │
│  │  签发地: 北京                       │  │
│  │  签发日期: 2015-06-01               │  │
│  │                                    │  │
│  │  [✓ 全选] [填入表单] [取消]        │  │
│  │                                    │  │
│  └────────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

### 1.4 MRZ 解析

**支持格式**：
- TD1：身份证（3 行，每行 30 字符）
- TD2：旧版护照（2 行，每行 36 字符）
- TD3：ICAO 护照（2 行，每行 44 字符）

**解析字段**：
```typescript
interface MrzResult {
  documentType: string;      // P (护照), I (身份证), etc.
  documentNumber: string;
  nationality: string;       // ISO 三位代码
  dateOfBirth: string;       // YYMMDD
  gender: 'M' | 'F' | 'X';
  expiryDate: string;        // YYMMDD
  surname: string;
  names: string;
  optionalData?: string;
  compositeCheckDigit: string;
}
```

### 1.5 OCR 引擎状态

```typescript
interface OcrEngineStatus {
  initialized: boolean;
  detLoaded: boolean;    // 检测模型
  clsLoaded: boolean;    // 分类模型
  recLoaded: boolean;    // 识别模型
}
```

**初始化失败处理**：
- 显示"OCR 引擎初始化失败"
- 提供"重试"按钮
- 引导检查模型文件是否存在

---

## 模块 2：LLM 智能对话

### 2.1 页面：LlmChatPage

**职责**：独立的 AI 对话界面，支持多会话管理。

### 2.2 布局（宽屏 > 800px）

```
┌──────────┬──────────────────────────────────────┐
│          │  SoloGlassAppBar [LLM | 设置]        │
│ Sessions │──────────────────────────────────────│
│          │                                      │
│ [新会话+] │  🤖 AI: 你好！我是你的 AI 助手。     │
│          │      有什么我可以帮你的吗？           │
│ 会话 1   │                                      │
│ 会话 2   │  👤 用户: 我的护照什么时候过期？      │
│ 会话 3   │                                      │
│          │  🤖 AI: 根据您的数据，您的中国护照    │
│          │      (E12345678) 将于 2025-06-01    │
│          │      过期，距离现在还有 361 天。      │
│          │                                      │
│          │──────────────────────────────────────│
│          │  [输入消息...]              [发送]   │
└──────────┴──────────────────────────────────────┘
```

### 2.3 会话管理

**侧边栏功能**：
- 新建会话
- 删除会话
- 重命名会话
- 会话列表（按时间倒序）

### 2.4 消息类型

```typescript
interface LlmMessage {
  id: string;
  role: 'system' | 'user' | 'assistant';
  content: string;
  timestamp: Date;
  tokens?: number;        // 消耗的 token 数
}
```

### 2.5 上下文注入

**系统提示词模板**：
```
你是 SoloSoul 的智能助手，帮助用户管理个人数据。

用户当前数据概览：
- 身份: {profile.identity.fullName}
- 护照: {profile.travel.passport.number}，有效期至 {profile.travel.passport.expiryDate}
- 最近旅行: {profile.travel.history[0].destination}

注意：
1. 只在用户询问时引用其数据
2. 不要主动透露敏感信息
3. 如果用户请求修改数据，引导他们使用编辑器
```

### 2.6 后端支持

| 后端 | 连接方式 | 状态检测 |
|------|---------|---------|
| **OpenAI** | HTTP API | 直接调用 |
| **Anthropic** | HTTP API | 直接调用 |
| **Google** | HTTP API | 直接调用 |
| **Ollama** | localhost:11434 | `GET /api/tags` |

### 2.7 流式响应

```typescript
async function* streamMessage(messages: LlmMessage[]): AsyncGenerator<LlmChunk> {
  const response = await fetch('/api/chat', {
    method: 'POST',
    body: JSON.stringify({ messages, stream: true }),
  });
  
  const reader = response.body!.getReader();
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    yield parseChunk(value);
  }
}
```

### 2.8 用量统计

**统计维度**：
- 按模型（GPT-4 / Claude / Llama3 等）
- 按时间（日/周/月）
- 按会话

**统计页面**：`LlmStatsPage`

```
┌─────────────────────────────────────────┐
│  用量统计                                │
├─────────────────────────────────────────┤
│  ┌─ 今日 ─────┐  ┌─ 本周 ─────┐        │
│  │ 15,234    │  │ 89,456    │        │
│  │ tokens    │  │ tokens    │        │
│  └───────────┘  └───────────┘        │
│                                         │
│  [Token 使用趋势 Sparkline]             │
│                                         │
│  [模型使用饼图]                         │
│  GPT-4: 60%  Claude: 30%  Llama3: 10% │
│                                         │
│  [费用估算]                             │
│  本月预估: $12.50                       │
│                                         │
└─────────────────────────────────────────┘
```

---

## 模块 3：插件市场与执行

### 3.1 页面：PluginDashboardPage

**职责**：管理插件生命周期（安装/卸载/更新/运行）。

### 3.2 插件市场

```
┌─────────────────────────────────────────┐
│  SoloGlassAppBar [插件市场 | 刷新]       │
├─────────────────────────────────────────┤
│  [全部 ▼] [已安装 ▼] [可更新 ▼]        │  ← TabBar
├─────────────────────────────────────────┤
│  [🔍 搜索插件...]                       │
├─────────────────────────────────────────┤
│                                         │
│  ┌─ PluginCard 1 ───────────────────┐  │
│  │ [插件图标]                        │  │
│  │ 名称: Passport Extractor          │  │
│  │ 版本: 1.0.0                       │  │
│  │ 描述: 从护照图片提取结构化字段      │  │
│  │ 发布者: SoloSoul Team              │  │
│  │ 权限: 需要访问 passport.* 字段     │  │
│  │ [安装]                            │  │
│  └───────────────────────────────────┘  │
│                                         │
│  ┌─ PluginCard 2 ───────────────────┐  │
│  │ [插件图标]                        │  │
│  │ 名称: Tax Calculator              │  │
│  │ 版本: 2.1.0 → 2.2.0 (可更新)      │  │
│  │ [运行 ▼] [卸载] [更新]            │  │
│  └───────────────────────────────────┘  │
│                                         │
└─────────────────────────────────────────┘
```

### 3.3 插件执行流程

```
用户点击"运行"插件
    │
    ▼
调用 PluginService.runPlugin(pluginId, params)
    │
    ▼
Rust 端启动 Wasmtime 沙盒
    │
    ├──→ 加载插件 .wasm 文件
    ├──→ 验证 manifest 签名
    └──→ 初始化 Session
    │
    ▼
插件请求访问字段 → 发送 ConsentRequest 事件
    │
    ▼
前端显示 PluginConsentDialog
    │
    ├──→ 用户拒绝 → 终止插件，返回错误
    └──→ 用户授权 → 发送 approvedFields
        │
        ▼
    插件通过 Host Functions 访问授权字段
        │
        ├──→ 读取数据
        ├──→ 执行计算
        └──→ 返回结果
        │
        ▼
    前端接收 Completed 事件
        │
        └──→ 显示结果（结构化数据或文本）
```

### 3.4 字段授权对话框

```
┌─────────────────────────────────────────┐
│  🔌 Passport Extractor 请求访问          │
├─────────────────────────────────────────┤
│                                         │
│  此插件需要访问以下字段：                │
│                                         │
│  [☑️] passport.number                   │
│  [☑️] passport.expiryDate               │
│  [☑️] passport.nationality              │
│  [ ]  passport.issueDate    （可选）     │
│                                         │
│  授权有效期: 24 小时                    │
│                                         │
│  [拒绝]        [授权所选字段]            │
│                                         │
└─────────────────────────────────────────┘
```

### 3.5 插件限制

| 限制 | 说明 |
|------|------|
| **iOS 不支持** | Wasmtime JIT 在 iOS 上被禁止 |
| **沙盒** | 无法直接访问文件系统、网络、系统 API |
| **字段级授权** | 只能访问用户显式授权的字段 |
| **Session TTL** | 超时后自动销毁，数据擦除 |
| **签名验证** | 未签名插件显示警告 |

---

## 模块 4：本地文件扫描与导入

### 4.1 扫描流程

```
用户进入 Scan Config 页面
    │
    ▼
配置扫描参数（路径、深度、文件类型）
    │
    ▼
点击"开始扫描"
    │
    ▼
显示 Scan Progress 页面
    │
    ├──→ 文件列举
    ├──→ 内容解析
    ├──→ 分区检测
    └──→ AI 映射（可选）
    │
    ▼
显示 Scan Preview 页面
    │
    ├──→ 展示候选列表
    ├──→ 用户选择要导入的项
    └──→ 解决冲突
    │
    ▼
点击"导入"
    │
    ▼
显示导入进度 → Scan Import Result 页面
```

### 4.2 扫描配置页面

```
┌─────────────────────────────────────────┐
│  SoloGlassAppBar [← | 扫描配置]          │
├─────────────────────────────────────────┤
│                                         │
│  ── 扫描路径 ─────────────────────────  │
│  ☑️ 桌面                               │
│  ☑️ 文档                               │
│  ☑️ 下载                               │
│  ☐  图片                               │
│  [+ 添加自定义路径]                     │
│                                         │
│  ── 扫描深度 ─────────────────────────  │
│  (•) 文件名匹配（最快）                 │
│  ( ) 文件名 + 内容指纹                  │
│  ( ) 全文解析（最慢，最准确）            │
│                                         │
│  ── 文件类型 ─────────────────────────  │
│  ☑️ PDF  ☑️ Word  ☑️ Excel            │
│  ☑️ PPT  ☑️ 图片  ☑️ 文本             │
│                                         │
│  ── 高级 ─────────────────────────────  │
│  最大文件大小: [50 MB ▼]                │
│  最大文件数:   [10000 ▼]                │
│                                         │
│  [开始扫描]                             │
│                                         │
└─────────────────────────────────────────┘
```

### 4.3 扫描进度页面

```
┌─────────────────────────────────────────┐
│  SoloGlassAppBar [← | 扫描中...]         │
├─────────────────────────────────────────┤
│                                         │
│           [环形进度条]                   │
│              67%                        │
│                                         │
│  已扫描: 8,234 / 12,345                │
│  发现:   56 个候选                      │
│  跳过:   7,890 个（已扫描/无变更）      │
│                                         │
│  当前文件:                              │
│  /Users/xxx/Documents/tax_2025.pdf     │
│                                         │
│  [取消扫描]                             │
│                                         │
└─────────────────────────────────────────┘
```

### 4.4 扫描预览页面

```
┌─────────────────────────────────────────┐
│  SoloGlassAppBar [← | 扫描结果 | AI映射] │
├─────────────────────────────────────────┤
│                                         │
│  [☑️ 全选]  已选择: 23/56               │
│                                         │
│  ┌─ Candidate 1 ───────────────────┐   │
│  │ ☑️  passport_scan.pdf            │   │
│  │    检测分区: Passport            │   │
│  │    置信度: 95%                   │   │
│  │    映射字段:                     │   │
│  │      姓名 → identity.fullName    │   │
│  │      护照号 → passport.number    │   │
│  │    [编辑映射]                    │   │
│  └──────────────────────────────────┘   │
│                                         │
│  ┌─ Candidate 2 ───────────────────┐   │
│  │ ☑️  bank_statement.pdf           │   │
│  │    检测分区: Bank Account        │   │
│  │    置信度: 78%                   │   │
│  │    ⚠️  低置信度，请核对          │   │
│  └──────────────────────────────────┘   │
│                                         │
│  [冲突: 3 项] [查看并解决]              │
│                                         │
│  ┌─────────────────────────────────┐   │
│  │ [导入选中项] (23 项)             │   │
│  └─────────────────────────────────┘   │
│                                         │
└─────────────────────────────────────────┘
```

---

## AI 映射流程详解

### 5.1 触发条件

用户在 Scan Preview 页面点击 **"AI 智能映射"** 按钮。

### 5.2 映射流程

```
用户点击 AI 映射
    │
    ▼
检查 LLM 配置
    │
    ├──→ 未配置 → 显示引导："请先配置 LLM" → 跳转 LlmConfigPage
    └──→ 已配置 → 继续
    │
    ▼
对每个候选文件：
    │
    ├──→ 1. 提取文件文本内容
    │       ├──→ PDF: pdf-parser
    │       ├──→ Word/Excel: xml-parser
    │       └──→ 图片: OCR 识别
    │
    ├──→ 2. 构建 LLM Prompt
    │       ```
    │       请分析以下文档内容，将其映射到 SoloSoul 的字段结构。
    │       
    │       文档内容：
    │       {content}
    │       
    │       检测到的分区类型: {detectedSection}
    │       
    │       请返回 JSON 格式：
    │       {
    │         "fields": [
    │           { "key": "字段名", "value": "值", "type": "text|date|number" }
    │         ],
    │         "confidence": 0.95
    │       }
    │       ```
    │
    ├──→ 3. 调用 LLM API
    │
    ├──→ 4. 解析 LLM 输出
    │       └──→ LlmFieldMappingParser.parse(response)
    │
    └──→ 5. 更新候选对象的 properties
    │
    ▼
显示映射完成提示
    │
    └──→ 用户确认后导入
```

### 5.3 Prompt 模板

```typescript
const SCAN_MAPPING_PROMPT = `
You are a data extraction assistant for SoloSoul, a personal data manager.

Task: Analyze the following document and extract structured fields.

Detected document type: {sectionType}
Document text:
---
{documentText}
---

Available field types: text, number, date, select, multiSelect, checkbox, url

Return ONLY a JSON object in this exact format:
{
  "fields": [
    {
      "key": "machine-readable-field-name",
      "value": "extracted value",
      "type": "text|number|date|...",
      "confidence": 0.95
    }
  ],
  "overallConfidence": 0.92,
  "warnings": ["optional warning messages"]
}

Rules:
- Use English camelCase for keys
- Dates in ISO format (YYYY-MM-DD)
- Numbers without formatting
- If a field is unclear, set confidence < 0.8
- If the document doesn't match the detected type, set overallConfidence < 0.5
`;
```

---

## 异常处理

| 异常场景 | 处理方式 |
|---------|---------|
| OCR 引擎未初始化 | 提示"OCR 引擎未就绪，请稍后再试"，后台自动重试初始化 |
| OCR 识别无文本 | 提示"未能识别到文本，请检查图片清晰度" |
| LLM API 调用失败 | 提示"AI 服务暂时不可用，请检查网络或配置" |
| LLM 返回格式错误 | 提示"AI 响应格式异常，请重试"，记录原始响应到日志 |
| 插件执行超时 | 自动终止 Session，提示"插件执行超时" |
| 插件请求未授权字段 | 显示 ConsentDialog，用户可自主选择授权 |
| 扫描过程中磁盘空间不足 | 暂停扫描，提示清理空间后继续 |
| 扫描发现大量文件 | 显示"发现大量文件，预计需要 X 分钟"，允许用户调整范围 |
| AI 映射低置信度 | 标记候选为"需核对"，不自动勾选 |

---

## 从零实现顺序

1. **OCR 引擎（Rust）**
   - ONNX 模型加载（det/cls/rec）
   - MRZ 流水线（TD1/TD2/TD3 解析）
   - 通用 OCR 流水线
   - 引擎状态查询

2. **OCR UI 组件**
   - OcrScannerSheet（底部弹窗）
   - OcrScannerResultCard（结果卡片）
   - 与 ObjectEditor 集成

3. **LLM 服务**
   - 统一接口（Cloud/Local）
   - 流式响应处理
   - 上下文注入
   - 用量统计

4. **LLM Chat 页面**
   - 会话侧边栏
   - 聊天面板（消息列表 + 输入框）
   - Markdown 渲染
   - 响应式布局

5. **LLM 配置页面**
   - 后端类型选择
   - API Key 管理
   - 模型选择
   - Ollama 状态检测

6. **插件系统（Rust）**
   - Wasmtime 沙盒
   - Manifest 解析
   - Host Functions
   - Session 管理
   - 字段授权

7. **Plugin Dashboard 页面**
   - 插件列表（全部/已安装/可更新）
   - 插件卡片
   - 安装/卸载/更新
   - 运行按钮

8. **扫描服务**
   - 文件列举
   - 内容解析
   - 分区检测
   - 缓存机制

9. **扫描流程页面**
   - Scan Config
   - Scan Progress
   - Scan Preview（含 AI 映射）
   - Scan Import Result

10. **AI 映射**
    - Prompt 模板
    - 字段映射解析器
    - 置信度评估
    - 低置信度标记

---

*文档版本：v1.0*  
*创建日期：2026-06-04*  
*对应旅程：J5（AI 与自动化）*
