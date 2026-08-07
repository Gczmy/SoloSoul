# 28 — Tauri LLM 模式 B 与 Phase 2 实施计划
> **⚠️ 已归档（2026-08-07）**：本文档为 LLM 模式 B / Phase 2 的**历史实施计划**，其结论已并入 `20_LLM配置与AI对话规范.md` 第 10 章「实施记录」。保留作历史细节参考。

> **前置阅读**：`20_LLM配置与AI对话规范.md` v3.1、`archive/Tauri_LLM_双向分析_代码借鉴与文档优化.md`（已归档）、`archive/Flutter_LLM_实现分析与Tauri借鉴建议.md`（已归档）
> **文档状态**：已定稿（v1.1）；**Phase 2 已实施（2026-08）**——模式 B 后端上下文构建 + 真 SSE 流式已落地
> **编写日期**：2026-06-08

---

## 1. 执行摘要

本文档定义 Tauri LLM 功能从**模式 A（前端构建系统提示词）**迁移到**模式 B（后端构建系统提示词）**的完整技术方案与分阶段实施计划。

| 维度 | 模式 A（当前 Phase 1） | 模式 B（Phase 2 目标） |
|------|----------------------|----------------------|
| **系统提示词构建位置** | 前端 `systemPromptBuilder.ts` | Rust 后端 `LlmContextService` |
| **数据来源** | Zustand Store 缓存 | Vault 原生查询 |
| **隐私过滤** | 前端代码（可被绕过） | Rust 端强制过滤，不可绕过 |
| **前端职责** | 构建完整 messages → IPC 发送 | 只发送 prompt + history |
| **IPC 命令** | `llm_send_message_stream`（传完整 messages） | `llm_chat`（只传 prompt + history） |
| **调试友好性** | DevTools 可直接查看完整 prompt | 后端黑盒，需日志输出 |
| **适用场景** | 快速迭代、调试 | 生产环境、高安全需求 |

**模式 A 废弃**：模式 A（前端构建）仅作为 Provider 不支持 SSE 等技术降级场景的内部保留选项，不暴露给用户。所有用户-facing 的 AI 对话统一走模式 B（后端构建），用户无感。

---

## 2. 模式 B 架构设计

### 2.1 数据流

```
用户发送消息
    │
    ▼
前端: invoke('llm_chat', { accountId, conversationId, prompt, history })
    │
    ▼
Rust 后端: LlmContextService::build_context(accountId)
    │   1. 从 Vault 查询所有对象 → 筛选 SensitivityLevel.public
    │   2. 查询偏好设置（theme, language, autoLock 等）
    │   3. 查询已安装插件列表
    │   4. 从 STATS_MAP 获取实时使用统计
    │   5. 组装 7 Section 系统提示词
    │
    ▼
Rust 后端: GuideService::find_relevant_guides(prompt, language)
    │   （复用现有 llm_find_guides 逻辑）
    │
    ▼
Rust 后端: 组装完整 messages
    [0] system: 系统提示词
    [1] system: 帮助文档（如有匹配）
    [2..n] history messages
    [n+1] user: prompt
    │
    ▼
Rust 后端: HTTP Client 发送请求 → 流式 Event 推送
    │
    ▼
前端: Store 层监听 Event，逐字渲染
```

### 2.2 与 Flutter 实现的对齐

Flutter 的 `LlmContextService`（`flutter/lib/core/services/llm/llm_context_service.dart`）已有成熟实现，Tauri Phase 2 直接借鉴：

| Flutter 实现 | Tauri 对应 | 复用程度 |
|-------------|-----------|---------|
| `_extractPublicInfo()` — 对象遍历 + public 过滤 | `build_section3_public_objects()` | 逻辑完全复用，改 Rust 语法 |
| `_collectPreferences()` — 安全设置、语言、快捷操作 | `build_section4_preferences()` | 概念复用，具体字段根据 Tauri 偏好调整 |
| `_collectInstalledPlugins()` — 插件服务查询 | `build_section5_plugins()` | 当前 Tauri 插件系统未完整实现，**先留空预留，加 TODO 注释**，等插件系统上线后接入 |
| `_buildStatsSection()` — 实时统计注入 | `build_section6_stats()` | 直接复用 STATS_MAP 数据 |
| `_buildCacheKey()` — `updated_at` 总和 | `build_cache_key()` | **改进**：改用 `public_data_version` |
| `LlmPromptTemplates.chatSystemPrompt()` 模板 | Rust `format!()` 或 `tera` | 模板内容直接复用 |

