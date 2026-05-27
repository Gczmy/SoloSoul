import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:version/version.dart';

import 'package:solosoul_flutter/core/models/plugin_models.dart';
import 'package:solosoul_flutter/core/services/plugin_installer_service.dart';
import 'package:solosoul_flutter/core/services/plugin_registry_service.dart';
import 'package:solosoul_flutter/core/services/plugin_service.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/frb/api.dart' as frb;
import 'package:solosoul_flutter/frb/plugin/manager.dart' as frb_plugin;
import 'package:solosoul_flutter/frb/plugin/manifest.dart' as frb_manifest;

/// 跟踪每个插件的安装/更新中状态（pluginId -> isLoading）
class PluginInstallingNotifier extends Notifier<Map<String, bool>> {
  @override
  Map<String, bool> build() => const {};

  void setLoading(String pluginId, bool loading) {
    state = {...state, pluginId: loading};
  }

  void clear(String pluginId) {
    state = {...state}..remove(pluginId);
  }
}

final pluginInstallingProvider = NotifierProvider<PluginInstallingNotifier, Map<String, bool>>(() {
  return PluginInstallingNotifier();
});

// ============================================================================
// Service Providers
// ============================================================================

final pluginServiceProvider = Provider<PluginService>((ref) {
  return PluginService();
});

final pluginInstallerProvider = Provider<PluginInstallerService>((ref) {
  return PluginInstallerService();
});

final pluginRegistryProvider = Provider<PluginRegistryService>((ref) {
  try {
    return PluginRegistryService();
  } catch (e, stack) {
    SoloLog.e('PluginRegistry', 'ERROR creating PluginRegistryService: $e\n$stack');
    rethrow;
  }
});

// ============================================================================
// Initialized Service Provider (auto-init on first use)
// ============================================================================

final _initializedPluginServiceProvider = FutureProvider<PluginService>((ref) async {
  final service = ref.read(pluginServiceProvider);
  await service.initialize();
  return service;
});

final initializedPluginInstallerProvider = FutureProvider<PluginInstallerService>((ref) async {
  final service = ref.read(pluginInstallerProvider);
  await service.initialize();
  return service;
});

final _initializedPluginRegistryProvider = FutureProvider<PluginRegistryService>((ref) async {
  try {
    final service = ref.read(pluginRegistryProvider);
    await service.initialize();
    return service;
  } catch (e, stack) {
    SoloLog.e('PluginRegistry', 'ERROR initializing PluginRegistryService: $e\n$stack');
    rethrow;
  }
});

// ============================================================================
// Plugin Registry State
// ============================================================================

final pluginRegistryStateProvider = FutureProvider<PluginRegistry>((ref) async {
  try {
    final registryService = await ref.watch(_initializedPluginRegistryProvider.future);
    return await registryService.getRegistry();
  } catch (e, stack) {
    SoloLog.e('PluginRegistry', 'ERROR getting registry: $e\n$stack');
    rethrow;
  }
});

// ============================================================================
// Installed Plugins State
// ============================================================================

final installedPluginsProvider = FutureProvider<List<frb_manifest.PluginManifest>>((ref) async {
  try {
    final service = await ref.watch(_initializedPluginServiceProvider.future);
    return await service.loadInstalledPlugins();
  } catch (e, stack) {
    SoloLog.e('Plugin', 'ERROR loading installed plugins: $e\n$stack');
    rethrow;
  }
});

// ============================================================================
// Active Sessions State
// ============================================================================

final activeSessionsProvider = FutureProvider<List<frb.PluginSessionInfo>>((ref) async {
  try {
    final service = await ref.watch(_initializedPluginServiceProvider.future);
    return await service.listActiveSessions();
  } catch (e, stack) {
    SoloLog.e('Plugin', 'ERROR listing active sessions: $e\n$stack');
    rethrow;
  }
});



// ============================================================================
// Plugin Dashboard Combined State (AsyncNotifier — 支持局部更新，避免全页刷新)
// ============================================================================

class PluginDashboardNotifier extends AsyncNotifier<PluginDashboardData> {
  @override
  Future<PluginDashboardData> build() async {
    // 使用 ref.read 避免底层 provider 变化时自动 rebuild，
    // 安装/卸载通过 add/removeInstalledPlugin 局部更新，保持 UI 不闪动。
    final registry = await ref.read(pluginRegistryStateProvider.future);
    final installed = await ref.read(installedPluginsProvider.future);
    final activeSessions = await ref.read(activeSessionsProvider.future);

    return PluginDashboardData(
      registry: registry,
      installed: installed,
      activeSessions: activeSessions,
    );
  }

  /// 全量刷新 — 用户手动点击刷新按钮时使用
  Future<void> refresh() async {
    state = const AsyncLoading();
    state = await AsyncValue.guard(() async {
      final registry = await ref.read(pluginRegistryStateProvider.future);
      final installed = await ref.read(installedPluginsProvider.future);
      final activeSessions = await ref.read(activeSessionsProvider.future);
      return PluginDashboardData(
        registry: registry,
        installed: installed,
        activeSessions: activeSessions,
      );
    });
  }

  /// 安装成功后局部添加/替换插件 — 避免全页 loading
  void addInstalledPlugin(frb_manifest.PluginManifest manifest) {
    final current = state.value;
    if (current == null) return;
    final updated = [
      ...current.installed.where((m) => m.pluginId != manifest.pluginId),
      manifest,
    ];
    state = AsyncData(current.copyWith(installed: updated));
  }

