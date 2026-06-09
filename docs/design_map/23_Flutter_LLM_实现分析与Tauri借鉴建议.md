# Flutter LLM 实现分析与 Tauri 借鉴建议

> **文档目的**：本文档通过分析 Flutter 客户端已实现且运行良好的 LLM/AI 对话功能，提取可复用的设计模式、架构决策和隐私保护机制，为 Tauri 端实现相同功能提供参考。
>
> **分析范围**：Flutter `lib/core/services/llm/` 和 `lib/presentation/providers/llm/` 目录下的核心代码。
> **文档版本**：v1.0（基于 2026-06-08 代码快照）

---

## 1. 系统提示词（System Prompt）设计

### 1.1 Flutter 实现概览

Flutter 端的系统提示词并非简单的一句话，而是一个**多 Section 结构化的提示词模板**，包含以下组成部分：

```
【Section 1: AI 身份定义】
你是 SoloSoul（独灵）的 AI 助手 Solon，由 SoloSoul 团队开发。
你是用户的个人智能助手，了解用户的个人信息（仅限用户主动分享的部分）。
你的回答应当简洁、准确、有帮助。

【Section 2: 软件信息】
当前 SoloSoul 版本：{appVersion}
平台：{platform}
界面语言：{language}

【Section 3: 用户公开档案】
用户主动公开的个人信息（仅包含公开级别的字段）：
{userPublicInfo}  ← 仅包含 SensitivityLevel.public 的数据

【Section 4: 偏好设置】
{preferences}  ← 例如默认对象类型、主题等

【Section 5: 已安装插件】
{installedPlugins}  ← 插件名称列表

【Section 6: 使用统计】
{usageStats}  ← 累计使用次数、Token 消耗等

【Section 7: 行为规范】
1. 使用与用户提问相同的语言回答
2. 区分"插件"（功能扩展）和"对象"（用户数据）
3. 敏感/受限/关键数据需要重新验证密码，无法直接访问
4. 无法访问用户本地数据时，建议用户手动查找而非编造
5. 不泄露用户数据给插件或外部服务
6. 用户询问功能使用方法时，基于软件信息回答
```

### 1.2 关键设计决策

| 决策点 | Flutter 做法 | 说明 |
|--------|-------------|------|
| **注入位置** | 作为 `system` 角色的消息放入 messages 数组首位 | OpenAI/Anthropic 均支持，兼容性最佳 |
| **动态构建** | 每次发送消息前异步构建 | 通过 `LlmContextService` 实时获取最新用户数据 |
| **隐私边界** | 仅包含 `SensitivityLevel.public` 的字段 | 绝不暴露 internal/private/sensitive/restricted/critical 级别数据 |
| **长度控制** | 上下文 + 历史对话总量受 Token 估算限制 | 超出时截断用户数据部分，优先保留系统提示词 |
| **持久化** | 系统提示词**不进入**消息历史，也不持久化 | 每次发送前动态拼接，确保数据始终最新 |

### 1.3 注入流程（时序图）

```
用户发送消息
    │
    ▼
streamChat(prompt, history, includeSystemPrompt=true)
    │
    ▼
_streamChatWithContext() ──► LlmContextService.buildContext(accountId)
    │                              │
    │                              ▼
    │                         1. 获取用户公开资料
    │                         2. 获取偏好设置
    │                         3. 获取已安装插件列表
    │                         4. 获取使用统计（实时）
    │                         5. 组装 systemPrompt 字符串
    │                              │
    │◄─────────────────────────────┘
    ▼
UserGuideService.findRelevantGuides(prompt, language)
    │
    ▼
构建 messages 数组：
  [0] system: context.systemPrompt
  [1] system: docPrompt（如有匹配指南）
  [2..n] history messages
  [n+1] user: prompt
    │
    ▼
调用推理服务（云端或本地）
```

### 1.4 对 Tauri 的借鉴建议

