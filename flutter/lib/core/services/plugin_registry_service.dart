import 'dart:convert';
import 'dart:io';

import 'package:flutter/services.dart';
import 'package:http/http.dart' as http;

import 'package:solosoul_flutter/core/models/plugin_models.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/frb/api.dart' as frb;

/// 插件注册表服务：管理远程 registry 的拉取与本地缓存
///
/// 支持多插件源配置，每个源对应一个 GitHub 公开仓库。
/// 默认使用官方市场（jsDelivr CDN 加速 + GitHub Raw 回退）。
///
/// ⚠️ 路径必须与 Rust 侧 PluginStore::base_dir() 完全一致。
class PluginRegistryService {
  static const _builtinAssetPath = 'assets/registry.json';

  /// 已配置的插件源列表，默认包含官方源。
  ///
  /// 使用 nullable 内部字段 + getter，防御热重载后旧实例字段为 null 的边缘情况。
  final List<PluginSource>? _sources;

  List<PluginSource> get sources => _sources ?? [PluginSource.official];

  late final Directory _pluginDir;

  PluginRegistryService({
    List<PluginSource>? sources,
  }) : _sources = sources ?? [PluginSource.official];

  Future<void> initialize() async {
    final baseDir = await frb.frbGetPluginBaseDir();
    _pluginDir = Directory(baseDir);
    await _pluginDir.create(recursive: true);
  }

  /// 获取合并后的注册表（所有远程源优先，离线回退本地缓存，首次启动回退内置资源）
  ///
  /// 多个源的插件按 ID 合并，同名插件后覆盖前（用户自定义源优先级更高）。
  Future<PluginRegistry> getRegistry() async {
    // 1. 尝试从远程源获取
    final remoteRegistry = await _fetchFromSources();

    // 2. 获取内置注册表（作为版本基准，防御 CDN 缓存延迟）
    final builtinRegistry = await _loadBuiltinRegistry();

    // 3. 合并：远程 + 内置，取每个插件的较新版本
    if (remoteRegistry != null && builtinRegistry != null) {
      final merged = _mergeRegistries(remoteRegistry, builtinRegistry);
      await _saveLocalCache(jsonEncode(merged.toJson()));
      return merged;
    }

    if (remoteRegistry != null) {
      await _saveLocalCache(jsonEncode(remoteRegistry.toJson()));
      return remoteRegistry;
    }

    // 4. 离线回退：本地缓存（即使过期）
    final cached = await _loadLocalCache();
    if (cached != null) {
      return cached;
    }

    // 5. 首次启动兜底：内置资源
    if (builtinRegistry != null) {
      return builtinRegistry;
    }

    return PluginRegistry.empty();
  }

  /// 从所有已配置源获取注册表并合并。
  ///
  /// 对每个源，同时请求主 URL 和备用 URL，合并结果保留所有历史版本。
  Future<PluginRegistry?> _fetchFromSources() async {
    final mergedPlugins = <String, PluginRegistryEntry>{};
    DateTime? latestUpdatedAt;
    bool anySuccess = false;

    for (final source in sources) {
      try {
        final registry = await _fetchWithFallback(source);
        if (registry != null) {
          anySuccess = true;
          // 合并插件：保留所有历史版本（取版本并集）
          for (final entry in registry.plugins.entries) {
            final pluginId = entry.key;
            final newEntry = entry.value;
            if (mergedPlugins.containsKey(pluginId)) {
              mergedPlugins[pluginId] = _mergePluginEntry(
                mergedPlugins[pluginId]!,
                newEntry,
              );
            } else {
              mergedPlugins[pluginId] = newEntry;
            }
          }
          if (latestUpdatedAt == null || registry.updatedAt.isAfter(latestUpdatedAt)) {
            latestUpdatedAt = registry.updatedAt;
          }
        }
      } on Exception catch (e) {
        // 单个源失败不阻断其他源
        _log('Source ${source.name} failed: $e');
      }
    }

    if (!anySuccess) return null;

    return PluginRegistry(
      version: '1',
      updatedAt: latestUpdatedAt ?? DateTime.now().toUtc(),
      plugins: mergedPlugins,
    );
  }

  /// 对一个源同时请求主 URL 和备用 URL，合并结果。
  Future<PluginRegistry?> _fetchWithFallback(PluginSource source) async {
    // 并行请求主 URL 和备用 URL
    final primaryFuture = _fetchSingleSource(source.registryUrl);
    final fallbackFuture = _fetchSingleSource(source.fallbackRegistryUrl);

    final results = await Future.wait([primaryFuture, fallbackFuture]);
    final primary = results[0];
    final fallback = results[1];

    if (primary == null && fallback == null) return null;
    if (primary == null) return fallback;
    if (fallback == null) return primary;

    // 两者都成功：合并结果，保留所有历史版本
    return _mergeRegistries(primary, fallback);
  }

