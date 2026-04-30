import 'package:flutter/material.dart';
// ignore_for_file: use_build_context_synchronously
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    show AppTheme, showOverlaySnackBar, SnackBarType;
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/core/services/operation_logger.dart';
import 'package:solosoul_flutter/presentation/models/operation_log_models.dart'
    show LogSection, LogAction;
import 'package:solosoul_flutter/presentation/providers/operation_log_provider.dart'
    show OperationLogService;
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart'
    show fieldHistoriesProvider;
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/field_history_dialog.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart';
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart'
    show effectiveSensitivityProvider;
import 'package:solosoul_flutter/presentation/widgets/unified_form_section.dart'
    show FormFieldDef;

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
          Expanded(child: _buildTrashContent(theme)),
        ],
      ),
    );
  }

  Widget _buildTrashContent(ThemeData theme) {
    final deletedUnifiedObjects = ref.watch(deletedObjectsProvider);

    // Filter unified objects based on search query (by name and typeId only)
    final filteredUnifiedObjects = _searchQuery.isEmpty
        ? deletedUnifiedObjects
        : deletedUnifiedObjects.where((obj) {
            final lowerQuery = _searchQuery.toLowerCase();
            return obj.name.toLowerCase().contains(lowerQuery) ||
                (obj.typeId?.toLowerCase().contains(lowerQuery) ?? false);
          }).toList();

    final totalCount = filteredUnifiedObjects.length;

    // Results count when searching
    if (_searchQuery.isNotEmpty) {
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
                ? _buildEmptyState(theme)
                : _buildTrashList(theme, filteredUnifiedObjects),
          ),
        ],
      );
    }

    // Empty state
    if (totalCount == 0) {
      return _buildEmptyState(theme);
    }

    // Items list with empty trash action
    return _buildTrashList(theme, filteredUnifiedObjects);
  }

  Widget _buildEmptyState(ThemeData theme) {
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

  Widget _buildTrashList(ThemeData theme, List<UnifiedObject> objects) {
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.end,
            children: [
              TextButton.icon(
                onPressed: () => _confirmEmptyTrash(context, objects.length),
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
                        child: _UnifiedObjectTrashCard(
                          object: object,
                          onRestore: () =>
                              _confirmRestoreUnifiedObject(context, object),
                          onPurge: () =>
                              _confirmPurgeUnifiedObject(context, object),
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
              Navigator.pop(snackBarContext);

              // Permanently delete all soft-deleted unified objects
              final deletedObjects = ref.read(deletedObjectsProvider);
              final notifier = ref.read(unifiedObjectProvider.notifier);
              for (final obj in deletedObjects) {
                await notifier.permanentlyDeleteObject(obj.id);
                final logSection = _logSectionForTypeId(obj.typeId ?? '');
                if (logSection != null) {
                  final entry = OperationLogger.logCustomSection(
                    section: logSection.value,
                    action: LogAction.purge,
                    description: 'Permanently deleted ${obj.name}',
                  );
                  await OperationLogService.instance.addEntry(entry);
                }
              }

              if (mounted) {
                showOverlaySnackBar(
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

  Future<void> _confirmRestoreUnifiedObject(
    BuildContext context,
    UnifiedObject object,
  ) async {
    final messenger = ScaffoldMessenger.of(context);
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
      final logSection = _logSectionForTypeId(object.typeId ?? '');
      if (logSection != null) {
        final entry = OperationLogger.logCustomSection(
          section: logSection.value,
          action: LogAction.restore,
          description: 'Restored ${object.name}',
        );
        await OperationLogService.instance.addEntry(entry);
      }
      if (mounted) {
        messenger.showSnackBar(
          SnackBar(content: Text('Restored "${object.name}"')),
        );
      }
    }
  }

  Future<void> _confirmPurgeUnifiedObject(
    BuildContext context,
    UnifiedObject object,
  ) async {
    final messenger = ScaffoldMessenger.of(context);
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
      final logSection = _logSectionForTypeId(object.typeId ?? '');
      if (logSection != null) {
        final entry = OperationLogger.logCustomSection(
          section: logSection.value,
          action: LogAction.purge,
          description: 'Permanently deleted ${object.name}',
        );
        await OperationLogService.instance.addEntry(entry);
      }
      if (mounted) {
        messenger.showSnackBar(
          SnackBar(content: Text('Permanently deleted "${object.name}"')),
        );
      }
    }
  }

  /// Map typeId to LogSection for operation logging.
  LogSection? _logSectionForTypeId(String typeId) {
    return switch (typeId) {
      'profile_identity' => LogSection.identity,
      'profile_contact' => LogSection.contactInformation,
      'profile_id_card' => LogSection.idCard,
      'profile_address' => LogSection.address,
      'travel_passport' => LogSection.passport,
      'travel_visa' => LogSection.visa,
      'travel_history' => LogSection.travelHistory,
      'financial_bank_account' => LogSection.bankAccount,
      'financial_card' => LogSection.card,
      'financial_tax_id' => LogSection.financial,
      'professional_education' => LogSection.education,
      'professional_employment' => LogSection.employment,
      'professional_skill' => LogSection.skill,
      'professional_language' => LogSection.language,
      'professional_award' => LogSection.professional,
      _ => null,
    };
  }
}

// =============================================================================
// Unified Object Trash Card
// =============================================================================

class _UnifiedObjectTrashCard extends ConsumerWidget {
  final UnifiedObject object;
  final VoidCallback onRestore;
  final VoidCallback onPurge;

  const _UnifiedObjectTrashCard({
    required this.object,
    required this.onRestore,
    required this.onPurge,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final deletedAt = object.deletedAt;
    final daysRemaining = deletedAt != null
        ? 30 - DateTime.now().difference(deletedAt).inDays
        : 30;
    final isExpiringSoon = daysRemaining <= 7;

    // History lookup
    final fieldPrefix = _fieldPrefixForTypeId(object.typeId ?? '');
    final history = fieldPrefix.isNotEmpty
        ? ref.watch(fieldHistoriesProvider.select((h) => h.getHistory(object.id, fieldPrefix)))
        : null;

    // Field defs for history dialog
    final typeDef = ObjectTypeRegistry.getType(object.typeId ?? '');
    final fieldDefs = typeDef?.properties.map((prop) {
      return FormFieldDef(
        fieldId: prop.id,
        label: prop.name,
      );
    }).toList() ?? [];

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
              LayoutBuilder(
                builder: (context, constraints) {
                  final narrow = constraints.maxWidth < 420;
                  return Row(
                    children: [
                      Icon(
                        Icons.access_time,
                        size: 14,
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                      const SizedBox(width: 4),
                      Expanded(
                        child: Text(
                          deletedAt != null
                              ? 'Deleted ${_formatTimeAgo(deletedAt)}'
                              : 'Deleted recently',
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                          overflow: TextOverflow.ellipsis,
                          maxLines: 1,
                        ),
                      ),
                      _buildActionButton(
                        narrow: narrow,
                        icon: Icons.info_outline,
                        label: 'Details',
                        onPressed: () => _showDetailDialog(context, ref),
                      ),
                      const SizedBox(width: 4),
                      _buildHistoryButton(
                        context: context,
                        narrow: narrow,
                        count: history?.entries.length ?? 0,
                        onShowHistory: () => FieldHistoryDialog.show(
                          context: context,
                          title: object.name,
                          icon: UnifiedObjectService.getIconFromName(object.iconName),
                          fieldDefs: fieldDefs,
                          history: history,
                          fieldPrefix: fieldPrefix,
                        ),
                      ),
                      const SizedBox(width: 4),
                      _buildActionButton(
                        narrow: narrow,
                        icon: Icons.restore_from_trash,
                        label: 'Restore',
                        onPressed: onRestore,
                      ),
                      const SizedBox(width: 4),
                      _buildActionButton(
                        narrow: narrow,
                        icon: Icons.delete_forever,
                        label: 'Purge',
                        onPressed: onPurge,
                        color: AppTheme.errorColor,
                      ),
                    ],
                  );
                },
              ),
            ],
          ),
        ),
      ),
    ).animate().fadeIn(duration: 300.ms);
  }

  Widget _buildHistoryButton({
    required BuildContext context,
    required bool narrow,
    required int count,
    required VoidCallback onShowHistory,
  }) {
    final hasHist = count > 0;
    final iconColor = hasHist
        ? null
        : Theme.of(context).colorScheme.onSurfaceVariant.withValues(alpha: 0.4);
    final icon = Icon(Icons.history, size: narrow ? 18 : 16, color: iconColor);

    final stackIcon = Stack(
      clipBehavior: Clip.none,
      children: [
        icon,
        Positioned(
          right: -6,
          top: -6,
          child: Text(
            '$count',
            style: TextStyle(
              fontSize: 10,
              color: iconColor,
              fontWeight: FontWeight.w500,
              height: 1,
            ),
          ),
        ),
      ],
    );

    if (narrow) {
      return IconButton(
        icon: stackIcon,
        onPressed: hasHist
            ? onShowHistory
            : () => showOverlaySnackBar(
                  context,
                  content: 'No history available',
                  type: SnackBarType.info,
                ),
        padding: const EdgeInsets.all(2),
        constraints: const BoxConstraints(minWidth: 24, minHeight: 24),
        tooltip: hasHist ? 'History ($count)' : 'No history yet',
      );
    }

    return TextButton.icon(
      onPressed: hasHist
          ? onShowHistory
          : () => showOverlaySnackBar(
                context,
                content: 'No history available',
                type: SnackBarType.info,
              ),
      icon: stackIcon,
      label: const Text('History'),
      style: TextButton.styleFrom(
        padding: const EdgeInsets.symmetric(horizontal: 4),
        minimumSize: Size.zero,
        foregroundColor: iconColor,
      ),
    );
  }

  Widget _buildActionButton({
    required bool narrow,
    required IconData icon,
    required String label,
    required VoidCallback onPressed,
    Color? color,
  }) {
    if (narrow) {
      return IconButton(
        icon: Icon(icon, size: 18, color: color),
        onPressed: onPressed,
        padding: const EdgeInsets.all(2),
        constraints: const BoxConstraints(minWidth: 24, minHeight: 24),
        tooltip: label,
      );
    }
    return TextButton.icon(
      onPressed: onPressed,
      icon: Icon(icon, size: 16),
      label: Text(label),
      style: TextButton.styleFrom(
        padding: const EdgeInsets.symmetric(horizontal: 4),
        minimumSize: Size.zero,
        foregroundColor: color,
      ),
    );
  }

  void _showDetailDialog(BuildContext context, WidgetRef ref) {
    final fieldPrefix = _fieldPrefixForTypeId(object.typeId ?? '');
    final deletedAt = object.deletedAt;
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(object.name),
        content: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              // Deleted time info
              if (deletedAt != null) ...[
                Container(
                  width: double.infinity,
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: Theme.of(ctx).colorScheme.errorContainer.withValues(alpha: 0.3),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          Icon(
                            Icons.delete_outline,
                            size: 16,
                            color: Theme.of(ctx).colorScheme.error,
                          ),
                          const SizedBox(width: 8),
                          Text(
                            'Deleted ${_formatTimeAgo(deletedAt)}',
                            style: Theme.of(ctx).textTheme.bodyMedium?.copyWith(
                              color: Theme.of(ctx).colorScheme.error,
                              fontWeight: FontWeight.w600,
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 4),
                      Text(
                        _formatFullTimestamp(deletedAt),
                        style: Theme.of(ctx).textTheme.bodySmall?.copyWith(
                          color: Theme.of(ctx).colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 16),
              ],
              // Properties with sensitivity tags
              ...object.properties.entries.map((e) {
                final value = e.value;
                final text = switch (value) {
                  TextProperty() => value.text,
                  _ => '',
                };
                final fieldId = fieldPrefix.isNotEmpty ? '$fieldPrefix.${e.key}' : e.key;
                final sensitivity = ref.read(effectiveSensitivityProvider(fieldId));
                return Padding(
                  padding: const EdgeInsets.symmetric(vertical: 4),
                  child: Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              _formatLabel(e.key),
                              style: Theme.of(ctx).textTheme.labelSmall?.copyWith(
                                color: Theme.of(ctx).colorScheme.onSurfaceVariant,
                              ),
                            ),
                            const SizedBox(height: 2),
                            Text(
                              text.isEmpty ? '(empty)' : text,
                              style: Theme.of(ctx).textTheme.bodyMedium,
                            ),
                          ],
                        ),
                      ),
                      const SizedBox(width: 8),
                      SensitivityTag(level: sensitivity),
                    ],
                  ),
                );
              }),
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('Close'),
          ),
        ],
      ),
    );
  }

  String _formatLabel(String key) {
    final spaced = key.replaceAllMapped(
      RegExp(r'([a-z])([A-Z])'),
      (m) => '${m[1]} ${m[2]}',
    );
    return spaced.replaceAll('_', ' ').split(' ').map((word) {
      if (word.isEmpty) return word;
      return word[0].toUpperCase() + word.substring(1).toLowerCase();
    }).join(' ');
  }

  String _formatFullTimestamp(DateTime dt) {
    return '${dt.year}-${dt.month.toString().padLeft(2, '0')}-${dt.day.toString().padLeft(2, '0')} '
        '${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}';
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

/// Map typeId to field-prefix used by FieldRegistry.
String _fieldPrefixForTypeId(String typeId) {
  return switch (typeId) {
    'profile_identity' => 'identity',
    'profile_contact' => 'contact',
    'profile_id_card' => 'idCard',
    'profile_address' => 'address',
    'travel_passport' => 'passport',
    'travel_visa' => 'visa',
    'travel_history' => 'travel',
    'financial_bank_account' => 'bankAccount',
    'financial_card' => 'card',
    'financial_tax_id' => 'taxId',
    'professional_education' => 'education',
    'professional_employment' => 'employment',
    'professional_skill' => 'skill',
    'professional_language' => 'language',
    'professional_award' => 'award',
    _ => typeId,
  };
}
