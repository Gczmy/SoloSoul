import 'dart:async';
import 'dart:convert';
import 'dart:math';

import 'package:http/http.dart' as http;

// =============================================================================
// LLM Backend Type
// =============================================================================

/// Supported LLM inference backends.
enum LlmBackendType {
  /// Local model inference via HTTP API (Ollama) or Rust native layer.
  local,

  /// Cloud API (OpenAI-compatible, Anthropic, self-hosted endpoint).
  cloud,
}

// =============================================================================
// LLM Cloud Provider Type
// =============================================================================

/// 云端 LLM API 提供商类型。
enum LlmCloudProviderType {
  /// OpenAI-compatible API（含 DeepSeek、Moonshot、DashScope 等）。
  openai,

  /// Anthropic Messages API。
  anthropic,
}

extension LlmCloudProviderTypeExtension on LlmCloudProviderType {
  String get label => switch (this) {
        LlmCloudProviderType.openai => 'OpenAI',
        LlmCloudProviderType.anthropic => 'Anthropic',
      };

  String toJson() => name;

  static LlmCloudProviderType fromJson(String? raw) {
    if (raw == null) return LlmCloudProviderType.openai;
    return switch (raw) {
      'openai' => LlmCloudProviderType.openai,
      'anthropic' => LlmCloudProviderType.anthropic,
      _ => LlmCloudProviderType.openai,
    };
  }
}

// =============================================================================
// LLM Message
// =============================================================================

/// A single message in a multi-turn conversation.
///
/// Follows the OpenAI chat completions message format.
class LlmMessage {
  final String role; // 'system' | 'user' | 'assistant'
  final String content;

  const LlmMessage({required this.role, required this.content});

  Map<String, String> toJson() => {'role': role, 'content': content};

  factory LlmMessage.fromJson(Map<String, dynamic> json) => LlmMessage(
        role: json['role'] as String,
        content: json['content'] as String,
      );

  @override
  String toString() => 'LlmMessage($role: ${content.substring(0, content.length > 30 ? 30 : content.length)}...)';
}

// =============================================================================
// LLM Inference Response
// =============================================================================

/// Standardized response from any LLM provider (cloud or local).
class LlmInferenceResponse {
  final String content;
  final String model;
  final String provider;
  final String? finishReason;
  final LlmTokenUsage usage;

  const LlmInferenceResponse({
    required this.content,
    required this.model,
    required this.provider,
    this.finishReason,
    this.usage = const LlmTokenUsage(),
  });

  static const empty = LlmInferenceResponse(
    content: '',
    model: '',
    provider: '',
  );
}

/// Token usage statistics from an inference call.
class LlmTokenUsage {
  final int promptTokens;
  final int completionTokens;
  final int totalTokens;

  const LlmTokenUsage({
    this.promptTokens = 0,
    this.completionTokens = 0,
    this.totalTokens = 0,
  });

  factory LlmTokenUsage.fromJson(Map<String, dynamic> json) => LlmTokenUsage(
        promptTokens: json['prompt_tokens'] as int? ?? 0,
        completionTokens: json['completion_tokens'] as int? ?? 0,
        totalTokens: json['total_tokens'] as int? ?? 0,
      );
}

// =============================================================================
// LLM Service Interface
// =============================================================================

/// Abstract interface for LLM inference.
///
/// **Thread safety:** All implementations must guarantee that [infer] is safe
/// to call concurrently. Heavy local inference runs on a background isolate;
/// cloud calls are isolated per [LlmSession].
abstract class LlmService {
  /// Perform a single-turn inference.
  ///
  /// [prompt] – sanitized prompt string (caller must strip PII before cloud).
  /// [maxTokens] – hard cap on output length.
  ///
  /// Returns the generated text, or throws [LlmException] on failure.
  Future<String> infer(String prompt, {int maxTokens = 512});

