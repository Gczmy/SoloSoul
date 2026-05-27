import 'dart:convert';
import 'dart:io';

import 'package:crypto/crypto.dart';
import 'package:flutter/foundation.dart';
import 'package:version/version.dart';

import 'package:solosoul_flutter/core/models/plugin_models.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/frb/api.dart' as frb;


/// 插件安装器：负责从市场/本地安装、更新、卸载插件
///
/// 下载策略：优先使用 jsDelivr CDN（downloadUrl），失败时 fallback 到 GitHub Raw（rawUrl）。
///
/// ⚠️ 路径必须与 Rust 侧 PluginStore::base_dir() 完全一致。
class PluginInstallerService {
  late final Directory _pluginDir;

  Future<void> initialize() async {
    final baseDir = await frb.frbGetPluginBaseDir();
    _pluginDir = Directory(baseDir);
    await _pluginDir.create(recursive: true);
  }

  /// 从远程市场下载插件工件（wasm + manifest），返回原始字节和 JSON。
  /// [targetVersion] 为 null 时下载最新版本。
  Future<PluginArtifacts> downloadPluginArtifacts(
    String pluginId,
    PluginRegistryEntry entry,
    String appVersion,
    String pluginApiVersion, {
    String? targetVersion,
  }) async {
    final versionToInstall = targetVersion ?? entry.latestVersion;
    final versionInfo = entry.versions[versionToInstall];
    if (versionInfo == null) {
      throw PluginSecurityException(
        'Version $versionToInstall not found for $pluginId',
      );
    }

    // 1. 版本兼容性检查
    if (!_isCompatible(versionInfo, appVersion, pluginApiVersion)) {
      throw PluginIncompatibleException(pluginId);
    }

    // 2. 下载 wasm（带 CDN → Raw fallback）
    final wasmBytes = await _downloadWasm(versionInfo);

    // 3. 大小限制（防止恶意超大 wasm 导致 OOM）
    const maxWasmSize = 10 * 1024 * 1024; // 10MB
    if (wasmBytes.length > maxWasmSize) {
      throw PluginSecurityException('Wasm file exceeds 10MB limit');
    }

    // 4. SHA-256 校验
    final computedHash = sha256.convert(wasmBytes).toString();
    if (computedHash != versionInfo.sha256) {
      throw PluginSecurityException(
        'Hash mismatch for $pluginId: expected ${versionInfo.sha256}, got $computedHash',
      );
    }

    // 5. 下载 manifest.json（从 wasm URL 推断 manifest URL）
    final manifestJson = await _downloadManifest(versionInfo, versionToInstall);

    return PluginArtifacts(
      wasmBytes: wasmBytes,
      manifestJson: manifestJson,
      version: versionToInstall,
    );
  }

  /// 从已下载的工件安装插件。
  Future<void> installFromArtifacts(PluginArtifacts artifacts) async {
    final tempDir = await Directory.systemTemp.createTemp('solosoul_plugin_');
    try {
      final tempWasmPath = '${tempDir.path}/plugin.wasm';
      final tempManifestPath = '${tempDir.path}/manifest.json';
      await File(tempWasmPath).writeAsBytes(artifacts.wasmBytes);
      await File(tempManifestPath).writeAsString(artifacts.manifestJson);

      await frb.frbPluginInstall(
        wasmPath: tempWasmPath,
        manifestPath: tempManifestPath,
      );
    } finally {
      await tempDir.delete(recursive: true);
    }
  }

  /// 便捷方法：下载并直接安装（跳过审查流程）。
  Future<void> installFromMarket(
    String pluginId,
    PluginRegistryEntry entry,
    String appVersion,
    String pluginApiVersion, {
    String? targetVersion,
  }) async {
    final artifacts = await downloadPluginArtifacts(
      pluginId,
      entry,
      appVersion,
      pluginApiVersion,
      targetVersion: targetVersion,
    );
    await installFromArtifacts(artifacts);
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

    // 2. 删除插件目录（Rust 侧的 list_installed 直接扫描目录，必须物理删除）
    final pluginDir = Directory('${_pluginDir.path}/$pluginId');
    if (await pluginDir.exists()) {
      await pluginDir.delete(recursive: true);
    }

    // 3. 更新 installed.json（标记为已卸载）
    await _updateInstalledIndex(pluginId, null, 'uninstalled');
  }

  /// 记录插件最近使用时间
  Future<void> recordLastUsed(String pluginId) async {
    final index = await _loadInstalledIndex();
    final info = index[pluginId];
    if (info == null || info.status != 'installed') return;
    index[pluginId] = InstalledPluginInfo(
      version: info.version,
      status: info.status,
      installedAt: info.installedAt,
      uninstalledAt: info.uninstalledAt,
      lastUsedAt: DateTime.now(),
    );
    await _saveInstalledIndex(index);
  }

