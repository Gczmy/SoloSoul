import 'dart:convert';

import 'package:solosoul_flutter/core/services/llm/llm_config_models.dart';
import 'package:solosoul_flutter/core/services/llm/llm_service.dart';
import 'package:solosoul_flutter/core/services/llm/llm_usage_stats.dart';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:uuid/uuid.dart';

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

  /// 内存中的 API Key 保险库。Key: apiKeyRef, Value: apiKey 明文。
  /// **绝不序列化**，仅在应用生命周期内存在内存中。
  final Map<String, String> _apiKeyVault = {};

  // ---------------------------------------------------------------------------
  // Load / Save
  // ---------------------------------------------------------------------------

  Future<_LlmConfig> _load(String accountId) async {
    final jsonStr = await _vault.loadSettingDecrypted(accountId);
    if (jsonStr == null) {
      SoloLog.d('LlmConfigService', '_load account=$accountId 无已存配置');
      return const _LlmConfig();
    }
    try {
      final map = jsonDecode(jsonStr) as Map<String, dynamic>;
      final config = _LlmConfig.fromJson(map);
      SoloLog.d('LlmConfigService', '_load account=$accountId '
          'usage=${config.usageCount} prompt=${config.totalPromptTokens} '
          'completion=${config.totalCompletionTokens} '
          'models=${config.perModelStats.length} days=${config.dailyStats.length}');
      // 自动迁移：旧配置无 profiles 时，将单配置字段包装为第一个 Profile
      if (config.cloudProfiles.isEmpty &&
          (config._legacyCloudApiKey?.isNotEmpty == true ||
           config._legacyCloudEndpoint?.isNotEmpty == true)) {
        final ref = const Uuid().v4();
        final migratedProfile = LlmCloudProfile(
          id: const Uuid().v4(),
          name: '默认配置',
          providerType: config.cloudProviderType,
          apiKeyRef: ref,
          apiKey: config._legacyCloudApiKey ?? '',
          endpoint: config._legacyCloudEndpoint ?? 'https://api.openai.com/v1',
          model: config._legacyCloudModel ?? 'gpt-4o-mini',
          anthropicVersion: config.cloudAnthropicVersion,
        );
        _apiKeyVault[ref] = config._legacyCloudApiKey ?? '';
        final migrated = config.copyWith(
          cloudProfiles: [migratedProfile],
          activeCloudProfileId: migratedProfile.id,
          cloudApiKey: null,
          cloudEndpoint: null,
          cloudModel: null,
        );
        // 保存迁移后的配置，确保下次加载时 id 一致
        await _save(accountId, migrated);
        return migrated;
      }
      // 从加密配置中恢复 API Key 到内存保险库
      for (final p in config.cloudProfiles) {
        if (p.apiKey.isNotEmpty) {
          _apiKeyVault[p.apiKeyRef] = p.apiKey;
        }
      }
      return config;
    } on Object catch (e, st) {
      SoloLog.e('LlmConfigService', '_load 配置解析失败，重置为默认', e, st);
      return const _LlmConfig();
    }
  }

  Future<void> _save(String accountId, _LlmConfig config) async {
    final jsonData = jsonEncode(config.toJson());
    SoloLog.d('LlmConfigService', '_save account=$accountId '
        'usage=${config.usageCount} prompt=${config.totalPromptTokens} '
        'completion=${config.totalCompletionTokens} '
        'models=${config.perModelStats.length} days=${config.dailyStats.length}');
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
  // Cloud Profile CRUD (secure, apiKey isolated in memory)
  // ---------------------------------------------------------------------------

  Future<List<LlmCloudProfile>> getCloudProfiles(String accountId) async {
    final config = await _load(accountId);
    return List.unmodifiable(config.cloudProfiles);
  }

  Future<String> addCloudProfile(
    String accountId, {
    required String name,
    required LlmCloudProviderType providerType,
    required String apiKey,
    required String endpoint,
    required String model,
    String? anthropicVersion,
  }) async {
    final config = await _load(accountId);
    final ref = const Uuid().v4();
    final profile = LlmCloudProfile(
      id: const Uuid().v4(),
      name: name,
      providerType: providerType,
      apiKeyRef: ref,
      apiKey: apiKey,
      endpoint: endpoint,
      model: model,
      anthropicVersion: anthropicVersion,
    );
    _apiKeyVault[ref] = apiKey;
    final updatedProfiles = [...config.cloudProfiles, profile];
    var updated = config.copyWith(cloudProfiles: updatedProfiles);
    // 第一个 profile 自动设为激活
    if (updated.activeCloudProfileId == null) {
      updated = updated.copyWith(activeCloudProfileId: profile.id);
    }
    await _save(accountId, updated);
    return profile.id;
  }

  Future<void> updateCloudProfile(
    String accountId, {
    required String profileId,
    String? name,
    LlmCloudProviderType? providerType,
    String? apiKey, // null = 不修改
    String? endpoint,
    String? model,
    String? anthropicVersion,
  }) async {
    final config = await _load(accountId);
    final profiles = config.cloudProfiles.map((p) {
      if (p.id != profileId) return p;
      final newApiKey = (apiKey != null && apiKey.isNotEmpty) ? apiKey : p.apiKey;
      if (newApiKey.isNotEmpty) {
        _apiKeyVault[p.apiKeyRef] = newApiKey;
      }
      return p.copyWith(
        name: name,
        providerType: providerType,
        apiKey: newApiKey,
        endpoint: endpoint,
        model: model,
        anthropicVersion: anthropicVersion,
      );
    }).toList();
    await _save(accountId, config.copyWith(cloudProfiles: profiles));
  }

  Future<void> deleteCloudProfile(
    String accountId,
    String profileId,
  ) async {
    final config = await _load(accountId);
    final profiles = config.cloudProfiles.where((p) => p.id != profileId).toList();

    // 若删空所有云端配置，同时清除 legacy 字段防止自动迁移重新触发
    final isEmpty = profiles.isEmpty;
    var updated = config.copyWith(
      cloudProfiles: profiles,
      activeCloudProfileId: config.activeCloudProfileId == profileId
          ? (isEmpty ? null : profiles.first.id)
          : config.activeCloudProfileId,
      cloudApiKey: isEmpty ? null : config._legacyCloudApiKey,
      cloudEndpoint: isEmpty ? null : config._legacyCloudEndpoint,
      cloudModel: isEmpty ? null : config._legacyCloudModel,
    );

    // 清理内存中的 API Key
    final removedProfiles = config.cloudProfiles.where((p) => p.id == profileId).toList();
    if (removedProfiles.isNotEmpty) {
      _apiKeyVault.remove(removedProfiles.first.apiKeyRef);
    }
    await _save(accountId, updated);
  }

  Future<String?> getActiveCloudProfileId(String accountId) async {
    final config = await _load(accountId);
    return config.activeCloudProfileId ??
        (config.cloudProfiles.isNotEmpty ? config.cloudProfiles.first.id : null);
  }

  Future<void> setActiveCloudProfileId(
    String accountId,
    String profileId,
  ) async {
    final config = await _load(accountId);
    if (!config.cloudProfiles.any((p) => p.id == profileId)) {
      throw Exception('配置不存在');
    }
    await _save(accountId, config.copyWith(activeCloudProfileId: profileId));
  }

  /// 通过 apiKeyRef 获取 API Key 明文。
  /// **仅应在构造 LlmCloudService 时调用，用后立即丢弃。**
  Future<String?> getApiKeyByRef(String apiKeyRef) async {
    return _apiKeyVault[apiKeyRef];
  }

  // ---------------------------------------------------------------------------
  // Legacy getters (auto-proxy to active profile for backward compat)
  // ---------------------------------------------------------------------------

  Future<String?> getCloudApiKey(String accountId) async {
    final config = await _load(accountId);
    final activeId = config.activeCloudProfileId;
    if (activeId == null || config.cloudProfiles.isEmpty) {
      return config._legacyCloudApiKey;
    }
    final profile = config.cloudProfiles.firstWhere(
      (p) => p.id == activeId,
      orElse: () => config.cloudProfiles.first,
    );
    return _apiKeyVault[profile.apiKeyRef];
  }

  Future<void> setCloudApiKey(String accountId, String apiKey) async {
    final config = await _load(accountId);
    final activeId = config.activeCloudProfileId;
    if (activeId != null) {
      final profile = config.cloudProfiles.firstWhere((p) => p.id == activeId);
      _apiKeyVault[profile.apiKeyRef] = apiKey;
    }
    await _save(accountId, config.copyWith(cloudApiKey: apiKey));
  }

  Future<void> clearCloudApiKey(String accountId) async {
    final config = await _load(accountId);
    final activeId = config.activeCloudProfileId;
    if (activeId != null) {
      final profile = config.cloudProfiles.firstWhere((p) => p.id == activeId);
      _apiKeyVault[profile.apiKeyRef] = '';
    }
    await _save(accountId, config.copyWith(cloudApiKey: null));
  }

  Future<String> getCloudEndpoint(String accountId) async {
    final config = await _load(accountId);
    final activeId = config.activeCloudProfileId;
    if (activeId == null || config.cloudProfiles.isEmpty) {
      return config.cloudEndpoint ?? 'https://api.openai.com/v1';
    }
    final profile = config.cloudProfiles.firstWhere(
      (p) => p.id == activeId,
      orElse: () => config.cloudProfiles.first,
    );
    return profile.endpoint;
  }

  Future<void> setCloudEndpoint(String accountId, String endpoint) async {
    final config = await _load(accountId);
    await _save(accountId, config.copyWith(cloudEndpoint: endpoint));
  }

  Future<String> getCloudModel(String accountId) async {
    final config = await _load(accountId);
    final activeId = config.activeCloudProfileId;
    if (activeId == null || config.cloudProfiles.isEmpty) {
      return config.cloudModel ?? 'gpt-4o-mini';
    }
    final profile = config.cloudProfiles.firstWhere(
      (p) => p.id == activeId,
      orElse: () => config.cloudProfiles.first,
    );
    return profile.model;
  }

  Future<void> setCloudModel(String accountId, String model) async {
    final config = await _load(accountId);
    await _save(accountId, config.copyWith(cloudModel: model));
  }

  Future<LlmCloudProviderType> getCloudProviderType(String accountId) async {
    final config = await _load(accountId);
    final activeId = config.activeCloudProfileId;
    if (activeId == null || config.cloudProfiles.isEmpty) {
      return config.cloudProviderType;
    }
    final profile = config.cloudProfiles.firstWhere(
      (p) => p.id == activeId,
      orElse: () => config.cloudProfiles.first,
    );
    return profile.providerType;
  }

  Future<void> setCloudProviderType(
    String accountId,
    LlmCloudProviderType type,
  ) async {
    final config = await _load(accountId);
    await _save(accountId, config.copyWith(cloudProviderType: type));
  }

  Future<String> getCloudAnthropicVersion(String accountId) async {
    final config = await _load(accountId);
    final activeId = config.activeCloudProfileId;
    if (activeId == null || config.cloudProfiles.isEmpty) {
      return config.cloudAnthropicVersion ?? '2023-06-01';
    }
    final profile = config.cloudProfiles.firstWhere(
      (p) => p.id == activeId,
      orElse: () => config.cloudProfiles.first,
    );
    return profile.anthropicVersion ?? '2023-06-01';
  }

  Future<void> setCloudAnthropicVersion(
    String accountId,
    String version,
  ) async {
    final config = await _load(accountId);
    await _save(accountId, config.copyWith(cloudAnthropicVersion: version));
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

  // ---------------------------------------------------------------------------
  // Batch load (single Vault IO)
  // ---------------------------------------------------------------------------

  /// 一次性加载所有配置字段，仅触发一次 Vault IO。
  Future<LlmConfigState> getLlmConfigState(String accountId) async {
    final config = await _load(accountId);
    final activeId = config.activeCloudProfileId;

    String apiKey = '';
    String endpoint = 'https://api.openai.com/v1';
    String model = 'gpt-4o-mini';
    LlmCloudProviderType providerType = config.cloudProviderType;
    String anthropicVersion = '2023-06-01';

    if (activeId != null && config.cloudProfiles.isNotEmpty) {
      final profile = config.cloudProfiles.firstWhere(
        (p) => p.id == activeId,
        orElse: () => config.cloudProfiles.first,
      );
      apiKey = _apiKeyVault[profile.apiKeyRef] ?? '';
      endpoint = profile.endpoint;
      model = profile.model;
      providerType = profile.providerType;
      anthropicVersion = profile.anthropicVersion ?? '2023-06-01';
    } else {
      apiKey = config._legacyCloudApiKey ?? '';
      endpoint = config.cloudEndpoint ?? 'https://api.openai.com/v1';
      model = config.cloudModel ?? 'gpt-4o-mini';
      anthropicVersion = config.cloudAnthropicVersion ?? '2023-06-01';
    }

    return LlmConfigState(
      backendType: config.backendType,
      cloudApiKey: apiKey,
      cloudEndpoint: endpoint,
      cloudModel: model,
      localModelPath: config.localModelPath,
      cloudConsent: config.cloudConsent == true,
      cloudProviderType: providerType,
      cloudAnthropicVersion: anthropicVersion,
      cloudProfiles: config.cloudProfiles,
      activeCloudProfileId: activeId,
    );
  }

  // ---------------------------------------------------------------------------
  // Usage Stats (persisted within the same encrypted config blob)
  // ---------------------------------------------------------------------------

  Future<LlmUsageStats> getStats(String accountId) async {
    final config = await _load(accountId);
    return LlmUsageStats(
      usageCount: config.usageCount,
      totalPromptTokens: config.totalPromptTokens,
      totalCompletionTokens: config.totalCompletionTokens,
      lastLoadTime: config._parseDt(config.statsLastLoadTime),
      lastUsedTime: config._parseDt(config.statsLastUsedTime),
      perModelStats: config.perModelStats,
      dailyStats: config.dailyStats,
    );
  }

  Future<void> setStats(String accountId, LlmUsageStats stats) async {
    final config = await _load(accountId);
    await _save(accountId, config.copyWith(
      usageCount: stats.usageCount,
      totalPromptTokens: stats.totalPromptTokens,
      totalCompletionTokens: stats.totalCompletionTokens,
      statsLastLoadTime: stats.lastLoadTime?.toIso8601String(),
      statsLastUsedTime: stats.lastUsedTime?.toIso8601String(),
      perModelStats: stats.perModelStats,
      dailyStats: stats.dailyStats,
    ));
  }

}

// =============================================================================
// Internal Config Model
// =============================================================================

/// 安全解析 JSON 列表，单个元素解析失败时跳过而非抛出。
List<T> _safeParseList<T>(
  dynamic json,
  T Function(dynamic) parser,
) {
  final list = json as List<dynamic>?;
  if (list == null) return const [];
  final result = <T>[];
  for (final e in list) {
    try {
      result.add(parser(e));
    } on Object catch (err) {
      SoloLog.w('LlmConfigService', '列表元素解析失败，跳过', err);
    }
  }
  return result;
}

class _LlmConfig {
  final LlmBackendType backendType;

  // Legacy single-config fields (kept for backward-compat, not used by UI)
  final String? _legacyCloudApiKey;
  final String? _legacyCloudEndpoint;
  final String? _legacyCloudModel;
  final String? localModelPath;
  final bool? cloudConsent;
  final LlmCloudProviderType cloudProviderType;
  final String? cloudAnthropicVersion;

  // New multi-profile fields
  final List<LlmCloudProfile> cloudProfiles;
  final String? activeCloudProfileId;

  // Usage stats fields (account-level persisted)
  final int usageCount;
  final int totalPromptTokens;
  final int totalCompletionTokens;
  final String? statsLastLoadTime;
  final String? statsLastUsedTime;
  final List<LlmModelUsage> perModelStats;
  final List<LlmDailyUsage> dailyStats;

  const _LlmConfig({
    this.backendType = LlmBackendType.local,
    String? cloudApiKey,
    String? cloudEndpoint,
    String? cloudModel,
    this.localModelPath,
    this.cloudConsent,
    this.cloudProviderType = LlmCloudProviderType.openai,
    this.cloudAnthropicVersion,
    this.cloudProfiles = const [],
    this.activeCloudProfileId,
    this.usageCount = 0,
    this.totalPromptTokens = 0,
    this.totalCompletionTokens = 0,
    this.statsLastLoadTime,
    this.statsLastUsedTime,
    this.perModelStats = const [],
    this.dailyStats = const [],
  })  : _legacyCloudApiKey = cloudApiKey,
        _legacyCloudEndpoint = cloudEndpoint,
        _legacyCloudModel = cloudModel;

  DateTime? _parseDt(String? raw) {
    if (raw == null || raw.isEmpty) return null;
    return DateTime.tryParse(raw);
  }

  // Legacy accessors
  String? get cloudApiKey => _legacyCloudApiKey;
  String? get cloudEndpoint => _legacyCloudEndpoint;
  String? get cloudModel => _legacyCloudModel;

  static const _sentinel = Object();

  _LlmConfig copyWith({
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
    int? usageCount,
    int? totalPromptTokens,
    int? totalCompletionTokens,
    Object? statsLastLoadTime = _sentinel,
    Object? statsLastUsedTime = _sentinel,
    List<LlmModelUsage>? perModelStats,
    List<LlmDailyUsage>? dailyStats,
  }) {
    return _LlmConfig(
      backendType: backendType ?? this.backendType,
      cloudApiKey: cloudApiKey == _sentinel ? _legacyCloudApiKey : cloudApiKey as String?,
      cloudEndpoint: cloudEndpoint == _sentinel ? _legacyCloudEndpoint : cloudEndpoint as String?,
      cloudModel: cloudModel == _sentinel ? _legacyCloudModel : cloudModel as String?,
      localModelPath: localModelPath ?? this.localModelPath,
      cloudConsent: cloudConsent == _sentinel ? this.cloudConsent : cloudConsent as bool?,
      cloudProviderType: cloudProviderType ?? this.cloudProviderType,
      cloudAnthropicVersion: cloudAnthropicVersion == _sentinel ? this.cloudAnthropicVersion : cloudAnthropicVersion as String?,
      cloudProfiles: cloudProfiles ?? this.cloudProfiles,
      activeCloudProfileId: activeCloudProfileId == _sentinel ? this.activeCloudProfileId : activeCloudProfileId as String?,
      usageCount: usageCount ?? this.usageCount,
      totalPromptTokens: totalPromptTokens ?? this.totalPromptTokens,
      totalCompletionTokens: totalCompletionTokens ?? this.totalCompletionTokens,
      statsLastLoadTime: statsLastLoadTime == _sentinel ? this.statsLastLoadTime : statsLastLoadTime as String?,
      statsLastUsedTime: statsLastUsedTime == _sentinel ? this.statsLastUsedTime : statsLastUsedTime as String?,
      perModelStats: perModelStats ?? this.perModelStats,
      dailyStats: dailyStats ?? this.dailyStats,
    );
  }

  Map<String, dynamic> toJson() => {
        'backendType': backendType.name,
        'cloudApiKey': _legacyCloudApiKey,
        'cloudEndpoint': _legacyCloudEndpoint,
        'cloudModel': _legacyCloudModel,
        'localModelPath': localModelPath,
        'cloudConsent': cloudConsent,
        'cloudProviderType': cloudProviderType.name,
        'cloudAnthropicVersion': cloudAnthropicVersion,
        'cloudProfiles': cloudProfiles.map((p) => p.toJson()).toList(),
        'activeCloudProfileId': activeCloudProfileId,
        'usageCount': usageCount,
        'totalPromptTokens': totalPromptTokens,
        'totalCompletionTokens': totalCompletionTokens,
        'statsLastLoadTime': statsLastLoadTime,
        'statsLastUsedTime': statsLastUsedTime,
        'perModelStats': perModelStats.map((m) => m.toJson()).toList(),
        'dailyStats': dailyStats.map((d) => d.toJson()).toList(),
      };

  factory _LlmConfig.fromJson(Map<String, dynamic> json) {
    LlmBackendType parseBackend(String? raw) {
      if (raw == null) return LlmBackendType.local;
      return LlmBackendType.values.byName(raw);
    }

    final profilesJson = json['cloudProfiles'] as List<dynamic>?;
    final profiles = profilesJson != null
        ? profilesJson
            .map((e) => LlmCloudProfile.fromJson(e as Map<String, dynamic>))
            .toList()
        : const <LlmCloudProfile>[];

    return _LlmConfig(
      backendType: parseBackend(json['backendType'] as String?),
      cloudApiKey: json['cloudApiKey'] as String?,
      cloudEndpoint: json['cloudEndpoint'] as String?,
      cloudModel: json['cloudModel'] as String?,
      localModelPath: json['localModelPath'] as String?,
      cloudConsent: json['cloudConsent'] as bool?,
      cloudProviderType:
          LlmCloudProviderTypeExtension.fromJson(json['cloudProviderType'] as String?),
      cloudAnthropicVersion: json['cloudAnthropicVersion'] as String?,
      cloudProfiles: profiles,
      activeCloudProfileId: json['activeCloudProfileId'] as String?,
      usageCount: json['usageCount'] as int? ?? 0,
      totalPromptTokens: json['totalPromptTokens'] as int? ?? (json['totalTokensUsed'] as int? ?? 0),
      totalCompletionTokens: json['totalCompletionTokens'] as int? ?? 0,
      statsLastLoadTime: json['statsLastLoadTime'] as String?,
      statsLastUsedTime: json['statsLastUsedTime'] as String?,
      perModelStats: _safeParseList(
        json['perModelStats'],
        (e) => LlmModelUsage.fromJson(e as Map<String, dynamic>),
      ),
      dailyStats: _safeParseList(
        json['dailyStats'],
        (e) => LlmDailyUsage.fromJson(e as Map<String, dynamic>),
      ),
    );
  }
}