  /// Perform multi-turn inference with message history.
  ///
  /// [messages] – conversation history including system prompt.
  /// [maxTokens] – hard cap on output length.
  ///
  /// Returns the generated text, or throws [LlmException] on failure.
  Future<String> inferMessages(List<LlmMessage> messages, {int maxTokens = 512});

  /// Test whether the service is reachable and functional.
  ///
  /// Sends a minimal inference request (e.g. max_tokens=10).
  /// Returns normally if healthy, throws [LlmException] otherwise.
  Future<void> testConnection();

  /// Token usage from the most recent inference call.
  ///
  /// Returns [LlmTokenUsage.empty] if no inference has been performed yet,
  /// or if the underlying provider does not report token counts (e.g. stream).
  LlmTokenUsage get lastTokenUsage;

  /// Release resources held by this service (e.g. HTTP clients).
  void dispose();
}

// =============================================================================
// Cloud LLM Service (OpenAI + Anthropic)
// =============================================================================

/// Production implementation for cloud LLM APIs.
///
/// Supports OpenAI-compatible providers (OpenAI, DeepSeek, Moonshot,
/// DashScope, etc.) and Anthropic Messages API.
///
/// 内部通过 [LlmCloudProviderType] 分支处理请求构造、响应解析、
/// 流式 SSE 解析和错误解析的差异。
class LlmCloudService implements LlmService {
  final String apiKey;
  final String endpoint;
  final String model;
  final LlmCloudProviderType provider;
  final String anthropicVersion;

  /// Request timeout.
  final Duration timeout;

  /// HTTP client — can be overridden for testing.
  final http.Client _client;

  LlmTokenUsage _lastTokenUsage = const LlmTokenUsage();

  LlmCloudService({
    required this.apiKey,
    this.endpoint = 'https://api.openai.com/v1',
    this.model = 'gpt-4o-mini',
    this.provider = LlmCloudProviderType.openai,
    this.anthropicVersion = '2023-06-01',
    this.timeout = const Duration(seconds: 60),
    http.Client? client,
  }) : _client = client ?? http.Client();

  @override
  LlmTokenUsage get lastTokenUsage => _lastTokenUsage;

  @override
  void dispose() {
    _client.close();
  }

  // ---------------------------------------------------------------------------
  // Provider-specific request construction
  // ---------------------------------------------------------------------------

  String get _url {
    final base = endpoint.endsWith('/') ? endpoint.substring(0, endpoint.length - 1) : endpoint;
    return switch (provider) {
      LlmCloudProviderType.openai => '$base/chat/completions',
      LlmCloudProviderType.anthropic => '$base/v1/messages',
    };
  }

  Map<String, String> get _headers => switch (provider) {
        LlmCloudProviderType.openai => {
            'Authorization': 'Bearer $apiKey',
            'Content-Type': 'application/json',
          },
        LlmCloudProviderType.anthropic => {
            'x-api-key': apiKey,
            'anthropic-version': anthropicVersion,
            'Content-Type': 'application/json',
          },
      };

  Map<String, dynamic> _buildRequestBody({
    required List<LlmMessage> messages,
    required int maxTokens,
    required double temperature,
    double topP = 1.0,
  }) {
    switch (provider) {
      case LlmCloudProviderType.openai:
        return {
          'model': model,
          'messages': messages.map((m) => m.toJson()).toList(),
          'max_tokens': maxTokens,
          'temperature': temperature,
          'top_p': topP,
        };
      case LlmCloudProviderType.anthropic:
        // Anthropic: system 消息提取为顶层参数
        String? systemPrompt;
        final chatMessages = <LlmMessage>[];
        for (final m in messages) {
          if (m.role == 'system') {
            systemPrompt = m.content;
          } else {
            chatMessages.add(m);
          }
        }
        final body = <String, dynamic>{
          'model': model,
          'messages': chatMessages.map((m) => m.toJson()).toList(),
          'max_tokens': maxTokens,
          'temperature': temperature,
        };
        if (systemPrompt != null) {
          body['system'] = systemPrompt;
        }
        return body;
    }
  }

