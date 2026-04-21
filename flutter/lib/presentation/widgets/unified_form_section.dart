import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/section_card.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart'
    show SensitivityTag;
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart'
    show authNotifierProvider, sensitivePageAccessProvider;
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart'
    show fieldHistoriesProvider;
import 'package:solosoul_flutter/presentation/widgets/field_history_view.dart';

/// Configuration for recording field history on saves.
class HistoryRecordingConfig<T> {
  final String Function(T item) itemIdExtractor;
  final String fieldIdPrefix;

  const HistoryRecordingConfig({
    required this.itemIdExtractor,
    required this.fieldIdPrefix,
  });
}

/// A save callback that includes old values for history recording.
typedef HistoryAwareSave<T> = Future<void> Function(
  T? newItem,
  Map<String, String> values,
  T? editingItem, [
  Map<String, String>? oldValues,
]);

/// Field definition for a single form field
class FormFieldDef {
  final String fieldId;
  final String label;
  final String? hintText;
  final SensitivityLevel sensitivity;
  final String? initialValue;

  const FormFieldDef({
    required this.fieldId,
    required this.label,
    this.hintText,
    this.sensitivity = SensitivityLevel.public,
    this.initialValue,
  });
}

/// Unified form section widget that abstracts add/edit/delete/copy/expand-collapse logic
class UnifiedFormSection<T> extends ConsumerStatefulWidget {
  final String title;
  final IconData icon;
  final List<T> items;
  final List<FormFieldDef> fieldDefs;
  final Widget Function(T item) displayItemBuilder;
  final Future<void> Function(T item) onDelete;
  /// onSave receives (newItem, values, editingItem):
  /// - For adds: newItem is the item created by itemFactory (with correct ID), editingItem is null
  /// - For edits: newItem is null, editingItem is the original item, values has updated fields
  final Future<void> Function(T? newItem, Map<String, String> values, T? editingItem)
  onSave;
  final void Function(T item, String fieldId, String value)? onCopy;

  /// Optional callback for copying all fields at once
  /// Optional callback for copying all fields at once (async to allow password verification)
  final Future<void> Function(T item, String formattedText)? onCopyAll;
  final int maxVisibleItems;
  final T Function(Map<String, String> values, {String? id})? itemFactory;

  /// Converts a T item to a Map of fieldId -> value for populating edit form
  final Map<String, String> Function(T item)? itemToMap;

  /// Optional custom form builder. If provided, overrides the default
  /// TextField-based form. Parameters: context, theme, controllers, mode,
  /// onSubmit, onCancel.
  final Widget Function(
    BuildContext,
    ThemeData,
    Map<String, TextEditingController>,
    String /*mode*/,
    VoidCallback /*onSubmit*/,
    VoidCallback /*onCancel*/,
  )?
  customFormBuilder;

  /// Optional configuration for recording field history on saves.
  final HistoryRecordingConfig<T>? historyConfig;

  /// Optional history-aware save callback. If provided, it replaces onSave for
  /// history recording purposes. The callback receives oldValues as the 4th
  /// parameter for edit mode.
  final HistoryAwareSave<T>? historyAwareOnSave;

  /// If true, each item shows a History(N) button that expands FieldHistoryView inline.
  final bool showHistoryExpansion;

  /// The fieldId prefix used to look up history for items (e.g. 'contact', 'address').
  /// Required when showHistoryExpansion is true.
  final String? historyFieldIdPrefix;

  /// An extractor function to get the item ID for history lookups.
  /// If not provided, uses (item as dynamic).id as a fallback.
  final String Function(T item)? itemIdExtractor;

  const UnifiedFormSection({
    super.key,
    required this.title,
    required this.icon,
    required this.items,
    required this.fieldDefs,
    required this.displayItemBuilder,
    required this.onDelete,
    required this.onSave,
    this.onCopy,
    this.onCopyAll,
    this.maxVisibleItems = 3,
    this.itemFactory,
    this.itemToMap,
    this.customFormBuilder,
    this.historyConfig,
    this.historyAwareOnSave,
    this.showHistoryExpansion = false,
    this.historyFieldIdPrefix,
    this.itemIdExtractor,
  });

