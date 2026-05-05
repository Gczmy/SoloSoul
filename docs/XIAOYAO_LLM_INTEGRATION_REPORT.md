# SoloSoul ← xiaoyaosearch LLM 集成借鉴评估报告

> **分析日期**: 2026-05-01
> **分析范围**:
> - xiaoyaosearch: `backend/app/services/ai_model_base.py`, `ai_model_manager.py`, `openai_llm_service.py`, `ollama_service.py`, `openai_embedding_service.py`, `llm_query_enhancer.py`, `mcp/server.py`, `mcp/tools/semantic_search.py`
> - SoloSoul: `flutter/lib/core/services/llm/llm_service.dart`, `llm_privacy_filter.dart`, `llm_config_service.dart`, `llm_extraction_result.dart`, `presentation/pages/llm/llm_config_page.dart`, `presentation/providers/llm/llm_config_provider.dart`, `presentation/providers/llm/llm_stub_provider.dart`, `docs/TODO.md`, `docs/LOCAL_SEARCH_IMPORT_DESIGN.md`
> **报告作者**: Kimi Code CLI

---

## 目录

1. [双方现状对比](#一双方现状对比)
2. [可直接借鉴的核心模块](#二可直接借鉴的核心模块按优先级排序)
   - P1: OpenAI 兼容云端 LLM 服务
   - P2: Ollama 本地 LLM 服务
   - P3: AI 模型管理器生命周期
   - P4: LLM 查询增强器
   - P5: Embedding 批量处理与容错
   - P6: MCP (Model Context Protocol) 服务器
3. [不建议直接借鉴的部分](#三不建议直接借鉴的部分)
4. [具体实施路线图建议](#四具体实施路线图建议)
5. [代码复用清单](#五代码复用清单)
6. [风险与注意事项](#六风险与注意事项)
7. [总结](#七总结)

---

## 一、双方现状对比

| 维度 | xiaoyaosearch（Python/FastAPI） | SoloSoul（Flutter/Dart + Rust） |
|------|-------------------------------|--------------------------------|
| **架构层** | 完整的 ABC + ModelManager + 工厂模式 | `LlmService` 抽象接口 + `LlmLocal/CloudService` Stub ✅ |
| **云端 LLM** | `aiohttp` 直连 OpenAI 兼容 API，支持多供应商 | 仅 Stub，无真实 HTTP 客户端 🚫 |
| **本地 LLM** | Ollama HTTP API 封装，自动拉模型 | 仅 Stub，Rust FFI 未暴露 🚫 |
| **Embedding** | BGE-M3 本地 + OpenAI 云端，批量+并发+重试 | 无实现 🚫 |
| **隐私/安全** | API Key 日志脱敏（前7后4） | Vault 加密存储 API Key + `LlmPrivacyFilter` 分级脱敏 ✅ |
| **配置管理** | SQLite 持久化，支持热重载 | `RustVaultService` AES-256-GCM 加密存储 ✅ |
| **错误处理** | `AIModelException` + 状态机 + 优雅降级 | `LlmException` enum 已定义，待填充逻辑 🚫 |
| **查询增强** | LLM 查询重写 + 规则回退 | 无 🚫 |
| **MCP 暴露** | 5 个搜索工具供外部 Agent 调用 | 无 🚫 |
| **流式响应** | Ollama 支持 `stream_chat` | 无 🚫 |

**结论**: xiaoyaosearch 在**推理执行层**（HTTP 客户端、状态机、批量处理）非常成熟；SoloSoul 在**隐私安全层**（Vault 加密、敏感数据分级）设计更先进。两者互补性极强。

---

## 二、可直接借鉴的核心模块（按优先级排序）

### 🔥 P1：OpenAI 兼容云端 LLM 服务

**xiaoyaosearch 文件**: `backend/app/services/openai_llm_service.py`（310 行）

**核心设计**:
- 支持所有兼容 OpenAI API 标准的供应商（OpenAI、阿里云 DashScope、DeepSeek、Moonshot 等）
- 使用 `aiohttp` 直接发送 `POST /chat/completions`，不依赖 `openai` SDK 的高级封装
- 请求参数：`model`, `messages`, `temperature`, `max_tokens`, `top_p`
- 响应提取：`choices[0].message.content` + `usage` Token 统计

**可借鉴内容**:

#### 1. 消息标准化方法 (`_standardize_messages`)

```python
# xiaoyaosearch 实现
async def predict(self, messages: Union[str, List[Message], List[Dict]], **kwargs) -> Dict[str, Any]:
    standardized_messages = self._standardize_messages(messages)
    # ...

def _standardize_messages(self, messages: Union[str, List[Message], List[Dict]]) -> List[Dict]:
    if isinstance(messages, str):
        return [{"role": "user", "content": messages}]
    elif isinstance(messages, list):
        if messages and isinstance(messages[0], Message):
            return [{"role": msg.role, "content": msg.content} for msg in messages]
        elif messages and isinstance(messages[0], dict):
            return messages
        else:
            raise AIModelException(f"不支持的消息格式: {type(messages[0])}")
    else:
        raise AIModelException(f"不支持的消息类型: {type(messages)}")
```

**借鉴价值**: SoloSoul 当前 `LlmService.infer(String prompt)` 是单字符串接口，可扩展为接受 `List<LlmMessage>` 以支持多轮对话和 system prompt。翻译为 Dart:

```dart
class LlmMessage {
  final String role; // 'system' | 'user' | 'assistant'
  final String content;
  const LlmMessage({required this.role, required this.content});
}

List<Map<String, String>> standardizeMessages(dynamic input) {
  if (input is String) return [{'role': 'user', 'content': input}];
  if (input is List<LlmMessage>) {
    return input.map((m) => {'role': m.role, 'content': m.content}).toList();
  }
  throw LlmException('不支持的消息类型');
}
```

#### 2. 连接验证模式 (`load_model` 中发送 test request)

```python
async def load_model(self) -> bool:
    self.session = aiohttp.ClientSession(timeout=...)
    await self._test_connection()  # 发送 max_tokens=10 的极小请求
    self.update_status(ModelStatus.LOADED)

async def _test_connection(self) -> bool:
    request_data = {
        "model": self.model,
        "messages": [{"role": "user", "content": "Hi"}],
        "max_tokens": 10
    }
    # 验证返回 200 即认为可用
```

**借鉴价值**: SoloSoul `llm_config_page.dart` 的"测试连接"按钮可直接复用该模式。避免用户配置错误 endpoint/apiKey/model 后，在真正推理时才发现问题。

#### 3. API Key 脱敏日志

```python
masked_key = f"{self.api_key[:7]}...{self.api_key[-4:]}" if len(self.api_key) > 11 else "***"
logger.info(f"初始化OpenAI兼容服务，... API密钥: {masked_key}")
```

**借鉴价值**: 直接移植到 Dart 端，任何涉及 API Key 的日志输出都必须经过此掩码处理。这是零知识架构下的基本安全 hygiene。

#### 4. 统一的响应封装结构

```python
return {
    "content": content,
    "model": self.model,
    "provider": "cloud",
    "finish_reason": choice.get("finish_reason"),
    "usage": {
        "prompt_tokens": usage.get("prompt_tokens", 0),
        "completion_tokens": usage.get("completion_tokens", 0),
        "total_tokens": usage.get("total_tokens", 0)
    }
}
```

**借鉴价值**: SoloSoul 可定义 Dart 类 `LlmInferenceResponse`，为后续 Token 计费、用量审计、速率限制做准备。此结构也适用于本地 Ollama 模型（Ollama 同样返回 prompt_eval_count / eval_count）。

**SoloSoul 实现位置**: `flutter/lib/core/services/llm/llm_cloud_service.dart`（替换现有 Stub）

---

### 🔥 P2：Ollama 本地 LLM 服务

**xiaoyaosearch 文件**: `backend/app/services/ollama_service.py`（506 行）

**核心设计**:
- 默认模型 `qwen2.5:1.5b`，HTTP 直连 `http://localhost:11434`
- 不依赖 `ollama` Python SDK，直接调用 Ollama HTTP API
- 支持标准推理 (`stream: False`) 和流式推理 (`stream: True`)
- 包含性能基准测试 (`benchmark_performance`)

**可借鉴内容**:

#### 1. HTTP API 直连模式

```python
# 标准推理
request_data = {
    "model": self.model_name,
    "messages": messages,
    "options": {
        "temperature": ...,
        "top_p": ...,
        "num_predict": ...,
        "num_ctx": ...,
    },
    "stream": False
}
url = f"{self.config['base_url']}/api/chat"
async with self.session.post(url, json=request_data) as response:
    result = await response.json()
```

**借鉴价值**: 避免了引入重量级 SDK，符合 SoloSoul 最小依赖原则。Dart `http` 包即可实现，无需额外原生依赖。Ollama 的 `/api/chat` 接口与 OpenAI `/chat/completions` 结构相似，可共用大量序列化/反序列化逻辑。

#### 2. 模型存在性检查 + 自动拉取

```python
async def _check_model_exists(self) -> bool:
    async with self.session.get(f"{base_url}/api/tags") as response:
        data = await response.json()
        models = [m.get("name", "").split(":")[0] for m in data.get("models", [])]
        if self.model_name.split(":")[0] in models:
            return True
        else:
            return await self._pull_model()

async def _pull_model(self) -> bool:
    async with self.session.post(f"{base_url}/api/pull", json={"name": self.model_name}) as response:
        # 流式读取拉取进度
        async for line in response.content:
            data = json.loads(line.decode())
            status = data.get("status", "")
            if "download" in status.lower():
                logger.info(f"模型拉取进度: {status}")
```

**借鉴价值**: SoloSoul 本地模型配置页可实现"检测/下载"按钮。`/api/tags` 返回已安装模型列表用于下拉选择；`/api/pull` 的流式进度可用于 UI 进度条展示。

#### 3. 流式聊天接口 (`stream_chat`)

```python
async def stream_chat(self, message: str, history: Optional[List[Dict]] = None, **kwargs):
    request_data = {
        "model": self.model_name,
        "messages": messages,
        "options": {...},
        "stream": True
    }
    async with self.session.post(url, json=request_data) as response:
        async for line in response.content:
            if line:
                data = json.loads(line.decode())
                if "message" in data and "content" in data["message"]:
                    content = data["message"]["content"]
                    if content:
                        yield content
```

**借鉴价值**: Ollama 流式响应每行一个 JSON，包含增量 `content`。Dart 端可映射为 `Stream<String>`，供 Flutter UI 逐字渲染。这是本地 LLM 提供良好用户体验的关键。

#### 4. 性能基准测试 (`benchmark_performance`)

```python
async def benchmark_performance(self, test_messages: List[str], num_runs: int = 3) -> Dict[str, Any]:
    times = []
    total_tokens = 0
    for run in range(num_runs):
        for message in test_messages:
            start_time = time.time()
            result = await self.predict(message)
            end_time = time.time()
            times.append(end_time - start_time)
            total_tokens += result.get("usage", {}).get("total_tokens", 0)

    return {
        "avg_time": np.mean(times),
        "tokens_per_second": total_tokens / sum(times),
        "throughput": len(test_messages) / np.mean(times),
    }
```

**借鉴价值**: 固定测试消息（如"你好"、"请介绍一下自己"）多次运行，统计 `tokens_per_second`。可放到 SoloSoul LLM 配置页作为"本地模型性能测试"功能，帮助用户评估硬件是否满足需求。

**SoloSoul 实现位置**: `flutter/lib/core/services/llm/llm_local_service.dart`（替换现有 Stub，初期通过 HTTP 调用 Ollama 而非直接 Rust FFI，降低实现复杂度）

---

### 🔥 P3：AI 模型管理器生命周期

**xiaoyaosearch 文件**: `backend/app/services/ai_model_base.py`（433 行）+ `ai_model_manager.py`（995 行）

**核心设计**:
- `BaseAIModel` 抽象基类：定义 `load_model()`, `unload_model()`, `predict()`, `get_model_info()`
- `ModelManager`: 管理多个模型实例的注册、加载、卸载、健康检查
- `AIModelService`: 更高层 orchestrator，从数据库加载配置，创建默认模型实例
- 状态机: `UNLOADED → LOADING → LOADED → ERROR`

**可借鉴内容**:

#### 1. 状态机枚举

```python
class ModelStatus(Enum):
    UNLOADED = "unloaded"
    LOADING = "loading"
    LOADED = "loaded"
    ERROR = "error"
```

**借鉴价值**: SoloSoul 当前缺少运行时状态管理。引入 `LlmModelState` enum 后，UI 可显示"模型加载中/不可用/就绪"，且所有推理调用前必须检查 `status == LOADED`，否则抛异常。防止用户在模型未就绪时触发请求。

#### 2. 并行加载优化

```python
async def load_all_models(self) -> Dict[str, bool]:
    load_tasks = []
    for model_id, model in self.models.items():
        if model.status == ModelStatus.LOADED:
            continue
        async def load_with_error_handling(mid, m):
            try:
                return mid, await m.load_model()
            except Exception as e:
                return mid, False
        load_tasks.append(load_with_error_handling(model_id, model))

    results_list = await asyncio.gather(*load_tasks, return_exceptions=True)
```

**借鉴价值**: SoloSoul 若同时初始化 Embedding + LLM，可用 Dart `Future.wait` 并行加载，减少启动等待时间。每个加载任务独立捕获异常，避免一个模型失败导致整批中断。

#### 3. 热重载机制 (`reload_model`)

```python
async def reload_model(self, model_type: str) -> Dict[str, Any]:
    current_model_id = self.default_models.get(model_type)
    # 1. 卸载当前模型
    await self.unload_model(current_model_id)
    # 2. 重新读取配置
    await self._load_model_configs_from_db()
    # 3. 创建并加载新模型
    new_model = create_..._service(config)
    self.model_manager.register_model(new_model_id, new_model)
    await self.model_manager.load_model(new_model_id)
    # 4. 更新默认映射
    self.default_models[model_type] = new_model_id
```

**借鉴价值**: SoloSoul 用户在不重启 App 的情况下切换模型（如从 `gpt-4o-mini` 切到 `gpt-4o`，或从 `qwen2.5:1.5b` 切到 `llama3.2`）时，需要完整的卸载-重建-加载流程。此模式可直接翻译为 Dart 实现。

#### 4. 健康检查

```python
async def health_check(self) -> bool:
    if self.status != ModelStatus.LOADED:
        return False
    test_input = self._get_test_input()
    await self.predict(test_input)
    return True
```

**借鉴价值**: 后台定时检测模型可用性（如 App 从后台恢复时），失败时自动切换 fallback 或提示用户。`_get_test_input()` 是抽象方法，各子类提供适合自身的测试输入（LLM 用短句，Embedding 用"test"）。

**SoloSoul 实现位置**: 新增 `flutter/lib/core/services/llm/llm_model_manager.dart`

---

### 🔥 P4：LLM 查询增强器（Query Enhancer）

**xiaoyaosearch 文件**: `backend/app/services/llm_query_enhancer.py`（211 行）

**核心设计**:
- 使用当前激活的 LLM 模型对搜索查询进行扩展和重写
- 智能判断是否需要增强（避免对文件名、布尔查询等做无意义增强）
- 三层降级策略：LLM JSON 解析 → 规则同义词扩展 → 返回原始查询
- Prompt 要求 JSON 结构化输出

**可借鉴内容**:

#### 1. "是否增强"的智能判断

```python
def _should_enhance_query(self, query: str) -> bool:
    query = query.strip()
    if len(query) <= 2:
        return False
    if any(sep in query for sep in ['.', '/', '\\', ':']):
        return False  # 文件名或路径查询
    if query.startswith('"') and query.endswith('"'):
        return False  # 已用引号的精确查询
    if any(op in query.lower() for op in [' and ', ' or ', ' not ', '+', '-']):
        return False  # 包含操作符的复杂查询
    return True
```

**借鉴价值**: 这个规则集可直接翻译为 Dart 工具函数，用于 SoloSoul 的智能搜索、标签推荐、字段映射建议等场景。避免对已经精确或结构化的输入做无意义的 LLM 调用，节省 Token 和时间。

#### 2. Prompt 工程模板

```python
def _build_simple_prompt(self, query: str) -> str:
    return f"""你是一个搜索查询优化专家。请对以下中文查询进行优化：

原始查询：{query}

请提供JSON格式响应：
{{
    "expanded_query": "扩展查询（添加3-5个同义词，用空格分隔）",
    "rewritten_query": "重写查询（更准确的表达）"
}}

示例：
输入：怎么用Python
输出：{{"expanded_query": "怎么用Python Python使用方法 Python教程 Python入门 Python操作指南", "rewritten_query": "Python使用方法教程"}}

请只返回JSON，不要其他内容。"""
```

**借鉴价值**:
- **结构化输出约束**: 强制 JSON 格式，便于程序解析
- **低 temperature (0.3)**: 降低随机性，提高输出稳定性
- **少样本示例 (few-shot)**: 在 prompt 中给出一个完整示例，减少模型"自由发挥"
- **保守 max_tokens (150)**: 查询增强不需要长输出，限制长度可减少等待时间和 Token 消耗

SoloSoul 的"脱敏后申请理由生成"、"文本润色"、"AI 辅助字段映射"等业务场景可直接套用该模板结构。

#### 3. 三层降级策略

```python
async def enhance_query(self, query: str) -> Dict[str, any]:
    if not self._should_enhance_query(query):
        return _create_fallback_response(query)  # 第一层：不需要增强
    try:
        response = await ai_model_service.text_generation(...)
        result = self._parse_simple_response(response.get('content', ''), query)
        return result  # 第二层：LLM 成功
    except Exception as e:
        return self._create_rule_based_response(query)  # 第三层：规则回退
```

**借鉴价值**: 这是生产级 LLM 集成的关键模式。**即使 LLM 失败、返回垃圾、超时，核心业务功能绝不能挂掉**。SoloSoul 的所有 LLM 应用场景都必须实现类似的降级链。

**SoloSoul 应用场景**:
- 本地文件导入时的 AI 辅助字段映射（`LOCAL_SEARCH_IMPORT_DESIGN.md` Phase 4 规划）
- 搜索查询扩展
- 非敏感文本润色/翻译（`docs/TODO.md` P5）

---

### 🔥 P5：Embedding 批量处理与容错

**xiaoyaosearch 文件**: `backend/app/services/openai_embedding_service.py`（473 行）

**核心设计**:
- 支持 OpenAI 兼容的云端 Embedding API
- 批量处理（`batch_size=10`）+ 并发控制（`concurrent_requests=4`）
- `tenacity` 重试：指数退避，3 次尝试，等待 2~10 秒
- 零向量降级：某一批次失败时返回该 batch 大小的零向量

**可借鉴内容**:

#### 1. 批量 + 并发控制

```python
batch_size = kwargs.get("batch_size", self.config.get("batch_size", 10))
concurrent_limit = kwargs.get("concurrent_requests", self.config.get("concurrent_requests", 4))
semaphore = asyncio.Semaphore(concurrent_limit)

async def process_batch(batch_info: dict) -> tuple:
    async with semaphore:
        result = await self._call_embeddings_api(batch_texts)
        # ...

tasks = [process_batch(batch_info) for batch_info in batches]
results = await asyncio.gather(*tasks)
```

**借鉴价值**: 避免一次性发送超大请求被云端限流。Dart 端可用 `pool` 包或自定义 `Future` 队列实现 Semaphore 效果。

#### 2. tenacity 重试机制

```python
@retry(
    stop=stop_after_attempt(3),
    wait=wait_exponential(multiplier=1, min=2, max=10),
    retry=retry_if_exception_type((aiohttp.ClientError, asyncio.TimeoutError)),
    reraise=True
)
async def _call_embeddings_api(self, texts: List[str]) -> Dict[str, Any]:
    # ...
```

**借鉴价值**: 指数退避重试是调用第三方 API 的标准做法。Dart `dio` 的 `RetryInterceptor` 可实现相同效果：

```dart
final dio = Dio();
dio.interceptors.add(RetryInterceptor(
  dio: dio,
  retries: 3,
  retryDelays: const [
    Duration(seconds: 2),
    Duration(seconds: 4),
    Duration(seconds: 8),
  ],
));
```

#### 3. 零向量降级

```python
except Exception as e:
    logger.error(f"批次 {batch_num} 嵌入失败: {str(e)}")
    if self._embedding_dim:
        empty_embeddings = [[0.0] * self._embedding_dim] * len(batch_texts)
        return (batch_index, empty_embeddings, error_msg)
    else:
        return (batch_index, None, error_msg)
```

**借鉴价值**: 批量处理中单个批次失败时，用零向量占位，保证整体流程不中断。SoloSoul 若未来实现本地语义搜索索引，此模式可防止单个坏文档破坏整批索引。日志记录失败批次，便于后续排查。

**注意**: SoloSoul 目前无 Embedding 需求，但 `LOCAL_SEARCH_IMPORT_DESIGN.md` Phase 4 提到"本地 LLM"可能涉及语义能力，此模块可作为技术储备。

---

### 🔥 P6：MCP (Model Context Protocol) 服务器

**xiaoyaosearch 文件**: `mcp/server.py`, `mcp/tools/semantic_search.py`, `mcp/tools/hybrid_search.py`, `mcp/tools/voice_search.py`, `mcp/tools/image_search.py`, `mcp/tools/fulltext_search.py`

**核心设计**:
- 使用 `fastmcp` 框架创建 MCP 服务器
- 将搜索能力注册为 5 个 MCP Tool：`semantic_search`, `hybrid_search`, `fulltext_search`, `voice_search`, `image_search`
- 外部 Agent（Claude、Cursor）可通过 MCP 协议调用 xiaoyaosearch 的搜索能力
- 每个 Tool 内部可选择性调用 `query_enhancer.enhance_query()`

**可借鉴内容**:

#### 1. 将 SoloSoul 数据暴露为 MCP Tool

xiaoyaosearch 的模式:

```python
@mcp.tool()
async def semantic_search(
    query: str,
    limit: int = default_limit,
    threshold: float = default_threshold,
    enable_query_enhancement: bool = True
) -> str:
    """基于BGE-M3模型的语义搜索..."""
    # 参数校验
    if not query or len(query) > 500:
        raise ValueError("query 必须为1-500字符")
    # LLM 查询增强（可选）
    if enable_query_enhancement:
        enhanced_query = await query_enhancer.enhance_query(query)
    # 执行搜索
    result = await service.search(query=enhanced_query, ...)
    return format_search_result(result)
```

**借鉴价值**: SoloSoul 可将核心数据能力封装为 MCP Tool:
- `query_profile`：查询个人档案中的特定字段
- `search_objects`：按类型/时间/关键词检索 UnifiedObject
- `summarize_travel`：总结旅行记录
- `analyze_financial`：分析财务数据趋势
- `extract_insights`：从操作日志中提取洞察

外部 Agent（如 Claude Desktop、Cursor）在用户显式授权下，通过 MCP 调用 SoloSoul 本地数据。**这完美契合 SoloSoul " Centralized Schema, decentralized storage" 的核心理念** —— 数据永不离开本地，但外部 Agent 可以在本地 sandbox 中读取。

#### 2. 参数校验模式

```python
if not 1 <= limit <= 100:
    raise ValueError("limit 必须为1-100")
if not 0.0 <= threshold <= 1.0:
    raise ValueError("threshold 必须为0.0-1.0")
```

**借鉴价值**: 工具函数内显式校验参数范围，错误直接抛异常，由 MCP 框架自动转换为标准错误格式。避免 Agent 传入异常值导致内部崩溃。

#### 3. LLM 增强与工具解耦

```python
if enable_query_enhancement:
    try:
        query_enhancer = get_llm_query_enhancer()
        enhancement_result = await query_enhancer.enhance_query(query)
        if enhancement_result.get('success') and enhancement_result.get('enhanced'):
            enhanced_query = enhancement_result.get('expanded_query', query)
    except Exception as e:
        logger.warning(f"LLM查询增强失败，使用原始查询: {str(e)}")
        enhanced_query = query
```

**借鉴价值**: MCP Tool 内部可选择性调用本地/云端 LLM 做查询理解，但失败时始终回退到原始输入。这保证了即使 LLM 层完全不可用，Tool 的基础功能仍然正常。

**SoloSoul 实现位置**: 可考虑在 Go 后端 `core/api/` 侧或 Flutter Desktop 侧实现 MCP server。Dart 生态已有 `mcp_dart` 包可供评估。

---

## 三、不建议直接借鉴的部分

| xiaoyaosearch 做法 | 不建议原因 | SoloSoul 替代方案 |
|-------------------|-----------|------------------|
| 直接依赖 `aiohttp` + `tenacity` + `numpy` | Python 生态，Dart/Flutter 不可用 | Dart `http` / `dio` + 自定义 `RetryInterceptor` |
| SQLite 存储模型配置 | SoloSoul 已有 Vault 加密体系，且 SQLite 在移动端需额外配置 | 复用 `RustVaultService.saveSettingEncrypted()` 存储 LLM 配置 |
| 全局单例 `ai_model_service = AIModelService()` | 难以测试，与 Flutter Riverpod 架构冲突 | 使用 Riverpod `AsyncNotifier` 管理 LLM 生命周期，支持 widget 级 dispose |
| 云端 Embedding 批量处理 | 当前无明确需求，且批量文本上云存在隐私风险 | 如需 Embedding，优先本地 BGE 模型（通过 Ollama 或 Rust FFI） |
| 术语库扩展 (`GlossaryService`) | 业务域差异大（通用搜索 vs 个人数据管理） | 复用 SoloSoul 现有的 Schema/字段体系做 Prompt 上下文增强 |
| 语音搜索 (`voice_search`) | 依赖 Whisper + 音频处理，SoloSoul 已有 OCR 但无 STT 规划 | 如需语音输入，复用系统级语音转文本 API（iOS/Android native） |
| CLIP 图像理解 | 依赖 PyTorch + CN-CLIP 模型，体积巨大 | 如有图像 AI 需求，考虑云端 Vision API 或 CoreML/TensorFlow Lite 本地模型 |

---

## 四、具体实施路线图建议

基于 xiaoyaosearch 的成熟模式 + SoloSoul 现有脚手架，建议按以下顺序落地：

### Phase 1：云端 LLM 最小可用（1~2 周）

**目标**: 让 SoloSoul 用户能真正调用 OpenAI/Claude/DeepSeek 等云端 API 完成基础推理。

1. **替换 `LlmCloudService` Stub**
   - 使用 Dart `http` 客户端实现 `POST {endpoint}/chat/completions`
   - 支持 `List<LlmMessage>` 输入（借鉴 `_standardize_messages`）
   - 请求体包含 `model`, `messages`, `temperature`, `max_tokens`, `top_p`
   - 响应解析为 `LlmInferenceResponse`（含 content + usage）

2. **引入重试与超时**
   - 简单指数退避：失败时延迟 1s → 2s → 4s 后重试，最多 3 次
   - HTTP 401 → `LlmErrorCode.unauthorized`
   - HTTP 429 → `LlmErrorCode.rateLimited`
   - HTTP 5xx → `LlmErrorCode.network`
   - Dart `TimeoutException` → `LlmErrorCode.timeout`

3. **连接测试按钮**
   - 在 `llm_config_page.dart` 中实现"测试连接"
   - 发送 `max_tokens=10` 的探活请求（借鉴 `_test_connection`）
   - 成功/失败给出明确 UI 反馈

4. **移除 `kDebugMode` 限制**
   - `settings_page.dart` 中 `_LLMSettingsSection` 对所有用户可见
   - 增加"实验性功能"提示标签
   - 默认 backend 仍为 `local`，用户需手动切换到 `cloud`

### Phase 2：本地 Ollama 集成（1~2 周）

**目标**: 支持用户在本地运行开源模型，实现零上云的隐私保护推理。

1. **实现 `LlmLocalService`**
   - HTTP 调用 `http://localhost:11434/api/chat`
   - 请求体使用 Ollama 格式（`model`, `messages`, `options`, `stream`）
   - 响应解析 Ollama 特有的 `message.content` + `eval_count`

2. **模型管理功能**
   - 模型检测：`GET /api/tags` 获取已安装模型列表
   - 自动拉取：`POST /api/pull` 流式读取下载进度
   - 配置页显示本地模型下拉选择器 + "检测 Ollama" 按钮

3. **流式响应支持**
   - `stream_chat()` 返回 `Stream<String>`
   - 解析 Ollama SSE 格式的每行 JSON，提取增量 content
   - Flutter UI 使用 `StreamBuilder` 逐字渲染

4. **性能测试**
   - 固定测试消息（"你好"、"请介绍一下自己"）运行 3 轮
   - 统计平均响应时间 + tokens/second
   - 结果展示在配置页

### Phase 3：查询增强与业务落地（2 周）

**目标**: 将 LLM 能力融入 SoloSoul 的具体业务场景。

1. **实现 `LlmQueryEnhancer`（Dart 版）**
   - 翻译 `_should_enhance_query` 规则集
   - Prompt 模板库：字段映射、申请理由生成、文本润色、翻译
   - 三层降级：LLM → 规则 → 原始输入
   - 所有 Prompt 都要求 JSON 结构化输出

2. **本地文件导入 AI 辅助**
   - `LOCAL_SEARCH_IMPORT_DESIGN.md` Phase 4 的"AI 辅助字段映射"
   - 调用本地 Ollama 模型，避免敏感文件内容上云
   - 输入：文件名 + 文件内容摘要 + SoloSoul Schema 定义
   - 输出：建议的字段映射关系（JSON 格式）

3. **非敏感文本处理**
   - 润色：优化用户输入的笔记/描述文本
   - 翻译：将内容翻译为其他语言
   - 摘要：对长文本生成简短摘要
   - 所有调用前必须经过 `LlmPrivacyFilter.checkBatch()` 脱敏检查

### Phase 4：高级架构扩展（后续版本）

1. **LLM 模型管理器**
   - 引入 `LlmModelState` 状态机
   - 支持热切换模型（卸载旧 → 加载新）
   - 后台健康检查 + 自动 fallback

2. **Embedding 基础设施**
   - 本地：通过 Ollama 运行 `nomic-embed-text` 等嵌入模型
   - 或 Rust FFI 集成轻量级嵌入库
   - 为本地语义搜索、相似内容推荐做准备

3. **MCP 服务器**
   - 暴露 SoloSoul 数据为 MCP Tool
   - `query_profile`, `search_objects`, `summarize_travel`, `analyze_financial`
   - 所有 Tool 都经过 `LlmPrivacyFilter` 脱敏检查
   - 用户可撤销授权、设置过期时间

---

## 五、代码复用清单

| xiaoyaosearch 源码位置 | 可复用逻辑 | SoloSoul 目标文件 |
|-----------------------|-----------|------------------|
| `openai_llm_service.py:211-223` | `_standardize_messages` 多类型输入标准化 | `llm_cloud_service.dart` |
| `openai_llm_service.py:225-250` | `_test_connection` 最小探活请求 | `llm_config_page.dart` 测试按钮逻辑 |
| `openai_llm_service.py:83-85` | API Key 日志脱敏 `f"{k[:7]}...{k[-4:]}"` | 全局日志工具或 `llm_service.dart` |
| `openai_llm_service.py:187-204` | 统一响应结构（content + model + provider + usage） | 新增 `llm_inference_response.dart` |
| `ollama_service.py:247-258` | `_check_ollama_service` 服务可用性检测 | `llm_local_service.dart` |
| `ollama_service.py:260-307` | `_check_model_exists` + `_pull_model` 模型管理 | `llm_local_service.dart` |
| `ollama_service.py:352-409` | `stream_chat` SSE 流式解析 | `llm_local_service.dart` Stream API |
| `ollama_service.py:437-493` | `benchmark_performance` 性能基准测试 | `llm_local_service.dart` + 配置页 |
| `ai_model_base.py:32-38` | `ModelStatus` 状态机枚举 | 新增 `llm_model_state.dart` |
| `ai_model_base.py:290-348` | `load_all_models` 并行加载 + 独立错误处理 | `llm_model_manager.dart` |
| `ai_model_manager.py:733-875` | `reload_model` 热重载流程 | `llm_model_manager.dart` |
| `llm_query_enhancer.py:95-115` | `_should_enhance_query` 智能判断规则 | `llm_query_enhancer.dart` |
| `llm_query_enhancer.py:117-133` | Prompt 模板结构（JSON 约束 + few-shot） | 业务 Prompt 模板库 |
| `llm_query_enhancer.py:135-155` | `_parse_simple_response` JSON 提取 + 规则回退 | `llm_response_parser.dart` |
| `openai_embedding_service.py:152-157` | `tenacity` 重试装饰器参数 | Dart `dio` `RetryInterceptor` 配置 |
| `openai_embedding_service.py:288-318` | 批量并发 Semaphore 模式 | 未来 Embedding 服务 |
| `mcp/tools/semantic_search.py:24-55` | MCP Tool 参数校验模式 | MCP Tool 实现模板 |
| `mcp/tools/semantic_search.py:58-70` | Tool 内可选 LLM 增强 + 失败回退 | MCP Tool 实现模板 |

---

## 六、风险与注意事项

### 1. 依赖膨胀风险

xiaoyaosearch 使用了 `numpy`, `torch`, `transformers`, `sentence-transformers` 等 Python 重型库，整体体积达数 GB。**SoloSoul 应坚持 HTTP API 调用模式**（直接调用 Ollama HTTP API 或 OpenAI REST API），而非在 Dart/Rust 侧本地加载 PyTorch 模型。这样可保持 App 包体积可控，且与现有架构兼容。

### 2. 隐私合规风险

xiaoyaosearch 虽然是本地优先搜索，但其隐私模型与 SoloSoul 不完全相同：
- xiaoyaosearch 的搜索索引默认本地存储，但用户可选择云端 LLM 增强查询
- SoloSoul 是**零知识架构**，所有敏感数据仅本地存储，绝不上传

**任何云端 LLM 调用必须经过 `LlmPrivacyFilter`**：
- `critical` 敏感度字段 → 整批拒绝
- `sensitive` 字段 → 脱敏为 `[REDACTED_SENSITIVE]`
- 保留完整的操作日志（`OperationEntry`），记录何时、调用了哪个模型、处理了哪些字段类型

### 3. 平台差异风险

Ollama 默认监听 `localhost:11434`，这存在平台限制：
- **macOS/Linux**: Ollama 可直接安装，无问题
- **iOS/Android**: 无法直接运行 Ollama，移动端本地 LLM 需要后续通过 Rust FFI 集成 `llama.cpp` 或 `candle`
- **Windows**: Ollama 支持，但路径和进程管理有差异

**建议策略**: Phase 2 先实现 Ollama HTTP 客户端，覆盖 Desktop 端；移动端本地 LLM 作为后续独立项目推进。

### 4. 状态恢复风险

xiaoyaosearch 的后端是常驻 Python 进程，模型状态常驻内存。SoloSoul Flutter App 可能被系统杀掉，需要设计**状态恢复机制**：
- App 启动时自动检测 Ollama 是否仍在运行（`GET /api/tags`）
- 云端模型无状态，每次 `infer()` 前检查网络连通性即可
- 使用 Riverpod `AsyncNotifier` 的 `build()` 方法做初始化检测，失败时自动进入 `error` 状态

### 5. 并发与线程安全

xiaoyaosearch 使用 Python `asyncio` 处理并发。SoloSoul Dart 端：
- HTTP 请求天然异步，无阻塞问题
- 本地 Rust FFI 推理（未来）需要确保在 background isolate 中执行
- `LlmService.infer()` 的文档中已注明"Heavy local inference runs on a Rust background thread pool"，需在实际实现中兑现

---

## 七、总结

xiaoyaosearch 的 LLM 集成代码展现了**清晰的分层架构**和**生产级的容错设计**：

1. **提供商抽象层**: `BaseAIModel` + 工厂模式，本地/云端无缝切换
2. **错误韧性**: 三层降级策略，LLM 失败绝不破坏核心业务
3. **工程细节**: API Key 脱敏、连接探活、批量并发、流式响应、性能基准
4. **生态扩展**: MCP 协议将搜索能力开放给外部 Agent

对于 SoloSoul 而言，**最大的借鉴价值在于推理执行层的实现模式**，而非 Python 特定的库依赖。建议按以下优先级推进：

| 优先级 | 模块 | 预计工作量 | 关键借鉴来源 |
|--------|------|-----------|-------------|
| P1 | 云端 LLM HTTP 客户端 | 3~5 天 | `openai_llm_service.py` |
| P2 | 本地 Ollama 集成 | 5~7 天 | `ollama_service.py` |
| P3 | 查询增强 + 业务 Prompt | 7~10 天 | `llm_query_enhancer.py` |
| P4 | 模型生命周期管理 | 3~5 天 | `ai_model_base.py` |
| P5 | Embedding 基础设施 | 后续版本 | `openai_embedding_service.py` |
| P6 | MCP 生态扩展 | 后续版本 | `mcp/server.py`, `mcp/tools/` |

SoloSoul 已具备业界领先的隐私安全脚手架（Vault 加密、`LlmPrivacyFilter`、操作日志）。现在最需要的，正是 xiaoyaosearch 所展示的**稳定、可靠、可降级的推理执行层**。将两者结合，可以快速将 SoloSoul 的 LLM 功能从 Debug-only 推进到用户可用的生产状态，同时坚守"数据永不离开本地"的核心承诺。

---

*本报告基于 xiaoyaosearch 2026-05-04 版本代码分析生成。*
