# Tauri LLM 实现与文档规范差距分析报告

> **文档目的**：对比 `20_LLM配置与AI对话规范.md`（v3.0）与当前 Tauri 代码实现，识别所有不一致和未实现项，按优先级分类，为后续开发提供明确的任务清单。
>
> **分析范围**：`tauri/src-tauri/src/commands/llm.rs`、`tauri/src/pages/ai/*.tsx`、`tauri/src/lib/ipc.ts`、`tauri/src/locales/*` 及相关文件。
> **分析日期**：2026-06-08

---

## 执行摘要

| 维度 | 状态 |
|------|------|
| **已完全实现** | Provider 管理、配置存储、基础对话 UI、对话生命周期、回收站、风险告知 |
| **部分实现** | API 调用（非流式）、Anthropic 适配 |
| **完全未实现** | 系统提示词注入、上下文构建服务、帮助文档检索、流式响应、使用统计、隐私分级过滤 |
| **关键差距数** | **P0 差距 8 项**、P1 差距 5 项、P2 差距 3 项 |

---

## 1. 按文档章节逐项对比

### §1 设计原则

| 原则 | 实现状态 | 说明 |
|------|---------|------|
| 用户主权 | ✅ | Provider 可自由配置 |
| 默认禁用 | ✅ | `AiFeatures` 默认全 `false` |
| 不绑定厂商 | ✅ | 支持自定义 Provider + OpenAI/Anthropic 双协议 |
| 密钥安全 | ✅ | API Key 存储在 Vault 加密 Profile 中 |
| 透明可控 | ⚠️ | 风险告知已实现，但缺少"系统提示词仅包含公开信息"的说明 |
| **隐私优先** | ❌ | 系统提示词未实现，无从谈起隐私过滤 |
| **本地优先** | ❌ | 帮助文档检索未实现 |

---

### §2 Provider 模型

| 规范项 | 实现状态 | 代码位置 | 差距说明 |
|--------|---------|---------|---------|
| 预置 Provider（5 个） | ✅ | `llm.rs:94-101` | 与文档一致 |
| 自定义 Provider | ✅ | `LlmConfigPage.tsx:107-109` | 支持添加自定义 |
| Provider 数据结构 | ✅ | `llm.rs:27-41` | `ProviderConfig` / `ProviderWithKey` 与文档一致 |
| **Base URL 必须以 `/v1` 结尾** | ❌ | `LlmConfigPage.tsx` | 文档 §2.3 要求验证，前端/后端均无此验证 |
| 名称 1-30 字符验证 | ❌ | `LlmConfigPage.tsx` | 无长度限制验证 |
| 测试连接功能 | ✅ | `llm.rs:434-474` | `llm_test_provider` 已实现 |

**差距 1（P1）**：Base URL 格式验证缺失。文档要求必须以 `/v1` 结尾，当前前端允许输入任意 URL，后端直接拼接，可能导致请求失败。

---

### §3 配置存储

| 规范项 | 实现状态 | 代码位置 | 差距说明 |
|--------|---------|---------|---------|
| Provider 列表（不含 key） | ✅ | `llm.rs:104-129` | 存于 `preferences.llmConfig` |
| API 密钥独立存储 | ✅ | `llm.rs:131-154` | 存于 `preferences.llmApiKeys` |
| 活跃 Provider ID | ✅ | `llm.rs:54-59` | `active_provider_id` |
| AI 功能开关 | ✅ | `llm.rs:45-50` | `ai_features_enabled` |
| **系统提示词开关** | ❌ | `llm.rs:52-59` | `LlmConfig` **缺少** `include_system_prompt: bool` 字段 |
| 系统提示词缓存 | ❌ | — | 内存缓存未实现 |
| 使用统计存储 | ❌ | — | 无统计数据结构 |

**差距 2（P0）**：`LlmConfig` 缺少 `include_system_prompt` 字段。文档 §3.3 明确要求该字段（默认 `true`，高级用户可关闭），当前数据模型不支持。

---

### §4 UI 规范

| 规范项 | 实现状态 | 代码位置 | 差距说明 |
|--------|---------|---------|---------|
| 设置页 LLM 配置入口 | ✅ | `LlmConfigPage.tsx` | 已实现 |
| Provider 列表项（单选按钮） | ✅ | `LlmConfigPage.tsx:139-149` | 已实现 |
| 风险告知对话框 | ✅ | `LlmConfigPage.tsx:184-205` | 已实现 |
| **系统提示词开关区域** | ❌ | `LlmConfigPage.tsx` | 文档 §4.1 要求三个开关（注入软件信息/用户公开档案/帮助文档），UI 中完全缺失 |
| **风险告知补充说明** | ❌ | `locales/*/settings.json` | 文档 §4.3 要求增加"系统提示词仅包含您主动公开的信息"承诺，翻译文件中无此内容 |

