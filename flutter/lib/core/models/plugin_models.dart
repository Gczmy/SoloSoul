// SoloSoul 插件系统 Dart 模型
//
// 注意：PluginManifest 类型由 FRB 从 Rust 侧自动生成，位于 lib/frb/plugin/manifest.dart。
// 本文件仅包含 Dart 端本地需要的模型（registry.json 解析等）。

import 'dart:convert';

import 'package:solosoul_flutter/frb/plugin/manifest.dart' as frb_manifest;


/// 插件注册表（registry.json）
class PluginRegistry {
  final String version;
  final DateTime updatedAt;
  final Map<String, PluginRegistryEntry> plugins;

  PluginRegistry({
    required this.version,
    required this.updatedAt,
    required this.plugins,
  });

  factory PluginRegistry.empty() {
    return PluginRegistry(
      version: '1',
      updatedAt: DateTime.now().toUtc(),
      plugins: {},
    );
  }

  factory PluginRegistry.fromJson(Map<String, dynamic> json) {
    final pluginsMap = (json['plugins'] as Map<String, dynamic>?) ?? {};
    return PluginRegistry(
      version: json['version'] as String? ?? '1',
      updatedAt: DateTime.tryParse(json['updated_at'] as String? ?? '') ?? DateTime.now().toUtc(),
      plugins: pluginsMap.map(
        (k, v) => MapEntry(k, PluginRegistryEntry.fromJson(v as Map<String, dynamic>)),
      ),
    );
  }

  Map<String, dynamic> toJson() => {
    'version': version,
    'updated_at': updatedAt.toIso8601String(),
    'plugins': plugins.map((k, v) => MapEntry(k, v.toJson())),
  };
}

/// 注册表中的插件条目
class PluginRegistryEntry {
  final String name;
  final String publisher;
  final String latestVersion;
  final Map<String, PluginVersionInfo> versions;
  /// 插件功能介绍（fallback，当 i18n 中无当前语言时使用）
  final String? description;
  /// 插件主页 URL
  final String? homepage;
  /// 多语言信息：locale -> {field -> text}
  /// 例如 {"zh": {"name": "地址格式化器", "description": "..."}}
  final Map<String, Map<String, String>>? i18n;

  PluginRegistryEntry({
    required this.name,
    required this.publisher,
    required this.latestVersion,
    required this.versions,
    this.description,
    this.homepage,
    this.i18n,
  });

  factory PluginRegistryEntry.fromJson(Map<String, dynamic> json) {
    final versionsMap = (json['versions'] as Map<String, dynamic>?) ?? {};
    final i18nRaw = json['i18n'] as Map<String, dynamic>?;
    return PluginRegistryEntry(
      name: json['name'] as String,
      publisher: json['publisher'] as String,
      latestVersion: json['latest_version'] as String,
      versions: versionsMap.map(
        (k, v) => MapEntry(k, PluginVersionInfo.fromJson(v as Map<String, dynamic>)),
      ),
      description: json['description'] as String?,
      homepage: json['homepage'] as String?,
      i18n: i18nRaw?.map(
        (locale, fields) => MapEntry(
          locale,
          (fields as Map<String, dynamic>).map(
            (k, v) => MapEntry(k, v as String),
          ),
        ),
      ),
    );
  }

  Map<String, dynamic> toJson() => {
    'name': name,
    'publisher': publisher,
    'latest_version': latestVersion,
    'versions': versions.map((k, v) => MapEntry(k, v.toJson())),
    if (description != null) 'description': description,
    if (homepage != null) 'homepage': homepage,
    if (i18n != null) 'i18n': i18n,
  };
}

/// 插件版本信息
class PluginVersionInfo {
  final String sha256;
  final String pluginApiVersion;
  final String minAppVersion;
  final String maxAppVersion;
  /// 优先下载地址（jsDelivr CDN 加速）
  final String downloadUrl;
  /// GitHub Raw 直连 fallback 地址
  final String? rawUrl;
  final DateTime releasedAt;
  /// 版本变更日志（支持多语言，默认中文）
  final String? changelog;

  PluginVersionInfo({
    required this.sha256,
    required this.pluginApiVersion,
    required this.minAppVersion,
    required this.maxAppVersion,
    required this.downloadUrl,
    this.rawUrl,
    required this.releasedAt,
    this.changelog,
  });

