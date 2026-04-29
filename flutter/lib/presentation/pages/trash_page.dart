// ignore_for_file: prefer_const_declarations

import 'package:flutter/material.dart';

// ignore_for_file: use_build_context_synchronously
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    show AppTheme, showOverlaySnackBar, SnackBarType;
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart';
import 'package:solosoul_flutter/presentation/utils/list_utils.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/core/services/operation_notification.dart';
import 'package:solosoul_flutter/core/services/operation_logger.dart';
import 'package:solosoul_flutter/presentation/models/operation_log_models.dart';
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';
import 'package:solosoul_flutter/presentation/widgets/field_history_view.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart';
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';

class TrashPage extends ConsumerStatefulWidget {
  const TrashPage({super.key});

  @override
  ConsumerState<TrashPage> createState() => _TrashPageState();
}

class _TrashPageState extends ConsumerState<TrashPage> {
  final _searchController = TextEditingController();
  String _searchQuery = '';

  // Pre-computed sensitivity levels by item type
  Map<String, SensitivityLevel> _sensitivityByItemType = {};

  /// Whether we have already shown the auto-prompt dialog on first build.
  bool _hasPromptedForVerification = false;

  @override
  void initState() {
    super.initState();
    // Load profile and field histories if not already loaded
    WidgetsBinding.instance.addPostFrameCallback((_) {
      ref.read(profileNotifierProvider.notifier).loadProfile();
      ref.read(fieldHistoriesProvider.notifier).loadHistories();
    });
  }

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  Future<void> _verifyPassword() async {
    // Use the shared password verification dialog with biometric support
    final authNotifier = ref.read(authNotifierProvider.notifier);
    final selectedAccount = authNotifier.selectedAccount;
    final result = await showPasswordVerificationDialog(
      context: context,
      ref: ref,
      message: 'Enter your master password to view the trash.',
      passwordHint: selectedAccount?.passwordHint,
      onVerify: authNotifier.verifyPasswordForSensitiveData,
    );

    if (!mounted) return;

    if (result != null) {
      // Mark as verified in shared sensitive page access
      ref.read(sensitivePageAccessProvider.notifier).markVerified();
    }
  }

