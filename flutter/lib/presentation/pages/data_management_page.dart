import 'dart:async' show unawaited;

import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:package_info_plus/package_info_plus.dart';

import 'package:solosoul_flutter/core/services/backup_service.dart';
import 'package:solosoul_flutter/core/services/operation_notification.dart';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart' show AppTheme;
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';
import 'package:solosoul_flutter/presentation/utils/format_utils.dart';
import 'package:solosoul_flutter/presentation/widgets/data_management/backup_list_tile.dart';
import 'package:solosoul_flutter/presentation/widgets/data_management/backup_progress_indicator.dart';

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
  // Initialized with localized fallback in _init()
  String _vaultDataSize = '';
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
      _vaultDataSize = formatBytes(stats.totalSizeBytes.toInt());
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
    final l10n = AppLocalizations.of(context);
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
          message: OperationMessage(
            type: OperationType.create,
            section: 'backup',
            customMessage: l10n.dataMgmtBackupCreated,
          ),
        );
      } else {
        OperationNotification.show(
          context,
          message: OperationMessage(
            type: OperationType.delete,
            section: 'backup',
            customMessage: l10n.dataMgmtBackupFailed,
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
          customMessage: l10n.dataMgmtBackupError(e.toString()),
        ),
      );
    }
  }

  Future<bool> _showConfirmDialog({
    required String title,
    required String content,
    String? confirmLabel,
  }) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(title),
        content: Text(content),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(false),
            child: Text(AppLocalizations.of(ctx).commonCancel),
          ),
          FilledButton(
            onPressed: () => Navigator.of(ctx).pop(true),
            child: Text(confirmLabel ?? AppLocalizations.of(ctx).commonConfirm),
          ),
        ],
      ),
    );
    return confirmed == true;
  }

  Future<void> _restoreBackup(BackupEntry entry) async {
    final l10n = AppLocalizations.of(context);
    if (_accountId == null) return;
    final confirmed = await _showConfirmDialog(
      title: AppLocalizations.of(context).dataMgmtRestoreBackup,
      content: l10n.dataMgmtRestoreOverwrite(entry.displayTime),
      confirmLabel: l10n.dataManagementRestoreBackupTooltip,
    );
    if (!confirmed || !mounted) return;

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
        message: OperationMessage(
          type: OperationType.restore,
          section: 'backup',
          customMessage: AppLocalizations.of(context).dataMgmtRestoreSuccess,
        ),
        duration: AppTheme.kPasswordHintDelay,
      );
    } else {
      OperationNotification.show(
        context,
        message: OperationMessage(
          type: OperationType.purge,
          section: 'backup',
          customMessage: AppLocalizations.of(context).dataMgmtRestoreFailed,
        ),
      );
    }
  }

  Future<void> _deleteBackup(BackupEntry entry) async {
    if (_accountId == null) return;

    // Step 1: Confirm deletion
    final confirmed = await _showConfirmDialog(
      title: AppLocalizations.of(context).dataMgmtDeleteBackup,
      content: AppLocalizations.of(context).dataMgmtDeleteBackupConfirm(entry.displayTime),
      confirmLabel: AppLocalizations.of(context).commonDelete,
    );
    if (!confirmed) return;
    if (!mounted) return;

    // Step 2: Verify password before destructive action
    final authNotifier = ref.read(authNotifierProvider.notifier);
    final selectedAccount = authNotifier.selectedAccount;
    final password = await showPasswordVerificationDialog(
      context: context,
      ref: ref,
      message: AppLocalizations.of(context).dataMgmtConfirmDeletion,
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
        message: OperationMessage(
          type: OperationType.purge,
          section: 'backup',
          customMessage: AppLocalizations.of(context).dataMgmtBackupDeleted,
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
            title: Text(AppLocalizations.of(context).dataManagementSpecialBackupLimit),
            content: Text(
              AppLocalizations.of(context).dataMgmtSpecialBackupPromoteLimit(BackupService.maxSpecialBackupCount),
            ),
            actions: [
              TextButton(
                onPressed: () => Navigator.of(ctx).pop(),
                child: Text(AppLocalizations.of(context).settingsOk),
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
          title: Text(AppLocalizations.of(context).dataManagementNameBackup),
          content: TextField(
            controller: controller,
            autofocus: true,
            decoration: InputDecoration(
              hintText: AppLocalizations.of(context).dataManagementBackupNameHint,
              labelText: AppLocalizations.of(context).dataManagementBackupNameLabel,
            ),
            maxLength: 50,
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(ctx).pop(),
              child: Text(AppLocalizations.of(context).commonCancel),
            ),
            FilledButton(
              onPressed: () {
                final text = controller.text.trim();
                if (text.isNotEmpty) Navigator.of(ctx).pop(text);
              },
              child: Text(AppLocalizations.of(context).commonSave),
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
          customMessage: AppLocalizations.of(context).dataMgmtSpecialBackupSaved(name),
        ),
      );
    } else {
      if (!mounted) return;
      OperationNotification.show(
        context,
        message: OperationMessage(
          type: OperationType.delete,
          section: 'backup',
          customMessage: AppLocalizations.of(context).dataMgmtSpecialBackupFailed,
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
            title: Text(AppLocalizations.of(context).dataManagementSpecialBackupLimit),
            content: Text(
              AppLocalizations.of(context).dataMgmtSpecialBackupLimit(BackupService.maxSpecialBackupCount),
            ),
            actions: [
              TextButton(
                onPressed: () => Navigator.of(ctx).pop(),
                child: Text(AppLocalizations.of(context).settingsOk),
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
          title: Text(AppLocalizations.of(context).dataManagementNameBackup),
          content: TextField(
            controller: controller,
            autofocus: true,
            decoration: InputDecoration(
              hintText: AppLocalizations.of(context).dataManagementBackupNameHint,
              labelText: AppLocalizations.of(context).dataManagementBackupNameLabel,
            ),
            maxLength: 50,
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(ctx).pop(),
              child: Text(AppLocalizations.of(context).commonCancel),
            ),
            FilledButton(
              onPressed: () {
                final text = controller.text.trim();
                if (text.isNotEmpty) Navigator.of(ctx).pop(text);
              },
              child: Text(AppLocalizations.of(context).dataManagementCreate),
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
          customMessage: AppLocalizations.of(context).dataMgmtSpecialBackupCreated(name),
        ),
      );
    } else {
      OperationNotification.show(
        context,
        message: OperationMessage(
          type: OperationType.delete,
          section: 'backup',
          customMessage: AppLocalizations.of(context).dataMgmtSpecialBackupCreateFailed,
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
          title: Text(AppLocalizations.of(context).dataManagementRenameBackup),
          content: TextField(
            controller: controller,
            autofocus: true,
            decoration: InputDecoration(
              labelText: AppLocalizations.of(context).dataManagementNewName,
            ),
            maxLength: 50,
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(ctx).pop(),
              child: Text(AppLocalizations.of(context).commonCancel),
            ),
            FilledButton(
              onPressed: () {
                final text = controller.text.trim();
                if (text.isNotEmpty) Navigator.of(ctx).pop(text);
              },
              child: Text(AppLocalizations.of(context).dataManagementRename),
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
          customMessage: AppLocalizations.of(context).dataMgmtRenamedTo(newName),
        ),
      );
    }
  }

  Future<void> _restoreSpecialBackup(BackupEntry entry) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(AppLocalizations.of(context).dataManagementRestoreBackupTitle),
        content: Text(
          '${AppLocalizations.of(context).dataManagementRestoreBackupConfirm(entry.fileName.replaceAll('.backup', ''))}\n'
          '${AppLocalizations.of(context).dataMgmtSafetyBackupNotice}',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(false),
            child: Text(AppLocalizations.of(context).commonCancel),
          ),
          FilledButton(
            onPressed: () => Navigator.of(ctx).pop(true),
            child: Text(AppLocalizations.of(context).commonConfirm),
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
        message: OperationMessage(
          type: OperationType.restore,
          section: 'backup',
          customMessage: AppLocalizations.of(context).dataMgmtSpecialBackupRestored,
        ),
        duration: AppTheme.kPasswordHintDelay,
      );
    } else {
      OperationNotification.show(
        context,
        message: OperationMessage(
          type: OperationType.purge,
          section: 'backup',
          customMessage: AppLocalizations.of(context).dataMgmtRestoreFailed,
        ),
      );
    }
  }

  Future<void> _deleteSpecialBackup(BackupEntry entry) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(AppLocalizations.of(context).dataManagementDeleteBackupTitle),
        content: Text(AppLocalizations.of(context).dataManagementDeleteBackupConfirm(entry.fileName.replaceAll('.backup', ''))),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(false),
            child: Text(AppLocalizations.of(context).commonCancel),
          ),
          FilledButton(
            onPressed: () => Navigator.of(ctx).pop(true),
            style: FilledButton.styleFrom(
              backgroundColor: Colors.red,
              foregroundColor: Colors.white,
            ),
            child: Text(AppLocalizations.of(context).commonDelete),
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
    final l10n = AppLocalizations.of(context);
    final totalSize = _backups.fold<int>(0, (s, e) => s + e.sizeBytes);

    return Scaffold(
      appBar: SoloGlassAppBar(
        title: Text(l10n.dataManagementTitle),
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
                  _VaultInfoCard(vaultDataSize: _vaultDataSize),
                  const SizedBox(height: 16),
                  _BackupSection(
                    isCreating: _isCreating,
                    backupProgress: _backupProgress,
                    backups: _backups,
                    isRestoring: _isRestoring,
                    totalSize: totalSize,
                    onCreateBackup: _createBackup,
                    onRestoreBackup: _restoreBackup,
                    onDeleteBackup: _deleteBackup,
                    onPromoteBackup: _promoteToSpecialBackup,
                  ),
                  const Divider(height: 1),
                  const SizedBox(height: 12),
                  _RestoreSection(
                    specialBackups: _specialBackups,
                    isCreatingSpecial: _isCreatingSpecial,
                    specialBackupProgress: _specialBackupProgress,
                    isRestoring: _isRestoring,
                    onCreateSpecialBackup: _createSpecialBackup,
                    onRestoreSpecialBackup: _restoreSpecialBackup,
                    onDeleteSpecialBackup: _deleteSpecialBackup,
                    onRenameSpecialBackup: _renameSpecialBackup,
                  ),
                ],
              ),
            ),
    );
  }
}

