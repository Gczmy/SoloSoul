import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/llm/llm_service.dart';
import 'package:solosoul_flutter/presentation/providers/llm/llm_config_provider.dart';

// =============================================================================
// LLM Service Provider
// =============================================================================

/// Provides a concrete [LlmService] instance based on current config.
///
/// Returns [LlmLocalService.instance] when backend is local;
/// returns a fresh [LlmCloudService] when backend is cloud and properly
/// configured (API key + consent). Falls back to local stub otherwise.
///
/// **Usage:**
/// ```dart
/// final llm = ref.read(llmServiceProvider);
/// final result = await llm.infer('Summarize this...');
/// ```
final llmServiceProvider = Provider<LlmService>((ref) {
  final configAsync = ref.watch(llmConfigProvider);

  final config = configAsync.value;
  if (config != null &&
      config.backendType == LlmBackendType.cloud &&
      config.canUseCloud) {
    return LlmCloudService(
      apiKey: config.cloudApiKey,
      endpoint: config.cloudEndpoint,
      model: config.cloudModel,
    );
  }

  return LlmLocalService.instance;
});