  @override
  ConsumerState<UnifiedFormSection<T>> createState() =>
      _UnifiedFormSectionState<T>();
}

class _UnifiedFormSectionState<T> extends ConsumerState<UnifiedFormSection<T>> {
  String _mode = 'idle';
  int _editingIndex = -1;
  late List<T> _items;

  /// Set of item indices whose history is currently expanded.
  final Set<int> _expandedHistoryIndices = {};

  /// Map of TextEditingControllers keyed by fieldId
  final Map<String, TextEditingController> _controllers = {};

  @override
  void initState() {
    super.initState();
    _items = List.from(widget.items);
    _initControllers();
  }

  void _initControllers() {
    for (final field in widget.fieldDefs) {
      _controllers[field.fieldId] = TextEditingController(
        text: field.initialValue ?? '',
      );
    }
  }

  @override
  void didUpdateWidget(UnifiedFormSection<T> oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.items != oldWidget.items) {
      setState(() {
        _items = List.from(widget.items);
      });
    }
    // Re-initialize controllers when field defs change
    if (widget.fieldDefs != oldWidget.fieldDefs) {
      _disposeControllers();
      _controllers.clear();
      _initControllers();
    }
  }

  void _disposeControllers() {
    for (final controller in _controllers.values) {
      controller.dispose();
    }
  }

  @override
  void dispose() {
    _disposeControllers();
    super.dispose();
  }

  void _startAdding() {
    _clearControllers();
    setState(() => _mode = 'adding');
  }

  void _startEditing(int index) {
    final item = _items[index];
    _populateControllersFromItem(item);
    setState(() {
      _mode = 'editing';
      _editingIndex = index;
    });
  }

  void _cancelEdit() {
    setState(() {
      _mode = 'idle';
      // Remove draft item if canceling from add mode
      if (_editingIndex == -1 && _items.isNotEmpty) {
        _items.removeAt(0);
      }
    });
  }

  void _clearControllers() {
    for (final controller in _controllers.values) {
      controller.clear();
    }
  }

  void _populateControllersFromItem(T item) {
    _clearControllers();
    if (widget.itemToMap != null) {
      final values = widget.itemToMap!(item);
      for (final field in widget.fieldDefs) {
        _controllers[field.fieldId]?.text = values[field.fieldId] ?? '';
      }
    }
  }

  Future<void> _deleteEntry(int index) async {
    final deleted = _items[index];
    final confirm = await showDeleteConfirmationDialog(
      context: context,
      itemName: _getItemName(deleted),
      itemType: widget.title,
    );
    if (confirm) {
      // Remove from local list immediately for responsive UI
      // The async onDelete will persist to storage
      setState(() {
        _items = List.from(_items)..removeAt(index);
      });
      await widget.onDelete(deleted);
    }
  }

  String _getItemName(T item) {
    // Try to extract a name from the item for display
    // This is a best-effort approach using common property names
    if (item is Map) {
      return (item as Map).values.firstOrNull?.toString() ?? widget.title;
    }
    // For data classes like VisaData, PassportData, etc.
    // Use reflection-like access through itemFactory
    return widget.title;
  }

  String _getItemId(T item) {
    if (widget.itemIdExtractor != null) {
      return widget.itemIdExtractor!(item);
    }
    // Fallback: try to get .id from the item
    return (item as dynamic).id as String? ?? '';
  }

  Future<void> _submitForm() async {
    final values = <String, String>{};
    for (final field in widget.fieldDefs) {
      values[field.fieldId] = _controllers[field.fieldId]?.text ?? '';
    }

    final wasAdding = _mode == 'adding';
    final editingItem = wasAdding ? null : _items[_editingIndex];

    // Capture old values for history recording (only in edit mode)
    Map<String, String>? oldValues;
    if (!wasAdding && editingItem != null && widget.historyConfig != null && widget.itemToMap != null) {
      oldValues = widget.itemToMap!(editingItem);
    }

    // Capture the newly created item (with correct ID) for "adding" mode
    T? createdItem;
    setState(() {
      if (wasAdding) {
        // Create new item via itemFactory
        if (widget.itemFactory != null) {
          createdItem = widget.itemFactory!(values);
          // Insert at position 0 instead of adding at end
          _items.insert(0, createdItem as T);
        }
      } else {
        // For editing, create updated item
        if (widget.itemFactory != null && editingItem != null) {
          _items[_editingIndex] = widget.itemFactory!(
            values,
            id: (editingItem as dynamic).id as String?,
          );
        }
      }
      _mode = 'idle';
    });

    // Always persist via onSave (handles notifications and storage)
    await widget.onSave(createdItem, values, editingItem);

    // Record history if configured (only for edits, adds have no oldValues)
    if (widget.historyAwareOnSave != null && widget.historyConfig != null && !wasAdding) {
      await widget.historyAwareOnSave!(
        createdItem,
        values,
        editingItem,
        oldValues,
      );
    }
  }

  Widget _buildForm(ThemeData theme, {bool autofocus = false}) {
    if (widget.customFormBuilder != null) {
      return widget.customFormBuilder!(
        context,
        theme,
        _controllers,
        _mode,
        _submitForm,
        _cancelEdit,
      );
    }

    final isAdding = _mode == 'adding';

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(
          isAdding ? 'Add ${widget.title}' : 'Edit ${widget.title}',
          style: theme.textTheme.titleSmall?.copyWith(
            fontWeight: FontWeight.w600,
          ),
        ),
        const SizedBox(height: 12),
        ...widget.fieldDefs.asMap().entries.map((e) {
          final fieldIndex = e.key;
          final field = e.value;
          return Padding(
            padding: const EdgeInsets.only(bottom: 12),
            child: TextField(
              controller: _controllers[field.fieldId],
              maxLength: kMaxFieldLength,
              autofocus: fieldIndex == 0 && autofocus,
              decoration: InputDecoration(
                labelText: field.label,
                hintText: field.hintText,
                counterText: '',
                border: const OutlineInputBorder(),
                suffixIcon: Padding(
                  padding: const EdgeInsets.only(right: 8),
                  child: SensitivityTag(level: field.sensitivity),
                ),
              ),
            ),
          );
        }),
        const SizedBox(height: 16),
        Row(
          mainAxisAlignment: MainAxisAlignment.end,
          children: [
            TextButton(
              onPressed: _cancelEdit,
              child: const Text('Cancel'),
            ),
            const SizedBox(width: 8),
            FilledButton(
              onPressed: _submitForm,
              child: Text(isAdding ? 'Add' : 'Save'),
            ),
          ],
        ),
      ],
    );
  }

  Widget _buildEmpty(ThemeData theme) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 24),
        child: Column(
          children: [
            Icon(
              widget.icon,
              size: 40,
              color: theme.colorScheme.onSurfaceVariant,
            ),
            const SizedBox(height: 8),
            Text(
              'No ${widget.title.toLowerCase()} saved',
              style: TextStyle(color: theme.colorScheme.onSurfaceVariant),
            ),
            const SizedBox(height: 12),
            TextButton.icon(
              onPressed: _startAdding,
              icon: const Icon(Icons.add),
              label: Text('Add ${widget.title}'),
            ),
          ],
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final isEditing = _mode == 'adding' || _mode == 'editing';
    final theme = Theme.of(context);

    // When adding mode, insert draft item at position 0 and show form inline
    final displayItems = <Widget>[];

    if (isEditing) {
      // Semi-transparent overlay over existing items
      displayItems.add(
        IgnorePointer(
          child: Container(
            color: Colors.black.withValues(alpha: 0.05),
          ),
        ),
      );
    }

    // Add form at top when editing (inline, not in footer)
    if (isEditing) {
      displayItems.add(
        Padding(
          padding: const EdgeInsets.only(bottom: 16),
          child: _buildForm(theme, autofocus: true),
        ),
      );
    }

    // Add existing items
    if (_items.isEmpty && !isEditing) {
      displayItems.add(_buildEmpty(theme));
    } else if (_items.isNotEmpty) {
      for (var i = 0; i < _items.length; i++) {
        final item = _items[i];
        final historyExpanded = widget.showHistoryExpansion && _expandedHistoryIndices.contains(i);
        final itemId = _getItemId(item);
        displayItems.add(
          _ItemWithHistory(
            key: ValueKey('item_$i'),
            item: item,
            index: i,
            historyExpanded: historyExpanded,
            onToggleHistory: widget.showHistoryExpansion
                ? () => setState(() {
                      if (_expandedHistoryIndices.contains(i)) {
                        _expandedHistoryIndices.remove(i);
                      } else {
                        _expandedHistoryIndices.add(i);
                      }
                    })
                : null,
            displayItemBuilder: widget.displayItemBuilder,
            onEdit: () => _startEditing(i),
            onDelete: () => _deleteEntry(i),
            onCopy: widget.onCopy != null
                ? (fieldId, value) => widget.onCopy!(item, fieldId, value)
                : null,
            onCopyAllPressed: widget.onCopyAll != null
                ? () => _handleCopyAllWithVerification(context, item)
                : null,
            showHistoryExpansion: widget.showHistoryExpansion,
            historyFieldIdPrefix: widget.historyFieldIdPrefix,
            itemId: itemId,
          ),
        );
      }
    }

    return CollapsibleSectionCard(
      title: widget.title,
      icon: widget.icon,
      maxVisibleItems: widget.maxVisibleItems,
      actionIcon: Icons.add,
      onAction: _startAdding,
      children: displayItems,
    );
  }

  bool _hasRestrictedField(T item) {
    for (final field in widget.fieldDefs) {
      if (field.sensitivity == SensitivityLevel.critical) {
        final values = widget.itemToMap?.call(item);
        final value = values?[field.fieldId];
        if (value != null && value.isNotEmpty) {
          return true;
        }
      }
    }
    return false;
  }

  Future<bool> _verifyPasswordForRestricted(BuildContext context) async {
    // Find a representative restricted field ID for verification
    String? restrictedFieldId;
    for (final field in widget.fieldDefs) {
      if (field.sensitivity == SensitivityLevel.critical) {
        restrictedFieldId = field.fieldId;
        break;
      }
    }
    if (restrictedFieldId == null) return true;

    final level = ref.read(fieldLevelProvider(restrictedFieldId));
    if (level != SensitivityLevel.critical) return true;

    // Check if user was verified within the last 1 minute (password cache)
    final sensitiveAccess = ref.read(sensitivePageAccessProvider);
    final oneMinuteAgo = DateTime.now().subtract(const Duration(minutes: 1));
    final hasRecentVerification =
        sensitiveAccess.lastVerified != null &&
        sensitiveAccess.lastVerified!.isAfter(oneMinuteAgo);
    if (hasRecentVerification) return true;

    // Show password dialog
    final authNotifier = ref.read(authNotifierProvider.notifier);
    final selectedAccount = authNotifier.selectedAccount;
    final password = await showPasswordVerificationDialog(
      context: context,
      ref: ref,
      passwordHint: selectedAccount?.passwordHint,
      onVerify: authNotifier.verifyPasswordForSensitiveData,
    );
    if (password == null) return false;

    ref.read(sensitivePageAccessProvider.notifier).markVerified();
    return true;
  }

  Future<void> _handleCopyAllWithVerification(
    BuildContext context,
    T item,
  ) async {
    final hasRestricted = _hasRestrictedField(item);
    if (hasRestricted) {
      final verified = await _verifyPasswordForRestricted(context);
      if (!verified) return;
    }
    if (!mounted) return;
    final formattedText = _formatAllFields(item);
    await widget.onCopyAll?.call(item, formattedText);
  }

  String _formatAllFields(T item) {
    if (widget.itemToMap == null) return '';
    final values = widget.itemToMap!(item);
    final buffer = StringBuffer();
    buffer.writeln(widget.title);
    for (final field in widget.fieldDefs) {
      final value = values[field.fieldId];
      if (value != null && value.isNotEmpty) {
        buffer.writeln('${field.label}: $value');
      }
    }
    return buffer.toString().trim();
  }
}