- **必须实现多 Section 系统提示词**：参考 Flutter 的 7 个 Section，定义 Tauri 端的标准系统提示词模板
- **明确隐私边界**：系统提示词只能包含用户**主动公开**的信息，绝不包含敏感数据
- **动态注入而非持久化**：系统提示词不应存储在对话历史中，每次发送前重新构建
- **支持开关**：提供 `includeSystemPrompt` 参数，允许高级用户关闭上下文注入

---

## 2. 上下文注入机制（Context Injection）

### 2.1 Flutter 实现：`LlmContextService`

`LlmContextService` 是上下文注入的核心服务，设计目标是**隐私优先 + 性能优化**。

#### 2.1.1 隐私优先的上下文构建

```dart
Future<BuildContextResult> buildContext({
  required String accountId,
  required LlmModelManager modelManager,
}) async {
  // 1. 构建缓存键（决定是否需要重新生成）
  final cacheKey = await _buildCacheKey(accountId);
  
  // 2. 检查缓存
  final cached = _promptCache[cacheKey];
  if (cached != null) {
    // 缓存命中：复用静态部分，只追加实时统计
    final stats = modelManager.buildStatsSnapshot();
    final systemPrompt = _injectRealTimeStats(cached.systemPrompt, stats);
    return BuildContextResult(systemPrompt: systemPrompt, wasCached: true, ...);
  }
  
  // 3. 缓存未命中：重新构建
  final profileData = await _collectPublicProfileData(accountId);
  final preferences = await _collectPreferences(accountId);
  final plugins = await _collectInstalledPlugins();
  final stats = modelManager.buildStatsSnapshot();
  
  // 4. 组装系统提示词
  final systemPrompt = LlmPromptTemplate.chatSystemPrompt(
    appVersion: ..., platform: ..., language: ...,
    userPublicInfo: profileData,
    preferences: preferences,
    installedPlugins: plugins,
    usageStats: stats.toMap(),
  );
  
  // 5. 缓存静态部分
  _promptCache[cacheKey] = CachedPrompt(...);
  
  return BuildContextResult(systemPrompt: systemPrompt, wasCached: false, ...);
}
```

#### 2.1.2 缓存策略

| 缓存维度 | 实现方式 |
|---------|---------|
| **缓存键** | `accountId + 对象数量 + 所有对象 updatedAt 的总和` |
| **缓存内容** | 静态部分（用户资料、偏好、插件列表） |
| **不缓存内容** | 实时统计（使用次数、Token 数）——每次注入时动态追加 |
| **缓存位置** | 内存（Map），随 App 生命周期存在 |
| **失效条件** | 缓存键变化（用户数据更新）或切换账户 |

#### 2.1.3 长度限制与截断策略

```dart
static const int _maxObjectsPerType = 3;      // 每类型最多 3 个对象
static const int _maxPropertiesPerObject = 8; // 每对象最多 8 个属性
static const int _maxValueLength = 100;       // 每个值最多 100 字符
static const int _maxTotalChars = 2000;       // 总提示词最多 2000 字符
```

**Token 估算规则**：
- 中文字符：≈ 1 token / 字符
- 拉丁字符：≈ 0.75 tokens / 字符
- 混合内容：加权平均

**截断策略**：
1. 优先截断用户数据部分（Section 3 用户公开档案）
2. 保留系统提示词核心部分（Section 1 AI 身份、Section 7 行为规范）
3. 在接近限制时按**行边界**截断（不切断句子）
4. 截断后追加提示：`（上下文过长，部分内容已省略）`

### 2.2 对 Tauri 的借鉴建议

- **必须实现缓存机制**：用户数据（Profile）不会频繁变更，静态部分应当缓存，避免每次对话都重新查询
- **明确长度限制**：定义清晰的 Token/字符上限，超出时智能截断
- **Token 估算**：在没有精确 tokenizer 的情况下，使用字符数近似估算（中文 1:1，英文 1:0.75）
- **分层截断**：优先截断非核心部分，保留 AI 身份和行为规范
- **隐私过滤**：在收集用户数据时，**严格过滤敏感级别**，只保留 public 级别