  /// 获取已安装插件的详细信息
  Future<InstalledPluginInfo?> getInstalledInfo(String pluginId) async {
    final index = await _loadInstalledIndex();
    return index[pluginId];
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

  /// 下载 wasm，优先 CDN，fallback 到 Raw。
  /// CDN 返回缓存旧版本时（hash 不匹配），自动 fallback 到 Raw。
  Future<List<int>> _downloadWasm(PluginVersionInfo info) async {
    final expectedHash = info.sha256;

    // 优先尝试 downloadUrl（jsDelivr CDN）
    try {
      final bytes = await _download(info.downloadUrl);
      final hash = sha256.convert(bytes).toString();
      if (hash == expectedHash) {
        return bytes;
      }
      _log('CDN returned stale wasm (hash mismatch): expected $expectedHash, got $hash');
    } on Exception catch (e) {
      _log('CDN download failed (${info.downloadUrl}): $e');
    }

    // fallback 到 rawUrl（GitHub Raw，无 CDN 缓存问题）
    final rawUrl = info.rawUrl;
    if (rawUrl != null && rawUrl != info.downloadUrl) {
      try {
        final bytes = await _download(rawUrl);
        final hash = sha256.convert(bytes).toString();
        if (hash == expectedHash) {
          return bytes;
        }
        _log('Raw returned stale wasm (hash mismatch): expected $expectedHash, got $hash');
      } on Exception catch (e) {
        _log('Raw download failed ($rawUrl): $e');
      }
    }

    throw PluginSecurityException(
      'Failed to download wasm from all available URLs (hash mismatch — CDN cache stale?)',
    );
  }

  /// 下载 manifest.json（从 wasm URL 推断 manifest 路径）。
  /// 支持 CDN → Raw fallback，并校验 manifest 中的 version 字段。
  Future<String> _downloadManifest(PluginVersionInfo info, String expectedVersion) async {
    String? manifestUrl;

    // 尝试从 downloadUrl 推断 manifest URL（jsDelivr CDN）
    try {
      manifestUrl = _inferManifestUrl(info.downloadUrl);
      final bytes = await _download(manifestUrl);
      final jsonStr = utf8.decode(bytes);
      final parsed = jsonDecode(jsonStr) as Map<String, dynamic>;
      final version = parsed['version'] as String?;
      if (version == expectedVersion) {
        return jsonStr;
      }
      _log('Manifest CDN returned stale version: expected $expectedVersion, got $version');
    } on Exception catch (e) {
      _log('Manifest CDN download failed ($manifestUrl): $e');
    }

    // fallback 到 rawUrl（GitHub Raw，无 CDN 缓存问题）
    final rawUrl = info.rawUrl;
    if (rawUrl != null) {
      try {
        manifestUrl = _inferManifestUrl(rawUrl);
        final bytes = await _download(manifestUrl);
        final jsonStr = utf8.decode(bytes);
        final parsed = jsonDecode(jsonStr) as Map<String, dynamic>;
        final version = parsed['version'] as String?;
        if (version == expectedVersion) {
          return jsonStr;
        }
        _log('Manifest Raw returned stale version: expected $expectedVersion, got $version');
      } on Exception catch (e) {
        _log('Manifest raw download failed ($manifestUrl): $e');
      }
    }

    throw PluginSecurityException(
      'Failed to download manifest.json from all available URLs (CDN cache stale?)',
    );
  }

  /// 从 wasm URL 推断 manifest.json URL
  /// 例如：.../plugin.wasm → .../manifest.json
  String _inferManifestUrl(String wasmUrl) {
    final uri = Uri.parse(wasmUrl);
    final pathSegments = List<String>.from(uri.pathSegments);
    if (pathSegments.isNotEmpty && pathSegments.last == 'plugin.wasm') {
      pathSegments.last = 'manifest.json';
    }
    return uri.replace(pathSegments: pathSegments).toString();
  }

  Future<List<int>> _download(String url) async {
    final client = HttpClient();
    try {
      final request = await client.getUrl(Uri.parse(url));
      final response = await request.close();
      if (response.statusCode != 200) {
        throw PluginSecurityException('Download failed: HTTP ${response.statusCode} for $url');
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
    final index = await _loadInstalledIndex();
    if (version == null) {
      index[pluginId] = InstalledPluginInfo(
        version: '',
        status: status,
        uninstalledAt: DateTime.now(),
      );
    } else {
      index[pluginId] = InstalledPluginInfo(
        version: version,
        status: status,
        installedAt: DateTime.now(),
      );
    }
    await _saveInstalledIndex(index);
  }

  Future<void> _saveInstalledIndex(
    Map<String, InstalledPluginInfo> index,
  ) async {
    final indexFile = File('${_pluginDir.path}/installed.json');
    final map = <String, dynamic>{};
    for (final entry in index.entries) {
      map[entry.key] = entry.value.toJson();
    }
    await indexFile.writeAsString(jsonEncode(map));
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

  void _log(String message) {
    if (kReleaseMode) return;
    SoloLog.d('PluginInstallerService', message);
  }
}
