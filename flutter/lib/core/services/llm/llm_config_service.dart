import 'dart:convert';

import 'package:solosoul_flutter/core/services/llm/llm_service.dart';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';

// =============================================================================
// LLM Config Service
// =============================================================================

/// Manages LLM backend selection and secure storage of cloud API credentials.
///
/// Settings are stored as a single encrypted JSON blob per account via
/// [RustVaultService.saveSettingEncrypted] / [loadSettingDecrypted].
class LlmConfigService {
  static LlmConfigService? _instance;
  static LlmConfigService get instance => _instance ??= LlmConfigService._();
  LlmConfigService._();

  final RustVaultService _vault = RustVaultService.instance;

  // ---------------------------------------------------------------------------
  // Load / Save
  // ---------------------------------------------------------------------------

  Future<_LlmConfig> _load(String accountId) async {
    final jsonStr = await _vault.loadSettingDecrypted(accountId);
    if (jsonStr == null) return const _LlmConfig();
    try {
      final map = jsonDecode(jsonStr) as Map<String, dynamic>;
      return _LlmConfig.fromJson(map);
    } on Object catch (_) {
      return const _LlmConfig();
    }
  }

  Future<void> _save(String accountId, _LlmConfig config) async {
    final jsonData = jsonEncode(config.toJson());
    await _vault.saveSettingEncrypted(accountId, jsonData);
  }

  // ---------------------------------------------------------------------------
  // Backend selection
  // ---------------------------------------------------------------------------

  Future<LlmBackendType> getBackendType(String accountId) async {
    final config = await _load(accountId);
    return config.backendType;
  }

  Future<void> setBackendType(
    String accountId,
    LlmBackendType type,
  ) async {
    final config = await _load(accountId);
    await _save(accountId, config.copyWith(backendType: type));
  }

  // ---------------------------------------------------------------------------
  // Cloud API credentials (encrypted at rest)
  // ---------------------------------------------------------------------------

  Future<String?> getCloudApiKey(String accountId) async {
    final config = await _load(accountId);
    return config.cloudApiKey;
  }

  Future<void> setCloudApiKey(String accountId, String apiKey) async {
    final config = await _load(accountId);
    await _save(accountId, config.copyWith(cloudApiKey: apiKey));
  }

  Future<void> clearCloudApiKey(String accountId) async {
    final config = await _load(accountId);
    await _save(accountId, config.copyWith(cloudApiKey: null));
  }

  Future<String> getCloudEndpoint(String accountId) async {
    final config = await _load(accountId);
    return config.cloudEndpoint ?? 'https://api.openai.com/v1';
  }

  Future<void> setCloudEndpoint(String accountId, String endpoint) async {
    final config = await _load(accountId);
    await _save(accountId, config.copyWith(cloudEndpoint: endpoint));
  }

  Future<String> getCloudModel(String accountId) async {
    final config = await _load(accountId);
    return config.cloudModel ?? 'gpt-4o-mini';
  }

  Future<void> setCloudModel(String accountId, String model) async {
    final config = await _load(accountId);
    await _save(accountId, config.copyWith(cloudModel: model));
  }

  // ---------------------------------------------------------------------------
  // Local model settings
  // ---------------------------------------------------------------------------

  Future<String?> getLocalModelPath(String accountId) async {
    final config = await _load(accountId);
    return config.localModelPath;
  }

  Future<void> setLocalModelPath(String accountId, String path) async {
    final config = await _load(accountId);
    await _save(accountId, config.copyWith(localModelPath: path));
  }

  // ---------------------------------------------------------------------------
  // Privacy settings
  // ---------------------------------------------------------------------------

  Future<bool> getCloudConsent(String accountId) async {
    final config = await _load(accountId);
    return config.cloudConsent == true;
  }

  Future<void> setCloudConsent(String accountId, bool consent) async {
    final config = await _load(accountId);
    await _save(accountId, config.copyWith(cloudConsent: consent));
  }
}

// =============================================================================
// Internal Config Model
// =============================================================================

class _LlmConfig {
  final LlmBackendType backendType;
  final String? cloudApiKey;
  final String? cloudEndpoint;
  final String? cloudModel;
  final String? localModelPath;
  final bool? cloudConsent;

  const _LlmConfig({
    this.backendType = LlmBackendType.local,
    this.cloudApiKey,
    this.cloudEndpoint,
    this.cloudModel,
    this.localModelPath,
    this.cloudConsent,
  });

  _LlmConfig copyWith({
    LlmBackendType? backendType,
    String? cloudApiKey,
    String? cloudEndpoint,
    String? cloudModel,
    String? localModelPath,
    bool? cloudConsent,
  }) {
    return _LlmConfig(
      backendType: backendType ?? this.backendType,
      cloudApiKey: cloudApiKey ?? this.cloudApiKey,
      cloudEndpoint: cloudEndpoint ?? this.cloudEndpoint,
      cloudModel: cloudModel ?? this.cloudModel,
      localModelPath: localModelPath ?? this.localModelPath,
      cloudConsent: cloudConsent ?? this.cloudConsent,
    );
  }

  Map<String, dynamic> toJson() => {
        'backendType': backendType.name,
        'cloudApiKey': cloudApiKey,
        'cloudEndpoint': cloudEndpoint,
        'cloudModel': cloudModel,
        'localModelPath': localModelPath,
        'cloudConsent': cloudConsent,
      };

  factory _LlmConfig.fromJson(Map<String, dynamic> json) {
    LlmBackendType parseBackend(String? raw) {
      if (raw == null) return LlmBackendType.local;
      return LlmBackendType.values.byName(raw);
    }

    return _LlmConfig(
      backendType: parseBackend(json['backendType'] as String?),
      cloudApiKey: json['cloudApiKey'] as String?,
      cloudEndpoint: json['cloudEndpoint'] as String?,
      cloudModel: json['cloudModel'] as String?,
      localModelPath: json['localModelPath'] as String?,
      cloudConsent: json['cloudConsent'] as bool?,
    );
  }
}
