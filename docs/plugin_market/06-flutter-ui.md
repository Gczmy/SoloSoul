## 7. Flutter 侧实现

### 7.1 授权弹窗（lib/presentation/widgets/plugin_consent_dialog.dart）

```dart
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';

class PluginConsentDialog extends ConsumerWidget {
  final String pluginId;
  final String pluginName;
  final String fieldId;
  final String sessionId;
  final SensitivityLevel sensitivity;

  const PluginConsentDialog({
    super.key,
    required this.pluginId,
    required this.pluginName,
    required this.fieldId,
    required this.sessionId,
    required this.sensitivity,
  });

  // TODO: i18n — 使用 AppLocalizations.of(context) 替换硬编码中文
  static final Map<String, String> _fieldNameMap = {
    'identity.full_name': '真实姓名',
    'identity.id_card.number': '身份证号码',
    'travel.primary_passport.number': '护照号码',
    'identity.contact.emails': '电子邮箱',
    'identity.contact.phones': '手机号码',
  };

  Color _getSensitivityColor() {
    return switch (sensitivity) {
      SensitivityLevel.public => Colors.green,
      SensitivityLevel.internal => Colors.blue,
      SensitivityLevel.sensitive => Colors.orange,
      SensitivityLevel.critical => Colors.red,
    };
  }

  String _getSensitivityLabel() {
    return switch (sensitivity) {
      SensitivityLevel.public => '公开',
      SensitivityLevel.internal => '内部',
      SensitivityLevel.sensitive => '敏感',
      SensitivityLevel.critical => '关键',
    };
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final fieldDisplayName = _fieldNameMap[fieldId] ?? fieldId;

    return AlertDialog(
      title: Row(
        children: [
          Icon(Icons.security, color: Theme.of(context).colorScheme.primary),
          const SizedBox(width: 8),
          const Text('插件请求数据授权'), // TODO: i18n
        ],
      ),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('插件 "$pluginName" 请求访问以下数据：'), // TODO: i18n
          const SizedBox(height: 16),
          Container(
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: _getSensitivityColor().withOpacity(0.1),
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: _getSensitivityColor()),
            ),
            child: Row(
              children: [
                Icon(Icons.warning_amber, color: _getSensitivityColor()),
                const SizedBox(width: 8),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        fieldDisplayName,
                        style: const TextStyle(fontWeight: FontWeight.bold),
                      ),
                      Text(
                        '敏感度: ${_getSensitivityLabel()}',
                        style: Theme.of(context).textTheme.bodySmall,
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 12),
          Text(
            '数据仅在本次会话中可用，到期后自动销毁。', // TODO: i18n
            style: Theme.of(context).textTheme.bodySmall,
          ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: const Text('拒绝'), // TODO: i18n
        ),
        ElevatedButton(
          onPressed: () => Navigator.of(context).pop(true),
          style: ElevatedButton.styleFrom(
            backgroundColor: Theme.of(context).colorScheme.primary,
          ),
          child: const Text('授权访问'), // TODO: i18n
        ),
      ],
    );
  }
}
```

### 7.2 Plugin Service 与 FRB 接口

**Rust FFI 接口定义**（`flutter/native/src/plugin/api.rs`，新增）：

```rust
use flutter_rust_bridge::StreamSink;

/// 插件事件流（Rust -> Dart）
#[derive(Debug, Clone)]
pub enum PluginEvent {
    ConsentRequest {
        request_id: String,
        plugin_id: String,
        plugin_name: String,
        field: String,
        sensitivity: String,
    },
    Log {
        level: String,
        message: String,
    },
    Progress {
        percent: u8,
    },
    ConsentTimeout {
        request_id: String,
    },
    Completed {
        exit_code: i32,
    },
    Error {
        message: String,
    },
}

/// 安装插件（Rust 侧直接读取文件，避免 Dart 内存拷贝）
pub fn plugin_install(wasm_path: String, manifest_path: String) -> Result<String, String> {
    Ok(String::new())
}

/// 执行插件（核心方法，返回事件流）
pub fn plugin_execute(
    plugin_id: String,
    session_ttl_seconds: u64,
    sink: StreamSink<PluginEvent>,
) -> Result<(), String> {
    Ok(())
}

/// 响应用户授权（Dart -> Rust）
pub fn plugin_consent_response(request_id: String, approved: bool, value: Option<String>) {
}

/// 撤销会话
pub fn plugin_revoke_session(session_id: String) -> Result<(), String> {
    Ok(())
}

/// 强制卸载插件（清理 Store + 内存）
pub fn plugin_force_unload(plugin_id: String) -> Result<(), String> {
    Ok(())
}
```