  // ---------------------------------------------------------------------------
  // Provider-specific response parsing
  // ---------------------------------------------------------------------------

  LlmInferenceResponse _parseResponse(Map<String, dynamic> json) {
    switch (provider) {
      case LlmCloudProviderType.openai:
        final choices = json['choices'] as List<dynamic>?;
        if (choices == null || choices.isEmpty) {
          throw const LlmException('API 返回空 choices', code: LlmErrorCode.unknown);
        }
        final choice = choices.first as Map<String, dynamic>;
        final message = choice['message'] as Map<String, dynamic>?;
        final content = message?['content'] as String? ?? '';
        final finishReason = choice['finish_reason'] as String?;
        final usageJson = json['usage'] as Map<String, dynamic>?;
        final usage = usageJson != null ? LlmTokenUsage.fromJson(usageJson) : const LlmTokenUsage();
        return LlmInferenceResponse(
          content: content,
          model: json['model'] as String? ?? model,
          provider: 'cloud-openai',
          finishReason: finishReason,
          usage: usage,
        );
      case LlmCloudProviderType.anthropic:
        final contentList = json['content'] as List<dynamic>?;
        if (contentList == null || contentList.isEmpty) {
          throw const LlmException('API 返回空 content', code: LlmErrorCode.unknown);
        }
        // Anthropic 格式：content 为 block 数组，text 类型块持有实际回复文本
        final textParts = <String>[];
        for (final block in contentList) {
          final blockMap = block as Map<String, dynamic>;
          if (blockMap['type'] == 'text') {
            final text = blockMap['text'] as String?;
            if (text != null && text.isNotEmpty) {
              textParts.add(text);
            }
          }
        }
        final content = textParts.join('');
        final finishReason = json['stop_reason'] as String?;
        final usageJson = json['usage'] as Map<String, dynamic>?;
        final usage = usageJson != null
            ? LlmTokenUsage(
                promptTokens: usageJson['input_tokens'] as int? ?? 0,
                completionTokens: usageJson['output_tokens'] as int? ?? 0,
                totalTokens: (usageJson['input_tokens'] as int? ?? 0) +
                    (usageJson['output_tokens'] as int? ?? 0),
              )
            : const LlmTokenUsage();
        return LlmInferenceResponse(
          content: content,
          model: json['model'] as String? ?? model,
          provider: 'cloud-anthropic',
          finishReason: finishReason,
          usage: usage,
        );
    }
  }

  // ---------------------------------------------------------------------------
  // Provider-specific error parsing
  // ---------------------------------------------------------------------------

  LlmApiError _parseError(Map<String, dynamic> json, int statusCode) {
    switch (provider) {
      case LlmCloudProviderType.openai:
        final errorObj = json['error'] as Map<String, dynamic>?;
        return LlmApiError(
          message: errorObj?['message']?.toString() ?? '未知错误',
          type: errorObj?['type']?.toString() ?? 'unknown',
          statusCode: statusCode,
        );
      case LlmCloudProviderType.anthropic:
        // Anthropic: {"type":"error","error":{"type":"...","message":"..."}}
        final errorObj = json['error'] as Map<String, dynamic>?;
        return LlmApiError(
          message: errorObj?['message']?.toString() ?? '未知错误',
          type: errorObj?['type']?.toString() ?? 'unknown',
          statusCode: statusCode,
        );
    }
  }

  // ---------------------------------------------------------------------------
  // Public API
  // ---------------------------------------------------------------------------

  @override
  Future<String> infer(String prompt, {int maxTokens = 512}) async {
    return inferMessages(
      [LlmMessage(role: 'user', content: prompt)],
      maxTokens: maxTokens,
    );
  }