**差距 3（P0）**：设置页缺少系统提示词注入开关。文档要求在 AI 功能开关下方增加"系统提示词（上下文注入）"区域，含三个复选框。

---

### §5 API 调用流程

| 规范项 | 实现状态 | 代码位置 | 差距说明 |
|--------|---------|---------|---------|
| 后端代理架构 | ✅ | `llm.rs:476-531` | 前端不直接接触外部 API |
| OpenAI 兼容格式 | ✅ | `llm.rs:517-518` | 标准格式 |
| Anthropic 适配层 | ✅ | `llm.rs:480-515` | 已分离 `system` 消息到顶层 `system` 字段 |
| **流式响应（stream=true）** | ❌ | `llm.rs:477,518` | 当前 `stream: false`，文档 §5.2 要求"始终启用流式响应" |
| **本地 Ollama SSE 流式** | ❌ | — | 未实现原生 SSE |
| **云端打字机效果 fallback** | ❌ | `LlmChatPage.tsx:209-272` | 当前是完整请求后一次性显示，没有逐字 emit |
| **系统提示词注入** | ❌ | `llm_send_message` | 后端直接转发 messages，未构建系统提示词 |
| **帮助文档检索注入** | ❌ | — | 未实现 |

**差距 4（P0）**：流式响应完全未实现。文档 §5.3 要求本地 Ollama 使用原生 SSE、云端使用打字机效果（逐字 emit，8ms/字），当前代码使用 `stream: false` 一次性返回完整响应。

**差距 5（P0）**：`llm_send_message` 未实现系统提示词构建和注入。当前直接将前端传入的 messages 转发给 LLM API，未经过系统提示词组装流程。

---

### §6 系统提示词与上下文注入（全新章节）

| 规范项 | 实现状态 | 差距说明 |
|--------|---------|---------|
| **7 Section 系统提示词模板** | ❌ | 完全未实现 |
| **LlmContextService** | ❌ | 完全未实现 |
| **隐私分级过滤（仅 public）** | ❌ | 完全未实现 |
| **缓存机制** | ❌ | 完全未实现 |
| **长度限制（2000 字符）** | ❌ | 完全未实现 |
| **Token 估算** | ❌ | 完全未实现 |
| **截断策略** | ❌ | 完全未实现 |
| **注入时序图流程** | ❌ | 完全未实现 |

**差距 6（P0）**：系统提示词与上下文注入整体缺失。这是文档 v3.0 新增的核心功能，当前代码没有任何相关实现。包含：
- 没有系统提示词模板构建逻辑
- 没有从 Profile/对象数据中提取 public 级别信息的逻辑
- 没有缓存服务
- 没有长度控制和截断

---

### §7 帮助文档检索与嵌入（全新章节）

| 规范项 | 实现状态 | 差距说明 |
|--------|---------|---------|
| **`resources/docs/guides/` 目录** | ❌ | 目录不存在 |
| **`index.json` 索引文件** | ❌ | 未创建 |
| **UserGuideService** | ❌ | 完全未实现 |
| **关键词匹配算法** | ❌ | 完全未实现 |
| **评分规则（标题+3/关键词+1）** | ❌ | 完全未实现 |
| **多语言回退** | ❌ | 完全未实现 |
| **注入格式** | ❌ | 完全未实现 |
| **内容截断（800 字符）** | ❌ | 完全未实现 |

**差距 7（P0）**：帮助文档检索与嵌入整体缺失。文档 v3.0 要求实现本地关键词匹配帮助文档检索，当前没有任何相关代码或资源文件。

---

### §9 AI 对话页面规范

#### 9.1-9.3 已实现的 UI 功能

