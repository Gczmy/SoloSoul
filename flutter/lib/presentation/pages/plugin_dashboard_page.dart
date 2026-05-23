import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';

import 'dart:io' show Platform;

import 'package:solosoul_flutter/frb/api.dart' as frb;
import 'package:solosoul_flutter/frb/plugin/manager.dart' as frb_plugin;
import 'package:solosoul_flutter/frb/plugin/manifest.dart' as frb_manifest;
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/plugin_provider.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';
import 'package:solosoul_flutter/presentation/widgets/plugin_consent_dialog.dart';

/// 插件看板页面 — 管理插件生命周期（安装/卸载/更新/运行）
class PluginDashboardPage extends ConsumerStatefulWidget {
  const PluginDashboardPage({super.key});

  @override
  ConsumerState<PluginDashboardPage> createState() => _PluginDashboardPageState();
}

class _PluginDashboardPageState extends ConsumerState<PluginDashboardPage>
    with SingleTickerProviderStateMixin {
  late final TabController _tabController;
  final _searchController = TextEditingController();

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 3, vsync: this);
    _tabController.addListener(() {
      ref.read(pluginSelectedTabProvider.notifier).set(_tabController.index);
    });
  }

  @override
  void dispose() {
    _tabController.dispose();
    _searchController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final dashboardAsync = ref.watch(pluginDashboardProvider);
    final searchQuery = ref.watch(pluginSearchQueryProvider);

    return Scaffold(
      appBar: SoloGlassAppBar(
        backRoute: '/',
        title: Text(l10n.pluginDashboardTitle),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: () => refreshPluginDashboard(ref),
          ),
        ],
      ),
      body: dashboardAsync.when(
        data: (data) => _buildBody(context, l10n, data, searchQuery),
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (err, stack) => Center(
          child: Text('${l10n.commonError}: $err'),
        ),
      ),
    );
  }

  Widget _buildBody(
    BuildContext context,
    AppLocalizations l10n,
    PluginDashboardData data,
    String searchQuery,
  ) {
    final isOffline = data.registry.plugins.isEmpty && data.installed.isEmpty;

    return Column(
      children: [
        // 搜索栏
        Padding(
          padding: const EdgeInsets.all(16),
          child: TextField(
            controller: _searchController,
            decoration: InputDecoration(
              hintText: l10n.pluginSearchHint,
              prefixIcon: const Icon(Icons.search),
              suffixIcon: searchQuery.isNotEmpty
                  ? IconButton(
                      icon: const Icon(Icons.clear),
                      onPressed: () {
                        _searchController.clear();
                        ref.read(pluginSearchQueryProvider.notifier).set('');
                      },
                    )
                  : null,
              border: OutlineInputBorder(borderRadius: BorderRadius.circular(12)),
            ),
            onChanged: (value) {
              ref.read(pluginSearchQueryProvider.notifier).set(value.toLowerCase());
            },
          ),
        ),

        // iOS 平台不支持插件运行（Wasmtime JIT 限制）
        if (Platform.isIOS)
          Container(
            width: double.infinity,
            padding: const EdgeInsets.symmetric(vertical: 12, horizontal: 16),
            color: Colors.amber.withValues(alpha: 0.15),
            child: Row(
              children: [
                Icon(Icons.info_outline, color: Colors.amber.shade800, size: 18),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    l10n.pluginIOSUnsupportedBanner,
                    style: TextStyle(color: Colors.amber.shade900, fontSize: 13),
                  ),
                ),
              ],
            ),
          ),

        // 离线横幅
        if (isOffline && data.installed.isEmpty)
          Container(
            width: double.infinity,
            padding: const EdgeInsets.symmetric(vertical: 12, horizontal: 16),
            color: Colors.orange.withValues(alpha: 0.15),
            child: Row(
              children: [
                Icon(Icons.wifi_off, color: Colors.orange.shade700, size: 18),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    l10n.pluginOfflineBanner,
                    style: TextStyle(color: Colors.orange.shade800, fontSize: 13),
                  ),
                ),
              ],
            ),
          ),

        // Tab 切换
        TabBar(
          controller: _tabController,
          tabs: [
            Tab(text: '${l10n.pluginTabAll} (${data.allPluginIds.length})'),
            Tab(text: '${l10n.pluginTabInstalled} (${data.installed.length})'),
            Tab(text: '${l10n.pluginTabAvailable} (${data.availableIds.length})'),
          ],
        ),

        // 插件列表
        Expanded(
          child: TabBarView(
            controller: _tabController,
            children: [
              _buildPluginList(context, l10n, data, data.allPluginIds, searchQuery),
              _buildPluginList(context, l10n, data, data.installedIds, searchQuery),
              _buildPluginList(context, l10n, data, data.availableIds, searchQuery),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildPluginList(
    BuildContext context,
    AppLocalizations l10n,
    PluginDashboardData data,
    List<String> pluginIds,
    String searchQuery,
  ) {
    // 搜索过滤
    final filtered = pluginIds.where((id) {
      final manifest = _getManifest(data, id);
      final name = manifest?.name.toLowerCase() ?? id.toLowerCase();
      return name.contains(searchQuery);
    }).toList();

    if (filtered.isEmpty) {
      return _buildEmptyState(context, l10n);
    }

    return ListView.builder(
      padding: const EdgeInsets.all(16),
      itemCount: filtered.length,
      itemBuilder: (context, index) {
        final pluginId = filtered[index];
        return _PluginCard(
          pluginId: pluginId,
          data: data,
        ).animate().fadeIn(delay: (index * 50).ms).slideY(begin: 0.1, end: 0);
      },
    );
  }

  Widget _buildEmptyState(BuildContext context, AppLocalizations l10n) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(Icons.extension_off_outlined, size: 64, color: Colors.grey.shade400),
          const SizedBox(height: 16),
          Text(
            l10n.pluginEmptyStateTitle,
            style: Theme.of(context).textTheme.titleMedium,
          ),
          const SizedBox(height: 8),
          Text(
            l10n.pluginEmptyStateSubtitle,
            style: Theme.of(context).textTheme.bodyMedium?.copyWith(
              color: Colors.grey.shade600,
            ),
          ),
        ],
      ),
    );
  }

  frb_manifest.PluginManifest? _getManifest(PluginDashboardData data, String pluginId) {
    for (final m in data.installed) {
      if (m.pluginId == pluginId) return m;
    }
    final entry = data.registry.plugins[pluginId];
    if (entry != null) {
      return frb_manifest.PluginManifest(
        pluginId: pluginId,
        name: entry.name,
        version: entry.latestVersion,
        pluginApiVersion: '',
        minAppVersion: '',
        maxAppVersion: '',
        description: '',
        publisher: entry.publisher,
        requiredFields: [],
        optionalFields: [],
        dataTtlSeconds: BigInt.from(300),
        requireUserConfirmation: true,
        consentValidityHours: BigInt.from(24),
      );
    }
    return null;
  }
}

