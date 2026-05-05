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
    if (!state.hasValue) return;
    final id = _accountId;
    if (id != null) await _service.setBackendType(id, type);
    if (state.hasValue) {
      state = AsyncData(state.value!.copyWith(backendType: type));
    }
  }

  Future<void> setCloudApiKey(String apiKey) async {
    if (!state.hasValue) return;
    final id = _accountId;
    if (id != null) await _service.setCloudApiKey(id, apiKey);
    if (state.hasValue) {
      state = AsyncData(state.value!.copyWith(cloudApiKey: apiKey));
    }
  }

  Future<void> clearCloudApiKey() async {
    if (!state.hasValue) return;
    final id = _accountId;
    if (id != null) await _service.clearCloudApiKey(id);
    if (state.hasValue) {
      state = AsyncData(state.value!.copyWith(cloudApiKey: ''));
    }
  }

  Future<void> setCloudEndpoint(String endpoint) async {
    if (!state.hasValue) return;
    final id = _accountId;
    if (id != null) await _service.setCloudEndpoint(id, endpoint);
    if (state.hasValue) {
      state = AsyncData(state.value!.copyWith(cloudEndpoint: endpoint));
    }
  }

  Future<void> setCloudModel(String model) async {
    if (!state.hasValue) return;
    final id = _accountId;
    if (id != null) await _service.setCloudModel(id, model);
    if (state.hasValue) {
      state = AsyncData(state.value!.copyWith(cloudModel: model));
    }
  }

  Future<void> setLocalModelPath(String path) async {
    if (!state.hasValue) return;
    final id = _accountId;
    if (id != null) await _service.setLocalModelPath(id, path);
    if (state.hasValue) {
      state = AsyncData(state.value!.copyWith(localModelPath: path));
    }
  }

  Future<void> setCloudConsent(bool consent) async {
    if (!state.hasValue) return;
    final id = _accountId;
    if (id != null) await _service.setCloudConsent(id, consent);
    if (state.hasValue) {
      state = AsyncData(state.value!.copyWith(cloudConsent: consent));
    }
  }

  Future<void> setCloudProviderType(LlmCloudProviderType type) async {
    if (!state.hasValue) return;
    final id = _accountId;
    if (id != null) await _service.setCloudProviderType(id, type);
    if (state.hasValue) {
      state = AsyncData(state.value!.copyWith(cloudProviderType: type));
    }
  }

  Future<void> setCloudAnthropicVersion(String version) async {
    if (!state.hasValue) return;
    final id = _accountId;
    if (id != null) await _service.setCloudAnthropicVersion(id, version);
    if (state.hasValue) {
      state = AsyncData(state.value!.copyWith(cloudAnthropicVersion: version));
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

  Future<void> updateCloudProfile({
    required String profileId,
    String? name,
    LlmCloudProviderType? providerType,
    String? apiKey,
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
    if (state.hasValue) {
      state = AsyncData(state.value!.copyWith(activeCloudProfileId: profileId));
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
