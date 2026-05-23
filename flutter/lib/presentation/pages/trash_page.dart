import 'dart:async';
import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
// ignore_for_file: use_build_context_synchronously
import 'package:solosoul_flutter/presentation/utils/property_value_utils.dart';
import 'package:solosoul_flutter/presentation/utils/log_section_utils.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    show AppTheme, showOverlaySnackBar, SnackBarType;
import 'package:solosoul_flutter/core/router/app_router.dart';
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';
import 'package:solosoul_flutter/presentation/widgets/section_renderer_registry.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/operation_logger.dart';
import 'package:solosoul_flutter/presentation/models/operation_log_models.dart'
    show LogSection, LogAction;
import 'package:solosoul_flutter/presentation/providers/operation_log_provider.dart'
    show OperationLogService;
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/trash/unified_object_trash_card.dart';
import 'package:solosoul_flutter/presentation/widgets/trash/trash_filter_section.dart';
import 'package:solosoul_flutter/presentation/providers/trash_filter_provider.dart';

// =============================================================================
// Unified abstraction for trash list items (enables ListView.builder)
// =============================================================================

@immutable
sealed class TrashEntry {
  const TrashEntry();
}

final class TrashSectionHeader extends TrashEntry {
  final String title;
  const TrashSectionHeader(this.title);
}

final class TrashUnifiedEntry extends TrashEntry {
  final UnifiedObject object;
  const TrashUnifiedEntry(this.object);
}

class TrashPage extends ConsumerStatefulWidget {
  const TrashPage({super.key});

  @override
  ConsumerState<TrashPage> createState() => _TrashPageState();
}

class _TrashPageState extends ConsumerState<TrashPage> {
  final _searchController = TextEditingController();
  String _searchQuery = '';

