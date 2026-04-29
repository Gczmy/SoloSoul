import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/field_history_service.dart';
import 'package:solosoul_flutter/core/models/field_history_models.dart'
    show FieldHistory;
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/core/services/operation_notification.dart';
import 'package:solosoul_flutter/core/services/operation_logger.dart';
import 'package:solosoul_flutter/presentation/models/operation_log_models.dart';
import 'package:solosoul_flutter/presentation/providers/operation_log_provider.dart';
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';

import 'package:solosoul_flutter/presentation/widgets/field_history_view.dart';
import 'package:solosoul_flutter/presentation/widgets/icon_picker_sheet.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitive_value_widget.dart';
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    show showOverlaySnackBar, SnackBarType;


/// Card displaying a Section and its Items.
///
/// A Section contains multiple Items (UnifiedObject with typeId='item').
/// Each Item has its own properties (key-value pairs).
/// The Section's `properties` field defines the Item template.
class ObjectCard extends ConsumerStatefulWidget {
  final UnifiedObject object;
  final List<UnifiedObject> items;

  const ObjectCard({
    super.key,
    required this.object,
    required this.items,
  });

  @override
  ConsumerState<ObjectCard> createState() => _ObjectCardState();
}

const int kMaxPropertyLength = 128;

class _ObjectCardState extends ConsumerState<ObjectCard> {
  static const String _historyFieldId = 'unified';
  final Set<String> _expandedHistoryItemIds = {};
  String? _editingItemId;
  bool _isAddingItem = false;
  bool _isExpanded = false;
  final Map<String, TextEditingController> _editControllers = {};
  /// Tracks original values when entering edit/add mode for dirty-check.
  final Map<String, String> _originalValues = {};
  bool _hasChanges = false;

  void _disposeControllers() {
    for (final c in _editControllers.values) {
      c.dispose();
    }
    _editControllers.clear();
    _originalValues.clear();
    _hasChanges = false;
  }

  /// Compare current controller values with originals; update [_hasChanges].
  void _checkForChanges() {
    var changed = false;
    for (final entry in _editControllers.entries) {
      final original = _originalValues[entry.key] ?? '';
      if (entry.value.text != original) {
        changed = true;
        break;
      }
    }
    if (_hasChanges != changed) {
      setState(() => _hasChanges = changed);
    }
  }

  void _setupChangeDetection() {
    for (final controller in _editControllers.values) {
      controller.addListener(_checkForChanges);
    }
  }

  @override
  void dispose() {
    _disposeControllers();
    super.dispose();
  }

  /// Get the display title for an item, looking up 'Title' then 'Item Name'.
  String _itemDisplayTitle(UnifiedObject item) {
    final titleProp = item.properties['Title'];
    if (titleProp is TextProperty && titleProp.text.isNotEmpty) {
      return titleProp.text;
    }
    final oldNameProp = item.properties['Item Name'];
    if (oldNameProp is TextProperty && oldNameProp.text.isNotEmpty) {
      return oldNameProp.text;
    }
    return item.name;
  }

  void _addItem() {
    setState(() {
      _isAddingItem = true;
      _editingItemId = null;
      _disposeControllers();

      final template = widget.object.properties;
      // Title defaults from template
      final titleValue = template['Title'] is TextProperty
          ? (template['Title'] as TextProperty).text
          : 'Item Name';
      _editControllers['__name__'] = TextEditingController(text: titleValue);
      _originalValues['__name__'] = titleValue;

      // Other fields start empty
      for (final key in template.keys.skip(1)) {
        _editControllers[key] = TextEditingController(text: '');
        _originalValues[key] = '';
      }

      _setupChangeDetection();
      _hasChanges = false;
    });
  }