| 功能 | 状态 | 说明 |
|------|------|------|
| 左右布局（侧边栏 + 消息区） | ✅ | `LlmChatPage.tsx:372-573` |
| 新建对话按钮 | ✅ | `LlmChatPage.tsx:202-207` |
| 消息气泡（用户右/AI 左） | ✅ | `LlmChatPage.tsx:474-504` |
| Markdown 渲染 | ✅ | `LlmChatPage.tsx:493` 使用 `ReactMarkdown` |
| 时间戳 | ✅ | `LlmChatPage.tsx:476-477` |
| 复制按钮 | ✅ | `LlmChatPage.tsx:499-502` |
| 输入框 + 发送按钮 | ✅ | `LlmChatPage.tsx:552-570` |
| 模型信息栏 | ✅ | `LlmChatPage.tsx:519-549` |
| 在线/离线状态 | ✅ | `LlmChatPage.tsx:96-180` |
| 对话生命周期 | ✅ | `LlmChatPage.tsx:209-272` |
| 自动命名 | ✅ | `LlmChatPage.tsx:221` 取前 30 字符 |
| 软删除/回收站 | ✅ | `LlmChatPage.tsx:288-319` |
| 恢复/永久删除 | ✅ | `LlmChatPage.tsx:297-310` |
| 悬浮卡片查看已删除对话 | ✅ | `LlmChatPage.tsx:576-609` |
| 打字动画（加载态） | ✅ | `LlmChatPage.tsx:507-511` |

#### 9.x 未实现的对话功能

| 功能 | 状态 | 代码位置 | 差距说明 |
|------|------|---------|---------|
| **流式输出** | ❌ | `LlmChatPage.tsx:209-272` | `sendMessage` 等待完整响应后一次性 setState，没有逐字累积 |
| **系统提示词注入** | ❌ | `LlmChatPage.tsx:237-243` | messages 直接传给 `llm_send_message`，未注入 system prompt |
| **页面切换后台继续** | ⚠️ | `LlmChatPage.tsx` | 当前实现在组件 unmount 后 stream 会中断（因为没有持久化 stream 订阅到 Store 层） |
| **AI 报错信息样式** | ⚠️ | `LlmChatPage.tsx:483-490` | 文档要求"红色/橙色背景"区分正常回复，当前只是文字颜色变红，背景未变 |

**差距 8（P0）**：对话页面未实现流式输出。当前 `sendMessage` 是阻塞式请求，等待 `llm_send_message` 返回完整响应后才更新 UI。文档要求：
- 本地 Ollama：原生 SSE 逐 chunk 更新
- 云端：打字机效果（完整获取后按 grapheme cluster 逐字 emit，8ms/字）

---

### §10 使用统计（全新章节）

| 规范项 | 实现状态 | 差距说明 |
|--------|---------|---------|
| **推理调用次数统计** | ❌ | 未实现 |
| **Prompt Token 估算** | ❌ | 未实现 |
| **Completion Token 估算** | ❌ | 未实现 |
| **总 Token 统计** | ❌ | 未实现 |
| **按模型统计** | ❌ | 未实现 |
| **每日统计** | ❌ | 未实现 |
| **`llm_get_stats` 命令** | ❌ | 未注册 |
| **`llm_reset_stats` 命令** | ❌ | 未注册 |
| **会话级统计（内存）** | ❌ | 未实现 |
| **账户级统计（Vault 持久化）** | ❌ | 未实现 |
| **Token 估算函数** | ❌ | 未实现 |
| **统计 UI 展示** | ❌ | 未实现 |

**差距 9（P1）**：使用统计整体缺失。文档 v3.0 新增要求追踪会话级和账户级使用统计，当前没有任何相关代码。

---

### §11 完成标准差距汇总

#### P0 必须项（当前未实现）

| # | 完成标准项 | 差距说明 |
|---|-----------|---------|
| 1 | 系统提示词 7 Section 模板 | 完全未实现 |
| 2 | 上下文注入服务（隐私分级过滤、缓存、长度限制、截断） | 完全未实现 |
| 3 | 帮助文档检索（关键词匹配、评分、阈值、Top-1） | 完全未实现 |
| 4 | 流式响应（本地 SSE / 云端打字机效果） | 完全未实现，当前 `stream: false` |
| 5 | 设置页系统提示词开关区域 | UI 中缺失 |
| 6 | `includeSystemPrompt` 配置字段 | `LlmConfig` 缺少该字段 |
| 7 | 风险告知补充说明（系统提示词隐私承诺） | 翻译文件中缺失 |
| 8 | Base URL `/v1` 后缀验证 | 无验证 |

#### P1 重要项（当前未实现）

