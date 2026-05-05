import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/llm/llm_model_manager.dart';
import 'package:solosoul_flutter/core/services/llm/llm_model_state.dart';
import 'package:solosoul_flutter/core/services/llm/llm_service.dart';

void main() {
  group('LlmModelManager', () {
    late LlmModelManager manager;

    setUp(() async {
      manager = LlmModelManager.instance;
      // Reset to clean state before each test
      await manager.unload();
      manager.resetStats();
    });

    test('initial state is unloaded', () {
      expect(manager.state, LlmModelState.unloaded);
      expect(manager.isReady, false);
      expect(manager.service, isNull);
    });

    test('unload resets state', () async {
      await manager.unload();
      expect(manager.state, LlmModelState.unloaded);
      expect(manager.service, isNull);
      expect(manager.isReady, false);
    });

    test('recordUsage increments counter', () {
      expect(manager.usageCount, 0);
      manager.recordUsage();
      expect(manager.usageCount, 1);
      manager.recordUsage();
      expect(manager.usageCount, 2);
    });

    test('resetStats clears counter', () {
      manager.recordUsage();
      manager.recordUsage();
      expect(manager.usageCount, 2);
      manager.resetStats();
      expect(manager.usageCount, 0);
    });

    test('infer throws when not loaded', () async {
      await manager.unload();
      expect(
        () => manager.infer('test'),
        throwsA(isA<LlmException>().having(
          (e) => e.code,
          'code',
          LlmErrorCode.modelNotFound,
        )),
      );
    });

    test('inferMessages throws when not loaded', () async {
      await manager.unload();
      expect(
        () => manager.inferMessages([const LlmMessage(role: 'user', content: 'test')]),
        throwsA(isA<LlmException>().having(
          (e) => e.code,
          'code',
          LlmErrorCode.modelNotFound,
        )),
      );
    });

    test('healthCheck returns false when unloaded', () async {
      await manager.unload();
      final result = await manager.healthCheck();
      expect(result, false);
    });

    test('state getters work correctly', () {
      expect(LlmModelState.unloaded.isUnloaded, true);
      expect(LlmModelState.loading.isLoading, true);
      expect(LlmModelState.loaded.isLoaded, true);
      expect(LlmModelState.error.isError, true);
      expect(LlmModelState.loaded.isReady, true);
    });

    test('LlmModelState labels are Chinese', () {
      expect(LlmModelState.unloaded.label, '未加载');
      expect(LlmModelState.loading.label, '加载中');
      expect(LlmModelState.loaded.label, '就绪');
      expect(LlmModelState.error.label, '错误');
    });

    test('loadCloud transitions to error on bad API key', () async {
      expect(manager.state, LlmModelState.unloaded);
      // We can't fully test loadCloud without a mock HTTP client injection,
      // but we can verify it throws and ends in error state.
      await expectLater(
        manager.loadCloud(apiKey: 'invalid-key-that-will-fail'),
        throwsA(isA<LlmException>()),
      );
      expect(manager.state, LlmModelState.error);
      expect(manager.errorMessage, isNotNull);
    });

    test('loadLocal transitions to error when Ollama not running', () async {
      await expectLater(
        manager.loadLocal(baseUrl: 'http://localhost:19999', validateModel: true),
        throwsA(isA<LlmException>()),
      );
      expect(manager.state, LlmModelState.error);
    });
  });
}