// =============================================================================
// Extracted widgets
// =============================================================================

class _VaultInfoCard extends StatelessWidget {
  final String vaultDataSize;

  const _VaultInfoCard({required this.vaultDataSize});

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 20),
      child: Row(
        children: [
          Text(
            l10n.dataMgmtVaultSize,
            style: theme.textTheme.bodyMedium?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
          Text(
            vaultDataSize,
            style: theme.textTheme.bodyMedium?.copyWith(
              fontWeight: FontWeight.w600,
            ),
          ),
        ],
      ),
    );
  }
}

class _BackupSection extends StatelessWidget {
  final bool isCreating;
  final double backupProgress;
  final List<BackupEntry> backups;
  final bool isRestoring;
  final int totalSize;
  final VoidCallback onCreateBackup;
  final ValueChanged<BackupEntry> onRestoreBackup;
  final ValueChanged<BackupEntry> onDeleteBackup;
  final ValueChanged<BackupEntry> onPromoteBackup;

  const _BackupSection({
    required this.isCreating,
    required this.backupProgress,
    required this.backups,
    required this.isRestoring,
    required this.totalSize,
    required this.onCreateBackup,
    required this.onRestoreBackup,
    required this.onDeleteBackup,
    required this.onPromoteBackup,
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
                l10n.dataManagementBackupsSummary(backups.length, formatBytes(totalSize)),
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

class _RestoreSection extends StatelessWidget {
  final List<BackupEntry> specialBackups;
  final bool isCreatingSpecial;
  final double specialBackupProgress;
  final bool isRestoring;
  final VoidCallback onCreateSpecialBackup;
  final ValueChanged<BackupEntry> onRestoreSpecialBackup;
  final ValueChanged<BackupEntry> onDeleteSpecialBackup;
  final ValueChanged<BackupEntry> onRenameSpecialBackup;

  const _RestoreSection({
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
