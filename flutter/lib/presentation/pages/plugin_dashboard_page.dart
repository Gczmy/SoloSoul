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
import 'package:solosoul_flutter/core/models/plugin_models.dart' show PluginRegistryEntry, resolvePluginI18n;
import 'package:solosoul_flutter/core/models/semantic_type_registry.dart';
import 'package:flutter_markdown/flutter_markdown.dart';
import 'package:solosoul_flutter/presentation/widgets/plugin_access_review_dialog.dart';
import 'package:solosoul_flutter/presentation/widgets/plugin_consent_dialog.dart';
import 'package:solosoul_flutter/presentation/widgets/plugin_detail_dialog.dart';
import 'package:solosoul_flutter/presentation/widgets/plugin_radio_list_dialog.dart';

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
typedef _ResultCardBuilder = Widget Function(BuildContext context, _PluginResultData result);

/// 渲染器注册表（策略模式）
final Map<String, _ResultCardBuilder> _resultCardRenderers = {
  'text': (context, result) => _TextResultCard(data: result.data),
  'key_value': (context, result) => _KeyValueResultCard(data: result.data),
  'table': (context, result) => _TableResultCard(data: result.data),
  'markdown': (context, result) => _MarkdownResultCard(data: result.data),
  'calendar_events': (context, result) => _CalendarEventsResultCard(data: result.data),
  'data_completeness': (context, result) => _DataCompletenessResultCard(data: result.data),
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

/// 键值对结果卡片（支持 tag、逐条复制、入场动画）
class _KeyValueResultCard extends StatelessWidget {
  final Map<String, dynamic> data;

  const _KeyValueResultCard({required this.data});

  Color _tagColor(String? code) {
    final upper = (code ?? '').toUpperCase();
    switch (upper) {
      case 'CN': return Colors.red.shade100;
      case 'US': return Colors.blue.shade100;
      case 'GB': case 'UK': return Colors.indigo.shade100;
      case 'JP': return Colors.pink.shade100;
      case 'KR': return Colors.purple.shade100;
      case 'DE': return Colors.amber.shade100;
      case 'FR': return Colors.lightBlue.shade100;
      case 'AU': return Colors.green.shade100;
      case 'CA': return Colors.orange.shade100;
      case 'SG': return Colors.teal.shade100;
      default:
        final hue = (upper.codeUnits.fold<int>(0, (a, b) => a + b) * 37) % 360;
        return HSLColor.fromAHSL(1.0, hue.toDouble(), 0.6, 0.88).toColor();
    }
  }

  Color _tagTextColor(String? code) {
    final upper = (code ?? '').toUpperCase();
    switch (upper) {
      case 'CN': return Colors.red.shade900;
      case 'US': return Colors.blue.shade900;
      case 'GB': case 'UK': return Colors.indigo.shade900;
      case 'JP': return Colors.pink.shade900;
      case 'KR': return Colors.purple.shade900;
      case 'DE': return Colors.amber.shade900;
      case 'FR': return Colors.lightBlue.shade900;
      case 'AU': return Colors.green.shade900;
      case 'CA': return Colors.orange.shade900;
      case 'SG': return Colors.teal.shade900;
      default: return Colors.grey.shade800;
    }
  }

  /// 插件结果 key 的本地化映射
  String _localizeKey(String key) {
    switch (key) {
      // 通用联系信息
      case 'Name': return '姓名';
      case 'Email': return '邮箱';
      case 'Phone': return '电话';
      case 'Website': return '网站';
      case 'Title': return '职位';
      case 'Organization': return '组织';
      case 'Address': return '地址';
      case 'Street': return '街道';
      case 'City': return '城市';
      case 'State': return '省份';
      case 'Postal Code': return '邮编';
      case 'Country': return '国家';
      case 'Field': return '字段';
      case 'Value': return '值';
      // resume-builder
      case 'LinkedIn': return 'LinkedIn';
      case '工作': return '工作';
      case '在职时间': return '在职时间';
      case '教育': return '教育';
      case '核心技能': return '核心技能';
      case '其他技能': return '其他技能';
      case '母语': return '母语';
      case '其他语言': return '其他语言';
      // tax-profile
      case '纳税人': return '纳税人';
      case '出生日期': return '出生日期';
      case '税务居民国': return '税务居民国';
      case '税号': return '税号';
      case '住址': return '住址';
      case '雇主': return '雇主';
      case '职位': return '职位';
      case '收入来源': return '收入来源';
      // totp-gen
      case '标签': return '标签';
      case '验证码': return '验证码';
      case '剩余时间': return '剩余时间';
      // travel-footprint
      case '国籍': return '国籍';
      case '到访国家数': return '到访国家数';
      case '签证数量': return '签证数量';
      case '亚洲': return '亚洲';
      case '欧洲': return '欧洲';
      case '北美洲': return '北美洲';
      case '大洋洲': return '大洋洲';
      case '南美洲': return '南美洲';
      case '非洲': return '非洲';
      case '其他': return '其他';
      // phone-fmt
      case '原始号码': return '原始号码';
      case '格式化后': return '格式化后';
      // emergency-card
      case '血型': return '血型';
      case '过敏': return '过敏';
      case '用药': return '用药';
      case '紧急联系人': return '紧急联系人';
      // id-validator
      case '证件类型': return '证件类型';
      case '校验结果': return '校验结果';
      case '脱敏号码': return '脱敏号码';
      // expiry-guardian
      case '证件列表': return '证件列表';
      case '剩余天数': return '剩余天数';
      case '紧急程度': return '紧急程度';
      // form-prefiller
      case '场景': return '场景';
      case '字段就绪状态': return '字段就绪状态';
      // doc-checklist
      case '场景名称': return '场景名称';
      case '材料项状态列表': return '材料项状态列表';
      // identity-timeline
      case '时间线事件列表': return '时间线事件列表';
      // mrz-encoder
      case 'MRZ 行': return 'MRZ 行';
      case '脱敏预览': return '脱敏预览';
      // digital-will
      case '立嘱人': return '立嘱人';
      case '资产': return '资产';
      case '数字账户': return '数字账户';
      default: return key;
    }
  }

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
          const SizedBox(height: 12),
        ],
        ...pairs.asMap().entries.map<Widget>((entry) {
          final index = entry.key;
          final pair = entry.value as Map<String, dynamic>;
          final rawKey = pair['key'] as String? ?? '';
          final key = _localizeKey(rawKey);
          final value = pair['value'] as String? ?? '';
          final tag = pair['tag'] as String?;
          final tagCode = pair['tagCode'] as String?;

          return Container(
            margin: const EdgeInsets.only(bottom: 10),
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
            decoration: BoxDecoration(
              color: Theme.of(context).colorScheme.surfaceContainerHighest.withValues(alpha: 0.35),
              borderRadius: BorderRadius.circular(10),
              border: Border.all(
                color: Theme.of(context).colorScheme.outlineVariant.withValues(alpha: 0.3),
              ),
            ),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.center,
              children: [
                // 标签 chip（空心 outline 风格）
                Chip(
                  label: Text(key),
                  visualDensity: VisualDensity.compact,
                  backgroundColor: Colors.transparent,
                  side: BorderSide(
                    color: Theme.of(context).colorScheme.outline.withValues(alpha: 0.5),
                    width: 1,
                  ),
                  labelStyle: TextStyle(
                    fontSize: 11,
                    fontWeight: FontWeight.w600,
                    color: Theme.of(context).colorScheme.onSurface.withValues(alpha: 0.7),
                  ),
                  padding: const EdgeInsets.symmetric(horizontal: 2),
                  materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                ),
                const SizedBox(width: 10),
                // 值
                Expanded(
                  child: SelectableText(
                    value,
                    style: Theme.of(context).textTheme.bodyMedium?.copyWith(height: 1.5),
                  ),
                ),
                const SizedBox(width: 8),
                // 国家 tag
                if (tag != null && tag.isNotEmpty)
                  Chip(
                    label: Text(tag, style: const TextStyle(fontSize: 11)),
                    visualDensity: VisualDensity.compact,
                    backgroundColor: _tagColor(tagCode),
                    side: BorderSide.none,
                    labelStyle: TextStyle(
                      fontSize: 11,
                      fontWeight: FontWeight.w600,
                      color: _tagTextColor(tagCode),
                    ),
                    padding: EdgeInsets.zero,
                  ),
                const SizedBox(width: 6),
                // 单条复制按钮
                IconButton(
                  icon: Icon(Icons.copy, size: 18, color: Theme.of(context).colorScheme.primary),
                  tooltip: '复制',
                  onPressed: () {
                    Clipboard.setData(ClipboardData(text: value));
                    ScaffoldMessenger.of(context).showSnackBar(
                      const SnackBar(content: Text('已复制到剪贴板'), duration: Duration(seconds: 1)),
                    );
                  },
                  visualDensity: VisualDensity.compact,
                  padding: EdgeInsets.zero,
                  constraints: const BoxConstraints(),
                ),
              ],
            ),
          ).animate().fadeIn(delay: (index * 80).ms, duration: 300.ms).slideY(
            begin: 0.15,
            end: 0,
            delay: (index * 80).ms,
            duration: 300.ms,
            curve: Curves.easeOutCubic,
          );
        }),
      ],
    );
  }
}