---

## 3. Phase 2 分阶段实施计划

### 3.1 Phase 2.1 — Rust 端 LlmContextService（核心基础设施）

**目标**：在 Rust 后端重建 Flutter `LlmContextService` 的全部能力。

**新建文件**：`tauri/src-tauri/src/services/llm_context.rs`

**核心模块设计**：

```rust
// 1. 缓存层（内存 + Vault 跨会话持久化）
static PROMPT_CACHE: Lazy<Mutex<HashMap<String, CachedPrompt>>> = Lazy::new(...);

struct CachedPrompt {
    static_prompt: String,      // Section 1-5（不含实时统计）
    created_at: Instant,
}

// 缓存元数据持久化到 Vault，App 重启后恢复缓存键，避免冷启动重复构建
const PROMPT_CACHE_META_KEY: &str = "llmPromptCacheMeta";

// 2. 构建入口
pub async fn build_context(
    account_id: &str,
    vault: &VaultStore,
    stats: &LlmUsageStats,
    language: &str,
) -> Result<String, String>

// 3. 各 Section 构建器
fn build_section1_identity() -> String;
fn build_section2_software_info() -> String;
async fn build_section3_public_objects(vault: &VaultStore, account_id: &str) -> Result<String, String>;
async fn build_section4_preferences(vault: &VaultStore, account_id: &str) -> Result<String, String>;
async fn build_section5_plugins(vault: &VaultStore) -> Result<String, String>;
fn build_section6_stats(stats: &LlmUsageStats) -> String;
fn build_section7_guidelines() -> String;

// 4. 缓存键
fn build_cache_key(account_id: &str, public_data_version: u64) -> String;
```

**关键技术点**：

| 问题 | 方案 |
|------|------|
| **Vault 中对象数据格式** | Profile JSON 中 `preferences.objects` 存储了对象列表，需要解析并筛选 `sensitivityLevel == "public"` |
| **性能** | 缓存静态部分（Section 1-5），仅实时统计（Section 6）每次重新注入 |
| **缓存失效** | 使用 `public_data_version` 计数器（见 Phase 2.4），避免遍历所有对象计算 `updated_at` 总和 |
| **长度截断** | 优先截断 Section 3（用户公开对象数据），保留 Section 1 和 Section 7 |

**参考 Flutter 实现直接复用的逻辑**：
- 对象按 `typeId` 分组，每类型最多 3 个对象
- 每对象最多 8 个属性，属性值截断至 100 字符
- `typeId` 的 `__preset_` 前缀去除，snake_case 转 Title Case
- 属性 key 的 camelCase/snake_case 转中文标签

---

### 3.2 Phase 2.2 — 统一聊天命令 `llm_chat`

**目标**：前端只需发送用户输入，后端包办一切。

**新增命令**（`tauri/src-tauri/src/commands/llm.rs`）：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatRequest {
    pub account_id: String,
    pub conversation_id: String,
    pub prompt: String,
    pub history: Vec<ChatMessage>,        // 只传 role + content + created_at
    pub include_system_prompt: bool,      // 是否注入系统提示词
    pub include_help_doc: bool,           // 是否检索帮助文档
    pub language: String,                 // 'zh-CN' | 'en-US'
}

#[tauri::command]
pub async fn llm_chat(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    request: ChatRequest,
) -> Result<(), String> {
    // 1. 读取 Provider 配置 + 解密 API Key
    // 2. 如启用，构建系统提示词（LlmContextService::build_context）
    // 3. 如启用，检索帮助文档（复用 llm_find_guides 逻辑）
    // 4. 组装 messages 数组
    // 5. 发送 HTTP 请求
    // 6. 流式 Event 推送（复用现有 llm_send_message_stream 逻辑）
    // 7. 记录统计
}
```

**前端适配**（`LlmChatPage.tsx`）：

```typescript
// 模式 B：极简调用
invoke('llm_chat', {
  accountId,
  conversationId: convId,
  prompt: text,
  history: updatedMessages,
  includeSystemPrompt,
  includeHelpDoc: true,
  language: i18n.language || 'zh-CN',
});
```

**与现有命令的关系**：

| 命令 | 模式 | 状态 |
|------|------|------|
| `llm_send_message_stream` | 模式 A | **保留为内部降级**，仅当 Provider 不支持 SSE 时内部调用，**不暴露给用户** |
| `llm_send_message` | 模式 A（非流式） | **保留**，供内部测试使用 |
| `llm_chat` | 模式 B | **新增**，作为唯一用户-facing 命令 |
| `llm_find_guides` | 独立服务 | **保留**，模式 B 内部调用 |

---

### 3.3 Phase 2.3 — 真正的 SSE 流式（替代打字机效果）

**当前问题**：
- 云端 Provider（OpenAI/Anthropic/DeepSeek）目前是"完整获取响应 → 逐字 grapheme emit"
- 这造成双重延迟：网络请求等待 + 打字机效果等待

**技术方案**：

**Ollama（本地）**：已支持 SSE，使用 `reqwest` 的 SSE 解析：

```rust
use reqwest::Response;
use futures::StreamExt;

