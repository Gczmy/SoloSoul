import 'package:flutter/material.dart';
import 'package:solosoul_flutter/presentation/utils/format_relative_time.dart';
import 'package:solosoul_flutter/presentation/widgets/history_sheet.dart' show HistoryChangeItem;

/// Widget displaying a single history change entry.
class HistoryChangeTile extends StatelessWidget {
  final HistoryChangeItem change;
  final ThemeData theme;

  const HistoryChangeTile({
    super.key,
    required this.change,
    required this.theme,
  });

  String _formatTimestamp(DateTime timestamp) => formatRelativeTime(timestamp);

  String _formatFullTimestamp(DateTime timestamp) {
    return '${timestamp.year}-${timestamp.month.toString().padLeft(2, '0')}-${timestamp.day.toString().padLeft(2, '0')} '
        '${timestamp.hour.toString().padLeft(2, '0')}:${timestamp.minute.toString().padLeft(2, '0')}';
  }

  @override
  Widget build(BuildContext context) {
    return Card(
      margin: const EdgeInsets.only(bottom: 8),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    change.fieldId,
                    style: theme.textTheme.bodyMedium?.copyWith(
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                  const SizedBox(height: 4),
                  Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: change.values.entries.map((e) {
                      return Padding(
                        padding: const EdgeInsets.only(bottom: 2),
                        child: Row(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            SizedBox(
                              width: 80,
                              child: Text(
                                e.key,
                                style: theme.textTheme.bodySmall?.copyWith(
                                  color: theme.colorScheme.onSurfaceVariant,
                                  fontWeight: FontWeight.w500,
                                ),
                              ),
                            ),
                            Expanded(
                              child: Text(
                                e.value.isNotEmpty ? e.value : '(empty)',
                                style: theme.textTheme.bodyMedium?.copyWith(
                                  fontStyle: e.value.isEmpty ? FontStyle.italic : null,
                                  color: e.value.isEmpty
                                      ? theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.6)
                                      : null,
                                ),
                              ),
                            ),
                          ],
                        ),
                      );
                    }).toList(),
                  ),
                ],
              ),
            ),
            const SizedBox(width: 8),
            Tooltip(
              message: _formatFullTimestamp(change.timestamp),
              child: Text(
                _formatTimestamp(change.timestamp),
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}