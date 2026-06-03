import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';

import 'dart:convert' show jsonDecode, jsonEncode;
import 'dart:io' show Platform;

import 'package:solosoul_flutter/frb/plugin/manifest.dart' as frb_manifest;
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/plugin_provider.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';
import 'package:solosoul_flutter/core/models/plugin_models.dart' show PluginRegistryEntry, resolvePluginI18n;
import 'package:solosoul_flutter/presentation/pages/plugin_dashboard/plugin_card.dart';
import 'package:solosoul_flutter/presentation/pages/plugin_dashboard/plugin_result_cards.dart';

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
        backRoute: AppRoutes.home,
        title: Text(l10n.pluginDashboardTitle),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: () => ref.read(pluginDashboardProvider.notifier).refresh(),
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
      final manifest = getPluginManifest(data, id);
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
        return PluginCard(
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
}

/// Resolves a plugin manifest from installed list or registry entry.
frb_manifest.PluginManifest? getPluginManifest(
  PluginDashboardData data,
  String pluginId,
) {
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

// Phase 2: 结构化结果卡片渲染系统
// ============================================================================

/// Mutable state for a single plugin run session.
/// Extracted to reduce `_onRun` nesting and enable method extraction.
class PluginRunSession {
  final List<String> pluginLogs = [];
  final List<String> errorMessages = [];
  final List<PluginResultData> pluginResults = [];
  final List<dynamic> batchRequests = [];
  final Map<String, String> dialogConfigs = {};
  String? batchPluginName;
  int? completedExitCode;
  bool hasCompleted = false;
  bool batchPreConsentPhase = true;
}

/// 插件结构化结果数据（从 solosoul_result 通道的 JSON 解析）
class PluginResultData {
  final String type;
  final Map<String, dynamic> data;

  const PluginResultData({required this.type, required this.data});

  factory PluginResultData.fromJson(String jsonStr) {
    final json = jsonDecode(jsonStr) as Map<String, dynamic>;
    final type = json['type'] as String? ?? 'text';
    return PluginResultData(type: type, data: json);
  }

  /// 生成适合复制的纯文本格式
  String toCopyText() {
    switch (type) {
      case 'text':
        return data['content'] as String? ?? '';
      case 'key_value':
        // 如果提供了 csv 字段，优先返回 CSV 格式（如联系人导出）
        final csv = data['csv'] as String?;
        if (csv != null && csv.isNotEmpty) {
          return csv;
        }
        final pairs = (data['pairs'] as List<dynamic>?) ?? [];
        return pairs.map((p) {
          final pair = p as Map<String, dynamic>;
          return '${pair['key']}: ${pair['value']}';
        }).join('\n');
      case 'table':
        final headers = (data['headers'] as List<dynamic>?)?.cast<String>() ?? [];
        final rows = (data['rows'] as List<dynamic>?)?.cast<List<dynamic>>() ?? [];
        final buffer = StringBuffer();
        buffer.writeln(headers.join('\t'));
        for (final row in rows) {
          buffer.writeln(row.cast<String?>().join('\t'));
        }
        return buffer.toString();
      case 'markdown':
        return data['content'] as String? ?? '';
      default:
        return jsonEncode(data);
    }
  }
}

/// 结果卡片渲染器函数签名
typedef ResultCardBuilder = Widget Function(BuildContext context, PluginResultData result);

/// 渲染器注册表（策略模式）
final Map<String, ResultCardBuilder> resultCardRenderers = {
  'text': (context, result) => TextResultCard(data: result.data),
  'key_value': (context, result) => KeyValueResultCard(data: result.data),
  'table': (context, result) => TableResultCard(data: result.data),
  'markdown': (context, result) => MarkdownResultCard(data: result.data),
  'calendar_events': (context, result) => CalendarEventsResultCard(data: result.data),
  'data_completeness': (context, result) => DataCompletenessResultCard(data: result.data),
};