### 7.3 Dart Service 封装

**PluginRegistryService**（`lib/core/services/plugin_registry_service.dart`）：

```dart
import 'dart:convert';
import 'dart:io';
import 'package:http/http.dart' as http;
import 'package:path_provider/path_provider.dart';

/// 插件注册表服务：管理远程 registry 的拉取与本地缓存
class PluginRegistryService {
  static const _remoteUrl = 'https://plugins.solosoul.dev/registry.json';
  static const _cacheTtl = Duration(hours: 24);

  late final Directory _pluginDir;

  Future<void> initialize() async {
    // ⚠️ 路径必须与 Rust 侧 PluginStore::base_dir() 完全一致。
    // 建议通过 FFI 调用 Rust 的 get_plugin_dir() 获取统一路径。
    final baseDir = await _getUnifiedPluginDir();
    _pluginDir = Directory('$baseDir');
    await _pluginDir.create(recursive: true);
  }

  Future<String> _getUnifiedPluginDir() async {
    // 方式1（推荐）：通过 FFI 调用 Rust 侧的 PluginStore::base_dir()
    // return await rustApi.getPluginBaseDir();
    // 方式2（fallback）：手动拼接，需与 Rust 侧保持一致
    // return '${Platform.environment['HOME']}/.solosoul/plugins';
    throw UnimplementedError('必须通过 FFI 获取 Rust 侧统一路径');
  }

  /// 获取合并后的注册表（远程优先，离线回退本地缓存）
  Future<PluginRegistry> getRegistry() async {
    // 优先使用本地缓存（若未过期）
    final cached = await _loadLocalCache();
    if (cached != null && await _isCacheFresh()) {
      return cached;
    }

    try {
      final response = await http.get(Uri.parse(_remoteUrl))
          .timeout(const Duration(seconds: 10));
      if (response.statusCode == 200) {
        final registry = PluginRegistry.fromJson(jsonDecode(response.body));
        await _saveLocalCache(response.body);
        return registry;
      }
    } catch (e) {
      // 离线或网络错误，回退本地缓存（即使已过期）
    }
    return cached ?? PluginRegistry.empty();
  }

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
    if (!await cacheFile.exists()) {
      return null;
    }
    final json = await cacheFile.readAsString();
    return PluginRegistry.fromJson(jsonDecode(json));
  }
}
```

**PluginInstallerService**（`lib/core/services/plugin_installer_service.dart`）：

