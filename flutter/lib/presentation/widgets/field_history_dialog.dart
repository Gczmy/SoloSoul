import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/models/field_history_models.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitive_value_widget.dart';
import 'package:solosoul_flutter/presentation/widgets/form_field_def.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/utils/format_relative_time.dart';
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart'
    show effectiveSensitivityProvider;
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart';

/// A generic dialog that displays field history with sensitivity-aware masking.
///
/// Accepts a list of [FormFieldDef] field definitions to determine sensitivity
/// levels for each field, and [FieldHistory] containing the history entries.
class FieldHistoryDialog extends StatelessWidget {
  final String title;
  final IconData icon;
  final List<FormFieldDef> fieldDefs;
  final FieldHistory? history;
  final String? fieldPrefix;

  const FieldHistoryDialog({
    super.key,
    required this.title,
    required this.icon,
    required this.fieldDefs,
    this.history,
    this.fieldPrefix,
  });

  /// Shows the history dialog.
  static Future<void> show({
    required BuildContext context,
    required String title,
    required IconData icon,
    required List<FormFieldDef> fieldDefs,
    required FieldHistory? history,
    String? fieldPrefix,
  }) {
    return showDialog(
      context: context,
      builder: (context) => FieldHistoryDialog(
        title: title,
        icon: icon,
        fieldDefs: fieldDefs,
        history: history,
        fieldPrefix: fieldPrefix,
      ),
    );
  }

  String _formatTimestamp(DateTime timestamp) => formatRelativeTime(timestamp);

  String _formatFullTimestamp(DateTime timestamp) {
    return '${timestamp.year}-${timestamp.month.toString().padLeft(2, '0')}-${timestamp.day.toString().padLeft(2, '0')} '
        '${timestamp.hour.toString().padLeft(2, '0')}:${timestamp.minute.toString().padLeft(2, '0')}';
  }

  /// Finds the field definition for a given key by stripping the prefix.
  ///
  /// Keys in history entries are in the format "prefix.fieldName" (e.g.,
  /// "contact.email", "idCard.number"). This method strips the prefix and
  /// matches against the fieldId in fieldDefs.
  FormFieldDef? _findFieldDef(String key) {
    final strippedKey = key.contains('.')
        ? key.substring(key.indexOf('.') + 1)
        : key;

    // First try exact match (some fieldDefs may include prefix)
    for (final def in fieldDefs) {
      if (def.fieldId == strippedKey || def.fieldId == key) {
        return def;
      }
    }
    return null;
  }

  /// Converts a stripped field key to a human-readable label.
  String _toDisplayLabel(String strippedKey) {
    // Convert camelCase to Title Case for display (e.g., "expiryDate" -> "Expiry Date")
    return strippedKey.replaceAllMapped(
      RegExp(r'([A-Z]|[0-9]+)'),
      (match) => match.group(0)!.isEmpty
          ? ''
          : (match.start == 0
              ? match.group(0)!
              : ' ${match.group(0)}'),
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final entries = history?.entries ?? [];
    final reversedEntries = entries.reversed.toList(); // Most recent first

    return AlertDialog(
      title: Row(
        children: [
          Icon(icon, size: 20),
          const SizedBox(width: 8),
          Text(title),
        ],
      ),
      content: SizedBox(
        width: MediaQuery.of(context).size.width * 0.85,
        height: MediaQuery.of(context).size.height * 0.6,
        child: entries.isEmpty
            ? Center(
                child: Text(
                  'No history available',
                  style: theme.textTheme.bodyMedium?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              )
            : ListView.builder(
                shrinkWrap: true,
                itemCount: reversedEntries.length,
                itemBuilder: (context, index) {
                  final entry = reversedEntries[index];
                  final isLatest = index == 0;

                  return _HistoryEntryTile(
                    entry: entry,
                    isLatest: isLatest,
                    fieldDefs: fieldDefs,
                    findFieldDef: _findFieldDef,
                    toDisplayLabel: _toDisplayLabel,
                    formatTimestamp: _formatTimestamp,
                    formatFullTimestamp: _formatFullTimestamp,
                    title: title,
                    fieldPrefix: fieldPrefix,
                  );
                },
              ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Close'),
        ),
      ],
    );
  }
}

class _HistoryEntryTile extends ConsumerWidget {
  final FieldHistoryEntry entry;
  final bool isLatest;
  final List<FormFieldDef> fieldDefs;
  final FormFieldDef? Function(String) findFieldDef;
  final String Function(String) toDisplayLabel;
  final String Function(DateTime) formatTimestamp;
  final String Function(DateTime) formatFullTimestamp;
  final String title;
  final String? fieldPrefix;

  const _HistoryEntryTile({
    required this.entry,
    required this.isLatest,
    required this.fieldDefs,
    required this.findFieldDef,
    required this.toDisplayLabel,
    required this.formatTimestamp,
    required this.formatFullTimestamp,
    required this.title,
    this.fieldPrefix,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    // Use provided fieldPrefix or fallback to deriving from title
    final prefix = fieldPrefix ?? title.toLowerCase().replaceAll(' ', '');

    return Container(
      margin: const EdgeInsets.only(bottom: 12),
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.3),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(
          color: isLatest
              ? AppTheme.primaryColor.withValues(alpha: 0.5)
              : theme.colorScheme.outline.withValues(alpha: 0.2),
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Header with timestamp
          Row(
            children: [
              if (isLatest)
                Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 6,
                    vertical: 2,
                  ),
                  decoration: BoxDecoration(
                    color: AppTheme.primaryColor.withValues(alpha: 0.1),
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Text(
                    'Latest',
                    style: theme.textTheme.labelSmall?.copyWith(
                      color: AppTheme.primaryColor,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ),
              if (isLatest) const SizedBox(width: 8),
              Text(
                formatTimestamp(entry.timestamp),
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
              const Spacer(),
              Text(
                formatFullTimestamp(entry.timestamp),
                textAlign: TextAlign.right,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.6),
                  fontSize: 11,
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          // Field values
          ...entry.values.entries.map((e) {
            final value = e.value;
            final strippedKey = e.key.contains('.')
                ? e.key.substring(e.key.indexOf('.') + 1)
                : e.key;
            final fieldDef = findFieldDef(e.key);
            final displayLabel = fieldDef?.label ?? toDisplayLabel(strippedKey);
            // Build full fieldId for sensitivity lookup
            final fieldId = '$prefix.$strippedKey';
            final sensitivity = ref.watch(effectiveSensitivityProvider(fieldId));

            return Padding(
              padding: const EdgeInsets.only(bottom: 4),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  SizedBox(
                    width: 100,
                    child: Text(
                      displayLabel,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                        fontWeight: FontWeight.w500,
                      ),
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: value.isNotEmpty
                        ? SensitiveValueWidget(
                            fieldId: e.key,
                            value: value,
                          )
                        : Text(
                            '(empty)',
                            style: theme.textTheme.bodyMedium?.copyWith(
                              fontStyle: FontStyle.italic,
                              color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.6),
                            ),
                          ),
                  ),
                  const SizedBox(width: 6),
                  SensitivityTag(level: sensitivity),
                ],
              ),
            );
          }),
        ],
      ),
    );
  }
}
