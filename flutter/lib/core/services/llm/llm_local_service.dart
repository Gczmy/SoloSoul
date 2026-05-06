import 'dart:async';
import 'dart:convert';

import 'package:http/http.dart' as http;

import 'llm_message.dart';
import 'llm_response.dart';
import 'llm_exception.dart';
import 'llm_service_interface.dart';
import 'ollama_status.dart';

class LlmLocalService implements LlmService {
  /// Default local model name (Ollama).
  static const String defaultModelName = 'qwen2.5:1.5b';

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
    this.modelName = defaultModelName,
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

  String get _normalizedBaseUrl {
    return baseUrl.endsWith('/')
        ? baseUrl.substring(0, baseUrl.length - 1)
        : baseUrl;
  }

  String get _chatUrl => '$_normalizedBaseUrl/api/chat';
  String get _tagsUrl => '$_normalizedBaseUrl/api/tags';
  String get _pullUrl => '$_normalizedBaseUrl/api/pull';

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
        'top_p': topP,
        'num_predict': maxTokens,
        'num_ctx': numCtx,
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

