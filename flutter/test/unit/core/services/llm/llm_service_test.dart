import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:solosoul_flutter/core/services/llm/llm_service.dart';

// =============================================================================
// Test Helpers
// =============================================================================

String _defaultSuccessResponse() {
  return jsonEncode({
    'choices': [
      {
        'message': {'content': 'Hello world'},
        'finish_reason': 'stop',
      }
    ],
    'model': 'gpt-4o-mini',
    'usage': {
      'prompt_tokens': 5,
      'completion_tokens': 2,
      'total_tokens': 7,
    },
  });
}

String _defaultOllamaResponse() {
  return jsonEncode({
    'message': {'role': 'assistant', 'content': 'Local response'},
    'done': true,
    'prompt_eval_count': 3,
    'eval_count': 2,
  });
}

void main() {
  group('LlmMessage', () {
    test('toJson produces correct map', () {
      const msg = LlmMessage(role: 'user', content: 'hello');
      expect(msg.toJson(), {'role': 'user', 'content': 'hello'});
    });

    test('fromJson parses correctly', () {
      const msg = LlmMessage(role: 'system', content: 'you are helpful');
      final json = msg.toJson();
      final parsed = LlmMessage.fromJson(json);
      expect(parsed.role, 'system');
      expect(parsed.content, 'you are helpful');
    });
  });

  group('LlmTokenUsage', () {
    test('fromJson parses standard fields', () {
      final usage = LlmTokenUsage.fromJson({
        'prompt_tokens': 10,
        'completion_tokens': 20,
        'total_tokens': 30,
      });
      expect(usage.promptTokens, 10);
      expect(usage.completionTokens, 20);
      expect(usage.totalTokens, 30);
    });

    test('fromJson handles missing fields with defaults', () {
      final usage = LlmTokenUsage.fromJson({});
      expect(usage.promptTokens, 0);
      expect(usage.completionTokens, 0);
      expect(usage.totalTokens, 0);
    });
  });

  group('LlmCloudService', () {
    late Map<String, dynamic> lastRequestBody;
    late Map<String, String> lastRequestHeaders;

    http.Client createMockClient({
      required int statusCode,
      String? responseBody,
    }) {
      return MockClient((request) async {
        lastRequestBody = jsonDecode(request.body) as Map<String, dynamic>;
        lastRequestHeaders = {...request.headers};
        return http.Response(
          responseBody ?? _defaultSuccessResponse(),
          statusCode,
        );
      });
    }

    test('infer sends correct request body', () async {
      final client = createMockClient(statusCode: 200);
      final service = LlmCloudService(
        apiKey: 'sk-test123',
        model: 'gpt-4o-mini',
        client: client,
      );

      final result = await service.infer('Say hi', maxTokens: 100);

      expect(result, 'Hello world');
      expect(lastRequestBody['model'], 'gpt-4o-mini');
      expect(lastRequestBody['max_tokens'], 100);
      expect(lastRequestBody['temperature'], 0.7);
      expect(lastRequestBody['messages'], [
        {'role': 'user', 'content': 'Say hi'}
      ]);
    });

    test('inferMessages supports multi-turn conversation', () async {
      final client = createMockClient(statusCode: 200);
      final service = LlmCloudService(
        apiKey: 'sk-test123',
        client: client,
      );

      await service.inferMessages([
        const LlmMessage(role: 'system', content: 'You are helpful'),
        const LlmMessage(role: 'user', content: 'Hello'),
      ]);

      final messages = lastRequestBody['messages'] as List<dynamic>;
      expect(messages.length, 2);
      expect(messages[0]['role'], 'system');
      expect(messages[1]['role'], 'user');
    });

    test('sets Authorization header correctly', () async {
      final client = createMockClient(statusCode: 200);
      final service = LlmCloudService(
        apiKey: 'sk-secret',
        client: client,
      );

      await service.infer('test');

      expect(lastRequestHeaders['Authorization'], 'Bearer sk-secret');
      expect(lastRequestHeaders['Content-Type'], 'application/json');
    });

    test('throws unauthorized on 401', () async {
      final client = createMockClient(
        statusCode: 401,
        responseBody: jsonEncode({'error': {'message': 'Invalid key'}}),
      );
      final service = LlmCloudService(apiKey: 'bad-key', client: client);

      expect(
        () => service.infer('test'),
        throwsA(isA<LlmException>().having(
          (e) => e.code,
          'code',
          LlmErrorCode.unauthorized,
        )),
      );
    });

    test('throws rateLimited on 429', () async {
      final client = createMockClient(statusCode: 429);
      final service = LlmCloudService(apiKey: 'key', client: client);

      expect(
        () => service.infer('test'),
        throwsA(isA<LlmException>().having(
          (e) => e.code,
          'code',
          LlmErrorCode.rateLimited,
        )),
      );
    });

    test('throws timeout on delayed response', () async {
      final client = MockClient((_) async {
        await Future.delayed(const Duration(seconds: 10));
        return http.Response('', 200);
      });
      final service = LlmCloudService(
        apiKey: 'key',
        timeout: const Duration(milliseconds: 50),
        client: client,
      );

      expect(
        () => service.infer('test'),
        throwsA(isA<LlmException>().having(
          (e) => e.code,
          'code',
          LlmErrorCode.timeout,
        )),
      );
    });

    test('testConnection sends minimal request', () async {
      final client = createMockClient(statusCode: 200);
      final service = LlmCloudService(apiKey: 'key', client: client);

      await service.testConnection();

      expect(lastRequestBody['max_tokens'], 10);
      final messages = lastRequestBody['messages'] as List<dynamic>;
      expect(messages[0]['content'], 'Hi');
    });

    test('maskApiKey masks correctly', () {
      expect(LlmCloudService.maskApiKey('sk-abcdefghijklmnopqrstuvwxyz'), 'sk-abcd...wxyz');
      expect(LlmCloudService.maskApiKey('short'), '***');
    });

    test('endpoint trailing slash is handled', () {
      final service = LlmCloudService(apiKey: 'k', endpoint: 'https://api.example.com/v1/');
      expect(service.endpoint, 'https://api.example.com/v1/');
    });
  });

  group('LlmLocalService (Ollama)', () {
    http.Client createMockClient({
      required int statusCode,
      String? responseBody,
    }) {
      return MockClient((request) async {
        return http.Response(
          responseBody ?? _defaultOllamaResponse(),
          statusCode,
        );
      });
    }

    test('infer sends correct Ollama format', () async {
      final client = createMockClient(statusCode: 200);
      final service = LlmLocalService(
        modelName: 'qwen2.5:1.5b',
        client: client,
      );

      final result = await service.infer('Hello');
      expect(result, 'Local response');
    });

    test('checkStatus returns correct state when service running', () async {
      final client = MockClient((request) async {
        if (request.url.path == '/api/tags') {
          return http.Response(
            jsonEncode({
              'models': [
                {'name': 'qwen2.5:1.5b'},
                {'name': 'llama3.2:latest'},
              ]
            }),
            200,
          );
        }
        return http.Response('', 404);
      });
      final service = LlmLocalService(
        modelName: 'qwen2.5:1.5b',
        client: client,
      );

      final status = await service.checkStatus();
      expect(status.serviceRunning, true);
      expect(status.modelAvailable, true);
      expect(status.installedModels, contains('qwen2.5'));
    });

    test('checkStatus returns not running when service down', () async {
      final client = MockClient((_) async {
        throw Exception('Connection refused');
      });
      final service = LlmLocalService(client: client);

      final status = await service.checkStatus();
      expect(status.serviceRunning, false);
      expect(status.modelAvailable, false);
    });

    test('streamChat yields content fragments', () async {
      final client = _MockStreamedClient([
        jsonEncode({'message': {'content': 'Hello'}}),
        jsonEncode({'message': {'content': ' world'}}),
      ]);
      final service = LlmLocalService(client: client);

      final fragments = await service.streamChat('Hi').toList();
      expect(fragments, ['Hello', ' world']);
    });
  });

  group('LlmException', () {
    test('toString includes code and message', () {
      const ex = LlmException('Something went wrong', code: LlmErrorCode.network);
      expect(ex.toString(), 'LlmException[LlmErrorCode.network]: Something went wrong');
    });
  });
}

/// Custom mock client that returns a streamed response for testing streamChat.
class _MockStreamedClient extends http.BaseClient {
  final List<String> _lines;

  _MockStreamedClient(this._lines);

  @override
  Future<http.StreamedResponse> send(http.BaseRequest request) async {
    final body = _lines.map((l) => utf8.encode('$l\n')).expand((b) => b).toList();
    final stream = Stream.fromIterable([body]);
    return http.StreamedResponse(stream, 200);
  }
}