  @override
  Widget build(BuildContext context) {
    // Show password verification dialog if not yet verified
    if (!ref.watch(isSensitiveAccessGrantedProvider)) {
      // Auto-prompt only on first build; after cancellation show Verify button
      if (!_hasPromptedForVerification) {
        _hasPromptedForVerification = true;
        WidgetsBinding.instance.addPostFrameCallback((_) {
          _verifyPassword();
        });
      }
      return Scaffold(
        appBar: AppBar(title: const Text('Trash')),
        body: Center(
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Icon(
                Icons.lock_outline,
                size: 64,
                color: Theme.of(context).colorScheme.primary,
              ),
              const SizedBox(height: 24),
              Text(
                'Password Required',
                style: Theme.of(context).textTheme.headlineSmall,
              ),
              const SizedBox(height: 16),
              FilledButton.icon(
                onPressed: _verifyPassword,
                icon: const Icon(Icons.lock_open),
                label: const Text('Verify'),
              ),
            ],
          ),
        ),
      );
    }
    return _buildTrashView();
  }

  Widget _buildTrashView() {
    final theme = Theme.of(context);
    final profile = ref.watch(profileNotifierProvider).value;

    // Always show scaffold with search bar and warning banner
    return Scaffold(
      appBar: AppBar(
        title: const Text('Trash'),
        actions: const [HeaderActionButtons()],
      ),
      body: Column(
        children: [
          // Search bar
          Padding(
            padding: const EdgeInsets.all(16),
            child: TextField(
              controller: _searchController,
              onChanged: (value) => setState(() => _searchQuery = value),
              decoration: InputDecoration(
                hintText: 'Search trash...',
                prefixIcon: const Icon(Icons.search),
                suffixIcon: _searchQuery.isNotEmpty
                    ? IconButton(
                        icon: const Icon(Icons.clear),
                        onPressed: () {
                          _searchController.clear();
                          setState(() => _searchQuery = '');
                        },
                      )
                    : null,
                border: OutlineInputBorder(
                  borderRadius: BorderRadius.circular(12),
                ),
                contentPadding: const EdgeInsets.symmetric(
                  horizontal: 16,
                  vertical: 12,
                ),
              ),
            ),
          ),

          // Trash info banner - always shown
          Container(
            margin: const EdgeInsets.symmetric(horizontal: 16),
            padding: const EdgeInsets.all(16),
            decoration: BoxDecoration(
              color: Colors.orange.withValues(alpha: 0.1),
              borderRadius: BorderRadius.circular(12),
              border: Border.all(color: Colors.orange.withValues(alpha: 0.3)),
            ),
            child: Row(
              children: [
                Icon(Icons.warning_amber, color: Colors.orange.shade700),
                const SizedBox(width: 12),
                Expanded(
                  child: Text(
                    'Items in trash are permanently deleted after 30 days',
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: Colors.orange.shade700,
                    ),
                  ),
                ),
              ],
            ),
          ).animate().fadeIn(duration: 400.ms),

          const SizedBox(height: 16),

          // Main content area
          Expanded(child: _buildTrashContent(theme, profile)),
        ],
      ),
    );
  }

  Widget _buildTrashContent(ThemeData theme, profile) {
    if (profile == null) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              Icons.delete_outline,
              size: 64,
              color: theme.colorScheme.onSurfaceVariant,
            ),
            const SizedBox(height: 16),
            Text(
              'Trash is empty',
              style: theme.textTheme.titleMedium?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(height: 8),
            Text(
              'Deleted items will appear here',
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ),
      );
    }

    final deletedItems = ProfileStorageService.instance.getDeletedItems(profile);
    final deletedUnifiedObjects = ref.watch(deletedObjectsProvider);

    // Pre-compute sensitivity levels by item type (once, not per-item)
    final sensitivityByItemType = <String, SensitivityLevel>{};
    for (final type in DeletedItemInfo.itemTypes) {
      final fieldId = _getFieldIdForItem(type);
      if (fieldId != null) {
        sensitivityByItemType[type] =
            ref.watch(effectiveSensitivityProvider(fieldId));
      }
    }
    _sensitivityByItemType = sensitivityByItemType;

    // Filter items based on search query
    final query = _searchQuery.toLowerCase();
    final filteredItems = _searchQuery.isEmpty
        ? deletedItems
        : deletedItems.where((item) {
            return item.itemLabel.toLowerCase().contains(query) ||
                item.section.toLowerCase().contains(query) ||
                item.itemType.toLowerCase().contains(query);
          }).toList();

    final filteredUnifiedObjects = _searchQuery.isEmpty
        ? deletedUnifiedObjects
        : deletedUnifiedObjects.where((obj) {
            return obj.name.toLowerCase().contains(query) ||
                (obj.typeId?.toLowerCase().contains(query) ?? false);
          }).toList();

    final totalCount = filteredItems.length + filteredUnifiedObjects.length;

    // Results count when searching
    if (_searchQuery.isNotEmpty) {
      Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16),
        child: Row(
          children: [
            Text(
              totalCount > 0
                  ? 'Found $totalCount result(s)'
                  : 'No results found',
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: totalCount > 0
                        ? Theme.of(context).colorScheme.onSurfaceVariant
                        : Colors.orange,
                  ),
            ),
            const Spacer(),
            Text(
              '${deletedItems.length + deletedUnifiedObjects.length} total items in trash',
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: Theme.of(context).colorScheme.onSurfaceVariant,
                  ),
            ),
          ],
        ),
      );

      const SizedBox(height: 8);
    }

    // Empty state
    if (totalCount == 0) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              _searchQuery.isEmpty ? Icons.delete_outline : Icons.search_off,
              size: 64,
              color: theme.colorScheme.onSurfaceVariant,
            ),
            const SizedBox(height: 16),
            Text(
              _searchQuery.isEmpty ? 'Trash is empty' : 'No matching items',
              style: theme.textTheme.titleMedium?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(height: 8),
            Text(
              _searchQuery.isEmpty
                  ? 'Deleted items will appear here'
                  : 'Try adjusting your search',
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ),
      );
    }

    // Items list with empty trash action
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.end,
            children: [
              TextButton.icon(
                onPressed: () =>
                    _confirmEmptyTrash(context, totalCount),
                icon: const Icon(
                  Icons.delete_forever,
                  color: AppTheme.errorColor,
                ),
                label: const Text(
                  'Empty Trash',
                  style: TextStyle(color: AppTheme.errorColor),
                ),
              ),
            ],
          ),
        ),
        Expanded(
          child: ListView(
            padding: const EdgeInsets.all(16),
            children: [
              // Unified Objects section
              if (filteredUnifiedObjects.isNotEmpty) ...[
                Text(
                  'Pages & Objects',
                  style: theme.textTheme.titleSmall?.copyWith(
                    fontWeight: FontWeight.w600,
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
                const SizedBox(height: 8),
                ...filteredUnifiedObjects.map((obj) => _UnifiedObjectTrashCard(
                  object: obj,
                  onRestore: () => _confirmRestoreUnifiedObject(context, obj),
                  onPurge: () => _confirmPurgeUnifiedObject(context, obj),
                )),
                if (filteredItems.isNotEmpty) const SizedBox(height: 24),
              ],
              // Legacy Items section
              if (filteredItems.isNotEmpty) ...[
                if (filteredUnifiedObjects.isNotEmpty)
                  Text(
                    'Legacy Items',
                    style: theme.textTheme.titleSmall?.copyWith(
                      fontWeight: FontWeight.w600,
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                if (filteredUnifiedObjects.isNotEmpty) const SizedBox(height: 8),
                ...filteredItems.asMap().entries.map((entry) {
                  final index = entry.key;
                  final item = entry.value;
                  final hasHistory = _itemHasHistory(item);
                  return Padding(
                    padding: EdgeInsets.only(
                      bottom: index < filteredItems.length - 1 ? 8 : 0,
                    ),
                    child: _TrashItemCard(
                      item: item,
                      hasHistory: hasHistory,
                      sensitivityLevel:
                          _sensitivityByItemType[item.itemType] ??
                              SensitivityLevel.public,
                      onRestore: (item) => _confirmRestore(item),
                      onPurge: (item) => _confirmPurge(context, item),
                      onDetail: () => _showDetail(context, item),
                      onHistory: hasHistory
                          ? () => _showHistoryForItem(context, item)
                          : null,
                    ),
                  );
                }),
              ],
            ],
          ),
        ),
      ],
    );
  }

  Future<void> _restoreItem(DeletedItemInfo item) async {
    await ref
        .read(profileNotifierProvider.notifier)
        .restore(section: item.section, itemType: item.itemType, id: item.id);

    if (mounted) {
      // Rebuild triggered by provider state change
      

      OperationNotification.show(
        context,
        message: OperationLogger.createNotification(
          section: _getLogSection(item.section),
          action: LogAction.restore,
          itemName: item.itemLabel,
        ),
        duration: const Duration(seconds: 3),
      );
    }
  }

  Future<void> _confirmRestore(DeletedItemInfo item) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Row(
          children: [
            Icon(Icons.restore, color: AppTheme.primaryColor),
            SizedBox(width: 8),
            Text('Confirm Restore'),
          ],
        ),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'Are you sure you want to restore "${item.itemLabel}"?',
              style: Theme.of(ctx).textTheme.bodyMedium,
            ),
            const SizedBox(height: 12),
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: Colors.blue.shade50,
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: Colors.blue.shade200),
              ),
              child: Row(
                children: [
                  Icon(
                    Icons.info_outline,
                    color: Colors.blue.shade700,
                    size: 20,
                  ),
                  const SizedBox(width: 8),
                  const Expanded(
                    child: Text(
                      'The item will be moved back to its original location.',
                      style: TextStyle(
                        color: Colors.blue,
                        fontSize: 13,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('Restore'),
          ),
        ],
      ),
    );
    if (confirmed == true) {
      await _restoreItem(item);
    }
  }

  Future<void> _confirmPurge(BuildContext context, DeletedItemInfo item) async {
    await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Row(
          children: [
            Icon(Icons.warning_amber, color: Colors.orange.shade700),
            const SizedBox(width: 8),
            const Text('Confirm Permanent Delete'),
          ],
        ),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'Are you sure you want to permanently delete "${item.itemLabel}"?',
              style: Theme.of(context).textTheme.bodyMedium,
            ),
            const SizedBox(height: 12),
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: Colors.red.shade50,
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: Colors.red.shade200),
              ),
              child: Row(
                children: [
                  Icon(
                    Icons.info_outline,
                    color: Colors.red.shade700,
                    size: 20,
                  ),
                  const SizedBox(width: 8),
                  const Expanded(
                    child: Text(
                      'This action cannot be undone. The item will be permanently removed.',
                      style: TextStyle(
                        color: Colors.red,
                        fontSize: 13,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            style: FilledButton.styleFrom(backgroundColor: AppTheme.errorColor),
            child: const Text('Delete Forever'),
          ),
        ],
      ),
    ).then((confirmed) async {
      if (confirmed == true) {
        await _purgeItem(item);
      }
    });
  }

  Future<void> _purgeItem(DeletedItemInfo item) async {
    await ref
        .read(profileNotifierProvider.notifier)
        .permanentDelete(
          section: item.section,
          itemType: item.itemType,
          id: item.id,
        );

    if (mounted) {
      // Rebuild triggered by provider state change
      
      showOverlaySnackBar(
        context,
        content: '${item.itemLabel} permanently deleted',
        type: SnackBarType.error,
      );
    }
  }

  void _confirmEmptyTrash(BuildContext context, int itemCount) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: Row(
          children: [
            Icon(Icons.warning_amber, color: Colors.orange.shade700),
            const SizedBox(width: 8),
            const Text('Empty Trash'),
          ],
        ),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'Are you sure you want to permanently delete all $itemCount items in trash?',
              style: Theme.of(context).textTheme.bodyMedium,
            ),
            const SizedBox(height: 12),
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: Colors.red.shade50,
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: Colors.red.shade200),
              ),
              child: Row(
                children: [
                  Icon(
                    Icons.info_outline,
                    color: Colors.red.shade700,
                    size: 20,
                  ),
                  const SizedBox(width: 8),
                  const Expanded(
                    child: Text(
                      'This action cannot be undone. All items will be permanently removed.',
                      style: TextStyle(
                        color: Colors.red,
                        fontSize: 13,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () async {
              final snackBarContext = context;
              final overlaySnackBar = showOverlaySnackBar;
              Navigator.pop(snackBarContext);
              await ref.read(profileNotifierProvider.notifier).emptyAllTrash();
              if (mounted) {
                
                overlaySnackBar(
                  snackBarContext,
                  content: 'All $itemCount items permanently deleted',
                  type: SnackBarType.error,
                );
              }
            },
            style: FilledButton.styleFrom(backgroundColor: AppTheme.errorColor),
            child: const Text('Empty Trash'),
          ),
        ],
      ),
    );
  }

  void _showDetail(BuildContext context, DeletedItemInfo item) {
    final theme = Theme.of(context);
    final profile = ref.read(profileNotifierProvider).value;
    if (profile == null) return;

    String detailText = '';
    DateTime? deletedAt;

    switch (item.section) {
      case 'travel':
        if (item.itemType == 'passport') {
          final idx =
              profile.travel?.passports.indexById(item.id, (p) => p.id) ??
              -1;
          if (idx >= 0) {
            final p = profile.travel!.passports[idx];
            detailText =
                'Country: ${p.country ?? "N/A"}\n'
                'Number: ${p.number ?? "N/A"}\n'
                'Expiry: ${p.expiryDate ?? "N/A"}';
            deletedAt = p.deletedAt;
          }
        } else if (item.itemType == 'visa') {
          final idx =
              profile.travel?.visas.indexById(item.id, (v) => v.id) ?? -1;
          if (idx >= 0) {
            final v = profile.travel!.visas[idx];
            detailText =
                'Country: ${v.country ?? "N/A"}\n'
                'Type: ${v.visaType ?? "N/A"}\n'
                'Number: ${v.number ?? "N/A"}\n'
                'Expiry: ${v.expiryDate ?? "N/A"}';
            deletedAt = v.deletedAt;
          }
        } else if (item.itemType == 'travel_history') {
          final idx =
              profile.travel?.travelHistory.indexById(item.id, (t) => t.id) ??
              -1;
          if (idx >= 0) {
            final t = profile.travel!.travelHistory[idx];
            detailText =
                'Destination: ${t.destination}\n'
                'Date: ${t.date ?? "N/A"}';
            deletedAt = t.deletedAt;
          }
        }
        break;
      case 'financial':
        if (item.itemType == 'bank_account') {
          final idx =
              profile.financial?.bankAccounts.indexById(item.id, (b) => b.id) ??
              -1;
          if (idx >= 0) {
            final b = profile.financial!.bankAccounts[idx];
            detailText =
                'Bank: ${b.bankName ?? "N/A"}\n'
                'Account: ${b.accountNumber ?? "N/A"}\n'
                'Currency: ${b.currency ?? "N/A"}';
            deletedAt = b.deletedAt;
          }
        } else if (item.itemType == 'card') {
          final idx =
              profile.financial?.cards.indexById(item.id, (c) => c.id) ?? -1;
          if (idx >= 0) {
            final c = profile.financial!.cards[idx];
            detailText =
                'Type: ${c.cardType ?? "N/A"}\n'
                'Number: ${c.cardNumber ?? "N/A"}\n'
                'Expiry: ${c.expiryDate ?? "N/A"}';
            deletedAt = c.deletedAt;
          }
        }
        break;
      case 'professional':
        if (item.itemType == 'education') {
          final idx =
              profile.professional?.education.indexById(item.id, (e) => e.id) ??
              -1;
          if (idx >= 0) {
            final e = profile.professional!.education[idx];
            detailText =
                'Institution: ${e.institution ?? "N/A"}\n'
                'Degree: ${e.degree ?? "N/A"}\n'
                'Field: ${e.field ?? "N/A"}';
            deletedAt = e.deletedAt;
          }
        } else if (item.itemType == 'employment') {
          final idx =
              profile.professional?.employment.indexById(item.id, (emp) => emp.id) ??
              -1;
          if (idx >= 0) {
            final emp = profile.professional!.employment[idx];
            detailText =
                'Company: ${emp.company ?? "N/A"}\n'
                'Position: ${emp.position ?? "N/A"}\n'
                'Period: ${emp.startDate ?? "N/A"} - ${emp.endDate ?? "N/A"}';
            deletedAt = emp.deletedAt;
          }
        } else if (item.itemType == 'skill') {
          final idx =
              profile.professional?.skills.indexById(item.id, (s) => s.id) ??
              -1;
          if (idx >= 0) {
            final s = profile.professional!.skills[idx];
            detailText =
                'Name: ${s.name}\n'
                'Level: ${s.level ?? "N/A"}';
            deletedAt = s.deletedAt;
          }
        } else if (item.itemType == 'language') {
          final idx =
              profile.professional?.languages.indexById(item.id, (l) => l.id) ??
              -1;
          if (idx >= 0) {
            final l = profile.professional!.languages[idx];
            detailText =
                'Name: ${l.name}\n'
                'Proficiency: ${l.proficiency ?? "N/A"}';
            deletedAt = l.deletedAt;
          }
        }
        break;
      case 'profile':
        if (item.itemType == 'contact') {
          final idx =
              profile.identity?.contact?.entries.indexById(item.id, (e) => e.id) ??
              -1;
          if (idx >= 0) {
            final e = profile.identity!.contact!.entries[idx];
            detailText =
                'Title: ${e.title}\n'
                'Type: ${e.type}\n'
                'Value: ${e.value}';
            deletedAt = e.deletedAt;
          }
        } else if (item.itemType == 'idCard') {
          final idx =
              profile.identity?.idCards?.indexById(item.id, (c) => c.id) ??
              -1;
          if (idx >= 0) {
            final c = profile.identity!.idCards![idx];
            detailText =
                'Label: ${c.title ?? "N/A"}\n'
                'Number: ${c.number ?? "N/A"}\n'
                'Country: ${c.country ?? "N/A"}';
            deletedAt = c.deletedAt;
          }
        } else if (item.itemType == 'address') {
          final idx =
              profile.identity?.addresses?.indexById(item.id, (a) => a.id) ??
              -1;
          if (idx >= 0) {
            final a = profile.identity!.addresses![idx];
            detailText =
                'Label: ${a.title ?? "N/A"}\n'
                'Street: ${a.street ?? "N/A"}\n'
                'City: ${a.city ?? "N/A"}\n'
                'Country: ${a.country ?? "N/A"}';
            deletedAt = a.deletedAt;
          }
        }
        break;
    }

    final deletedAtDate = deletedAt ?? item.deletedAt;
    final daysRemaining = 30 - DateTime.now().difference(deletedAtDate).inDays;

    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: Row(
          children: [
            Icon(item.meta?.icon ?? Icons.help_outline, color: AppTheme.primaryColor),
            const SizedBox(width: 8),
            Expanded(child: Text(item.itemLabel)),
          ],
        ),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: theme.colorScheme.surfaceContainerHighest.withValues(
                  alpha: 0.3,
                ),
                borderRadius: BorderRadius.circular(8),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    '${item.meta?.label ?? item.itemType} - ${item.meta?.sectionLabel ?? item.section}',
                    style: Theme.of(context).textTheme.labelMedium?.copyWith(
                      color: AppTheme.primaryColor,
                    ),
                  ),
                  const SizedBox(height: 8),
                  Text(
                    detailText.isNotEmpty ? detailText : 'No details available',
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                ],
              ),
            ),
            const SizedBox(height: 16),
            Row(
              children: [
                Icon(Icons.access_time, size: 16, color: Colors.grey.shade600),
                const SizedBox(width: 8),
                Text(
                  'Deleted: ${_formatDate(deletedAtDate)}',
                  style: Theme.of(
                    context,
                  ).textTheme.bodySmall?.copyWith(color: Colors.grey.shade600),
                ),
              ],
            ),
            const SizedBox(height: 4),
            Row(
              children: [
                Icon(
                  Icons.timer,
                  size: 16,
                  color: daysRemaining <= 7
                      ? Colors.orange
                      : Colors.grey.shade600,
                ),
                const SizedBox(width: 8),
                Text(
                  '$daysRemaining days until permanent deletion',
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: daysRemaining <= 7
                        ? Colors.orange
                        : Colors.grey.shade600,
                    fontWeight: daysRemaining <= 7
                        ? FontWeight.w600
                        : FontWeight.normal,
                  ),
                ),
              ],
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

  bool _itemHasHistory(DeletedItemInfo item) {
    final fieldIdPrefix = item.meta?.fieldIdPrefix;
    if (fieldIdPrefix == null) return false;

    final history = ref
        .read(fieldHistoriesProvider.notifier)
        .getHistory(item.id, fieldIdPrefix);
    return history != null && history.entries.isNotEmpty;
  }

  LogSection _getLogSection(String section) {
    switch (section) {
      case 'travel':
        return LogSection.travel;
      case 'financial':
        return LogSection.financial;
      case 'professional':
        return LogSection.professional;
      case 'profile':
        return LogSection.identity;
      default:
        return LogSection.identity;
    }
  }

  String _formatDate(DateTime date) {
    return '${date.year}-${date.month.toString().padLeft(2, '0')}-${date.day.toString().padLeft(2, '0')}';
  }

  String? _getFieldIdForItem(String itemType) {
    // Get sensitivity field ID from meta configuration
    return DeletedItemInfo.metaFor(itemType)?.sensitivityFieldId;
  }

  void _showHistoryForItem(BuildContext context, DeletedItemInfo item) {
    final fieldIdPrefix = item.meta?.fieldIdPrefix;

    if (fieldIdPrefix == null) {
      showOverlaySnackBar(
        context,
        content: 'History not available for this item type',
        type: SnackBarType.info,
      );
      return;
    }

    final history = ref
        .read(fieldHistoriesProvider.notifier)
        .getHistory(item.id, fieldIdPrefix);

    if (history == null || history.entries.isEmpty) {
      showOverlaySnackBar(
        context,
        content: 'No history available for this item',
        type: SnackBarType.info,
      );
      return;
    }

    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: Row(
          children: [
            const Icon(Icons.history, color: AppTheme.primaryColor),
            const SizedBox(width: 8),
            Expanded(child: Text('${item.itemLabel} - History')),
          ],
        ),
        content: SizedBox(
          width: double.maxFinite,
          child: FieldHistoryView(fieldName: fieldIdPrefix, history: history),
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

  // ---------------------------------------------------------------------------
  // Unified Object Trash Actions
  // ---------------------------------------------------------------------------

  Future<void> _confirmRestoreUnifiedObject(
    BuildContext context,
    UnifiedObject object,
  ) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Row(
          children: [
            Icon(Icons.restore, color: AppTheme.primaryColor),
            SizedBox(width: 8),
            Text('Confirm Restore'),
          ],
        ),
        content: Text(
          'Are you sure you want to restore "${object.name}"?',
          style: Theme.of(ctx).textTheme.bodyMedium,
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('Restore'),
          ),
        ],
      ),
    );
    if (confirmed == true) {
      await ref.read(unifiedObjectProvider.notifier).restoreObject(object.id);
      if (mounted) {
        
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Restored "${object.name}"')),
        );
      }
    }
  }

  Future<void> _confirmPurgeUnifiedObject(
    BuildContext context,
    UnifiedObject object,
  ) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Row(
          children: [
            Icon(Icons.warning_amber, color: Colors.orange.shade700),
            const SizedBox(width: 8),
            const Text('Confirm Permanent Delete'),
          ],
        ),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'Are you sure you want to permanently delete "${object.name}"?',
              style: Theme.of(ctx).textTheme.bodyMedium,
            ),
            const SizedBox(height: 12),
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: Colors.red.shade50,
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: Colors.red.shade200),
              ),
              child: Row(
                children: [
                  Icon(
                    Icons.info_outline,
                    color: Colors.red.shade700,
                    size: 20,
                  ),
                  const SizedBox(width: 8),
                  const Expanded(
                    child: Text(
                      'This action cannot be undone. The item will be permanently removed.',
                      style: TextStyle(
                        color: Colors.red,
                        fontSize: 13,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            style: FilledButton.styleFrom(
              backgroundColor: AppTheme.errorColor,
              foregroundColor: Colors.white,
            ),
            child: const Text('Delete Permanently'),
          ),
        ],
      ),
    );
    if (confirmed == true) {
      await ref
          .read(unifiedObjectProvider.notifier)
          .permanentlyDeleteObject(object.id);
      if (mounted) {
        
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Permanently deleted "${object.name}"')),
        );
      }
    }
  }
}