```dart
import 'dart:convert';
import 'dart:io';
import 'package:crypto/crypto.dart';
import 'package:flutter/foundation.dart';
import 'package:path_provider/path_provider.dart';
import 'package:version/version.dart';

/// 插件安装器：负责从市场/本地安装、更新、卸载插件
class PluginInstallerService {
  late final Directory _pluginDir;

  Future<void> initialize() async {
    final appDocDir = await getApplicationDocumentsDirectory();
    _pluginDir = Directory('${appDocDir.path}/plugins');
    await _pluginDir.create(recursive: true);
  }

  /// 从远程市场安装插件
  Future<void> installFromMarket(
    String pluginId,
    PluginRegistryEntry entry,
    String appVersion,
    String pluginApiVersion,
  ) async {
    // 1. 版本兼容性检查
    if (!_isCompatible(entry, appVersion, pluginApiVersion)) {
      throw PluginIncompatibleException(pluginId);
    }

    // 2. 下载 wasm + manifest 到临时目录
    final tempDir = await Directory.systemTemp.createTemp('solosoul_plugin_');
    try {
      final wasmBytes = await _download('${entry.downloadUrl}/plugin.wasm');
      final manifestJson = await _download('${entry.downloadUrl}/manifest.json');

      // 2.5 大小限制（防止恶意超大 wasm 导致 OOM）
      const maxWasmSize = 10 * 1024 * 1024; // 10MB
      if (wasmBytes.length > maxWasmSize) {
        throw PluginSecurityException('Wasm file exceeds 10MB limit');
      }

      // 3. SHA-256 校验
      final computedHash = sha256.convert(wasmBytes).toString();
      if (computedHash != entry.sha256) {
        throw PluginSecurityException('Hash mismatch for $pluginId');
      }

      // 4. 安装到独立目录
      final installDir = Directory('${_pluginDir.path}/$pluginId');
      await installDir.create(recursive: true);
      await File('${installDir.path}/plugin.wasm').writeAsBytes(wasmBytes);
      await File('${installDir.path}/manifest.json').writeAsString(manifestJson);

      // 5. 更新 installed.json
      await _updateInstalledIndex(pluginId, entry.version, 'installed');
    } finally {
      await tempDir.delete(recursive: true);
    }
  }

  /// 从本地文件安装（开发者模式 / 侧载）
  Future<void> installFromLocal(String wasmPath, String manifestPath) async {
    if (kReleaseMode) {
      throw PluginSecurityException(
        'Sideloading is only allowed in debug mode',
      );
    }

    final wasmBytes = await File(wasmPath).readAsBytes();
    final manifestJson = await File(manifestPath).readAsString();
    final manifest = PluginManifest.fromJson(jsonDecode(manifestJson));

    // Debug 模式跳过 SHA-256 白名单校验，但仍解析 manifest
    final installDir = Directory('${_pluginDir.path}/${manifest.pluginId}');
    await installDir.create(recursive: true);
    await File('${installDir.path}/plugin.wasm').writeAsBytes(wasmBytes);
    await File('${installDir.path}/manifest.json').writeAsString(manifestJson);

    await _updateInstalledIndex(manifest.pluginId, manifest.version, 'installed');
  }

  /// 卸载插件（与主软件完全分离）
  Future<void> uninstall(String pluginId) async {
    // 1. 撤销所有活跃 Session（Rust 侧）
    await rustApi.pluginForceUnload(pluginId);

    // 2. 删除插件目录（wasm + manifest + config + cache）
    final pluginDir = Directory('${_pluginDir.path}/$pluginId');
    if (await pluginDir.exists()) {
      await pluginDir.delete(recursive: true);
    }

    // 3. 更新 installed.json（标记为已卸载）
    await _updateInstalledIndex(pluginId, null, 'uninstalled');
  }

  /// 检查更新
  Future<List<PluginUpdateInfo>> checkForUpdates(PluginRegistry registry) async {
    final installed = await _loadInstalledIndex();
    final updates = <PluginUpdateInfo>[];

    for (final pluginId in installed.keys) {
      final localVersion = installed[pluginId]!.version;
      final remoteEntry = registry.plugins[pluginId];
      if (remoteEntry != null && remoteEntry.latestVersion != localVersion) {
        updates.add(PluginUpdateInfo(
          pluginId: pluginId,
          currentVersion: localVersion,
          latestVersion: remoteEntry.latestVersion,
        ));
      }
    }
    return updates;
  }

  bool _isCompatible(
    PluginRegistryEntry entry,
    String appVersion,
    String pluginApiVersion,
  ) {
    // 1. plugin_api_version 必须完全匹配
    if (entry.pluginApiVersion != pluginApiVersion) {
      return false;
    }
    // 2. appVersion 需在 [min, max] 范围内
    final app = Version.parse(appVersion);
    final min = Version.parse(entry.minAppVersion);
    final max = Version.parse(entry.maxAppVersion);
    return app >= min && app <= max;
  }

  Future<void> _updateInstalledIndex(
    String pluginId,
    String? version,
    String status,
  ) async {
    final indexFile = File('${_pluginDir.path}/installed.json');
    Map<String, dynamic> index = {};
    if (await indexFile.exists()) {
      index = jsonDecode(await indexFile.readAsString());
    }
    if (version == null) {
      index[pluginId] = {'status': status, 'uninstalled_at': DateTime.now().toIso8601String()};
    } else {
      index[pluginId] = {'version': version, 'status': status, 'installed_at': DateTime.now().toIso8601String()};
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
        reconciled[entry.key] = InstalledPluginInfo.fromJson(entry.value);
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
```

**PluginService**（`lib/core/services/plugin_service.dart`）：

