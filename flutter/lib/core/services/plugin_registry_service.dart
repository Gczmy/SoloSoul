import 'dart:convert';
import 'dart:io';

import 'package:flutter/services.dart';
import 'package:http/http.dart' as http;

import 'package:solosoul_flutter/core/models/plugin_models.dart';
import 'package:solosoul_flutter/frb/api.dart' as frb;

/// 插件注册表服务：管理远程 registry 的拉取与本地缓存
///
/// ⚠️ 路径必须与 Rust 侧 PluginStore::base_dir() 完全一致。
class PluginRegistryService {
  static const _remoteUrl = 'https://plugins.solosoul.dev/registry.json';
  static const _cacheTtl = Duration(hours: 24);
  static const _builtinAssetPath = 'assets/registry.json';

  late final Directory _pluginDir;

  Future<void> initialize() async {
    final baseDir = await frb.frbGetPluginBaseDir();
    _pluginDir = Directory(baseDir);
    await _pluginDir.create(recursive: true);
  }

  /// 获取合并后的注册表（远程优先，离线回退本地缓存，首次启动回退内置资源）
  Future<PluginRegistry> getRegistry() async {
    // 优先使用本地缓存（若未过期）
    final cached = await _loadLocalCache();
    if (cached != null && await _isCacheFresh()) {
      return cached;
    }

    try {
      final response = await http
          .get(Uri.parse(_remoteUrl))
          .timeout(const Duration(seconds: 10));
      if (response.statusCode == 200) {
        final registry = PluginRegistry.fromJson(jsonDecode(response.body));
        await _saveLocalCache(response.body);
        return registry;
      }
    } on Exception catch (_) {
      // 离线或网络错误，回退本地缓存（即使已过期）
    }

    // 回退顺序：本地缓存（即使过期）→ 内置资源 → 空 registry
    return cached ?? await _loadBuiltinRegistry() ?? PluginRegistry.empty();
  }

  /// 检查本地缓存是否在 24h 有效期内
  Future<bool> _isCacheFresh() async {
    final cacheFile = File('${_pluginDir.path}/registry.json');
    if (!await cacheFile.exists()) return false;
    final modified = await cacheFile.lastModified();
    return DateTime.now().difference(modified) < _cacheTtl;
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
}