class _TrashItemCard extends StatefulWidget {
  final DeletedItemInfo item;
  final bool hasHistory;
  final SensitivityLevel sensitivityLevel;
  final Future<void> Function(DeletedItemInfo item) onRestore;
  final Future<void> Function(DeletedItemInfo item) onPurge;
  final VoidCallback onDetail;
  final VoidCallback? onHistory;

  const _TrashItemCard({
    required this.item,
    required this.hasHistory,
    required this.sensitivityLevel,
    required this.onRestore,
    required this.onPurge,
    required this.onDetail,
    this.onHistory,
  });

  @override
  State<_TrashItemCard> createState() => _TrashItemCardState();
}

class _TrashItemCardState extends State<_TrashItemCard> {
  bool _isRestoring = false;
  bool _isPurging = false;

  bool get _isProcessing => _isRestoring || _isPurging;

  Future<void> _handleRestore() async {
    if (_isProcessing) return;

    setState(() {
      _isRestoring = true;
    });

    try {
      await widget.onRestore(widget.item);
    } on Exception {
      if (mounted) {
        showOverlaySnackBar(
            context,
            content: 'Failed to restore ${widget.item.itemLabel}',
            type: SnackBarType.warning,
          );
      }
    } finally {
      // Reset flag after operation completes (success or failure)
      if (mounted) {
        setState(() {
          _isRestoring = false;
        });
      }
    }
  }

