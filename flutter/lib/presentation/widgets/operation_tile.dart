import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/utils/format_field_label.dart';
import '../models/operation_log_models.dart';

class OperationTile extends StatelessWidget {
  final OperationEntry entry;

  const OperationTile({super.key, required this.entry});

  void _showDetailDialog(BuildContext context) {
    final hasProperties = entry.properties != null && entry.properties!.isNotEmpty;

    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: Row(
          children: [
            Icon(_actionIcon, color: _actionColor(context)),
            const SizedBox(width: 8),
            const Expanded(child: Text('Operation Details')),
          ],
        ),
        content: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              _DetailRow(label: 'Timestamp', value: _formatFullTimestamp(entry.timestamp)),
              const SizedBox(height: 12),
              _DetailRow(label: 'Action', value: _actionLabel),
              const SizedBox(height: 12),
              _DetailRow(label: 'Section', value: entry.section.toUpperCase()),
              if (entry.fieldPath != null) ...[
                const SizedBox(height: 12),
                _DetailRow(label: 'Field Path', value: entry.fieldPath!),
              ],
              const SizedBox(height: 12),
              _DetailRow(label: 'Description', value: entry.description),
              const SizedBox(height: 12),
              _DetailRow(label: 'Device', value: _getDeviceLabel(entry.device)),
              if (hasProperties) ...[
                const SizedBox(height: 16),
                const Divider(),
                const SizedBox(height: 8),
                Text(
                  'Property Snapshot',
                  style: Theme.of(context).textTheme.titleSmall?.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: 8),
                ...entry.properties!.entries.map((e) {
                  final levelName = entry.propertyLevels?[e.key];
                  final level = levelName != null
                      ? SensitivityLevel.values.firstWhere(
                          (l) => l.name == levelName,
                          orElse: () => SensitivityLevel.public,
                        )
                      : SensitivityLevel.public;
                  final levelColor = _sensitivityColor(level);
                  return Padding(
                    padding: const EdgeInsets.only(bottom: 8),
                    child: Row(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Expanded(
                          flex: 2,
                          child: Text(
                            formatFieldLabel(e.key),
                            style: Theme.of(context).textTheme.bodySmall?.copyWith(
                              color: Theme.of(context).colorScheme.onSurfaceVariant,
                            ),
                          ),
                        ),
                        Expanded(
                          flex: 3,
                          child: Text(
                            e.value.isEmpty ? '(empty)' : e.value,
                            style: Theme.of(context).textTheme.bodyMedium,
                          ),
                        ),
                        Container(
                          padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                          decoration: BoxDecoration(
                            color: levelColor.withValues(alpha: 0.1),
                            borderRadius: BorderRadius.circular(4),
                            border: Border.all(
                              color: levelColor.withValues(alpha: 0.3),
                            ),
                          ),
                          child: Text(
                            level.label,
                            style: Theme.of(context).textTheme.labelSmall?.copyWith(
                              color: levelColor,
                              fontWeight: FontWeight.w500,
                            ),
                          ),
                        ),
                      ],
                    ),
                  );
                }),
              ],
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Close'),
          ),
        ],
      ),
    );
  }

  String _formatFullTimestamp(DateTime dt) {
    return '${dt.year}-${dt.month.toString().padLeft(2, '0')}-${dt.day.toString().padLeft(2, '0')} '
        '${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}:${dt.second.toString().padLeft(2, '0')}';
  }

  IconData get _actionIcon {
    switch (entry.action) {
      case 'create':
        return Icons.add_circle_outline;
      case 'update':
        return Icons.edit_outlined;
      case 'delete':
        return Icons.delete_outline;
      case 'restore':
        return Icons.restore;
      case 'purge':
        return Icons.delete_forever;
      default:
        return Icons.info_outline;
    }
  }

  Color _actionColor(BuildContext context) {
    switch (entry.action) {
      case 'create':
        return AppTheme.successColor;
      case 'update':
        return AppTheme.primaryColor;
      case 'delete':
        return Colors.orange.shade700;
      case 'restore':
        return Colors.blue;
      case 'purge':
        return AppTheme.errorColor;
      default:
        return Theme.of(context).colorScheme.onSurfaceVariant;
    }
  }

  String get _actionLabel {
    switch (entry.action) {
      case 'create':
        return 'Created';
      case 'update':
        return 'Updated';
      case 'delete':
        return 'Deleted';
      case 'restore':
        return 'Restored';
      case 'purge':
        return 'Purged';
      default:
        return entry.action;
    }
  }

  IconData _deviceIcon(String device) {
    switch (device.toLowerCase()) {
      case 'macos':
        return Icons.laptop_mac;
      case 'ios':
        return Icons.phone_iphone;
      case 'android':
        return Icons.phone_android;
      case 'windows':
        return Icons.desktop_windows;
      case 'linux':
        return Icons.computer;
      case 'web':
        return Icons.web;
      default:
        return Icons.devices;
    }
  }

  Color _sensitivityColor(SensitivityLevel level) {
    switch (level) {
      case SensitivityLevel.critical:
        return Colors.red.shade900;
      case SensitivityLevel.sensitive:
        return Colors.orange;
      case SensitivityLevel.internal:
        return Colors.blue;
      case SensitivityLevel.public:
        return Colors.green;
    }
  }

  String _formatTime(DateTime dt) {
    final now = DateTime.now();
    final diff = now.difference(dt);
    if (diff.inMinutes < 1) return 'Just now';
    if (diff.inMinutes < 60) return '${diff.inMinutes}m ago';
    if (diff.inHours < 24) return '${diff.inHours}h ago';
    if (diff.inDays < 7) return '${diff.inDays}d ago';
    return '${dt.day}/${dt.month}/${dt.year}';
  }

  String _getDeviceLabel(String device) {
    switch (device.toLowerCase()) {
      case 'macos':
        return 'macOS';
      case 'ios':
        return 'iOS';
      case 'android':
        return 'Android';
      case 'windows':
        return 'Windows';
      case 'linux':
        return 'Linux';
      case 'web':
        return 'Web';
      default:
        return device;
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Container(
              width: 36,
              height: 36,
              decoration: BoxDecoration(
                color: _actionColor(context).withValues(alpha: 0.1),
                borderRadius: BorderRadius.circular(8),
              ),
              child: Icon(_actionIcon, size: 18, color: _actionColor(context)),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  // First row: action type, section, and time
                  Row(
                    children: [
                      Container(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 6,
                          vertical: 2,
                        ),
                        decoration: BoxDecoration(
                          color: _actionColor(context).withValues(alpha: 0.1),
                          borderRadius: BorderRadius.circular(4),
                        ),
                        child: Text(
                          _actionLabel,
                          style: theme.textTheme.labelSmall?.copyWith(
                            color: _actionColor(context),
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                      ),
                      const SizedBox(width: 8),
                      Text(
                        entry.section.toUpperCase(),
                        style: theme.textTheme.labelSmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                      const Spacer(),
                      Text(
                        _formatTime(entry.timestamp),
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 6),
                  // Second row: description
                  Text(entry.description, style: theme.textTheme.bodyMedium),
                  const SizedBox(height: 8),
                  // Third row: device tag only
                  Wrap(
                    spacing: 8,
                    runSpacing: 4,
                    children: [
                      Container(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 6,
                          vertical: 2,
                        ),
                        decoration: BoxDecoration(
                          color: Colors.grey.withValues(alpha: 0.1),
                          borderRadius: BorderRadius.circular(4),
                          border: Border.all(
                            color: Colors.grey.withValues(alpha: 0.3),
                          ),
                        ),
                        child: Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Icon(
                              _deviceIcon(entry.device),
                              size: 12,
                              color: Colors.grey.shade700,
                            ),
                            const SizedBox(width: 4),
                            Text(
                              _getDeviceLabel(entry.device),
                              style: theme.textTheme.labelSmall?.copyWith(
                                color: Colors.grey.shade700,
                                fontWeight: FontWeight.w500,
                              ),
                            ),
                          ],
                        ),
                      ),
                    ],
                  ),
                ],
              ),
            ),
            IconButton(
              icon: const Icon(Icons.info_outline, size: 20),
              onPressed: () => _showDetailDialog(context),
              tooltip: 'View details',
              visualDensity: VisualDensity.compact,
            ),
          ],
        ),
      ),
    );
  }
}

class _DetailRow extends StatelessWidget {
  final String label;
  final String value;

  const _DetailRow({
    required this.label,
    required this.value,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          label,
          style: theme.textTheme.labelSmall?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
        const SizedBox(height: 2),
        Text(
          value,
          style: theme.textTheme.bodyMedium,
        ),
      ],
    );
  }
}
