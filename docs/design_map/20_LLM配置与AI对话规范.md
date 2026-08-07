# 20 — LLM 配置与 AI 对话规范

> **前置阅读**：`13_用户数据边界与加密存储.md`、`24_矛盾冲突与决策记录.md`、`10_跨平台视觉规范与主题系统.md`（侧边栏规范）
> **Manifesto 对齐**：用户主权 | 隐私优先 | 安全默认
>
> **[状态] 已实施（2026-08）**：本文档所述 Provider 管理、后端代理、流式响应、RAG 均已落地；模式 A（前端构建，已废弃）仅作历史参考。
> **文档定位**：定义 SoloSoul LLM（大语言模型）集成的全部规范，包括 Provider 管理、API 配置、模型选择，AI 对话页面的功能规格与 UI 设计，以及系统提示词、上下文注入、帮助文档嵌入等 AI 智能能力。AI 功能是软件的附加扩展，不影响核心离线本地功能。

---

## 1. 设计原则

| 原则 | 说明 |
|------|------|
| **用户主权** | 用户选择用什么模型、哪家服务商、甚至自建 API |
| **默认禁用** | AI 功能默认全部关闭，首次开启需风险告知确认 |
| **不绑定厂商** | 不限制任何特定厂商，支持所有 OpenAI 兼容 API |
| **密钥安全** | API 密钥 `critical` 级别加密存储，使用后内存擦除 |
| **透明可控** | 每次 AI 调用前用户确认发送内容，审计日志记录调用元数据 |
| **隐私优先** | 系统提示词仅包含用户主动公开的信息，绝不暴露敏感数据 |
| **本地优先** | 帮助文档检索、上下文构建均在本地完成，不上传用户数据到外部服务 |

---

## 2. Provider 模型

### 2.1 什么是 Provider

Provider 是 LLM 服务的配置单元，包含连接一个 LLM API 所需的全部信息。

```typescript
interface LlmProvider {
  id: string;              // UUID，内部唯一标识
  name: string;            // 用户自定义名称（如"我的工作 GPT"、"本地 Ollama"）
  baseUrl: string;         // API Base URL（如 https://api.openai.com/v1）
  apiKey: string;          // API 密钥（critical 级别，加密存储）
  model: string;           // 模型名称（如 gpt-4o、claude-3-sonnet、llama3.1）
  isEnabled: boolean;      // 是否启用
  isBuiltIn: boolean;      // 是否系统预置（用户不可删除预置 Provider）
}
```

### 2.2 预置 Provider

软件内置以下预置 Provider，用户可直接启用或修改：

| 预置 Provider | Base URL（默认） | 模型（默认） | 说明 |
|--------------|-----------------|-------------|------|
| OpenAI | `https://api.openai.com/v1` | `gpt-4o` | 需用户填写 API Key |
| Anthropic | `https://api.anthropic.com/v1` | `claude-3-sonnet-20241022` | 需用户填写 API Key |
| Ollama（本地） | `http://localhost:11434/v1` | `llama3.1` | 本地运行，无外传风险 |
| DeepSeek | `https://api.deepseek.com/v1` | `deepseek-chat` | 需用户填写 API Key |
| 阿里云百炼 | `https://dashscope.aliyuncs.com/compatible-mode/v1` | `qwen-max` | 需用户填写 API Key |

> **预置 Provider 规则**：
> - 预置 Provider 的 `apiKey` 字段为空，用户首次启用时填写
> - 预置 Provider 的 `baseUrl` 和 `model` 可修改（如切换到 OpenAI 的 `gpt-4o-mini`）
> - 预置 Provider 不可删除，但可禁用
> - **默认值兜底 + 用户修改覆盖**：预置 Provider 的默认 baseUrl/model 作为兜底配置加载，用户修改后的值持久化覆盖。软件升级新增预置 Provider 时，已保存的自定义配置不会丢失。

### 2.3 自定义 Provider

用户可添加任意 OpenAI 兼容格式的 API：

```
添加自定义 Provider：

┌──────────────────────────────────────────┐
│ 添加 AI 服务商                            │
│ ──────────────────────────────────────  │
│  名称        [我的工作助手          ]     │  ← 用户自定义命名
│  Base URL    [https://api.example.com/v1] │  ← API 基础地址
│  API 密钥    [************************]  │  ← 密码输入框，遮蔽
│  模型名称    [gpt-4o-mini           ]     │  ← 模型 ID
│                                           │
│  [测试连接]                               │  ← 验证配置是否正确
│                                           │
│  [  保存  ]  [  取消  ]                   │
└──────────────────────────────────────────┘
```

**自定义 Provider 规则**：

| 字段 | 必填 | 验证 | 说明 |
|------|------|------|------|
| 名称 | 是 | 1-30 字符 | 用户自定义，支持国际化；超出 30 字符截断或禁止输入 |
| Base URL | 是 | 有效 HTTP(S) URL | 通常为 `/v1` 结尾（OpenAI 兼容），但允许特殊路径（如 Azure OpenAI）；后端拼接路径时自动处理尾部斜杠 |
| API 密钥 | 是 | 非空 | `SecurePasswordInput` 组件，始终遮蔽 |
| 模型名称 | 是 | 非空 | 服务商提供的模型 ID |

**测试连接**：
- 点击"测试连接"后，发送一条极简请求（如 `"Hello"`）到 Provider
- 成功：显示"连接成功" + 模型返回的简短响应
- 失败：显示错误信息（网络错误 / 认证失败 / 模型不存在等）

---

## 3. 配置存储

### 3.1 存储分层

| 数据 | 存储位置 | 理由 |
|------|---------|------|
| Provider 列表（不含 apiKey） | `preferences.enc`（Vault 加密） | 含 baseUrl/model，可推断用户行为偏好 |
| API 密钥 | 独立加密存储 + 内存安全 | `critical` 级别，等同密码 |
| 当前活跃 Provider ID | `preferences.enc`（Vault 加密） | 用户偏好的一部分 |
| AI 功能开关状态 | `preferences.enc`（Vault 加密） | 用户偏好的一部分 |
| 系统提示词缓存 | 内存（主缓存）+ Vault（元数据跨会话恢复） | 静态部分缓存在内存，缓存键持久化到 Vault 避免冷启动 |
| 使用统计 | Vault 加密（按账户隔离） | 跨会话持久化 |

### 3.2 API 密钥安全存储

```rust
// Rust 侧：API 密钥单独加密存储
struct LlmApiKeyStorage {
    // 使用 Vault 主密钥派生的独立子密钥加密
    // 密钥不与其他偏好数据混存，降低泄露面
}

// 读取流程
async fn get_api_key(provider_id: &str, vault: &Vault) -> Result<SecureString, Error> {
    let encrypted = api_key_repo.get(provider_id).await?;
    let decrypted = vault.decrypt_api_key(&encrypted)?;
    // 返回后由调用方负责在使用后擦除内存
    Ok(decrypted)
}
```

**安全约束**：
- API 密钥仅在内存中存在，使用 `zeroize` crate 确保使用后擦除
- 密钥绝不写入日志、审计记录或任何持久化存储
- 前端代码**绝不接触**原始 API 密钥——所有 API 调用由 Rust 后端代理

**返回掩码规范**：
- `llm_get_providers` 返回 Provider 列表时，必须将所有非空 `apiKey` 替换为掩码字符串 `••••••••`
- 保存 Provider 时，如果 `apiKey == "••••••••"`，表示用户未修改密钥，跳过保存避免用掩码覆盖真实密钥
- 此机制确保前端开发者即使在控制台打印返回值，也不会意外泄露原始 API Key

### 3.3 Provider 列表数据结构

```typescript
// 存储于 preferences.enc 中的 LLM 配置
interface LlmConfig {
  providers: ProviderConfig[];     // Provider 列表（不含 apiKey）
  activeProviderId: string | null; // 当前使用的 Provider
  aiFeaturesEnabled: {             // AI 功能开关（默认全部 false）
    chat: boolean;
    smartFill: boolean;
    commandGen: boolean;
    naturalLanguageSearch: boolean;
  };
  includeSystemPrompt: boolean;    // 是否注入系统提示词（默认 true，高级用户可关闭）
}

// Provider 配置（存储版本，不含密钥）
interface ProviderConfig {
  id: string;
  name: string;
  baseUrl: string;
  model: string;
  isEnabled: boolean;
  isBuiltIn: boolean;
}
```

---

## 4. UI 规范

### 4.1 设置页 LLM 配置入口