  @override
  Future<String> inferMessages(List<LlmMessage> messages, {int maxTokens = 512}) async {
    final response = await _chatCompletion(
      messages: messages,
      maxTokens: maxTokens,
      temperature: 0.7,
    );
    _lastTokenUsage = response.usage;
    return response.content;
  }

  /// Low-level chat completion with full parameter control.
  Future<LlmInferenceResponse> chatCompletion({
    required List<LlmMessage> messages,
    int maxTokens = 512,
    double temperature = 0.7,
    double topP = 1.0,
  }) async {
    return _chatCompletion(
      messages: messages,
      maxTokens: maxTokens,
      temperature: temperature,
      topP: topP,
    );
  }

  Future<LlmInferenceResponse> _chatCompletion({
    required List<LlmMessage> messages,
    required int maxTokens,
    required double temperature,
    double topP = 1.0,
  }) async {
    final body = jsonEncode(_buildRequestBody(
      messages: messages,
      maxTokens: maxTokens,
      temperature: temperature,
      topP: topP,
    ));

    http.Response response;
    try {
      response = await _client
          .post(
            Uri.parse(_url),
            headers: _headers,
            body: body,
          )
          .timeout(timeout);
    } on TimeoutException {
      throw const LlmException('请求超时', code: LlmErrorCode.timeout);
    } on FormatException catch (e) {
      throw LlmException('无效的 endpoint URL: $e', code: LlmErrorCode.network);
    } on Exception catch (e) {
      throw LlmException('网络请求失败: $e', code: LlmErrorCode.network);
    }

    // Status-code mapping
    if (response.statusCode != 200) {
      final errorBody = _safeDecode(response.body);
      final apiError = _parseError(errorBody, response.statusCode);

      switch (response.statusCode) {
        case 401:
          throw LlmException('API Key 无效: ${apiError.message}', code: LlmErrorCode.unauthorized);
        case 429:
          throw LlmException('请求频率超限: ${apiError.message}', code: LlmErrorCode.rateLimited);
        case 500:
        case 502:
        case 503:
          throw LlmException('服务端错误 (${response.statusCode}): ${apiError.message}', code: LlmErrorCode.network);
        default:
          throw LlmException('API 错误 (${response.statusCode}): ${apiError.message}', code: LlmErrorCode.unknown);
      }
    }

    return _parseResponse(_safeDecode(response.body));
  }

  /// Stream chat response from cloud API (SSE).
  ///
  /// Supports both OpenAI-compatible and Anthropic streaming formats.
  Stream<String> streamChat(
    String prompt, {
    List<LlmMessage>? history,
    int maxTokens = 512,
  }) async* {
    final messages = <LlmMessage>[
      ...?history,
      LlmMessage(role: 'user', content: prompt),
    ];

    final body = jsonEncode(_buildRequestBody(
      messages: messages,
      maxTokens: maxTokens,
      temperature: 0.7,
      topP: 1.0,
    )..['stream'] = true);

    final request = http.Request('POST', Uri.parse(_url))
      ..headers.addAll(_headers)
      ..body = body;

    final streamedResponse = await _client.send(request).timeout(timeout);

    if (streamedResponse.statusCode != 200) {
      final bodyStr = await streamedResponse.stream.bytesToString();
      final errorBody = _safeDecode(bodyStr);
      final apiError = _parseError(errorBody, streamedResponse.statusCode);
      throw LlmException(
        '流式请求失败 (${streamedResponse.statusCode}): ${apiError.message}',
        code: LlmErrorCode.network,
      );
    }

    // SSE parsing
    String? currentEvent;
    await for (final line in streamedResponse.stream
        .transform(utf8.decoder)
        .transform(const LineSplitter())) {
      final trimmed = line.trim();
      if (trimmed.isEmpty) {
        currentEvent = null;
        continue;
      }

      // OpenAI SSE: data: {...}
      // Anthropic SSE: event: content_block_delta\ndata: {...}
      if (trimmed.startsWith('event:')) {
        currentEvent = trimmed.substring(6).trim();
        continue;
      }
      if (!trimmed.startsWith('data:')) continue;

      final dataStr = trimmed.substring(5).trim();
      if (dataStr == '[DONE]') break; // OpenAI 结束标记

      try {
        final json = jsonDecode(dataStr) as Map<String, dynamic>;

        // 错误检查（两种格式都可能在流中返回错误）
        if (json['error'] != null || json['type'] == 'error') {
          final apiError = _parseError(json, streamedResponse.statusCode);
          throw LlmException('流式错误: ${apiError.message}', code: LlmErrorCode.unknown);
        }

        final chunk = _parseStreamEvent(json, currentEvent);
        if (chunk != null && chunk.isNotEmpty) {
          yield chunk;
        }
      } on FormatException catch (_) {
        // Skip malformed SSE lines
      }
    }
  }