  Future<void> _saveNewItem() async {
    final template = widget.object.properties;
    final properties = Map<String, PropertyValue>.from(template);

    final nameInput = _editControllers['__name__']?.text.trim() ?? '';
    if (properties.containsKey('Title')) {
      final oldTitle = properties['Title']!;
      properties['Title'] = TextProperty(
        text: nameInput.isNotEmpty ? nameInput : 'Item Name',
        sensitivity: oldTitle.sensitivity,
      );
    }

    for (final key in template.keys.skip(1)) {
      final controller = _editControllers[key];
      if (controller != null && properties.containsKey(key)) {
        final oldValue = template[key]!;
        properties[key] = _parsePropertyValue(oldValue, controller.text);
      }
    }

    final name = (properties['Title'] is TextProperty)
        ? (properties['Title'] as TextProperty).text
        : 'Item';

    await ref.read(unifiedObjectProvider.notifier).createObject(
      name: name,
      typeId: 'item',
      parentId: widget.object.id,
      properties: properties,
    );

    OperationLogService.instance.addEntry(
      OperationLogger.logCustomSection(
        section: widget.object.name,
        action: LogAction.create,
        description: 'Created item "$name"',
      ),
    );

    if (mounted) {
      final isPrivacyMode =
          ref.read(accountStyleProvider).value?.displayMode ==
              SensitivityDisplayMode.hidePrivate;
      OperationNotification.show(
        context,
        message: OperationLogger.createNotificationForSection(
          section: widget.object.name,
          action: LogAction.create,
          itemName: name,
          isPrivacyModeActive: isPrivacyMode,
        ),
      );
    }

    setState(() {
      _isAddingItem = false;
      _disposeControllers();
    });
  }

  void _cancelAddItem() {
    setState(() {
      _isAddingItem = false;
      _disposeControllers();
    });
  }

  bool _itemHasSensitiveProperties(UnifiedObject item) {
    return item.properties.values.any(
      (p) => p.sensitivity == SensitivityLevel.sensitive ||
             p.sensitivity == SensitivityLevel.critical,
    );
  }

  Future<void> _handleWithVerification(VoidCallback onSuccess) async {
    if (ref.read(isSensitiveAccessGrantedProvider)) {
      onSuccess();
      return;
    }
    final authNotifier = ref.read(authNotifierProvider.notifier);
    final selectedAccount = authNotifier.selectedAccount;
    final password = await showPasswordVerificationDialog(
      context: context,
      ref: ref,
      passwordHint: selectedAccount?.passwordHint,
      onVerify: authNotifier.verifyPasswordForSensitiveData,
    );
    if (password == null) return;
    ref.read(sensitivePageAccessProvider.notifier).markVerified();
    onSuccess();
  }

