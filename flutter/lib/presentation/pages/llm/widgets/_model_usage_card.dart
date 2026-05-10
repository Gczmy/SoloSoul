import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/core/services/llm/llm_usage_stats.dart';

// =============================================================================
// Model Usage Card
// =============================================================================

class LLMModelUsageCard extends StatelessWidget {
  final List<LlmModelUsage> perModel;
  final ThemeData theme;

  const LLMModelUsageCard({
    super.key,
    required this.perModel,
    required this.theme,
  });

  @override
  Widget build(BuildContext context) {
    final totalTokens = perModel.fold<int>(0, (sum, m) => sum + m.totalTokens);

    return Card(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 16, 16, 8),
            child: Text(
              AppLocalizations.of(context).llmStatsModelSummary(
                perModel.length,
                _formatTokens(totalTokens),
              ),
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ),
          ...perModel.map((m) {
            final ratio = totalTokens == 0 ? 0.0 : m.totalTokens / totalTokens;
            return Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Expanded(
                        child: Text(
                          m.modelName,
                          style: theme.textTheme.bodyMedium?.copyWith(
                            fontWeight: FontWeight.w600,
                          ),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                      const SizedBox(width: 8),
                      Text(
                        '${(ratio * 100).toStringAsFixed(1)}%',
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 4),
                  ClipRRect(
                    borderRadius: BorderRadius.circular(3),
                    child: LinearProgressIndicator(
                      value: ratio,
                      minHeight: 6,
                      backgroundColor:
                          theme.colorScheme.surfaceContainerHighest,
                      color: theme.colorScheme.secondary,
                    ),
                  ),
                  const SizedBox(height: 2),
                  Text(
                    AppLocalizations.of(context).llmStatsModelDetail(
                      m.provider,
                      _formatTokens(m.totalTokens),
                      m.usageCount,
                    ),
                    style: theme.textTheme.labelSmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ),
            );
          }),
          const SizedBox(height: 8),
        ],
      ),
    );
  }

  static String _formatTokens(int n) {
    if (n >= 1000000) return '${(n / 1000000).toStringAsFixed(1)}M';
    if (n >= 1000) return '${(n / 1000).toStringAsFixed(1)}K';
    return n.toString();
  }
}
