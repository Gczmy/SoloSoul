import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:solosoul_flutter/presentation/utils/property_value_utils.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/field_history_service.dart';
import 'package:solosoul_flutter/core/services/operation_notification.dart';
import 'package:solosoul_flutter/core/services/operation_logger.dart';
import 'package:solosoul_flutter/presentation/models/operation_log_models.dart';
import 'package:solosoul_flutter/presentation/providers/operation_log_provider.dart';
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';

import 'package:solosoul_flutter/presentation/widgets/icon_picker_sheet.dart';
import 'package:solosoul_flutter/presentation/widgets/object_card/object_card_header.dart';
import 'package:solosoul_flutter/presentation/widgets/object_card/object_card_item_tile_widget.dart';
import 'package:solosoul_flutter/presentation/widgets/object_card/object_card_new_item_form.dart';
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    show showOverlaySnackBar, SnackBarType;
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';


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

class _ObjectCardState extends ConsumerState<ObjectCard> {
  final Set<String> _expandedHistoryItemIds = {};
  String? _editingItemId;
  bool _isAddingItem = false;
  bool _isExpanded = false;
  final Map<String, TextEditingController> _editControllers = {};
  /// Tracks original values when entering edit/add mode for dirty-check.
  final Map<String, String> _originalValues = {};
  bool _hasChanges = false;
  bool _showDeprecated = false;

  /// Resolved template: [itemTemplate] takes precedence over [object.properties].
  Map<String, PropertyValue> get _template =>
      widget.itemTemplate ?? widget.object.properties;

  /// Keys in the currently-editing item that are NOT in the template (deprecated).
  List<String> _deprecatedKeysFor(UnifiedObject item) {
    return item.properties.keys
        .where((k) => !_template.containsKey(k))
        .toList();
  }

