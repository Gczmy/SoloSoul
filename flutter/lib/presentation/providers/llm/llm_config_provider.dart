import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/llm/llm_config_service.dart';
import 'package:solosoul_flutter/core/services/llm/llm_service.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';

// =============================================================================
// LLM Config State
// =============================================================================

/// Immutable snapshot of LLM configuration for UI consumption.
class LlmConfigState {
  final LlmBackendType backendType;
  final String cloudApiKey;
  final String cloudEndpoint;
  final String cloudModel;
  final String? localModelPath;
  final bool cloudConsent;

  const LlmConfigState({
    this.backendType = LlmBackendType.local,
    this.cloudApiKey = '',
    this.cloudEndpoint = 'https://api.openai.com/v1',
    this.cloudModel = 'gpt-4o-mini',
    this.localModelPath,
    this.cloudConsent = false,
  });

  LlmConfigState copyWith({
    LlmBackendType? backendType,
    String? cloudApiKey,
    String? cloudEndpoint,
    String? cloudModel,
    String? localModelPath,
    bool? cloudConsent,
  }) {
    return LlmConfigState(
      backendType: backendType ?? this.backendType,
      cloudApiKey: cloudApiKey ?? this.cloudApiKey,
      cloudEndpoint: cloudEndpoint ?? this.cloudEndpoint,
      cloudModel: cloudModel ?? this.cloudModel,
      localModelPath: localModelPath ?? this.localModelPath,
      cloudConsent: cloudConsent ?? this.cloudConsent,
    );
  }

  /// Whether cloud mode is usable (key + consent + endpoint).
  bool get canUseCloud =>
      backendType == LlmBackendType.cloud &&
      cloudApiKey.isNotEmpty &&
      cloudConsent;
}

// =============================================================================
// Async Notifier
// =============================================================================

/// Manages LLM configuration state backed by encrypted Vault storage.
///
/// Depends on [authNotifierProvider] for current account ID.
class LlmConfigNotifier extends AsyncNotifier<LlmConfigState> {
  final LlmConfigService _service = LlmConfigService.instance;

  String? get _accountId =>
      ref.read(authNotifierProvider.notifier).selectedAccountId;

  @override
  Future<LlmConfigState> build() async {
    final accountId = _accountId;
    if (accountId == null) return const LlmConfigState();

    final backend = await _service.getBackendType(accountId);
    final key = await _service.getCloudApiKey(accountId) ?? '';
    final endpoint = await _service.getCloudEndpoint(accountId);
    final model = await _service.getCloudModel(accountId);
    final localPath = await _service.getLocalModelPath(accountId);
    final consent = await _service.getCloudConsent(accountId);

    return LlmConfigState(
      backendType: backend,
      cloudApiKey: key,
      cloudEndpoint: endpoint,
      cloudModel: model,
      localModelPath: localPath,
      cloudConsent: consent,
    );
  }

  // ---------------------------------------------------------------------------
  // Mutations
  // ---------------------------------------------------------------------------

  Future<void> setBackendType(LlmBackendType type) async {
    if (!state.hasValue) return;
    state = AsyncData(state.value!.copyWith(backendType: type));
    final id = _accountId;
    if (id != null) await _service.setBackendType(id, type);
  }

  Future<void> setCloudApiKey(String apiKey) async {
    if (!state.hasValue) return;
    state = AsyncData(state.value!.copyWith(cloudApiKey: apiKey));
    final id = _accountId;
    if (id != null) await _service.setCloudApiKey(id, apiKey);
  }

  Future<void> clearCloudApiKey() async {
    if (!state.hasValue) return;
    state = AsyncData(state.value!.copyWith(cloudApiKey: ''));
    final id = _accountId;
    if (id != null) await _service.clearCloudApiKey(id);
  }

  Future<void> setCloudEndpoint(String endpoint) async {
    if (!state.hasValue) return;
    state = AsyncData(state.value!.copyWith(cloudEndpoint: endpoint));
    final id = _accountId;
    if (id != null) await _service.setCloudEndpoint(id, endpoint);
  }

  Future<void> setCloudModel(String model) async {
    if (!state.hasValue) return;
    state = AsyncData(state.value!.copyWith(cloudModel: model));
    final id = _accountId;
    if (id != null) await _service.setCloudModel(id, model);
  }

  Future<void> setLocalModelPath(String path) async {
    if (!state.hasValue) return;
    state = AsyncData(state.value!.copyWith(localModelPath: path));
    final id = _accountId;
    if (id != null) await _service.setLocalModelPath(id, path);
  }

  Future<void> setCloudConsent(bool consent) async {
    if (!state.hasValue) return;
    state = AsyncData(state.value!.copyWith(cloudConsent: consent));
    final id = _accountId;
    if (id != null) await _service.setCloudConsent(id, consent);
  }
}

// =============================================================================
// Riverpod Provider
// =============================================================================

final llmConfigProvider =
    AsyncNotifierProvider<LlmConfigNotifier, LlmConfigState>(
  () => LlmConfigNotifier(),
);