let mut stream = resp.bytes_stream();
while let Some(chunk) = stream.next().await {
    let text = parse_sse_chunk(&chunk)?;
    app.emit("llm-stream-chunk", ...)?;
}
```

**云端 Provider**：OpenAI 兼容 API 也支持 SSE（`stream: true`）：

```rust
let body = json!({ "model": model, "messages": messages, "stream": true });
let mut stream = client.post(&url).json(&body).send().await?.bytes_stream();
while let Some(chunk) = stream.next().await {
    // 解析 SSE: data: {...}\n\n
    for line in chunk.lines() {
        if line.starts_with("data: ") {
            let json_str = &line[6..];
            if json_str == "[DONE]" { break; }
            let delta = parse_delta(json_str)?;
            app.emit("llm-stream-chunk", ...)?;
        }
    }
}
```

**内部降级策略**：
- 当 Provider 不支持 SSE 或 SSE 解析失败时，**内部回退**到完整获取后打字机效果
- 此降级对用户不可见，不暴露 `streamMode` 设置选项给用户

---

### 3.4 Phase 2.4 — `public_data_version` 计数器机制

**目标**：解决缓存键性能问题（避免每次发送消息都遍历所有对象求 `updated_at` 总和）。

**实现方案（已决策：Rust 端维护）**

在 Vault Profile JSON 中新增字段：

```json
{
  "preferences": {
    "llmPublicDataVersion": 42
  }
}
```

Rust 端在 `object_create` / `object_update` 命令中检测 `sensitivityLevel == "public"` 的变更，自动 +1。

**决策理由**：
- Rust 端维护与 Vault 数据一致性更好，用户完全无感
- 避免前端 Store 引入额外的同步逻辑和状态管理负担
- `public_data_version` 作为 Vault 数据的一部分，随账户切换自然隔离

**模式 B 缓存键**：

```rust
fn build_cache_key(account_id: &str, public_data_version: u64) -> String {
    format!("{}_{}", account_id, public_data_version)
}
```

---

### 3.5 Phase 2.5 — 统计持久化增强（Debounce 机制）

**当前问题**：
- 统计仅在账户切换 / Vault 锁定 / App 退出时保存
- 崩溃或强制退出会导致数据丢失

**改进方案**：

```rust
// 每次 record_usage 后，启动/重置一个 30 秒 debounce timer
static STATS_SAVE_DEBOUNCE: Lazy<Mutex<HashMap<String, JoinHandle<()>>>> = Lazy::new(...);