  /// 解析单条 SSE 事件中的增量文本。
  String? _parseStreamEvent(Map<String, dynamic> json, String? eventType) {
    switch (provider) {
      case LlmCloudProviderType.openai:
        final choices = json['choices'] as List<dynamic>?;
        if (choices == null || choices.isEmpty) return null;
        final delta = choices[0]['delta'] as Map<String, dynamic>?;
        return delta?['content'] as String?;
      case LlmCloudProviderType.anthropic:
        // 忽略非内容事件：message_start, content_block_start, content_block_stop, message_stop
        if (eventType != null && eventType != 'content_block_delta') return null;
        if (json['type'] == 'content_block_delta') {
          final delta = json['delta'] as Map<String, dynamic>?;
          return delta?['text'] as String?;
        }
        return null;
    }
  }

  @override
  Future<void> testConnection() async {
    await _chatCompletion(
      messages: const [LlmMessage(role: 'user', content: 'Hi')],
      maxTokens: 10,
      temperature: 0.0,
      topP: 1.0,
    );
  }

  Map<String, dynamic> _safeDecode(String body) {
    try {
      return jsonDecode(body) as Map<String, dynamic>;
    } on FormatException catch (_) {
      return const {};
    }
  }

  /// Masks an API key for safe logging.
  static String maskApiKey(String key) {
    if (key.length <= 11) return '***';
    return '${key.substring(0, 7)}...${key.substring(key.length - 4)}';
  }
}

// =============================================================================
// LLM API Error (unified across providers)
// =============================================================================

/// 统一的云端 API 错误结构，屏蔽 OpenAI / Anthropic 的格式差异。
class LlmApiError {
  final String message;
  final String type;
  final int statusCode;

  const LlmApiError({
    required this.message,
    required this.type,
    required this.statusCode,
  });
}

// =============================================================================
// Local LLM Service (Ollama)
// =============================================================================

/// Local LLM inference via Ollama HTTP API.
///
/// Communicates with `http://localhost:11434/api/chat` by default.
/// Supports streaming responses via [streamChat].
class LlmLocalService implements LlmService {
  final String baseUrl;
  final String modelName;
  final Duration timeout;

  /// Default generation parameters.
  final double temperature;
  final double topP;
  final int numPredict;
  final int numCtx;

  final http.Client _client;

  LlmTokenUsage _lastTokenUsage = const LlmTokenUsage();

  LlmLocalService({
    this.baseUrl = 'http://localhost:11434',
    this.modelName = 'qwen2.5:1.5b',
    this.timeout = const Duration(seconds: 60),
    this.temperature = 0.7,
    this.topP = 0.9,
    this.numPredict = 2048,
    this.numCtx = 2048,
    http.Client? client,
  }) : _client = client ?? http.Client();

  @override
  LlmTokenUsage get lastTokenUsage => _lastTokenUsage;

  @override
  void dispose() {
    _client.close();
  }

  String get _chatUrl => '$baseUrl/api/chat';
  String get _tagsUrl => '$baseUrl/api/tags';
  String get _pullUrl => '$baseUrl/api/pull';