```dart
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path_provider/path_provider.dart';

/// 插件服务：管理插件加载、白名单校验、沙盒执行
class PluginService {
  late final Directory _pluginDir;

  Future<void> initialize() async {
    // ⚠️ 必须与 PluginRegistryService / PluginInstallerService 使用同一套路径获取逻辑
    final baseDir = await _getUnifiedPluginDir();
    _pluginDir = Directory(baseDir);
  }

  Future<String> _getUnifiedPluginDir() async {
    // 方式1（推荐）：通过 FFI 调用 Rust 侧的 PluginStore::base_dir()
    // return await rustApi.getPluginBaseDir();
    throw UnimplementedError('必须通过 FFI 获取 Rust 侧统一路径');
  }

  /// 从独立目录加载已安装插件清单
  Future<List<PluginManifest>> loadInstalledPlugins() async {
    final plugins = <PluginManifest>[];
    if (!await _pluginDir.exists()) return plugins;

    for (final entry in _pluginDir.listSync().whereType<Directory>()) {
      final manifestFile = File('${entry.path}/manifest.json');
      if (await manifestFile.exists()) {
        final json = jsonDecode(await manifestFile.readAsString());
        plugins.add(PluginManifest.fromJson(json));
      }
    }
    return plugins;
  }

  /// 运行插件（核心方法）
  Future<PluginResult> runPlugin(
    String pluginId, {
    Map<String, dynamic>? params,
  }) async {
    // 1. 校验插件目录存在
    final pluginDir = Directory('${_pluginDir.path}/$pluginId');
    if (!await pluginDir.exists()) {
      throw PluginNotFoundException(pluginId);
    }

    // 2. 通过 Rust FFI 加载 manifest（避免 Dart 重复解析）
    final manifest = await rustApi.pluginLoadManifest(pluginId);

    // 3. 通过 flutter_rust_bridge 调用 Rust 执行
    //    Rust 侧直接从文件系统读取 wasm，Dart 不持有 wasmBytes
    final stream = await rustApi.pluginExecute(
      pluginId: pluginId,
      sessionTtlSeconds: manifest.dataTtlSeconds,
    );

    // 4. 监听 PluginEvent Stream，弹出授权对话框 / 处理进度 / 超时
    await for (final event in stream) {
      switch (event) {
        case PluginEventConsentRequest(:final requestId, :final field, :final sensitivity):
          final approved = await showConsentDialog(field, sensitivity);
          await rustApi.pluginConsentResponse(
            requestId: requestId,
            approved: approved,
            value: approved ? await _resolveFieldValue(field) : null,
          );
        case PluginEventConsentTimeout(:final requestId):
          // Rust 侧超时，关闭弹窗（如果还开着）
          _closeConsentDialogIfOpen(requestId);
        case PluginEventCompleted(:final exitCode):
          return PluginResult(exitCode: exitCode);
        case PluginEventError(:final message):
          throw PluginExecutionException(message);
        default:
          break;
      }
    }

    throw PluginExecutionException('Plugin stream ended unexpectedly');
  }
}

// 统一的 PluginManifest 模型，提取到 lib/core/models/plugin_manifest.dart
// 所有 Service 共用此模型，禁止重复定义
class PluginManifest {
  final String pluginId;
  final String name;
  final String version;
  final String pluginApiVersion;
  final String minAppVersion;
  final String maxAppVersion;
  final List<String> requiredFields;
  final List<String> optionalFields;
  final int dataTtlSeconds;
  final NetworkPolicy? networkPolicy; // 新增：网络策略

  PluginManifest({
    required this.pluginId,
    required this.name,
    required this.version,
    required this.pluginApiVersion,
    required this.minAppVersion,
    required this.maxAppVersion,
    required this.requiredFields,
    required this.optionalFields,
    required this.dataTtlSeconds,
    this.networkPolicy,
  });

  factory PluginManifest.fromJson(Map<String, dynamic> json) {
    return PluginManifest(
      pluginId: json['plugin_id'] as String,
      name: json['name'] as String,
      version: json['version'] as String,
      pluginApiVersion: json['plugin_api_version'] as String? ?? '1.0',
      minAppVersion: json['min_app_version'] as String? ?? '1.0.0',
      maxAppVersion: json['max_app_version'] as String? ?? '999.999.999',
      requiredFields: List<String>.from(json['required_fields'] ?? []),
      optionalFields: List<String>.from(json['optional_fields'] ?? []),
      dataTtlSeconds: json['data_ttl_seconds'] as int? ?? 300,
      networkPolicy: json['network_policy'] != null
          ? NetworkPolicy.fromJson(json['network_policy'])
          : null,
    );
  }
}

class NetworkPolicy {
  final bool blockAllOutbound;
  final List<String> allowedDomains;

  NetworkPolicy({required this.blockAllOutbound, required this.allowedDomains});

  factory NetworkPolicy.fromJson(Map<String, dynamic> json) {
    return NetworkPolicy(
      blockAllOutbound: json['block_all_outbound'] as bool? ?? true,
      allowedDomains: List<String>.from(json['allowed_domains'] ?? []),
    );
  }
}

class PluginResult {
  final int exitCode;
  final String? output;

  PluginResult({required this.exitCode, this.output});
}

class PluginNotFoundException implements Exception {
  final String pluginId;
  PluginNotFoundException(this.pluginId);
}

final pluginServiceProvider = Provider<PluginService>((ref) => PluginService());
```
