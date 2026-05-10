import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/core/services/backup_service.dart';
import 'package:solosoul_flutter/presentation/widgets/data_management/backup_list_tile.dart';
import 'package:solosoul_flutter/presentation/widgets/data_management/backup_progress_indicator.dart';

class RestoreSection extends StatelessWidget {
  final List<BackupEntry> specialBackups;
  final bool isCreatingSpecial;
  final double specialBackupProgress;
  final bool isRestoring;
  final VoidCallback onCreateSpecialBackup;
  final ValueChanged<BackupEntry> onRestoreSpecialBackup;
  final ValueChanged<BackupEntry> onDeleteSpecialBackup;
  final ValueChanged<BackupEntry> onRenameSpecialBackup;

  const RestoreSection({
    super.key,
    required this.specialBackups,
    required this.isCreatingSpecial,
    required this.specialBackupProgress,
    required this.isRestoring,
    required this.onCreateSpecialBackup,
    required this.onRestoreSpecialBackup,
    required this.onDeleteSpecialBackup,
    required this.onRenameSpecialBackup,
  });

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);

    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 20),
          child: Row(
            children: [
              Icon(
                Icons.star_outline,
                color: theme.colorScheme.secondary,
                size: 20,
              ),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  l10n.dataManagementSpecialBackupsTitle,
                  style: theme.textTheme.titleMedium?.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
              TextButton.icon(
                onPressed: isCreatingSpecial ? null : onCreateSpecialBackup,
                icon: const Icon(Icons.add, size: 16),
                label: Text(AppLocalizations.of(context).dataManagementCreate),
              ),
            ],
          ),
        ),
        if (isCreatingSpecial) ...[
          const SizedBox(height: 8),
          BackupProgressIndicator(progress: specialBackupProgress),
        ],
        const SizedBox(height: 4),
        if (specialBackups.isEmpty)
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
            child: Text(
              l10n.dataManagementNoSpecialBackups,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          )
        else
          Padding(
            padding: const EdgeInsets.only(bottom: 12),
            child: Column(
              children: specialBackups.map((entry) {
                return BackupListTile(
                  entry: entry,
                  isSpecial: true,
                  isRestoring: isRestoring,
                  onRename: () => onRenameSpecialBackup(entry),
                  onRestore: () => onRestoreSpecialBackup(entry),
                  onDelete: () => onDeleteSpecialBackup(entry),
                );
              }).toList(),
            ),
          ),
        if (specialBackups.isNotEmpty)
          Padding(
            padding: const EdgeInsets.only(bottom: 16),
            child: Center(
              child: Text(
                l10n.dataManagementSpecialBackupsCount(specialBackups.length, BackupService.maxSpecialBackupCount),
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
            ),
          ),
      ],
    );
  }
}