---

## 3. 帮助文档检索与嵌入（Help Document Embedding）

### 3.1 Flutter 实现：`UserGuideService`

Flutter 端并未使用向量数据库或 RAG（Retrieval-Augmented Generation），而是采用了**轻量级关键词匹配**方案，理由：

1. **本地优先**：不需要外部 embedding 服务，保护隐私
2. **数据量小**：帮助文档数量有限（通常 < 100 篇），关键词匹配足够
3. **即时响应**：无需向量检索延迟

#### 3.1.1 实现流程

```
用户提问: "如何导出数据？"
    │
    ▼
分词 + 停用词过滤
  输入: "如何 导出 数据"
  停用词过滤后: "导出 数据"
    │
    ▼
遍历所有指南文档，计算匹配分数
  分数 = 关键词匹配数 × 1 + 标题匹配数 × 3
    │
    ▼
筛选分数 ≥ 2 的文档，取 Top-1
    │
    ▼
内容截断至 800 字符
    │
    ▼
组装为 system message 注入对话
```

#### 3.1.2 文档结构

帮助文档存储在 `assets/docs/guides/` 目录下：

```
guides/
├── index.json          # 文档索引（id, title, keywords, file）
├── zh/
│   ├── export_data.md
│   ├── import_data.md
│   └── ...
├── en/
│   ├── export_data.md
│   └── ...
└── [其他语言]/
```

`index.json` 示例：
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

#### 3.1.3 多语言回退

```dart
// 请求语言 → 英文 → 第一个可用语言
String _resolveLanguage(String requested) {
  if (content.containsKey(requested)) return requested;
  if (content.containsKey('en')) return 'en';
  return content.keys.first;
}
```

#### 3.1.4 注入格式

匹配到的指南内容被包装为 `system` 角色的消息：

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

### 3.2 对 Tauri 的借鉴建议

- **阶段一（当前）**：采用与 Flutter 相同的**关键词匹配方案**
  - 维护 `index.json` 索引文件
  - 中英文关键词混合匹配
  - 标题匹配权重更高（3x）
  - 分数阈值设为 2，只返回 Top-1
  - 内容截断至 800 字符

- **阶段二（未来）**：考虑升级为**向量检索**
  - 仅在本地运行 embedding 模型（如 `all-MiniLM-L6-v2`）
  - 使用 `solosoul-crypto` 保护向量数据库
  - 保持本地优先原则，不上传数据到外部服务

- **文档管理**：
  - 与 Flutter 共用同一套文档源（`assets/docs/guides/`）
  - Tauri 端通过 `tauri::api::path` 读取文档
  - 支持热重载（开发模式下）

---

## 4. 对话流架构

### 4.1 Flutter 实现概览

Flutter 端的对话流由三个核心组件协作完成：

| 组件 | 职责 | 文件 |
|------|------|------|
| `LlmModelNotifier` | 管理模型生命周期、推理调用、统计追踪 | `llm_model_provider.dart` |
| `LlmChatSessionNotifier` | 管理消息列表、持久化、流式接收 | `llm_chat_session_provider.dart` |
| `LlmChatPanel` | UI 层：渲染消息、输入框、功能按钮 | `llm_chat_panel.dart` |

### 4.2 流式推理实现

#### 4.2.1 本地 Ollama 服务

```dart
// 原生 SSE 流式
await for (final chunk in service.streamChatMessages(messages, ...)) {
  yield chunk;  // 直接转发 chunk
}
```

#### 4.2.2 云端服务 Fallback

由于云端 OpenAI/Anthropic API 可能不支持原生流式（或需要额外适配），Flutter 采用了**打字机效果模拟**：

```dart
// 1. 先完整获取推理结果
final result = await _manager.inferMessages(messages, maxTokens: maxTokens);

// 2. 按 grapheme cluster（避免切开 surrogate pair）逐字 emit
final chars = result.characters.toList();
for (var i = 0; i < chars.length; i++) {
  yield chars[i];
  await Future.delayed(const Duration(milliseconds: 8));
}
```