```
设置 → AI 与智能助手

┌──────────────────────────────────────────┐
│ AI 与智能助手                              │
│ ──────────────────────────────────────  │
│                                          │
│  [IconWarning] AI 功能涉及数据外传风险              │
│  所有 AI 功能默认关闭，开启前请阅读风险告知  │
│                                          │
│  AI 功能开关                              │
│  ├─ [ ] AI 对话                          │
│  ├─ [ ] 智能填充                          │
│  ├─ [ ] 命令生成                          │
│  └─ [ ] 自然语言搜索                       │
│                                          │
│  系统提示词（上下文注入）                    │
│  ├─ [√] 注入软件信息和使用统计              │
│  ├─ [√] 注入用户公开档案                    │
│  └─ [√] 注入相关帮助文档                    │
│                                          │
│ ──────────────────────────────────────  │
│                                          │
│  AI 服务商（Provider）                     │
│  ┌────────────────────────────────────┐ │
│  │ ● OpenAI          gpt-4o      [IconEdit] │ │  ← 当前活跃
│  │ ○ Anthropic       claude-3    [IconEdit] │ │
│  │ ○ Ollama(本地)    llama3.1    [IconEdit] │ │
│  │ ○ 我的工作助手    gpt-4o-mini [IconEdit] │ │  ← 自定义
│  └────────────────────────────────────┘ │
│  [+ 添加自定义服务商]                      │
│                                          │
└──────────────────────────────────────────┘
```

### 4.2 Provider 列表项

| 元素 | 规范 |
|------|------|
| 单选按钮 | 左侧 `Radio` 图标，选中为 `--accent-primary`，未选为 `--border-subtle` |
| 名称 | 用户自定义名称（或预置名称），`--text-primary`，Body（14px） |
| 模型 | 当前配置的模型名称，`--text-secondary`，Caption（12px） |
| 编辑按钮 | 右侧 `Settings` 图标，点击打开 Provider 配置面板 |
| 禁用状态 | Provider 未填写 API Key 时，名称灰色显示，hover 提示"请配置 API 密钥" |

### 4.3 风险告知对话框

用户首次开启任何 AI 功能时，强制展示风险告知：

```
┌──────────────────────────────────────────┐
│  [IconWarning] 启用 AI 功能前请确认                    │
│ ──────────────────────────────────────  │
│                                          │
│  您即将启用 AI 对话功能。根据您选择的 AI   │
│  服务商，您的输入内容将被发送到外部服务器   │
│  进行处理。                                │
│                                          │
│  SoloSoul 承诺：                          │
│  • 不会自动发送任何数据                    │
│  • 每次发送前会请您确认内容                │
│  • API 密钥仅存储在您的本地设备            │
│  • 可随时关闭此功能                        │
│  • 系统提示词仅包含您主动公开的信息        │
│                                          │
│  [IconCheck] 我已了解风险并同意开启               │
│                                          │
│  [  确认开启  ]  [  取消  ]               │
└──────────────────────────────────────────┘
```

- 必须勾选"我已了解风险"才能点击"确认开启"
- 确认后写入审计日志，记录时间戳和开启的功能类型

---

## 5. API 调用流程

### 5.1 后端代理架构

前端**不直接调用**任何外部 LLM API，所有请求由 Rust 后端代理。

**模式 B（后端构建，唯一用户-facing 路径）**：
前端只发送用户 prompt 和历史对话，后端在 Rust 端查询 Vault 数据、构建系统提示词、检索帮助文档、组装 messages 并发送请求。
- 隐私过滤在 Rust 端强制完成，不可被绕过
- 前端无需关心系统提示词构建细节，用户完全无感
- 所有用户-facing 的 AI 对话统一走此路径

**模式 A（前端构建，已废弃）**：
原前端利用 Zustand Store 缓存数据构建系统提示词的方案。现仅作为 Provider 不支持 SSE 时的**内部降级保留**，不暴露给用户，不在任何 UI 中出现。

```
模式 B（后端构建，唯一用户-facing 路径）
─────────────────────────────────────────────
前端（React）
    ↓ IPC: invoke('llm_chat', { accountId, conversationId, prompt, history })
    ↓ （用户无感，只传 prompt + history）
Rust 后端（Tauri）
    ↓ 1. 读取 Provider 配置 + 解密 API Key
    ↓ 2. LlmContextService::build_context(accountId)
    │     查询 Vault public 对象 → 偏好 → 插件 → 统计 → 组装 7 Section
    ↓ 3. GuideService::find_relevant_guides(prompt, language)
    ↓ 4. 组装 messages（system + help doc + history + user prompt）
HTTP Client（reqwest）
    ↓ SSE 流式发送请求（stream: true）
外部 LLM API
    ↓ 逐 token 响应
Rust 后端
    ↓ app.emit("llm-stream-chunk", ...) 逐段推送
前端（Store 层监听 Event，累积渲染）
```

**内部降级路径**（用户不可见）：
当 Provider 不支持 SSE 或 SSE 解析失败时，后端内部回退到完整获取后打字机效果。此降级封装在 Rust 内部，前端调用方式不变，用户无感知。

**设计原则**：
- API 密钥从不离开 Rust 后端
- 前端代码不依赖任何特定 LLM SDK
- 统一错误处理和重试逻辑
- 便于审计日志记录
- 用户不需要选择模式，不需要调试，只需要一致的 AI 对话体验

### 5.2 OpenAI 兼容格式

所有 Provider（包括预置和自定义）均使用 OpenAI 兼容的 Chat Completions API：

```rust
// 统一请求格式
struct ChatCompletionRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,  // 始终启用流式响应
    temperature: Option<f32>,
    max_tokens: Option<u32>,
}

// Message 结构体 — role 使用 String 而非枚举，保持扩展性
struct Message {
    role: String,      // "system" | "user" | "assistant"，未来可扩展 "tool" 等
    content: String,
}
```

> **为什么 role 用 String 而不是枚举？**
> - 保持扩展性：未来可能支持 `tool`、`function` 等新角色，无需修改数据结构
> - Anthropic 适配层需要在 messages 中识别 `system` 并分离到顶层 `system` 字段，String 让过滤更直接
> - 前端可以灵活地在消息历史中保留 `system` 消息（如用户之前手动添加的系统提示）

// Anthropic 适配层：在 Rust 后端将 OpenAI 格式转换为 Anthropic 格式
// Ollama 适配层：Ollama 已原生支持 OpenAI 兼容 API
// 自定义 Provider：直接使用用户提供的 baseUrl，发送标准 OpenAI 格式

> **Anthropic 特殊处理**：Anthropic API 不是原生 OpenAI 兼容格式，Rust 后端需要提供适配层，将 OpenAI 格式的请求转换为 Anthropic 格式（Messages API）。
>
> **Anthropic thinking 模型处理**：Claude 3.7 Sonnet 等 thinking 模型返回 `content` 为数组（content blocks），格式如 `[{"type":"thinking",...}, {"type":"text","text":"..."}]`。适配层必须过滤出 `type: "text"` 或 `type` 缺失的块来提取实际回复文本，不能直接取 `content` 字符串。

### 5.3 流式响应策略

**Tauri v2 技术限制**：`invoke` 命令不支持原生返回 Stream，必须通过 Tauri Event 实现流式推送。

| Provider 类型 | 流式实现 | 技术方案 |
|--------------|---------|---------|
| **本地 Ollama** | SSE → Tauri Event | 后端用 reqwest 解析 SSE，通过 `app.emit("llm-stream-chunk", ...)` 逐段推送 |
| **云端 OpenAI 兼容** | SSE → Tauri Event | 后端用 reqwest 解析 SSE（`stream: true`），逐 token 推送 |
| **云端 Anthropic** | SSE → Tauri Event | 同上，适配层转换后解析 SSE 逐 token 推送 |
| **降级 fallback** | 打字机效果 | 当 Provider 不支持 SSE 或解析失败时，内部回退到完整获取后打字机 emit |

> **设计决策**：所有 Provider 统一走 SSE 流式。打字机效果仅作为内部降级，不暴露 `streamMode` 设置选项给用户。

#### 5.3.1 流式 IPC 技术实现（Tauri Event）

**SSE 流式路径（默认）**：

