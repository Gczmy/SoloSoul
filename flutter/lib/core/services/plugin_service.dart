import 'dart:io';

import 'package:solosoul_flutter/core/models/plugin_models.dart';
import 'package:solosoul_flutter/frb/api.dart' as frb;
import 'package:solosoul_flutter/frb/plugin/manifest.dart' as frb_manifest;

/// 插件服务：管理插件加载、运行、沙盒执行
///
/// ⚠️ 路径必须与 Rust 侧 PluginStore::base_dir() 完全一致。
class PluginService {
  late final Directory _pluginDir;

  Future<void> initialize() async {
    final baseDir = await frb.frbGetPluginBaseDir();
    _pluginDir = Directory(baseDir);
  }

  /// 从独立目录加载已安装插件清单
  Future<List<frb_manifest.PluginManifest>> loadInstalledPlugins() async {
    final pluginIds = await frb.frbPluginListInstalled();
    final manifests = <frb_manifest.PluginManifest>[];
    for (final id in pluginIds) {
      try {
        final manifest = await frb.frbPluginLoadManifest(pluginId: id);
        manifests.add(manifest);
      } on Exception catch (_) {
        // 跳过损坏的插件
        continue;
      }
    }
    return manifests;
  }

  /// 运行插件（核心方法）
  ///
  /// 当前为简化实现：直接调用 Rust FFI 执行，不通过 Stream 处理事件。
  /// TODO: 启用 FRB Stream 后，改为监听 PluginEvent Stream，处理 ConsentRequest。
  Future<PluginRunResult> runPlugin(
    String pluginId, {
    Map<String, dynamic>? params,
  }) async {
    // 1. 校验插件目录存在
    final pluginDir = Directory('${_pluginDir.path}/$pluginId');
    if (!await pluginDir.exists()) {
      throw PluginNotFoundException(pluginId);
    }

    // 2. 通过 Rust FFI 加载 manifest（避免 Dart 重复解析）
    final manifest = await frb.frbPluginLoadManifest(pluginId: pluginId);

    // 3. 通过 Rust FFI 执行插件
    final exitCode = await frb.frbPluginExecute(
      pluginId: pluginId,
      sessionTtlSeconds: manifest.dataTtlSeconds,
    );

    return PluginRunResult(exitCode: exitCode);
  }

  /// 列出活跃 Session
  Future<List<frb.PluginSessionInfo>> listActiveSessions() async {
    return await frb.frbPluginListActiveSessions();
  }

  /// 强制卸载插件
  Future<void> forceUnload(String pluginId) async {
    await frb.frbPluginForceUnload(pluginId: pluginId);
  }
}
