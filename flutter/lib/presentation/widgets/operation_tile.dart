import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import '../models/operation_log_models.dart';

class OperationTile extends StatelessWidget {
  final OperationEntry entry;

  const OperationTile({super.key, required this.entry});

  void _showDetailDialog(BuildContext context) {
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
        content: Column(
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
            const SizedBox(height: 12),
            _DetailRow(
              label: 'Sensitivity Level',
              value: entry.sensitivityLevel.label,
              valueColor: _sensitivityColor(entry.sensitivityLevel),
            ),
          ],
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
        return Colors.red;
      case SensitivityLevel.sensitive:
        return Colors.orange;
      case SensitivityLevel.internal:
        return Colors.green;
      case SensitivityLevel.public:
        return Colors.blue;
    }
  }

  IconData _sensitivityIcon(SensitivityLevel level) {
    switch (level) {
      case SensitivityLevel.critical:
        return Icons.lock;
      case SensitivityLevel.sensitive:
        return Icons.visibility_off;
      case SensitivityLevel.internal:
        return Icons.folder;
      case SensitivityLevel.public:
        return Icons.public;
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
    final sensitivityColor = _sensitivityColor(entry.sensitivityLevel);

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
                  // Third row: tags
                  Wrap(
                    spacing: 8,
                    runSpacing: 4,
                    children: [
                      // Device tag
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
                      // Sensitivity tag
                      Container(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 6,
                          vertical: 2,
                        ),
                        decoration: BoxDecoration(
                          color: sensitivityColor.withValues(alpha: 0.1),
                          borderRadius: BorderRadius.circular(4),
                          border: Border.all(
                            color: sensitivityColor.withValues(alpha: 0.3),
                          ),
                        ),
                        child: Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Icon(
                              _sensitivityIcon(entry.sensitivityLevel),
                              size: 12,
                              color: sensitivityColor,
                            ),
                            const SizedBox(width: 4),
                            Text(
                              entry.sensitivityLevel.label,
                              style: theme.textTheme.labelSmall?.copyWith(
                                color: sensitivityColor,
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
  final Color? valueColor;

  const _DetailRow({
    required this.label,
    required this.value,
    this.valueColor,
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
          style: theme.textTheme.bodyMedium?.copyWith(
            color: valueColor,
          ),
        ),
      ],
    );
  }
}
