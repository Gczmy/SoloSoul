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
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/operation_logger.dart';
import 'package:solosoul_flutter/presentation/models/operation_log_models.dart'
    show LogSection, LogAction;
import 'package:solosoul_flutter/presentation/providers/operation_log_provider.dart'
    show OperationLogService;
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/trash/unified_object_trash_card.dart';

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
    final l10n = AppLocalizations.of(context);
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
          title: Text(l10n.trashTitle),
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
                'Password Required',
                style: Theme.of(context).textTheme.headlineSmall,
              ),
              const SizedBox(height: 16),
              FilledButton.icon(
                onPressed: _verifyPassword,
                icon: const Icon(Icons.lock_open),
                label: Text(l10n.trashVerify),
              ),
            ],
          ),
        ),
      );
    }
    final theme = Theme.of(context);
    final deletedUnifiedObjects = ref.watch(deletedObjectsProvider);
    return _TrashViewWidget(
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
            child: Text(AppLocalizations.of(context).commonCancel),
          ),
          FilledButton(
            onPressed: () async {
              final snackBarContext = context;
              Navigator.pop(snackBarContext);

              // Permanently delete all soft-deleted unified objects in one batch
              final deletedObjects = ref.read(deletedObjectsProvider);
              final notifier = ref.read(unifiedObjectProvider.notifier);

              // Log operations first (before objects are removed)
              for (final obj in deletedObjects) {
                final logSection = _logSectionForTypeId(obj.typeId ?? '');
                if (logSection != null) {
                  final properties = <String, String>{
                    for (final entry in obj.properties.entries)
                      entry.key: propValueToString(entry.value),
                  };
                  final propertyLevels = <String, String>{
                    for (final entry in obj.properties.entries)
                      entry.key: entry.value.sensitivity.name,
                  };
                  final entry = OperationLogger.logCustomSection(
                    section: logSection.value,
                    action: LogAction.purge,
                    description: '${AppLocalizations.of(context).trashPermanentlyDeleted}${obj.name}',
                    properties: properties,
                    propertyLevels: propertyLevels,
                  );
                  await OperationLogService.instance.addEntry(entry);
                }
              }

              // Batch delete + single save
              final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
              await notifier.permanentlyDeleteMultiple(
                deletedObjects.map((o) => o.id).toList(),
                accountId: accountId,
              );

              if (mounted) {
                showOverlaySnackBar(
                  snackBarContext,
                  content: 'All $itemCount items permanently deleted',
                  type: SnackBarType.error,
                );
              }
            },
            style: FilledButton.styleFrom(backgroundColor: AppTheme.errorColor),
            child: Text(AppLocalizations.of(context).trashEmptyTrash),
          ),
        ],
      ),
    );
  }

  Future<void> _confirmRestoreUnifiedObject(UnifiedObject object) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Row(
          children: [
            const Icon(Icons.restore, color: AppTheme.primaryColor),
            const SizedBox(width: 8),
            Text(AppLocalizations.of(context).trashConfirmRestore),
          ],
        ),
        content: Text(
          'Are you sure you want to restore "${object.name}"?',
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
          description: '${AppLocalizations.of(context).trashRestored}${object.name}',
        );
        await OperationLogService.instance.addEntry(entry);
      }
      if (mounted) {
        showOverlaySnackBar(
          context,
          content: 'Restored "${object.name}"',
          type: SnackBarType.success,
        );
      }
    }
  }

  Future<void> _confirmPurgeUnifiedObject(UnifiedObject object) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Row(
          children: [
            Icon(Icons.warning_amber, color: Colors.orange.shade700),
            const SizedBox(width: 8),
            Text(AppLocalizations.of(context).trashConfirmPermanentDelete),
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
          description: '${AppLocalizations.of(context).trashPermanentlyDeleted}${object.name}',
          properties: properties,
          propertyLevels: propertyLevels,
        );
        await OperationLogService.instance.addEntry(entry);
      }
      if (mounted) {
        showOverlaySnackBar(
          context,
          content: 'Permanently deleted "${object.name}"',
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
            searchQuery.isEmpty ? 'Trash is empty' : 'No matching items',
            style: theme.textTheme.titleMedium?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            searchQuery.isEmpty
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
                label: const Text(
                  'Empty Trash',
                  style: TextStyle(color: AppTheme.errorColor),
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
                entries.add(const TrashSectionHeader('Pages & Objects'));
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

class _TrashViewWidget extends StatelessWidget {
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
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: SoloGlassAppBar(
        backRoute: AppRoutes.home,
        title: Text(AppLocalizations.of(context).trashTitle),
        actions: const [HeaderActionButtons()],
      ),
      body: Column(
        children: [
          // Search bar
          Padding(
            padding: const EdgeInsets.all(16),
            child: TextField(
              controller: searchController,
              onChanged: onSearchChanged,
              decoration: InputDecoration(
                hintText: AppLocalizations.of(context).trashSearchHint,
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
          Expanded(child: trashContent),
        ],
      ),
    );
  }
}

class _TrashContentWidget extends StatelessWidget {
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
  Widget build(BuildContext context) {
    // Filter unified objects based on search query (by name and typeId only)
    final filteredUnifiedObjects = searchQuery.isEmpty
        ? deletedUnifiedObjects
        : deletedUnifiedObjects.where((obj) {
            final lowerQuery = searchQuery.toLowerCase();
            return obj.name.toLowerCase().contains(lowerQuery) ||
                (obj.typeId?.toLowerCase().contains(lowerQuery) ?? false);
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
                  '${deletedUnifiedObjects.length} total items in trash',
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