/// 日历事件结果卡片
class _CalendarEventsResultCard extends StatelessWidget {
  final Map<String, dynamic> data;

  const _CalendarEventsResultCard({required this.data});

  IconData _kindIcon(String? kind) {
    switch (kind) {
      case 'passport': return Icons.book_outlined;
      case 'visa': return Icons.fact_check_outlined;
      case 'idcard': return Icons.badge_outlined;
      case 'card': return Icons.credit_card_outlined;
      default: return Icons.event_outlined;
    }
  }

  Color _kindColor(String? kind) {
    switch (kind) {
      case 'passport': return const Color(0xFF1565C0);
      case 'visa': return const Color(0xFF6A1B9A);
      case 'idcard': return const Color(0xFF2E7D32);
      case 'card': return const Color(0xFFEF6C00);
      default: return Colors.grey;
    }
  }

  @override
  Widget build(BuildContext context) {
    final title = data['title'] as String? ?? '日历提醒事件';
    final eventCount = data['eventCount'] as int? ?? 0;
    final events = (data['events'] as List<dynamic>?) ?? [];
    final ics = data['ics'] as String? ?? '';

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // 标题 + 事件数量
        Row(
          children: [
            Expanded(
              child: Text(
                '$title ($eventCount)',
                style: Theme.of(context).textTheme.titleSmall?.copyWith(
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
            // 导出 ICS 按钮
            if (ics.isNotEmpty)
              TextButton.icon(
                onPressed: () {
                  Clipboard.setData(ClipboardData(text: ics));
                  ScaffoldMessenger.of(context).showSnackBar(
                    const SnackBar(content: Text('已复制 ICS 到剪贴板'), duration: Duration(seconds: 1)),
                  );
                },
                icon: const Icon(Icons.calendar_month, size: 16),
                label: const Text('复制 ICS', style: TextStyle(fontSize: 12)),
              ),
          ],
        ),
        const SizedBox(height: 12),
        // 事件列表
        ...events.asMap().entries.map<Widget>((entry) {
          final index = entry.key;
          final event = entry.value as Map<String, dynamic>;
          final kind = event['kind'] as String?;
          final summary = event['summary'] as String? ?? '';
          final date = event['date'] as String? ?? '';
          final alarmDays = event['alarmDays'] as int? ?? 0;
          final description = event['description'] as String? ?? '';
          final color = _kindColor(kind);

          return Container(
            margin: const EdgeInsets.only(bottom: 10),
            padding: const EdgeInsets.all(14),
            decoration: BoxDecoration(
              color: Theme.of(context).colorScheme.surfaceContainerHighest.withValues(alpha: 0.35),
              borderRadius: BorderRadius.circular(10),
              border: Border.all(
                color: Theme.of(context).colorScheme.outlineVariant.withValues(alpha: 0.3),
              ),
            ),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                // 图标
                Container(
                  width: 40,
                  height: 40,
                  decoration: BoxDecoration(
                    color: color.withValues(alpha: 0.12),
                    borderRadius: BorderRadius.circular(10),
                  ),
                  child: Icon(_kindIcon(kind), color: color, size: 22),
                ),
                const SizedBox(width: 12),
                // 内容
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          Expanded(
                            child: Text(
                              summary,
                              style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                                fontWeight: FontWeight.w600,
                              ),
                            ),
                          ),
                          // 提前提醒 badge
                          Container(
                            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                            decoration: BoxDecoration(
                              color: color.withValues(alpha: 0.1),
                              borderRadius: BorderRadius.circular(12),
                            ),
                            child: Text(
                              '$alarmDays 天前提醒',
                              style: TextStyle(
                                fontSize: 11,
                                color: color,
                                fontWeight: FontWeight.w500,
                              ),
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 4),
                      Text(
                        '到期日: $date',
                        style: TextStyle(
                          fontSize: 13,
                          color: Theme.of(context).colorScheme.onSurfaceVariant,
                        ),
                      ),
                      if (description.isNotEmpty) ...[
                        const SizedBox(height: 4),
                        Text(
                          description,
                          style: TextStyle(
                            fontSize: 12,
                            color: Theme.of(context).colorScheme.onSurfaceVariant.withValues(alpha: 0.8),
                          ),
                        ),
                      ],
                    ],
                  ),
                ),
              ],
            ),
          ).animate().fadeIn(delay: (index * 100).ms, duration: 300.ms).slideY(
            begin: 0.15,
            end: 0,
            delay: (index * 100).ms,
            duration: 300.ms,
            curve: Curves.easeOutCubic,
          );
        }),
      ],
    );
  }
}