```
前端调用: invoke('llm_chat', { accountId, conversationId, prompt, history })
    ↓
Rust 后端: 启动异步任务，发送 HTTP 请求（stream: true）
    ↓
Rust 后端: 解析 SSE 数据流
    for each SSE chunk:
        app.emit("llm-stream-chunk", Payload { conversation_id, chunk: "..." })
    ↓
前端监听: listen("llm-stream-chunk", handler)
    ↓
前端: 累积 chunk 到当前 AI 消息
    ↓
Rust 后端: SSE 流结束（或收到 [DONE]）
    app.emit("llm-stream-chunk", Payload { conversation_id, chunk: "", is_done: true })
    或出错时: app.emit("llm-stream-chunk", Payload { conversation_id, error: "..." })
```

**打字机降级路径（内部，用户不可见）**：

```
Rust 后端: 检测到 Provider 不支持 SSE 或 SSE 解析失败
    ↓
Rust 后端: 完整获取响应文本
    ↓
Rust 后端: 按 grapheme cluster 逐字 emit（避免切断 emoji / 中文）
    for each grapheme:
        app.emit("llm-stream-chunk", Payload { conversation_id, chunk: "..." })
        sleep(2ms / 4ms)
```

**前端状态管理要求**：
- Stream 订阅必须提升到 **Zustand Store 层**（而非组件 `useEffect`）
- 组件 unmount 时订阅保持，切换页面后仍能接收 chunk
- Store 提供 `startStream()` / `stopStream()` / `onChunk()` / `onDone()` / `onError()` 方法

#### 5.3.2 打字机效果参数（内部降级用）

```rust
// 仅当 SSE 不可用时内部使用
let graphemes = full_response.graphemes(true).collect::<Vec<_>>();
let total = graphemes.len();
let max_typing_ms = 3000; // 最多打字 3 秒
let delay_ms = if total <= 50 { 2 } else { 4 }; // 前 50 字 2ms，后续 4ms

for (i, g) in graphemes.iter().enumerate() {
    let elapsed = i * delay_ms;
    if elapsed >= max_typing_ms {
        emit_to_frontend(&graphemes[i..].concat());
        break;
    }
    emit_to_frontend(g);
    tokio::time::sleep(Duration::from_millis(delay_ms as u64)).await;
}
```

**参数说明**：
- 默认延迟：4ms/字符
- 变速：前 50 个字符 2ms（快速呈现开头），后续 4ms
- 上限：最多打字 3 秒，超出直接显示剩余内容
- **不暴露给用户**：此降级路径完全封装在 Rust 后端，用户设置中无 `streamMode` 或 `instantDisplay` 选项

---

## 6. 系统提示词与上下文注入

### 6.1 设计目标

系统提示词（System Prompt）是 AI 对话的"底层指令"，定义了 AI 助手的身份、能力边界和行为规范。SoloSoul 的系统提示词设计遵循以下原则：

1. **隐私优先**：仅包含用户主动公开的信息，绝不暴露敏感数据
2. **动态注入**：每次发送消息前重新构建，确保数据始终最新
3. **长度可控**：总提示词上限 2000 字符，超出时智能截断
4. **缓存优化**：静态部分（用户资料、偏好）缓存，实时统计动态追加

### 6.2 系统提示词结构

系统提示词由 **7 个 Section** 组成，按以下顺序拼接：

```
【Section 1: AI 身份定义】
你是 SoloSoul（独灵）的 AI 助手 Solon，由 SoloSoul 团队开发。
你是用户的个人智能助手，了解用户的个人信息（仅限用户主动分享的部分）。
你的回答应当简洁、准确、有帮助。

【Section 2: 软件信息】
当前 SoloSoul 版本：{appVersion}
平台：{platform}
界面语言：{language}

【Section 3: 用户公开对象数据】
用户主动公开的信息（从所有对象中提取 SensitivityLevel.public 级别的属性）：
{userPublicObjectData}

> **数据收集方式**：遍历所有对象 → 筛选 `sensitivity_level == "public"` 的属性 → 按对象类型分组 → 最多每类型 3 个对象、每对象 8 个属性 → 属性值截断至 100 字符。
> 示例：联系人（姓名：张三、职业：工程师）、旅行记录（目的地：东京、日期：2026-05-01）

【Section 4: 偏好设置】
{preferences}

【Section 5: 已安装插件】
{installedPlugins}

【Section 6: 使用统计】
{usageStats}

【Section 7: 行为规范】
1. 使用与用户提问相同的语言回答
2. 区分"插件"（功能扩展）和"对象"（用户数据）
3. 敏感/受限/关键数据需要重新验证密码，无法直接访问
4. 无法访问用户本地数据时，建议用户手动查找而非编造
5. 不泄露用户数据给插件或外部服务
6. 用户询问功能使用方法时，基于软件信息回答
```

### 6.3 各 Section 数据来源

| Section | 数据来源 | 敏感级别过滤 | 说明 |
|---------|---------|-------------|------|
| 1. AI 身份 | 硬编码 | — | 固定文本，不可修改 |
| 2. 软件信息 | 运行时获取 | — | appVersion、platform、language |
| 3. 用户公开对象数据 | 对象服务 | **仅 public** | 遍历所有对象，提取 `SensitivityLevel.public` 级别的属性 |
| 4. 偏好设置 | Preferences 服务 | **仅 public** | 主题、默认对象类型等 |
| 5. 已安装插件 | Plugin 服务 | — | 插件名称列表（**当前留空预留，TODO：等插件系统上线后接入**） |
| 6. 使用统计 | LLM 统计服务 | — | 累计使用次数、Token 消耗（实时） |
| 7. 行为规范 | 硬编码 | — | 固定文本，不可修改 |

### 6.4 上下文注入服务（LlmContextService）

Rust 后端提供 `LlmContextService`，负责在每次发送消息前构建系统提示词。

#### 6.4.1 构建流程

```rust
async fn build_context(account_id: &str, model_manager: &LlmModelManager) -> Result<ContextResult, Error> {
    // 1. 构建缓存键
    let cache_key = build_cache_key(account_id).await?;
    
    // 2. 检查缓存
    if let Some(cached) = PROMPT_CACHE.get(&cache_key) {
        // 缓存命中：复用静态部分，追加实时统计
        let stats = model_manager.build_stats_snapshot();
        let system_prompt = inject_realtime_stats(&cached.system_prompt, &stats);
        return Ok(ContextResult { system_prompt, was_cached: true, ... });
    }
    
    // 3. 缓存未命中：重新构建
    let profile_data = collect_public_profile_data(account_id).await?;
    let preferences = collect_preferences(account_id).await?;
    let plugins = collect_installed_plugins().await?;
    let stats = model_manager.build_stats_snapshot();
    
    // 4. 组装系统提示词
    let system_prompt = PromptTemplate::chat_system_prompt(
        app_version: ..., platform: ..., language: ...,
        user_public_info: &profile_data,
        preferences: &preferences,
        installed_plugins: &plugins,
        usage_stats: &stats,
    );
    
    // 5. 缓存静态部分
    PROMPT_CACHE.insert(cache_key, CachedPrompt { ... });
    
    Ok(ContextResult { system_prompt, was_cached: false, ... })
}
```

#### 6.4.2 缓存策略

| 缓存维度 | 实现方式 |
|---------|---------|
| **缓存键** | `account_id + 公开对象数量 + public_data_version` |
| **缓存内容** | 静态部分（Section 1-5：AI 身份、软件信息、用户公开对象数据、偏好、插件列表） |
| **不缓存内容** | 实时统计（Section 6：使用次数、Token 数）——每次注入时动态追加 |
| **缓存位置** | Rust 内存 `HashMap`（主缓存）+ Vault 持久化（缓存元数据，跨会话恢复） |
| **失效条件** | `public_data_version` 变化（任何 public 级别数据变更时 +1）或切换账户 |

> **`public_data_version` 实现**：在 Vault Profile JSON `preferences.llmPublicDataVersion` 中维护计数器。Rust 端在 `object_create` / `object_update` 命令中检测 `sensitivityLevel == "public"` 的变更时自动 +1。避免遍历所有对象计算 `updated_at` 总和的性能开销，且与 Vault 数据一致性更好。

> **跨会话持久化**：缓存元数据（缓存键 + 时间戳）持久化到 Vault，App 重启后恢复，避免每次启动后的冷启动构建。缓存的实际内容（系统提示词文本）不持久化（因长度较大），仅恢复元数据以判断缓存是否仍有效。

#### 6.4.3 长度限制与截断策略

长度限制分层（避免单一层级限制导致过度截断）：

