import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:solosoul_flutter/presentation/utils/format_field_label.dart';
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

import 'package:solosoul_flutter/presentation/widgets/entry_actions_context.dart';
import 'package:solosoul_flutter/presentation/widgets/field_history_view.dart';
import 'package:solosoul_flutter/presentation/widgets/icon_picker_sheet.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitive_value_widget.dart';
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    show showOverlaySnackBar, SnackBarType;


/// Card displaying a Section and its Items.
///
/// A Section contains multiple Items. Each Item has its own properties (key-value pairs).
/// The Section's `properties` field defines the Item template by default,
/// but can be overridden via [itemTemplate] for preset pages.
class ObjectCard extends ConsumerStatefulWidget {
  final UnifiedObject object;
  final List<UnifiedObject> items;

  /// Override for the item type ID. Defaults to `'item'`.
  final String itemTypeId;

  /// Optional template that defines the schema for new/edited items.
  /// When null, uses [object.properties] as the template.
  final Map<String, PropertyValue>? itemTemplate;

  /// Prefix for field history recording. Defaults to `'unified'`.
  final String historyFieldIdPrefix;

  /// Optional callback to derive display name from property values.
  /// When null, falls back to Title/Item Name lookup.
  final String Function(Map<String, String>)? nameExtractor;

  /// Whether to show the "Add Item" button in the header. Defaults to `true`.
  final bool showAddButton;

  /// Whether to show edit/delete/change-icon buttons in the header. Defaults to `true`.
  final bool showEditActions;

  /// Optional override for save logic. Returns the item ID (new or existing).
  final Future<String> Function({
    required String? itemId,
    required String name,
    required Map<String, PropertyValue> properties,
  })? onSaveItem;

  /// Optional override for delete logic. Returns `true` on success.
  final Future<bool> Function(String itemId)? onDeleteItem;

  /// Optional custom form builder. When provided, replaces the default field list
  /// in both add and edit modes. Signature matches [UnifiedFormSection.customFormBuilder].
  final Widget Function(
    BuildContext context,
    ThemeData theme,
    Map<String, TextEditingController> controllers,
    String mode,
    VoidCallback onSubmit,
    VoidCallback onCancel,
    Map<String, SensitivityLevel> fieldSensitivities,
  )? customFormBuilder;

  /// Optional custom item tile renderer. When non-null, replaces [_ObjectCardItemTile].
  final Widget Function(BuildContext context, UnifiedObject item, {required bool isEditing})?
      displayItemBuilder;

  /// Optional callback for copying all fields of an item.
  /// Receives the item and a pre-formatted text string.
  final Future<void> Function(UnifiedObject item, String formattedText)? onCopyAll;

  /// The property key that maps to the title/name input field.
  /// Defaults to `'Title'`. Set to a different key (e.g. `'title'`, `'fullName'`)
  /// for preset pages whose schema uses a different title property.
  final String titlePropertyKey;

  /// Static dummy controller to avoid creating a new TextEditingController on every build.
  static final _dummyController = TextEditingController();

  const ObjectCard({
    super.key,
    required this.object,
    required this.items,
    this.itemTypeId = 'item',
    this.itemTemplate,
    this.historyFieldIdPrefix = 'unified',
    this.nameExtractor,
    this.showAddButton = true,
    this.showEditActions = true,
    this.onSaveItem,
    this.onDeleteItem,
    this.customFormBuilder,
    this.displayItemBuilder,
    this.onCopyAll,
    this.titlePropertyKey = 'Title',
  });

  @override
  ConsumerState<ObjectCard> createState() => _ObjectCardState();
}

const int kMaxPropertyLength = 128;

class _ObjectCardState extends ConsumerState<ObjectCard> {
  final Set<String> _expandedHistoryItemIds = {};
  String? _editingItemId;
  bool _isAddingItem = false;
  bool _isExpanded = false;
  final Map<String, TextEditingController> _editControllers = {};
  /// Tracks original values when entering edit/add mode for dirty-check.
  final Map<String, String> _originalValues = {};
  bool _hasChanges = false;

