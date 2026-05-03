import 'dart:async' show unawaited;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:package_info_plus/package_info_plus.dart';

import 'package:solosoul_flutter/core/services/backup_service.dart';
import 'package:solosoul_flutter/core/services/operation_notification.dart';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart' show AppTheme;

/// Data Management page — full-screen backup & restore UI.
class DataManagementPage extends ConsumerStatefulWidget {
  const DataManagementPage({super.key});

  @override
  ConsumerState<DataManagementPage> createState() => _DataManagementPageState();
}

class _DataManagementPageState extends ConsumerState<DataManagementPage> {
  List<BackupEntry> _backups = const [];
  List<BackupEntry> _specialBackups = const [];
  bool _isLoading = true;
  bool _isCreating = false;
  bool _isCreatingSpecial = false;
  bool _isRestoring = false;
  double _backupProgress = 0.0;
  double _specialBackupProgress = 0.0;

  String? _accountId;
  String _vaultDataSize = 'Unknown';
  String? _appVersion;

  @override
  void initState() {
    super.initState();
    _init();
  }

  Future<void> _init() async {
    final authNotifier = ref.read(authNotifierProvider.notifier);
    _accountId = authNotifier.selectedAccountId;

    final stats = await RustVaultService.instance.getVaultStats();
    if (stats != null) {
      _vaultDataSize = _formatBytes(stats.totalSizeBytes.toInt());
    }

    final packageInfo = await PackageInfo.fromPlatform();
    _appVersion = packageInfo.version;

    if (mounted) setState(() {});
    if (_accountId != null) await _loadAllBackups();
  }

  Future<void> _loadAllBackups() async {
    if (_accountId == null) return;
    final regular = await BackupService.instance.listBackups(_accountId!);
    final special = await BackupService.instance.listSpecialBackups(_accountId!);
    if (mounted) {
      setState(() {
        _backups = regular;
        _specialBackups = special;
        _isLoading = false;
      });
    }
  }

  Future<void> _loadBackups() async {
    if (_accountId == null) return;
    final list = await BackupService.instance.listBackups(_accountId!);
    if (mounted) {
      setState(() {
        _backups = list;
      });
    }
  }

  Future<void> _loadSpecialBackups() async {
    if (_accountId == null) return;
    final list = await BackupService.instance.listSpecialBackups(_accountId!);
    if (mounted) {
      setState(() {
        _specialBackups = list;
      });
    }
  }

  // -------------------------------------------------------------------------
  // 常规备份
  // -------------------------------------------------------------------------

  Future<void> _createBackup() async {
    if (_accountId == null) return;
    setState(() {
      _isCreating = true;
      _backupProgress = 0.0;
    });
    try {
      final fileName = await BackupService.instance.createBackup(
        _accountId!,
        appVersion: _appVersion,
        onProgress: (p) {
          if (mounted) setState(() => _backupProgress = p);
        },
      );
      if (!mounted) return;
      setState(() => _isCreating = false);
      if (fileName != null) {
        await _loadBackups();
        if (!mounted) return;
        unawaited(ref.read(authNotifierProvider.notifier).updateOperation('Created backup'));
        OperationNotification.show(
          context,
          message: const OperationMessage(
            type: OperationType.create,
            section: 'backup',
            customMessage: 'Backup created successfully',
          ),
        );
      } else {
        OperationNotification.show(
          context,
          message: const OperationMessage(
            type: OperationType.delete,
            section: 'backup',
            customMessage: 'Backup failed',
          ),
        );
      }
    } on Exception catch (e) {
      if (!mounted) return;
      setState(() => _isCreating = false);
      OperationNotification.show(
        context,
        message: OperationMessage(
          type: OperationType.purge,
          section: 'backup',
          customMessage: 'Backup error: $e',
        ),
      );
    }
  }

  Future<void> _restoreBackup(BackupEntry entry) async {
    if (_accountId == null) return;
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Restore Backup?'),
        content: Text(
          'This will overwrite your current data with the backup from ${entry.displayTime}. '
          'A safety backup of the current state will be created first.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(ctx).pop(true),
            child: const Text('Restore'),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted) return;

    setState(() => _isRestoring = true);
    final success = await BackupService.instance.restoreBackup(
      _accountId!,
      entry.fileName,
    );
    if (!mounted) return;
    setState(() => _isRestoring = false);
    if (success) {
      await _loadAllBackups();
      if (!mounted) return;
      unawaited(ref.read(authNotifierProvider.notifier).updateOperation('Restored backup'));
      OperationNotification.show(
        context,
        message: const OperationMessage(
          type: OperationType.restore,
          section: 'backup',
          customMessage: 'Restore successful. Please restart the app.',
        ),
        duration: AppTheme.kPasswordHintDelay,
      );
    } else {
      OperationNotification.show(
        context,
        message: const OperationMessage(
          type: OperationType.purge,
          section: 'backup',
          customMessage: 'Restore failed',
        ),
      );
    }
  }

