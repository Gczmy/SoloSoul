import 'dart:convert';
import 'dart:io';

import 'package:flutter/services.dart';
import 'package:http/http.dart' as http;

import 'package:solosoul_flutter/core/models/plugin_models.dart';
import 'package:solosoul_flutter/frb/api.dart' as frb;

/// 插件注册表服务：管理远程 registry 的拉取与本地缓存
///
/// 支持多插件源配置，每个源对应一个 GitHub 公开仓库。
/// 默认使用官方市场（jsDelivr CDN 加速）。
///
/// ⚠️ 路径必须与 Rust 侧 PluginStore::base_dir() 完全一致。
class PluginRegistryService {
  static const _builtinAssetPath = 'assets/registry.json';

  /// 已配置的插件源列表，默认包含官方源
  final List<PluginSource> sources;

  late final Directory _pluginDir;

  PluginRegistryService({
    List<PluginSource>? sources,
  }) : sources = sources ?? const [PluginSource.official];

  Future<void> initialize() async {
    final baseDir = await frb.frbGetPluginBaseDir();
    _pluginDir = Directory(baseDir);
    await _pluginDir.create(recursive: true);
  }

  /// 获取合并后的注册表（所有远程源优先，离线回退本地缓存，首次启动回退内置资源）
  ///
  /// 多个源的插件按 ID 合并，同名插件后覆盖前（用户自定义源优先级更高）。
  Future<PluginRegistry> getRegistry() async {
    // 1. 尝试从远程源获取并合并
    final remoteRegistry = await _fetchFromSources();
    if (remoteRegistry != null) {
      await _saveLocalCache(jsonEncode(remoteRegistry.toJson()));
      return remoteRegistry;
    }

    // 2. 离线回退：本地缓存（即使过期）
    final cached = await _loadLocalCache();
    if (cached != null) {
      return cached;
    }

    // 3. 首次启动兜底：内置资源
    final builtin = await _loadBuiltinRegistry();
    if (builtin != null) {
      return builtin;
    }

    return PluginRegistry.empty();
  }

  /// 从所有已配置源获取注册表并合并
  Future<PluginRegistry?> _fetchFromSources() async {
    final mergedPlugins = <String, PluginRegistryEntry>{};
    DateTime? latestUpdatedAt;
    bool anySuccess = false;

    for (final source in sources) {
      try {
        final registry = await _fetchSingleSource(source.registryUrl);
        if (registry != null) {
          anySuccess = true;
          mergedPlugins.addAll(registry.plugins);
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

  /// 获取单个源的注册表
  Future<PluginRegistry?> _fetchSingleSource(String url) async {
    final response = await http
        .get(Uri.parse(url))
        .timeout(const Duration(seconds: 15));
    if (response.statusCode == 200) {
      return PluginRegistry.fromJson(jsonDecode(response.body));
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
    // ignore: avoid_print
    if (const bool.fromEnvironment('dart.vm.product')) return;
    // ignore: avoid_print
    print('[PluginRegistryService] $message');
  }
}