  Future<void> _deleteItem(String itemId) async {
    final item = ref.read(objectByIdProvider(itemId));
    if (item == null) return;

    final itemName = _itemDisplayTitle(item);
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) {
        final theme = Theme.of(context);
        return AlertDialog(
          title: const Text('Delete Item'),
          content: Text('Are you sure you want to delete "$itemName"?'),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(context).pop(false),
              child: const Text('Cancel'),
            ),
            TextButton(
              onPressed: () => Navigator.of(context).pop(true),
              child: Text('Delete', style: TextStyle(color: theme.colorScheme.error)),
            ),
          ],
        );
      },
    );
    if (confirmed != true) return;

    Future<void> doDelete() async {
      await ref.read(unifiedObjectProvider.notifier).deleteObject(itemId);

      OperationLogService.instance.addEntry(
        OperationLogger.logCustomSection(
          section: widget.object.name,
          action: LogAction.delete,
          description: 'Deleted item "${_itemDisplayTitle(item)}"',
          fieldPath: itemId,
        ),
      );

      if (mounted) {
        final isPrivacyMode =
            ref.read(accountStyleProvider).value?.displayMode ==
                SensitivityDisplayMode.hidePrivate;
        OperationNotification.show(
          context,
          message: OperationLogger.createNotificationForSection(
            section: widget.object.name,
            action: LogAction.delete,
            itemName: _itemDisplayTitle(item),
            isPrivacyModeActive: isPrivacyMode,
          ),
          duration: const Duration(seconds: 5),
          onUndo: () async {
            await ref.read(unifiedObjectProvider.notifier).restoreObject(itemId);
          },
        );
      }
    }

    if (_itemHasSensitiveProperties(item)) {
      await _handleWithVerification(doDelete);
    } else {
      await doDelete();
    }
  }

  void _startEditingItem(UnifiedObject item) {
    void doEdit() {
      setState(() {
        _editingItemId = item.id;
        _isAddingItem = false;
        _disposeControllers();
        _editControllers['__name__'] = TextEditingController(text: _itemDisplayTitle(item));
        _originalValues['__name__'] = _itemDisplayTitle(item);
        for (final entry in item.properties.entries) {
          final textValue = _propValueToString(entry.value);
          _editControllers[entry.key] = TextEditingController(text: textValue);
          _originalValues[entry.key] = textValue;
        }
        _setupChangeDetection();
        _hasChanges = false;
      });
    }

    if (_itemHasSensitiveProperties(item)) {
      _handleWithVerification(doEdit);
    } else {
      doEdit();
    }
  }

  void _cancelEditItem() {
    setState(() {
      _editingItemId = null;
      _disposeControllers();
    });
  }

  Future<void> _saveEditItem(String itemId) async {
    final item = ref.read(objectByIdProvider(itemId));
    if (item == null) return;

    final updatedProps = Map<String, PropertyValue>.from(item.properties);

    // Sync __name__ input to Title property so title and property stay aligned
    final nameInput = _editControllers['__name__']?.text.trim() ?? item.name;
    if (updatedProps.containsKey('Title')) {
      final oldTitle = updatedProps['Title']!;
      updatedProps['Title'] = TextProperty(
        text: nameInput,
        sensitivity: oldTitle.sensitivity,
      );
    } else if (updatedProps.containsKey('Item Name')) {
      final oldName = updatedProps['Item Name']!;
      updatedProps['Item Name'] = TextProperty(
        text: nameInput,
        sensitivity: oldName.sensitivity,
      );
    }

    for (final key in item.properties.keys) {
      if (key == 'Title' || key == 'Item Name') continue; // already handled above
      final controller = _editControllers[key];
      if (controller != null) {
        final oldValue = item.properties[key]!;
        updatedProps[key] = _parsePropertyValue(oldValue, controller.text);
      }
    }

    // name is taken from Title/Item Name property if present
    final name = _itemDisplayTitle(item.copyWith(properties: updatedProps, name: nameInput));

    // Record history before update
    final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
    if (accountId != null) {
      final oldValues = <String, String>{};
      for (final entry in item.properties.entries) {
        oldValues[entry.key] = _propValueToString(entry.value);
      }
      await ref.read(fieldHistoriesProvider.notifier).recordSnapshot(
        accountId: accountId,
        itemId: itemId,
        fieldIdPrefix: _historyFieldId,
        allFieldValues: oldValues,
      );
    }

    await ref.read(unifiedObjectProvider.notifier).updateObject(
      itemId,
      name: name,
      properties: updatedProps,
    );

    OperationLogService.instance.addEntry(
      OperationLogger.logCustomSection(
        section: widget.object.name,
        action: LogAction.update,
        description: 'Updated item "$name"',
        fieldPath: itemId,
      ),
    );

    if (mounted) {
      final isPrivacyMode =
          ref.read(accountStyleProvider).value?.displayMode ==
              SensitivityDisplayMode.hidePrivate;
      OperationNotification.show(
        context,
        message: OperationLogger.createNotificationForSection(
          section: widget.object.name,
          action: LogAction.update,
          itemName: name,
          isPrivacyModeActive: isPrivacyMode,
        ),
      );
    }

    _cancelEditItem();
  }

  PropertyValue _parsePropertyValue(PropertyValue oldValue, String newText) {
    return switch (oldValue) {
      TextProperty() => TextProperty(text: newText, sensitivity: oldValue.sensitivity),
      NumberProperty() => NumberProperty(value: double.tryParse(newText), sensitivity: oldValue.sensitivity),
      DateProperty() => DateProperty(isoDate: newText, sensitivity: oldValue.sensitivity),
      CheckboxProperty() => CheckboxProperty(
          checked: newText.toLowerCase() == 'true' || newText == '1' || newText == 'yes',
          sensitivity: oldValue.sensitivity,
        ),
      SelectProperty() => oldValue,
      MultiSelectProperty() => oldValue,
      RelationProperty() => oldValue,
      UrlProperty() => oldValue,
    };
  }

  void _editObject() {
    context.push('/object_editor?id=${widget.object.id}');
  }

  void _deleteObject() async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Delete Section'),
        content: Text(
          'Are you sure you want to delete "${widget.object.name}"?',
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

    if (confirmed == true) {
      await ref.read(unifiedObjectProvider.notifier).deleteObject(widget.object.id);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Section deleted')),
        );
      }
    }
  }

  Future<void> _changeIcon() async {
    final result = await showModalBottomSheet<String>(
      context: context,
      builder: (ctx) => IconPickerSheet(currentIcon: widget.object.iconName),
    );
    if (result != null && result != widget.object.iconName) {
      await ref.read(unifiedObjectProvider.notifier).updateObject(
        widget.object.id,
        iconName: result,
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final items = widget.items;
    final shouldCollapse = items.length > 3;
    final visibleItems = _isExpanded ? items : items.take(3).toList();

    // 一次性读取 fieldHistories，避免每个 item 都创建独立 provider 订阅
    final histories = ref.watch(fieldHistoriesProvider);

    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            _ObjectCardHeader(
              object: widget.object,
              onChangeIcon: _changeIcon,
              onEdit: _editObject,
              onDelete: _deleteObject,
              onAddItem: _addItem,
            ),

            const Divider(height: 24),

            // Items list
            if (items.isEmpty && !_isAddingItem)
              Center(
                child: Padding(
                  padding: const EdgeInsets.symmetric(vertical: 16),
                  child: TextButton.icon(
                    onPressed: _addItem,
                    icon: const Icon(Icons.add, size: 18),
                    label: const Text('Add Item'),
                  ),
                ),
              )
            else ...[
              if (_isAddingItem) _buildNewItemForm(),
              ...visibleItems.map((item) {
                final history = histories.getHistory(item.id, _historyFieldId);
                return _buildItemTile(item, history);
              }),
              if (shouldCollapse && !_isAddingItem) ...[
                const SizedBox(height: 8),
                InkWell(
                  onTap: () => setState(() => _isExpanded = !_isExpanded),
                  borderRadius: BorderRadius.circular(8),
                  child: Padding(
                    padding: const EdgeInsets.symmetric(vertical: 8),
                    child: Row(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        Icon(
                          _isExpanded ? Icons.keyboard_arrow_up : Icons.keyboard_arrow_down,
                          size: 20,
                          color: theme.colorScheme.primary,
                        ),
                        const SizedBox(width: 4),
                        Text(
                          _isExpanded
                              ? 'Show less'
                              : 'Show ${items.length - 3} more',
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: theme.colorScheme.primary,
                            fontWeight: FontWeight.w500,
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ],
            ],


          ],
        ),
      ),
    );
  }

  Widget _buildItemTile(UnifiedObject item, FieldHistory? history) {
    final isEditing = _editingItemId == item.id;
    return isEditing
        ? _buildItemEditMode(item)
        : _buildItemViewMode(item, history);
  }

  void _toggleItemHistory(String itemId) {
    setState(() {
      if (_expandedHistoryItemIds.contains(itemId)) {
        _expandedHistoryItemIds.remove(itemId);
      } else {
        _expandedHistoryItemIds.add(itemId);
      }
    });
  }

  Widget _buildItemViewMode(UnifiedObject item, FieldHistory? history) {
    final theme = Theme.of(context);
    final isHistoryExpanded = _expandedHistoryItemIds.contains(item.id);
    final hasHistory = history?.entries.isNotEmpty == true;

    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Item header: name + actions
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    SelectableText(
                      _itemDisplayTitle(item),
                      style: theme.textTheme.bodyLarge?.copyWith(
                        fontWeight: FontWeight.w500,
                      ),
                    ),
                    const SizedBox(height: 4),
                    _ObjectCardPropertiesList(item: item),
                  ],
                ),
              ),
              // Action buttons
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  IconButton(
                    icon: const Icon(Icons.copy_all, size: 20),
                    tooltip: 'Copy',
                    onPressed: () => _copyItem(item),
                    visualDensity: VisualDensity.compact,
                  ),
                  IconButton(
                    icon: const Icon(Icons.edit_outlined, size: 20),
                    tooltip: 'Edit',
                    onPressed: () => _startEditingItem(item),
                    visualDensity: VisualDensity.compact,
                  ),
                  if (hasHistory)
                    IconButton(
                      icon: Icon(
                        isHistoryExpanded ? Icons.history_toggle_off : Icons.history,
                        size: 20,
                      ),
                      tooltip: 'History',
                      onPressed: () => _toggleItemHistory(item.id),
                      visualDensity: VisualDensity.compact,
                    ),
                  IconButton(
                    icon: Icon(Icons.delete_outline, size: 20, color: theme.colorScheme.error),
                    tooltip: 'Delete',
                    onPressed: () => _deleteItem(item.id),
                    visualDensity: VisualDensity.compact,
                  ),
                ],
              ),
            ],
          ),
          // Item history
          if (isHistoryExpanded) ...[
            const SizedBox(height: 8),
            _ObjectCardHistorySection(history: history),
          ],
          const Divider(height: 16),
        ],
      ),
    );
  }

  Widget _buildNewItemForm() {
    final template = widget.object.properties;

    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Title input
          _buildTitleField(),
          const SizedBox(height: 12),
          // Property inputs (skip the first fixed Title entry)
          ...template.keys.skip(1).map((key) {
            return Padding(
              padding: const EdgeInsets.only(bottom: 8),
              child: _buildEditPropertyField(key, template[key]!),
            );
          }),
          const SizedBox(height: 12),
          // Action buttons
          Row(
            mainAxisAlignment: MainAxisAlignment.end,
            children: [
              TextButton(
                onPressed: _cancelAddItem,
                child: const Text('Cancel'),
              ),
              const SizedBox(width: 8),
              FilledButton(
                onPressed: _hasChanges ? _saveNewItem : null,
                child: const Text('Add'),
              ),
            ],
          ),
          const Divider(height: 16),
        ],
      ),
    );
  }

  Widget _buildItemEditMode(UnifiedObject item) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Title input
          _buildTitleField(),
          const SizedBox(height: 12),
          // Property inputs (skip the first fixed Title entry — already shown above)
          ...item.properties.keys.skip(1).map((key) {
            return Padding(
              padding: const EdgeInsets.only(bottom: 8),
              child: _buildEditPropertyField(key, item.properties[key]!),
            );
          }),
          const SizedBox(height: 12),
          // Action buttons
          Row(
            mainAxisAlignment: MainAxisAlignment.end,
            children: [
              TextButton(
                onPressed: _cancelEditItem,
                child: const Text('Cancel'),
              ),
              const SizedBox(width: 8),
              FilledButton(
                onPressed: _hasChanges ? () => _saveEditItem(item.id) : null,
                child: const Text('Save'),
              ),
            ],
          ),
          const Divider(height: 16),
        ],
      ),
    );
  }

  Widget _buildEditPropertyField(String key, PropertyValue value) {
    final controller = _editControllers[key];

    return switch (value) {
      CheckboxProperty(:final checked) => Row(
        children: [
          Checkbox(
            value: checked,
            onChanged: (newValue) {
              setState(() {
                _editControllers[key]?.text = (newValue ?? false) ? 'Yes' : 'No';
              });
            },
          ),
          Text(key),
          const SizedBox(width: 8),
          SensitivityTag(level: value.sensitivity),
        ],
      ),
      _ => Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          Expanded(
            child: TextField(
              controller: controller,
              maxLength: kMaxPropertyLength,
              buildCounter: (context, {required currentLength, required isFocused, maxLength}) => null,
              decoration: InputDecoration(
                labelText: key,
                border: const OutlineInputBorder(),
                suffixIcon: Padding(
                  padding: const EdgeInsets.only(right: 12),
                  child: Align(
                    alignment: Alignment.centerRight,
                    widthFactor: 1,
                    child: SensitivityTag(level: value.sensitivity),
                  ),
                ),
              ),
              keyboardType: value is NumberProperty ? TextInputType.number : null,
            ),
          ),
          const SizedBox(width: 8),
          SizedBox(
            width: 64,
            child: ValueListenableBuilder<TextEditingValue>(
              valueListenable: controller ?? TextEditingController(),
              builder: (context, val, child) {
                final len = val.text.length;
                const max = kMaxPropertyLength;
                return Row(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    SizedBox(
                      width: 28,
                      child: Text(
                        '$len',
                        textAlign: TextAlign.right,
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: len >= max ? Theme.of(context).colorScheme.error : Theme.of(context).colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ),
                    Text(
                      '/',
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: len >= max ? Theme.of(context).colorScheme.error : Theme.of(context).colorScheme.onSurfaceVariant,
                      ),
                    ),
                    SizedBox(
                      width: 28,
                      child: Text(
                        '$max',
                        textAlign: TextAlign.left,
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: len >= max ? Theme.of(context).colorScheme.error : Theme.of(context).colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ),
                  ],
                );
              },
            ),
          ),
        ],
      ),
    };
  }

  Widget _buildTitleField() {
    final controller = _editControllers['__name__'];
    return Row(
      crossAxisAlignment: CrossAxisAlignment.center,
      children: [
        Expanded(
          child: TextField(
            controller: controller,
            maxLength: kMaxPropertyLength,
            buildCounter: (context, {required currentLength, required isFocused, maxLength}) => null,
            decoration: const InputDecoration(
              labelText: 'Title',
              border: OutlineInputBorder(),
            ),
          ),
        ),
        const SizedBox(width: 8),
        SizedBox(
          width: 64,
          child: ValueListenableBuilder<TextEditingValue>(
            valueListenable: controller ?? TextEditingController(),
            builder: (context, val, child) {
              final len = val.text.length;
              const max = kMaxPropertyLength;
              return Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  SizedBox(
                    width: 28,
                    child: Text(
                      '$len',
                      textAlign: TextAlign.right,
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: len >= max ? Theme.of(context).colorScheme.error : Theme.of(context).colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ),
                  Text(
                    '/',
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: len >= max ? Theme.of(context).colorScheme.error : Theme.of(context).colorScheme.onSurfaceVariant,
                    ),
                  ),
                  SizedBox(
                    width: 28,
                    child: Text(
                      '$max',
                      textAlign: TextAlign.left,
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: len >= max ? Theme.of(context).colorScheme.error : Theme.of(context).colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ),
                ],
              );
            },
          ),
        ),
      ],
    );
  }

  void _copyItem(UnifiedObject item) {
    void doCopy() {
      final buffer = StringBuffer();
      buffer.writeln('${item.name}:');
      for (final entry in item.properties.entries) {
        buffer.writeln('  ${entry.key}: ${_propValueToString(entry.value)}');
      }
      Clipboard.setData(ClipboardData(text: buffer.toString()));
      showOverlaySnackBar(
        context,
        content: 'Copied to clipboard',
        type: SnackBarType.success,
      );
    }

    if (_itemHasSensitiveProperties(item)) {
      _handleWithVerification(doCopy);
    } else {
      doCopy();
    }
  }
}