```rust
const MAX_OBJECTS_PER_TYPE: usize = 3;       // 每类型最多 3 个对象
const MAX_PROPERTIES_PER_OBJECT: usize = 8;  // 每对象最多 8 个属性
const MAX_VALUE_LENGTH: usize = 100;         // 每个值最多 100 字符
const MAX_SYSTEM_PROMPT_CHARS: usize = 1500; // 系统提示词（Section 1-7）上限
const MAX_DOC_CONTENT_CHARS: usize = 800;    // 单篇帮助文档内容上限
const MAX_HISTORY_MSG_CHARS: usize = 2000;   // 单条历史消息上限（超出时截断旧消息）
const MAX_TOTAL_CONTEXT_CHARS: usize = 8000; // 总上下文上限（system + docs + 最近 N 条历史 + prompt）
```

**Token 估算规则**（⚠️ 近似估算，非精确值）：

> 由于不同 LLM 使用不同的 tokenizer（GPT-4 用 cl100k_base，Claude 用自有方案，Llama/Qwen 用 SentencePiece），本软件的 Token 估算仅用于**长度控制和粗略统计**，不作为计费依据。

保守估算策略（确保不会因估算错误而超限）：
- 所有字符统一按 **1 token / 字符** 估算（向上取整）
- 实际 Token 数通常低于估算值，这确保了不会因估算错误而截断过多内容
- 未来可提供模型特定系数表（P2）

**截断策略**：
1. 优先截断用户数据部分（Section 3 用户公开对象数据）
2. 其次截断历史消息（从最早的消息开始）
3. 保留系统提示词核心部分（Section 1 AI 身份、Section 7 行为规范）
4. 在接近限制时按**段落边界**截断（不切断句子）
5. 截断后追加提示：`（上下文过长，部分内容已省略）`

### 6.5 注入流程（时序图）

**模式 B（后端构建，唯一用户-facing 路径）**：

```
用户发送消息
    │
    ▼
前端: invoke('llm_chat', { account_id, conversation_id, prompt, history, include_system_prompt })
    │
    ▼
Rust 后端: LlmContextService::build_context(account_id)
    │   1. 检查缓存（内存 + Vault 元数据）
    │   2. 缓存命中：复用静态部分；缓存未命中：查询 Vault 重新构建
    │   3. 查询 Vault 中的 public 级别对象数据
    │   4. 查询偏好设置
    │   5. 查询已安装插件（当前留空，TODO）
    │   6. 获取使用统计（STATS_MAP 实时）
    │   7. 组装 system_prompt（7 Section）
    │
    ▼
Rust 后端: UserGuideService::find_relevant_guides(prompt, language)
    │
    ▼
Rust 后端: 组装 messages 并发送 SSE 流式请求
    │
    ▼
SSE 流式响应 → IPC Event 推送 → 前端逐 token 渲染
```

**模式 A（前端构建，已废弃，仅内部参考）**：

> 原前端利用 Zustand Store 缓存数据构建系统提示词的方案。当前代码中保留的 `systemPromptBuilder.ts` 和 `llm_send_message_stream` 命令仅作为 Provider 不支持 SSE 时的**内部降级**，不暴露给用户。
>
> ```
> 前端: 遍历 Zustand Store → 组装 system prompt → IPC 发送完整 messages
> Rust 后端: 直接转发 → HTTP 请求
> ```

### 6.6 隐私分级暴露规则

系统提示词严格遵循 SoloSoul 的敏感数据分级系统：

| 敏感度级别 | 是否进入系统提示词 | 说明 |
|-----------|------------------|------|
| `public` | ✅ 是 | 用户主动公开的信息 |
| `internal` | ❌ 否 | 内部使用数据，不暴露给 AI |
| `private` | ❌ 否 | 私人数据，需要显式授权 |
| `sensitive` | ❌ 否 | 敏感数据，需重新验证密码 |
| `restricted` | ❌ 否 | 受限数据，需重新验证密码 |
| `critical` | ❌ 否 | 关键数据，需重新验证密码 |

**AI 行为约束**（硬编码在系统提示词 Section 7）：
1. **语言匹配**：使用与用户提问相同的语言回答
2. **概念区分**：区分"插件"（功能扩展）和"对象"（用户数据）
3. **敏感数据拒绝**：当被问及敏感/受限/关键数据时，告知用户需要重新验证密码
4. **不编造**：无法访问用户本地数据时，建议用户手动查找，绝不编造
5. **数据保护**：不泄露用户数据给插件或外部服务

### 6.7 持久化规则

| 数据 | 持久化位置 | 说明 |
|------|-----------|------|
| 系统提示词 | **不持久化** | 每次发送前动态生成，不进入消息历史 |
| 系统提示词缓存元数据 | Vault 加密 | 缓存键 + 时间戳，App 重启后恢复，避免冷启动 |
| 对话消息（user/assistant） | Vault 加密 | 每 2 秒 debounce 保存 |
| 使用统计 | Vault 加密（按账户隔离） | **每次推理后 30 秒 debounce 保存**（主要机制）+ 对话切换 / 账户切换 / Vault 锁定前立即保存 |

---

## 7. 帮助文档检索与嵌入

### 7.1 设计目标

当用户询问软件功能使用方法时，AI 助手应能参考官方帮助文档给出准确回答。设计遵循以下原则：

1. **本地优先**：所有检索在本地完成，不上传用户查询到外部服务
2. **轻量高效**：关键词匹配方案，无需向量数据库或外部 embedding 服务
3. **多语言支持**：自动匹配用户当前语言，支持回退
4. **按需注入**：仅在用户问题与帮助文档相关时才注入，避免污染上下文

### 7.2 文档结构

帮助文档存储在应用资源目录下：

```
resources/
├── docs/
│   └── guides/
│       ├── index.json          # 文档索引（id, title, keywords, file）
│       ├── zh/
│       │   ├── export_data.md
│       │   ├── import_data.md
│       │   └── ...
│       ├── en/
│       │   ├── export_data.md
│       │   └── ...
│       └── [其他语言]/
```

`index.json` 格式：

```json
{
  "guides": [
    {
      "id": "export-data",
      "title": "如何导出数据",
      "keywords": ["导出", "备份", "数据迁移", "export", "backup"],
      "files": {
        "zh": "zh/export_data.md",
        "en": "en/export_data.md"
      }
    }
  ]
}
```

### 7.3 检索算法

```rust
async fn find_relevant_guides(query: &str, language: &str) -> Vec<GuideContent> {
    // 1. 加载索引
    let index = load_guide_index().await?;
    
    // 2. 分词 + 停用词过滤
    let tokens = tokenize(query);
    let filtered = remove_stop_words(tokens);  // 中英文停用词表
    
    // 3. 遍历所有指南，计算匹配分数
    let mut scored: Vec<(Guide, i32)> = vec![];
    for guide in &index.guides {
        let mut score = 0;
        for token in &filtered {
            // 关键词匹配 +1
            if guide.keywords.iter().any(|k| k.to_lowercase().contains(token)) {
                score += 1;
            }
            // 标题匹配 +3
            if guide.title.to_lowercase().contains(token) {
                score += 3;
            }
        }
        if score >= 2 {  // 分数阈值
            scored.push((guide.clone(), score));
        }
    }
    
    // 4. 按分数排序，取 Top-1
    scored.sort_by(|a, b| b.1.cmp(&a.1));
    scored.into_iter().take(1).map(|(g, _)| load_guide_content(&g, language)).collect()
}
```

**评分规则**：
- 关键词匹配：+1 分/匹配词
- 标题匹配：+3 分/匹配词
- 返回数量：仅 Top-1（避免注入过多文档内容）

**动态分数阈值**（避免短查询 miss）：

```rust
let threshold = if filtered_tokens.len() >= 2 { 2 } else { 1 };
if score >= threshold {
    scored.push((guide.clone(), score));
}
```

| 查询分词数 | 阈值 | 说明 |
|-----------|------|------|
| ≥ 2 | 2 | 多关键词查询要求至少匹配 2 分 |
| 1 | 1 | 单关键词查询（如"导出""备份"）允许 1 分命中 |
| 0（全是停用词） | — | 不返回任何文档 |

### 7.4 多语言回退

```rust
fn resolve_language(content: &HashMap<String, String>, requested: &str) -> &str {
    if content.contains_key(requested) { return requested; }
    if content.contains_key("en") { return "en"; }
    content.keys().next().unwrap_or("en")  // 第一个可用语言
}
```

回退链：`请求语言 → 英文 → 第一个可用语言`

### 7.5 注入格式

匹配到的指南内容被包装为 `system` 角色的消息，插入到系统提示词之后、历史消息之前：