pub async fn record_usage(account_id: &str, model: &str, prompt: &str, completion: &str) {
    // 1. 更新内存统计
    // 2. 取消旧的 debounce timer
    // 3. 启动新的 30 秒 timer：到时保存到 Vault
}
```

**保存时机汇总**：

| 时机 | 行为 | 原因 |
|------|------|------|
| 每次推理完成后 | 内存累计，**延迟 30 秒 debounce 保存** | 确保即使崩溃也只丢失最近 30 秒数据 |
| 对话切换时 | 立即保存当前会话统计 | 会话粒度切换 |
| 账户切换时 | 立即保存旧账户，加载新账户 | 账户隔离 |
| Vault 锁定前 | 立即保存 | 安全退出 |
| App 正常退出时 | 尝试保存（best effort） | 额外保险 |

---

### 3.6 Phase 2.6 — 移除模式 A 暴露层

**目标**：清理前端中面向用户的模式 A 逻辑，确保所有用户-facing 的 AI 对话统一走模式 B。

**前端变更清单**：

1. **删除** `LlmConfigPage.tsx` 中的"系统提示词构建模式"切换区域
2. **删除** `systemPromptBuilder.ts` 的导出（或移至 `src/lib/llm/_deprecated/` 作为内部参考）
3. **删除** `guideService.ts` 中对 `llm_find_guides` 的直接调用（模式 B 内部已由后端调用）
4. **简化** `LlmChatPage.tsx` 的 `sendMessage`：

```typescript
const sendMessage = async () => {
  // 唯一用户-facing 路径：模式 B
  await invoke('llm_chat', {
    accountId,
    conversationId: convId,
    prompt: text,
    history: updatedMessages,
    includeSystemPrompt,
    includeHelpDoc: true,
    language: i18n.language || 'zh-CN',
  });
};
```

**保留的内部降级路径**（不暴露给用户）：
- 当 `llm_chat` 检测到 Provider 不支持 SSE 时，内部调用 `llm_send_message_stream`（完整获取后打字机效果）
- 此降级逻辑封装在 Rust 后端，前端无需关心

---

## 4. 实施优先级与依赖关系

```
Phase 2.1 ──→ Phase 2.2 ──→ Phase 2.3
   │              │              │
   │              ▼              │
   │         Phase 2.6          │
   │              │              │
   ▼              ▼              ▼
Phase 2.4    Phase 2.5
（可并行）   （可并行）
```

| 阶段 | 预估工作量 | 依赖 | 风险 |
|------|-----------|------|------|
| **2.1 LlmContextService** | 3-4 天 | 无 | 中：Vault 中对象数据解析格式需对齐 |
| **2.2 `llm_chat` 命令** | 1-2 天 | 2.1 | 低：主要整合现有逻辑 |
| **2.3 SSE 流式** | 2-3 天 | 2.2 | 中：SSE 解析边界情况多 |
| **2.4 `public_data_version`** | 1 天 | 无 | 低：纯计数器逻辑 |
| **2.5 统计 Debounce** | 0.5 天 | 无 | 低：已有保存逻辑，加 timer |
| **2.6 移除模式 A 暴露层** | 0.5 天 | 2.2 | 低：删除代码 + 简化调用 |

**总计**：约 **8-10 天** 开发时间（单人全职）。

---

## 5. 验收标准

Phase 2 完成时，应满足：

1. **功能**：`llm_chat` 命令成功替代前端构建模式，系统提示词完整注入
2. **隐私**：Rust 端强制过滤，任何非 `public` 级别数据不会进入提示词
3. **性能**：缓存命中率 > 90%（连续对话场景），单次构建 < 50ms
4. **流式**：SSE 模式下，首个 token 到达前端延迟 < 100ms（本地 Ollama）
5. **稳定性**：统计 debounce 保存，强制退出丢失数据 < 1 次会话
6. **用户体验**：用户对模式 A/B 的存在完全无感知，所有 AI 对话统一表现一致

---

## 6. 已决策事项

所有选项已根据"产品效果优先、用户无感、安全至上"方针决策完毕：

| # | 事项 | 决策 | 理由 |
|---|------|------|------|
| 1 | **Phase 2.4 `public_data_version` 维护位置** | **Option B：Rust 端维护** | 与 Vault 数据一致性更好，用户完全无感，避免前端引入额外同步逻辑 |
| 2 | **Phase 2.3 SSE 实现范围** | **全量同步：Ollama + 所有云端 Provider** | 产品效果最好，真正的逐 token 流式体验；不牺牲任何 Provider |
| 3 | **Phase 2.6 默认/暴露模式** | **模式 B 唯一暴露，模式 A 完全废弃** | 用户不需要决定模式，不需要调试；模式 A 仅作为 Provider 不支持 SSE 时的内部降级 |
| 4 | **缓存跨会话持久化** | **是，Vault 持久化** | 用户体验至上，避免每次 App 启动后的冷启动构建；数据风险极低 |
| 5 | **Section 5 插件数据** | **留空预留，加 TODO 注释** | 插件系统未上线，不阻塞 LLM 功能；等插件系统就绪后接入 |

---

*文档版本：v1.0 草案*
*对应代码基线：2026-06-08*
*对应规范：`20_LLM配置与AI对话规范.md` v3.1*