// =============================================================================
// File-level helpers (used by sub-widgets)
// =============================================================================

String _propValueToString(PropertyValue value) {
  return switch (value) {
    TextProperty(:final text) => text,
    NumberProperty(:final value) => value?.toString() ?? '',
    DateProperty(:final isoDate) => isoDate ?? '',
    CheckboxProperty(:final checked) => checked ? 'Yes' : 'No',
    SelectProperty(:final selectedId) => selectedId ?? '',
    _ => '',
  };
}

String _wrapEveryNChars(String text, int n) {
  if (text.length <= n) return '$text: ';
  final buffer = StringBuffer();
  for (var i = 0; i < text.length; i += n) {
    if (i > 0) buffer.write('\n');
    buffer.write(text.substring(i, i + n > text.length ? text.length : i + n));
  }
  buffer.write(': ');
  return buffer.toString();
}

// =============================================================================
// _ObjectCardHeader — Card header with icon, name, and action buttons
// =============================================================================

class _ObjectCardHeader extends StatelessWidget {
  final UnifiedObject object;
  final VoidCallback onChangeIcon;
  final VoidCallback onEdit;
  final VoidCallback onDelete;
  final VoidCallback onAddItem;

  const _ObjectCardHeader({
    required this.object,
    required this.onChangeIcon,
    required this.onEdit,
    required this.onDelete,
    required this.onAddItem,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final icon = UnifiedObjectService.getIconFromName(object.iconName);

    return Row(
      children: [
        InkWell(
          onTap: onChangeIcon,
          borderRadius: BorderRadius.circular(6),
          child: Padding(
            padding: const EdgeInsets.all(4),
            child: Icon(icon, color: theme.colorScheme.primary, size: 20),
          ),
        ),
        const SizedBox(width: 8),
        Expanded(
          child: Text(
            object.name,
            style: theme.textTheme.titleMedium?.copyWith(
              fontWeight: FontWeight.w600,
            ),
            overflow: TextOverflow.ellipsis,
          ),
        ),
        IconButton(
          icon: const Icon(Icons.edit_outlined, size: 18),
          onPressed: onEdit,
          tooltip: 'Edit',
          padding: EdgeInsets.zero,
          constraints: const BoxConstraints(minWidth: 28, minHeight: 28),
          visualDensity: VisualDensity.compact,
        ),
        IconButton(
          icon: const Icon(Icons.delete_outline, size: 18),
          onPressed: onDelete,
          tooltip: 'Delete',
          padding: EdgeInsets.zero,
          constraints: const BoxConstraints(minWidth: 28, minHeight: 28),
          visualDensity: VisualDensity.compact,
        ),
        IconButton(
          icon: const Icon(Icons.add, size: 18),
          onPressed: onAddItem,
          tooltip: 'Add Item',
          padding: EdgeInsets.zero,
          constraints: const BoxConstraints(minWidth: 28, minHeight: 28),
          visualDensity: VisualDensity.compact,
        ),
      ],
    );
  }
}

// =============================================================================
// _ObjectCardPropertiesList — Renders an item's property rows
// =============================================================================

class _ObjectCardPropertiesList extends StatelessWidget {
  final UnifiedObject item;