  /// Whether we have already shown the auto-prompt dialog on first build.
  bool _hasPromptedForVerification = false;

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
      message: AppLocalizations.of(context).trashVerifyPassword,
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
        appBar: SoloGlassAppBar(
          backRoute: AppRoutes.home,
          title: Text(AppLocalizations.of(context).trashTitle),
        ),
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
                AppLocalizations.of(context).trashPasswordRequired,
                style: Theme.of(context).textTheme.headlineSmall,
              ),
              const SizedBox(height: 16),
              FilledButton.icon(
                onPressed: _verifyPassword,
                icon: const Icon(Icons.lock_open),
                label: Text(AppLocalizations.of(context).trashVerify),
              ),
            ],
          ),
        ),
      );
    }
    final theme = Theme.of(context);
    final deletedUnifiedObjects = ref.watch(trashRootDeletedObjectsProvider);
    // Extend sensitive access timeout on user activity to avoid interrupting active browsing
    return Listener(
      onPointerDown: (_) => ref.read(sensitivePageAccessProvider.notifier).markVerified(),
      onPointerMove: (_) => ref.read(sensitivePageAccessProvider.notifier).markVerified(),
      child: _TrashViewWidget(
        searchController: _searchController,
        searchQuery: _searchQuery,
        onSearchChanged: (value) => setState(() => _searchQuery = value),
        onClearSearch: () {
          _searchController.clear();
          setState(() => _searchQuery = '');
        },
        trashContent: _TrashContentWidget(
          theme: theme,
          searchQuery: _searchQuery,
          deletedUnifiedObjects: deletedUnifiedObjects,
          onEmptyTrash: (count) => _confirmEmptyTrash(context, count),
          onRestore: _confirmRestoreUnifiedObject,
          onPurge: _confirmPurgeUnifiedObject,
        ),
      ),
    );
  }

  void _confirmEmptyTrash(BuildContext context, int itemCount) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: Row(
          children: [
            Icon(Icons.warning_amber, color: Colors.orange.shade700),
            const SizedBox(width: 8),
            Text(AppLocalizations.of(context).trashEmptyTrash),
          ],
        ),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              AppLocalizations.of(context).trashEmptyConfirm(itemCount),
              style: Theme.of(context).textTheme.bodyMedium,
            ),
            const SizedBox(height: 12),
            _DangerWarningBox(message: AppLocalizations.of(context).trashEmptyWarning),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: Text(AppLocalizations.of(context).commonCancel),
          ),
          FilledButton(
            onPressed: () => _performEmptyTrash(context, itemCount),
            style: FilledButton.styleFrom(backgroundColor: AppTheme.errorColor),
            child: Text(AppLocalizations.of(context).trashEmptyTrash),
          ),
        ],
      ),
    );
  }

  Future<void> _performEmptyTrash(BuildContext dialogContext, int itemCount) async {
    Navigator.pop(dialogContext);
    final l10n = AppLocalizations.of(context);

    final deletedObjects = ref.read(deletedObjectsProvider);
    final notifier = ref.read(unifiedObjectProvider.notifier);

    // Log operations first (before objects are removed)
    for (final obj in deletedObjects) {
      final logSection = _logSectionForTypeId(obj.typeId ?? '');
      if (logSection != null) {
        final properties = <String, String>{};
        final propertyLevels = <String, String>{};
        for (final entry in obj.properties.entries) {
          properties[entry.key] = propValueToString(entry.value);
          propertyLevels[entry.key] = entry.value.sensitivity.name;
        }
        final opEntry = OperationLogger.logCustomSection(
          section: logSection.value,
          action: LogAction.purge,
          description: l10n.trashPermanentDeletedItem(obj.name),
          properties: properties,
          propertyLevels: propertyLevels,
          descriptionKey: 'purgedUnifiedItem',
          descriptionArgs: {'name': obj.name},
        );
        await OperationLogService.instance.addEntry(opEntry);
      }
    }

    if (deletedObjects.isNotEmpty) {
      unawaited(ref.read(authNotifierProvider.notifier).updateOperation('Purged items'));
    }

    // Batch delete + single save
    final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
    await notifier.permanentlyDeleteMultiple(
      deletedObjects.map((o) => o.id).toList(),
      accountId: accountId,
    );

    if (mounted) {
      showOverlaySnackBar(
        context,
        content: l10n.trashEmptyComplete(itemCount),
        type: SnackBarType.error,
      );
    }
  }

  Future<void> _confirmRestoreUnifiedObject(UnifiedObject object) async {
    final l10nDlg = AppLocalizations.of(context);
    final displayName = getLocalizedObjectName(l10nDlg, object);
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Row(
          children: [
            const Icon(Icons.restore, color: AppTheme.primaryColor),
            const SizedBox(width: 8),
            Text(l10nDlg.trashConfirmRestore),
          ],
        ),
        content: Text(
          l10nDlg.trashRestoreConfirmBody(displayName),
          style: Theme.of(ctx).textTheme.bodyMedium,
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: Text(AppLocalizations.of(context).commonCancel),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: Text(AppLocalizations.of(context).commonConfirm),
          ),
        ],
      ),
    );
    if (confirmed == true) {
      await ref.read(unifiedObjectProvider.notifier).restoreObject(object.id);
      final logSection = _logSectionForTypeId(object.typeId ?? '');
      if (logSection != null) {
        final entry = OperationLogger.logCustomSection(
          section: logSection.value,
          action: LogAction.restore,
          description: l10nDlg.trashRestoredItem(displayName),
          descriptionKey: 'restoredTrashItem',
          descriptionArgs: {'name': displayName},
        );
        await OperationLogService.instance.addEntry(entry);
        unawaited(ref.read(authNotifierProvider.notifier).updateOperation('Restored item'));
      }
      if (mounted) {
        showOverlaySnackBar(
          context,
          content: l10nDlg.trashRestoredItem(displayName),
          type: SnackBarType.success,
        );
      }
    }
  }

  Future<void> _confirmPurgeUnifiedObject(UnifiedObject object) async {
    final l10nDlg = AppLocalizations.of(context);
    final displayName = getLocalizedObjectName(l10nDlg, object);
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Row(
          children: [
            Icon(Icons.warning_amber, color: Colors.orange.shade700),
            const SizedBox(width: 8),
            Text(l10nDlg.trashConfirmPermanentDelete),
          ],
        ),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              l10nDlg.trashPermanentDeleteConfirm(displayName),
              style: Theme.of(ctx).textTheme.bodyMedium,
            ),
            const SizedBox(height: 12),
            _DangerWarningBox(message: AppLocalizations.of(context).trashPermanentDeleteWarning),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: Text(AppLocalizations.of(context).commonCancel),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            style: FilledButton.styleFrom(
              backgroundColor: AppTheme.errorColor,
              foregroundColor: Colors.white,
            ),
            child: Text(AppLocalizations.of(context).commonDelete),
          ),
        ],
      ),
    );
    if (confirmed == true) {
      final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
      await ref
          .read(unifiedObjectProvider.notifier)
          .permanentlyDeleteObject(object.id, accountId: accountId);
      final logSection = _logSectionForTypeId(object.typeId ?? '');
      if (logSection != null) {
        final properties = <String, String>{
          for (final entry in object.properties.entries)
            entry.key: propValueToString(entry.value),
        };
        final propertyLevels = <String, String>{
          for (final entry in object.properties.entries)
            entry.key: entry.value.sensitivity.name,
        };
        final entry = OperationLogger.logCustomSection(
          section: logSection.value,
          action: LogAction.purge,
          description: l10nDlg.trashPermanentDeletedItem(displayName),
          properties: properties,
          propertyLevels: propertyLevels,
          descriptionKey: 'purgedUnifiedItem',
          descriptionArgs: {'name': displayName},
        );
        await OperationLogService.instance.addEntry(entry);
        unawaited(ref.read(authNotifierProvider.notifier).updateOperation('Purged item'));
      }
      if (mounted) {
        showOverlaySnackBar(
          context,
          content: l10nDlg.trashPermanentDeletedItem(displayName),
          type: SnackBarType.error,
        );
      }
    }
  }

  /// Map typeId to LogSection for operation logging.
  LogSection? _logSectionForTypeId(String typeId) => logSectionForTypeId(typeId);
}

