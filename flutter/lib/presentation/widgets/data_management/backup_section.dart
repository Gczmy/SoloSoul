import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/core/services/backup_service.dart';

import 'package:solosoul_flutter/presentation/widgets/data_management/backup_list_tile.dart';
import 'package:solosoul_flutter/presentation/widgets/data_management/backup_progress_indicator.dart';

class BackupSection extends StatelessWidget {
  final bool isCreating;
  final double backupProgress;
  final List<BackupEntry> backups;
  final bool isRestoring;
  final int totalSize;
  final String backupPoolSize;
  final VoidCallback onCreateBackup;
  final ValueChanged<BackupEntry> onRestoreBackup;
  final ValueChanged<BackupEntry> onDeleteBackup;
  final ValueChanged<BackupEntry> onPromoteBackup;

  const BackupSection({
    super.key,
    required this.isCreating,
    required this.backupProgress,
    required this.backups,
    required this.isRestoring,
    required this.totalSize,
    required this.backupPoolSize,
    required this.onCreateBackup,
    required this.onRestoreBackup,
    required this.onDeleteBackup,
    required this.onPromoteBackup,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 20),
          child: Row(
            children: [
              Expanded(
                child: FilledButton.icon(
                  onPressed: isCreating ? null : onCreateBackup,
                  icon: isCreating
                      ? const SizedBox(
                          width: 16,
                          height: 16,
                          child: CircularProgressIndicator(
                            strokeWidth: 2,
                            color: Colors.white,
                          ),
                        )
                      : const Icon(Icons.backup, size: 18),
                  label: Text(AppLocalizations.of(context).dataManagementBackupNow),
                ),
              ),
            ],
          ),
        ),
        if (isCreating) ...[
          const SizedBox(height: 12),
          BackupProgressIndicator(progress: backupProgress),
        ],
        const SizedBox(height: 8),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 20),
          child: Row(
            children: [
              Icon(Icons.info_outline, size: 14, color: theme.colorScheme.onSurfaceVariant),
              const SizedBox(width: 6),
              Expanded(
                child: Text(
                  AppLocalizations.of(context).dataMgmtBackupEncryptionDesc,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: 16),
        const Divider(height: 1),
        const SizedBox(height: 8),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 20),
          child: Row(
            children: [
              Icon(Icons.backup_outlined, color: theme.colorScheme.primary, size: 20),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  AppLocalizations.of(context).dataMgmtRegularBackups,
                  style: theme.textTheme.titleMedium?.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: 8),
        if (backups.isEmpty)
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 24),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(
                  Icons.cloud_off_outlined,
                  size: 48,
                  color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.4),
                ),
                const SizedBox(height: 12),
                Text(
                  AppLocalizations.of(context).dataMgmtNoBackups,
                  style: theme.textTheme.bodyLarge?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
            ),
          )
        else
          Column(
            children: backups.map((entry) {
              return BackupListTile(
                entry: entry,
                isRestoring: isRestoring,
                onPromote: () => onPromoteBackup(entry),
                onRestore: () => onRestoreBackup(entry),
                onDelete: () => onDeleteBackup(entry),
              );
            }).toList(),
          ),
        if (backups.isNotEmpty)
          Padding(
            padding: const EdgeInsets.only(bottom: 12),
            child: Center(
              child: Text(
                '${backups.length} 个备份 · 附件池 $backupPoolSize',
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