  Future<void> _handlePurge() async {
    if (_isProcessing) return;

    setState(() {
      _isPurging = true;
    });

    try {
      // Await the entire purge flow: dialog confirmation + actual deletion
      await widget.onPurge(widget.item);
    } on Exception {
      if (mounted) {
        showOverlaySnackBar(
            context,
            content: 'Failed to purge ${widget.item.itemLabel}',
            type: SnackBarType.warning,
          );
      }
    } finally {
      // Reset flag after operation completes (success or failure)
      if (mounted) {
        setState(() {
          _isPurging = false;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final daysRemaining =
        30 - DateTime.now().difference(widget.item.deletedAt).inDays;
    final isExpiringSoon = daysRemaining <= 7;

    return Card(
      child: InkWell(
        onTap: _isProcessing ? null : widget.onDetail,
        borderRadius: BorderRadius.circular(12),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Container(
                    width: 40,
                    height: 40,
                    decoration: BoxDecoration(
                      color: Colors.orange.withValues(alpha: 0.1),
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Icon(
                      widget.item.meta?.icon ?? Icons.help_outline,
                      color: Colors.orange,
                      size: 20,
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          widget.item.itemLabel,
                          style: theme.textTheme.titleSmall?.copyWith(
                            fontWeight: FontWeight.w600,
                          ),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                        const SizedBox(height: 2),
                        Text(
                          '${widget.item.meta?.label ?? widget.item.itemType} - ${widget.item.meta?.sectionLabel ?? widget.item.section}',
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                        const SizedBox(width: 8),
                        SensitivityTag(level: widget.sensitivityLevel),
                      ],
                    ),
                  ),
                  if (isExpiringSoon)
                    Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 8,
                        vertical: 4,
                      ),
                      decoration: BoxDecoration(
                        color: Colors.orange.shade100,
                        borderRadius: BorderRadius.circular(12),
                      ),
                      child: Text(
                        '$daysRemaining days',
                        style: theme.textTheme.labelSmall?.copyWith(
                          color: Colors.orange.shade800,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                ],
              ),
              const SizedBox(height: 12),
              Row(
                children: [
                  Icon(
                    Icons.access_time,
                    size: 14,
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                  const SizedBox(width: 4),
                  Text(
                    'Deleted ${_formatTimeAgo(widget.item.deletedAt)}',
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                  const Spacer(),
                  if (widget.hasHistory && widget.onHistory != null)
                    TextButton.icon(
                      onPressed: widget.onHistory,
                      icon: const Icon(Icons.history, size: 16),
                      label: const Text('History'),
                      style: TextButton.styleFrom(
                        padding: const EdgeInsets.symmetric(horizontal: 8),
                        minimumSize: Size.zero,
                        foregroundColor: AppTheme.primaryColor,
                      ),
                    ),
                  if (widget.hasHistory && widget.onHistory != null)
                    const SizedBox(width: 4),
                  TextButton.icon(
                    onPressed: _isRestoring ? null : _handleRestore,
                    icon: _isRestoring
                        ? const SizedBox(
                            width: 16,
                            height: 16,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : const Icon(Icons.restore, size: 16),
                    label: Text(_isRestoring ? 'Restoring...' : 'Restore'),
                    style: TextButton.styleFrom(
                      padding: const EdgeInsets.symmetric(horizontal: 8),
                      minimumSize: Size.zero,
                    ),
                  ),
                  const SizedBox(width: 4),
                  TextButton.icon(
                    onPressed: _isPurging ? null : _handlePurge,
                    icon: _isPurging
                        ? const SizedBox(
                            width: 16,
                            height: 16,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : const Icon(Icons.delete_forever, size: 16),
                    label: Text(_isPurging ? 'Purging...' : 'Purge'),
                    style: TextButton.styleFrom(
                      padding: const EdgeInsets.symmetric(horizontal: 8),
                      minimumSize: Size.zero,
                      foregroundColor: AppTheme.errorColor,
                    ),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    ).animate().fadeIn(duration: 300.ms);
  }

  String _formatTimeAgo(DateTime date) {
    final diff = DateTime.now().difference(date);
    if (diff.inDays > 0) {
      return '${diff.inDays}d ago';
    } else if (diff.inHours > 0) {
      return '${diff.inHours}h ago';
    } else if (diff.inMinutes > 0) {
      return '${diff.inMinutes}m ago';
    } else {
      return 'Just now';
    }
  }
}

// =============================================================================
// Unified Object Trash Card
// =============================================================================

class _UnifiedObjectTrashCard extends StatelessWidget {
  final UnifiedObject object;
  final VoidCallback onRestore;
  final VoidCallback onPurge;

  const _UnifiedObjectTrashCard({
    required this.object,
    required this.onRestore,
    required this.onPurge,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final deletedAt = object.deletedAt;
    final daysRemaining = deletedAt != null
        ? 30 - DateTime.now().difference(deletedAt).inDays
        : 30;
    final isExpiringSoon = daysRemaining <= 7;

    return Card(
      child: InkWell(
        borderRadius: BorderRadius.circular(12),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Container(
                    width: 40,
                    height: 40,
                    decoration: BoxDecoration(
                      color: Colors.orange.withValues(alpha: 0.1),
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Icon(
                      UnifiedObjectService.getIconFromName(object.iconName),
                      color: Colors.orange,
                      size: 20,
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          object.name,
                          style: theme.textTheme.titleSmall?.copyWith(
                            fontWeight: FontWeight.w600,
                          ),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                        const SizedBox(height: 2),
                        Text(
                          object.typeId ?? 'object',
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                      ],
                    ),
                  ),
                  if (isExpiringSoon)
                    Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 8,
                        vertical: 4,
                      ),
                      decoration: BoxDecoration(
                        color: Colors.orange.shade100,
                        borderRadius: BorderRadius.circular(12),
                      ),
                      child: Text(
                        '$daysRemaining days',
                        style: theme.textTheme.labelSmall?.copyWith(
                          color: Colors.orange.shade800,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                ],
              ),
              const SizedBox(height: 12),
              Row(
                children: [
                  Icon(
                    Icons.access_time,
                    size: 14,
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                  const SizedBox(width: 4),
                  Text(
                    deletedAt != null
                        ? 'Deleted ${_formatTimeAgo(deletedAt)}'
                        : 'Deleted recently',
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                  const Spacer(),
                  TextButton.icon(
                    onPressed: onRestore,
                    icon: const Icon(Icons.restore, size: 16),
                    label: const Text('Restore'),
                    style: TextButton.styleFrom(
                      padding: const EdgeInsets.symmetric(horizontal: 8),
                      minimumSize: Size.zero,
                    ),
                  ),
                  const SizedBox(width: 4),
                  TextButton.icon(
                    onPressed: onPurge,
                    icon: const Icon(Icons.delete_forever, size: 16),
                    label: const Text('Purge'),
                    style: TextButton.styleFrom(
                      padding: const EdgeInsets.symmetric(horizontal: 8),
                      minimumSize: Size.zero,
                      foregroundColor: AppTheme.errorColor,
                    ),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    ).animate().fadeIn(duration: 300.ms);
  }

  String _formatTimeAgo(DateTime date) {
    final diff = DateTime.now().difference(date);
    if (diff.inDays > 0) {
      return '${diff.inDays}d ago';
    } else if (diff.inHours > 0) {
      return '${diff.inHours}h ago';
    } else if (diff.inMinutes > 0) {
      return '${diff.inMinutes}m ago';
    } else {
      return 'Just now';
    }
  }
}