class _TrashEmptyState extends StatelessWidget {
  final String searchQuery;
  final ThemeData theme;

  const _TrashEmptyState({required this.searchQuery, required this.theme});

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(
            searchQuery.isEmpty ? Icons.delete_outline : Icons.search_off,
            size: 64,
            color: theme.colorScheme.onSurfaceVariant,
          ),
          const SizedBox(height: 16),
          Text(
            searchQuery.isEmpty ? AppLocalizations.of(context).trashEmpty : AppLocalizations.of(context).trashNoMatching,
            style: theme.textTheme.titleMedium?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            searchQuery.isEmpty
                ? AppLocalizations.of(context).trashDeletedAppear
                : AppLocalizations.of(context).trashAdjustSearch,
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
        ],
      ),
    );
  }
}

class _TrashListView extends StatelessWidget {
  final ThemeData theme;
  final List<UnifiedObject> objects;
  final VoidCallback onEmptyTrash;
  final ValueChanged<UnifiedObject> onRestore;
  final ValueChanged<UnifiedObject> onPurge;

  const _TrashListView({
    required this.theme,
    required this.objects,
    required this.onEmptyTrash,
    required this.onRestore,
    required this.onPurge,
  });

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.end,
            children: [
              TextButton.icon(
                onPressed: onEmptyTrash,
                icon: const Icon(
                  Icons.delete_forever,
                  color: AppTheme.errorColor,
                ),
                label: Text(
                  AppLocalizations.of(context).trashEmptyTrashButton,
                  style: const TextStyle(color: AppTheme.errorColor),
                ),
              ),
            ],
          ),
        ),
        Expanded(
          child: Builder(
            builder: (context) {
              final entries = <TrashEntry>[];
              if (objects.isNotEmpty) {
                entries.add(TrashSectionHeader(AppLocalizations.of(context).trashSectionTitle));
                entries.addAll(
                  objects.map((o) => TrashUnifiedEntry(o)),
                );
              }

              return ListView.builder(
                padding: const EdgeInsets.all(16),
                itemCount: entries.length,
                itemBuilder: (context, index) {
                  final entry = entries[index];
                  return switch (entry) {
                    TrashSectionHeader(:final title) => Padding(
                        padding: const EdgeInsets.only(bottom: 8),
                        child: Text(
                          title,
                          style: theme.textTheme.titleSmall?.copyWith(
                            fontWeight: FontWeight.w600,
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                      ),
                    TrashUnifiedEntry(:final object) => Padding(
                        padding: const EdgeInsets.only(bottom: 8),
                        child: UnifiedObjectTrashCard(
                          object: object,
                          onRestore: () => onRestore(object),
                          onPurge: () => onPurge(object),
                        ),
                      ),
                  };
                },
              );
            },
          ),
        ),
      ],
    );
  }
}

