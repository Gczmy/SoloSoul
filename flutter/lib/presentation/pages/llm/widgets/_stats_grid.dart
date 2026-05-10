import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/core/services/llm/llm_usage_stats.dart';

// =============================================================================
// Stats Grid (Session)
// =============================================================================

class LLMStatsGrid extends StatelessWidget {
  final int usageCount;
  final int totalTokens;
  final int promptTokens;
  final int completionTokens;
  final List<LlmModelUsage> modelUsages;
  final DateTime? lastLoadTime;
  final DateTime? lastUsedTime;
  final ThemeData theme;

  const LLMStatsGrid({
    super.key,
    required this.usageCount,
    required this.totalTokens,
    required this.promptTokens,
    required this.completionTokens,
    required this.modelUsages,
    required this.lastLoadTime,
    required this.lastUsedTime,
    required this.theme,
  });

  @override
  Widget build(BuildContext context) {
    String formatDate(DateTime? dt) {
      if (dt == null) return '—';
      final m = dt.month.toString().padLeft(2, '0');
      final d = dt.day.toString().padLeft(2, '0');
      final h = dt.hour.toString().padLeft(2, '0');
      final min = dt.minute.toString().padLeft(2, '0');
      return '$m-$d $h:$min';
    }

    final tile1 = _StatTile(
      icon: Icons.chat_bubble_outline,
      label: AppLocalizations.of(context).llmStatsConversationCount,
      value: usageCount.toString(),
      modelUsages: modelUsages,
      modelValue: (m) => m.usageCount.toString(),
      theme: theme,
    );
    final tile2 = _StatTile(
      icon: Icons.token,
      label: AppLocalizations.of(context).llmStatsTokenConsumption,
      value: _formatTokens(totalTokens),
      modelUsages: modelUsages,
      modelValue: (m) => _formatTokens(m.totalTokens),
      theme: theme,
    );
    final tile3 = _StatTile(
      icon: Icons.download_done,
      label: AppLocalizations.of(context).llmStatsLastLoaded,
      value: formatDate(lastLoadTime),
      modelUsages: modelUsages,
      modelValue: (m) => formatDate(m.lastLoadTime),
      theme: theme,
      valueStyle: theme.textTheme.titleMedium,
    );
    final tile4 = _StatTile(
      icon: Icons.schedule,
      label: AppLocalizations.of(context).llmStatsLastUsed,
      value: formatDate(lastUsedTime),
      modelUsages: modelUsages,
      modelValue: (m) => formatDate(m.lastUsedTime),
      theme: theme,
      valueStyle: theme.textTheme.titleMedium,
    );

    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Expanded(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [tile1, const SizedBox(height: 12), tile3],
          ),
        ),
        const SizedBox(width: 12),
        Expanded(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [tile2, const SizedBox(height: 12), tile4],
          ),
        ),
      ],
    );
  }

  static String _formatTokens(int n) {
    if (n >= 1000000) return '${(n / 1000000).toStringAsFixed(1)}M';
    if (n >= 1000) return '${(n / 1000).toStringAsFixed(1)}K';
    return n.toString();
  }
}

// =============================================================================
// Stat Tile
// =============================================================================

class _StatTile extends StatelessWidget {
  final IconData icon;
  final String label;
  final String value;
  final List<LlmModelUsage> modelUsages;
  final String Function(LlmModelUsage) modelValue;
  final ThemeData theme;
  final TextStyle? valueStyle;

  const _StatTile({
    required this.icon,
    required this.label,
    required this.value,
    required this.modelUsages,
    required this.modelValue,
    required this.theme,
    this.valueStyle,
  });

  @override
  Widget build(BuildContext context) {
    // Dynamic height: only enable scrolling when models exceed max height
    const baseHeight = 84.0;
    const rowHeight = 12.0;
    const maxTileHeight = 140.0;
    final contentHeight = baseHeight + modelUsages.length * rowHeight;
    final tileHeight = contentHeight > maxTileHeight
        ? maxTileHeight
        : contentHeight;
    final needsScroll = contentHeight > maxTileHeight;

    Widget modelList() {
      final children = modelUsages
          .map(
            (m) => Text(
              '${m.modelName} · ${m.provider} · ${modelValue(m)}',
              style: theme.textTheme.labelSmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
                fontSize: 10,
              ),
              overflow: TextOverflow.ellipsis,
              maxLines: 1,
            ),
          )
          .toList();

      if (needsScroll) {
        return Expanded(
          child: ListView(padding: EdgeInsets.zero, children: children),
        );
      }
      return Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: children,
      );
    }

    return SizedBox(
      height: tileHeight,
      child: Card(
        margin: EdgeInsets.zero,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(12, 10, 12, 6),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Icon(icon, size: 16, color: theme.colorScheme.primary),
                  const SizedBox(width: 6),
                  Text(
                    label,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 2),
              Text(
                value,
                style:
                    valueStyle ??
                    theme.textTheme.headlineSmall?.copyWith(
                      fontWeight: FontWeight.w700,
                      color: theme.colorScheme.onSurface,
                    ),
              ),
              const SizedBox(height: 4),
              if (modelUsages.isNotEmpty) modelList(),
            ],
          ),
        ),
      ),
    );
  }
}
