import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'dart:convert' show jsonEncode;

import 'package:flutter_markdown_plus/flutter_markdown_plus.dart';

// ============================================================================
// Phase 2: 结构化结果卡片渲染系统
// ============================================================================

/// 纯文本结果卡片
class TextResultCard extends StatelessWidget {
  final Map<String, dynamic> data;

  const TextResultCard({super.key, required this.data});

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
class KeyValueResultCard extends StatelessWidget {
  final Map<String, dynamic> data;

  const KeyValueResultCard({super.key, required this.data});

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
class CalendarEventsResultCard extends StatelessWidget {
  final Map<String, dynamic> data;

  const CalendarEventsResultCard({super.key, required this.data});

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
class DataCompletenessResultCard extends StatelessWidget {
  final Map<String, dynamic> data;

  const DataCompletenessResultCard({super.key, required this.data});

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
class TableResultCard extends StatelessWidget {
  final Map<String, dynamic> data;

  const TableResultCard({super.key, required this.data});

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
class MarkdownResultCard extends StatelessWidget {
  final Map<String, dynamic> data;

  const MarkdownResultCard({super.key, required this.data});

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
class UnknownResultCard extends StatelessWidget {
  final Map<String, dynamic> data;

  const UnknownResultCard({super.key, required this.data});

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