class _TrashViewWidget extends ConsumerWidget {
  final TextEditingController searchController;
  final String searchQuery;
  final ValueChanged<String> onSearchChanged;
  final VoidCallback onClearSearch;
  final Widget trashContent;

  const _TrashViewWidget({
    required this.searchController,
    required this.searchQuery,
    required this.onSearchChanged,
    required this.onClearSearch,
    required this.trashContent,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context);
    final deletedUnifiedObjects = ref.watch(trashRootDeletedObjectsProvider);
    final timeFilter = ref.watch(trashTimeFilterProvider);
    final typeFilters = ref.watch(trashTypeFilterProvider);

    // Compute filtered count for the badge
    final filteredCount = deletedUnifiedObjects.where((obj) {
      if (timeFilter != null && timeFilter != 'all') {
        if (obj.deletedAt == null) return false;
        final cutoff = getTimeFilterCutoff(timeFilter);
        if (cutoff != null && !obj.deletedAt!.isAfter(cutoff)) return false;
      }
      if (typeFilters.isNotEmpty) {
        bool matches = false;
        for (final filter in typeFilters) {
          if (filter == 'item') {
            if (isItemType(obj.typeId)) { matches = true; break; }
          } else if (filter == obj.typeId) {
            matches = true;
            break;
          }
        }
        if (!matches) return false;
      }
      return true;
    }).length;

    return Scaffold(
      appBar: SoloGlassAppBar(
        backRoute: AppRoutes.home,
        title: Text(l10n.trashTitle),
        actions: const [HeaderActionButtons()],
      ),
      body: Column(
        children: [
          // Trash info banner - always shown, above search
          Container(
            margin: const EdgeInsets.symmetric(horizontal: 16),
            padding: const EdgeInsets.all(8),
            decoration: BoxDecoration(
              color: Colors.orange.withValues(alpha: 0.1),
              borderRadius: BorderRadius.circular(12),
              border: Border.all(color: Colors.orange.withValues(alpha: 0.3)),
            ),
            child: Row(
              children: [
                Icon(Icons.warning_amber, color: Colors.orange.shade700, size: 16),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    l10n.trashAutoPurgeNotice,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: Colors.orange.shade700,
                    ),
                  ),
                ),
              ],
            ),
          ).animate().fadeIn(duration: 400.ms),

          // Search bar
          Padding(
            padding: const EdgeInsets.all(16),
            child: TextField(
              controller: searchController,
              onChanged: onSearchChanged,
              decoration: InputDecoration(
                hintText: l10n.trashSearchHint,
                prefixIcon: const Icon(Icons.search),
                suffixIcon: searchQuery.isNotEmpty
                    ? IconButton(
                        icon: const Icon(Icons.clear),
                        onPressed: onClearSearch,
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

          // Filter section (always visible, GenericFilterSection handles collapse)
          TrashFilterSection(
            resultCount: filteredCount,
            onClearAll: () {
              ref.read(trashTimeFilterProvider.notifier).clear();
              ref.read(trashTypeFilterProvider.notifier).clear();
            },
          ),

          // Main content area
          Expanded(child: trashContent),
        ],
      ),
    );
  }
}

class _TrashContentWidget extends ConsumerWidget {
  final ThemeData theme;
  final String searchQuery;
  final List<UnifiedObject> deletedUnifiedObjects;
  final void Function(int count) onEmptyTrash;
  final ValueChanged<UnifiedObject> onRestore;
  final ValueChanged<UnifiedObject> onPurge;