/// Context to pass action callbacks from _FormSectionItem to EntryItemWidget
class EntryActionsContext extends InheritedWidget {
  final VoidCallback? onEdit;
  final VoidCallback? onDelete;
  final Future<void> Function(String)? onCopy;

  const EntryActionsContext({
    super.key,
    required super.child,
    this.onEdit,
    this.onDelete,
    this.onCopy,
  });

  static EntryActionsContext? of(BuildContext context) {
    return context.dependOnInheritedWidgetOfExactType<EntryActionsContext>();
  }

  @override
  bool updateShouldNotify(EntryActionsContext old) {
    return onEdit != old.onEdit ||
        onDelete != old.onDelete ||
        onCopy != old.onCopy;
  }
}

/// Wrapper widget that wraps an item with edit/delete actions and optional history expansion.
class _ItemWithHistory<T> extends ConsumerWidget {
  final T item;
  final int index;
  final bool historyExpanded;
  final VoidCallback? onToggleHistory;
  final Widget Function(T item) displayItemBuilder;
  final VoidCallback onEdit;
  final VoidCallback onDelete;
  final void Function(String fieldId, String value)? onCopy;
  final VoidCallback? onCopyAllPressed;
  final bool showHistoryExpansion;
  final String? historyFieldIdPrefix;
  final String itemId;

