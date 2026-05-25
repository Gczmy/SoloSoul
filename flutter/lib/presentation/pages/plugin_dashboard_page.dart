import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:version/version.dart';

import 'dart:io' show Platform;

import 'package:solosoul_flutter/frb/api.dart' as frb;
import 'package:solosoul_flutter/frb/plugin/manager.dart' as frb_plugin;
import 'package:solosoul_flutter/frb/plugin/manifest.dart' as frb_manifest;
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/plugin_provider.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/plugin_models.dart' show PluginArtifacts, PluginRegistryEntry, resolvePluginI18n;
import 'package:solosoul_flutter/core/models/semantic_type_registry.dart';
import 'package:solosoul_flutter/presentation/widgets/plugin_access_review_dialog.dart';
import 'package:solosoul_flutter/presentation/widgets/plugin_consent_dialog.dart';
import 'package:solosoul_flutter/presentation/widgets/plugin_detail_dialog.dart';

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
    final locale = Localizations.localeOf(context).toString();
    final filtered = pluginIds.where((id) {
      final manifest = _getManifest(data, id);
      final entry = data.registry.plugins[id];
      final displayName = resolvePluginI18n(
        entry?.i18n, 'name', locale, manifest?.name ?? id,
      );
      return displayName.toLowerCase().contains(searchQuery);
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
        description: entry.description ?? '',
        publisher: entry.publisher,
        requiredFields: [],
        optionalFields: [],
        dataTtlSeconds: BigInt.from(300),
        requireUserConfirmation: true,
        consentValidityHours: BigInt.from(24),
        i18N: entry.i18n ?? {},
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
    final registryEntry = data.registry.plugins[pluginId];
    final locale = Localizations.localeOf(context).toString();
    final name = resolvePluginI18n(
      registryEntry?.i18n, 'name', locale, manifest?.name ?? pluginId,
    );
    final version = manifest?.version ?? '';
    final publisher = manifest?.publisher ?? '';

    final isInstalled = data.isInstalled(pluginId);
    final isRunning = data.isRunning(pluginId);
    final hasUpdate = data.hasUpdate(pluginId);

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
                      Row(
                        children: [
                          Text(
                            '$publisher · v$version',
                            style: TextStyle(
                              fontSize: 13,
                              color: Colors.grey.shade600,
                            ),
                          ),
                          if (registryEntry != null)
                            GestureDetector(
                              onTap: () => _showVersionHistory(context, ref),
                              child: Padding(
                                padding: const EdgeInsets.only(left: 6),
                                child: Icon(
                                  Icons.history,
                                  size: 14,
                                  color: Theme.of(context).colorScheme.primary,
                                ),
                              ),
                            ),
                        ],
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
          OutlinedButton.icon(
            onPressed: () => _onInstall(context, ref),
            icon: const Icon(Icons.download_rounded, size: 16),
            label: Text(l10n.pluginActionInstall),
            style: OutlinedButton.styleFrom(
              shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(20)),
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
              visualDensity: VisualDensity.compact,
              textStyle: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600),
            ),
          ),
        );
      }
    } else {
      // 已安装：显示运行/停止按钮
      if (isRunning) {
        buttons.push(
          OutlinedButton.icon(
            onPressed: Platform.isIOS ? null : () => _onStop(context, ref),
            icon: const Icon(Icons.stop_rounded, size: 16),
            label: Text(l10n.pluginActionStop),
            style: OutlinedButton.styleFrom(
              shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(20)),
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
              visualDensity: VisualDensity.compact,
              textStyle: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600),
            ),
          ),
        );
      } else {
        buttons.push(
          OutlinedButton.icon(
            onPressed: Platform.isIOS ? null : () => _onRun(context, ref),
            icon: const Icon(Icons.play_arrow_rounded, size: 16),
            label: Text(l10n.pluginActionRun),
            style: OutlinedButton.styleFrom(
              shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(20)),
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
              visualDensity: VisualDensity.compact,
              textStyle: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600),
            ),
          ),
        );
      }

      // 有更新：显示更新按钮
      if (hasUpdate) {
        buttons.push(
          OutlinedButton.icon(
            onPressed: () => _onUpdate(context, ref),
            icon: const Icon(Icons.update_rounded, size: 16),
            label: Text(l10n.pluginActionUpdate),
            style: OutlinedButton.styleFrom(
              shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(20)),
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
              visualDensity: VisualDensity.compact,
              textStyle: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600),
            ),
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

    // 详细信息按钮（始终显示在最右侧）
    buttons.push(
      TextButton(
        onPressed: () => _showPluginDetail(context, ref),
        child: Text(l10n.pluginActionDetail),
      ),
    );

    return buttons;
  }

  Future<void> _showPluginDetail(BuildContext context, WidgetRef ref) async {
    final installer = await ref.read(initializedPluginInstallerProvider.future);
    final installedInfo = await installer.getInstalledInfo(pluginId);

    frb_manifest.PluginManifest? installedManifest;
    if (data.isInstalled(pluginId)) {
      for (final m in data.installed) {
        if (m.pluginId == pluginId) {
          installedManifest = m;
          break;
        }
      }
    }

    if (context.mounted) {
      await showDialog<void>(
        context: context,
        builder: (ctx) => PluginDetailDialog(
          pluginId: pluginId,
          registryEntry: data.registry.plugins[pluginId],
          installedManifest: installedManifest,
          installedInfo: installedInfo,
          isInstalled: data.isInstalled(pluginId),
        ),
      );
    }
  }

  Future<void> _onInstall(BuildContext context, WidgetRef ref) async {
    await _performInstallOrUpdate(context, ref, isUpdate: false);
  }

  Future<void> _onUpdate(BuildContext context, WidgetRef ref) async {
    await _performInstallOrUpdate(context, ref, isUpdate: true);
  }

  Future<void> _performInstallOrUpdate(
    BuildContext context,
    WidgetRef ref, {
    required bool isUpdate,
    String? targetVersion,
  }) async {
    final l10n = AppLocalizations.of(context);
    final dashboard = ref.read(pluginDashboardProvider).asData?.value;
    if (dashboard == null) return;

    final entry = dashboard.registry.plugins[pluginId];
    if (entry == null) return;

    try {
      final packageInfo = await PackageInfo.fromPlatform();
      final versionKey = targetVersion ?? entry.latestVersion;
      final versionInfo = entry.versions[versionKey];
      final appVersion = packageInfo.version;
      final pluginApiVersion = versionInfo?.pluginApiVersion ?? '1.0';

      // 1. 下载插件工件（wasm + manifest）
      final artifacts = await downloadPluginArtifacts(
        ref,
        pluginId,
        entry,
        appVersion,
        pluginApiVersion,
        targetVersion: targetVersion,
      );

      // 2. 解析 field_access 并进行安装前审查
      final fieldAccess = artifacts.parseFieldAccess();
      if (fieldAccess != null &&
          fieldAccess.isNotEmpty &&
          context.mounted) {
        final shouldContinue = await _showAccessReview(
          context,
          ref,
          entry,
          fieldAccess,
        );
        if (!shouldContinue) return;
      }

      // 3. 执行安装
      await installFromArtifacts(ref, artifacts);

      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(
              isUpdate ? l10n.pluginUpdateSuccess : l10n.pluginInstallSuccess,
            ),
          ),
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

  /// 显示插件字段访问审查弹窗。
  /// 返回 `true` 表示用户选择继续安装，`false` 表示取消。
  Future<bool> _showAccessReview(
    BuildContext context,
    WidgetRef ref,
    PluginRegistryEntry entry,
    List<Map<String, dynamic>> fieldAccess,
  ) async {
    final locale = Localizations.localeOf(context).toString();
    final languageCode = Localizations.localeOf(context).languageCode;
    final pluginName = resolvePluginI18n(
      entry.i18n, 'name', locale, entry.name,
    );

    // 构建 FieldAccessStatus 列表（基于 manifest 声明，不扫描实际数据）
    final fieldStatuses = fieldAccess.map((access) {
      final semanticType = access['semantic_type'] as String?;
      final key = access['key'] as String?;
      final requiredSensitivityStr = access['required_sensitivity'] as String?;

      final requiredSensitivity = _parseSensitivity(requiredSensitivityStr);

      // 尝试从语义类型注册表获取标签
      String? fieldLabel;
      if (semanticType != null) {
        final type = SemanticTypeRegistry.getType(semanticType);
        fieldLabel = type?.getLabel(languageCode) ?? semanticType;
      }

      return FieldAccessStatus(
        fieldKey: key,
        fieldLabel: fieldLabel ?? key ?? semanticType ?? 'Unknown',
        semanticType: semanticType,
        sectionName: null,
        actualSensitivity: null,
        requiredSensitivity: requiredSensitivity,
        status: AccessStatus.ok,
      );
    }).toList();

    if (!context.mounted) return false;

    final result = await showDialog<bool>(
      context: context,
      barrierDismissible: false,
      builder: (ctx) => PluginAccessReviewDialog(
        pluginName: pluginName,
        fieldStatuses: fieldStatuses,
        onModifySensitivity: () {
          Navigator.of(ctx).pop(false);
        },
        onCreateMissingFields: () {
          Navigator.of(ctx).pop(false);
        },
        onContinueInstall: () => Navigator.of(ctx).pop(true),
        onCancel: () => Navigator.of(ctx).pop(false),
      ),
    );

    return result == true;
  }

  SensitivityLevel? _parseSensitivity(String? value) {
    return switch (value?.toLowerCase()) {
      'public' => SensitivityLevel.public,
      'internal' => SensitivityLevel.internal,
      'private' => SensitivityLevel.internal,
      'sensitive' => SensitivityLevel.sensitive,
      'restricted' => SensitivityLevel.sensitive,
      'critical' => SensitivityLevel.critical,
      _ => null,
    };
  }

  Future<void> _showVersionHistory(BuildContext context, WidgetRef ref) async {
    final l10n = AppLocalizations.of(context);
    final registryEntry = data.registry.plugins[pluginId];
    if (registryEntry == null) return;

    final installedVersion = data.installedVersion(pluginId);
    final versions = registryEntry.versions.entries.toList()
      ..sort((a, b) {
        try {
          return Version.parse(b.key).compareTo(Version.parse(a.key));
        } on Exception {
          return b.key.compareTo(a.key);
        }
      });

    await showModalBottomSheet<void>(
      context: context,
      isScrollControlled: true,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
      ),
      builder: (ctx) {
        return DraggableScrollableSheet(
          initialChildSize: 0.6,
          minChildSize: 0.3,
          maxChildSize: 0.9,
          expand: false,
          builder: (_, scrollController) {
            return Column(
              children: [
                Padding(
                  padding: const EdgeInsets.fromLTRB(16, 12, 16, 8),
                  child: Row(
                    children: [
                      Expanded(
                        child: Text(
                          l10n.pluginVersionHistoryTitle,
                          style: Theme.of(context).textTheme.titleMedium,
                        ),
                      ),
                      IconButton(
                        icon: const Icon(Icons.close),
                        onPressed: () => Navigator.of(ctx).pop(),
                        visualDensity: VisualDensity.compact,
                      ),
                    ],
                  ),
                ),
                const Divider(height: 1),
                Expanded(
                  child: ListView.builder(
                    controller: scrollController,
                    itemCount: versions.length,
                    itemBuilder: (context, index) {
                      final ver = versions[index].key;
                      final info = versions[index].value;
                      final isCurrent = ver == installedVersion;

                      return ListTile(
                        leading: Container(
                          width: 56,
                          alignment: Alignment.center,
                          child: isCurrent
                              ? Icon(
                                  Icons.check_circle,
                                  color: Theme.of(context).colorScheme.primary,
                                )
                              : Text(
                                  'v$ver',
                                  style: const TextStyle(
                                    fontWeight: FontWeight.w600,
                                    fontSize: 12,
                                  ),
                                ),
                        ),
                        title: Text(
                          isCurrent ? l10n.pluginVersionCurrentLabel(ver) : 'v$ver',
                          style: TextStyle(
                            fontWeight: isCurrent ? FontWeight.bold : FontWeight.normal,
                            color: isCurrent
                                ? Theme.of(context).colorScheme.primary
                                : null,
                          ),
                        ),
                        subtitle: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              info.releasedAt.toLocal().toString().split(' ').first,
                              style: TextStyle(
                                fontSize: 12,
                                color: Colors.grey.shade600,
                              ),
                            ),
                            if (info.changelog != null && info.changelog!.isNotEmpty)
                              Padding(
                                padding: const EdgeInsets.only(top: 4),
                                child: Text(
                                  info.changelog!,
                                  style: Theme.of(context).textTheme.bodySmall,
                                ),
                              ),
                          ],
                        ),
                        trailing: isCurrent
                            ? Chip(
                                label: Text(l10n.pluginDetailCurrent),
                                visualDensity: VisualDensity.compact,
                                backgroundColor: Colors.transparent,
                                side: BorderSide(
                                  color: Theme.of(context)
                                      .colorScheme
                                      .primary
                                      .withValues(alpha: 0.5),
                                ),
                              )
                            : TextButton(
                                onPressed: () async {
                                  Navigator.of(ctx).pop();
                                  await _performInstallOrUpdate(
                                    context,
                                    ref,
                                    isUpdate: data.isInstalled(pluginId),
                                    targetVersion: ver,
                                  );
                                },
                                child: Text(l10n.pluginActionInstall),
                              ),
                      );
                    },
                  ),
                ),
              ],
            );
          },
        );
      },
    );
  }

  Future<void> _onRun(BuildContext context, WidgetRef ref) async {
    final l10n = AppLocalizations.of(context);
    final stream = runPlugin(ref, pluginId);
    final List<String> formattedResults = [];
    final List<String> errorMessages = [];
    final batchRequests = <frb_plugin.PluginEvent_ConsentRequest>[];
    String? batchPluginName;

    try {
      await for (final event in stream) {
        switch (event) {
          case frb_plugin.PluginEvent_ConsentRequest(
              pluginName: final pname,
            ):
            // 批量模式：缓存请求，等待 batch_end 信号后统一弹窗
            batchRequests.add(event);
            if (batchPluginName == null) {
              final entry = data.registry.plugins[pluginId];
              final locale = Localizations.localeOf(context).toString();
              batchPluginName = resolvePluginI18n(
                entry?.i18n, 'name', locale, pname,
              );
            }
          case frb_plugin.PluginEvent_Log(level: final level, message: final message):
            // 批量预授权结束信号：显示批量授权对话框
            if (level == 'batch_end' && batchRequests.isNotEmpty) {
              final approved = await showDialog<bool>(
                context: context,
                barrierDismissible: false,
                builder: (ctx) => PluginBatchConsentDialog(
                  pluginId: pluginId,
                  pluginName: batchPluginName ?? pluginId,
                  requests: batchRequests.map((r) => BatchConsentRequest(
                    requestId: r.requestId,
                    field: r.field,
                    sensitivity: r.sensitivity,
                  )).toList(),
                ),
              );
              // 逐个响应所有预授权请求
              for (final req in batchRequests) {
                try {
                  await frb.frbPluginConsentResponse(
                    requestId: req.requestId,
                    approved: approved == true,
                    value: null,
                  );
                } on Exception catch (_) {
                  // 忽略 consent 响应错误
                }
              }
              batchRequests.clear();
              // 如果用户拒绝，本次执行后续不会再有 ConsentRequest（Rust 已终止）
            }
            // 收集插件输出的格式化结果日志
            if (level == 'info' && message.contains('格式化结果:')) {
              final result = message.split('格式化结果:').last.trim();
              if (result.isNotEmpty) {
                formattedResults.add(result);
              }
            }
            // 收集插件错误日志
            if (level == 'error') {
              errorMessages.add(message);
            }
          case frb_plugin.PluginEvent_Completed(exitCode: final exitCode):
            // 记录最近使用时间
            final installer = await ref.read(initializedPluginInstallerProvider.future);
            await installer.recordLastUsed(pluginId);
            if (context.mounted) {
              if (formattedResults.isNotEmpty) {
                final registryEntry = data.registry.plugins[pluginId];
                final locale = Localizations.localeOf(context).toString();
                final pluginName = resolvePluginI18n(
                  registryEntry?.i18n, 'name', locale, _getManifest()?.name ?? pluginId,
                );
                await showDialog<void>(
                  context: context,
                  builder: (ctx) => AlertDialog(
                    insetPadding: EdgeInsets.symmetric(
                      horizontal: MediaQuery.of(ctx).size.width * 0.2,
                      vertical: 24,
                    ),
                    title: Row(
                      children: [
                        const Icon(Icons.check_circle_outline),
                        const SizedBox(width: 8),
                        Expanded(child: Text('$pluginName 结果')),
                        IconButton(
                          icon: const Icon(Icons.close),
                          onPressed: () => Navigator.of(ctx).pop(),
                          visualDensity: VisualDensity.compact,
                        ),
                      ],
                    ),
                    content: SizedBox(
                      width: double.maxFinite,
                      child: ListView.builder(
                        shrinkWrap: true,
                        itemCount: formattedResults.length,
                        itemBuilder: (context, index) {
                          final parts = formattedResults[index].split(' | ');
                          final label = parts.length > 1 ? parts[0] : null;
                          final address = parts.length > 1 ? parts[1] : formattedResults[index];
                          return Padding(
                            padding: const EdgeInsets.symmetric(vertical: 6),
                            child: Row(
                              crossAxisAlignment: CrossAxisAlignment.center,
                              children: [
                                Chip(
                                  label: Text(label ?? '${index + 1}'),
                                  visualDensity: VisualDensity.compact,
                                  backgroundColor: Colors.transparent,
                                  side: BorderSide(
                                    color: Theme.of(context).colorScheme.primary.withValues(alpha: 0.5),
                                  ),
                                  labelStyle: TextStyle(
                                    fontSize: 11,
                                    fontWeight: FontWeight.bold,
                                    color: Theme.of(context).colorScheme.primary,
                                  ),
                                ),
                                const SizedBox(width: 12),
                                Expanded(
                                  child: SelectableText(
                                    address,
                                    style: Theme.of(context).textTheme.bodyMedium,
                                  ),
                                ),
                                const SizedBox(width: 8),
                                IconButton(
                                  icon: Icon(
                                    Icons.copy,
                                    size: 18,
                                    color: Theme.of(context).colorScheme.primary,
                                  ),
                                  tooltip: '复制',
                                  onPressed: () {
                                    Clipboard.setData(
                                      ClipboardData(text: address),
                                    );
                                    ScaffoldMessenger.of(context).showSnackBar(
                                      const SnackBar(
                                        content: Text('已复制到剪贴板'),
                                        duration: Duration(seconds: 1),
                                      ),
                                    );
                                  },
                                  visualDensity: VisualDensity.compact,
                                  padding: EdgeInsets.zero,
                                  constraints: const BoxConstraints(),
                                ),
                              ],
                            ),
                          );
                        },
                      ),
                    ),
                    actions: [
                      TextButton(
                        onPressed: () => Navigator.of(ctx).pop(),
                        child: Text(l10n.commonClose),
                      ),
                    ],
                  ),
                );
              } else if (exitCode == 0) {
                // 只有真正成功且没有结果时才显示成功提示
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(
                    content: Text('${l10n.pluginRunSuccess} (exit: $exitCode)'),
                  ),
                );
              } else {
                // 插件执行失败，显示收集到的错误日志
                final errorMsg = errorMessages.isNotEmpty
                    ? errorMessages.join('\n')
                    : '插件执行失败 (exit: $exitCode)';
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(
                    content: Text(errorMsg),
                    backgroundColor: AppTheme.errorColor,
                  ),
                );
              }
            }
          case frb_plugin.PluginEvent_Error(message: final message):
            // 用户主动拒绝授权或超时，属于正常流程，不显示错误提示
            if (message.contains('User denied or timed out field access')) {
              break;
            }
            if (context.mounted) {
              ScaffoldMessenger.of(context).showSnackBar(
                SnackBar(content: Text('${l10n.commonError}: $message')),
              );
            }
          default:
            // 忽略 ConsentTimeout / Progress 等事件
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
        description: entry.description ?? '',
        publisher: entry.publisher,
        requiredFields: [],
        optionalFields: [],
        dataTtlSeconds: BigInt.from(300),
        requireUserConfirmation: true,
        consentValidityHours: BigInt.from(24),
        i18N: entry.i18n ?? {},
      );
    }
    return null;
  }
}

// Dart 3 的 List.push 扩展
extension _ListPush<T> on List<T> {
  void push(T item) => add(item);
}