  const _ObjectCardPropertiesList({required this.item});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: item.properties.entries
          .where((e) => e.key != 'Title')
          .map((entry) {
        final sensitivity = entry.value.sensitivity;
        final isSensitive = sensitivity == SensitivityLevel.sensitive ||
            sensitivity == SensitivityLevel.critical;
        final valueStr = _propValueToString(entry.value);
        final isEmptyValue = valueStr.isEmpty;

        return Padding(
          padding: const EdgeInsets.only(left: 8, bottom: 2),
          child: Row(
            children: [
              ConstrainedBox(
                constraints: const BoxConstraints(maxWidth: 160),
                child: SelectableText(
                  _wrapEveryNChars(entry.key, 12),
                  style: theme.textTheme.bodyMedium?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              ),
              if (isEmptyValue)
                Expanded(
                  child: Text(
                    '(empty)',
                    style: theme.textTheme.bodyMedium?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                      fontStyle: FontStyle.italic,
                    ),
                  ),
                )
              else if (isSensitive)
                Expanded(
                  child: SensitiveValueWidget(
                    fieldId: 'item.${item.id}.${entry.key}',
                    value: valueStr,
                    sensitivityLevel: sensitivity,
                  ),
                )
              else
                Expanded(
                  child: SelectableText(
                    valueStr,
                    style: theme.textTheme.bodyMedium,
                  ),
                ),
              const SizedBox(width: 6),
              SensitivityTag(level: sensitivity),
            ],
          ),
        );
      }).toList(),
    );
  }
}

// =============================================================================
// _ObjectCardHistorySection — Renders field history for an item
// =============================================================================

class _ObjectCardHistorySection extends StatelessWidget {
  final FieldHistory? history;

  const _ObjectCardHistorySection({this.history});

  @override
  Widget build(BuildContext context) {
    return FieldHistoryView(
      fieldName: 'unified',
      history: history,
      initiallyExpanded: true,
    );
  }
}
