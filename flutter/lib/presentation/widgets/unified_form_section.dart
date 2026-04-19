import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/section_card.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart'
    show SensitivityTag;
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart'
    show authNotifierProvider, sensitivePageAccessProvider;

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
  final Future<void> Function(Map<String, String> values, T? editingItem)
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

  /// If false, hides the internal copy/edit/delete action buttons on _FormSectionItem.
  /// Set to false when displayItemBuilder (EntryItemWidget) provides its own actions.
  final bool showInternalActions;

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
    this.showInternalActions = true,
  });

  @override
  ConsumerState<UnifiedFormSection<T>> createState() =>
      _UnifiedFormSectionState<T>();
}

class _UnifiedFormSectionState<T> extends ConsumerState<UnifiedFormSection<T>> {
  String _mode = 'idle';
  int _editingIndex = -1;
  late List<T> _items;

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
    setState(() => _mode = 'idle');
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

  void _submitForm() {
    final values = <String, String>{};
    for (final field in widget.fieldDefs) {
      values[field.fieldId] = _controllers[field.fieldId]?.text ?? '';
    }

    final wasAdding = _mode == 'adding';
    final editingItem = wasAdding ? null : _items[_editingIndex];

    setState(() {
      if (wasAdding) {
        // Create new item via itemFactory
        if (widget.itemFactory != null) {
          _items.add(widget.itemFactory!(values));
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

    widget.onSave(values, editingItem);
  }

  Widget _buildForm(ThemeData theme) {
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
        ...widget.fieldDefs.map((field) {
          return Padding(
            padding: const EdgeInsets.only(bottom: 12),
            child: TextField(
              controller: _controllers[field.fieldId],
              maxLength: kMaxFieldLength,
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

    return CollapsibleSectionCard(
      title: widget.title,
      icon: widget.icon,
      maxVisibleItems: widget.maxVisibleItems,
      actionIcon: Icons.add,
      onAction: _startAdding,
      footer: isEditing ? _buildForm(Theme.of(context)) : null,
      children: _items.isEmpty
          ? [_buildEmpty(Theme.of(context))]
          : _items.asMap().entries.map((e) {
              final i = e.key;
              final item = e.value;
              return GestureDetector(
                onLongPress: () => _showItemActions(context, item, i),
                child: _FormSectionItem(
                  item: item,
                  displayItemBuilder: widget.displayItemBuilder,
                  onEdit: () => _startEditing(i),
                  onDelete: () => _deleteEntry(i),
                  onCopy: widget.onCopy != null
                      ? (fieldId, value) => widget.onCopy!(item, fieldId, value)
                      : null,
                  onCopyAllPressed: widget.onCopyAll != null
                      ? () => _handleCopyAllWithVerification(context, item)
                      : null,
                  showInternalActions: widget.showInternalActions,
                ),
              );
            }).toList(),
    );
  }

  Future<void> _showItemActions(BuildContext context, T item, int index) async {
    if (widget.onCopyAll != null) {
      final hasRestricted = _hasRestrictedField(item);
      if (hasRestricted) {
        final verified = await _verifyPasswordForRestricted(context);
        if (!verified) return;
      }
    }

    if (!mounted) return;
    showModalBottomSheet(
      context: context,
      builder: (ctx) => SafeArea(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            if (widget.onCopyAll != null)
              ListTile(
                leading: const Icon(Icons.copy_all),
                title: const Text('Copy All'),
                onTap: () async {
                  Navigator.pop(ctx);
                  await widget.onCopyAll!(item, _formatAllFields(item));
                },
              ),
            ListTile(
              leading: const Icon(Icons.edit_outlined),
              title: const Text('Edit'),
              onTap: () {
                Navigator.pop(ctx);
                _startEditing(index);
              },
            ),
            ListTile(
              leading: const Icon(Icons.delete_outline),
              title: const Text('Delete'),
              onTap: () {
                Navigator.pop(ctx);
                _deleteEntry(index);
              },
            ),
          ],
        ),
      ),
    );
  }

  bool _hasRestrictedField(T item) {
    for (final field in widget.fieldDefs) {
      if (field.sensitivity == SensitivityLevel.restricted) {
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
    final settings = ref.read(sensitivitySettingsProvider);
    // Find a representative restricted field ID for verification
    String? restrictedFieldId;
    for (final field in widget.fieldDefs) {
      if (field.sensitivity == SensitivityLevel.restricted) {
        restrictedFieldId = field.fieldId;
        break;
      }
    }
    if (restrictedFieldId == null) return true;

    final level = settings.getFieldLevel(restrictedFieldId);
    if (level != SensitivityLevel.restricted) return true;

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

/// Wrapper widget that wraps an item with edit/delete actions
class _FormSectionItem<T> extends StatelessWidget {
  final T item;
  final Widget Function(T item) displayItemBuilder;
  final VoidCallback onEdit;
  final VoidCallback onDelete;
  final void Function(String fieldId, String value)? onCopy;

  /// Called when copy-all button is pressed. Parent handles verification + formatting.
  final VoidCallback? onCopyAllPressed;

  /// If true, shows internal copy/edit/delete buttons. Set to false when
  /// displayItemBuilder (EntryItemWidget) already provides its own actions.
  final bool showInternalActions;

  const _FormSectionItem({
    required this.item,
    required this.displayItemBuilder,
    required this.onEdit,
    required this.onDelete,
    this.onCopy,
    this.onCopyAllPressed,
    this.showInternalActions = true,
  });

  @override
  Widget build(BuildContext context) {
    return EntryActionsContext(
      onEdit: showInternalActions ? null : onEdit,
      onDelete: showInternalActions ? null : onDelete,
      onCopy: showInternalActions ? null : (onCopyAllPressed != null ? (text) async => onCopyAllPressed!() : null),
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 8),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Expanded(child: displayItemBuilder(item)),
            if (showInternalActions) ...[
              if (onCopyAllPressed != null)
                IconButton(
                  icon: const Icon(Icons.copy_all, size: 20),
                  tooltip: 'Copy All',
                  onPressed: onCopyAllPressed,
                  visualDensity: VisualDensity.compact,
                ),
              IconButton(
                icon: const Icon(Icons.edit_outlined, size: 20),
                tooltip: 'Edit',
                onPressed: onEdit,
                visualDensity: VisualDensity.compact,
              ),
              IconButton(
                icon: const Icon(Icons.delete_outline, size: 20),
                tooltip: 'Delete',
                onPressed: onDelete,
                visualDensity: VisualDensity.compact,
              ),
            ],
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