  /// Resolved template: [itemTemplate] takes precedence over [object.properties].
  Map<String, PropertyValue> get _template =>
      widget.itemTemplate ?? widget.object.properties;

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
    if (widget.nameExtractor != null) {
      final props = <String, String>{
        for (final entry in item.properties.entries)
          entry.key: _propValueToString(entry.value),
      };
      return widget.nameExtractor!(props);
    }
    return _objectItemDisplayTitle(item);
  }



  void _addItem() {
    setState(() {
      _isAddingItem = true;
      _editingItemId = null;
      _disposeControllers();

      final template = _template;
      final titleKey = widget.titlePropertyKey;
      // Title defaults from template
      final titleValue = template[titleKey] is TextProperty
          ? (template[titleKey] as TextProperty).text
          : '';
      _editControllers['__name__'] = TextEditingController(text: titleValue);
      _originalValues['__name__'] = titleValue;

      // Other fields start empty
      for (final key in template.keys.where((k) => k != titleKey)) {
        _editControllers[key] = TextEditingController(text: '');
        _originalValues[key] = '';
      }

      _setupChangeDetection();
      _hasChanges = false;
    });
  }

  Future<void> _saveNewItem() async {
    final template = _template;
    final properties = Map<String, PropertyValue>.from(template);

    final titleKey = widget.titlePropertyKey;
    final nameInput = _editControllers['__name__']?.text.trim() ?? '';
    if (properties.containsKey(titleKey)) {
      final oldTitle = properties[titleKey]!;
      properties[titleKey] = TextProperty(
        text: nameInput,
        sensitivity: oldTitle.sensitivity,
      );
    }

    for (final key in template.keys.where((k) => k != titleKey)) {
      final controller = _editControllers[key];
      if (controller != null && properties.containsKey(key)) {
        final oldValue = template[key]!;
        properties[key] = _parsePropertyValue(oldValue, controller.text);
      }
    }

    // Compute display name using extractor or title property
    final String name;
    if (widget.nameExtractor != null) {
      final props = <String, String>{
        for (final entry in properties.entries)
          entry.key: _propValueToString(entry.value),
      };
      name = widget.nameExtractor!(props);
    } else {
      name = (properties[titleKey] is TextProperty)
          ? (properties[titleKey] as TextProperty).text
          : nameInput;
    }

    if (widget.onSaveItem != null) {
      await widget.onSaveItem!(
        itemId: null,
        name: name,
        properties: properties,
      );
    } else {
      await ref.read(unifiedObjectProvider.notifier).createObject(
        name: name,
        typeId: widget.itemTypeId,
        parentId: widget.object.id,
        properties: properties,
      );
    }

    await OperationLogService.instance.addEntry(
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
      final bool success;
      if (widget.onDeleteItem != null) {
        success = await widget.onDeleteItem!(itemId);
      } else {
        await ref.read(unifiedObjectProvider.notifier).deleteObject(itemId);
        success = true;
      }

      if (!success) return;

      await OperationLogService.instance.addEntry(
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

        // Merge template keys with actual item properties so template-defined
        // fields that the item lacks still appear in edit mode.
        final allKeys = {..._template, ...item.properties}.keys;
        for (final key in allKeys) {
          final value = item.properties[key];
          final textValue = value != null
              ? _propValueToString(value)
              : _template[key] != null
                  ? _propValueToString(_template[key]!)
                  : '';
          _editControllers[key] = TextEditingController(text: textValue);
          _originalValues[key] = textValue;
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

    // Sync __name__ input to the title property so title and property stay aligned
    final titleKey = widget.titlePropertyKey;
    final nameInput = _editControllers['__name__']?.text.trim() ?? item.name;
    if (updatedProps.containsKey(titleKey)) {
      final oldTitle = updatedProps[titleKey]!;
      updatedProps[titleKey] = TextProperty(
        text: nameInput,
        sensitivity: oldTitle.sensitivity,
      );
    }

    for (final key in item.properties.keys) {
      if (key == titleKey) continue; // already handled above
      final controller = _editControllers[key];
      if (controller != null) {
        final oldValue = item.properties[key]!;
        updatedProps[key] = _parsePropertyValue(oldValue, controller.text);
      }
    }

    // Compute display name using extractor or title property
    final String name;
    if (widget.nameExtractor != null) {
      final props = <String, String>{
        for (final entry in updatedProps.entries)
          entry.key: _propValueToString(entry.value),
      };
      name = widget.nameExtractor!(props);
    } else {
      name = _itemDisplayTitle(item.copyWith(properties: updatedProps, name: nameInput));
    }

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
        fieldIdPrefix: widget.historyFieldIdPrefix,
        allFieldValues: oldValues,
      );
    }

    if (widget.onSaveItem != null) {
      await widget.onSaveItem!(
        itemId: itemId,
        name: name,
        properties: updatedProps,
      );
    } else {
      await ref.read(unifiedObjectProvider.notifier).updateObject(
        itemId,
        name: name,
        properties: updatedProps,
      );
    }

    await OperationLogService.instance.addEntry(
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
              showEditActions: widget.showEditActions,
              showAddButton: widget.showAddButton,
              showEditSection: widget.itemTemplate == null && widget.object.properties.isEmpty,
            ),

            const Divider(height: 24),

            // Items list
            if (items.isEmpty && !_isAddingItem)
              Center(
                child: Padding(
                  padding: const EdgeInsets.symmetric(vertical: 16),
                  child: TextButton.icon(
                    onPressed: widget.itemTemplate == null && widget.object.properties.isEmpty
                        ? _editObject
                        : _addItem,
                    icon: Icon(
                      widget.itemTemplate == null && widget.object.properties.isEmpty
                          ? Icons.edit_note
                          : Icons.add,
                      size: 18,
                    ),
                    label: Text(
                      widget.itemTemplate == null && widget.object.properties.isEmpty
                          ? 'Edit Section'
                          : 'Add Item',
                    ),
                  ),
                ),
              )
            else ...[
              if (_isAddingItem) _buildNewItemForm(),
              ...visibleItems.map((item) => _buildItemTile(item)),
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

  Widget _buildItemTile(UnifiedObject item) {
    final isEditing = _editingItemId == item.id;
    if (isEditing) {
      return _buildItemEditMode(item);
    }
    if (widget.displayItemBuilder != null) {
      return EntryActionsContext(
        onEdit: () => _startEditingItem(item),
        onDelete: () => _deleteItem(item.id),
        onCopy: (_) async => _copyItem(item),
        onToggleHistory: () => _toggleItemHistory(item.id),
        child: widget.displayItemBuilder!(context, item, isEditing: false),
      );
    }
    return _ObjectCardItemTile(
      item: item,
      isHistoryExpanded: _expandedHistoryItemIds.contains(item.id),
      onToggleHistory: () => _toggleItemHistory(item.id),
      onCopy: () => _copyItem(item),
      onStartEdit: () => _startEditingItem(item),
      onDelete: () => _deleteItem(item.id),
      historyFieldIdPrefix: widget.historyFieldIdPrefix,
      nameExtractor: widget.nameExtractor,
      titlePropertyKey: widget.titlePropertyKey,
    );
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

  Widget _buildNewItemForm() {
    final template = _template;
    final theme = Theme.of(context);

    if (widget.customFormBuilder != null) {
      final sensitivities = <String, SensitivityLevel>{
        for (final entry in template.entries)
          entry.key: entry.value.sensitivity,
      };
      return widget.customFormBuilder!(
        context,
        theme,
        _editControllers,
        'add',
        _saveNewItem,
        _cancelAddItem,
        sensitivities,
      );
    }

    final hasTitleField = template.containsKey(widget.titlePropertyKey);
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Title input (only when template defines a title property)
          if (hasTitleField) ...[
            _buildTitleField(),
            const SizedBox(height: 12),
          ],
          // Property inputs (skip the title property — already shown above)
          ...template.keys.where((k) => k != widget.titlePropertyKey).map((key) {
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
    final template = _template;
    final theme = Theme.of(context);

    if (widget.customFormBuilder != null) {
      final sensitivities = <String, SensitivityLevel>{
        for (final entry in template.entries)
          entry.key: entry.value.sensitivity,
      };
      return widget.customFormBuilder!(
        context,
        theme,
        _editControllers,
        'edit',
        () => _saveEditItem(item.id),
        _cancelEditItem,
        sensitivities,
      );
    }

    final hasTitleField = item.properties.containsKey(widget.titlePropertyKey);
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Title input (only when item has a title property)
          if (hasTitleField) ...[
            _buildTitleField(),
            const SizedBox(height: 12),
          ],
          // Property inputs (skip the title property — already shown above)
          ...item.properties.keys.where((k) => k != widget.titlePropertyKey).map((key) {
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
          Text(_formatLabel(key)),
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
                labelText: _formatLabel(key),
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
              valueListenable: controller ?? ObjectCard._dummyController,
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
            valueListenable: controller ?? ObjectCard._dummyController,
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
      // Internal/system fields that users should not see
      const internalKeys = {
        'TypeId',
        'IconName',
        'ParentId',
        'ChildrenIds',
        'IsDeleted',
        'Id',
        'CreatedAt',
        'UpdatedAt',
        'DeletedAt',
      };

      final buffer = StringBuffer();
      if (item.name.isNotEmpty) {
        buffer.writeln('${item.name}:');
      }
      for (final entry in item.properties.entries) {
        if (internalKeys.contains(entry.key)) continue;
        final value = _propValueToString(entry.value);
        if (value.isEmpty) continue;
        buffer.writeln('  ${entry.key}: $value');
      }
      final text = buffer.toString().trimRight();

      if (widget.onCopyAll != null) {
        widget.onCopyAll!(item, text);
      } else {
        Clipboard.setData(ClipboardData(text: text));
        showOverlaySnackBar(
          context,
          content: 'Copied to clipboard',
          type: SnackBarType.success,
        );
      }
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

/// camelCase / snake_case → Title Case (e.g. "givenName" → "Given Name", "visa_type" → "Visa Type")
String _formatLabel(String key) => formatFieldLabel(key);

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

/// Get the display title for an item.
/// Uses [nameExtractor] if provided, otherwise falls back to [titlePropertyKey]
/// and finally [item.name].
String _objectItemDisplayTitle(
  UnifiedObject item, {
  String Function(Map<String, String>)? nameExtractor,
  String titlePropertyKey = 'Title',
}) {
  if (nameExtractor != null) {
    final props = <String, String>{
      for (final entry in item.properties.entries)
        entry.key: _propValueToString(entry.value),
    };
    final extracted = nameExtractor(props);
    if (extracted.isNotEmpty && extracted != 'Untitled') {
      return extracted;
    }
  }
  final titleProp = item.properties[titlePropertyKey];
  if (titleProp is TextProperty && titleProp.text.isNotEmpty) {
    return titleProp.text;
  }
  // Legacy fallback for old 'Item Name' property
  final oldNameProp = item.properties['Item Name'];
  if (oldNameProp is TextProperty && oldNameProp.text.isNotEmpty) {
    return oldNameProp.text;
  }
  return item.name;
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
  final bool showEditActions;
  final bool showAddButton;
  final bool showEditSection;

  const _ObjectCardHeader({
    required this.object,
    required this.onChangeIcon,
    required this.onEdit,
    required this.onDelete,
    required this.onAddItem,
    required this.showEditActions,
    required this.showAddButton,
    this.showEditSection = false,
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
        if (showEditActions) ...[
          // When showEditSection is true, the Add button slot becomes Edit Section,
          // so hide the redundant Edit button here to avoid two edit icons.
          if (!showEditSection)
            IconButton(
              icon: const Icon(Icons.edit_outlined, size: 18),
              onPressed: onEdit,
              tooltip: 'Edit',
              padding: EdgeInsets.zero,
              constraints: const BoxConstraints(minWidth: 32, minHeight: 32),
              visualDensity: VisualDensity.compact,
            ),
          if (!showEditSection) const SizedBox(width: 8),
          IconButton(
            icon: const Icon(Icons.delete_outline, size: 18),
            onPressed: onDelete,
            tooltip: 'Delete',
            padding: EdgeInsets.zero,
            constraints: const BoxConstraints(minWidth: 32, minHeight: 32),
            visualDensity: VisualDensity.compact,
          ),
          const SizedBox(width: 8),
        ],
        if (showAddButton)
          if (showEditSection)
            IconButton(
              icon: const Icon(Icons.edit_note, size: 18),
              onPressed: onEdit,
              tooltip: 'Edit Section',
              padding: EdgeInsets.zero,
              constraints: const BoxConstraints(minWidth: 32, minHeight: 32),
              visualDensity: VisualDensity.compact,
            )
          else
            IconButton(
              icon: const Icon(Icons.add, size: 18),
              onPressed: onAddItem,
              tooltip: 'Add Item',
              padding: EdgeInsets.zero,
              constraints: const BoxConstraints(minWidth: 32, minHeight: 32),
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
  final String titlePropertyKey;

  const _ObjectCardPropertiesList({
    required this.item,
    this.titlePropertyKey = 'Title',
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: item.properties.entries
          .where((e) => e.key != titlePropertyKey)
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
                  _wrapEveryNChars(_formatLabel(entry.key), 12),
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

// =============================================================================
// _ObjectCardItemTile — Per-item tile with fine-grained fieldHistory select
// =============================================================================

class _ObjectCardItemTile extends ConsumerWidget {
  final UnifiedObject item;
  final bool isHistoryExpanded;
  final VoidCallback onToggleHistory;
  final VoidCallback onCopy;
  final VoidCallback onStartEdit;
  final VoidCallback onDelete;
  final String historyFieldIdPrefix;
  final String Function(Map<String, String>)? nameExtractor;
  final String titlePropertyKey;

  const _ObjectCardItemTile({
    required this.item,
    required this.isHistoryExpanded,
    required this.onToggleHistory,
    required this.onCopy,
    required this.onStartEdit,
    required this.onDelete,
    required this.historyFieldIdPrefix,
    this.nameExtractor,
    this.titlePropertyKey = 'Title',
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final history = ref.watch(
      fieldHistoriesProvider.select((h) => h.getHistory(item.id, historyFieldIdPrefix)),
    );
    final count = history?.entries.length ?? 0;
    final hasHist = count > 0;

    // Check if any property requires sensitive verification for history
    final requiresVerification = item.properties.values.any(
      (p) => p.sensitivity == SensitivityLevel.sensitive ||
             p.sensitivity == SensitivityLevel.critical,
    );

    final iconData = isHistoryExpanded ? Icons.history_toggle_off : Icons.history;
    final iconColor = hasHist
        ? theme.colorScheme.onSurfaceVariant
        : theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.4);
    final historyIcon = Icon(iconData, size: 20, color: iconColor);

    Future<void> handleHistoryPress() async {
      if (hasHist) {
        if (requiresVerification) {
          final isGranted = ref.read(isSensitiveAccessGrantedProvider);
          if (!isGranted) {
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
          }
        }
        onToggleHistory();
      } else {
        if (context.mounted) {
          showOverlaySnackBar(
            context,
            content: 'No history available',
            type: SnackBarType.info,
          );
        }
      }
    }

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
                      _objectItemDisplayTitle(
                        item,
                        nameExtractor: nameExtractor,
                        titlePropertyKey: titlePropertyKey,
                      ),
                      style: theme.textTheme.bodyLarge?.copyWith(
                        fontWeight: FontWeight.w500,
                      ),
                    ),
                    const SizedBox(height: 4),
                    _ObjectCardPropertiesList(
                      item: item,
                      titlePropertyKey: titlePropertyKey,
                    ),
                  ],
                ),
              ),
              // Action buttons: Copy | Edit | History | Delete
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  IconButton(
                    icon: const Icon(Icons.copy_all, size: 20),
                    tooltip: 'Copy',
                    onPressed: onCopy,
                    visualDensity: VisualDensity.compact,
                  ),
                  IconButton(
                    icon: const Icon(Icons.edit_outlined, size: 20),
                    tooltip: 'Edit',
                    onPressed: onStartEdit,
                    visualDensity: VisualDensity.compact,
                  ),
                  IconButton(
                    icon: hasHist
                        ? Stack(
                            clipBehavior: Clip.none,
                            children: [
                              historyIcon,
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
                          )
                        : Stack(
                            clipBehavior: Clip.none,
                            children: [
                              historyIcon,
                              Positioned(
                                right: -6,
                                top: -6,
                                child: Text(
                                  '0',
                                  style: TextStyle(
                                    fontSize: 10,
                                    color: iconColor,
                                    fontWeight: FontWeight.w500,
                                    height: 1,
                                  ),
                                ),
                              ),
                            ],
                          ),
                    tooltip: hasHist ? 'History ($count)' : 'No history yet',
                    onPressed: handleHistoryPress,
                    visualDensity: VisualDensity.compact,
                  ),
                  const SizedBox(width: 8),
                  IconButton(
                    icon: const Icon(
                      Icons.delete_outline,
                      size: 20,
                    ),
                    tooltip: 'Delete',
                    onPressed: onDelete,
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
}
