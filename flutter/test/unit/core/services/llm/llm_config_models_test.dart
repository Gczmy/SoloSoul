import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/llm/llm_config_models.dart';
import 'package:solosoul_flutter/core/services/llm/llm_service.dart';

void main() {
  group('LlmCloudProfile', () {
    test('constructs with required fields', () {
      const profile = LlmCloudProfile(
        id: 'p1',
        name: 'Test Profile',
        providerType: LlmCloudProviderType.openai,
        apiKeyRef: 'ref-1',
        endpoint: 'https://api.openai.com/v1',
        model: 'gpt-4o-mini',
      );
      expect(profile.id, 'p1');
      expect(profile.name, 'Test Profile');
      expect(profile.providerType, LlmCloudProviderType.openai);
      expect(profile.apiKeyRef, 'ref-1');
      expect(profile.apiKey, '');
      expect(profile.endpoint, 'https://api.openai.com/v1');
      expect(profile.model, 'gpt-4o-mini');
      expect(profile.anthropicVersion, isNull);
    });

    test('toJson roundtrips via fromJson', () {
      const profile = LlmCloudProfile(
        id: 'p1',
        name: 'Test',
        providerType: LlmCloudProviderType.anthropic,
        apiKeyRef: 'ref-2',
        apiKey: 'sk-test',
        endpoint: 'https://api.anthropic.com',
        model: 'claude-3-sonnet',
        anthropicVersion: '2023-06-01',
      );
      final json = profile.toJson();
      final restored = LlmCloudProfile.fromJson(json);

      expect(restored.id, 'p1');
      expect(restored.name, 'Test');
      expect(restored.providerType, LlmCloudProviderType.anthropic);
      expect(restored.apiKeyRef, 'ref-2');
      expect(restored.apiKey, 'sk-test');
      expect(restored.endpoint, 'https://api.anthropic.com');
      expect(restored.model, 'claude-3-sonnet');
      expect(restored.anthropicVersion, '2023-06-01');
    });

    test('fromJson assigns defaults for missing fields', () {
      final restored = LlmCloudProfile.fromJson({});
      expect(restored.id, isNotEmpty); // UUID fallback
      expect(restored.name, 'Unnamed Configuration');
      expect(restored.providerType, LlmCloudProviderType.openai);
      expect(restored.apiKeyRef, isNotEmpty);
      expect(restored.apiKey, '');
      expect(restored.endpoint, '');
      expect(restored.model, '');
    });

    test('copyWith updates selected fields', () {
      const profile = LlmCloudProfile(
        id: 'p1',
        name: 'Old',
        providerType: LlmCloudProviderType.openai,
        apiKeyRef: 'ref-1',
        endpoint: 'https://old.com',
        model: 'old-model',
      );
      final updated = profile.copyWith(name: 'New', model: 'new-model');
      expect(updated.id, 'p1');
      expect(updated.name, 'New');
      expect(updated.providerType, LlmCloudProviderType.openai);
      expect(updated.endpoint, 'https://old.com');
      expect(updated.model, 'new-model');
    });

    test('maskApiKey masks correctly', () {
      expect(
        LlmCloudProfile.maskApiKey('sk-abcdefghijklmnopqrstuvwxyz'),
        'sk-a...wxyz',
      );
      expect(LlmCloudProfile.maskApiKey('short'), '****');
      expect(LlmCloudProfile.maskApiKey('sk-12345678'), 'sk-1...5678');
    });

    test('maskApiKey supports prefix', () {
      expect(
        LlmCloudProfile.maskApiKey('sk-abcdefghijklmnopqrstuvwxyz', prefix: 'Key: '),
        'Key: sk-a...wxyz',
      );
      expect(
        LlmCloudProfile.maskApiKey('sk-test', prefix: 'Key: '),
        'Key: ****',
      );
    });

    test('toString returns expected format', () {
      const profile = LlmCloudProfile(
        id: 'p1',
        name: 'MyProfile',
        providerType: LlmCloudProviderType.openai,
        apiKeyRef: 'ref',
        endpoint: 'https://api.openai.com/v1',
        model: 'gpt-4',
      );
      expect(profile.toString(), 'LlmCloudProfile(MyProfile, OpenAI, gpt-4)');
    });
  });

  group('LlmConfigState', () {
    test('constructs with defaults', () {
      const state = LlmConfigState();
      expect(state.backendType, LlmBackendType.local);
      expect(state.cloudApiKey, '');
      expect(state.cloudEndpoint, 'https://api.openai.com/v1');
      expect(state.cloudModel, 'gpt-4o-mini');
      expect(state.localModelPath, isNull);
      expect(state.cloudConsent, false);
      expect(state.cloudProviderType, LlmCloudProviderType.openai);
      expect(state.cloudAnthropicVersion, '2023-06-01');
      expect(state.cloudProfiles, isEmpty);
      expect(state.activeCloudProfileId, isNull);
    });

    test('activeCloudProfile returns first when no active id set', () {
      const profile = LlmCloudProfile(
        id: 'p1',
        name: 'Only',
        providerType: LlmCloudProviderType.openai,
        apiKeyRef: 'ref',
        endpoint: 'https://api.openai.com/v1',
        model: 'gpt-4',
      );
      const state = LlmConfigState(cloudProfiles: [profile]);
      expect(state.activeCloudProfile?.id, 'p1');
    });

    test('activeCloudProfile returns null when empty', () {
      const state = LlmConfigState();
      expect(state.activeCloudProfile, isNull);
    });

    test('activeCloudProfile falls back to first when active id not found', () {
      const p1 = LlmCloudProfile(
        id: 'p1',
        name: 'First',
        providerType: LlmCloudProviderType.openai,
        apiKeyRef: 'r1',
        endpoint: 'https://a.com',
        model: 'm1',
      );
      const state = LlmConfigState(
        cloudProfiles: [p1],
        activeCloudProfileId: 'nonexistent',
      );
      expect(state.activeCloudProfile?.id, 'p1');
    });

    test('copyWith updates fields', () {
      const state = LlmConfigState(cloudApiKey: 'old');
      final updated = state.copyWith(cloudApiKey: 'new');
      expect(updated.cloudApiKey, 'new');
      expect(updated.cloudEndpoint, state.cloudEndpoint);
    });

    test('copyWith supports sentinel semantics for clearing values', () {
      const state = LlmConfigState(cloudApiKey: 'key');
      final cleared = state.copyWith(cloudApiKey: '');
      expect(cleared.cloudApiKey, '');
    });

    test('canUseCloud returns false when backend is local', () {
      const state = LlmConfigState(
        backendType: LlmBackendType.local,
        cloudConsent: true,
        cloudProfiles: [
          LlmCloudProfile(
            id: 'p1',
            name: 'Test',
            providerType: LlmCloudProviderType.openai,
            apiKeyRef: 'r',
            endpoint: 'https://a.com',
            model: 'm',
          ),
        ],
      );
      expect(state.canUseCloud, isFalse);
    });

    test('canUseCloud returns false without consent', () {
      const state = LlmConfigState(
        backendType: LlmBackendType.cloud,
        cloudConsent: false,
        cloudProfiles: [
          LlmCloudProfile(
            id: 'p1',
            name: 'Test',
            providerType: LlmCloudProviderType.openai,
            apiKeyRef: 'r',
            endpoint: 'https://a.com',
            model: 'm',
          ),
        ],
      );
      expect(state.canUseCloud, isFalse);
    });

    test('canUseCloud returns false when active profile has empty endpoint', () {
      const state = LlmConfigState(
        backendType: LlmBackendType.cloud,
        cloudConsent: true,
        cloudProfiles: [
          LlmCloudProfile(
            id: 'p1',
            name: 'Test',
            providerType: LlmCloudProviderType.openai,
            apiKeyRef: 'r',
            endpoint: '',
            model: 'm',
          ),
        ],
      );
      expect(state.canUseCloud, isFalse);
    });

    test('canUseCloud returns true when all conditions met', () {
      const state = LlmConfigState(
        backendType: LlmBackendType.cloud,
        cloudConsent: true,
        cloudProfiles: [
          LlmCloudProfile(
            id: 'p1',
            name: 'Test',
            providerType: LlmCloudProviderType.openai,
            apiKeyRef: 'r',
            endpoint: 'https://a.com',
            model: 'm',
          ),
        ],
      );
      expect(state.canUseCloud, isTrue);
    });

    test('copyWith makes cloudProfiles unmodifiable', () {
      const state = LlmConfigState();
      final updated = state.copyWith(cloudProfiles: [
        const LlmCloudProfile(
          id: 'p1',
          name: 'Test',
          providerType: LlmCloudProviderType.openai,
          apiKeyRef: 'r',
          endpoint: 'https://a.com',
          model: 'm',
        ),
      ]);
      expect(
        () => updated.cloudProfiles.add(updated.cloudProfiles.first),
        throwsA(const TypeMatcher<UnsupportedError>()),
      );
    });
  });
}