| # | 完成标准项 | 差距说明 |
|---|-----------|---------|
| 1 | 使用统计追踪（会话级 + 账户级） | 完全未实现 |
| 2 | Token 用量估算与展示 | 完全未实现 |
| 3 | 页面切换后台继续（stream 订阅提升到 Store） | 当前 stream 在组件层，unmount 后中断 |
| 4 | 系统提示词注入开关（高级用户可关闭） | 依赖 P0 #5 |
| 5 | 审计日志记录每次 AI 调用 | `llm_send_message` 未记录审计日志 |

#### P2 增强项（当前未实现）

| # | 完成标准项 | 差距说明 |
|---|-----------|---------|
| 1 | 帮助文档向量检索 | 依赖 P0 #3 |
| 2 | 上下文压缩 | 依赖 P0 #2 |
| 3 | 智能截断 | 依赖 P0 #2 |

---

## 2. IPC 命令差距

### 已注册命令（`lib.rs`）

```rust
// LLM commands — 已注册
llm_get_config, llm_get_providers, llm_save_provider, llm_set_active_provider,
llm_set_ai_features, llm_accept_risk, llm_delete_provider, llm_get_api_key,
llm_test_provider, llm_send_message, llm_list_conversations, llm_get_conversation,
llm_save_conversation, llm_delete_conversation, llm_rename_conversation,
llm_soft_delete_conversation, llm_restore_conversation, llm_permanent_delete,
llm_list_trash, llm_check_connection
```

### 缺失命令

| 命令 | 用途 | 优先级 |
|------|------|--------|
| `llm_get_stats` | 获取使用统计 | P1 |
| `llm_reset_stats` | 重置使用统计 | P1 |
| `llm_stream_message` | 流式发送消息（SSE/打字机） | P0 |
| `llm_build_system_prompt` | 构建系统提示词（调试用） | P0 |

---

## 3. 翻译文件差距

### `tauri/src/locales/*/settings.json`

**已存在的 key（约 54 个）**：覆盖基础 LLM 配置、对话、回收站等功能。

**缺失的 key**：

| key | 中文 | 英文 | 优先级 |
|-----|------|------|--------|
| `ai_system_prompt_title` | 系统提示词（上下文注入） | System Prompt (Context Injection) | P0 |
| `ai_system_prompt_software` | 注入软件信息和使用统计 | Inject software info & usage stats | P0 |
| `ai_system_prompt_profile` | 注入用户公开档案 | Inject user public profile | P0 |
| `ai_system_prompt_guides` | 注入相关帮助文档 | Inject relevant help docs | P0 |
| `ai_risk_li5` | 系统提示词仅包含您主动公开的信息 | System prompt only includes info you actively share | P0 |
| `ai_stats_title` | 使用统计 | Usage Statistics | P1 |
| `ai_stats_calls` | 累计调用次数 | Total calls | P1 |
| `ai_stats_tokens` | 累计 Token 消耗 | Total tokens used | P1 |
| `ai_stats_session` | 当前会话 | Current session | P1 |
| `ai_streaming_generating` | 正在生成... | Generating... | P0 |

---

## 4. 资源文件差距

### 帮助文档目录

```
文档要求：
tauri/src-tauri/resources/docs/guides/
├── index.json
├── zh/
│   └── *.md
└── en/
    └── *.md

当前状态：
❌ 目录不存在
❌ 无任何帮助文档文件
```

---

## 5. 核心代码问题（除功能缺失外）

### 5.1 `llm_send_message` 签名不符合文档要求

**文档 §5.1 要求**：
```
前端发送：用户消息 + 历史对话
后端处理：1. 读取 Provider 配置 2. 构建系统提示词 3. 检索帮助文档 4. 组装 messages
```

**当前实现**（`llm.rs:476-477`）：
```rust
pub async fn llm_send_message(
    base_url: String, api_key: String, model: String, api_type: ApiType,
    messages: Vec<serde_json::Value>  // ← 前端已组装好的 messages，后端直接转发
) -> Result<String, String>
```

**问题**：系统提示词构建、帮助文档检索等应在 Rust 后端完成，但当前签名要求前端传入完整的 messages 数组，导致后端失去了注入 system prompt 的能力。

**建议修改签名**：
```rust
pub async fn llm_send_message(
    state: State<'_, AppState>,
    account_id: String,
    conversation_id: String,
    prompt: String,           // ← 仅用户当前输入
    history: Vec<ChatMessage>, // ← 历史对话（不含 system）
    include_system_prompt: bool,
) -> Result<String, String>   // ← 或 Stream<String>
```

### 5.2 对话存储在 Profile JSON 中

