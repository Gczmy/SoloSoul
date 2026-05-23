import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:solosoul_flutter/core/models/plugin_models.dart';
import 'package:solosoul_flutter/core/services/plugin_installer_service.dart';
import 'package:solosoul_flutter/core/services/plugin_registry_service.dart';
import 'package:solosoul_flutter/core/services/plugin_service.dart';
import 'package:solosoul_flutter/frb/api.dart' as frb;
import 'package:solosoul_flutter/frb/plugin/manager.dart' as frb_plugin;
import 'package:solosoul_flutter/frb/plugin/manifest.dart' as frb_manifest;

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
  return PluginRegistryService();
});

// ============================================================================
// Initialized Service Provider (auto-init on first use)
// ============================================================================

final _initializedPluginServiceProvider = FutureProvider<PluginService>((ref) async {
  final service = ref.read(pluginServiceProvider);
  await service.initialize();
  return service;
});

final _initializedPluginInstallerProvider = FutureProvider<PluginInstallerService>((ref) async {
  final service = ref.read(pluginInstallerProvider);
  await service.initialize();
  return service;
});

final _initializedPluginRegistryProvider = FutureProvider<PluginRegistryService>((ref) async {
  final service = ref.read(pluginRegistryProvider);
  await service.initialize();
  return service;
});

// ============================================================================
// Plugin Registry State
// ============================================================================

final pluginRegistryStateProvider = FutureProvider<PluginRegistry>((ref) async {
  final registryService = await ref.watch(_initializedPluginRegistryProvider.future);
  return registryService.getRegistry();
});

// ============================================================================
// Installed Plugins State
// ============================================================================

final installedPluginsProvider = FutureProvider<List<frb_manifest.PluginManifest>>((ref) async {
  final service = await ref.watch(_initializedPluginServiceProvider.future);
  return service.loadInstalledPlugins();
});

// ============================================================================
// Active Sessions State
// ============================================================================

final activeSessionsProvider = FutureProvider<List<frb.PluginSessionInfo>>((ref) async {
  final service = await ref.watch(_initializedPluginServiceProvider.future);
  return service.listActiveSessions();
});



// ============================================================================
// Plugin Dashboard Combined State
// ============================================================================

final pluginDashboardProvider = FutureProvider<PluginDashboardData>((ref) async {
  final registryAsync = ref.watch(pluginRegistryStateProvider);
  final installedAsync = ref.watch(installedPluginsProvider);
  final sessionsAsync = ref.watch(activeSessionsProvider);

  final registry = registryAsync.asData?.value ?? PluginRegistry.empty();
  final installed = installedAsync.asData?.value ?? <frb_manifest.PluginManifest>[];
  final activeSessions = sessionsAsync.asData?.value ?? <frb.PluginSessionInfo>[];

  return PluginDashboardData(
    registry: registry,
    installed: installed,
    activeSessions: activeSessions,
  );
});

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
    return local != null && remote != null && local != remote;
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
  String pluginApiVersion,
) async {
  final installer = await ref.read(_initializedPluginInstallerProvider.future);
  await installer.installFromMarket(pluginId, entry, appVersion, pluginApiVersion);
  ref.invalidate(installedPluginsProvider);
  ref.invalidate(pluginRegistryStateProvider);
}

Future<void> uninstallPlugin(WidgetRef ref, String pluginId) async {
  final installer = await ref.read(_initializedPluginInstallerProvider.future);
  await installer.uninstall(pluginId);
  ref.invalidate(installedPluginsProvider);
  ref.invalidate(activeSessionsProvider);
}

Stream<frb_plugin.PluginEvent> runPlugin(WidgetRef ref, String pluginId) async* {
  final service = await ref.read(_initializedPluginServiceProvider.future);
  yield* service.runPlugin(pluginId);
  ref.invalidate(activeSessionsProvider);
}