  const _ItemWithHistory({
    required super.key,
    required this.item,
    required this.index,
    required this.historyExpanded,
    this.onToggleHistory,
    required this.displayItemBuilder,
    required this.onEdit,
    required this.onDelete,
    this.onCopy,
    this.onCopyAllPressed,
    required this.showHistoryExpansion,
    this.historyFieldIdPrefix,
    required this.itemId,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    // Look up history if history expansion is enabled
    final history = showHistoryExpansion && historyFieldIdPrefix != null
        ? ref.watch(fieldHistoriesProvider.notifier).getHistory(itemId, historyFieldIdPrefix!)
        : null;

    return EntryActionsContext(
      onEdit: onEdit,
      onDelete: onDelete,
      onCopy: onCopyAllPressed != null ? (text) async => onCopyAllPressed!() : null,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          GestureDetector(
            onLongPress: () => _showActions(context),
            child: Padding(
              padding: const EdgeInsets.symmetric(vertical: 8),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Expanded(child: displayItemBuilder(item)),
                ],
              ),
            ),
          ),
          // History button row
          if (showHistoryExpansion)
            Padding(
              padding: const EdgeInsets.only(left: 32, bottom: 4),
              child: TextButton.icon(
                icon: Icon(
                  historyExpanded ? Icons.expand_less : Icons.history,
                  size: 16,
                ),
                label: Text('History(${history?.entries.length ?? 0})'),
                onPressed: onToggleHistory,
              ),
            ),
          // Expanded history view
          if (showHistoryExpansion && historyExpanded && history != null)
            Padding(
              padding: const EdgeInsets.only(left: 32, bottom: 8),
              child: FieldHistoryView(
                fieldName: historyFieldIdPrefix ?? 'field',
                history: history,
              ),
            ),
        ],
      ),
    );
  }

  Future<void> _showActions(BuildContext context) async {
    showModalBottomSheet(
      context: context,
      builder: (ctx) => SafeArea(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            if (onCopyAllPressed != null)
              ListTile(
                leading: const Icon(Icons.copy_all),
                title: const Text('Copy All'),
                onTap: () {
                  Navigator.pop(ctx);
                  onCopyAllPressed!();
                },
              ),
            ListTile(
              leading: const Icon(Icons.edit_outlined),
              title: const Text('Edit'),
              onTap: () {
                Navigator.pop(ctx);
                onEdit();
              },
            ),
            ListTile(
              leading: const Icon(Icons.delete_outline),
              title: const Text('Delete'),
              onTap: () {
                Navigator.pop(ctx);
                onDelete();
              },
            ),
          ],
        ),
      ),
    );
  }
}

/// Delete confirmation dialog
Future<bool> showDeleteConfirmationDialog({
  required BuildContext context,
  required String itemName,
  required String itemType,
}) async {
  final result = await showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text('Delete $itemType?'),
      content: Text(
        'Are you sure you want to delete "$itemName"?\n\nThis $itemType will be moved to trash and permanently deleted after 30 days.',
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context, false),
          child: const Text('Cancel'),
        ),
        TextButton(
          onPressed: () => Navigator.pop(context, true),
          style: TextButton.styleFrom(foregroundColor: Colors.red),
          child: const Text('Delete'),
        ),
      ],
    ),
  );
  return result ?? false;
}
