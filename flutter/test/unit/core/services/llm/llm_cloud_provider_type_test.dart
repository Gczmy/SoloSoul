import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/llm/llm_cloud_provider_type.dart';

void main() {
  group('LlmCloudProviderType', () {
    test('openai label is OpenAI', () {
      expect(LlmCloudProviderType.openai.label, 'OpenAI');
    });

    test('anthropic label is Anthropic', () {
      expect(LlmCloudProviderType.anthropic.label, 'Anthropic');
    });
  });

  group('LlmCloudProviderTypeExtension', () {
    test('toJson returns name', () {
      expect(LlmCloudProviderType.openai.toJson(), 'openai');
      expect(LlmCloudProviderType.anthropic.toJson(), 'anthropic');
    });

    test('fromJson parses openai', () {
      expect(
        LlmCloudProviderTypeExtension.fromJson('openai'),
        LlmCloudProviderType.openai,
      );
    });

    test('fromJson parses anthropic', () {
      expect(
        LlmCloudProviderTypeExtension.fromJson('anthropic'),
        LlmCloudProviderType.anthropic,
      );
    });

    test('fromJson defaults to openai for null', () {
      expect(
        LlmCloudProviderTypeExtension.fromJson(null),
        LlmCloudProviderType.openai,
      );
    });

    test('fromJson defaults to openai for unknown values', () {
      expect(
        LlmCloudProviderTypeExtension.fromJson('unknown'),
        LlmCloudProviderType.openai,
      );
      expect(
        LlmCloudProviderTypeExtension.fromJson(''),
        LlmCloudProviderType.openai,
      );
    });
  });
}