  @override
  Future<String> infer(String prompt, {int maxTokens = 512}) async {
    return inferMessages(
      [LlmMessage(role: 'user', content: prompt)],
      maxTokens: maxTokens,
    );
  }

  @override
  Future<String> inferMessages(List<LlmMessage> messages, {int maxTokens = 512}) async {
    final response = await _chat(
      messages: messages,
      stream: false,
      options: {
        'temperature': temperature,
        'top_p': topP,
        'num_predict': maxTokens,
        'num_ctx': numCtx,
      },
    );
    _lastTokenUsage = response.usage;
    return response.content;
  }

  /// Low-level chat request to Ollama.
  Future<LlmInferenceResponse> _chat({
    required List<LlmMessage> messages,
    required bool stream,
    Map<String, dynamic> options = const {},
  }) async {
    final body = jsonEncode({
      'model': modelName,
      'messages': messages.map((m) => m.toJson()).toList(),
      'options': options,
      'stream': stream,
    });

    http.Response response;
    try {
      response = await _client
          .post(
            Uri.parse(_chatUrl),
            headers: {'Content-Type': 'application/json'},
            body: body,
          )
          .timeout(timeout);
    } on TimeoutException {
      throw const LlmException('Ollama 请求超时', code: LlmErrorCode.timeout);
    } on Exception catch (e) {
      throw LlmException('无法连接到 Ollama: $e', code: LlmErrorCode.network);
    }

    if (response.statusCode != 200) {
      throw LlmException(
        'Ollama 错误 (${response.statusCode}): ${response.body}',
        code: LlmErrorCode.modelNotFound,
      );
    }

    final json = _safeDecode(response.body);
    final message = json['message'] as Map<String, dynamic>?;
    final content = message?['content'] as String? ?? '';

    final promptEvalCount = json['prompt_eval_count'] as int? ?? 0;
    final evalCount = json['eval_count'] as int? ?? 0;

    return LlmInferenceResponse(
      content: content,
      model: modelName,
      provider: 'local',
      usage: LlmTokenUsage(
        promptTokens: promptEvalCount,
        completionTokens: evalCount,
        totalTokens: promptEvalCount + evalCount,
      ),
    );
  }

  /// Stream chat response from Ollama.
  ///
  /// Yields text fragments as they are generated.
  Stream<String> streamChat(
    String prompt, {
    List<LlmMessage>? history,
    int maxTokens = 512,
  }) async* {
    final messages = <LlmMessage>[...?history, LlmMessage(role: 'user', content: prompt)];

    final body = jsonEncode({
      'model': modelName,
      'messages': messages.map((m) => m.toJson()).toList(),
      'options': {
        'temperature': temperature,
        'num_predict': maxTokens,
      },
      'stream': true,
    });

    final request = http.Request('POST', Uri.parse(_chatUrl))
      ..headers['Content-Type'] = 'application/json'
      ..body = body;

    final streamedResponse = await _client.send(request).timeout(timeout);

    if (streamedResponse.statusCode != 200) {
      final bodyStr = await streamedResponse.stream.bytesToString();
      throw LlmException(
        'Ollama 流式错误 (${streamedResponse.statusCode}): $bodyStr',
        code: LlmErrorCode.modelNotFound,
      );
    }

    await for (final chunk in streamedResponse.stream.transform(utf8.decoder).transform(const LineSplitter())) {
      if (chunk.trim().isEmpty) continue;
      try {
        final json = jsonDecode(chunk) as Map<String, dynamic>;
        final message = json['message'] as Map<String, dynamic>?;
        final content = message?['content'] as String? ?? '';
        if (content.isNotEmpty) {
          yield content;
        }
      } on FormatException catch (_) {
        // Skip malformed lines
      }
    }
  }