/// 档案完整度结果卡片
class _DataCompletenessResultCard extends StatelessWidget {
  final Map<String, dynamic> data;

  const _DataCompletenessResultCard({required this.data});

  @override
  Widget build(BuildContext context) {
    final overall = data['overall'] as int? ?? 0;
    final sections = (data['sections'] as List<dynamic>?) ?? [];
    final message = data['message'] as String? ?? '';
    final color = overall >= 80
        ? Colors.green
        : overall >= 50
            ? Colors.orange
            : Colors.red;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // 总体完成度
        Container(
          width: double.infinity,
          padding: const EdgeInsets.all(16),
          decoration: BoxDecoration(
            color: color.withValues(alpha: 0.08),
            borderRadius: BorderRadius.circular(12),
            border: Border.all(color: color.withValues(alpha: 0.2)),
          ),
          child: Column(
            children: [
              Text(
                '$overall%',
                style: TextStyle(
                  fontSize: 36,
                  fontWeight: FontWeight.bold,
                  color: color,
                ),
              ),
              const SizedBox(height: 4),
              Text(
                '档案完整度',
                style: TextStyle(
                  fontSize: 13,
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(height: 12),
              // 总体进度条
              ClipRRect(
                borderRadius: BorderRadius.circular(4),
                child: LinearProgressIndicator(
                  value: overall / 100,
                  minHeight: 8,
                  backgroundColor: color.withValues(alpha: 0.15),
                  valueColor: AlwaysStoppedAnimation<Color>(color),
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: 16),
        // 各分区进度
        ...sections.asMap().entries.map<Widget>((entry) {
          final index = entry.key;
          final section = entry.value as Map<String, dynamic>;
          final name = section['name'] as String? ?? '';
          final icon = section['icon'] as String? ?? '📋';
          final percentage = section['percentage'] as int? ?? 0;
          final totalFields = section['totalFields'] as int? ?? 0;
          final filledFields = section['filledFields'] as int? ?? 0;
          final missing = (section['missing'] as List<dynamic>?)?.cast<String>() ?? [];
          final sectionColor = percentage >= 80
              ? Colors.green
              : percentage >= 50
                  ? Colors.orange
                  : Colors.red;

          return Container(
            margin: const EdgeInsets.only(bottom: 10),
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: Theme.of(context).colorScheme.surfaceContainerHighest.withValues(alpha: 0.3),
              borderRadius: BorderRadius.circular(10),
              border: Border.all(
                color: Theme.of(context).colorScheme.outlineVariant.withValues(alpha: 0.3),
              ),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Text(icon, style: const TextStyle(fontSize: 18)),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        name,
                        style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                    Text(
                      '$filledFields/$totalFields',
                      style: TextStyle(
                        fontSize: 12,
                        color: Theme.of(context).colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
                ClipRRect(
                  borderRadius: BorderRadius.circular(3),
                  child: LinearProgressIndicator(
                    value: (percentage / 100).clamp(0.0, 1.0),
                    minHeight: 6,
                    backgroundColor: sectionColor.withValues(alpha: 0.12),
                    valueColor: AlwaysStoppedAnimation<Color>(sectionColor),
                  ),
                ),
                if (missing.isNotEmpty) ...[
                  const SizedBox(height: 6),
                  Text(
                    '💡 建议补充: ${missing.join(", ")}',
                    style: TextStyle(
                      fontSize: 11,
                      color: Theme.of(context).colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ],
            ),
          ).animate().fadeIn(delay: (index * 80).ms, duration: 300.ms).slideY(
            begin: 0.1,
            end: 0,
            delay: (index * 80).ms,
            duration: 300.ms,
            curve: Curves.easeOutCubic,
          );
        }),
        if (message.isNotEmpty) ...[
          const SizedBox(height: 12),
          Container(
            width: double.infinity,
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: color.withValues(alpha: 0.06),
              borderRadius: BorderRadius.circular(8),
            ),
            child: Text(
              message,
              textAlign: TextAlign.center,
              style: TextStyle(
                fontSize: 13,
                color: color.withValues(alpha: 0.8),
                fontWeight: FontWeight.w500,
              ),
            ),
          ),
        ],
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
      case 'calendar_events':
        return Icons.event_outlined;
      case 'data_completeness':
        return Icons.data_usage_outlined;
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
      case 'calendar_events':
        return '日历事件';
      case 'data_completeness':
        return '档案完整度';
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
    final installingMap = ref.watch(pluginInstallingProvider);
    final isUpdating = installingMap[pluginId] ?? false;

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
              children: _buildActionButtons(context, ref, l10n, isInstalled, isRunning, hasUpdate, isUpdating),
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
    bool isUpdating,
  ) {
    final buttons = <Widget>[];

    if (!isInstalled) {
      // 未安装：显示安装按钮（安装中时显示 loading）
      final dash = ref.read(pluginDashboardProvider).asData?.value;
      if (dash != null && dash.registry.plugins.containsKey(pluginId)) {
        buttons.push(
          OutlinedButton.icon(
            onPressed: isUpdating ? null : () => _onInstall(context, ref),
            icon: isUpdating
                ? const SizedBox(
                    width: 14,
                    height: 14,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Icon(Icons.download_rounded, size: 16),
            label: Text(isUpdating ? '安装中' : l10n.pluginActionInstall),
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

      // 有更新：显示更新按钮（更新中时显示 loading）
      if (hasUpdate) {
        buttons.push(
          OutlinedButton.icon(
            onPressed: isUpdating ? null : () => _onUpdate(context, ref),
            icon: isUpdating
                ? const SizedBox(
                    width: 14,
                    height: 14,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Icon(Icons.update_rounded, size: 16),
            label: Text(isUpdating ? '更新中' : l10n.pluginActionUpdate),
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

    // 标记安装/更新中
    ref.read(pluginInstallingProvider.notifier).setLoading(pluginId, true);

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

      // 4. 局部更新安装状态，避免全页刷新
      final manifest = artifacts.toManifest();
      if (manifest != null) {
        ref.read(pluginDashboardProvider.notifier).addInstalledPlugin(manifest);
      }

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
    } finally {
      // 清除安装/更新中状态
      ref.read(pluginInstallingProvider.notifier).clear(pluginId);
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
                        subtitle: Builder(
                          builder: (context) {
                            final changelog = info.changelog;
                            return Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                Text(
                                  info.releasedAt.toLocal().toString().split(' ').first,
                                  style: TextStyle(
                                    fontSize: 12,
                                    color: Colors.grey.shade600,
                                  ),
                                ),
                                if (changelog != null && changelog.isNotEmpty)
                                  Padding(
                                    padding: const EdgeInsets.only(top: 4),
                                    child: Text(
                                      changelog,
                                      style: Theme.of(context).textTheme.bodySmall,
                                    ),
                                  ),
                              ],
                            );
                          },
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

    // 材料清单插件特殊处理：先进行场景选择，再传入参数执行
    Map<String, dynamic>? initialParams;
    if (pluginId == 'com.solosoul.official.doc-checklist') {
      final scenarioResult = await _showDocChecklistScenarioDialog(context);
      if (scenarioResult == null) return; // 用户取消，终止流程
      initialParams = {
        'scenario_id': scenarioResult['id'],
        'fields': scenarioResult['fields'],
      };
    }
    // 表单预填插件特殊处理：先进行场景选择，再传入参数执行
    if (pluginId == 'com.solosoul.official.form-prefiller') {
      if (!context.mounted) return;
      final scenarioResult = await _showFormPrefillerScenarioDialog(context);
      if (scenarioResult == null) return; // 用户取消，终止流程
      initialParams = {
        'scenario_id': scenarioResult['id'],
      };
    }

    final stream = runPlugin(ref, pluginId, params: initialParams);
    final List<String> pluginLogs = [];
    final List<String> errorMessages = [];
    final List<_PluginResultData> pluginResults = [];
    final batchRequests = <frb_plugin.PluginEvent_ConsentRequest>[];
    String? batchPluginName;

    // 延迟弹出结果对话框的状态
    int? completedExitCode;
    bool hasCompleted = false;

    // 对话框配置缓存（用于 solosoul_show_dialog）
    final dialogConfigs = <String, String>{};

    // 批量预授权阶段标记：true 表示可能还在批量预授权阶段（WASM 执行前）
    // 当 batch_end 到达或收到 WASM 执行期间的事件（Log/Result）时设为 false
    var batchPreConsentPhase = true;

    try {
      await for (final event in stream) {
        switch (event) {
          case frb_plugin.PluginEvent_ConsentRequest(
              requestId: final reqId,
              field: '__dialog__',
            ):
            // 处理插件通过 solosoul_show_dialog 请求的通用对话框
            debugPrint('[plugin_dialog] received __dialog__ ConsentRequest reqId=$reqId');

            // 场景选择前置优化：如果 Dart 端已传入场景参数，直接返回结果，
            // 避免插件再次弹出场景选择对话框。
            // （Rust 端 solosoul_get_param 在某些情况下无法读取到参数，作为兜底方案）
            if (initialParams != null && initialParams['scenario_id'] != null) {
              debugPrint('[plugin_dialog] skipping dialog, using pre-selected scenario: ${initialParams['scenario_id']}');
              await frb.frbPluginConsentResponse(
                requestId: reqId,
                approved: true,
                value: jsonEncode({'selected': initialParams['scenario_id']}),
              );
              break;
            }

            var configJson = dialogConfigs.remove(reqId);
            // 时序保护：dialog_config Log 事件可能晚于 ConsentRequest 到达，
            // 短暂等待确保配置已缓存（最多重试 10 次，每次 100ms = 1s）
            // Rust 端已发送 3 次 dialog_config（间隔 100ms），通常第一次就能命中
            for (var retry = 0; retry < 10 && configJson == null; retry++) {
              await Future.delayed(const Duration(milliseconds: 100));
              configJson = dialogConfigs.remove(reqId);
            }
            if (configJson == null) {
              debugPrint('[plugin_dialog] config NOT FOUND for reqId=$reqId after 2s, dialogConfigs keys=${dialogConfigs.keys.toList()}');
            }
            if (configJson == null || !context.mounted) {
              await frb.frbPluginConsentResponse(
                requestId: reqId,
                approved: false,
              );
              break;
            }
            try {
              final config = jsonDecode(configJson) as Map<String, dynamic>;
              final type = config['type'] as String?;
              if (type == 'radio_list') {
                final locale = Localizations.localeOf(context);
                String resolveL10n(dynamic raw) {
                  return switch (raw) {
                    Map<String, dynamic> map =>
                      map[locale.toString()] ??
                      map[locale.languageCode] ??
                      map['en'] ??
                      map.values.first as String,
                    String s => s,
                    _ => '',
                  };
                }

                final items = (config['items'] as List).map((e) {
                  final map = e as Map<String, dynamic>;
                  return PluginRadioItem(
                    id: map['id'] as String,
                    label: resolveL10n(map['label']),
                  );
                }).toList();

                final title = resolveL10n(config['title']);
                final description = config['description'] != null
                    ? resolveL10n(config['description'])
                    : null;

                final selected = await showDialog<String>(
                  context: context,
                  builder: (_) => PluginRadioListDialog(
                    title: title,
                    description: description,
                    items: items,
                  ),
                );

                await frb.frbPluginConsentResponse(
                  requestId: reqId,
                  approved: selected != null,
                  value: selected != null
                      ? jsonEncode({'selected': selected})
                      : null,
                );
              } else {
                await frb.frbPluginConsentResponse(
                  requestId: reqId,
                  approved: false,
                );
              }
            } on Exception catch (_) {
              await frb.frbPluginConsentResponse(
                requestId: reqId,
                approved: false,
              );
            }
          case frb_plugin.PluginEvent_ConsentRequest(
              requestId: final reqId,
              field: final field,
              sensitivity: final sensitivityStr,
              pluginName: final pname,
            ):
            if (batchPreConsentPhase) {
              // 批量预授权阶段：缓存请求，等待 batch_end 后统一弹窗
              batchRequests.add(event);
              if (batchPluginName == null) {
                final entry = data.registry.plugins[pluginId];
                if (!context.mounted) break;
                final locale = Localizations.localeOf(context).toString();
                batchPluginName = resolvePluginI18n(
                  entry?.i18n, 'name', locale, pname,
                );
              }
            } else {
              // WASM 执行期间的运行时单个授权：立即弹出授权对话框
              debugPrint('[plugin_consent] runtime single consent for field=$field');
              if (!context.mounted) {
                await frb.frbPluginConsentResponse(requestId: reqId, approved: false);
                break;
              }
              final approved = await showPluginConsentDialog(
                context: context,
                pluginId: pluginId,
                pluginName: batchPluginName ?? pname,
                fieldId: field,
                requestId: reqId,
                sensitivity: _parseSensitivity(sensitivityStr) ?? SensitivityLevel.sensitive,
              );
              await frb.frbPluginConsentResponse(
                requestId: reqId,
                approved: approved == true,
              );
            }
          case frb_plugin.PluginEvent_Result(jsonData: final jsonData):
            // WASM 已执行到发送结果阶段，批量预授权阶段一定已结束
            batchPreConsentPhase = false;
            // Phase 2: 收集结构化结果
            try {
              pluginResults.add(_PluginResultData.fromJson(jsonData));
            } on Exception catch (e) {
              // JSON 解析失败时，将原始 JSON 作为文本结果降级展示
              pluginResults.add(_PluginResultData(
                type: 'text',
                data: {'content': '结果解析失败: $e\n\n原始数据:\n$jsonData'},
              ));
              pluginLogs.add('[结果解析错误] $e');
            }
          case frb_plugin.PluginEvent_Log(level: final level, message: final message):
            // 缓存对话框配置（solosoul_show_dialog 通过 Log 事件传递配置）
            if (level == 'dialog_config') {
              final idx = message.indexOf('|');
              if (idx > 0) {
                final reqId = message.substring(0, idx);
                final config = message.substring(idx + 1);
                dialogConfigs[reqId] = config;
                debugPrint('[plugin_dialog] cached config for reqId=$reqId, config_len=${config.length}');
              } else {
                debugPrint('[plugin_dialog] invalid dialog_config format: $message');
              }
              break;
            }
            // 批量预授权结束信号：显示批量授权对话框
            if (level == 'batch_end') {
              batchPreConsentPhase = false;
              if (batchRequests.isNotEmpty) {
                if (!context.mounted) break;
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
            }
            // 收到 WASM 执行期间的日志，说明批量预授权阶段已结束
            if (level == 'info' || level == 'error') {
              batchPreConsentPhase = false;
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
            batchPreConsentPhase = false;
            // 记录最近使用时间，但不在此处弹出对话框（延迟到 stream 结束后）
            final installer = await ref.read(initializedPluginInstallerProvider.future);
            await installer.recordLastUsed(pluginId);
            completedExitCode = exitCode;
            hasCompleted = true;
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

      // Stream 完全结束后，统一弹出结果对话框。
      // 这样可以确保 Result 事件无论先于还是后于 Completed 到达，都会被收集到 pluginResults 中。
      if (hasCompleted && context.mounted) {
        final registryEntry = data.registry.plugins[pluginId];
        final locale = Localizations.localeOf(context).toString();
        final pluginName = resolvePluginI18n(
          registryEntry?.i18n, 'name', locale, _getManifest()?.name ?? pluginId,
        );
        final exitCode = completedExitCode!;

        if (pluginLogs.isNotEmpty || pluginResults.isNotEmpty) {
          // 有日志或结构化结果：弹出结果展示对话框
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

        // 清除 Riverpod 缓存，确保刷新时不再显示已卸载的插件
        ref.invalidate(installedPluginsProvider);

        // 局部更新安装状态，避免全页刷新
        ref.read(pluginDashboardProvider.notifier).removeInstalledPlugin(pluginId);

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

  /// 材料清单插件场景选择对话框
  /// 返回 {id, fields} 或 null（用户取消）
  Future<Map<String, dynamic>?> _showDocChecklistScenarioDialog(BuildContext context) async {
    final locale = Localizations.localeOf(context);

    // 场景定义（与 Rust 插件源码及 scenarios.json 保持一致）
    final scenarios = [
      {
        'id': 'japan-visa',
        'label': {'zh': '日本签证', 'en': 'Japan Visa'},
        'fields': ['passport.number', 'identity.idPhoto', 'employment.company', 'financial.bankStatement', 'travel.itinerary', 'travel.hotelBooking'],
      },
      {
        'id': 'us-visa',
        'label': {'zh': '美国签证 (B1/B2)', 'en': 'US Visa (B1/B2)'},
        'fields': ['passport.number', 'identity.idPhoto', 'visa.ds160Confirmation', 'visa.interviewAppointment', 'financial.bankStatement', 'employment.company'],
      },
      {
        'id': 'schengen-visa',
        'label': {'zh': '申根签证', 'en': 'Schengen Visa'},
        'fields': ['passport.number', 'identity.idPhoto', 'insurance.travel', 'travel.itinerary', 'travel.hotelBooking', 'financial.bankStatement', 'employment.company'],
      },
      {
        'id': 'uk-visa',
        'label': {'zh': '英国签证', 'en': 'UK Visa'},
        'fields': ['passport.number', 'identity.idPhoto', 'medical.tbTest', 'visa.casLetter', 'financial.bankStatement', 'travel.hotelBooking'],
      },
      {
        'id': 'bank-account',
        'label': {'zh': '银行开户', 'en': 'Bank Account'},
        'fields': ['passport.number', 'address.street', 'employment.company'],
      },
      {
        'id': 'hotel-checkin',
        'label': {'zh': '酒店入住', 'en': 'Hotel Check-in'},
        'fields': ['passport.number', 'travel.hotelBooking', 'card.number'],
      },
    ];

    String resolveL10n(Map<String, String> map) {
      return map[locale.toString()] ??
          map[locale.languageCode] ??
          map['en'] ??
          map.values.first;
    }

    final items = scenarios.map((s) {
      return PluginRadioItem(
        id: s['id'] as String,
        label: resolveL10n(s['label'] as Map<String, String>),
      );
    }).toList();

    final selected = await showDialog<String>(
      context: context,
      builder: (_) => PluginRadioListDialog(
        title: '选择签证/业务类型',
        description: '选择场景后，插件将请求访问相关字段，请继续授权。',
        items: items,
      ),
    );

    if (selected == null) return null;

    final scenario = scenarios.firstWhere((s) => s['id'] == selected);
    return {
      'id': selected,
      'fields': scenario['fields'],
    };
  }

  /// 表单预填插件场景选择对话框
  /// 返回 {id} 或 null（用户取消）
  Future<Map<String, dynamic>?> _showFormPrefillerScenarioDialog(BuildContext context) async {
    final locale = Localizations.localeOf(context);

    final scenarios = [
      {
        'id': 'visa-application',
        'label': {'zh': '签证申请表', 'en': 'Visa Application'},
      },
      {
        'id': 'hotel-checkin',
        'label': {'zh': '酒店入住', 'en': 'Hotel Check-in'},
      },
      {
        'id': 'bank-account',
        'label': {'zh': '银行开户', 'en': 'Bank Account'},
      },
      {
        'id': 'airline-checkin',
        'label': {'zh': '航空值机', 'en': 'Airline Check-in'},
      },
    ];

    String resolveL10n(Map<String, String> map) {
      return map[locale.toString()] ??
          map[locale.languageCode] ??
          map['en'] ??
          map.values.first;
    }

    final items = scenarios.map((s) {
      return PluginRadioItem(
        id: s['id'] as String,
        label: resolveL10n(s['label'] as Map<String, String>),
      );
    }).toList();

    final selected = await showDialog<String>(
      context: context,
      builder: (_) => PluginRadioListDialog(
        title: '选择表单场景',
        description: '选择场景后，插件将生成 Vault 字段到表单字段的映射表。',
        items: items,
      ),
    );

    if (selected == null) return null;
    return {'id': selected};
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
