import 'dart:async' show unawaited;

import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:package_info_plus/package_info_plus.dart';

import 'package:solosoul_flutter/core/services/attachment_storage_service.dart';
import 'package:solosoul_flutter/core/services/backup_service.dart';
import 'package:solosoul_flutter/core/services/operation_logger.dart';
import 'package:solosoul_flutter/core/services/operation_notification.dart';
import 'package:solosoul_flutter/presentation/models/operation_log_models.dart';
import 'package:solosoul_flutter/presentation/providers/operation_log_provider.dart';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart' show AppTheme;
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';
import 'package:solosoul_flutter/presentation/utils/format_utils.dart';
import 'package:solosoul_flutter/presentation/widgets/data_management/vault_info_card.dart';
import 'package:solosoul_flutter/presentation/widgets/data_management/backup_section.dart';
import 'package:solosoul_flutter/presentation/widgets/data_management/restore_section.dart';
import 'package:solosoul_flutter/core/router/app_router.dart';
import 'package:go_router/go_router.dart';

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
  String _attachmentSize = '0 B';
  int _attachmentCount = 0;
  String _totalSize = '0 B';
  String? _appVersion;
  int _backupPoolSizeBytes = 0;

  @override
  void initState() {
    super.initState();
    _init();
  }

  Future<void> _init() async {
    final authNotifier = ref.read(authNotifierProvider.notifier);
    _accountId = authNotifier.selectedAccountId;

    final stats = await RustVaultService.instance.getVaultStats();
    final vaultSize = stats?.totalSizeBytes.toInt() ?? 0;
    _vaultDataSize = formatBytes(vaultSize);

    // 统计附件大小
    final accountId = _accountId;
    if (accountId != null) {
      final attachmentSize = await AttachmentStorageService().getTotalAttachmentSize(accountId);
      final attachmentCount = await AttachmentStorageService().getAttachmentCount(accountId);
      _attachmentSize = formatBytes(attachmentSize);
      _attachmentCount = attachmentCount;
      _totalSize = formatBytes(vaultSize + attachmentSize);
    }

    final packageInfo = await PackageInfo.fromPlatform();
    _appVersion = packageInfo.version;

    if (mounted) setState(() {});
    if (_accountId != null) await _loadAllBackups();
  }

  Future<void> _loadAllBackups() async {
    final accountId = _accountId;
    if (accountId == null) return;
    final regular = await BackupService.instance.listBackups(accountId);
    final special = await BackupService.instance.listSpecialBackups(accountId);
    final poolSize = await BackupService.instance.getAttachmentPoolSize(accountId);
    if (mounted) {
      setState(() {
        _backups = regular;
        _specialBackups = special;
        _backupPoolSizeBytes = poolSize;
        _isLoading = false;
      });
    }
  }

  Future<void> _loadBackups() async {
    final accountId = _accountId;
    if (accountId == null) return;
    final list = await BackupService.instance.listBackups(accountId);
    if (mounted) {
      setState(() {
        _backups = list;
      });
    }
  }

  Future<void> _loadSpecialBackups() async {
    final accountId = _accountId;
    if (accountId == null) return;
    final list = await BackupService.instance.listSpecialBackups(accountId);
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
        // Log backup creation
        unawaited(OperationLogService.instance.addEntry(
          OperationLogger.logBackup(
            action: LogAction.create,
            description: l10n.logBackupCreated(fileName),
            backupName: fileName,
            descriptionKey: 'createdBackup',
            descriptionArgs: {'name': fileName},
          ),
        ));
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
      // Log backup restore
      unawaited(OperationLogService.instance.addEntry(
        OperationLogger.logBackup(
          action: LogAction.restore,
          description: l10n.logBackupRestored(entry.fileName),
          backupName: entry.fileName,
          descriptionKey: 'restoredBackup',
          descriptionArgs: {'name': entry.fileName},
        ),
      ));
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
      // Log backup deletion
      unawaited(OperationLogService.instance.addEntry(
        OperationLogger.logBackup(
          action: LogAction.delete,
          description: AppLocalizations.of(context).logBackupDeleted(entry.fileName),
          backupName: entry.fileName,
          descriptionKey: 'deletedBackup',
          descriptionArgs: {'name': entry.fileName},
        ),
      ));
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
    final backupFilesSize = _backups.fold<int>(0, (s, e) => s + e.sizeBytes);
    final totalBackupSize = backupFilesSize + _backupPoolSizeBytes;

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
                  VaultInfoCard(
                    vaultDataSize: _vaultDataSize,
                    attachmentSize: _attachmentSize,
                    attachmentCount: _attachmentCount,
                    totalSize: _totalSize,
                  ),
                  const SizedBox(height: 16),
                  BackupSection(
                    isCreating: _isCreating,
                    backupProgress: _backupProgress,
                    backups: _backups,
                    isRestoring: _isRestoring,
                    totalSize: totalBackupSize,
                    backupPoolSize: formatBytes(_backupPoolSizeBytes),
                    onCreateBackup: _createBackup,
                    onRestoreBackup: _restoreBackup,
                    onDeleteBackup: _deleteBackup,
                    onPromoteBackup: _promoteToSpecialBackup,
                  ),
                  const Divider(height: 1),
                  const SizedBox(height: 12),
                  RestoreSection(
                    specialBackups: _specialBackups,
                    isCreatingSpecial: _isCreatingSpecial,
                    specialBackupProgress: _specialBackupProgress,
                    isRestoring: _isRestoring,
                    onCreateSpecialBackup: _createSpecialBackup,
                    onRestoreSpecialBackup: _restoreSpecialBackup,
                    onDeleteSpecialBackup: _deleteSpecialBackup,
                    onRenameSpecialBackup: _renameSpecialBackup,
                  ),
                  const Divider(height: 1),
                  const SizedBox(height: 12),
                  ListTile(
                    leading: const Icon(Icons.import_export),
                    title: Text(l10n.exportImportTitle),
                    subtitle: Text(l10n.exportImportSubtitle),
                    trailing: const Icon(Icons.chevron_right),
                    onTap: () => context.push(AppRoutes.exportImport),
                  ),
                ],
              ),
            ),
    );
  }
}

