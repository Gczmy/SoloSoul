# 26 — LLM 配置规范

> **前置阅读**：`13_用户数据边界与加密存储方案.md`、`21_矛盾冲突与待审批事项.md`
> **Manifesto 对齐**：用户主权 | 隐私优先 | 安全默认
> **文档定位**：定义 SoloSoul LLM（大语言模型）集成的配置规范——Provider 管理、API 配置、模型选择、UI 交互。AI 功能默认禁用，用户完全控制。

---

## 1. 设计原则

| 原则 | 说明 |
|------|------|
| **用户主权** | 用户选择用什么模型、哪家服务商、甚至自建 API |
| **默认禁用** | AI 功能默认全部关闭，首次开启需风险告知确认 |
| **不绑定厂商** | 不限制任何特定厂商，支持所有 OpenAI 兼容 API |
| **密钥安全** | API 密钥 `critical` 级别加密存储，使用后内存擦除 |
| **透明可控** | 每次 AI 调用前用户确认发送内容，审计日志记录调用元数据 |

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
| 名称 | 是 | 1-30 字符 | 用户自定义，支持国际化 |
| Base URL | 是 | 有效 HTTP(S) URL | 必须以 `/v1` 结尾（OpenAI 兼容格式） |
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
│  [⚠️] AI 功能涉及数据外传风险              │
│  所有 AI 功能默认关闭，开启前请阅读风险告知  │
│                                          │
│  AI 功能开关                              │
│  ├─ [ ] AI 对话                          │
│  ├─ [ ] 智能填充                          │
│  ├─ [ ] 命令生成                          │
│  └─ [ ] 自然语言搜索                       │
│                                          │
│ ──────────────────────────────────────  │
│                                          │
│  AI 服务商（Provider）                     │
│  ┌────────────────────────────────────┐ │
│  │ ● OpenAI          gpt-4o      [✎] │ │  ← 当前活跃
│  │ ○ Anthropic       claude-3    [✎] │ │
│  │ ○ Ollama(本地)    llama3.1    [✎] │ │
│  │ ○ 我的工作助手    gpt-4o-mini [✎] │ │  ← 自定义
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
│  ⚠️ 启用 AI 功能前请确认                    │
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
│                                          │
│  [✓] 我已了解风险并同意开启               │
│                                          │
│  [  确认开启  ]  [  取消  ]               │
└──────────────────────────────────────────┘
```

- 必须勾选"我已了解风险"才能点击"确认开启"
- 确认后写入审计日志，记录时间戳和开启的功能类型

---

## 5. API 调用流程

### 5.1 后端代理架构

前端**不直接调用**任何外部 LLM API，所有请求由 Rust 后端代理：

```
前端（React）
    ↓ IPC 命令
Rust 后端（Tauri）
    ↓ 读取 Provider 配置 + 解密 API 密钥
HTTP Client（reqwest）
    ↓ 发送请求
外部 LLM API（OpenAI / Anthropic / 自定义 / Ollama）
    ↓ 流式响应
Rust 后端
    ↓ IPC 事件（流式）
前端（逐字显示）
```

**优势**：
- API 密钥从不离开 Rust 后端
- 前端代码不依赖任何特定 LLM SDK
- 统一错误处理和重试逻辑
- 便于审计日志记录

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

// Anthropic 适配层：在 Rust 后端将 OpenAI 格式转换为 Anthropic 格式
// Ollama 适配层：Ollama 已原生支持 OpenAI 兼容 API
// 自定义 Provider：直接使用用户提供的 baseUrl，发送标准 OpenAI 格式
```

> **Anthropic 特殊处理**：Anthropic API 不是原生 OpenAI 兼容格式，Rust 后端需要提供适配层，将 OpenAI 格式的请求转换为 Anthropic 格式（Messages API）。

---

## 6. 与现有文档的关联

| 文档 | 关联内容 |
|------|---------|
| 13_用户数据边界 | API 密钥 `critical` 级别加密存储；AI 功能开关状态存储位置 |
| 21_矛盾冲突 | AI 功能默认禁用；风险告知；本地模型（Ollama）推荐 |
| 07_IPC 接口 | `llm_send_message`（流式）、`llm_get_config`、`llm_set_config`、`llm_test_provider` |
| 09_视觉规范 | 设置页 LLM 配置区域 UI 风格 |
| 25_对象规范 | AI 智能填充功能调用的对象属性接口 |

---

## 7. 完成标准

- [ ] AI 功能默认全部禁用
- [ ] 首次开启任何 AI 功能时强制展示风险告知对话框
- [ ] 支持预置 Provider：OpenAI、Anthropic、Ollama、DeepSeek、阿里云百炼
- [ ] 支持添加任意自定义 Provider（名称 + Base URL + API Key + 模型名称）
- [ ] 自定义 Provider 使用 OpenAI 兼容 API 格式
- [ ] 预置 Provider 的 Base URL 和模型可修改
- [ ] Provider 配置支持"测试连接"功能
- [ ] API 密钥使用 `SecurePasswordInput` 组件，始终遮蔽
- [ ] API 密钥 `critical` 级别加密存储，使用后内存擦除
- [ ] 前端不直接调用外部 API，所有请求由 Rust 后端代理
- [ ] 支持流式响应（逐字显示）
- [ ] Anthropic Provider 有适配层处理格式转换
- [ ] 当前活跃 Provider 通过单选按钮切换
- [ ] 每个 AI 功能可独立开关
- [ ] 审计日志记录每次 AI 调用（provider/model/timestamp，不记录用户输入）

---

*文档版本：v1.0*
*创建日期：2026-06-05*
*最后更新：2026-06-05*
*对应开发阶段：Phase 5（插件与扩展系统）*
*前置依赖：07、13、21*
