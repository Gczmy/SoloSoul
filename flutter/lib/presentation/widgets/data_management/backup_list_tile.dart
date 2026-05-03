import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/services/backup_service.dart';
import 'package:solosoul_flutter/presentation/utils/format_utils.dart';

class BackupListTile extends StatelessWidget {
  final BackupEntry entry;
  final bool isSpecial;
  final bool isRestoring;
  final VoidCallback? onPromote;
  final VoidCallback? onRestore;
  final VoidCallback? onDelete;
  final VoidCallback? onRename;

  const BackupListTile({
    super.key,
    required this.entry,
    this.isSpecial = false,
    this.isRestoring = false,
    this.onPromote,
    this.onRestore,
    this.onDelete,
    this.onRename,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    if (isSpecial) {
      final displayName = entry.fileName.replaceAll('.backup', '');
      return ListTile(
        dense: true,
        leading: Icon(
          Icons.star,
          color: theme.colorScheme.secondary,
          size: 20,
        ),
        title: Text(displayName),
        subtitle: Text(
          '${entry.displayTime}  ·  ${formatBytes(entry.sizeBytes)}',
          style: theme.textTheme.bodySmall,
        ),
        trailing: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            IconButton(
              icon: const Icon(Icons.edit, size: 18),
              tooltip: 'Rename',
              onPressed: onRename,
            ),
            IconButton(
              icon: const Icon(Icons.restore, size: 18),
              tooltip: 'Restore',
              onPressed: isRestoring ? null : onRestore,
            ),
            IconButton(
              icon: const Icon(Icons.delete_outline, size: 18),
              tooltip: 'Delete',
              style: IconButton.styleFrom(
                foregroundColor: theme.colorScheme.error,
                overlayColor: theme.colorScheme.error.withValues(alpha: 0.1),
              ),
              onPressed: onDelete,
            ),
          ],
        ),
      );
    }

    return ListTile(
      leading: Icon(
        Icons.backup_outlined,
        color: theme.colorScheme.primary,
      ),
      title: Text(entry.displayTime),
      subtitle: Text(
        formatBytes(entry.sizeBytes),
        style: theme.textTheme.bodySmall,
      ),
      trailing: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          IconButton(
            icon: Icon(
              Icons.star_outline,
              size: 20,
              color: theme.colorScheme.secondary,
            ),
            tooltip: 'Save as special backup',
            onPressed: onPromote,
          ),
          IconButton(
            icon: const Icon(Icons.restore, size: 20),
            tooltip: 'Restore',
            onPressed: isRestoring ? null : onRestore,
          ),
          IconButton(
            icon: const Icon(Icons.delete_outline, size: 20),
            tooltip: 'Delete',
            style: IconButton.styleFrom(
              foregroundColor: theme.colorScheme.error,
              overlayColor: theme.colorScheme.error.withValues(alpha: 0.1),
            ),
            onPressed: onDelete,
          ),
        ],
      ),
    );
  }
}
