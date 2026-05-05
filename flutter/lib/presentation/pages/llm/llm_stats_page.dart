import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/llm/llm_model_state.dart';
import 'package:solosoul_flutter/core/services/llm/llm_service.dart';
import 'package:solosoul_flutter/presentation/providers/llm/llm_config_provider.dart';
import 'package:solosoul_flutter/presentation/providers/llm/llm_model_provider.dart';

// =============================================================================
// LLM Usage Stats Page
// =============================================================================

/// Displays LLM usage statistics: token consumption, inference count,
/// model load times, and current backend configuration.
class LlmStatsPage extends ConsumerWidget {
  const LlmStatsPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final modelAsync = ref.watch(llmModelProvider);
    final configAsync = ref.watch(llmConfigProvider);
    final modelNotifier = ref.read(llmModelProvider.notifier);

    return Scaffold(
      appBar: AppBar(
        title: const Text('使用统计'),
        centerTitle: true,
      ),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          // Current model info
          _SectionTitle(title: '当前模型', theme: theme),
          _ModelInfoCard(
            modelAsync: modelAsync,
            configAsync: configAsync,
            theme: theme,
          ),
          const SizedBox(height: 24),

          // Usage stats
          _SectionTitle(title: '使用统计', theme: theme),
          _StatsGrid(
            usageCount: modelNotifier.usageCount,
            totalTokens: modelNotifier.totalTokensUsed,
            lastLoadTime: modelNotifier.lastLoadTime,
            lastUsedTime: modelNotifier.lastUsedTime,
            theme: theme,
          ),
          const SizedBox(height: 24),

          // Token breakdown (only meaningful when > 0)
          if (modelNotifier.totalTokensUsed > 0) ...[
            _SectionTitle(title: 'Token 消耗', theme: theme),
            _TokenBar(
              totalTokens: modelNotifier.totalTokensUsed,
              theme: theme,
            ),
            const SizedBox(height: 24),
          ],

          // Reset button
          OutlinedButton.icon(
            onPressed: () => _confirmReset(context, ref),
            icon: Icon(Icons.restart_alt, color: theme.colorScheme.error),
            label: Text(
              '重置统计',
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
        title: const Text('重置统计'),
        content: const Text('确认重置所有使用统计吗？此操作不可撤销。'),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            style: FilledButton.styleFrom(
              backgroundColor: Theme.of(ctx).colorScheme.error,
            ),
            child: const Text('重置'),
          ),
        ],
      ),
    );

    if (confirmed == true) {
      await ref.read(llmModelProvider.notifier).resetStats();
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('统计已重置')),
        );
      }
    }
  }
}

// =============================================================================
// Model Info Card
// =============================================================================

class _ModelInfoCard extends StatelessWidget {
  final AsyncValue<dynamic> modelAsync;
  final AsyncValue<dynamic> configAsync;
  final ThemeData theme;

  const _ModelInfoCard({
    required this.modelAsync,
    required this.configAsync,
    required this.theme,
  });

  @override
  Widget build(BuildContext context) {
    String backendLabel = '未知';
    String modelName = '—';
    String providerLabel = '—';
    String statusLabel = '未加载';
    Color statusColor = theme.colorScheme.outline;

    if (modelAsync.hasValue) {
      final state = modelAsync.value! as LlmModelState;
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
      backendLabel = config.backendType == LlmBackendType.cloud ? '云端 API' : '本地模型 (Ollama)';
      if (config.backendType == LlmBackendType.cloud) {
        final profile = config.activeCloudProfile;
        if (profile != null) {
          modelName = profile.model;
          providerLabel = profile.providerType.label;
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
            _InfoRow(label: '模型', value: modelName),
            const SizedBox(height: 4),
            _InfoRow(label: '提供商', value: providerLabel),
          ],
        ),
      ),
    );
  }
}

// =============================================================================
// Stats Grid
// =============================================================================

class _StatsGrid extends StatelessWidget {
  final int usageCount;
  final int totalTokens;
  final DateTime? lastLoadTime;
  final DateTime? lastUsedTime;
  final ThemeData theme;

  const _StatsGrid({
    required this.usageCount,
    required this.totalTokens,
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

    return GridView.count(
      crossAxisCount: 2,
      shrinkWrap: true,
      physics: const NeverScrollableScrollPhysics(),
      crossAxisSpacing: 12,
      mainAxisSpacing: 12,
      childAspectRatio: 1.4,
      children: [
        _StatTile(
          icon: Icons.chat_bubble_outline,
          label: '对话次数',
          value: usageCount.toString(),
          theme: theme,
        ),
        _StatTile(
          icon: Icons.token,
          label: 'Token 消耗',
          value: _formatTokens(totalTokens),
          theme: theme,
        ),
        _StatTile(
          icon: Icons.download_done,
          label: '最后加载',
          value: formatDate(lastLoadTime),
          theme: theme,
          valueStyle: theme.textTheme.titleMedium,
        ),
        _StatTile(
          icon: Icons.schedule,
          label: '最后使用',
          value: formatDate(lastUsedTime),
          theme: theme,
          valueStyle: theme.textTheme.titleMedium,
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
// Token Bar
// =============================================================================

class _TokenBar extends StatelessWidget {
  final int totalTokens;
  final ThemeData theme;

  const _TokenBar({
    required this.totalTokens,
    required this.theme,
  });

  @override
  Widget build(BuildContext context) {
    // Rough cost estimates (per 1K tokens) for visualization only
    const usdPer1k = 0.002; // approximate average
    final estimatedCost = (totalTokens / 1000) * usdPer1k;

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(Icons.account_balance_wallet_outlined,
                    color: theme.colorScheme.secondary),
                const SizedBox(width: 8),
                Text(
                  '预估消耗',
                  style: theme.textTheme.titleSmall?.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 12),
            LinearProgressIndicator(
              value: 0.0, // static visual, no cap
              backgroundColor: theme.colorScheme.surfaceContainerHighest,
              color: theme.colorScheme.secondary,
              minHeight: 8,
              borderRadius: BorderRadius.circular(4),
            ),
            const SizedBox(height: 8),
            Text(
              '约 ${estimatedCost.toStringAsFixed(4)} USD（按平均 \$0.002/1K tokens 估算，仅供参照）',
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ),
      ),
    );
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
  final ThemeData theme;
  final TextStyle? valueStyle;

  const _StatTile({
    required this.icon,
    required this.label,
    required this.value,
    required this.theme,
    this.valueStyle,
  });

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Row(
              children: [
                Icon(icon, size: 18, color: theme.colorScheme.primary),
                const SizedBox(width: 6),
                Text(
                  label,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 8),
            Text(
              value,
              style: valueStyle ??
                  theme.textTheme.headlineSmall?.copyWith(
                    fontWeight: FontWeight.w700,
                    color: theme.colorScheme.onSurface,
                  ),
            ),
          ],
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
