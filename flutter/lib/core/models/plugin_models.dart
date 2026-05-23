// SoloSoul 插件系统 Dart 模型
//
// 注意：PluginManifest 类型由 FRB 从 Rust 侧自动生成，位于 lib/frb/plugin/manifest.dart。
// 本文件仅包含 Dart 端本地需要的模型（registry.json 解析等）。


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

  PluginRegistryEntry({
    required this.name,
    required this.publisher,
    required this.latestVersion,
    required this.versions,
  });

  factory PluginRegistryEntry.fromJson(Map<String, dynamic> json) {
    final versionsMap = (json['versions'] as Map<String, dynamic>?) ?? {};
    return PluginRegistryEntry(
      name: json['name'] as String,
      publisher: json['publisher'] as String,
      latestVersion: json['latest_version'] as String,
      versions: versionsMap.map(
        (k, v) => MapEntry(k, PluginVersionInfo.fromJson(v as Map<String, dynamic>)),
      ),
    );
  }

  Map<String, dynamic> toJson() => {
    'name': name,
    'publisher': publisher,
    'latest_version': latestVersion,
    'versions': versions.map((k, v) => MapEntry(k, v.toJson())),
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

  PluginVersionInfo({
    required this.sha256,
    required this.pluginApiVersion,
    required this.minAppVersion,
    required this.maxAppVersion,
    required this.downloadUrl,
    this.rawUrl,
    required this.releasedAt,
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

  /// registry.json 的远程地址
  String get registryUrl => useCdn
      ? 'https://cdn.jsdelivr.net/gh/$repoOwner/$repoName@$branch/registry.json'
      : 'https://raw.githubusercontent.com/$repoOwner/$repoName/$branch/registry.json';

  @override
  String toString() => 'PluginSource($name: $repoOwner/$repoName@$branch)';
}

/// 已安装插件信息（installed.json 中的条目）
class InstalledPluginInfo {
  final String version;
  final String status; // 'installed' | 'uninstalled'
  final DateTime? installedAt;
  final DateTime? uninstalledAt;

  InstalledPluginInfo({
    required this.version,
    required this.status,
    this.installedAt,
    this.uninstalledAt,
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
    );
  }

  Map<String, dynamic> toJson() {
    final map = <String, dynamic>{
      'version': version,
      'status': status,
    };
    if (installedAt != null) map['installed_at'] = installedAt!.toIso8601String();
    if (uninstalledAt != null) map['uninstalled_at'] = uninstalledAt!.toIso8601String();
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

class PluginExecutionException implements Exception {
  final String message;
  PluginExecutionException(this.message);
  @override
  String toString() => 'PluginExecutionException: $message';
}
