import 'dart:convert';
import 'dart:io';

import 'package:crypto/crypto.dart';
import 'package:flutter/foundation.dart';
import 'package:version/version.dart';

import 'package:solosoul_flutter/core/models/plugin_models.dart';
import 'package:solosoul_flutter/frb/api.dart' as frb;


/// 插件安装器：负责从市场/本地安装、更新、卸载插件
///
/// ⚠️ 路径必须与 Rust 侧 PluginStore::base_dir() 完全一致。
class PluginInstallerService {
  late final Directory _pluginDir;

  Future<void> initialize() async {
    final baseDir = await frb.frbGetPluginBaseDir();
    _pluginDir = Directory(baseDir);
    await _pluginDir.create(recursive: true);
  }

  /// 从远程市场安装插件
  Future<void> installFromMarket(
    String pluginId,
    PluginRegistryEntry entry,
    String appVersion,
    String pluginApiVersion,
  ) async {
    final versionInfo = entry.versions[entry.latestVersion];
    if (versionInfo == null) {
      throw PluginSecurityException('Version info missing for $pluginId');
    }

    // 1. 版本兼容性检查
    if (!_isCompatible(versionInfo, appVersion, pluginApiVersion)) {
      throw PluginIncompatibleException(pluginId);
    }

    // 2. 下载 wasm + manifest 到临时目录
    final tempDir = await Directory.systemTemp.createTemp('solosoul_plugin_');
    try {
      final wasmBytes = await _download('${versionInfo.downloadUrl}/plugin.wasm');
      final manifestJsonBytes = await _download('${versionInfo.downloadUrl}/manifest.json');
      final manifestJson = utf8.decode(manifestJsonBytes);

      // 2.5 大小限制（防止恶意超大 wasm 导致 OOM）
      const maxWasmSize = 10 * 1024 * 1024; // 10MB
      if (wasmBytes.length > maxWasmSize) {
        throw PluginSecurityException('Wasm file exceeds 10MB limit');
      }

      // 3. SHA-256 校验
      final computedHash = sha256.convert(wasmBytes).toString();
      if (computedHash != versionInfo.sha256) {
        throw PluginSecurityException('Hash mismatch for $pluginId');
      }

      // 4. 保存到临时文件，然后调用 Rust FFI 安装
      final tempWasmPath = '${tempDir.path}/plugin.wasm';
      final tempManifestPath = '${tempDir.path}/manifest.json';
      await File(tempWasmPath).writeAsBytes(wasmBytes);
      await File(tempManifestPath).writeAsString(manifestJson);

      await frb.frbPluginInstall(
        wasmPath: tempWasmPath,
        manifestPath: tempManifestPath,
      );
    } finally {
      await tempDir.delete(recursive: true);
    }
  }

  /// 从本地文件安装（开发者模式 / 侧载）
  Future<void> installFromLocal(String wasmPath, String manifestPath) async {
    if (kReleaseMode) {
      throw PluginSecurityException('Sideloading is only allowed in debug mode');
    }
    await frb.frbPluginInstall(wasmPath: wasmPath, manifestPath: manifestPath);
  }

  /// 卸载插件（与主软件完全分离）
  Future<void> uninstall(String pluginId) async {
    // 1. 撤销所有活跃 Session（Rust 侧）
    await frb.frbPluginForceUnload(pluginId: pluginId);

    // 2. 更新 installed.json（标记为已卸载）
    await _updateInstalledIndex(pluginId, null, 'uninstalled');
  }

  /// 检查更新
  Future<List<PluginUpdateInfo>> checkForUpdates(PluginRegistry registry) async {
    final installed = await _loadInstalledIndex();
    final updates = <PluginUpdateInfo>[];

    for (final pluginId in installed.keys) {
      final info = installed[pluginId]!;
      if (info.status != 'installed') continue;

      final remoteEntry = registry.plugins[pluginId];
      if (remoteEntry != null && remoteEntry.latestVersion != info.version) {
        updates.add(PluginUpdateInfo(
          pluginId: pluginId,
          currentVersion: info.version,
          latestVersion: remoteEntry.latestVersion,
        ));
      }
    }
    return updates;
  }

  bool _isCompatible(
    PluginVersionInfo versionInfo,
    String appVersion,
    String pluginApiVersion,
  ) {
    // 1. plugin_api_version 必须完全匹配
    if (versionInfo.pluginApiVersion != pluginApiVersion) {
      return false;
    }
    // 2. appVersion 需在 [min, max] 范围内
    final app = Version.parse(appVersion);
    final min = Version.parse(versionInfo.minAppVersion);
    final max = Version.parse(versionInfo.maxAppVersion);
    return app >= min && app <= max;
  }

  Future<List<int>> _download(String url) async {
    final client = HttpClient();
    try {
      final request = await client.getUrl(Uri.parse(url));
      final response = await request.close();
      if (response.statusCode != 200) {
        throw PluginSecurityException('Download failed: HTTP ${response.statusCode}');
      }
      return response.expand((chunk) => chunk).toList();
    } finally {
      client.close();
    }
  }

  Future<void> _updateInstalledIndex(
    String pluginId,
    String? version,
    String status,
  ) async {
    final indexFile = File('${_pluginDir.path}/installed.json');
    Map<String, dynamic> index = {};
    if (await indexFile.exists()) {
      index = jsonDecode(await indexFile.readAsString()) as Map<String, dynamic>;
    }
    if (version == null) {
      index[pluginId] = {
        'status': status,
        'uninstalled_at': DateTime.now().toIso8601String(),
      };
    } else {
      index[pluginId] = {
        'version': version,
        'status': status,
        'installed_at': DateTime.now().toIso8601String(),
      };
    }
    await indexFile.writeAsString(jsonEncode(index));
  }

  Future<Map<String, InstalledPluginInfo>> _loadInstalledIndex() async {
    final indexFile = File('${_pluginDir.path}/installed.json');
    if (!await indexFile.exists()) return {};
    final json = jsonDecode(await indexFile.readAsString()) as Map<String, dynamic>;

    // 一致性校验：移除 index 中有但目录不存在的 orphan 记录
    final reconciled = <String, InstalledPluginInfo>{};
    for (final entry in json.entries) {
      final pluginDir = Directory('${_pluginDir.path}/${entry.key}');
      if (await pluginDir.exists() &&
          await File('${pluginDir.path}/manifest.json').exists() &&
          await File('${pluginDir.path}/plugin.wasm').exists()) {
        reconciled[entry.key] = InstalledPluginInfo.fromJson(entry.value as Map<String, dynamic>);
      }
    }
    // 反向校验：目录存在但 index 中没有的，自动重新索引
    for (final dir in _pluginDir.listSync().whereType<Directory>()) {
      final id = dir.path.split(Platform.pathSeparator).last;
      if (!id.startsWith('.') && !reconciled.containsKey(id)) {
        final manifestFile = File('${dir.path}/manifest.json');
        if (await manifestFile.exists()) {
          final manifest = jsonDecode(await manifestFile.readAsString());
          reconciled[id] = InstalledPluginInfo(
            version: manifest['version'] as String,
            status: 'installed',
            installedAt: DateTime.now(),
          );
        }
      }
    }
    return reconciled;
  }
}