```
---
以下是与用户问题相关的功能使用文档，请参考这些信息回答用户问题。

【文档：如何导出数据】
1. 打开设置页面
2. 选择"导入导出"选项
3. ...
【文档结束】
---
```

**注入位置**：
```
messages[0] = system: 系统提示词（Section 1-7）
messages[1] = system: 帮助文档（如有匹配）
messages[2..n] = user/assistant: 历史对话
messages[n+1] = user: 当前用户输入
```

### 7.6 内容截断

- 单篇指南内容截断至 **800 字符**
- **截断策略**：按段落边界（`\n\n`）截断，不切断句子；优先在列表项、代码块边界处截断
- **Markdown 完整性保护**：
  - 如果在代码块（```）中间截断，自动补全闭合标记
  - 如果在列表中间截断，截断到上一个完整列表项
  - 如果标题后紧跟内容，截断时保留标题 + 至少一段内容
- 截断后追加：`（文档内容过长，已截断）`

### 7.7 未来扩展（P2）

| 阶段 | 方案 | 说明 |
|------|------|------|
| **阶段一（当前）** | 关键词匹配 | 轻量、无外部依赖、保护隐私 |
| **阶段二（未来）** | 本地向量检索 | 使用本地 embedding 模型（如 `all-MiniLM-L6-v2`），向量数据库存储于本地 Vault 目录 |

---

## 8. 与现有文档的关联

| 文档 | 关联内容 |
|------|---------|
| 13_用户数据边界 | API 密钥 `critical` 级别加密存储；AI 功能开关状态存储位置；敏感数据分级规则 |
| 24_矛盾冲突 | AI 功能默认禁用；风险告知；本地模型（Ollama）推荐 |
| 07_IPC 接口 | `llm_chat`（模式B统一命令）、`llm_send_message_stream`（内部降级保留）、`llm_get_config`、`llm_set_config`、`llm_test_provider`、`llm_get_stats`、`llm_reset_stats`、`llm_find_guides` |
| 10_跨平台视觉规范 | 设置页 LLM 配置区域 UI 风格；侧边栏 AI 对话入口 |
| 08_对象与模板规范 | AI 智能填充功能调用的对象属性接口 |

---

## 9. AI 对话页面规范（LlmChatPage）

> **前置文档**：本章节需先后阅读 §4（设置页入口与风险告知）、§2（Provider 模型）、§6（系统提示词）、§7（帮助文档）再阅读本节。
> **功能定位**：AI 对话是 SoloSoul 的附加扩展功能，**不影响核心功能使用**。核心功能完全离线本地优先，AI 功能仅为用户提供便利。

AI 对话页面的主要能力：
1. **AI 问答**：通过配置 LLM Provider，直接在软件中与 AI 对话
2. **软件信息提供**：回答关于 SoloSoul 的功能、理念、定位等问题（通过系统提示词 Section 2）
3. **帮助与指南**：提供软件的使用文档，回答用户关于各种功能的使用问题（通过帮助文档检索注入）
4. **用户数据查询（需授权）**：在用户授权后，有限度查询对话统计、Token 用量等信息（通过系统提示词 Section 6），以及授权范围内的对象数据

### 9.1 页面布局

```
┌──────────────────────────────────────────────────────┐
│ ← 返回（或侧边栏）    AI 对话              [IconSettings]  │  ← AppBar
├─────────────────┬────────────────────────────────────┤
│                 │                                    │
│  [+ 新建对话]   │  ┌─ 消息气泡（AI 回复）─────────┐  │
│                 │  │  2026-06-07 14:30:25          │  │
│ ────────       │  │  ┌────────────────────────┐  │  │
│                 │  │  │ AI 回复内容...         │  │  │
│ 对话 1          │  │  └────────────────────────┘  │  │
│ 2026-06-07     │  │  [IconCopy]                   │  │
│                 │  ├───────────────────────────────┤  │
│ 对话 2          │  │  ┌─ 消息气泡（用户输入）─────┐  │
│ 2026-06-07     │  │  │  2026-06-07 14:30:20      │  │
│                 │  │  │  ┌────────────────────┐  │  │
│ 对话 3          │  │  │  │ 用户输入内容...    │  │  │
│ 2026-06-06     │  │  │  └────────────────────┘  │  │
│                 │  │  └───────────────────────────┘  │
│                 │  │  [IconCopy]                      │
│                 │  │                                  │
│                 │  ├──────────────────────────────────┤
│                 │  │  [OpenAI · gpt-4o · 云端]        │  ← 模型信息栏
│                 │  │  [输入消息...            ] [→ 发送]│  ← 输入区域
│                 └──────────────────────────────────────┘
│                                                         │
│         对话列表（侧边栏）          消息区                 │
└──────────────────────────────────────────────────────────┘
```

#### 9.1.1 整体结构

| 区域 | 说明 |
|------|------|
| **AppBar** | 标题"AI 对话"；右上角"设置"按钮（齿轮图标），点击跳转 LLM 配置页面（§4） |
| **对话侧边栏** | 左侧窄列，显示对话列表 + 新建对话按钮；可通过拖拽或折叠按钮调整宽度 |
| **消息区** | 右侧主区域，展示当前对话的消息列表；底部固定输入栏 |

#### 9.1.2 对话侧边栏

```
┌──────────────────────┐
│  [+ 新建对话]         │
│ ──────────────────── │
│                       │
│  对话 1               │  ← 按最后更新时间倒序
│  2026-06-07 14:30     │
│                       │
│  对话 2               │
│  2026-06-07 10:15     │
│                       │
│  对话 3               │
│  2026-06-06 20:00     │
│                       │
│  ...                  │
│                       │
│ ──────────────────── │
│  [IconTrash]          │  ← 回收站入口，始终位于侧边栏最底部
└──────────────────────┘
```

| 元素 | 规范 |
|------|------|
| **新建对话按钮** | 顶部 `[+ 新建对话]`，点击后创建**临时对话**，进入空消息区；用户输入第一条消息并发送后才正式建立对话 |
| **对话列表** | 每项显示对话名称 + 最后更新时间 |
| **排序** | 按最后更新时间倒序，越新的对话排在最上方 |
| **对话命名** | 自动命名：用户发送第一条消息后，根据用户输入内容自动生成对话名称；用户可随时手动重命名（双击名称或右键菜单"重命名"） |
| **对话时间戳** | 显示对话的最后更新时间，格式："刚刚"/"X 分钟前"/"X 小时前"/具体日期 |
| **多对话支持** | 无上限，所有对话保存在本地 |
| **回收站入口** | 侧边栏最底部，显示 `[IconTrash]`（垃圾桶图标），点击展开回收站面板，展示已软删除的对话列表 |
| **回收站开关** | 点击回收站入口切换展开/收起；展开时侧边栏下方显示回收站对话列表，不影响上方正常对话列表 |

### 9.2 消息区规范

#### 9.2.1 消息气泡

| 属性 | 用户消息 | AI 回复 |
|------|---------|--------|
| **对齐** | 右侧 | 左侧 |
| **背景** | `--accent-primary`（主题色），白色文字 | `--bg-elevated`，`--text-primary` 文字 |
| **圆角** | 16px 16px 4px 16px（右上方圆角较小） | 16px 16px 16px 4px（左上方圆角较小） |
| **最大宽度** | 70% 消息区宽度 | 85% 消息区宽度（代码块可稍宽） |
| **排版** | 正文样式，无代码高亮 | 支持 Markdown 渲染（正文、代码块、列表、表格、引用） |
| **时间戳** | 气泡右上角/下方，Caption（11px），`--text-tertiary` | 同左 |
| **加载态** | — | 显示打字动画（跳动圆点），表示流式响应进行中 |
| **空态** | — | `showHintButton={false}` |

> **参考 UI**：具体消息气泡、输入框、侧边栏的视觉风格可参考 Claude Code Desktop、ChatGPT Web、Codex 等主流 AI 对话软件，保持一致的用户预期。

#### 9.2.2 时间戳与复制

| 功能 | 规范 |
|------|------|
| **时间戳** | 每条消息（包括用户输入和 AI 回复）均附带具体时间戳，格式 `YYYY-MM-DD HH:mm:ss`；同一批连续消息可省略重复时间戳，但切换对话后恢复完整显示 |
| **复制按钮** | 每条消息气泡下方/右侧有 `[IconCopy]` 文字按钮（透明背景，hover 显示）；点击后复制该条消息的**全部文本内容**到剪贴板；复制成功后短暂显示"已复制"反馈（1.5 秒后自动恢复） |
| **复制范围** | 用户消息：复制用户输入的纯文本；AI 回复：复制渲染后的纯文本（去除 Markdown 标记） |

#### 9.2.3 输入区域

输入区域固定于消息区底部，不随消息滚动：

```
┌──────────────────────────────────────────────┐
│ [OpenAI · gpt-4o · 云端 · [IconOnline] 在线]           │  ← 模型信息栏
│ [输入消息...                         ] [→ 发送]│  ← 输入框 + 发送按钮
└──────────────────────────────────────────────┘
```

| 元素 | 规范 |
|------|------|
| **模型信息栏** | 位于输入框上方，显示当前使用的模型详细信息：`{Provider名称} · {模型名称} · {类型} · {在线状态}`；类型分为"云端"和"本地"；本地模型（Ollama）显示绿色标签，云端模型显示蓝色标签；在线状态紧跟在类型之后，用分隔符 `·` 连接 |
| **在线状态指示** | `[IconOnline] 在线`（绿色文字+绿色圆点 SVG 图标）表示模型/网络连接可用；`[IconOffline] 离线`（红色文字+红色圆点 SVG 图标）表示连接不可用；刷新/重试触发重新检测连接状态；连接检测通过 `llm_test_provider` 等轻量探针实现 |
| **输入框** | 多行文本输入框（支持 Shift+Enter 换行），空状态显示 placeholder"输入消息..." |
| **发送按钮** | 右侧圆形/圆角按钮，箭头图标；输入框为空时禁用（灰色），有内容时可用（主题色）；快捷键 Enter 发送；模型离线时按钮置灰并 tooltip"模型离线，无法发送" |
| **加载态** | 等待 AI 回复期间，输入框锁定（`disabled`），发送按钮显示旋转加载指示器 |
| **流式提示** | AI 回复过程中输入框下方显示"正在生成..."或打字动画 |

### 9.3 对话与命名

#### 9.3.1 对话生命周期

```
用户点击 [+ 新建对话] → 创建临时对话（ID 已生成，但不可见）
         ↓
