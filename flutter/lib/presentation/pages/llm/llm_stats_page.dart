import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';
import 'package:solosoul_flutter/presentation/providers/llm/llm_config_provider.dart';
import 'package:solosoul_flutter/presentation/providers/llm/llm_model_provider.dart';
import 'widgets/widgets.dart';

// =============================================================================
// LLM Usage Stats Page
// =============================================================================

/// Displays LLM usage statistics: token consumption, inference count,
/// model load times, per-model breakdown, daily sparkline trends,
/// and current backend configuration.
class LlmStatsPage extends ConsumerWidget {
  const LlmStatsPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final modelAsync = ref.watch(llmModelProvider);
    final configAsync = ref.watch(llmConfigProvider);
    final modelNotifier = ref.read(llmModelProvider.notifier);

    final sessionUsage = modelNotifier.sessionUsageCount;
    final sessionTokens = modelNotifier.sessionTotalTokens;
    final sessionPrompt = modelNotifier.sessionPromptTokens;
    final sessionCompletion = modelNotifier.sessionCompletionTokens;

    final accountUsage = modelNotifier.accountUsageCount;
    final accountTokens = modelNotifier.accountTotalTokens;

    final perModel = modelNotifier.perModelStats;
    final daily = modelNotifier.dailyStats;

    return Scaffold(
      appBar: SoloGlassAppBar(
        title: Text(AppLocalizations.of(context).llmStatsTitle),
        centerTitle: true,
      ),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          // Current model info
          LLMSectionTitle(
            title: AppLocalizations.of(context).llmStatsCurrentModel,
            theme: theme,
          ),
          LLMModelInfoCard(
            modelAsync: modelAsync,
            configAsync: configAsync,
            theme: theme,
          ),
          const SizedBox(height: 24),

          // Session stats
          LLMSectionTitle(
            title: AppLocalizations.of(context).llmStatsSessionStats,
            theme: theme,
          ),
          LLMStatsGrid(
            usageCount: sessionUsage,
            totalTokens: sessionTokens,
            promptTokens: sessionPrompt,
            completionTokens: sessionCompletion,
            modelUsages: modelNotifier.sessionPerModelStats,
            lastLoadTime: modelNotifier.lastLoadTime,
            lastUsedTime: modelNotifier.lastUsedTime,
            theme: theme,
          ),
          const SizedBox(height: 24),

          // Account lifetime stats
          LLMSectionTitle(
            title: AppLocalizations.of(context).llmStatsAccountStats,
            theme: theme,
          ),
          LLMAccountStatsCard(
            usageCount: accountUsage,
            totalTokens: accountTokens,
            modelUsages: modelNotifier.perModelStats,
            theme: theme,
          ),
          const SizedBox(height: 24),

          // Token breakdown (prompt vs completion)
          if (sessionTokens > 0 || accountTokens > 0) ...[
            LLMSectionTitle(
              title: AppLocalizations.of(context).llmStatsTokenBreakdown,
              theme: theme,
            ),
            LLMTokenBreakdownCard(
              sessionPrompt: sessionPrompt,
              sessionCompletion: sessionCompletion,
              accountPrompt: modelNotifier.accountPromptTokens,
              accountCompletion: modelNotifier.accountCompletionTokens,
              theme: theme,
            ),
            const SizedBox(height: 24),
          ],

          // Daily sparkline
          if (daily.isNotEmpty) ...[
            LLMSectionTitle(
              title: AppLocalizations.of(context).llmStatsDailyTrend,
              theme: theme,
            ),
            LLMDailySparklineCard(daily: daily, theme: theme),
            const SizedBox(height: 24),
          ],

          // Per-model usage
          if (perModel.isNotEmpty) ...[
            LLMSectionTitle(
              title: AppLocalizations.of(context).llmStatsModelUsage,
              theme: theme,
            ),
            LLMModelUsageCard(perModel: perModel, theme: theme),
            const SizedBox(height: 24),
          ],

          // Reset button
          OutlinedButton.icon(
            onPressed: () => _confirmReset(context, ref),
            icon: Icon(Icons.restart_alt, color: theme.colorScheme.error),
            label: Text(
              AppLocalizations.of(context).llmStatsReset,
              style: TextStyle(color: theme.colorScheme.error),
            ),
            style: OutlinedButton.styleFrom(
              side: BorderSide(color: theme.colorScheme.error),
              padding: const EdgeInsets.symmetric(vertical: 12),
            ),
          ),
        ],
      ),
    );
  }

  Future<void> _confirmReset(BuildContext context, WidgetRef ref) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(AppLocalizations.of(context).llmStatsReset),
        content: Text(AppLocalizations.of(context).llmStatsResetConfirm),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: Text(AppLocalizations.of(context).commonCancel),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            style: FilledButton.styleFrom(
              backgroundColor: Theme.of(ctx).colorScheme.error,
            ),
            child: Text(AppLocalizations.of(context).llmStatsReset),
          ),
        ],
      ),
    );

    if (confirmed == true) {
      await ref.read(llmModelProvider.notifier).resetStats();
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(AppLocalizations.of(context).llmStatsResetSuccess),
          ),
        );
      }
    }
  }
}