  /// 合并两个注册表结果，保留所有历史版本。
  PluginRegistry _mergeRegistries(PluginRegistry a, PluginRegistry b) {
    final mergedPlugins = <String, PluginRegistryEntry>{};
    mergedPlugins.addAll(a.plugins);

    for (final entry in b.plugins.entries) {
      final pluginId = entry.key;
      final bEntry = entry.value;
      if (mergedPlugins.containsKey(pluginId)) {
        mergedPlugins[pluginId] = _mergePluginEntry(mergedPlugins[pluginId]!, bEntry);
      } else {
        mergedPlugins[pluginId] = bEntry;
      }
    }

    return PluginRegistry(
      version: '1',
      updatedAt: a.updatedAt.isAfter(b.updatedAt) ? a.updatedAt : b.updatedAt,
      plugins: mergedPlugins,
    );
  }

  /// 合并两个插件条目，保留所有历史版本（取版本并集，latest_version 取较新者）。
  PluginRegistryEntry _mergePluginEntry(PluginRegistryEntry a, PluginRegistryEntry b) {
    final mergedVersions = <String, PluginVersionInfo>{};
    mergedVersions.addAll(a.versions);
    mergedVersions.addAll(b.versions);

    // 合并 i18n：取并集，b 中额外的语言覆盖 a
    final mergedI18n = <String, Map<String, String>>{};
    if (a.i18n != null) {
      for (final entry in a.i18n!.entries) {
        mergedI18n[entry.key] = Map<String, String>.from(entry.value);
      }
    }
    if (b.i18n != null) {
      for (final entry in b.i18n!.entries) {
        mergedI18n[entry.key] = Map<String, String>.from(entry.value);
      }
    }

    // 取较新的 latest_version
    final latestVersion = _compareVersion(a.latestVersion, b.latestVersion) >= 0
        ? a.latestVersion
        : b.latestVersion;

    return PluginRegistryEntry(
      name: a.name.isNotEmpty ? a.name : b.name,
      publisher: a.publisher.isNotEmpty ? a.publisher : b.publisher,
      latestVersion: latestVersion,
      description: a.description?.isNotEmpty == true ? a.description : b.description,
      homepage: a.homepage?.isNotEmpty == true ? a.homepage : b.homepage,
      versions: mergedVersions,
      i18n: mergedI18n.isNotEmpty ? mergedI18n : null,
    );
  }

  /// 简单的语义化版本比较（x.y.z）。
  /// 返回 >0 表示 a 较新，<0 表示 b 较新，0 表示相等。
  int _compareVersion(String a, String b) {
    try {
      final aParts = a.split('.').map(int.parse).toList();
      final bParts = b.split('.').map(int.parse).toList();
      for (var i = 0; i < aParts.length && i < bParts.length; i++) {
        final diff = aParts[i] - bParts[i];
        if (diff != 0) return diff;
      }
      return aParts.length - bParts.length;
    } on Exception {
      return a.compareTo(b);
    }
  }

  /// 获取单个 URL 的注册表
  Future<PluginRegistry?> _fetchSingleSource(String url) async {
    try {
      final response = await http
          .get(Uri.parse(url))
          .timeout(const Duration(seconds: 15));
      if (response.statusCode == 200) {
        return PluginRegistry.fromJson(jsonDecode(response.body));
      }
    } on Exception catch (_) {
      // 忽略单个请求失败
    }
    return null;
  }

  Future<void> _saveLocalCache(String json) async {
    final cacheFile = File('${_pluginDir.path}/registry.json');
    await cacheFile.writeAsString(json);
  }

  Future<PluginRegistry?> _loadLocalCache() async {
    final cacheFile = File('${_pluginDir.path}/registry.json');
    if (!await cacheFile.exists()) return null;
    final json = await cacheFile.readAsString();
    return PluginRegistry.fromJson(jsonDecode(json));
  }

  /// 从应用内置资源加载默认注册表（首次启动兜底）
  Future<PluginRegistry?> _loadBuiltinRegistry() async {
    try {
      final jsonString = await rootBundle.loadString(_builtinAssetPath);
      final registry = PluginRegistry.fromJson(jsonDecode(jsonString));
      // 同时写入本地缓存，避免每次启动都读 asset
      await _saveLocalCache(jsonString);
      return registry;
    } on Exception catch (_) {
      return null;
    }
  }

  void _log(String message) {
    if (const bool.fromEnvironment('dart.vm.product')) return;
    SoloLog.d('PluginRegistryService', message);
  }
}
