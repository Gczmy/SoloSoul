import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'dart:convert' show jsonEncode;

import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/pages/plugin_dashboard/plugin_result_cards.dart';
import 'package:solosoul_flutter/presentation/pages/plugin_dashboard_page.dart'
    show PluginResultData, resultCardRenderers, ResultCardBuilder;

// ============================================================================
// Plugin Result Dialog — 通用插件结果展示对话框
// ============================================================================

class PluginResultDialog extends StatefulWidget {
  final String pluginName;
  final List<String> logs;
  final List<PluginResultData> results;
  final int exitCode;
  final bool hasErrors;

  const PluginResultDialog({
    super.key,
    required this.pluginName,
    required this.logs,
    this.results = const [],
    required this.exitCode,
    required this.hasErrors,
  });

  @override
  State<PluginResultDialog> createState() => _PluginResultDialogState();
}

class _PluginResultDialogState extends State<PluginResultDialog> {
  bool _logsExpanded = false;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final allText = widget.logs.join('\n');

    return AlertDialog(
      insetPadding: EdgeInsets.symmetric(
        horizontal: MediaQuery.of(context).size.width * 0.15,
        vertical: 24,
      ),
      title: _buildTitle(),
      content: SizedBox(
        width: double.maxFinite,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            _buildLogsSection(context, allText),
            const SizedBox(height: 12),
            _buildResultsSection(context, allText),
            if (widget.hasErrors) _buildErrorBanner(),
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

  /// 对话框标题：状态图标 + 插件名 + 关闭按钮。
  Widget _buildTitle() {
    return Row(
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
    );
  }

  /// 执行日志区（可展开折叠）。
  Widget _buildLogsSection(BuildContext context, String allText) {
    return Material(
      color: Colors.transparent,
      child: Theme(
        data: Theme.of(context).copyWith(dividerColor: Colors.transparent),
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
    );
  }

  /// 结果展示区：结构化结果列表 / 纯日志 / 空状态。
  Widget _buildResultsSection(BuildContext context, String allText) {
    final hasResults = widget.results.isNotEmpty;

    if (hasResults) {
      return Flexible(
        child: ListView.separated(
          shrinkWrap: true,
          itemCount: widget.results.length,
          separatorBuilder: (_, __) => const SizedBox(height: 8),
          itemBuilder: (context, index) {
            final result = widget.results[index];
            final builder = resultCardRenderers[result.type];
            final cardContent = builder != null
                ? builder(context, result)
                : UnknownResultCard(data: result.data);

            return ResultCard(
              result: result,
              child: cardContent,
            );
          },
        ),
      );
    }

    if (widget.logs.isNotEmpty) {
      return Flexible(
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
      );
    }

    return Container(
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
    );
  }

  /// 错误提示横幅。
  Widget _buildErrorBanner() {
    return Padding(
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
    );
  }
}

/// 结构化结果卡片容器（统一提供复制按钮和卡片样式）
class ResultCard extends StatelessWidget {
  final PluginResultData result;
  final Widget child;

  const ResultCard({super.key, required this.result, required this.child});

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
