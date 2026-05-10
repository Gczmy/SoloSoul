import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/core/services/llm/llm_usage_stats.dart';

// =============================================================================
// Account Stats Card
// =============================================================================

class LLMAccountStatsCard extends StatelessWidget {
  final int usageCount;
  final int totalTokens;
  final List<LlmModelUsage> modelUsages;
  final ThemeData theme;

  const LLMAccountStatsCard({
    super.key,
    required this.usageCount,
    required this.totalTokens,
    required this.modelUsages,
    required this.theme,
  });

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 12, horizontal: 16),
        child: IntrinsicHeight(
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Expanded(
                child: _AccountStatItem(
                  icon: Icons.chat_bubble_outline,
                  label: AppLocalizations.of(
                    context,
                  ).llmStatsTotalConversations,
                  value: usageCount.toString(),
                  modelUsages: modelUsages,
                  modelValue: (m) => m.usageCount.toString(),
                  theme: theme,
                ),
              ),
              VerticalDivider(
                width: 1,
                color: theme.colorScheme.outlineVariant,
              ),
              Expanded(
                child: _AccountStatItem(
                  icon: Icons.token,
                  label: AppLocalizations.of(context).llmStatsTotalTokens,
                  value: _formatTokens(totalTokens),
                  modelUsages: modelUsages,
                  modelValue: (m) => _formatTokens(m.totalTokens),
                  theme: theme,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  static String _formatTokens(int n) {
    if (n >= 1000000) return '${(n / 1000000).toStringAsFixed(1)}M';
    if (n >= 1000) return '${(n / 1000).toStringAsFixed(1)}K';
    return n.toString();
  }
}

class _AccountStatItem extends StatelessWidget {
  final IconData icon;
  final String label;
  final String value;
  final List<LlmModelUsage> modelUsages;
  final String Function(LlmModelUsage) modelValue;
  final ThemeData theme;

  const _AccountStatItem({
    required this.icon,
    required this.label,
    required this.value,
    required this.modelUsages,
    required this.modelValue,
    required this.theme,
  });

  @override
  Widget build(BuildContext context) {
    // Dynamic height: base height + per-model height, only enable scroll when exceeding max
    const baseHeight = 92.0;
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
              textAlign: TextAlign.center,
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
        crossAxisAlignment: CrossAxisAlignment.center,
        children: children,
      );
    }

    return SizedBox(
      height: tileHeight,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          Icon(icon, size: 20, color: theme.colorScheme.primary),
          const SizedBox(height: 4),
          Text(
            value,
            style: theme.textTheme.headlineSmall?.copyWith(
              fontWeight: FontWeight.w700,
            ),
          ),
          const SizedBox(height: 2),
          Text(
            label,
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: 4),
          if (modelUsages.isNotEmpty) modelList(),
        ],
      ),
    );
  }
}