用户输入第一条消息并点击发送 → 对话正式持久化到本地存储
         ↓  ↓
AI 开始思考/输出    对话已持久化保存（即使后续报错，已有内容亦不丢失）
         ↓
用户可随时切换页面 → 对话在后台继续执行，不受用户导航影响
         ↓
AI 输出结束（成功或报错） → 最终对话记录持久化存档
         ↓
用户切换回对话页 → 可查看完整的最终结果（或完整报错信息）
         ↓
后端根据用户输入自动生成对话名称（使用 LLM 总结或截取前 X 字）
         ↓
用户可手动重命名 → 重命名后覆盖自动名称
```

**后台持久化规则**：

| 规则 | 说明 |
|------|------|
| **发送即持久化** | 用户点击发送后，对话立即持久化（包括用户消息和对话元数据），不等 AI 返回。用户第一条消息发送即代表临时对话→正式对话 |
| **页面切换不中断** | 用户发送消息后，无论 AI 是否正在返回，用户可自由切换页面（回到主页、查看设置等）。对话在后台持续运行，不受前端导航影响 |
| **切换后恢复查看** | 用户切换回对话页面后，自动显示当前对话的最新内容。若 AI 仍在运行中，继续显示流式输出；若已完成，显示最终结果 |
| **报错也保存** | 如果 AI 调用过程中发生错误（网络断开、API 错误、超时等），错误信息作为 AI 回复写入对话记录并持久化。错误信息显示在 AI 回复气泡中（左侧对齐，红色/橙色背景区分正常回复） |
| **完整存档** | 无论成功或失败，整条对话（含用户输入、AI 部分输出、最终状态或报错信息）全部持久化存档，不会因为页面切换或对话切换而丢失任何数据 |

| 状态 | 行为 |
|------|------|
| **临时对话（未发送）** | 对话在侧边栏显示为"新对话"，无持久化；用户切换对话时，若临时对话无内容则丢弃 |
| **发送后（AI 运行中）** | 对话已持久化；用户切换页面不中断 AI 后台运行；切换回来后继续显示流式输出 |
| **发送后（AI 已完成）** | 对话已完整持久化；用户可随时查看最终结果 |
| **发送后（AI 报错）** | 报错信息作为 AI 回复写入对话并持久化；用户可查看完整错误内容 |
| **重命名** | 双击对话名称，进入编辑模式；或右键菜单 → "重命名" |
| **删除对话** | 软删除：对话移至 AI 对话回收站，可从回收站恢复或永久删除（详见 §9.3.3） |

#### 9.3.2 自动命名规范

- 用户发送首条消息后，取消息内容前 30 个字符作为默认对话名称
- 若有 LLM 可用（且用户同意），可由 LLM 总结成简短标题（不超过 20 字）
- 自动命名后，对话名称后缀 `(自动)` 标记，用户手动重命名后移除标记

### 9.3.3 对话回收站

AI 对话拥有独立的回收站系统，专用于对话的软删除和永久删除，与全局回收站（文档 15）隔离。

#### 回收站入口与布局

```
┌─ AI 对话侧边栏 ───────────────────────────┐
│  [+ 新建对话]                               │
│  ────────────────────────────────────       │
│                                             │
│  对话 1                                     │
│  对话 2                                     │
│  ...                                        │
│                                             │
│  ────────────────────────────────────       │
│  [IconTrash] ← 点击展开 / 收起              │
│  ────────────────────────────────────       │
│  ┌ 回收站（展开后）──────────────────────┐  │
│  │                                        │  │
│  │  已删除对话 1    [IconRestore] [IconDeleteForever] │  │
│  │  2026-06-07 14:30                      │  │
│  │                                        │  │
│  │  已删除对话 2    [IconRestore] [IconDeleteForever] │  │
│  │  2026-06-06 20:00                      │  │
│  │                                        │  │
│  └────────────────────────────────────────┘  │
└──────────────────────────────────────────────┘
```

| 元素 | 规范 |
|------|------|
| **回收站入口** | 侧边栏最底部固定显示 `[IconTrash]`，点击展开/折叠回收站面板 |
| **回收站位置** | 始终位于侧边栏最下方，与正常对话列表用分隔线隔开 |
| **回收站列表** | 展开后显示已软删除的对话列表，每项显示对话名称 + 软删除时的时间戳 |
| **无数据状态** | 回收站为空时展开后显示"回收站为空"（灰色文字，居中） |

#### 软删除流程

```
侧边栏对话项 → 右键菜单/悬停删除按钮 → 确认提示 → 对话移至回收站
                                                              ↓
