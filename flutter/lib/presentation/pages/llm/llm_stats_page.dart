import 'dart:math' as math;
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/llm/llm_config_models.dart';
import 'package:solosoul_flutter/core/services/llm/llm_model_state.dart';
import 'package:solosoul_flutter/core/services/llm/llm_service.dart';
import 'package:solosoul_flutter/core/services/llm/llm_usage_stats.dart';
import 'package:solosoul_flutter/presentation/providers/llm/llm_config_provider.dart';
import 'package:solosoul_flutter/presentation/providers/llm/llm_model_provider.dart';

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
          _SectionTitle(title: AppLocalizations.of(context).llmStatsCurrentModel, theme: theme),
          _ModelInfoCard(
            modelAsync: modelAsync,
            configAsync: configAsync,
            theme: theme,
          ),
          const SizedBox(height: 24),

          // Session stats
          _SectionTitle(title: AppLocalizations.of(context).llmStatsSessionStats, theme: theme),
          _StatsGrid(
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
          _SectionTitle(title: AppLocalizations.of(context).llmStatsAccountStats, theme: theme),
          _AccountStatsCard(
            usageCount: accountUsage,
            totalTokens: accountTokens,
            modelUsages: modelNotifier.perModelStats,
            theme: theme,
          ),
          const SizedBox(height: 24),

          // Token breakdown (prompt vs completion)
          if (sessionTokens > 0 || accountTokens > 0) ...[
            _SectionTitle(title: AppLocalizations.of(context).llmStatsTokenBreakdown, theme: theme),
            _TokenBreakdownCard(
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
            _SectionTitle(title: AppLocalizations.of(context).llmStatsDailyTrend, theme: theme),
            _DailySparklineCard(daily: daily, theme: theme),
            const SizedBox(height: 24),
          ],

          // Per-model usage
          if (perModel.isNotEmpty) ...[
            _SectionTitle(title: AppLocalizations.of(context).llmStatsModelUsage, theme: theme),
            _ModelUsageCard(perModel: perModel, theme: theme),
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
          SnackBar(content: Text(AppLocalizations.of(context).llmStatsResetSuccess)),
        );
      }
    }
  }
}

// =============================================================================
// Model Info Card
// =============================================================================

class _ModelInfoCard extends StatelessWidget {
  final AsyncValue<LlmModelState> modelAsync;
  final AsyncValue<LlmConfigState> configAsync;
  final ThemeData theme;

  const _ModelInfoCard({
    required this.modelAsync,
    required this.configAsync,
    required this.theme,
  });

  @override
  Widget build(BuildContext context) {
    String backendLabel = AppLocalizations.of(context).llmStatsUnknown;
    String modelName = '—';
    String providerLabel = '—';
    String statusLabel = AppLocalizations.of(context).llmStatsNotLoaded;
    Color statusColor = theme.colorScheme.outline;

    if (modelAsync.hasValue) {
      final state = modelAsync.value!;
      statusLabel = state.label;
      statusColor = switch (state) {
        LlmModelState.loaded => theme.colorScheme.primary,
        LlmModelState.loading => theme.colorScheme.tertiary,
        LlmModelState.error => theme.colorScheme.error,
        LlmModelState.unloaded => theme.colorScheme.outline,
      };
    }

    if (configAsync.hasValue) {
      final config = configAsync.value!;
      backendLabel = config.backendType == LlmBackendType.cloud
          ? AppLocalizations.of(context).llmConfigBackendCloud
          : AppLocalizations.of(context).llmStatsLocalModelOllama;
      if (config.backendType == LlmBackendType.cloud) {
        final profile = config.activeCloudProfile;
        if (profile != null) {
          modelName = profile.model;
          providerLabel = switch (profile.providerType) {
            LlmCloudProviderType.openai => 'OpenAI',
            LlmCloudProviderType.anthropic => 'Anthropic',
          };
        }
      } else {
        modelName = config.localModelPath ?? 'qwen2.5:1.5b';
        providerLabel = 'Ollama';
      }
    }

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(Icons.memory, color: theme.colorScheme.primary),
                const SizedBox(width: 8),
                Text(
                  backendLabel,
                  style: theme.textTheme.titleSmall?.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const Spacer(),
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                  decoration: BoxDecoration(
                    color: statusColor.withValues(alpha: 0.1),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Text(
                    statusLabel,
                    style: theme.textTheme.labelSmall?.copyWith(
                      color: statusColor,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 12),
            _InfoRow(label: AppLocalizations.of(context).llmStatsModelLabel, value: modelName),
            const SizedBox(height: 4),
            _InfoRow(label: AppLocalizations.of(context).llmStatsProviderLabel, value: providerLabel),
          ],
        ),
      ),
    );
  }
}