  factory PluginVersionInfo.fromJson(Map<String, dynamic> json) {
    return PluginVersionInfo(
      sha256: json['sha256'] as String,
      pluginApiVersion: json['plugin_api_version'] as String,
      minAppVersion: json['min_app_version'] as String,
      maxAppVersion: json['max_app_version'] as String,
      downloadUrl: json['download_url'] as String,
      rawUrl: json['raw_url'] as String?,
      releasedAt: DateTime.tryParse(json['released_at'] as String? ?? '') ?? DateTime.now().toUtc(),
      changelog: json['changelog'] as String?,
    );
  }

  Map<String, dynamic> toJson() => {
    'sha256': sha256,
    'plugin_api_version': pluginApiVersion,
    'min_app_version': minAppVersion,
    'max_app_version': maxAppVersion,
    'download_url': downloadUrl,
    if (rawUrl != null) 'raw_url': rawUrl,
    'released_at': releasedAt.toIso8601String(),
    if (changelog != null) 'changelog': changelog,
  };
}

/// 插件市场源配置
///
/// 用户可配置多个插件源，每个源对应一个 GitHub 公开仓库。
class PluginSource {
  final String name;
  final String repoOwner;
  final String repoName;
  final String branch;
  final bool useCdn;

  const PluginSource({
    required this.name,
    required this.repoOwner,
    required this.repoName,
    this.branch = 'main',
    this.useCdn = true,
  });

  /// 官方默认源
  static const official = PluginSource(
    name: 'SoloSoul Official',
    repoOwner: 'Gczmy',
    repoName: 'SoloSoul_plugin_market',
    branch: 'main',
    useCdn: true,
  );

  /// registry.json 的主地址（CDN 优先，中国大陆访问更快）
  String get registryUrl => useCdn
      ? 'https://cdn.jsdelivr.net/gh/$repoOwner/$repoName@$branch/registry.json'
      : 'https://raw.githubusercontent.com/$repoOwner/$repoName/$branch/registry.json';

  /// registry.json 的备用地址（CDN 缓存未刷新时回退）
  String get fallbackRegistryUrl => useCdn
      ? 'https://raw.githubusercontent.com/$repoOwner/$repoName/$branch/registry.json'
      : 'https://cdn.jsdelivr.net/gh/$repoOwner/$repoName@$branch/registry.json';

  @override
  String toString() => 'PluginSource($name: $repoOwner/$repoName@$branch)';
}

/// 已安装插件信息（installed.json 中的条目）
class InstalledPluginInfo {
  final String version;
  final String status; // 'installed' | 'uninstalled'
  final DateTime? installedAt;
  final DateTime? uninstalledAt;
  final DateTime? lastUsedAt;

  InstalledPluginInfo({
    required this.version,
    required this.status,
    this.installedAt,
    this.uninstalledAt,
    this.lastUsedAt,
  });

  factory InstalledPluginInfo.fromJson(Map<String, dynamic> json) {
    return InstalledPluginInfo(
      version: json['version'] as String? ?? '',
      status: json['status'] as String? ?? 'installed',
      installedAt: json['installed_at'] != null
          ? DateTime.tryParse(json['installed_at'] as String)
          : null,
      uninstalledAt: json['uninstalled_at'] != null
          ? DateTime.tryParse(json['uninstalled_at'] as String)
          : null,
      lastUsedAt: json['last_used_at'] != null
          ? DateTime.tryParse(json['last_used_at'] as String)
          : null,
    );
  }

  Map<String, dynamic> toJson() {
    final map = <String, dynamic>{
      'version': version,
      'status': status,
    };
    if (installedAt != null) map['installed_at'] = installedAt!.toIso8601String();
    if (uninstalledAt != null) map['uninstalled_at'] = uninstalledAt!.toIso8601String();
    if (lastUsedAt != null) map['last_used_at'] = lastUsedAt!.toIso8601String();
    return map;
  }
}

/// 插件更新信息
class PluginUpdateInfo {
  final String pluginId;
  final String currentVersion;
  final String latestVersion;

  PluginUpdateInfo({
    required this.pluginId,
    required this.currentVersion,
    required this.latestVersion,
  });
}

/// 插件运行结果
class PluginRunResult {
  final int exitCode;

  PluginRunResult({required this.exitCode});
}

/// 插件相关异常
class PluginNotFoundException implements Exception {
  final String pluginId;
  PluginNotFoundException(this.pluginId);
  @override
  String toString() => 'PluginNotFoundException: $pluginId';
}

class PluginIncompatibleException implements Exception {
  final String pluginId;
  PluginIncompatibleException(this.pluginId);
  @override
  String toString() => 'PluginIncompatibleException: $pluginId';
}

