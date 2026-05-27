import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:version/version.dart';

import 'dart:convert' show jsonDecode, jsonEncode;
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
import 'package:flutter_markdown/flutter_markdown.dart';
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
// Address Formatter Result Dialog — 结构化地址结果展示
// ============================================================================

class _AddressResult {
  final int index;
  final String label;
  final String formattedAddress;
  final String country;
  final String countryCode;

  const _AddressResult({
    required this.index,
    required this.label,
    required this.formattedAddress,
    required this.country,
    required this.countryCode,
  });
}

class _AddressFormatterResultDialog extends StatefulWidget {
  final String pluginName;
  final List<String> logs;
  final int exitCode;
  final bool hasErrors;

  const _AddressFormatterResultDialog({
    required this.pluginName,
    required this.logs,
    required this.exitCode,
    required this.hasErrors,
  });

  @override
  State<_AddressFormatterResultDialog> createState() => _AddressFormatterResultDialogState();
}

class _AddressFormatterResultDialogState extends State<_AddressFormatterResultDialog> {
  bool _logsExpanded = false;

  List<_AddressResult> _parseResults() {
    final countries = <int, String>{};
    final countryCodes = <int, String>{};
    final labels = <int, String>{};
    final formatted = <int, String>{};

    for (final log in widget.logs) {
      // 匹配: 地址[0] 国家识别: 中国 → CN
      final countryMatch = RegExp(r'地址\[(\d+)\] 国家识别: (.+) → (.+)').firstMatch(log);
      if (countryMatch != null) {
        final idx = int.parse(countryMatch.group(1)!);
        countries[idx] = countryMatch.group(2)!.trim();
        countryCodes[idx] = countryMatch.group(3)!.trim();
        continue;
      }

      // 匹配: 地址[0] 格式化结果: label | formatted
      // 或: 地址[0] 格式化结果: formatted
      final resultMatch = RegExp(r'地址\[(\d+)\] 格式化结果: (.+)').firstMatch(log);
      if (resultMatch != null) {
        final idx = int.parse(resultMatch.group(1)!);
        final rest = resultMatch.group(2)!.trim();
        if (rest.contains(' | ')) {
          final parts = rest.split(' | ');
          labels[idx] = parts[0].trim();
          formatted[idx] = parts.sublist(1).join(' | ').trim();
        } else {
          labels[idx] = '';
          formatted[idx] = rest;
        }
        continue;
      }
    }

    final allIndices = {...countries.keys, ...formatted.keys};
    final sortedIndices = allIndices.toList()..sort();

    return sortedIndices.map((idx) {
      final label = labels[idx];
      return _AddressResult(
        index: idx,
        label: label != null && label.isNotEmpty ? label : '地址 ${idx + 1}',
        formattedAddress: formatted[idx] ?? '',
        country: countries[idx] ?? '',
        countryCode: countryCodes[idx] ?? '',
      );
    }).toList();
  }

  Color _countryChipColor(String countryCode) {
    // 基于国家代码哈希生成稳定颜色，常用国家使用固定色
    final upper = countryCode.toUpperCase();
    switch (upper) {
      case 'CN':
        return Colors.red.shade100;
      case 'US':
        return Colors.blue.shade100;
      case 'GB':
      case 'UK':
        return Colors.indigo.shade100;
      case 'JP':
        return Colors.pink.shade100;
      case 'KR':
        return Colors.purple.shade100;
      case 'DE':
        return Colors.amber.shade100;
      case 'FR':
        return Colors.lightBlue.shade100;
      case 'AU':
        return Colors.green.shade100;
      case 'CA':
        return Colors.orange.shade100;
      default:
        // 基于哈希的稳定颜色
        final hue = (upper.codeUnits.fold<int>(0, (a, b) => a + b) * 37) % 360;
        return HSLColor.fromAHSL(1.0, hue.toDouble(), 0.6, 0.88).toColor();
    }
  }