// ============================================================================
// Plugin Card
// ============================================================================

class _PluginCard extends ConsumerWidget {
  final String pluginId;
  final PluginDashboardData data;

  const _PluginCard({required this.pluginId, required this.data});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final manifest = _getManifest();
    final name = manifest?.name ?? pluginId;
    final version = manifest?.version ?? '';
    final publisher = manifest?.publisher ?? '';

    final isInstalled = data.isInstalled(pluginId);
    final isRunning = data.isRunning(pluginId);
    final hasUpdate = data.hasUpdate(pluginId);
    final registryEntry = data.registry.plugins[pluginId];

    // 确定状态标签
    String statusLabel;
    Color statusColor;
    if (isRunning) {
      statusLabel = l10n.pluginStatusRunning;
      statusColor = Colors.purple;
    } else if (hasUpdate) {
      statusLabel = l10n.pluginStatusUpdateAvailable;
      statusColor = Colors.orange;
    } else if (isInstalled) {
      statusLabel = l10n.pluginStatusInstalled;
      statusColor = Colors.green;
    } else if (registryEntry != null) {
      statusLabel = l10n.pluginStatusNotInstalled;
      statusColor = Colors.grey;
    } else {
      statusLabel = l10n.pluginStatusIncompatible;
      statusColor = Colors.red;
    }

    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Container(
                  width: 40,
                  height: 40,
                  decoration: BoxDecoration(
                    color: Theme.of(context).colorScheme.primaryContainer,
                    borderRadius: BorderRadius.circular(10),
                  ),
                  child: Icon(
                    Icons.extension,
                    color: Theme.of(context).colorScheme.primary,
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        name,
                        style: const TextStyle(
                          fontWeight: FontWeight.w600,
                          fontSize: 16,
                        ),
                      ),
                      const SizedBox(height: 2),
                      Text(
                        '$publisher · v$version',
                        style: TextStyle(
                          fontSize: 13,
                          color: Colors.grey.shade600,
                        ),
                      ),
                    ],
                  ),
                ),
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                  decoration: BoxDecoration(
                    color: statusColor.withValues(alpha: 0.12),
                    borderRadius: BorderRadius.circular(6),
                  ),
                  child: Text(
                    statusLabel,
                    style: TextStyle(
                      fontSize: 12,
                      fontWeight: FontWeight.w500,
                      color: statusColor,
                    ),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 12),
            Wrap(
              spacing: 8,
              children: _buildActionButtons(context, ref, l10n, isInstalled, isRunning, hasUpdate),
            ),
          ],
        ),
      ),
    );
  }

  List<Widget> _buildActionButtons(
    BuildContext context,
    WidgetRef ref,
    AppLocalizations l10n,
    bool isInstalled,
    bool isRunning,
    bool hasUpdate,
  ) {
    final buttons = <Widget>[];

    if (!isInstalled) {
      // 未安装：显示安装按钮
      final dash = ref.read(pluginDashboardProvider).asData?.value;
      if (dash != null && dash.registry.plugins.containsKey(pluginId)) {
        buttons.push(
          FilledButton.tonal(
            onPressed: () => _onInstall(context, ref),
            child: Text(l10n.pluginActionInstall),
          ),
        );
      }
    } else {
      // 已安装：显示运行/停止按钮
      if (isRunning) {
        buttons.push(
          OutlinedButton(
            onPressed: Platform.isIOS ? null : () => _onStop(context, ref),
            child: Text(l10n.pluginActionStop),
          ),
        );
      } else {
        buttons.push(
          FilledButton(
            onPressed: Platform.isIOS ? null : () => _onRun(context, ref),
            child: Text(l10n.pluginActionRun),
          ),
        );
      }

      // 有更新：显示更新按钮
      if (hasUpdate) {
        buttons.push(
          FilledButton.tonal(
            onPressed: () => _onUpdate(context, ref),
            child: Text(l10n.pluginActionUpdate),
          ),
        );
      }

      // 卸载按钮
      buttons.push(
        TextButton(
          onPressed: () => _onUninstall(context, ref),
          style: TextButton.styleFrom(foregroundColor: AppTheme.errorColor),
          child: Text(l10n.pluginActionUninstall),
        ),
      );
    }

    return buttons;
  }

  Future<void> _onInstall(BuildContext context, WidgetRef ref) async {
    final l10n = AppLocalizations.of(context);
    final dashboard = ref.read(pluginDashboardProvider).asData?.value;
    if (dashboard == null) return;

    final entry = dashboard.registry.plugins[pluginId];
    if (entry == null) return;

    try {
      // TODO: 获取实际 appVersion 和 pluginApiVersion
      await installPlugin(ref, pluginId, entry, '1.0.0', '1.0');
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(l10n.pluginInstallSuccess)),
        );
      }
    } on Exception catch (e) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('${l10n.commonError}: $e')),
        );
      }
    }
  }

  Future<void> _onUpdate(BuildContext context, WidgetRef ref) async {
    // 更新逻辑与安装相同（覆盖安装）
    await _onInstall(context, ref);
  }

  Future<void> _onRun(BuildContext context, WidgetRef ref) async {
    final l10n = AppLocalizations.of(context);
    final stream = runPlugin(ref, pluginId);

    try {
      await for (final event in stream) {
        switch (event) {
          case frb_plugin.PluginEvent_ConsentRequest(
              requestId: final requestId,
              pluginId: final pid,
              pluginName: final pname,
              field: final field,
              sensitivity: final sensitivityStr,
            ):
            final approved = await showDialog<bool>(
              context: context,
              barrierDismissible: false,
              builder: (ctx) => PluginConsentDialog(
                pluginId: pid,
                pluginName: pname,
                fieldId: field,
                requestId: requestId,
                sensitivity: _parseSensitivity(sensitivityStr),
              ),
            );
            await frb.frbPluginConsentResponse(
              requestId: requestId,
              approved: approved ?? false,
              value: null,
            );
          case frb_plugin.PluginEvent_Completed(exitCode: final exitCode):
            if (context.mounted) {
              ScaffoldMessenger.of(context).showSnackBar(
                SnackBar(
                  content: Text('${l10n.pluginRunSuccess} (exit: $exitCode)'),
                ),
              );
            }
          case frb_plugin.PluginEvent_Error(message: final message):
            if (context.mounted) {
              ScaffoldMessenger.of(context).showSnackBar(
                SnackBar(content: Text('${l10n.commonError}: $message')),
              );
            }
          default:
            // 忽略 ConsentTimeout / Log / Progress 等事件
            break;
        }
      }
    } on Exception catch (e) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('${l10n.commonError}: $e')),
        );
      }
    }
  }

  SensitivityLevel _parseSensitivity(String value) {
    return switch (value.toLowerCase()) {
      'public' => SensitivityLevel.public,
      'internal' => SensitivityLevel.internal,
      'sensitive' => SensitivityLevel.sensitive,
      'critical' => SensitivityLevel.critical,
      _ => SensitivityLevel.sensitive,
    };
  }

  Future<void> _onStop(BuildContext context, WidgetRef ref) async {
    final service = ref.read(pluginServiceProvider);
    await service.initialize();
    await service.forceUnload(pluginId);
    ref.invalidate(activeSessionsProvider);
  }

  Future<void> _onUninstall(BuildContext context, WidgetRef ref) async {
    final l10n = AppLocalizations.of(context);
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(l10n.pluginUninstallConfirmTitle),
        content: Text(l10n.pluginUninstallConfirmMessage),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: Text(l10n.commonCancel),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            style: FilledButton.styleFrom(backgroundColor: AppTheme.errorColor),
            child: Text(l10n.pluginActionUninstall),
          ),
        ],
      ),
    );

    if (confirmed == true) {
      try {
        await uninstallPlugin(ref, pluginId);
        if (context.mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(content: Text(l10n.pluginUninstallSuccess)),
          );
        }
      } on Exception catch (e) {
        if (context.mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(content: Text('${l10n.commonError}: $e')),
          );
        }
      }
    }
  }

  frb_manifest.PluginManifest? _getManifest() {
    for (final m in data.installed) {
      if (m.pluginId == pluginId) return m;
    }
    final entry = data.registry.plugins[pluginId];
    if (entry != null) {
      return frb_manifest.PluginManifest(
        pluginId: pluginId,
        name: entry.name,
        version: entry.latestVersion,
        pluginApiVersion: '',
        minAppVersion: '',
        maxAppVersion: '',
        description: '',
        publisher: entry.publisher,
        requiredFields: [],
        optionalFields: [],
        dataTtlSeconds: BigInt.from(300),
        requireUserConfirmation: true,
        consentValidityHours: BigInt.from(24),
      );
    }
    return null;
  }
}

// Dart 3 的 List.push 扩展
extension _ListPush<T> on List<T> {
  void push(T item) => add(item);
}