  /// 卸载成功后局部移除插件 — 避免全页 loading
  void removeInstalledPlugin(String pluginId) {
    final current = state.value;
    if (current == null) return;
    final updated = current.installed.where((m) => m.pluginId != pluginId).toList();
    state = AsyncData(current.copyWith(installed: updated));
  }
}

final pluginDashboardProvider = AsyncNotifierProvider<PluginDashboardNotifier, PluginDashboardData>(
  () => PluginDashboardNotifier(),
);

/// 插件看板聚合数据
class PluginDashboardData {
  final PluginRegistry registry;
  final List<frb_manifest.PluginManifest> installed;
  final List<frb.PluginSessionInfo> activeSessions;

  PluginDashboardData({
    required this.registry,
    required this.installed,
    required this.activeSessions,
  });

  bool isInstalled(String pluginId) {
    for (final m in installed) {
      if (m.pluginId == pluginId) return true;
    }
    return false;
  }

  bool isRunning(String pluginId) {
    for (final s in activeSessions) {
      if (s.pluginId == pluginId) return true;
    }
    return false;
  }

  String? installedVersion(String pluginId) {
    for (final m in installed) {
      if (m.pluginId == pluginId) return m.version;
    }
    return null;
  }

  String? latestVersion(String pluginId) => registry.plugins[pluginId]?.latestVersion;

  bool hasUpdate(String pluginId) {
    final local = installedVersion(pluginId);
    final remote = latestVersion(pluginId);
    if (local == null || remote == null) return false;
    try {
      return Version.parse(remote) > Version.parse(local);
    } on Exception {
      // 回退到字符串比较
      return remote != local;
    }
  }

  List<String> get allPluginIds {
    final ids = <String>{...registry.plugins.keys};
    for (final m in installed) {
      ids.add(m.pluginId);
    }
    return ids.toList()..sort();
  }

  List<String> get installedIds {
    final ids = <String>[];
    for (final m in installed) {
      ids.add(m.pluginId);
    }
    return ids;
  }

  List<String> get availableIds =>
      registry.plugins.keys.where((id) => !isInstalled(id)).toList();

  PluginDashboardData copyWith({
    PluginRegistry? registry,
    List<frb_manifest.PluginManifest>? installed,
    List<frb.PluginSessionInfo>? activeSessions,
  }) {
    return PluginDashboardData(
      registry: registry ?? this.registry,
      installed: installed ?? this.installed,
      activeSessions: activeSessions ?? this.activeSessions,
    );
  }
}

// ============================================================================
// Plugin Search / Filter State
// ============================================================================

class _PluginSearchQueryNotifier extends Notifier<String> {
  @override
  String build() => '';
  void set(String value) => state = value;
}

class _PluginSelectedTabNotifier extends Notifier<int> {
  @override
  int build() => 0;
  void set(int value) => state = value;
}

final pluginSearchQueryProvider = NotifierProvider<_PluginSearchQueryNotifier, String>(
  () => _PluginSearchQueryNotifier(),
);

final pluginSelectedTabProvider = NotifierProvider<_PluginSelectedTabNotifier, int>(
  () => _PluginSelectedTabNotifier(),
); // 0=all, 1=installed, 2=available

// ============================================================================
// Plugin Actions
// ============================================================================

Future<void> refreshPluginDashboard(WidgetRef ref) async {
  ref.invalidate(pluginRegistryStateProvider);
  ref.invalidate(installedPluginsProvider);
  ref.invalidate(activeSessionsProvider);
}

Future<void> installPlugin(
  WidgetRef ref,
  String pluginId,
  PluginRegistryEntry entry,
  String appVersion,
  String pluginApiVersion, {
  String? targetVersion,
}) async {
  final installer = await ref.read(initializedPluginInstallerProvider.future);
  await installer.installFromMarket(
    pluginId,
    entry,
    appVersion,
    pluginApiVersion,
    targetVersion: targetVersion,
  );
  ref.invalidate(installedPluginsProvider);
  ref.invalidate(pluginRegistryStateProvider);
}

/// 下载插件工件（不安装），用于安装前审查。
Future<PluginArtifacts> downloadPluginArtifacts(
  WidgetRef ref,
  String pluginId,
  PluginRegistryEntry entry,
  String appVersion,
  String pluginApiVersion, {
  String? targetVersion,
}) async {
  final installer = await ref.read(initializedPluginInstallerProvider.future);
  return installer.downloadPluginArtifacts(
    pluginId,
    entry,
    appVersion,
    pluginApiVersion,
    targetVersion: targetVersion,
  );
}

/// 从已下载的工件安装插件。
Future<void> installFromArtifacts(
  WidgetRef ref,
  PluginArtifacts artifacts,
) async {
  final installer = await ref.read(initializedPluginInstallerProvider.future);
  await installer.installFromArtifacts(artifacts);
  ref.invalidate(installedPluginsProvider);
  ref.invalidate(pluginRegistryStateProvider);
}

Future<void> uninstallPlugin(WidgetRef ref, String pluginId) async {
  final installer = await ref.read(initializedPluginInstallerProvider.future);
  await installer.uninstall(pluginId);
  ref.invalidate(installedPluginsProvider);
  ref.invalidate(activeSessionsProvider);
}

Stream<frb_plugin.PluginEvent> runPlugin(WidgetRef ref, String pluginId) async* {
  final service = await ref.read(_initializedPluginServiceProvider.future);
  yield* service.runPlugin(pluginId);
  ref.invalidate(activeSessionsProvider);
}
