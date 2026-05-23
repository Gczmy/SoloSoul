import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:solosoul_flutter/core/router/app_router.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/ocr_scanner_llm_option.dart';

/// LLM assist section widget for OCR scanner.
class OcrScannerLlmSection extends StatelessWidget {
  final bool useLlmAssist;
  final List<OcrScannerLlmOption> modelOptions;
  final bool isCheckingModels;
  final String? selectedModelId;
  final ValueChanged<bool?> onLlmAssistChanged;
  final ValueChanged<String?> onModelChanged;

  const OcrScannerLlmSection({
    super.key,
    required this.useLlmAssist,
    required this.modelOptions,
    required this.isCheckingModels,
    required this.selectedModelId,
    required this.onLlmAssistChanged,
    required this.onModelChanged,
  });

  @override
  Widget build(BuildContext context) {
    final hasModels = modelOptions.isNotEmpty;

    return Card(
      margin: EdgeInsets.zero,
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            CheckboxListTile(
              contentPadding: EdgeInsets.zero,
              controlAffinity: ListTileControlAffinity.leading,
              title: Text(AppLocalizations.of(context).ocrLlmAssist),
              subtitle: Text(AppLocalizations.of(context).ocrLlmAssistSubtitle),
              value: useLlmAssist,
              onChanged: onLlmAssistChanged,
            ),
            if (useLlmAssist) ...[
              const Divider(height: 1),
              const SizedBox(height: 8),
              if (isCheckingModels)
                const Padding(
                  padding: EdgeInsets.all(8),
                  child: Center(
                    child: SizedBox(
                      height: 24,
                      width: 24,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    ),
                  ),
                )
              else if (!hasModels)
                _buildNoModelState(context)
              else
                _buildModelSelector(context),
            ],
            Align(
              alignment: Alignment.centerRight,
              child: TextButton.icon(
                onPressed: () => context.push(AppRoutes.llmConfig),
                icon: const Icon(Icons.settings, size: 16),
                label: Text(AppLocalizations.of(context).ocrLlmConfig),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildNoModelState(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(
                Icons.error_outline,
                size: 16,
                color: Theme.of(context).colorScheme.error,
              ),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  AppLocalizations.of(context).ocrNoModelAvailable,
                  style: TextStyle(
                    color: Theme.of(context).colorScheme.error,
                    fontSize: 13,
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          FilledButton.tonalIcon(
            onPressed: () => context.push(AppRoutes.llmConfig),
            icon: const Icon(Icons.arrow_forward, size: 16),
            label: Text(AppLocalizations.of(context).ocrGoToConfig),
          ),
        ],
      ),
    );
  }

  Widget _buildModelSelector(BuildContext context) {
    return DropdownButtonFormField<String>(
      initialValue: selectedModelId,
      isExpanded: true,
      decoration: InputDecoration(
        labelText: AppLocalizations.of(context).ocrModelSelectorLabel,
        border: const OutlineInputBorder(),
        contentPadding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      ),
      items: modelOptions.map((option) {
        return DropdownMenuItem<String>(
          value: option.id,
          child: Row(
            children: [
              Icon(
                Icons.circle,
                size: 8,
                color: option.isAvailable ? Colors.green : Colors.red,
              ),
              const SizedBox(width: 8),
              Icon(
                option.isLocal ? Icons.computer : Icons.cloud,
                size: 16,
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  option.displayName,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(
                    color: option.isAvailable
                        ? Theme.of(context).colorScheme.onSurface
                        : Theme.of(context).colorScheme.onSurfaceVariant,
                  ),
                ),
              ),
            ],
          ),
        );
      }).toList(),
      onChanged: onModelChanged,
    );
  }
}