  @override
  Future<void> testConnection() async {
    await _chat(
      messages: const [LlmMessage(role: 'user', content: '你好')],
      stream: false,
      options: {'temperature': temperature, 'num_predict': 10},
    );
  }

  /// Check whether Ollama service is running and the model exists.
  Future<OllamaStatus> checkStatus() async {
    try {
      final response = await _client.get(Uri.parse(_tagsUrl)).timeout(const Duration(seconds: 5));
      if (response.statusCode != 200) {
        return const OllamaStatus(serviceRunning: false, modelAvailable: false);
      }

      final json = _safeDecode(response.body);
      final models = json['models'] as List<dynamic>? ?? [];
      final modelNames = models.map((m) {
        final name = m['name'] as String? ?? '';
        return name.split(':').first;
      }).toSet();

      final targetBaseName = modelName.split(':').first;
      final available = modelNames.contains(targetBaseName);

      return OllamaStatus(
        serviceRunning: true,
        modelAvailable: available,
        installedModels: modelNames.toList(),
      );
    } on Exception catch (_) {
      return const OllamaStatus(serviceRunning: false, modelAvailable: false);
    }
  }

  /// Pull a model from Ollama registry.
  ///
  /// Returns a stream of progress strings.
  Stream<String> pullModel() async* {
    final body = jsonEncode({'name': modelName});

    final request = http.Request('POST', Uri.parse(_pullUrl))
      ..headers['Content-Type'] = 'application/json'
      ..body = body;

    final streamedResponse = await _client.send(request).timeout(timeout);

    if (streamedResponse.statusCode != 200) {
      final bodyStr = await streamedResponse.stream.bytesToString();
      throw LlmException(
        '拉取模型失败 (${streamedResponse.statusCode}): $bodyStr',
        code: LlmErrorCode.modelNotFound,
      );
    }

    await for (final chunk in streamedResponse.stream.transform(utf8.decoder).transform(const LineSplitter())) {
      if (chunk.trim().isEmpty) continue;
      try {
        final json = jsonDecode(chunk) as Map<String, dynamic>;
        final status = json['status'] as String? ?? '';
        if (status.isNotEmpty) {
          yield status;
        }
      } on FormatException catch (_) {
        // Skip malformed lines
      }
    }
  }

  Map<String, dynamic> _safeDecode(String body) {
    try {
      return jsonDecode(body) as Map<String, dynamic>;
    } on FormatException catch (_) {
      return {};
    }
  }
}

// =============================================================================
// Ollama Status
// =============================================================================

class OllamaStatus {
  final bool serviceRunning;
  final bool modelAvailable;
  final List<String> installedModels;

  const OllamaStatus({
    required this.serviceRunning,
    required this.modelAvailable,
    this.installedModels = const [],
  });

  bool get isReady => serviceRunning && modelAvailable;
}

// =============================================================================
// Session Manager
// =============================================================================

/// Lightweight session handle for tracking LLM usage and applying
/// per-account rate limits / privacy gates.
class LlmSession {
  final String sessionId;
  final DateTime createdAt;
  final LlmBackendType backend;

  LlmSession._(this.sessionId, this.backend) : createdAt = DateTime.now();

  factory LlmSession.create(LlmBackendType backend) {
    final id = _generateSessionId();
    return LlmSession._(id, backend);
  }

  static String _generateSessionId() {
    final bytes = List<int>.generate(16, (_) => Random.secure().nextInt(256));
    return base64Url.encode(bytes).substring(0, 22);
  }
}

// =============================================================================
// Exceptions
// =============================================================================

class LlmException implements Exception {
  final String message;
  final LlmErrorCode code;

  const LlmException(this.message, {this.code = LlmErrorCode.unknown});

  @override
  String toString() => 'LlmException[$code]: $message';
}

enum LlmErrorCode {
  unknown,
  timeout,
  network,
  unauthorized,
  rateLimited,
  privacyBlocked,
  modelNotFound,
}