当前对话存储在 `preferences.llmConversations` 中，与文档 §3.1 的存储分层没有直接冲突，但文档未明确对话的存储位置。当对话数量增长时，整个 Profile JSON 会变大，可能影响性能。建议在后续迭代中考虑独立存储。

### 5.3 API Key 存储方式

当前 API Key 以明文 JSON 存储在 Vault 加密的 Profile 中（`preferences.llmApiKeys`）。文档 §3.2 要求使用"Vault 主密钥派生的独立子密钥加密"，当前实现虽然数据在 Vault 中，但密钥派生逻辑未单独实现。不过考虑到 Vault 整体已加密，此差距可接受为 P1。

---

## 6. 差距优先级矩阵

| 差距 | 影响范围 | 实现复杂度 | 优先级 |
|------|---------|-----------|--------|
| 系统提示词 7 Section 模板 + LlmContextService | 后端 | 中 | **P0** |
| 流式响应（SSE/打字机） | 后端 + 前端 | 高 | **P0** |
| 帮助文档检索（关键词匹配） | 后端 + 资源文件 | 低 | **P0** |
| 设置页系统提示词开关 UI | 前端 | 低 | **P0** |
| `includeSystemPrompt` 配置字段 | 后端 + 前端 | 低 | **P0** |
| 隐私分级过滤（仅 public） | 后端 | 中 | **P0** |
| 风险告知补充翻译 | 翻译文件 | 低 | **P0** |
| Base URL 验证 | 前端 | 低 | P1 |
| 使用统计追踪 | 后端 + 前端 | 中 | P1 |
| 审计日志记录 AI 调用 | 后端 | 低 | P1 |
| Stream 订阅提升到 Store 层 | 前端 | 中 | P1 |
| Token 估算函数 | 后端 | 低 | P1 |
| 帮助文档向量检索 | 后端 | 高 | P2 |
| 上下文压缩 | 后端 | 高 | P2 |
| 智能截断 | 后端 | 中 | P2 |

---

## 7. 推荐实施顺序

### Phase 1（P0 核心功能）

1. **数据模型扩展**：`LlmConfig` 增加 `include_system_prompt: bool`
2. **系统提示词模板**：Rust 端实现 7 Section 模板 + 硬编码行为规范
3. **上下文注入服务**：`LlmContextService` — 收集 public 级别数据、缓存、长度限制、截断
4. **帮助文档检索**：`UserGuideService` — 关键词匹配 + 评分 + Top-1
5. **资源文件**：创建 `resources/docs/guides/` + `index.json` + 中英文文档
6. **设置页 UI**：增加系统提示词开关区域
7. **翻译补充**：添加所有缺失的 key
8. **后端命令改造**：`llm_send_message` 改造为在 Rust 端组装 messages（含 system prompt + help doc）

### Phase 2（P0 流式响应）

9. **本地 Ollama SSE**：实现 `llm_stream_message` 命令，使用 SSE 流式
10. **云端打字机效果**：完整获取后按 grapheme cluster 逐字 emit，8ms/字
11. **前端流式 UI**：订阅 stream，逐 chunk 更新消息内容

### Phase 3（P1 增强）

12. **使用统计**：统计数据结构 + `llm_get_stats` / `llm_reset_stats` 命令
13. **审计日志**：`llm_send_message` 中记录调用元数据
14. **Token 估算**：字符级估算函数 + UI 展示
15. **Stream Store 层提升**：将 stream 订阅从组件层移到 Zustand Store

---

## 8. 与 Flutter 实现的关键差距

| 功能 | Flutter 实现 | Tauri 当前 | 差距 |
|------|-------------|-----------|------|
| 系统提示词 | ✅ 完整 7 Section | ❌ 无 | **大** |
| 上下文注入 | ✅ 隐私过滤 + 缓存 + 截断 | ❌ 无 | **大** |
| 帮助文档检索 | ✅ 关键词匹配 + 多语言 | ❌ 无 | **大** |
| 流式响应 | ✅ 本地 SSE + 云端打字机 | ❌ 非流式 | **大** |
| 使用统计 | ✅ 会话 + 账户级 | ❌ 无 | 中 |
| 对话回收站 | ✅ 独立回收站 | ✅ 已实现 | — |
| Provider 管理 | ✅ 预置 + 自定义 | ✅ 已实现 | — |
| 风险告知 | ✅ 强制弹窗 | ✅ 已实现 | — |

---

*报告版本：v1.0*
*分析日期：2026-06-08*
*对应文档：`20_LLM配置与AI对话规范.md` v3.0*
