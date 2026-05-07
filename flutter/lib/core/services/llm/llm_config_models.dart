import 'package:solosoul_flutter/core/services/llm/llm_service.dart';
import 'package:uuid/uuid.dart';

// =============================================================================
// LLM Cloud Profile
// =============================================================================

/// 单个云端 LLM 配置档案。
///
/// **安全设计：** 不直接持有 apiKey 明文，仅存储 `apiKeyRef`（随机 UUID）。
/// API Key 明文由 [LlmConfigService] 内部 `_apiKeyVault` 单独管理，
/// 不进入 Riverpod State，不序列化到 JSON。
class LlmCloudProfile {
  final String id;
  final String name;
  final LlmCloudProviderType providerType;

  /// API Key 引用 ID。用于内存保险库索引。
  final String apiKeyRef;

  /// API Key（加密 JSON 中存储，通过 Vault 整体加密保护）。
  /// UI 层应始终遮蔽显示，绝不暴露明文。
  final String apiKey;

  final String endpoint;
  final String model;
  final String? anthropicVersion;

  const LlmCloudProfile({
    required this.id,
    required this.name,
    required this.providerType,
    required this.apiKeyRef,
    this.apiKey = '',
    required this.endpoint,
    required this.model,
    this.anthropicVersion,
  });

  // ---------------------------------------------------------------------------
  // Serialization (apiKey included — protected by Vault encryption)
  // ---------------------------------------------------------------------------
  // NOTE: apiKey is serialized here because the entire JSON blob is encrypted
  // by RustVaultService. The _apiKeyVault in LlmConfigService provides an
  // additional in-memory isolation layer, but persistence relies on Vault.
  // ---------------------------------------------------------------------------

  Map<String, dynamic> toJson() => {
        'id': id,
        'name': name,
        'providerType': providerType.toJson(),
        'apiKeyRef': apiKeyRef,
        'apiKey': apiKey,
        'endpoint': endpoint,
        'model': model,
        'anthropicVersion': anthropicVersion,
      };

  factory LlmCloudProfile.fromJson(Map<String, dynamic> json) {
    return LlmCloudProfile(
      id: json['id'] as String? ?? const Uuid().v4(),
      name: json['name'] as String? ?? 'Unnamed Configuration',
      providerType: LlmCloudProviderTypeExtension.fromJson(
        json['providerType'] as String?,
      ),
      apiKeyRef: json['apiKeyRef'] as String? ?? const Uuid().v4(),
      apiKey: json['apiKey'] as String? ?? '',
      endpoint: json['endpoint'] as String? ?? '',
      model: json['model'] as String? ?? '',
      anthropicVersion: json['anthropicVersion'] as String?,
    );
  }

  // ---------------------------------------------------------------------------
  // Copy
  // ---------------------------------------------------------------------------

  LlmCloudProfile copyWith({
    String? id,
    String? name,
    LlmCloudProviderType? providerType,
    String? apiKeyRef,
    String? apiKey,
    String? endpoint,
    String? model,
    String? anthropicVersion,
  }) {
    return LlmCloudProfile(
      id: id ?? this.id,
      name: name ?? this.name,
      providerType: providerType ?? this.providerType,
      apiKeyRef: apiKeyRef ?? this.apiKeyRef,
      apiKey: apiKey ?? this.apiKey,
      endpoint: endpoint ?? this.endpoint,
      model: model ?? this.model,
      anthropicVersion: anthropicVersion ?? this.anthropicVersion,
    );
  }

  // ---------------------------------------------------------------------------
  // Display helpers
  // ---------------------------------------------------------------------------

  /// API Key 掩码显示（前4 + ... + 后4），通用格式。
  static String maskApiKey(String apiKey, {String? prefix}) {
    if (apiKey.length <= 8) return '${prefix ?? ''}****';
    return '${prefix ?? ''}${apiKey.substring(0, 4)}...${apiKey.substring(apiKey.length - 4)}';
  }

  @override
  String toString() => 'LlmCloudProfile($name, ${providerType.label}, $model)';
}

// =============================================================================
// LLM Config State (UI-facing immutable snapshot)
// =============================================================================

/// Immutable snapshot of LLM configuration for UI consumption.
class LlmConfigState {
  final LlmBackendType backendType;
  final String cloudApiKey;
  final String cloudEndpoint;
  final String cloudModel;
  final String? localModelPath;
  final bool cloudConsent;
  final LlmCloudProviderType cloudProviderType;
  final String cloudAnthropicVersion;

  // Multi-profile fields (no apiKey exposed)
  final List<LlmCloudProfile> cloudProfiles;
  final String? activeCloudProfileId;

  const LlmConfigState({
    this.backendType = LlmBackendType.local,
    this.cloudApiKey = '',
    this.cloudEndpoint = 'https://api.openai.com/v1',
    this.cloudModel = 'gpt-4o-mini',
    this.localModelPath,
    this.cloudConsent = false,
    this.cloudProviderType = LlmCloudProviderType.openai,
    this.cloudAnthropicVersion = '2023-06-01',
    this.cloudProfiles = const [],
    this.activeCloudProfileId,
  });

  /// 当前激活的云端配置（不含 apiKey）。
  LlmCloudProfile? get activeCloudProfile {
    if (activeCloudProfileId == null) return cloudProfiles.isNotEmpty ? cloudProfiles.first : null;
    try {
      return cloudProfiles.firstWhere((p) => p.id == activeCloudProfileId);
    } on StateError catch (_) {
      return cloudProfiles.isNotEmpty ? cloudProfiles.first : null;
    }
  }

  static const _sentinel = Object();

  LlmConfigState copyWith({
    LlmBackendType? backendType,
    Object? cloudApiKey = _sentinel,
    Object? cloudEndpoint = _sentinel,
    Object? cloudModel = _sentinel,
    String? localModelPath,
    Object? cloudConsent = _sentinel,
    LlmCloudProviderType? cloudProviderType,
    Object? cloudAnthropicVersion = _sentinel,
    List<LlmCloudProfile>? cloudProfiles,
    Object? activeCloudProfileId = _sentinel,
  }) {
    return LlmConfigState(
      backendType: backendType ?? this.backendType,
      cloudApiKey: cloudApiKey == _sentinel ? this.cloudApiKey : cloudApiKey as String,
      cloudEndpoint: cloudEndpoint == _sentinel ? this.cloudEndpoint : cloudEndpoint as String,
      cloudModel: cloudModel == _sentinel ? this.cloudModel : cloudModel as String,
      localModelPath: localModelPath ?? this.localModelPath,
      cloudConsent: cloudConsent == _sentinel ? this.cloudConsent : cloudConsent as bool,
      cloudProviderType: cloudProviderType ?? this.cloudProviderType,
      cloudAnthropicVersion: cloudAnthropicVersion == _sentinel ? this.cloudAnthropicVersion : cloudAnthropicVersion as String,
      cloudProfiles: cloudProfiles != null ? List.unmodifiable(cloudProfiles) : this.cloudProfiles,
      activeCloudProfileId: activeCloudProfileId == _sentinel ? this.activeCloudProfileId : activeCloudProfileId as String?,
    );
  }

  /// Whether cloud mode is usable (has active profile + consent).
  bool get canUseCloud {
    if (backendType != LlmBackendType.cloud) return false;
    if (!cloudConsent) return false;
    final active = activeCloudProfile;
    return active != null && active.endpoint.isNotEmpty && active.model.isNotEmpty;
  }
}
