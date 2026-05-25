import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/models/semantic_type_registry.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';

/// 语义类型重复绑定警告弹窗。
///
/// 当用户尝试在同一个 section 中创建第二个相同语义类型的字段时弹出。
class SemanticTypeDuplicateWarning extends StatelessWidget {
  final String semanticTypeId;
  final String existingFieldLabel;
  final String languageCode;
  final VoidCallback onContinue;
  final VoidCallback onCancel;

  const SemanticTypeDuplicateWarning({
    super.key,
    required this.semanticTypeId,
    required this.existingFieldLabel,
    required this.languageCode,
    required this.onContinue,
    required this.onCancel,
  });

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final semanticType = SemanticTypeRegistry.getType(semanticTypeId);
    final typeLabel = semanticType?.getLabel(languageCode) ?? semanticTypeId;

    return AlertDialog(
      icon: const Icon(
        Icons.warning_amber,
        color: Colors.orange,
        size: 32,
      ),
      title: Text(l10n.semanticTypeDuplicateTitle),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            l10n.semanticTypeDuplicateMessage(typeLabel, existingFieldLabel),
            style: theme.textTheme.bodyMedium,
          ),
          const SizedBox(height: 12),
          Container(
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: theme.colorScheme.errorContainer.withValues(alpha: 0.3),
              borderRadius: BorderRadius.circular(8),
              border: Border.all(
                color: theme.colorScheme.error.withValues(alpha: 0.2),
              ),
            ),
            child: Row(
              children: [
                Icon(
                  Icons.info_outline,
                  size: 18,
                  color: theme.colorScheme.error,
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    l10n.semanticTypeDuplicateHint,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onErrorContainer,
                    ),
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: onCancel,
          child: Text(l10n.commonCancel),
        ),
        FilledButton(
          onPressed: onContinue,
          style: FilledButton.styleFrom(
            backgroundColor: theme.colorScheme.error,
          ),
          child: Text(l10n.semanticTypeDuplicateContinue),
        ),
      ],
    );
  }
}
