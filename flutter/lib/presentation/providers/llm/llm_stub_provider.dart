import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/llm/llm_config_service.dart';
import 'package:solosoul_flutter/core/services/llm/llm_service.dart';
import 'package:solosoul_flutter/presentation/providers/llm/llm_config_provider.dart';

// =============================================================================
// LLM Service Provider
// =============================================================================

/// Provides a concrete [LlmService] instance based on current config.
///
/// Returns a configured [LlmLocalService] when backend is local;
/// returns a configured [LlmCloudService] when backend is cloud and properly
/// configured (API key + consent).
///
/// **Note:** For cloud backend, this provider asynchronously fetches the
/// API key from [LlmConfigService]'s internal vault via [apiKeyRef].
///
/// **Usage:**
/// ```dart
/// final llmAsync = ref.watch(llmServiceProvider);
/// llmAsync.whenData((service) => service.infer('...'));
/// ```
final llmServiceProvider = FutureProvider<LlmService?>((ref) async {
  final configAsync = ref.watch(llmConfigProvider);

  final config = configAsync.value;
  if (config == null) return null;

  if (config.backendType == LlmBackendType.cloud && config.canUseCloud) {
    final profile = config.activeCloudProfile;
    if (profile == null) return null;

    final apiKey = await LlmConfigService.instance.getApiKeyByRef(profile.apiKeyRef);
    if (apiKey == null || apiKey.isEmpty) return null;

    return LlmCloudService(
      apiKey: apiKey,
      endpoint: profile.endpoint,
      model: profile.model,
      provider: profile.providerType,
      anthropicVersion: profile.anthropicVersion ?? '2023-06-01',
    );
  }

  // Local backend (default)
  return LlmLocalService(
    modelName: config.localModelPath ?? 'qwen2.5:1.5b',
  );
});