  void _disposeControllers() {
    for (final c in _editControllers.values) {
      c.removeListener(_checkForChanges);
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
    if (_hasChanges != changed && mounted) {
      // 延迟到下一帧 setState，避免在祖先 widget 的 build 阶段直接调用
      //（例如父级在 build 中初始化 TextEditingController.text 时会触发 listener）
      SchedulerBinding.instance.addPostFrameCallback((_) {
        if (mounted) {
          setState(() => _hasChanges = changed);
        }
      });
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
          entry.key: propValueToString(entry.value),
      };
      return widget.nameExtractor!(props);
    }
    return objectItemDisplayTitle(item);
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
    final l10n = AppLocalizations.of(context);
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
          entry.key: propValueToString(entry.value),
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
        description: l10n.operationLogCreatedItem(name),
        descriptionKey: 'createdUnifiedItem',
        descriptionArgs: {'name': name},
      ),
    );
    unawaited(ref.read(authNotifierProvider.notifier).updateOperation('Created item'));

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
    final l10n = AppLocalizations.of(context);
    final item = ref.read(objectByIdProvider(itemId));
    if (item == null) return;

    final itemName = _itemDisplayTitle(item);
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) {
        final theme = Theme.of(context);
        return AlertDialog(
          title: Text(l10n.dialogDeleteItem),
          content: Text(l10n.dialogDeleteItemConfirm(itemName)),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(context).pop(false),
              child: Text(l10n.commonCancel),
            ),
            TextButton(
              onPressed: () => Navigator.of(context).pop(true),
              child: Text(l10n.commonDelete, style: TextStyle(color: theme.colorScheme.error)),
            ),
          ],
        );
      },
    );
    if (confirmed != true) return;

    Future<void> doDelete() async {
      final l10n = AppLocalizations.of(context);
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
          description: l10n.operationLogDeletedItem(_itemDisplayTitle(item)),
          fieldPath: itemId,
          descriptionKey: 'deletedUnifiedItem',
          descriptionArgs: {'name': _itemDisplayTitle(item)},
        ),
      );
      unawaited(ref.read(authNotifierProvider.notifier).updateOperation('Deleted item'));

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
        _showDeprecated = false;
        _disposeControllers();
        _editControllers['__name__'] = TextEditingController(text: _itemDisplayTitle(item));
        _originalValues['__name__'] = _itemDisplayTitle(item);

        // Only create controllers for active (template) keys. Keys that
        // exist in the item but not in the template are deprecated —
        // they are hidden behind a "Show Deprecated" toggle and
        // preserved in item.properties on save.
        for (final key in _template.keys) {
          final value = item.properties[key];
          final textValue = value != null
              ? propValueToString(value)
              : propValueToString(_template[key]!);
          _editControllers[key] = TextEditingController(text: textValue);
          _originalValues[key] = textValue;
        }
        // Also create controllers for deprecated keys (display-only).
        for (final key in item.properties.keys) {
          if (_template.containsKey(key)) continue;
          final value = item.properties[key]!;
          _editControllers[key] =
              TextEditingController(text: propValueToString(value));
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
      _showDeprecated = false;
      _disposeControllers();
    });
  }

  Future<void> _saveEditItem(String itemId) async {
    final l10n = AppLocalizations.of(context);
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

    // Iterate all controllers (template + item merged keys) so new schema
    // properties added after item creation are included in the save.
    for (final entry in _editControllers.entries) {
      final key = entry.key;
      if (key == '__name__' || key == titleKey) continue;
      final controller = entry.value;
      final oldValue = item.properties[key] ?? _template[key];
      if (oldValue != null) {
        updatedProps[key] = _parsePropertyValue(oldValue, controller.text);
      }
    }

    // Compute display name using extractor or title property
    final String name;
    if (widget.nameExtractor != null) {
      final props = <String, String>{
        for (final entry in updatedProps.entries)
          entry.key: propValueToString(entry.value),
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
        oldValues[entry.key] = propValueToString(entry.value);
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
        description: l10n.operationLogUpdatedItem(name),
        fieldPath: itemId,
        descriptionKey: 'updatedUnifiedItem',
        descriptionArgs: {'name': name},
      ),
    );
    unawaited(ref.read(authNotifierProvider.notifier).updateOperation('Updated item'));

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
          checked: newText.toLowerCase() == 'true' || newText.toLowerCase() == 'yes' || newText == '1',
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
    final l10n = AppLocalizations.of(context);
    // Capture overlay before any async gap — after deleteObject the section's
    // ObjectCard may be removed from the tree, making context unmounted.
    final overlay = Overlay.of(context);
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(l10n.dialogDeleteSection),
        content: Text(
          l10n.dialogDeleteItemConfirm(widget.object.name),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: Text(l10n.commonCancel),
          ),
          TextButton(
            onPressed: () => Navigator.pop(context, true),
            style: TextButton.styleFrom(foregroundColor: Colors.red),
            child: Text(l10n.commonDelete),
          ),
        ],
      ),
    );

    if (confirmed == true) {
      final sectionId = widget.object.id;
      final notifier = ref.read(unifiedObjectProvider.notifier);
      await notifier.deleteObject(sectionId);
      showOverlaySnackBar(
        null,
        forOverlay: overlay,
        content: l10n.workspaceSectionDeleted,
        duration: const Duration(seconds: 4),
        type: SnackBarType.warning,
        actionLabel: l10n.commonUndo,
        onAction: () {
          notifier.restoreObject(sectionId);
        },
      );
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
    final l10n = AppLocalizations.of(context);
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
            ObjectCardHeader(
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
                          ? l10n.pageEditorEditSectionTitle
                          : l10n.commonAddItem,
                    ),
                  ),
                ),
              )
            else ...[
              if (_isAddingItem)
                ObjectCardNewItemForm(
                  template: _template,
                  editControllers: _editControllers,
                  hasChanges: _hasChanges,
                  onSave: _saveNewItem,
                  onCancel: _cancelAddItem,
                  customFormBuilder: widget.customFormBuilder,
                  titlePropertyKey: widget.titlePropertyKey,
                  onCheckboxChanged: (key, value) {
                    setState(() {
                      _editControllers[key]?.text = (value ?? false) ? 'Yes' : 'No';
                    });
                  },
                ),
              ...visibleItems.map((item) => ObjectCardItemTileWidget(
                item: item,
                isEditing: _editingItemId == item.id,
                isHistoryExpanded: _expandedHistoryItemIds.contains(item.id),
                displayItemBuilder: widget.displayItemBuilder,
                onEdit: () => _startEditingItem(item),
                onDelete: () => _deleteItem(item.id),
                onCopy: () => _copyItem(item),
                onToggleHistory: () => _toggleItemHistory(item.id),
                onStartEdit: () => _startEditingItem(item),
                historyFieldIdPrefix: widget.historyFieldIdPrefix,
                nameExtractor: widget.nameExtractor,
                titlePropertyKey: widget.titlePropertyKey,
                template: _template,
                editControllers: _editControllers,
                hasChanges: _hasChanges,
                onSaveEditItem: () => _saveEditItem(item.id),
                onCancelEditItem: _cancelEditItem,
                customFormBuilder: widget.customFormBuilder,
                onCheckboxChanged: (key, value) {
                  setState(() {
                    _editControllers[key]?.text = (value ?? false) ? 'Yes' : 'No';
                  });
                },
                showDeprecated: _editingItemId == item.id && _showDeprecated,
                deprecatedKeys: _editingItemId == item.id
                    ? _deprecatedKeysFor(item)
                    : const [],
                onToggleDeprecated: () =>
                    setState(() => _showDeprecated = !_showDeprecated),
              )),
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
                              ? l10n.commonShowLess
                              : l10n.commonShowMore(items.length - 3),
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

  void _toggleItemHistory(String itemId) {
    setState(() {
      if (_expandedHistoryItemIds.contains(itemId)) {
        _expandedHistoryItemIds.remove(itemId);
      } else {
        _expandedHistoryItemIds.add(itemId);
      }
    });
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
        final value = propValueToString(entry.value);
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
          content: AppLocalizations.of(context).commonCopiedToClipboard,
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