  Future<void> _deleteBackup(BackupEntry entry) async {
    if (_accountId == null) return;

    // Step 1: Confirm deletion
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Delete Backup?'),
        content: Text('Delete backup from ${entry.displayTime}?'),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(ctx).pop(true),
            child: const Text('Delete'),
          ),
        ],
      ),
    );
    if (confirmed != true) return;
    if (!mounted) return;

    // Step 2: Verify password before destructive action
    final authNotifier = ref.read(authNotifierProvider.notifier);
    final selectedAccount = authNotifier.selectedAccount;
    final password = await showPasswordVerificationDialog(
      context: context,
      ref: ref,
      message: 'Enter your master password to confirm backup deletion.',
      passwordHint: selectedAccount?.passwordHint,
      onVerify: authNotifier.verifyPasswordForSensitiveData,
    );
    if (password == null) return; // User cancelled verification

    final success = await BackupService.instance.deleteBackup(
      _accountId!,
      entry.fileName,
    );
    if (success && mounted) {
      await _loadBackups();
      if (!mounted) return;
      unawaited(ref.read(authNotifierProvider.notifier).updateOperation('Deleted backup'));
      OperationNotification.show(
        context,
        message: const OperationMessage(
          type: OperationType.purge,
          section: 'backup',
          customMessage: 'Backup deleted',
        ),
      );
    }
  }

  // -------------------------------------------------------------------------
  // 普通备份 → 特别备份
  // -------------------------------------------------------------------------

  Future<void> _promoteToSpecialBackup(BackupEntry entry) async {
    if (_specialBackups.length >= BackupService.maxSpecialBackupCount) {
      if (mounted) {
        await showDialog<void>(
          context: context,
          builder: (ctx) => AlertDialog(
            title: const Text('Special Backup Limit Reached'),
            content: const Text(
              'You can keep up to ${BackupService.maxSpecialBackupCount} special backups. '
              'Please delete an existing one before promoting.',
            ),
            actions: [
              TextButton(
                onPressed: () => Navigator.of(ctx).pop(),
                child: const Text('OK'),
              ),
            ],
          ),
        );
      }
      return;
    }

    final controller = TextEditingController();
    final name = await showDialog<String>(
      context: context,
      builder: (ctx) {
        return AlertDialog(
          title: const Text('Name Special Backup'),
          content: TextField(
            controller: controller,
            autofocus: true,
            decoration: const InputDecoration(
              hintText: 'e.g. Before Major Update',
              labelText: 'Backup name',
            ),
            maxLength: 50,
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(ctx).pop(),
              child: const Text('Cancel'),
            ),
            FilledButton(
              onPressed: () {
                final text = controller.text.trim();
                if (text.isNotEmpty) Navigator.of(ctx).pop(text);
              },
              child: const Text('Save'),
            ),
          ],
        );
      },
    );
    controller.dispose();

    if (name == null || name.isEmpty) return;

    final result = await BackupService.instance.promoteBackupToSpecial(
      _accountId!,
      entry.fileName,
      name,
    );

    if (result != null) {
      if (!mounted) return;
      await _loadSpecialBackups();
      if (!mounted) return;
      unawaited(ref.read(authNotifierProvider.notifier).updateOperation('Promoted backup to special'));
      OperationNotification.show(
        context,
        message: OperationMessage(
          type: OperationType.create,
          section: 'backup',
          customMessage: 'Saved as special backup "$name"',
        ),
      );
    } else {
      if (!mounted) return;
      OperationNotification.show(
        context,
        message: const OperationMessage(
          type: OperationType.delete,
          section: 'backup',
          customMessage: 'Failed to save as special backup',
        ),
      );
    }
  }

  // -------------------------------------------------------------------------
  // 特别备份
  // -------------------------------------------------------------------------

  Future<void> _createSpecialBackup() async {
    if (_specialBackups.length >= BackupService.maxSpecialBackupCount) {
      if (mounted) {
        await showDialog<void>(
          context: context,
          builder: (ctx) => AlertDialog(
            title: const Text('Special Backup Limit Reached'),
            content: const Text(
              'You can keep up to ${BackupService.maxSpecialBackupCount} special backups. '
              'Please delete an existing one before creating a new special backup.',
            ),
            actions: [
              TextButton(
                onPressed: () => Navigator.of(ctx).pop(),
                child: const Text('OK'),
              ),
            ],
          ),
        );
      }
      return;
    }

    final controller = TextEditingController();
    final name = await showDialog<String>(
      context: context,
      builder: (ctx) {
        return AlertDialog(
          title: const Text('Name Special Backup'),
          content: TextField(
            controller: controller,
            autofocus: true,
            decoration: const InputDecoration(
              hintText: 'e.g. Before Major Update',
              labelText: 'Backup name',
            ),
            maxLength: 50,
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(ctx).pop(),
              child: const Text('Cancel'),
            ),
            FilledButton(
              onPressed: () {
                final text = controller.text.trim();
                if (text.isNotEmpty) Navigator.of(ctx).pop(text);
              },
              child: const Text('Create'),
            ),
          ],
        );
      },
    );
    controller.dispose();

    if (name == null || name.isEmpty) return;

    setState(() {
      _isCreatingSpecial = true;
      _specialBackupProgress = 0.0;
    });

    final fileName = await BackupService.instance.createSpecialBackup(
      _accountId!,
      name,
      onProgress: (p) {
        if (mounted) setState(() => _specialBackupProgress = p);
      },
    );

    if (!mounted) return;
    setState(() => _isCreatingSpecial = false);
    if (fileName != null) {
      await _loadSpecialBackups();
      if (!mounted) return;
      unawaited(ref.read(authNotifierProvider.notifier).updateOperation('Created special backup'));
      OperationNotification.show(
        context,
        message: OperationMessage(
          type: OperationType.create,
          section: 'backup',
          customMessage: 'Special backup "$name" created',
        ),
      );
    } else {
      OperationNotification.show(
        context,
        message: const OperationMessage(
          type: OperationType.delete,
          section: 'backup',
          customMessage: 'Special backup failed',
        ),
      );
    }
  }

  Future<void> _renameSpecialBackup(BackupEntry entry) async {
    final currentName = entry.fileName.replaceAll('.backup', '');
    final controller = TextEditingController(text: currentName);
    final newName = await showDialog<String>(
      context: context,
      builder: (ctx) {
        return AlertDialog(
          title: const Text('Rename Special Backup'),
          content: TextField(
            controller: controller,
            autofocus: true,
            decoration: const InputDecoration(
              labelText: 'New name',
            ),
            maxLength: 50,
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(ctx).pop(),
              child: const Text('Cancel'),
            ),
            FilledButton(
              onPressed: () {
                final text = controller.text.trim();
                if (text.isNotEmpty) Navigator.of(ctx).pop(text);
              },
              child: const Text('Rename'),
            ),
          ],
        );
      },
    );
    controller.dispose();

    if (newName == null || newName.isEmpty) return;

    final result = await BackupService.instance.renameSpecialBackup(
      _accountId!,
      entry.fileName,
      newName,
    );

    if (result != null) {
      if (!mounted) return;
      await _loadSpecialBackups();
      if (!mounted) return;
      unawaited(ref.read(authNotifierProvider.notifier).updateOperation('Renamed special backup'));
      OperationNotification.show(
        context,
        message: OperationMessage(
          type: OperationType.update,
          section: 'backup',
          customMessage: 'Renamed to "$newName"',
        ),
      );
    }
  }

  Future<void> _restoreSpecialBackup(BackupEntry entry) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Restore Special Backup?'),
        content: Text(
          'This will overwrite your current data with the special backup "${entry.fileName.replaceAll('.backup', '')}". '
          'A safety backup of the current state will be created first.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(ctx).pop(true),
            child: const Text('Restore'),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted) return;

    setState(() => _isRestoring = true);
    final success = await BackupService.instance.restoreSpecialBackup(
      _accountId!,
      entry.fileName,
    );
    if (!mounted) return;
    setState(() => _isRestoring = false);
    if (success) {
      await _loadAllBackups();
      if (!mounted) return;
      unawaited(ref.read(authNotifierProvider.notifier).updateOperation('Restored special backup'));
      OperationNotification.show(
        context,
        message: const OperationMessage(
          type: OperationType.restore,
          section: 'backup',
          customMessage: 'Special backup restored. Please restart the app.',
        ),
        duration: AppTheme.kPasswordHintDelay,
      );
    } else {
      OperationNotification.show(
        context,
        message: const OperationMessage(
          type: OperationType.purge,
          section: 'backup',
          customMessage: 'Restore failed',
        ),
      );
    }
  }

  Future<void> _deleteSpecialBackup(BackupEntry entry) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Delete Special Backup?'),
        content: Text('Delete special backup "${entry.fileName.replaceAll('.backup', '')}"?'),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(ctx).pop(true),
            style: FilledButton.styleFrom(
              backgroundColor: Colors.red,
              foregroundColor: Colors.white,
            ),
            child: const Text('Delete'),
          ),
        ],
      ),
    );
    if (confirmed != true) return;

    final success = await BackupService.instance.deleteSpecialBackup(
      _accountId!,
      entry.fileName,
    );
    if (success && mounted) {
      await _loadSpecialBackups();
      unawaited(ref.read(authNotifierProvider.notifier).updateOperation('Deleted special backup'));
    }
  }

  // -------------------------------------------------------------------------
  // Build
  // -------------------------------------------------------------------------

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final totalSize = _backups.fold<int>(0, (s, e) => s + e.sizeBytes);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Data Management'),
        actions: [
          if (_isCreating || _isRestoring || _isCreatingSpecial)
            const Padding(
              padding: EdgeInsets.only(right: 16),
              child: SizedBox(
                width: 20,
                height: 20,
                child: CircularProgressIndicator(strokeWidth: 2),
              ),
            ),
        ],
      ),
      body: _isLoading
          ? const Center(child: CircularProgressIndicator())
          : RefreshIndicator(
              onRefresh: _loadAllBackups,
              child: ListView(
                padding: const EdgeInsets.symmetric(vertical: 16),
                children: [
                  // Vault size
                  Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 20),
                    child: Row(
                      children: [
                        Text(
                          'Vault size: ',
                          style: theme.textTheme.bodyMedium?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                        Text(
                          _vaultDataSize,
                          style: theme.textTheme.bodyMedium?.copyWith(
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(height: 16),
                  // Backup Now button
                  Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 20),
                    child: Row(
                      children: [
                        Expanded(
                          child: FilledButton.icon(
                            onPressed: _isCreating ? null : _createBackup,
                            icon: _isCreating
                                ? const SizedBox(
                                    width: 16,
                                    height: 16,
                                    child: CircularProgressIndicator(
                                      strokeWidth: 2,
                                      color: Colors.white,
                                    ),
                                  )
                                : const Icon(Icons.backup, size: 18),
                            label: const Text('Backup Now'),
                          ),
                        ),
                      ],
                    ),
                  ),
                  // Backup progress
                  if (_isCreating) ...[
                    const SizedBox(height: 12),
                    Padding(
                      padding: const EdgeInsets.symmetric(horizontal: 20),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          LinearProgressIndicator(
                            value: _backupProgress > 0 ? _backupProgress : null,
                            borderRadius: BorderRadius.circular(4),
                          ),
                          const SizedBox(height: 4),
                          Text(
                            _backupProgress >= 1.0
                                ? 'Finishing...'
                                : _backupProgress >= 0.9
                                    ? 'Writing file...'
                                    : _backupProgress >= 0.5
                                        ? 'Encrypting...'
                                        : _backupProgress >= 0.3
                                            ? 'Encoding...'
                                            : 'Reading data...',
                            style: theme.textTheme.bodySmall?.copyWith(
                              color: theme.colorScheme.onSurfaceVariant,
                            ),
                          ),
                        ],
                      ),
                    ),
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
                            'Backups are encrypted with your vault key. Auto-backup runs on every unlock.',
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
                  // Regular backups header
                  Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 20),
                    child: Row(
                      children: [
                        Icon(Icons.backup_outlined, color: theme.colorScheme.primary, size: 20),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Text(
                            'Regular Backups',
                            style: theme.textTheme.titleMedium?.copyWith(
                              fontWeight: FontWeight.w600,
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(height: 8),
                  // Regular backup list
                  if (_backups.isEmpty)
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
                            'No backups yet',
                            style: theme.textTheme.bodyLarge?.copyWith(
                              color: theme.colorScheme.onSurfaceVariant,
                            ),
                          ),
                        ],
                      ),
                    )
                  else
                    Column(
                      children: _backups.map((entry) {
                        return ListTile(
                          leading: Icon(
                            Icons.backup_outlined,
                            color: theme.colorScheme.primary,
                          ),
                          title: Text(entry.displayTime),
                          subtitle: Text(
                            _formatBytes(entry.sizeBytes),
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
                                onPressed: () => _promoteToSpecialBackup(entry),
                              ),
                              IconButton(
                                icon: const Icon(Icons.restore, size: 20),
                                tooltip: 'Restore',
                                onPressed: _isRestoring
                                    ? null
                                    : () => _restoreBackup(entry),
                              ),
                              IconButton(
                                icon: const Icon(Icons.delete_outline, size: 20),
                                tooltip: 'Delete',
                                style: IconButton.styleFrom(
                                  foregroundColor: theme.colorScheme.error,
                                  overlayColor: theme.colorScheme.error.withValues(alpha: 0.1),
                                ),
                                onPressed: () => _deleteBackup(entry),
                              ),
                            ],
                          ),
                        );
                      }).toList(),
                    ),
                  if (_backups.isNotEmpty)
                    Padding(
                      padding: const EdgeInsets.only(bottom: 12),
                      child: Center(
                        child: Text(
                          '${_backups.length} regular backup(s) · total ${_formatBytes(totalSize)}',
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                      ),
                    ),
                  const Divider(height: 1),
                  const SizedBox(height: 12),
                  // Special backups header
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
                            'Special Backups',
                            style: theme.textTheme.titleMedium?.copyWith(
                              fontWeight: FontWeight.w600,
                            ),
                          ),
                        ),
                        TextButton.icon(
                          onPressed: _isCreatingSpecial ? null : _createSpecialBackup,
                          icon: const Icon(Icons.add, size: 16),
                          label: const Text('Create'),
                        ),
                      ],
                    ),
                  ),
                  if (_isCreatingSpecial) ...[
                    const SizedBox(height: 8),
                    Padding(
                      padding: const EdgeInsets.symmetric(horizontal: 20),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          LinearProgressIndicator(
                            value: _specialBackupProgress > 0 ? _specialBackupProgress : null,
                            borderRadius: BorderRadius.circular(4),
                          ),
                          const SizedBox(height: 4),
                          Text(
                            _specialBackupProgress >= 1.0
                                ? 'Finishing...'
                                : _specialBackupProgress >= 0.9
                                    ? 'Writing file...'
                                    : _specialBackupProgress >= 0.5
                                        ? 'Encrypting...'
                                        : _specialBackupProgress >= 0.3
                                            ? 'Encoding...'
                                            : 'Reading data...',
                            style: theme.textTheme.bodySmall?.copyWith(
                              color: theme.colorScheme.onSurfaceVariant,
                            ),
                          ),
                        ],
                      ),
                    ),
                  ],
                  const SizedBox(height: 4),
                  if (_specialBackups.isEmpty)
                    Padding(
                      padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
                      child: Text(
                        'No special backups yet. Create one to preserve a specific version.',
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                    )
                  else
                    Padding(
                      padding: const EdgeInsets.only(bottom: 12),
                      child: Column(
                        children: _specialBackups.map((entry) {
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
                              '${entry.displayTime}  ·  ${_formatBytes(entry.sizeBytes)}',
                              style: theme.textTheme.bodySmall,
                            ),
                            trailing: Row(
                              mainAxisSize: MainAxisSize.min,
                              children: [
                                IconButton(
                                  icon: const Icon(Icons.edit, size: 18),
                                  tooltip: 'Rename',
                                  onPressed: () => _renameSpecialBackup(entry),
                                ),
                                IconButton(
                                  icon: const Icon(Icons.restore, size: 18),
                                  tooltip: 'Restore',
                                  onPressed: _isRestoring
                                      ? null
                                      : () => _restoreSpecialBackup(entry),
                                ),
                                IconButton(
                                  icon: const Icon(Icons.delete_outline, size: 18),
                                  tooltip: 'Delete',
                                  style: IconButton.styleFrom(
                                    foregroundColor: theme.colorScheme.error,
                                    overlayColor: theme.colorScheme.error.withValues(alpha: 0.1),
                                  ),
                                  onPressed: () => _deleteSpecialBackup(entry),
                                ),
                              ],
                            ),
                          );
                        }).toList(),
                      ),
                    ),
                  if (_specialBackups.isNotEmpty)
                    Padding(
                      padding: const EdgeInsets.only(bottom: 16),
                      child: Center(
                        child: Text(
                          '${_specialBackups.length} / ${BackupService.maxSpecialBackupCount} special backups',
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                      ),
                    ),
                ],
              ),
            ),
    );
  }

  String _formatBytes(int bytes) {
    if (bytes < 1024) return '$bytes B';
    if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
    return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
  }
}