  const _TrashContentWidget({
    required this.theme,
    required this.searchQuery,
    required this.deletedUnifiedObjects,
    required this.onEmptyTrash,
    required this.onRestore,
    required this.onPurge,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    // Watch filter state
    final timeFilter = ref.watch(trashTimeFilterProvider);
    final typeFilters = ref.watch(trashTypeFilterProvider);

    // Filter unified objects based on search query, time filter, and type filter
    final filteredUnifiedObjects = deletedUnifiedObjects.where((obj) {
      // Search filter
      if (searchQuery.isNotEmpty) {
        final lowerQuery = searchQuery.toLowerCase();
        if (!obj.name.toLowerCase().contains(lowerQuery) &&
            !(obj.typeId?.toLowerCase().contains(lowerQuery) ?? false)) {
          return false;
        }
      }

      // Time filter
      if (timeFilter != null && timeFilter != 'all') {
        if (obj.deletedAt == null) return false;
        final cutoff = getTimeFilterCutoff(timeFilter);
        if (cutoff != null && !obj.deletedAt!.isAfter(cutoff)) {
          return false;
        }
      }

      // Type filter
      if (typeFilters.isNotEmpty) {
        bool matches = false;
        for (final filter in typeFilters) {
          if (filter == 'item') {
            if (isItemType(obj.typeId)) {
              matches = true;
              break;
            }
          } else if (filter == obj.typeId) {
            matches = true;
            break;
          }
        }
        if (!matches) return false;
      }

      return true;
    }).toList();

    final totalCount = filteredUnifiedObjects.length;

    // Results count when searching
    if (searchQuery.isNotEmpty) {
      return Column(
        children: [
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: Row(
              children: [
                Text(
                  totalCount > 0
                      ? AppLocalizations.of(context).trashFoundResults(totalCount)
                      : AppLocalizations.of(context).trashNoResults,
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: totalCount > 0
                            ? Theme.of(context).colorScheme.onSurfaceVariant
                            : Colors.orange,
                      ),
                ),
                const Spacer(),
                Text(
                  AppLocalizations.of(context).trashTotalItems(deletedUnifiedObjects.length),
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: Theme.of(context).colorScheme.onSurfaceVariant,
                      ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 8),
          Expanded(
            child: totalCount == 0
                ? _TrashEmptyState(searchQuery: searchQuery, theme: theme)
                : _TrashListView(
                    theme: theme,
                    objects: filteredUnifiedObjects,
                    onEmptyTrash: () => onEmptyTrash(filteredUnifiedObjects.length),
                    onRestore: onRestore,
                    onPurge: onPurge,
                  ),
          ),
        ],
      );
    }

    // Empty state
    if (totalCount == 0) {
      return _TrashEmptyState(searchQuery: searchQuery, theme: theme);
    }

    // Items list with empty trash action
    return _TrashListView(
      theme: theme,
      objects: filteredUnifiedObjects,
      onEmptyTrash: () => onEmptyTrash(filteredUnifiedObjects.length),
      onRestore: onRestore,
      onPurge: onPurge,
    );
  }
}

class _DangerWarningBox extends StatelessWidget {
  final String message;
  const _DangerWarningBox({required this.message});

  @override
  Widget build(BuildContext context) {
    return Container(
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
          Expanded(
            child: Text(
              message,
              style: const TextStyle(
                color: Colors.red,
                fontSize: 13,
              ),
            ),
          ),
        ],
      ),
    );
  }
}
