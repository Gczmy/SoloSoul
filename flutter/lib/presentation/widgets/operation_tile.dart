import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/utils/format_field_label.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import '../models/operation_log_models.dart';

class OperationTile extends StatelessWidget {
  final OperationEntry entry;

  const OperationTile({super.key, required this.entry});

  void _showDetailDialog(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final props = entry.properties;
    final hasProperties = props != null && props.isNotEmpty;

    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: Row(
          children: [
            Icon(_actionIcon, color: _actionColor(context)),
            const SizedBox(width: 8),
            Expanded(child: Text(l10n.operationDetails)),
          ],
        ),
        content: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              _DetailRow(label: l10n.operationLabelTimestamp, value: _formatFullTimestamp(entry.timestamp)),
              const SizedBox(height: 12),
              _DetailRow(label: l10n.operationLabelAction, value: _actionLabel(l10n)),
              const SizedBox(height: 12),
              _DetailRow(label: l10n.operationLabelSection, value: _sectionLabel(l10n)),
              if (entry.fieldPath != null) ...[
                const SizedBox(height: 12),
                _DetailRow(label: l10n.operationLabelFieldPath, value: entry.fieldPath!),
              ],
              const SizedBox(height: 12),
              _DetailRow(label: l10n.operationLabelDescription, value: entry.localizedDescription(l10n)),
              const SizedBox(height: 12),
              _DetailRow(label: l10n.operationLabelDevice, value: _getDeviceLabel(entry.device, l10n)),
              if (hasProperties) ...[
                const SizedBox(height: 16),
                const Divider(),
                const SizedBox(height: 8),
                Text(
                  l10n.operationLogPropertySnapshot,
                  style: Theme.of(context).textTheme.titleSmall?.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: 8),
                ..._buildPropertyList(context),
              ],
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: Text(l10n.commonClose),
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

  String _actionLabel(AppLocalizations l10n) {
    switch (entry.action) {
      case 'create':
        return l10n.operationActionCreate;
      case 'update':
        return l10n.operationActionUpdate;
      case 'delete':
        return l10n.operationActionDelete;
      case 'restore':
        return l10n.operationActionRestore;
      case 'purge':
        return l10n.operationActionPurge;
      default:
        return entry.action;
    }
  }

  String _sectionLabel(AppLocalizations l10n) {
    // Match both stored names (e.g. 'Passports') and normalized keys.
    final s = entry.section.toLowerCase();
    switch (s) {
      case 'identity':
      case 'identities':
        return l10n.logSectionIdentity;
      case 'contact':
      case 'contact information':
      case 'contacts':
        return l10n.logSectionContactInfo;
      case 'address':
      case 'addresses':
        return l10n.logSectionAddress;
      case 'id card':
      case 'id cards':
        return l10n.logSectionIdCard;
      case 'passport':
      case 'passports':
        return l10n.logSectionPassport;
      case 'visa':
      case 'visas':
        return l10n.logSectionVisa;
      case 'travel history':
      case 'travel histories':
        return l10n.logSectionTravelHistory;
      case 'bank account':
      case 'bank accounts':
        return l10n.logSectionBankAccount;
      case 'card':
      case 'cards':
        return l10n.logSectionCard;
      case 'education':
        return l10n.logSectionEducation;
      case 'employment':
      case 'employments':
        return l10n.logSectionEmployment;
      case 'skill':
      case 'skills':
        return l10n.logSectionSkill;
      case 'language':
      case 'languages':
        return l10n.logSectionLanguage;
      case 'travel':
        return l10n.logSectionTravel;
      case 'financial':
        return l10n.logSectionFinancial;
      case 'professional':
        return l10n.logSectionProfessional;
      case 'sensitivity settings':
        return l10n.logSectionSensitivity;
      default:
        return l10n.logSectionDefault;
    }
  }

  List<Widget> _buildPropertyList(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final properties = entry.properties;
    if (properties == null) return [];
    return properties.entries.map((e) {
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
                translateFieldLabel(e.key, l10n),
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
              ),
            ),
            Expanded(
              flex: 3,
              child: Text(
                e.value.isEmpty ? l10n.commonEmpty : e.value,
                style: Theme.of(context).textTheme.bodyMedium,
              ),
            ),
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
              decoration: BoxDecoration(
                color: levelColor.withValues(alpha: 0.1),
                borderRadius: BorderRadius.circular(4),
                border: Border.all(color: levelColor.withValues(alpha: 0.3)),
              ),
              child: Text(
                level.localizedLabel(l10n),
                style: Theme.of(context).textTheme.labelSmall?.copyWith(
                  color: levelColor,
                  fontWeight: FontWeight.w500,
                ),
              ),
            ),
          ],
        ),
      );
    }).toList();
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

  String _formatTime(DateTime dt, AppLocalizations l10n) {
    final now = DateTime.now();
    final diff = now.difference(dt);
    if (diff.inMinutes < 1) return l10n.trashJustNow;
    if (diff.inMinutes < 60) return l10n.trashMinutesAgo(diff.inMinutes);
    if (diff.inHours < 24) return l10n.trashHoursAgo(diff.inHours);
    if (diff.inDays < 7) return l10n.trashDaysAgo(diff.inDays);
    return '${dt.day}/${dt.month}/${dt.year}';
  }

  String _getDeviceLabel(String device, AppLocalizations l10n) {
    switch (device.toLowerCase()) {
      case 'macos':
        return l10n.operationPlatformMacos;
      case 'ios':
        return l10n.operationPlatformIos;
      case 'android':
        return l10n.operationPlatformAndroid;
      case 'windows':
        return l10n.operationPlatformWindows;
      case 'linux':
        return l10n.operationPlatformLinux;
      case 'web':
        return l10n.operationPlatformWeb;
      default:
        return device;
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
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
                          _actionLabel(l10n),
                          style: theme.textTheme.labelSmall?.copyWith(
                            color: _actionColor(context),
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                      ),
                      const SizedBox(width: 8),
                      Text(
                        _sectionLabel(l10n),
                        style: theme.textTheme.labelSmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                      const Spacer(),
                      Text(
                        _formatTime(entry.timestamp, l10n),
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 6),
                  // Second row: description
                  Text(entry.localizedDescription(l10n), style: theme.textTheme.bodyMedium),
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
                              _getDeviceLabel(entry.device, l10n),
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
              tooltip: l10n.operationViewDetails,
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