**延迟 8ms**：模拟人类打字速度，提升用户体验。

#### 4.2.3 消息接收与状态管理

```dart
// 订阅 stream，100ms debounce 批量刷新 UI
_streamSub = stream.listen(
  (chunk) {
    buffer.write(chunk);
    debounceTimer?.cancel();
    debounceTimer = Timer(const Duration(milliseconds: 100), () => flushState());
  },
  onDone: () {
    flushState(finish: true);  // 标记 isStreaming = false
    _debouncedSave();          // 保存到 Vault
    _updateSessionStats();     // 更新统计
  },
);
```

### 4.3 持久化设计

| 数据 | 持久化位置 | 时机 | 说明 |
|------|-----------|------|------|
| 对话消息 | 加密 Vault | 每 2 秒 debounce | 仅持久化 user/assistant 消息 |
| 系统提示词 | **不持久化** | N/A | 每次发送前动态生成 |
| 使用统计 | 加密 Vault | 账户切换 / App 退出 | 按账户隔离 |
| 会话元数据 | 加密 Vault | 实时 | 标题、消息数、创建时间 |

### 4.4 对 Tauri 的借鉴建议

- **统一消息模型**：
  ```typescript
  interface LlmMessage {
    role: 'system' | 'user' | 'assistant';
    content: string;
  }
  ```

- **流式处理**：
  - 本地模型：原生 SSE 流式
  - 云端模型：打字机 fallback（完整推理后逐字 emit）
  - 使用 grapheme cluster 而非字符拆分，避免切断 emoji/中文

- **状态管理**：
  - 使用 React Context + Zustand（Tauri 端已有 Zustand）
  - Stream 订阅提升到 Store 层，避免组件 unmount 导致中断
  - 100ms debounce 刷新 UI

- **持久化**：
  - 消息保存到 Rust Vault（通过 Tauri Command）
  - 2 秒 debounce，避免频繁 IO
  - 系统提示词不进入持久化历史

---

## 5. 隐私和安全设计

### 5.1 数据分级暴露

Flutter 端严格遵循 SoloSoul 的敏感数据分级系统：

| 敏感度级别 | 是否进入系统提示词 | 说明 |
|-----------|------------------|------|
| `public` | ✅ 是 | 用户主动公开的信息 |
| `internal` | ❌ 否 | 内部使用数据，不暴露给 AI |
| `private` | ❌ 否 | 私人数据，需要显式授权 |
| `sensitive` | ❌ 否 | 敏感数据，需重新验证密码 |
| `restricted` | ❌ 否 | 受限数据，需重新验证密码 |
| `critical` | ❌ 否 | 关键数据，需重新验证密码 |

### 5.2 AI 行为规范（硬编码）

系统提示词中明确约束 AI 的行为边界：

1. **语言匹配**：使用与用户提问相同的语言回答
2. **概念区分**：区分"插件"（功能扩展）和"对象"（用户数据）
3. **敏感数据拒绝**：当被问及敏感/受限/关键数据时，告知用户需要重新验证密码
4. **不编造**：无法访问用户本地数据时，建议用户手动查找，绝不编造
5. **数据保护**：不泄露用户数据给插件或外部服务

### 5.3 统计追踪设计

| 统计维度 | 粒度 | 持久化 | 说明 |
|---------|------|--------|------|
| 推理调用次数 | 会话 + 账户 | ✅ | 每次推理 +1 |
| Prompt Token | 会话 + 账户 | ✅ | 估算值 |
| Completion Token | 会话 + 账户 | ✅ | 估算值 |
| 总 Token | 会话 + 账户 | ✅ | Prompt + Completion |
| 按模型统计 | 账户 | ✅ | 各模型使用占比 |
| 每日统计 | 账户 | ✅ | 按天聚合 |

