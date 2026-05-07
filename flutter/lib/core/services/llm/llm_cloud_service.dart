import 'dart:async';
import 'dart:convert';

import 'package:http/http.dart' as http;

import 'llm_message.dart';
import 'llm_response.dart';
import 'llm_api_error.dart';
import 'llm_exception.dart';
import 'llm_cloud_provider_type.dart';
import 'llm_service_interface.dart';

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
          throw const LlmException('API returned empty choices', code: LlmErrorCode.unknown);
        }
        final first = choices.first;
        if (first is! Map<String, dynamic>) {
          throw const LlmException('API returned invalid choice format', code: LlmErrorCode.unknown);
        }
        final message = first['message'];
        final messageMap = message is Map<String, dynamic> ? message : null;
        final content = messageMap?['content'] as String? ?? '';
        final finishReason = first['finish_reason'] as String?;
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
          throw const LlmException('API returned empty content', code: LlmErrorCode.unknown);
        }
        // Anthropic 格式：content 为 block 数组，text 类型块持有实际回复文本
        final textParts = <String>[];
        for (final block in contentList) {
          if (block is! Map<String, dynamic>) continue;
          if (block['type'] == 'text') {
            final text = block['text'] as String?;
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
        final rawError = json['error'];
        final errorObj = rawError is Map<String, dynamic> ? rawError : null;
        return LlmApiError(
          message: errorObj?['message']?.toString() ?? 'Unknown error',
          type: errorObj?['type']?.toString() ?? 'unknown',
          statusCode: statusCode,
        );
      case LlmCloudProviderType.anthropic:
        // Anthropic: {"type":"error","error":{"type":"...","message":"..."}}
        final rawError = json['error'];
        final errorObj = rawError is Map<String, dynamic> ? rawError : null;
        return LlmApiError(
          message: errorObj?['message']?.toString() ?? 'Unknown error',
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
      throw const LlmException('Request timed out', code: LlmErrorCode.timeout);
    } on FormatException catch (e) {
      throw LlmException('Invalid endpoint URL: $e', code: LlmErrorCode.network);
    } on Exception catch (e) {
      throw LlmException('Network request failed: $e', code: LlmErrorCode.network);
    }

    // Status-code mapping
    if (response.statusCode != 200) {
      final errorBody = _safeDecode(response.body);
      final apiError = _parseError(errorBody, response.statusCode);

      switch (response.statusCode) {
        case 401:
          throw LlmException('Invalid API Key: ${apiError.message}', code: LlmErrorCode.unauthorized);
        case 429:
          throw LlmException('Rate limit exceeded: ${apiError.message}', code: LlmErrorCode.rateLimited);
        case 500:
        case 502:
        case 503:
          throw LlmException('Server error (${response.statusCode}): ${apiError.message}', code: LlmErrorCode.network);
        default:
          throw LlmException('API error (${response.statusCode}): ${apiError.message}', code: LlmErrorCode.unknown);
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
        'Streaming request failed (${streamedResponse.statusCode}): ${apiError.message}',
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
          throw LlmException('Streaming error: ${apiError.message}', code: LlmErrorCode.unknown);
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
        final first = choices[0];
        if (first is! Map<String, dynamic>) return null;
        final delta = first['delta'];
        final deltaMap = delta is Map<String, dynamic> ? delta : null;
        return deltaMap?['content'] as String?;
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

