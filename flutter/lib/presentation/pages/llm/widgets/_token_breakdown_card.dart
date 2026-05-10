import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';

// =============================================================================
// Token Breakdown Card
// =============================================================================

class LLMTokenBreakdownCard extends StatelessWidget {
  final int sessionPrompt;
  final int sessionCompletion;
  final int accountPrompt;
  final int accountCompletion;
  final ThemeData theme;

  const LLMTokenBreakdownCard({
    super.key,
    required this.sessionPrompt,
    required this.sessionCompletion,
    required this.accountPrompt,
    required this.accountCompletion,
    required this.theme,
  });

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Session
            Text(
              AppLocalizations.of(context).llmStatsSession,
              style: theme.textTheme.labelMedium?.copyWith(
                fontWeight: FontWeight.w600,
                color: theme.colorScheme.primary,
              ),
            ),
            const SizedBox(height: 8),
            _TokenBar(
              prompt: sessionPrompt,
              completion: sessionCompletion,
              promptColor: theme.colorScheme.primary,
              completionColor: theme.colorScheme.tertiary,
              theme: theme,
            ),
            const SizedBox(height: 16),
            // Account
            Text(
              AppLocalizations.of(context).llmStatsAccountTotal,
              style: theme.textTheme.labelMedium?.copyWith(
                fontWeight: FontWeight.w600,
                color: theme.colorScheme.primary,
              ),
            ),
            const SizedBox(height: 8),
            _TokenBar(
              prompt: accountPrompt,
              completion: accountCompletion,
              promptColor: theme.colorScheme.primary,
              completionColor: theme.colorScheme.tertiary,
              theme: theme,
            ),
          ],
        ),
      ),
    );
  }
}

// =============================================================================
// Token Bar
// =============================================================================

class _TokenBar extends StatelessWidget {
  final int prompt;
  final int completion;
  final Color promptColor;
  final Color completionColor;
  final ThemeData theme;

  const _TokenBar({
    required this.prompt,
    required this.completion,
    required this.promptColor,
    required this.completionColor,
    required this.theme,
  });

  @override
  Widget build(BuildContext context) {
    final total = prompt + completion;
    final promptRatio = total == 0 ? 0.0 : prompt / total;
    final completionRatio = total == 0 ? 0.0 : completion / total;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // Bar
        ClipRRect(
          borderRadius: BorderRadius.circular(4),
          child: SizedBox(
            height: 12,
            child: Row(
              children: [
                Expanded(
                  flex: (promptRatio * 1000).round(),
                  child: Container(color: promptColor),
                ),
                Expanded(
                  flex: (completionRatio * 1000).round(),
                  child: Container(color: completionColor),
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 6),
        // Legend
        Row(
          children: [
            _LegendDot(color: promptColor),
            const SizedBox(width: 4),
            Text(
              'Prompt ${_formatTokens(prompt)}',
              style: theme.textTheme.bodySmall,
            ),
            const SizedBox(width: 16),
            _LegendDot(color: completionColor),
            const SizedBox(width: 4),
            Text(
              'Completion ${_formatTokens(completion)}',
              style: theme.textTheme.bodySmall,
            ),
            const Spacer(),
            Text(
              AppLocalizations.of(
                context,
              ).llmStatsTotalFormatted(_formatTokens(total)),
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ],
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
// Legend Dot
// =============================================================================

class _LegendDot extends StatelessWidget {
  final Color color;
  const _LegendDot({required this.color});

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 8,
      height: 8,
      decoration: BoxDecoration(
        color: color,
        borderRadius: BorderRadius.circular(2),
      ),
    );
  }
}