对话从正常列表消失，出现在回收站列表中（更新排序）
对话内容完整保留（不可编辑，仅可查看/恢复/永久删除）
```

| 规则 | 说明 |
|------|------|
| **确认提示** | 点击删除后弹出确认对话框"确定要删除此对话？删除后可从回收站恢复"；确认后执行软删除 |
| **数据保留** | 软删除后对话的全部消息内容完整保留，仅标记删除时间戳（`deleted_at`），不删除任何数据 |
| **编辑限制** | 回收站中的对话不可编辑（不可发送新消息、不可重命名），仅可查看内容、恢复或永久删除 |
| **不进入全局回收站** | AI 对话回收站独立运行，与软件全局回收站（文档 15）完全隔离；全局回收站不显示 AI 对话 |

#### 查看已删除对话

| 交互 | 行为 |
|------|------|
| **点击回收站中的对话** | 以**卡片形式悬浮**在当前页面顶部/中央展示历史对话内容 |
| **悬浮卡片** | 卡片尺寸为 600×400（可拖拽调整），背景为 `--bg-elevated`，带投影 `--shadow-lg`，圆角 16px；卡片可拖拽移动位置 |
| **卡片内容** | 只读展示对话历史（用户消息 + AI 回复），时间戳、复制按钮等元素与正常对话一致，但输入框和发送按钮隐藏 |
| **关闭卡片** | 卡片右上角关闭按钮（X 图标）；点击卡片外部区域也可关闭 |
| **多卡片** | 同一时间仅可打开一个已删除对话的悬浮卡片；打开新的时自动关闭旧的 |
| **无数据** | 回收站为空时展开面板显示"回收站为空" |

#### 恢复流程

| 交互 | 行为 |
|------|------|
| **恢复按钮** | 回收站中每条对话右侧显示 `[IconRestore]` 按钮 |
| **恢复操作** | 点击恢复后，对话从回收站移出，恢复到侧边栏正常对话列表 |
| **位置规则** | 恢复后的对话**按软删除前的最后更新时间戳**插入到正常列表的对应位置（即恢复到删除前的时间顺序位置，而非恢复时刻的顶部） |
| **恢复后状态** | 恢复后对话可正常编辑、发送新消息、重命名，与删除前一致 |
| **已删除标记** | 恢复后 `deleted_at` 标记清除，不影响后续交互 |

#### 永久删除

| 交互 | 行为 |
|------|------|
| **永久删除按钮** | 回收站中每条对话右侧显示 `[IconDeleteForever]` 按钮 |
| **二次确认** | 点击后弹出确认对话框"确定要永久删除此对话？此操作无法撤销。"；确认后执行永久删除 |
| **删除效果** | 对话及其全部消息数据从本地存储中**彻底清除**，不可恢复 |
| **成功反馈** | 删除成功后显示 Toast 提示"对话已永久删除" |

#### 回收站状态总结

| 操作 | 数据影响 | 用户可见性 |
|------|---------|-----------|
| 软删除 | 标记 `deleted_at` 时间戳，数据完整保留 | 从正常列表移至回收站 |
| 恢复 | 清除 `deleted_at` 标记 | 回到正常列表删除前位置 |
| 永久删除 | 数据从本地存储彻底清除 | 从回收站消失 |
| 查看 | 无数据影响 | 悬浮卡片只读展示 |

### 9.4 功能边界与非核心定位

> **关键设计决策**：AI 对话是 SoloSoul 的附加扩展，不是核心功能。

| 场景 | 行为 |
|------|------|
| **无网络环境（云端 Provider）** | 显示提示"当前模型需要网络连接，请切换到本地模型或检查网络"；核心功能不受影响 |
| **Ollama 本地模型** | 完全离线可用，与核心功能一致 |
| **AI 功能未开启** | 页面显示"AI 功能未开启，请前往设置开启"（指向 LLM 配置页）；核心功能一切正常 |
| **LLM 未配置** | 显示"请先配置 LLM 服务商"并提供"前往配置"按钮 |
| **API 调用失败** | 消息中显示错误提示，不阻塞页面其他操作 |
| **用户关闭 AI 功能** | 对话页面不可访问（路由跳转到设置页）；已有对话数据保留不变，再次开启后可继续使用 |

### 9.5 授权与数据边界

| 数据类别 | 授权要求 | 敏感级别 | 说明 |
|---------|---------|---------|------|
| **对话内容** | 始终由用户触发发送，不自动发送任何数据 | — | 每条消息由用户手动输入/确认后发送 |
| **Token 用量/对话统计** | 默认可直接查询 | — | 无用户敏感信息，通过系统提示词 Section 6 注入 |
| **软件信息** | 默认可直接查询 | — | 版本、平台等，通过系统提示词 Section 2 注入 |
| **用户公开对象数据** | 默认可直接查询 | `public` | 遍历所有对象提取 `public` 级别属性，通过系统提示词 Section 3 注入 |
| **偏好设置** | 默认可直接查询 | `public` | 主题、默认对象类型等，通过系统提示词 Section 4 注入 |
| **对象数据（用户存储的信息）** | 需用户显式授权 | `internal`+ | AI 只能查询用户明确授权的对象范围；授权在对话页内以对话框形式确认；授权记录写入审计日志 |
| **用户身份信息** | 不允许 | `private`+ | AI 默认不知道用户是谁，也不主动询问 |
| **敏感/受限/关键数据** | 不允许 | `sensitive`+ | 系统提示词绝不包含此类数据；AI 被明确告知无法直接访问 |

### 9.6 设置页入口

从对话页右上角齿轮图标点击进入 LLM 配置页（§4），或从设置 → AI 与智能助手 → AI 对话开关 + Provider 选择进入。

### 9.7 实现补充（LlmChatPage 实际组件结构）

以下实现细节基于 `LlmChatPage` 的实际代码落地规范，补充 §9.1-§9.3 中蓝图型描述。

#### 9.7.1 组件树

```
LlmChatPage
├── AppShell
│   ├── ConversationSidebar                    ← 左侧对话列表
│   │   ├── 新建对话按钮
│   │   ├── 对话列表（按最后更新时间倒序）
│   │   └── 回收站入口（TrashConversationCard）
│   └── MessageArea                             ← 右侧主区域
│       ├── ChatMessageList                    ← 消息列表（含滚动）
│       │   └── ChatMessageBubble              ← 每条消息的气泡
│       └── ChatInputBar                       ← 底部固定输入栏
├── 未配置状态：UnconfiguredHint                ← 无 Provider/API Key 时的引导提示
└── AI 未开启状态：配置引导按钮                   ← 跳转 LLM 设置页
```

#### 9.7.2 未配置引导（UnconfiguredHint）

当 AI 功能未开启或 LLM 未配置时，页面不显示对话 UI，改为：

| 状态 | 展示 |
|------|------|
| **AI 功能未开启** | 提示"AI 功能未开启，请前往设置开启"，提供"前往配置"按钮（跳转 `/settings/ai`） |
| **LLM 未配置** | 显示 `UnconfiguredHint` 组件，引导用户配置 Provider |

#### 9.7.3 对话侧边栏（ConversationSidebar）

实际实现采用独立组件，包含：

| 区域 | 组件 | 说明 |
|------|------|------|
| 新建对话 | 顶部按钮 | 点击创建临时对话，光标聚焦输入框 |
| 对话列表 | 可滚动列表 | 每项显示对话名称 + 时间戳，当前活跃项高亮 |
| 回收站 | 侧边栏底部 | 点击展开 `TrashConversationCard`，展示已软删除对话 |

#### 9.7.4 消息气泡（ChatMessageBubble）

每条消息的实际渲染规则：

| 角色 | 对齐 | 样式 |
|------|------|------|
| `user` | 右侧 | `background: var(--accent-primary)`，白色文字 |
| `assistant` | 左侧 | `background: var(--bg-elevated)`，`var(--text-primary)` 文字，支持 Markdown 渲染 |
| `system` | 居中 | 灰色小字，用于提示信息（如"AI 正在思考..."） |

- 每条消息带时间戳（气泡底部，`fontSize: 11`）
- 复制按钮：悬停时出现在气泡右下方
- 加载态：AI 生成中显示打字动画（跳动圆点）

#### 9.7.5 输入栏（ChatInputBar）

固定于 MessageArea 底部：

| 元素 | 规范 |
|------|------|
| 模型信息 | 输入框上方一行，显示当前 `Provider名称 · 模型名称 · 类型标签 · 在线状态` |
| 输入框 | 多行 textarea，支持 Shift+Enter 换行，Enter 发送 |
| 发送按钮 | 右侧圆形按钮，输入为空时禁用（灰色） |
| 加载态 | 等待 AI 回复时，输入框 disabled，发送按钮显示 Loader 动画 |
| 离线提示 | 模型离线时输入框置灰并 tooltip"模型离线，无法发送" |

#### 9.7.6 状态管理

LLM 对话使用 `useLlmChat` 自定义 hook 管理状态，核心逻辑包括：
- 对话 CRUD（创建、切换、重命名、软删除、恢复、永久删除）
- 消息发送与 SSE 流式接收（通过 Zustand Store 层 `llmStore` 管理流订阅）
- Provider 配置检查
- 对话持久化（debounce 保存）

---

## 10. 使用统计

### 10.1 统计维度

| 统计项 | 粒度 | 持久化 | 说明 |
|-------|------|--------|------|
| 推理调用次数 | 会话 + 账户 | ✅ | 每次推理 +1 |
| Prompt Token | 会话 + 账户 | ✅ | 估算值（基于字符数） |
| Completion Token | 会话 + 账户 | ✅ | 估算值（基于字符数） |
| 总 Token | 会话 + 账户 | ✅ | Prompt + Completion |
| 按模型统计 | 账户 | ✅ | 各模型使用占比 |
| 每日统计 | 账户 | ✅ | 按天聚合 |

### 10.2 统计生命周期

| 级别 | 生命周期 | 持久化时机 |
|------|---------|-----------|
| **会话级** | 当前 App 生命周期内累计 | 不独立持久化，作为内存统计 |
| **账户级** | 跨会话持久化 | 见下表 |

**账户级统计持久化策略（防丢失）**：

| 时机 | 行为 | 原因 |
|------|------|------|
| **每次推理完成后** | 内存累计，**延迟 30 秒 debounce 保存**到 Vault | 确保即使崩溃也只丢失最近 30 秒数据 |
| **对话切换时** | 立即保存当前会话统计 | 会话粒度切换 |
| **账户切换时** | 立即保存旧账户，加载新账户 | 账户隔离 |
| **Vault 锁定前** | 立即保存 | 安全退出 |
| **App 正常退出时** | 尝试保存（best effort） | 额外保险 |

> **注意**："App 退出时"不可靠（用户可能强制关闭、系统崩溃），因此**每次推理后的 debounce 保存是主要持久化机制**。

**账户切换流程**：
1. 保存旧账户统计到 Vault
2. 重置内存统计
3. 加载新账户统计到内存

### 10.3 Token 估算

无精确 tokenizer 时的近似方案：

```rust
fn estimate_tokens(text: &str) -> u64 {
    // 保守估算：所有字符统一按 1 token / 字符计算
    // 实际 Token 数通常低于估算值，确保不会因估算错误而截断过多内容
    text.chars().count() as u64
}
```

### 10.4 UI 展示

| 位置 | 展示内容 |
|------|---------|
| 设置页 → AI 与智能助手 | 累计使用次数、总 Token 消耗、当前会话统计 |
| 对话页 → 模型信息栏 hover | 当前会话的调用次数和 Token 数 |

---

## 11. 完成标准

### P0（必须 — LLM 配置与 AI 对话基础）
- [ ] AI 功能默认全部禁用
- [ ] 首次开启任何 AI 功能时强制展示风险告知对话框
- [ ] 支持预置 Provider：OpenAI、Anthropic、Ollama、DeepSeek、阿里云百炼
- [ ] 支持添加任意自定义 Provider（名称 + Base URL + API Key + 模型名称）
- [ ] 自定义 Provider 使用 OpenAI 兼容 API 格式
- [ ] 预置 Provider 的 Base URL 和模型可修改
- [ ] API 密钥使用 `SecurePasswordInput` 组件，始终遮蔽
- [ ] API 密钥 `critical` 级别加密存储，使用后内存擦除
- [ ] 前端不直接调用外部 API，所有请求由 Rust 后端代理
- [ ] 支持 SSE 流式响应（所有 Provider），Stream 订阅在 Zustand Store 层；打字机效果仅作为 Provider 不支持 SSE 时的内部降级
- [ ] 当前活跃 Provider 通过单选按钮切换
- [ ] 每个 AI 功能可独立开关
- [ ] 系统提示词 7 Section 模板实现（AI 身份 / 软件信息 / 用户公开对象数据 / 偏好 / 插件 / 统计 / 行为规范）
- [ ] 上下文注入统一走模式 B（后端构建），模式 A 已废弃（仅作为内部降级保留，不暴露给用户）
- [ ] 隐私分级过滤（仅 `public` 级别数据进入系统提示词）
- [ ] 缓存机制：`public_data_version` 作为缓存键，避免遍历所有对象
- [ ] 长度限制分层：系统提示词 1500 / 帮助文档 800 / 单条历史 2000 / 总上下文 8000
- [ ] 帮助文档检索实现：关键词匹配、中英文停用词过滤、评分（标题+3/关键词+1）、动态阈值、Top-1、多语言回退
- [ ] AI 对话页面左右布局：左侧对话侧边栏 + 右侧消息区 + 底部固定输入栏
- [ ] 新建对话按钮创建临时对话，首条消息发送后正式持久化
- [ ] 用户消息右侧对齐、AI 回复左侧对齐，均附带具体时间戳
- [ ] 每条消息下方有复制按钮，点击复制消息全部内容
- [ ] 输入框上方显示当前模型详细信息（提供商 · 模型名称 · 云端/本地标签）
- [ ] 对话按最后更新时间倒序排列，支持自动命名和手动重命名
- [ ] 右上角设置按钮跳转 LLM 配置页
- [ ] AI 报错信息作为对话记录保存并持久化（红色/橙色气泡）

### P1（重要 — 用户体验完善）
- [ ] Provider 配置支持"测试连接"功能
- [ ] Anthropic Provider 有适配层处理格式转换
- [ ] 模型信息栏显示在线/离线状态（绿色/红色指示），离线时发送按钮置灰
- [ ] 用户发送消息后对话立即持久化，页面切换不中断 AI 后台执行
- [ ] 切换页面后返回可继续查看流式输出或最终结果
- [ ] 无网络时云端 Provider 有友好提示，不阻塞核心功能
- [ ] AI 功能未开启/LLM 未配置时显示引导状态
- [ ] 用户授权后方可查询对象数据，授权记录写入审计日志
- [ ] 对话软删除后移入 AI 回收站，不进入全局回收站
- [ ] 回收站入口位于侧边栏最底部，点击展开/折叠
- [ ] 回收站中对话可点击以悬浮卡片形式查看只读内容
- [ ] 回收站中每条对话有恢复按钮（恢复到删除前时间位置）和永久删除按钮（二次确认后清除）
- [ ] 回收站为空时显示"回收站为空"
- [ ] 使用统计追踪（会话级 + 账户级）及持久化
- [ ] Token 用量估算与展示
- [ ] 系统提示词注入开关（高级用户可关闭上下文注入）

### P2（增强）
- [ ] 审计日志记录每次 AI 调用（provider/model/timestamp，不记录用户输入）
- [ ] 帮助文档向量检索（本地 embedding 模型 + 向量数据库）
- [ ] 上下文压缩（长对话时自动压缩历史消息）
- [ ] 智能截断（基于语义重要性而非简单按行截断）
- [ ] 模型特定 Token 估算系数表（GPT-4 / Claude / Llama 分别配置）

## 10. 实施记录：模式 B 与 Phase 2 落地（2026-08）

> **历史**：本节内容源自原 `28_Tauri_LLM_模式B与Phase2实施计划.md`（已并入本文档）。原文包含逐阶段实施细节、代码草图与验收标准，需要回溯历史细节可在 git 历史中查阅原文档。
> **状态**：Phase 2 全部 6 个子阶段**已实施完成（2026-08）**。

### 10.1 实施阶段总览

| 阶段 | 内容 | 落地位置 | 状态 |
|------|------|---------|------|
| **2.1** | Rust 端 `LlmContextService`（7 Section 系统提示词构建） | `src-tauri/src/services/llm_context.rs` | ✅ 已实施 |
| **2.2** | 统一聊天命令 `llm_chat`（只传 prompt + history） | `src-tauri/src/commands/llm/` | ✅ 已实施 |
| **2.3** | 真正的 SSE 流式（替代打字机效果，Ollama + 云端 Provider 全量） | `src-tauri/src/commands/llm/chat_http.rs` | ✅ 已实施 |
| **2.4** | `public_data_version` 计数器（Rust 端维护，缓存失效） | Vault `preferences.llmPublicDataVersion` | ✅ 已实施 |
| **2.5** | 统计持久化 30s debounce | `commands/llm/stats` | ✅ 已实施 |
| **2.6** | 移除模式 A 暴露层（前端构建降级为内部保留） | 前端 `useLlmChatCore` / `systemPromptBuilder` | ✅ 已实施 |

### 10.2 关键决策记录

| # | 事项 | 决策 | 理由 |
|---|------|------|------|
| 1 | `public_data_version` 维护位置 | **Rust 端维护** | 与 Vault 数据一致性更好，用户无感，避免前端同步逻辑 |
| 2 | SSE 实现范围 | **全量同步：Ollama + 所有云端 Provider** | 真正的逐 token 流式体验，不牺牲任何 Provider |
| 3 | 默认/暴露模式 | **模式 B 唯一暴露，模式 A 废弃** | 用户不需要选择模式；模式 A 仅作 Provider 不支持 SSE 时的内部降级 |
| 4 | 缓存跨会话持久化 | **是，Vault 持久化**（仅元数据） | 避免冷启动重复构建 |
| 5 | Section 5 插件数据 | 留空预留（TODO） | 插件系统接入后启用 |

### 10.3 验收结果

1. ✅ `llm_chat` 替代前端构建模式，系统提示词完整注入（Rust 端强制过滤非 public 数据）
2. ✅ 缓存命中率达标；统计 debounce 保存，崩溃丢失窗口 ≤ 30 秒
3. ✅ 用户对模式 A/B 无感知，所有 AI 对话统一一致

---

*文档版本：v3.4*
*创建日期：2026-06-05*
*最后更新：2026-08-07（并入 28 实施记录为第 10 章；模式B定稿：废弃模式A用户暴露层、SSE全量同步、Rust端public_data_version、缓存Vault持久化、统计debounce、插件Section留空预留）*
*对应开发阶段：Phase 5（插件与扩展系统）*
*前置依赖：07、13、21、23*