class PluginSecurityException implements Exception {
  final String message;
  PluginSecurityException(this.message);
  @override
  String toString() => 'PluginSecurityException: $message';
}

// ============================================================================
// i18n 工具函数
// ============================================================================

/// 从插件多语言信息中获取当前语言对应的文本。
///
/// [i18n] 为插件 manifest 或 registry 中的 `i18n` 字段，格式：
/// `{"zh": {"name": "...", "description": "..."}, "en": {...}}`
///
/// [field] 为要获取的字段名，如 `"name"`、`"description"`。
/// [locale] 为当前语言代码（如 `"zh"`、`"en"`）。
/// [fallback] 为找不到对应翻译时的回退文本。
String resolvePluginI18n(
  Map<String, Map<String, String>>? i18n,
  String field,
  String locale,
  String fallback,
) {
  if (i18n == null || i18n.isEmpty) return fallback;

  // 精确匹配，如 "zh_CN" -> "zh_CN"
  final exact = i18n[locale]?[field];
  if (exact != null && exact.isNotEmpty) return exact;

  // 语言前缀匹配，如 "zh_CN" -> "zh"
  final langPrefix = locale.split('_').first;
  final prefixMatch = i18n[langPrefix]?[field];
  if (prefixMatch != null && prefixMatch.isNotEmpty) return prefixMatch;

  // 回退到默认语言（中文或英文）
  final zhFallback = i18n['zh']?[field];
  if (zhFallback != null && zhFallback.isNotEmpty) return zhFallback;

  final enFallback = i18n['en']?[field];
  if (enFallback != null && enFallback.isNotEmpty) return enFallback;

  return fallback;
}

/// 插件下载工件（wasm + manifest）
class PluginArtifacts {
  final List<int> wasmBytes;
  final String manifestJson;
  final String version;

  PluginArtifacts({
    required this.wasmBytes,
    required this.manifestJson,
    required this.version,
  });

  /// 从 manifest JSON 中解析 field_access 列表
  List<Map<String, dynamic>>? parseFieldAccess() {
    try {
      final manifest = jsonDecode(manifestJson) as Map<String, dynamic>;
      final fieldAccess = manifest['field_access'] as List<dynamic>?;
      if (fieldAccess == null || fieldAccess.isEmpty) return null;
      return fieldAccess.cast<Map<String, dynamic>>();
    } on Exception {
      return null;
    }
  }

  /// 将 manifest JSON 解析为 PluginManifest 对象
  frb_manifest.PluginManifest? toManifest() {
    try {
      final m = jsonDecode(manifestJson) as Map<String, dynamic>;
      frb_manifest.NetworkPolicy? parseNetworkPolicy() {
        final np = m['network_policy'] as Map<String, dynamic>?;
        if (np == null) return null;
        return frb_manifest.NetworkPolicy(
          allowedDomains: (np['allowed_domains'] as List<dynamic>? ?? []).cast<String>(),
          blockAllOutbound: np['block_all_outbound'] as bool? ?? false,
        );
      }

      Map<String, Map<String, String>> parseI18N() {
        final raw = m['i18n'] as Map<String, dynamic>?;
        if (raw == null) return {};
        return raw.map((k, v) => MapEntry(k, (v as Map<String, dynamic>).cast<String, String>()));
      }

      return frb_manifest.PluginManifest(
        pluginId: m['plugin_id'] as String? ?? '',
        name: m['name'] as String? ?? '',
        version: m['version'] as String? ?? '',
        pluginApiVersion: m['plugin_api_version'] as String? ?? '',
        minAppVersion: m['min_app_version'] as String? ?? '',
        maxAppVersion: m['max_app_version'] as String? ?? '',
        description: m['description'] as String? ?? '',
        publisher: m['publisher'] as String? ?? '',
        homepage: m['homepage'] as String?,
        signature: m['signature'] as String?,
        requiredFields: (m['required_fields'] as List<dynamic>? ?? []).cast<String>(),
        optionalFields: (m['optional_fields'] as List<dynamic>? ?? []).cast<String>(),
        networkPolicy: parseNetworkPolicy(),
        dataTtlSeconds: BigInt.from(m['data_ttl_seconds'] as int? ?? 300),
        requireUserConfirmation: m['require_user_confirmation'] as bool? ?? true,
        consentValidityHours: BigInt.from(m['consent_validity_hours'] as int? ?? 24),
        i18N: parseI18N(),
      );
    } on Exception {
      return null;
    }
  }
}

class PluginExecutionException implements Exception {
  final String message;
  PluginExecutionException(this.message);
  @override
  String toString() => 'PluginExecutionException: $message';
}
