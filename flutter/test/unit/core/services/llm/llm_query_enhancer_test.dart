import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/llm/llm_query_enhancer.dart';
import 'package:solosoul_flutter/core/services/llm/llm_service.dart';

void main() {
  // We test the public interface (enhance) instead of private methods.
  // Private methods are tested indirectly through the public API.

  group('EnhancementResult', () {
    test('factory original creates non-enhanced result', () {
      final result = EnhancementResult.original('query');
      expect(result.original, 'query');
      expect(result.expanded, 'query');
      expect(result.rewritten, 'query');
      expect(result.enhanced, false);
    });
  });

  group('LlmQueryEnhancer integration', () {
    test('short queries return original without enhancement', () async {
      final enhancer = _FakeLlmQueryEnhancer();
      final result = await enhancer.enhance('a');
      expect(result.enhanced, false);
      expect(result.expanded, 'a');
      expect(enhancer.wasLlmCalled, false);
    });

    test('file paths return original without enhancement', () async {
      final enhancer = _FakeLlmQueryEnhancer();
      final result = await enhancer.enhance('document.pdf');
      expect(result.enhanced, false);
      expect(enhancer.wasLlmCalled, false);
    });

    test('quoted queries return original without enhancement', () async {
      final enhancer = _FakeLlmQueryEnhancer();
      final result = await enhancer.enhance('"exact phrase"');
      expect(result.enhanced, false);
      expect(enhancer.wasLlmCalled, false);
    });

    test('boolean queries return original without enhancement', () async {
      final enhancer = _FakeLlmQueryEnhancer();
      final result = await enhancer.enhance('apple AND orange');
      expect(result.enhanced, false);
      expect(enhancer.wasLlmCalled, false);
    });

    test('normal queries call LLM and return enhanced result', () async {
      final enhancer = _FakeLlmQueryEnhancer()
        ..llmResponse = '{"expanded_query": "Python教程 入门", "rewritten_query": "Python入门教程"}';

      final result = await enhancer.enhance('怎么用Python');
      expect(result.enhanced, true);
      expect(result.expanded, 'Python教程 入门');
      expect(result.rewritten, 'Python入门教程');
      expect(enhancer.wasLlmCalled, true);
    });

    test('falls back to rule-based when LLM fails', () async {
      final enhancer = _FakeLlmQueryEnhancer()
        ..shouldLlmFail = true;

      final result = await enhancer.enhance('怎么用Flutter');
      expect(result.enhanced, true);
      expect(result.expanded, contains('怎么用Flutter'));
      expect(result.expanded, contains('使用方法'));
    });

    test('falls back to original when query has no known synonyms', () async {
      final enhancer = _FakeLlmQueryEnhancer()
        ..shouldLlmFail = true;

      final result = await enhancer.enhance('xyzabc unknown phrase');
      expect(result.enhanced, false);
      expect(result.expanded, 'xyzabc unknown phrase');
    });

    test('parses JSON from markdown-wrapped LLM response', () async {
      final enhancer = _FakeLlmQueryEnhancer()
        ..llmResponse = 'Sure!\n```json\n{"expanded_query": "x y z", "rewritten_query": "xyz"}\n```';

      final result = await enhancer.enhance('什么是AI');
      expect(result.enhanced, true);
      expect(result.expanded, 'x y z');
    });

    test('falls back to original on invalid LLM JSON', () async {
      final enhancer = _FakeLlmQueryEnhancer()
        ..llmResponse = 'not json at all';

      final result = await enhancer.enhance('什么是AI');
      // Should fall through to rule-based (since LLM returned garbage)
      // "什么是" matches rule-based pattern
      expect(result.enhanced, true);
      expect(result.expanded, contains('定义'));
    });
  });
}

/// A testable subclass that exposes internal behavior.
class _FakeLlmQueryEnhancer extends LlmQueryEnhancer {
  bool wasLlmCalled = false;
  bool shouldLlmFail = false;
  String llmResponse = '{"expanded_query": "test", "rewritten_query": "test"}';

  _FakeLlmQueryEnhancer() : super(llm: _FakeLlmService());

  @override
  Future<EnhancementResult> enhance(String query) async {
    wasLlmCalled = false;

    // Replicate the logic from the real enhance() but with fake LLM
    if (!LlmQueryEnhancer.shouldEnhance(query)) {
      return EnhancementResult.original(query);
    }

    try {
      if (shouldLlmFail) throw Exception('LLM failed');
      wasLlmCalled = true;
      final parsed = LlmQueryEnhancer.parseResponse(llmResponse, query);
      if (parsed.enhanced) return parsed;
    } on Exception catch (_) {
      // fall through
    }

    return LlmQueryEnhancer.ruleBasedEnhance(query);
  }
}

class _FakeLlmService implements LlmService {
  @override
  Future<String> infer(String prompt, {int maxTokens = 512}) async => '';

  @override
  Future<String> inferMessages(List<LlmMessage> messages, {int maxTokens = 512}) async => '';

  @override
  Future<void> testConnection() async {}
}
