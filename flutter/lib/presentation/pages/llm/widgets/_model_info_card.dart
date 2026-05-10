import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/core/services/llm/llm_backend_type.dart';
import 'package:solosoul_flutter/core/services/llm/llm_cloud_provider_type.dart';
import 'package:solosoul_flutter/core/services/llm/llm_config_models.dart';
import 'package:solosoul_flutter/core/services/llm/llm_model_state.dart';

// =============================================================================
// Model Info Card
// =============================================================================

class LLMModelInfoCard extends StatelessWidget {
  final AsyncValue<LlmModelState> modelAsync;
  final AsyncValue<LlmConfigState> configAsync;
  final ThemeData theme;

  const LLMModelInfoCard({
    super.key,
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
      final l10n = AppLocalizations.of(context);
      statusLabel = switch (state) {
        LlmModelState.unloaded => l10n.llmStatsNotLoaded,
        LlmModelState.loading => l10n.llmChatStatusLoading,
        LlmModelState.loaded => l10n.llmChatStatusReady,
        LlmModelState.error => l10n.llmChatStatusError,
      };
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
                  padding: const EdgeInsets.symmetric(
                    horizontal: 8,
                    vertical: 2,
                  ),
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
            _InfoRow(
              label: AppLocalizations.of(context).llmStatsModelLabel,
              value: modelName,
            ),
            const SizedBox(height: 4),
            _InfoRow(
              label: AppLocalizations.of(context).llmStatsProviderLabel,
              value: providerLabel,
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