  Color _countryTextColor(String countryCode) {
    final upper = countryCode.toUpperCase();
    switch (upper) {
      case 'CN':
        return Colors.red.shade900;
      case 'US':
        return Colors.blue.shade900;
      case 'GB':
      case 'UK':
        return Colors.indigo.shade900;
      case 'JP':
        return Colors.pink.shade900;
      case 'KR':
        return Colors.purple.shade900;
      case 'DE':
        return Colors.amber.shade900;
      case 'FR':
        return Colors.lightBlue.shade900;
      case 'AU':
        return Colors.green.shade900;
      case 'CA':
        return Colors.orange.shade900;
      default:
        return Colors.grey.shade800;
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final results = _parseResults();
    final allText = widget.logs.join('\n');

    return AlertDialog(
      insetPadding: EdgeInsets.symmetric(
        horizontal: MediaQuery.of(context).size.width * 0.12,
        vertical: 24,
      ),
      title: Row(
        children: [
          Icon(
            widget.hasErrors ? Icons.warning_amber_rounded : Icons.check_circle_outline,
            color: widget.hasErrors ? Colors.orange.shade700 : Colors.green.shade600,
          ),
          const SizedBox(width: 8),
          Expanded(child: Text('${widget.pluginName} 结果')),
          IconButton(
            icon: const Icon(Icons.close),
            onPressed: () => Navigator.of(context).pop(),
            visualDensity: VisualDensity.compact,
          ),
        ],
      ),
      content: SizedBox(
        width: double.maxFinite,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // === 执行日志区（可展开折叠，默认折叠，放在结果上方） ===
            Material(
              color: Colors.transparent,
              child: Theme(
                data: Theme.of(context).copyWith(
                  dividerColor: Colors.transparent,
                ),
                child: ExpansionTile(
                  title: Text(
                    '执行日志 (${widget.logs.length} 行)',
                    style: TextStyle(
                      fontSize: 13,
                      color: Theme.of(context).colorScheme.onSurfaceVariant,
                    ),
                  ),
                  tilePadding: const EdgeInsets.symmetric(horizontal: 4),
                  childrenPadding: EdgeInsets.zero,
                  initiallyExpanded: _logsExpanded,
                  onExpansionChanged: (expanded) => setState(() => _logsExpanded = expanded),
                  children: [
                    Container(
                      width: double.infinity,
                      constraints: const BoxConstraints(maxHeight: 300),
                      decoration: BoxDecoration(
                        color: Theme.of(context).colorScheme.surfaceContainerHighest.withValues(alpha: 0.5),
                        borderRadius: BorderRadius.circular(8),
                        border: Border.all(
                          color: Theme.of(context).colorScheme.outlineVariant.withValues(alpha: 0.5),
                        ),
                      ),
                      child: ClipRRect(
                        borderRadius: BorderRadius.circular(8),
                        child: Scrollbar(
                          thumbVisibility: true,
                          child: SingleChildScrollView(
                            padding: const EdgeInsets.all(12),
                            child: SelectableText(
                              allText,
                              style: TextStyle(
                                fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
                                fontSize: 12,
                                height: 1.5,
                                color: Theme.of(context).colorScheme.onSurface,
                              ),
                            ),
                          ),
                        ),
                      ),
                    ),
                    const SizedBox(height: 4),
                    Align(
                      alignment: Alignment.centerRight,
                      child: TextButton.icon(
                        onPressed: () {
                          Clipboard.setData(ClipboardData(text: allText));
                          ScaffoldMessenger.of(context).showSnackBar(
                            const SnackBar(
                              content: Text('已复制全部日志到剪贴板'),
                              duration: Duration(seconds: 1),
                            ),
                          );
                        },
                        icon: const Icon(Icons.copy_all, size: 16),
                        label: const Text('复制全部日志', style: TextStyle(fontSize: 12)),
                      ),
                    ),
                  ],
                ),
              ),
            ),

            const SizedBox(height: 12),

            // === 结果区 ===
            if (results.isNotEmpty)
              Flexible(
                child: ListView.separated(
                  shrinkWrap: true,
                  itemCount: results.length,
                  separatorBuilder: (_, __) => const Divider(height: 1),
                  itemBuilder: (context, i) {
                    final r = results[i];
                    return Padding(
                      padding: const EdgeInsets.symmetric(vertical: 10),
                      child: Row(
                        crossAxisAlignment: CrossAxisAlignment.center,
                        children: [
                          // Label Chip
                          Chip(
                            label: Text(r.label),
                            visualDensity: VisualDensity.compact,
                            backgroundColor: Theme.of(context).colorScheme.primaryContainer.withValues(alpha: 0.6),
                            side: BorderSide.none,
                            labelStyle: TextStyle(
                              fontSize: 12,
                              fontWeight: FontWeight.bold,
                              color: Theme.of(context).colorScheme.onPrimaryContainer,
                            ),
                          ),
                          const SizedBox(width: 10),
                          // 格式化地址
                          Expanded(
                            child: SelectableText(
                              r.formattedAddress,
                              style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                                height: 1.4,
                              ),
                            ),
                          ),
                          const SizedBox(width: 8),
                          // 国家 Chip
                          if (r.country.isNotEmpty)
                            Chip(
                              label: Text(r.country),
                              visualDensity: VisualDensity.compact,
                              backgroundColor: _countryChipColor(r.countryCode),
                              side: BorderSide.none,
                              labelStyle: TextStyle(
                                fontSize: 11,
                                fontWeight: FontWeight.w600,
                                color: _countryTextColor(r.countryCode),
                              ),
                              padding: EdgeInsets.zero,
                            ),
                          const SizedBox(width: 6),
                          // 复制按钮
                          IconButton(
                            icon: Icon(
                              Icons.copy,
                              size: 18,
                              color: Theme.of(context).colorScheme.primary,
                            ),
                            tooltip: '复制',
                            onPressed: () {
                              Clipboard.setData(
                                ClipboardData(text: r.formattedAddress),
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
              )
            else
              Container(
                width: double.infinity,
                padding: const EdgeInsets.symmetric(vertical: 24, horizontal: 16),
                decoration: BoxDecoration(
                  color: Theme.of(context).colorScheme.surfaceContainerHighest.withValues(alpha: 0.3),
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(
                    color: Theme.of(context).colorScheme.outlineVariant.withValues(alpha: 0.3),
                  ),
                ),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(
                      Icons.location_on_outlined,
                      size: 40,
                      color: Theme.of(context).colorScheme.onSurfaceVariant.withValues(alpha: 0.4),
                    ),
                    const SizedBox(height: 8),
                    Text(
                      '无结果返回',
                      style: TextStyle(
                        fontSize: 15,
                        fontWeight: FontWeight.w500,
                        color: Theme.of(context).colorScheme.onSurfaceVariant,
                      ),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      '具体细节请查看执行日志',
                      style: TextStyle(
                        fontSize: 12,
                        color: Theme.of(context).colorScheme.onSurfaceVariant.withValues(alpha: 0.7),
                      ),
                    ),
                  ],
                ),
              ),

            if (widget.hasErrors)
              Padding(
                padding: const EdgeInsets.only(top: 8),
                child: Row(
                  children: [
                    Icon(Icons.warning_amber_rounded, size: 14, color: Colors.orange.shade700),
                    const SizedBox(width: 4),
                    Expanded(
                      child: Text(
                        '插件执行过程中出现部分错误（exit: ${widget.exitCode}）',
                        style: TextStyle(
                          fontSize: 12,
                          color: Colors.orange.shade800,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.commonClose),
        ),
      ],
    );
  }
}

// ============================================================================
// Phase 2: 结构化结果卡片渲染系统
// ============================================================================

/// 插件结构化结果数据（从 solosoul_result 通道的 JSON 解析）
class _PluginResultData {
  final String type;
  final Map<String, dynamic> data;

  const _PluginResultData({required this.type, required this.data});

  factory _PluginResultData.fromJson(String jsonStr) {
    final json = jsonDecode(jsonStr) as Map<String, dynamic>;
    final type = json['type'] as String? ?? 'text';
    return _PluginResultData(type: type, data: json);
  }

  /// 生成适合复制的纯文本格式
  String toCopyText() {
    switch (type) {
      case 'text':
        return data['content'] as String? ?? '';
      case 'key_value':
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
typedef _ResultCardBuilder = Widget Function(BuildContext context, _PluginResultData result);

/// 渲染器注册表（策略模式）
final Map<String, _ResultCardBuilder> _resultCardRenderers = {
  'text': (context, result) => _TextResultCard(data: result.data),
  'key_value': (context, result) => _KeyValueResultCard(data: result.data),
  'table': (context, result) => _TableResultCard(data: result.data),
  'markdown': (context, result) => _MarkdownResultCard(data: result.data),
};

/// 纯文本结果卡片
class _TextResultCard extends StatelessWidget {
  final Map<String, dynamic> data;

  const _TextResultCard({required this.data});

  @override
  Widget build(BuildContext context) {
    final content = data['content'] as String? ?? '';
    return SelectableText(
      content,
      style: Theme.of(context).textTheme.bodyMedium?.copyWith(height: 1.5),
    );
  }
}

/// 键值对结果卡片
class _KeyValueResultCard extends StatelessWidget {
  final Map<String, dynamic> data;

  const _KeyValueResultCard({required this.data});

  @override
  Widget build(BuildContext context) {
    final title = data['title'] as String?;
    final pairs = (data['pairs'] as List<dynamic>?) ?? [];

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (title != null && title.isNotEmpty) ...[
          Text(
            title,
            style: Theme.of(context).textTheme.titleSmall?.copyWith(
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(height: 8),
        ],
        ...pairs.map<Widget>((p) {
          final pair = p as Map<String, dynamic>;
          final key = pair['key'] as String? ?? '';
          final value = pair['value'] as String? ?? '';
          return Padding(
            padding: const EdgeInsets.symmetric(vertical: 4),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  '$key: ',
                  style: const TextStyle(fontWeight: FontWeight.w600),
                ),
                Expanded(
                  child: SelectableText(value),
                ),
              ],
            ),
          );
        }),
      ],
    );
  }
}

/// 表格结果卡片
class _TableResultCard extends StatelessWidget {
  final Map<String, dynamic> data;

  const _TableResultCard({required this.data});

  @override
  Widget build(BuildContext context) {
    final headers = (data['headers'] as List<dynamic>?)?.cast<String>() ?? [];
    final rows = (data['rows'] as List<dynamic>?)?.cast<List<dynamic>>() ?? [];

    if (headers.isEmpty && rows.isEmpty) {
      return const Text('空表格');
    }

    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      child: DataTable(
        headingRowColor: WidgetStateProperty.all(
          Theme.of(context).colorScheme.primaryContainer.withValues(alpha: 0.3),
        ),
        border: TableBorder.all(
          color: Theme.of(context).colorScheme.outlineVariant.withValues(alpha: 0.3),
        ),
        columns: headers.map((h) => DataColumn(label: Text(h))).toList(),
        rows: rows.map((row) {
          final cells = row.cast<String?>();
          return DataRow(
            cells: cells.map((c) => DataCell(Text(c ?? ''))).toList(),
          );
        }).toList(),
      ),
    );
  }
}

/// Markdown 结果卡片（安全子集）
class _MarkdownResultCard extends StatelessWidget {
  final Map<String, dynamic> data;

  const _MarkdownResultCard({required this.data});

  @override
  Widget build(BuildContext context) {
    final content = data['content'] as String? ?? '';

    return MarkdownBody(
      data: content,
      selectable: true,
      styleSheet: MarkdownStyleSheet.fromTheme(Theme.of(context)).copyWith(
        p: Theme.of(context).textTheme.bodyMedium?.copyWith(height: 1.5),
      ),
      onTapLink: (text, href, title) {
        // 禁用外部链接跳转，仅展示提示
        if (href != null) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text('链接: $href'),
              duration: const Duration(seconds: 2),
              action: SnackBarAction(
                label: '复制',
                onPressed: () {
                  Clipboard.setData(ClipboardData(text: href));
                },
              ),
            ),
          );
        }
      },
    );
  }
}

/// 未知类型结果卡片（降级展示原始 JSON）
class _UnknownResultCard extends StatelessWidget {
  final Map<String, dynamic> data;

  const _UnknownResultCard({required this.data});

  @override
  Widget build(BuildContext context) {
    return SelectableText(
      jsonEncode(data),
      style: TextStyle(
        fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
        fontSize: 12,
        color: Theme.of(context).colorScheme.onSurfaceVariant,
      ),
    );
  }
}

// ============================================================================
// Plugin Result Dialog — 通用插件结果展示对话框
// ============================================================================

class _PluginResultDialog extends StatefulWidget {
  final String pluginName;
  final List<String> logs;
  final List<_PluginResultData> results;
  final int exitCode;
  final bool hasErrors;

  const _PluginResultDialog({
    required this.pluginName,
    required this.logs,
    this.results = const [],
    required this.exitCode,
    required this.hasErrors,
  });

  @override
  State<_PluginResultDialog> createState() => _PluginResultDialogState();
}

class _PluginResultDialogState extends State<_PluginResultDialog> {
  bool _logsExpanded = false;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final allText = widget.logs.join('\n');
    final hasResults = widget.results.isNotEmpty;

    return AlertDialog(
      insetPadding: EdgeInsets.symmetric(
        horizontal: MediaQuery.of(context).size.width * 0.15,
        vertical: 24,
      ),
      title: Row(
        children: [
          Icon(
            widget.hasErrors ? Icons.warning_amber_rounded : Icons.check_circle_outline,
            color: widget.hasErrors ? Colors.orange.shade700 : Colors.green.shade600,
          ),
          const SizedBox(width: 8),
          Expanded(child: Text('${widget.pluginName} 结果')),
          IconButton(
            icon: const Icon(Icons.close),
            onPressed: () => Navigator.of(context).pop(),
            visualDensity: VisualDensity.compact,
          ),
        ],
      ),
      content: SizedBox(
        width: double.maxFinite,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // === 执行日志区（可展开折叠，默认折叠，放在结果上方） ===
            Material(
              color: Colors.transparent,
              child: Theme(
                data: Theme.of(context).copyWith(
                  dividerColor: Colors.transparent,
                ),
                child: ExpansionTile(
                  title: Text(
                    '执行日志 (${widget.logs.length} 行)',
                    style: TextStyle(
                      fontSize: 13,
                      color: Theme.of(context).colorScheme.onSurfaceVariant,
                    ),
                  ),
                  tilePadding: const EdgeInsets.symmetric(horizontal: 4),
                  childrenPadding: EdgeInsets.zero,
                  initiallyExpanded: _logsExpanded,
                  onExpansionChanged: (expanded) => setState(() => _logsExpanded = expanded),
                  children: [
                    Container(
                      width: double.infinity,
                      constraints: const BoxConstraints(maxHeight: 300),
                      decoration: BoxDecoration(
                        color: Theme.of(context).colorScheme.surfaceContainerHighest.withValues(alpha: 0.5),
                        borderRadius: BorderRadius.circular(8),
                        border: Border.all(
                          color: Theme.of(context).colorScheme.outlineVariant.withValues(alpha: 0.5),
                        ),
                      ),
                      child: ClipRRect(
                        borderRadius: BorderRadius.circular(8),
                        child: Scrollbar(
                          thumbVisibility: true,
                          child: SingleChildScrollView(
                            padding: const EdgeInsets.all(12),
                            child: SelectableText(
                              allText,
                              style: TextStyle(
                                fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
                                fontSize: 12,
                                height: 1.5,
                                color: Theme.of(context).colorScheme.onSurface,
                              ),
                            ),
                          ),
                        ),
                      ),
                    ),
                    const SizedBox(height: 4),
                    Align(
                      alignment: Alignment.centerRight,
                      child: TextButton.icon(
                        onPressed: () {
                          Clipboard.setData(ClipboardData(text: allText));
                          ScaffoldMessenger.of(context).showSnackBar(
                            const SnackBar(
                              content: Text('已复制全部日志到剪贴板'),
                              duration: Duration(seconds: 1),
                            ),
                          );
                        },
                        icon: const Icon(Icons.copy_all, size: 16),
                        label: const Text('复制全部日志', style: TextStyle(fontSize: 12)),
                      ),
                    ),
                  ],
                ),
              ),
            ),

            const SizedBox(height: 12),

            // === 结果区 ===
            if (hasResults)
              Flexible(
                child: ListView.separated(
                  shrinkWrap: true,
                  itemCount: widget.results.length,
                  separatorBuilder: (_, __) => const SizedBox(height: 8),
                  itemBuilder: (context, index) {
                    final result = widget.results[index];
                    final builder = _resultCardRenderers[result.type];
                    final cardContent = builder != null
                        ? builder(context, result)
                        : _UnknownResultCard(data: result.data);

                    return _ResultCard(
                      result: result,
                      child: cardContent,
                    );
                  },
                ),
              )
            else if (widget.logs.isNotEmpty)
              Flexible(
                child: Container(
                  decoration: BoxDecoration(
                    color: Theme.of(context).colorScheme.surfaceContainerHighest.withValues(alpha: 0.3),
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(
                      color: Theme.of(context).colorScheme.outlineVariant.withValues(alpha: 0.3),
                    ),
                  ),
                  child: ClipRRect(
                    borderRadius: BorderRadius.circular(8),
                    child: Scrollbar(
                      thumbVisibility: true,
                      child: SingleChildScrollView(
                        padding: const EdgeInsets.all(12),
                        child: SelectableText(
                          allText,
                          style: TextStyle(
                            fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
                            fontSize: 13,
                            height: 1.6,
                            color: Theme.of(context).colorScheme.onSurface,
                          ),
                        ),
                      ),
                    ),
                  ),
                ),
              )
            else
              Container(
                width: double.infinity,
                padding: const EdgeInsets.symmetric(vertical: 24, horizontal: 16),
                decoration: BoxDecoration(
                  color: Theme.of(context).colorScheme.surfaceContainerHighest.withValues(alpha: 0.3),
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(
                    color: Theme.of(context).colorScheme.outlineVariant.withValues(alpha: 0.3),
                  ),
                ),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(
                      Icons.article_outlined,
                      size: 40,
                      color: Theme.of(context).colorScheme.onSurfaceVariant.withValues(alpha: 0.4),
                    ),
                    const SizedBox(height: 8),
                    Text(
                      '无结果返回',
                      style: TextStyle(
                        fontSize: 15,
                        fontWeight: FontWeight.w500,
                        color: Theme.of(context).colorScheme.onSurfaceVariant,
                      ),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      '具体细节请查看执行日志',
                      style: TextStyle(
                        fontSize: 12,
                        color: Theme.of(context).colorScheme.onSurfaceVariant.withValues(alpha: 0.7),
                      ),
                    ),
                  ],
                ),
              ),

            if (widget.hasErrors)
              Padding(
                padding: const EdgeInsets.only(top: 8),
                child: Row(
                  children: [
                    Icon(Icons.warning_amber_rounded, size: 14, color: Colors.orange.shade700),
                    const SizedBox(width: 4),
                    Expanded(
                      child: Text(
                        '插件执行过程中出现部分错误（exit: ${widget.exitCode}）',
                        style: TextStyle(
                          fontSize: 12,
                          color: Colors.orange.shade800,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.commonClose),
        ),
      ],
    );
  }
}

/// 结构化结果卡片容器（统一提供复制按钮和卡片样式）
class _ResultCard extends StatelessWidget {
  final _PluginResultData result;
  final Widget child;

  const _ResultCard({required this.result, required this.child});

  @override
  Widget build(BuildContext context) {
    return Container(
      width: double.infinity,
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surfaceContainerHighest.withValues(alpha: 0.4),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(
          color: Theme.of(context).colorScheme.outlineVariant.withValues(alpha: 0.4),
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // 卡片头部：类型标签 + 复制按钮
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
            decoration: BoxDecoration(
              color: Theme.of(context).colorScheme.primaryContainer.withValues(alpha: 0.3),
              borderRadius: const BorderRadius.vertical(top: Radius.circular(8)),
            ),
            child: Row(
              children: [
                Icon(
                  _typeIcon(result.type),
                  size: 16,
                  color: Theme.of(context).colorScheme.primary,
                ),
                const SizedBox(width: 6),
                Text(
                  _typeLabel(result.type),
                  style: TextStyle(
                    fontSize: 12,
                    fontWeight: FontWeight.w600,
                    color: Theme.of(context).colorScheme.primary,
                  ),
                ),
                const Spacer(),
                // 复制适合阅读的文本
                IconButton(
                  icon: const Icon(Icons.copy, size: 16),
                  tooltip: '复制结果',
                  visualDensity: VisualDensity.compact,
                  constraints: const BoxConstraints(),
                  padding: EdgeInsets.zero,
                  onPressed: () {
                    Clipboard.setData(ClipboardData(text: result.toCopyText()));
                    ScaffoldMessenger.of(context).showSnackBar(
                      const SnackBar(
                        content: Text('已复制结果到剪贴板'),
                        duration: Duration(seconds: 1),
                      ),
                    );
                  },
                ),
                // 复制原始 JSON（长按菜单）
                IconButton(
                  icon: const Icon(Icons.code, size: 16),
                  tooltip: '复制原始 JSON',
                  visualDensity: VisualDensity.compact,
                  constraints: const BoxConstraints(),
                  padding: EdgeInsets.zero,
                  onPressed: () {
                    Clipboard.setData(ClipboardData(text: jsonEncode(result.data)));
                    ScaffoldMessenger.of(context).showSnackBar(
                      const SnackBar(
                        content: Text('已复制原始 JSON 到剪贴板'),
                        duration: Duration(seconds: 1),
                      ),
                    );
                  },
                ),
              ],
            ),
          ),
          // 卡片内容
          Padding(
            padding: const EdgeInsets.all(12),
            child: child,
          ),
        ],
      ),
    );
  }

  IconData _typeIcon(String type) {
    switch (type) {
      case 'text':
        return Icons.text_snippet_outlined;
      case 'key_value':
        return Icons.format_list_bulleted;
      case 'table':
        return Icons.table_chart_outlined;
      case 'map':
        return Icons.map_outlined;
      case 'markdown':
        return Icons.text_format;
      default:
        return Icons.extension_outlined;
    }
  }

  String _typeLabel(String type) {
    switch (type) {
      case 'text':
        return '文本';
      case 'key_value':
        return '键值对';
      case 'table':
        return '表格';
      case 'map':
        return '地图';
      case 'markdown':
        return '富文本';
      default:
        return '未知类型';
    }
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
      if (!context.mounted) return;

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
        if (!context.mounted) return;
        if (!shouldContinue) return;
      }

      // 3. 执行安装
      final installer = await ref.read(initializedPluginInstallerProvider.future);
      if (!context.mounted) return;
      await installer.installFromArtifacts(artifacts);
      if (!context.mounted) return;
      ref.invalidate(installedPluginsProvider);
      ref.invalidate(pluginRegistryStateProvider);

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
    final List<String> pluginLogs = [];
    final List<String> errorMessages = [];
    final List<_PluginResultData> pluginResults = [];
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
          case frb_plugin.PluginEvent_Result(jsonData: final jsonData):
            // Phase 2: 收集结构化结果
            try {
              pluginResults.add(_PluginResultData.fromJson(jsonData));
            } on Exception catch (e) {
              // JSON 解析失败时降级为日志
              pluginLogs.add('[结果解析错误] $e');
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
            // 收集插件输出的 info 日志（排除预授权信号和空消息）
            if (level == 'info' && message.isNotEmpty && !message.startsWith('pre-consent|')) {
              pluginLogs.add(message);
            }
            // 收集插件错误日志
            if (level == 'error') {
              errorMessages.add(message);
            }
          case frb_plugin.PluginEvent_Completed(exitCode: final exitCode):
            // 记录最近使用时间
            final installer = await ref.read(initializedPluginInstallerProvider.future);
            await installer.recordLastUsed(pluginId);
            if (!context.mounted) break;

            final registryEntry = data.registry.plugins[pluginId];
            final locale = Localizations.localeOf(context).toString();
            final pluginName = resolvePluginI18n(
              registryEntry?.i18n, 'name', locale, _getManifest()?.name ?? pluginId,
            );

            if (pluginLogs.isNotEmpty) {
              // 有日志输出：弹出结果展示对话框
              // TODO(Phase 2): 待 solosoul_result 结构化通道 + key_value 卡片渲染器完成后，
              // 移除此特殊路由，所有插件统一使用 _PluginResultDialog。
              if (pluginId == 'com.solosoul.official.address-fmt') {
                await showDialog<void>(
                  context: context,
                  builder: (ctx) => _AddressFormatterResultDialog(
                    pluginName: pluginName,
                    logs: pluginLogs,
                    exitCode: exitCode,
                    hasErrors: exitCode != 0 && errorMessages.isNotEmpty,
                  ),
                );
              } else {
                await showDialog<void>(
                  context: context,
                  builder: (ctx) => _PluginResultDialog(
                    pluginName: pluginName,
                    logs: pluginLogs,
                    results: pluginResults,
                    exitCode: exitCode,
                    hasErrors: exitCode != 0 && errorMessages.isNotEmpty,
                  ),
                );
              }
            } else if (exitCode == 0) {
              // 无日志但执行成功：弹出执行完成确认对话框
              await showDialog<void>(
                context: context,
                builder: (ctx) => AlertDialog(
                  insetPadding: EdgeInsets.symmetric(
                    horizontal: MediaQuery.of(ctx).size.width * 0.25,
                    vertical: 24,
                  ),
                  title: Row(
                    children: [
                      Icon(Icons.check_circle, color: Colors.green.shade600),
                      const SizedBox(width: 8),
                      Expanded(child: Text(pluginName)),
                    ],
                  ),
                  content: Text(l10n.pluginRunSuccess),
                  actions: [
                    TextButton(
                      onPressed: () => Navigator.of(ctx).pop(),
                      child: Text(l10n.commonClose),
                    ),
                  ],
                ),
              );
            } else {
              // 执行失败：弹出错误对话框
              final errorMsg = errorMessages.isNotEmpty
                  ? errorMessages.join('\n')
                  : '插件执行失败 (exit: $exitCode)';
              await showDialog<void>(
                context: context,
                builder: (ctx) => AlertDialog(
                  insetPadding: EdgeInsets.symmetric(
                    horizontal: MediaQuery.of(ctx).size.width * 0.2,
                    vertical: 24,
                  ),
                  title: Row(
                    children: [
                      const Icon(Icons.error_outline, color: AppTheme.errorColor),
                      const SizedBox(width: 8),
                      Expanded(child: Text(pluginName)),
                    ],
                  ),
                  content: SelectableText(errorMsg),
                  actions: [
                    TextButton(
                      onPressed: () => Navigator.of(ctx).pop(),
                      child: Text(l10n.commonClose),
                    ),
                  ],
                ),
              );
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
        final installer = await ref.read(initializedPluginInstallerProvider.future);
        if (!context.mounted) return;
        await installer.uninstall(pluginId);
        if (!context.mounted) return;
        ref.invalidate(installedPluginsProvider);
        ref.invalidate(activeSessionsProvider);
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