// =============================================================================
// Stats Grid (Session)
// =============================================================================

class _StatsGrid extends StatelessWidget {
  final int usageCount;
  final int totalTokens;
  final int promptTokens;
  final int completionTokens;
  final List<LlmModelUsage> modelUsages;
  final DateTime? lastLoadTime;
  final DateTime? lastUsedTime;
  final ThemeData theme;

  const _StatsGrid({
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
            children: [
              tile1,
              const SizedBox(height: 12),
              tile3,
            ],
          ),
        ),
        const SizedBox(width: 12),
        Expanded(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              tile2,
              const SizedBox(height: 12),
              tile4,
            ],
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
// Account Stats Card
// =============================================================================

class _AccountStatsCard extends StatelessWidget {
  final int usageCount;
  final int totalTokens;
  final List<LlmModelUsage> modelUsages;
  final ThemeData theme;

  const _AccountStatsCard({
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
                  label: AppLocalizations.of(context).llmStatsTotalConversations,
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
    // 动态高度：基础高度 + 每行模型高度，仅在超出最大高度时启用滚动
    const baseHeight = 92.0;
    const rowHeight = 12.0;
    const maxTileHeight = 140.0;
    final contentHeight = baseHeight + modelUsages.length * rowHeight;
    final tileHeight = contentHeight > maxTileHeight ? maxTileHeight : contentHeight;
    final needsScroll = contentHeight > maxTileHeight;

    Widget modelList() {
      final children = modelUsages.map((m) => Text(
        '${m.modelName} · ${m.provider} · ${modelValue(m)}',
        style: theme.textTheme.labelSmall?.copyWith(
          color: theme.colorScheme.onSurfaceVariant,
          fontSize: 10,
        ),
        overflow: TextOverflow.ellipsis,
        maxLines: 1,
        textAlign: TextAlign.center,
      )).toList();

      if (needsScroll) {
        return Expanded(
          child: ListView(
            padding: EdgeInsets.zero,
            children: children,
          ),
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

// =============================================================================
// Token Breakdown Card
// =============================================================================

class _TokenBreakdownCard extends StatelessWidget {
  final int sessionPrompt;
  final int sessionCompletion;
  final int accountPrompt;
  final int accountCompletion;
  final ThemeData theme;

  const _TokenBreakdownCard({
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
              AppLocalizations.of(context).llmStatsTotalFormatted(_formatTokens(total)),
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

// =============================================================================
// Daily Line Chart Card
// =============================================================================

class _DailySparklineCard extends StatelessWidget {
  final List<LlmDailyUsage> daily;
  final ThemeData theme;

  const _DailySparklineCard({
    required this.daily,
    required this.theme,
  });

  @override
  Widget build(BuildContext context) {
    final sorted = List<LlmDailyUsage>.from(daily)
      ..sort((a, b) => a.date.compareTo(b.date));
    final last14 = sorted.length > 14 ? sorted.sublist(sorted.length - 14) : sorted;
    if (last14.isEmpty) return const SizedBox.shrink();

    // 构建数据序列：优先按模型分线，否则画总 Token 线
    final series = <_Series>[];
    final allModels = last14
        .expand((d) => d.perModelTokens.keys)
        .toSet()
        .toList()
      ..sort();
    if (allModels.isNotEmpty) {
      for (final model in allModels) {
        series.add(_Series(
          name: model.split('/').last,
          values: last14.map((d) => (d.perModelTokens[model] ?? 0).toDouble()).toList(),
        ));
      }
    } else {
      series.add(_Series(
        name: AppLocalizations.of(context).llmStatsAllModels,
        values: last14.map((d) => d.totalTokens.toDouble()).toList(),
      ));
    }

    final colors = _chartColors(theme);
    final allValues = series.expand((s) => s.values);
    final rawMax = allValues.isEmpty ? 1.0 : allValues.reduce((a, b) => a > b ? a : b);
    final yMax = _niceMax(rawMax);

    return Card(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 16, 16, 12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            CustomPaint(
              size: const Size(double.infinity, 200),
              painter: _LineChartPainter(
                theme: theme,
                dates: last14.map((d) => d.date).toList(),
                series: series,
                colors: colors,
                yMax: yMax,
              ),
            ),
            const SizedBox(height: 12),
            // 图例
            Wrap(
              spacing: 16,
              runSpacing: 8,
              children: List.generate(series.length, (i) {
                return Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Container(
                      width: 10,
                      height: 3,
                      decoration: BoxDecoration(
                        color: colors[i % colors.length],
                        borderRadius: BorderRadius.circular(2),
                      ),
                    ),
                    const SizedBox(width: 6),
                    Text(
                      series[i].name,
                      style: theme.textTheme.labelSmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                );
              }),
            ),
          ],
        ),
      ),
    );
  }

  static List<Color> _chartColors(ThemeData theme) => [
    theme.colorScheme.primary,
    theme.colorScheme.secondary,
    theme.colorScheme.tertiary,
    Colors.orange,
    Colors.purple,
    Colors.teal,
    Colors.pink,
    Colors.indigo,
  ];

  static double _niceMax(double max) {
    if (max <= 0) return 1;
    final exponent = (math.log(max) / math.ln10).floor();
    final fraction = max / math.pow(10, exponent);
    double nice;
    if (fraction <= 1) {
      nice = 1;
    } else if (fraction <= 2) {
      nice = 2;
    } else if (fraction <= 5) {
      nice = 5;
    } else {
      nice = 10;
    }
    return nice * math.pow(10, exponent);
  }
}

class _Series {
  final String name;
  final List<double> values;
  _Series({required this.name, required this.values});
}

class _LineChartPainter extends CustomPainter {
  final ThemeData theme;
  final List<DateTime> dates;
  final List<_Series> series;
  final List<Color> colors;
  final double yMax;

  _LineChartPainter({
    required this.theme,
    required this.dates,
    required this.series,
    required this.colors,
    required this.yMax,
  });

  @override
  void paint(Canvas canvas, Size size) {
    const plotLeft = 56.0;
    final plotRight = size.width - 12;
    const plotTop = 8.0;
    final plotBottom = size.height - 28;
    final plotWidth = plotRight - plotLeft;
    final plotHeight = plotBottom - plotTop;

    final n = dates.length;
    if (n < 1 || plotWidth <= 0 || plotHeight <= 0) return;

    double xForIndex(int i) {
      if (n <= 1) return plotLeft + plotWidth / 2;
      return plotLeft + i * (plotWidth / (n - 1));
    }

    final gridPaint = Paint()
      ..color = theme.colorScheme.outlineVariant.withValues(alpha: 0.3)
      ..strokeWidth = 0.5;

    final labelStyle = TextStyle(
      color: theme.colorScheme.onSurfaceVariant,
      fontSize: 10,
    );

    // Y 轴：5 个刻度 + 网格线
    const yTicks = 5;
    for (int i = 0; i <= yTicks; i++) {
      final value = (i / yTicks) * yMax;
      final y = plotBottom - (i / yTicks) * plotHeight;
      canvas.drawLine(
        Offset(plotLeft, y),
        Offset(plotRight, y),
        gridPaint,
      );
      final tp = TextPainter(
        text: TextSpan(text: _formatY(value), style: labelStyle),
        textDirection: TextDirection.ltr,
        textAlign: TextAlign.right,
      )..layout(maxWidth: plotLeft - 4);
      tp.paint(canvas, Offset(plotLeft - 4 - tp.width, y - tp.height / 2));
    }

    // X 轴标签（最多显示 4-5 个，避免拥挤）
    final xLabelStyle = TextStyle(
      color: theme.colorScheme.onSurfaceVariant,
      fontSize: 10,
    );
    final xStep = math.max(1, (n / 4).ceil());
    for (int i = 0; i < n; i += xStep) {
      final x = xForIndex(i);
      final label = '${dates[i].month}/${dates[i].day}';
      final tp = TextPainter(
        text: TextSpan(text: label, style: xLabelStyle),
        textDirection: TextDirection.ltr,
        textAlign: TextAlign.center,
      )..layout();
      tp.paint(canvas, Offset(x - tp.width / 2, plotBottom + 6));
    }

    // 绘制每条折线
    for (int s = 0; s < series.length; s++) {
      final color = colors[s % colors.length];
      final linePaint = Paint()
        ..color = color
        ..strokeWidth = 2
        ..style = PaintingStyle.stroke
        ..strokeCap = StrokeCap.round
        ..strokeJoin = StrokeJoin.round;

      final pointPaint = Paint()
        ..color = color
        ..style = PaintingStyle.fill;

      final path = Path();
      for (int i = 0; i < n; i++) {
        final x = xForIndex(i);
        final y = plotBottom - (series[s].values[i] / yMax).clamp(0, yMax) * plotHeight;
        if (i == 0) {
          path.moveTo(x, y);
        } else {
          path.lineTo(x, y);
        }
        canvas.drawCircle(Offset(x, y), 2.5, pointPaint);
      }
      canvas.drawPath(path, linePaint);
    }
  }

  static String _formatY(double value) {
    if (value >= 1000000) return '${(value / 1000000).toStringAsFixed(1)}M';
    if (value >= 1000) return '${(value / 1000).toStringAsFixed(1)}K';
    return value.toStringAsFixed(0);
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => true;
}

// =============================================================================
// Model Usage Card
// =============================================================================

class _ModelUsageCard extends StatelessWidget {
  final List<LlmModelUsage> perModel;
  final ThemeData theme;

  const _ModelUsageCard({
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
              AppLocalizations.of(context).llmStatsModelSummary(perModel.length, _formatTokens(totalTokens)),
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
                      backgroundColor: theme.colorScheme.surfaceContainerHighest,
                      color: theme.colorScheme.secondary,
                    ),
                  ),
                  const SizedBox(height: 2),
                  Text(
                    AppLocalizations.of(context).llmStatsModelDetail(m.provider, _formatTokens(m.totalTokens), m.usageCount),
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

// =============================================================================
// Reusable Widgets
// =============================================================================

class _SectionTitle extends StatelessWidget {
  final String title;
  final ThemeData theme;

  const _SectionTitle({required this.title, required this.theme});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Text(
        title,
        style: theme.textTheme.titleSmall?.copyWith(
          color: theme.colorScheme.primary,
          fontWeight: FontWeight.w600,
        ),
      ),
    );
  }
}

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
    // 动态高度：仅在模型多到超出最大高度时才启用滚动
    const baseHeight = 84.0;
    const rowHeight = 12.0;
    const maxTileHeight = 140.0;
    final contentHeight = baseHeight + modelUsages.length * rowHeight;
    final tileHeight = contentHeight > maxTileHeight ? maxTileHeight : contentHeight;
    final needsScroll = contentHeight > maxTileHeight;

    Widget modelList() {
      final children = modelUsages.map((m) => Text(
        '${m.modelName} · ${m.provider} · ${modelValue(m)}',
        style: theme.textTheme.labelSmall?.copyWith(
          color: theme.colorScheme.onSurfaceVariant,
          fontSize: 10,
        ),
        overflow: TextOverflow.ellipsis,
        maxLines: 1,
      )).toList();

      if (needsScroll) {
        return Expanded(
          child: ListView(
            padding: EdgeInsets.zero,
            children: children,
          ),
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
                style: valueStyle ??
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

class _InfoRow extends StatelessWidget {
  final String label;
  final String value;

  const _InfoRow({required this.label, required this.value});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Row(
      children: [
        Text(
          '$label：',
          style: theme.textTheme.bodySmall?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
        Expanded(
          child: Text(
            value,
            style: theme.textTheme.bodyMedium,
            overflow: TextOverflow.ellipsis,
          ),
        ),
      ],
    );
  }
}
