import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/llm/llm_config_models.dart';
import 'package:solosoul_flutter/core/services/llm/llm_config_service.dart';
import 'package:solosoul_flutter/core/services/llm/llm_service.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';

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
    return _service.getLlmConfigState(accountId);
  }

  // ---------------------------------------------------------------------------
  // Mutations
  // ---------------------------------------------------------------------------

  Future<void> setBackendType(LlmBackendType type) async {
    if (state case AsyncData(:final value)) {
      final id = _accountId;
      if (id != null) await _service.setBackendType(id, type);
      state = AsyncData(value.copyWith(backendType: type));
    }
  }

  Future<void> setCloudApiKey(String apiKey) async {
    if (state case AsyncData(:final value)) {
      final id = _accountId;
      if (id != null) await _service.setCloudApiKey(id, apiKey);
      state = AsyncData(value.copyWith(cloudApiKey: apiKey));
    }
  }

  Future<void> clearCloudApiKey() async {
    if (state case AsyncData(:final value)) {
      final id = _accountId;
      if (id != null) await _service.clearCloudApiKey(id);
      state = AsyncData(value.copyWith(cloudApiKey: ''));
    }
  }

  Future<void> setCloudEndpoint(String endpoint) async {
    if (state case AsyncData(:final value)) {
      final id = _accountId;
      if (id != null) await _service.setCloudEndpoint(id, endpoint);
      state = AsyncData(value.copyWith(cloudEndpoint: endpoint));
    }
  }

  Future<void> setCloudModel(String model) async {
    if (state case AsyncData(:final value)) {
      final id = _accountId;
      if (id != null) await _service.setCloudModel(id, model);
      state = AsyncData(value.copyWith(cloudModel: model));
    }
  }

  Future<void> setLocalModelPath(String path) async {
    if (state case AsyncData(:final value)) {
      final id = _accountId;
      if (id != null) await _service.setLocalModelPath(id, path);
      state = AsyncData(value.copyWith(localModelPath: path));
    }
  }

  Future<void> setCloudConsent(bool consent) async {
    if (state case AsyncData(:final value)) {
      final id = _accountId;
      if (id != null) await _service.setCloudConsent(id, consent);
      state = AsyncData(value.copyWith(cloudConsent: consent));
    }
  }

  Future<void> setCloudProviderType(LlmCloudProviderType type) async {
    final id = _accountId;
    if (id != null) await _service.setCloudProviderType(id, type);
    if (state case AsyncData(value: final value)) {
      state = AsyncData(value.copyWith(cloudProviderType: type));
    }
  }

  Future<void> setCloudAnthropicVersion(String version) async {
    final id = _accountId;
    if (id != null) await _service.setCloudAnthropicVersion(id, version);
    if (state case AsyncData(value: final value)) {
      state = AsyncData(value.copyWith(cloudAnthropicVersion: version));
    }
  }

  // ---------------------------------------------------------------------------
  // Profile CRUD
  // ---------------------------------------------------------------------------

  Future<void> addCloudProfile({
    required String name,
    required LlmCloudProviderType providerType,
    required String apiKey,
    required String endpoint,
    required String model,
    String? anthropicVersion,
  }) async {
    final id = _accountId;
    if (id == null) return;
    await _service.addCloudProfile(
      id,
      name: name,
      providerType: providerType,
      apiKey: apiKey,
      endpoint: endpoint,
      model: model,
      anthropicVersion: anthropicVersion,
    );
    state = AsyncData(await _service.getLlmConfigState(id));
  }

  static const _apiKeySentinel = Object();

  Future<void> updateCloudProfile({
    required String profileId,
    String? name,
    LlmCloudProviderType? providerType,
    Object? apiKey = _apiKeySentinel,
    String? endpoint,
    String? model,
    String? anthropicVersion,
  }) async {
    final id = _accountId;
    if (id == null) return;
    await _service.updateCloudProfile(
      id,
      profileId: profileId,
      name: name,
      providerType: providerType,
      apiKey: apiKey,
      endpoint: endpoint,
      model: model,
      anthropicVersion: anthropicVersion,
    );
    state = AsyncData(await _service.getLlmConfigState(id));
  }

  Future<void> deleteCloudProfile(String profileId) async {
    final id = _accountId;
    if (id == null) return;
    await _service.deleteCloudProfile(id, profileId);
    state = AsyncData(await _service.getLlmConfigState(id));
  }

  Future<void> setActiveCloudProfile(String profileId) async {
    final id = _accountId;
    if (id == null) return;
    await _service.setActiveCloudProfileId(id, profileId);
    if (state case AsyncData(value: final value)) {
      state = AsyncData(value.copyWith(activeCloudProfileId: profileId));
    }
  }
}

// =============================================================================
// Riverpod Provider
// =============================================================================

final llmConfigProvider =
    AsyncNotifierProvider<LlmConfigNotifier, LlmConfigState>(
  () => LlmConfigNotifier(),
);
