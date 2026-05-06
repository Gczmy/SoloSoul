import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/llm/llm_usage_stats.dart';

void main() {
  group('LlmModelUsage', () {
    test('constructs with defaults', () {
      const usage = LlmModelUsage(modelName: 'gpt-4', provider: 'openai');
      expect(usage.modelName, 'gpt-4');
      expect(usage.provider, 'openai');
      expect(usage.usageCount, 0);
      expect(usage.promptTokens, 0);
      expect(usage.completionTokens, 0);
      expect(usage.totalTokens, 0);
      expect(usage.lastLoadTime, isNull);
      expect(usage.lastUsedTime, isNull);
    });

    test('totalTokens sums prompt and completion', () {
      const usage = LlmModelUsage(
        modelName: 'gpt-4',
        provider: 'openai',
        promptTokens: 10,
        completionTokens: 5,
      );
      expect(usage.totalTokens, 15);
    });

    test('copyWith preserves unchanged fields', () {
      const usage = LlmModelUsage(
        modelName: 'gpt-4',
        provider: 'openai',
        usageCount: 3,
        promptTokens: 10,
        completionTokens: 5,
      );
      final updated = usage.copyWith(usageCount: 4);
      expect(updated.modelName, 'gpt-4');
      expect(updated.provider, 'openai');
      expect(updated.usageCount, 4);
      expect(updated.promptTokens, 10);
    });

    test('toJson roundtrips via fromJson', () {
      final now = DateTime(2024, 1, 15, 10, 30);
      final usage = LlmModelUsage(
        modelName: 'claude-3',
        provider: 'anthropic',
        usageCount: 5,
        promptTokens: 100,
        completionTokens: 50,
        lastLoadTime: now,
        lastUsedTime: now,
      );
      final json = usage.toJson();
      final restored = LlmModelUsage.fromJson(json);

      expect(restored.modelName, 'claude-3');
      expect(restored.provider, 'anthropic');
      expect(restored.usageCount, 5);
      expect(restored.promptTokens, 100);
      expect(restored.completionTokens, 50);
      expect(restored.lastLoadTime, now);
      expect(restored.lastUsedTime, now);
    });

    test('fromJson handles null and missing fields', () {
      final restored = LlmModelUsage.fromJson({});
      expect(restored.modelName, '');
      expect(restored.provider, '');
      expect(restored.usageCount, 0);
      expect(restored.promptTokens, 0);
      expect(restored.completionTokens, 0);
      expect(restored.lastLoadTime, isNull);
      expect(restored.lastUsedTime, isNull);
    });

    test('fromJson handles invalid dates gracefully', () {
      final restored = LlmModelUsage.fromJson({
        'lastLoadTime': 'not-a-date',
        'lastUsedTime': '',
      });
      expect(restored.lastLoadTime, isNull);
      expect(restored.lastUsedTime, isNull);
    });
  });

  group('LlmDailyUsage', () {
    test('constructs with defaults', () {
      final daily = LlmDailyUsage(date: DateTime(2024, 1, 15));
      expect(daily.totalTokens, 0);
      expect(daily.usageCount, 0);
      expect(daily.perModelTokens, isEmpty);
    });

    test('copyWith updates selected fields', () {
      final daily = LlmDailyUsage(
        date: DateTime(2024, 1, 15),
        totalTokens: 100,
        usageCount: 2,
        perModelTokens: const {'openai/gpt-4': 100},
      );
      final updated = daily.copyWith(totalTokens: 200, usageCount: 3);
      expect(updated.date, daily.date);
      expect(updated.totalTokens, 200);
      expect(updated.usageCount, 3);
      expect(updated.perModelTokens, daily.perModelTokens);
    });

    test('toJson roundtrips via fromJson', () {
      final daily = LlmDailyUsage(
        date: DateTime(2024, 1, 15, 8, 0),
        totalTokens: 500,
        usageCount: 10,
        perModelTokens: const {'openai/gpt-4': 300, 'anthropic/claude-3': 200},
      );
      final json = daily.toJson();
      final restored = LlmDailyUsage.fromJson(json);

      expect(restored.date, daily.date);
      expect(restored.totalTokens, 500);
      expect(restored.usageCount, 10);
      expect(restored.perModelTokens, daily.perModelTokens);
    });

    test('fromJson handles null date with fallback', () {
      final before = DateTime.now();
      final restored = LlmDailyUsage.fromJson({});
      final after = DateTime.now();
      expect(restored.date.isAfter(before) || restored.date.isAtSameMomentAs(before), isTrue);
      expect(restored.date.isBefore(after) || restored.date.isAtSameMomentAs(after), isTrue);
    });

    test('fromJson handles null perModelTokens', () {
      final restored = LlmDailyUsage.fromJson({
        'date': '2024-01-15T00:00:00.000',
      });
      expect(restored.perModelTokens, isEmpty);
    });
  });

  group('LlmUsageStats', () {
    test('constructs with defaults', () {
      const stats = LlmUsageStats();
      expect(stats.usageCount, 0);
      expect(stats.totalPromptTokens, 0);
      expect(stats.totalCompletionTokens, 0);
      expect(stats.totalTokensUsed, 0);
      expect(stats.sessionUsageCount, 0);
      expect(stats.sessionPromptTokens, 0);
      expect(stats.sessionCompletionTokens, 0);
      expect(stats.sessionTotalTokens, 0);
      expect(stats.perModelStats, isEmpty);
      expect(stats.dailyStats, isEmpty);
    });

    test('totalTokensUsed sums prompt and completion', () {
      const stats = LlmUsageStats(
        totalPromptTokens: 100,
        totalCompletionTokens: 50,
      );
      expect(stats.totalTokensUsed, 150);
    });

    test('sessionTotalTokens sums session tokens', () {
      const stats = LlmUsageStats(
        sessionPromptTokens: 20,
        sessionCompletionTokens: 10,
      );
      expect(stats.sessionTotalTokens, 30);
    });

    test('copyWith preserves and updates fields', () {
      const stats = LlmUsageStats(usageCount: 5, totalPromptTokens: 100);
      final updated = stats.copyWith(usageCount: 6);
      expect(updated.usageCount, 6);
      expect(updated.totalPromptTokens, 100);
      expect(updated.totalCompletionTokens, 0);
    });

    test('toJson / fromJson roundtrip', () {
      final now = DateTime(2024, 6, 1, 12, 0);
      final stats = LlmUsageStats(
        usageCount: 42,
        totalPromptTokens: 1000,
        totalCompletionTokens: 500,
        lastLoadTime: now,
        lastUsedTime: now,
        perModelStats: const [
          LlmModelUsage(modelName: 'gpt-4', provider: 'openai', usageCount: 10),
        ],
        dailyStats: [
          LlmDailyUsage(date: now, totalTokens: 1500, usageCount: 42),
        ],
        sessionUsageCount: 1,
        sessionPromptTokens: 50,
        sessionCompletionTokens: 25,
      );

      final json = stats.toJson();
      final restored = LlmUsageStats.fromJson(json);

      expect(restored.usageCount, 42);
      expect(restored.totalPromptTokens, 1000);
      expect(restored.totalCompletionTokens, 500);
      expect(restored.lastLoadTime, now);
      expect(restored.lastUsedTime, now);
      expect(restored.perModelStats.length, 1);
      expect(restored.perModelStats.first.modelName, 'gpt-4');
      expect(restored.dailyStats.length, 1);
      expect(restored.dailyStats.first.totalTokens, 1500);
      // Session fields are memory-only and NOT persisted
      expect(restored.sessionUsageCount, 0);
      expect(restored.sessionPromptTokens, 0);
      expect(restored.sessionCompletionTokens, 0);
    });

    test('fromJsonString roundtrips via toJsonString', () {
      const stats = LlmUsageStats(usageCount: 7, totalPromptTokens: 100);
      final jsonStr = stats.toJsonString();
      final restored = LlmUsageStats.fromJsonString(jsonStr);
      expect(restored.usageCount, 7);
      expect(restored.totalPromptTokens, 100);
    });

    test('fromJson handles empty object', () {
      final restored = LlmUsageStats.fromJson({});
      expect(restored.usageCount, 0);
      expect(restored.perModelStats, isEmpty);
      expect(restored.dailyStats, isEmpty);
    });

    test('fromJson throws on malformed perModelStats element', () {
      expect(
        () => LlmUsageStats.fromJson({
          'perModelStats': [
            {'modelName': 'good'},
            'not-a-map',
          ],
        }),
        throwsA(isA<TypeError>()),
      );
    });
  });
}