**关键设计**：
- 会话级统计：应用生命周期内累计，重启清零
- 账户级统计：持久化到加密 Vault，跨会话保留
- 账户切换时：先保存旧账户统计，再加载新账户统计
- Vault 锁定前：自动保存当前账户统计

### 5.4 对 Tauri 的借鉴建议

- **强制数据分级**：在收集用户数据注入系统提示词前，必须经过敏感级别过滤
- **硬编码行为约束**：系统提示词中必须包含 AI 行为规范，不可由用户修改
- **统计隔离**：按账户隔离使用统计，切换账户时正确保存/加载
- **Vault 集成**：统计和消息历史都保存到 Rust Vault，利用现有加密机制

---

## 6. Tauri 端实现建议（概要）

基于以上分析，建议 Tauri 端按以下优先级实现：

### P0（核心功能）

1. **系统提示词模板**：参考 Flutter 的 7 Section 结构，定义标准模板
2. **上下文注入服务**：
   - 构建 `LlmContextService`（Rust 或 TS 端）
   - 实现缓存机制（内存级，按账户隔离）
   - 实现长度限制和截断策略
3. **帮助文档检索**：
   - 复用 Flutter 的 `index.json` 结构
   - 实现关键词匹配（中英文）
   - 分数阈值 2，Top-1 返回

### P1（增强体验）

4. **流式推理**：
   - 本地模型原生 SSE
   - 云端模型打字机 fallback
5. **使用统计**：
   - 会话级 + 账户级统计
   - 持久化到 Vault
6. **多语言支持**：
   - 系统提示词语言跟随用户设置
   - 帮助文档多语言回退

### P2（未来优化）

7. **向量检索**：本地 embedding 模型 + 向量数据库
8. **上下文压缩**：长对话时自动压缩历史消息
9. **智能截断**：基于语义重要性（而非简单按行）截断用户数据

---

## 7. 附录：关键文件索引

| 文件 | Flutter 路径 | 说明 |
|------|-------------|------|
| 系统提示词模板 | `lib/core/services/llm/llm_prompt_templates.dart` | `LlmPromptTemplate.chatSystemPrompt()` |
| 上下文服务 | `lib/core/services/llm/llm_context_service.dart` | `LlmContextService.buildContext()` |
| 用户指南服务 | `lib/core/services/user_guide_service.dart` | `UserGuideService.findRelevantGuides()` |
| 模型 Provider | `lib/presentation/providers/llm/llm_model_provider.dart` | `LlmModelNotifier.streamChat()` |
| 会话 Provider | `lib/presentation/providers/llm/llm_chat_session_provider.dart` | `LlmChatSessionNotifier.sendMessage()` |
| 聊天面板 UI | `lib/presentation/widgets/llm/llm_chat_panel.dart` | 完整对话 UI 实现 |

---

## 8. 与现有 Tauri 文档的对比

| 功能 | Flutter 实现 | Tauri 文档现状 | 差距 |
|------|-------------|---------------|------|
| 系统提示词 | ✅ 完整 7 Section 模板 | ❌ 未提及 | **大差距** |
| 上下文注入 | ✅ 隐私优先 + 缓存 + 截断 | ❌ 仅提及"可配置" | **大差距** |
| 帮助文档嵌入 | ✅ 关键词匹配 + 多语言 | ⚠️ `22_帮助文档系统重构.md` 有计划但无实现 | 中等差距 |
| 流式推理 | ✅ 本地 SSE + 云端打字机 | ⚠️ `20_LLM配置与AI对话规范.md` 有计划 | 中等差距 |
| 隐私过滤 | ✅ 仅 public 级别数据 | ⚠️ 提及隐私但未细化 | 小差距 |
| 使用统计 | ✅ 会话 + 账户级 | ❌ 未提及 | 小差距 |

---

> **结论**：Flutter 端在 LLM/AI 对话功能上已经积累了成熟的设计模式和实现经验，特别是在**系统提示词设计**、**隐私优先的上下文注入**和**轻量级帮助文档检索**方面。Tauri 端应当优先借鉴这些已实现且验证过的方案，而非重新设计。
