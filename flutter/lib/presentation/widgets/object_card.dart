import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/field_history_service.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/field_history_view.dart';
import 'package:solosoul_flutter/presentation/widgets/icon_picker_sheet.dart';


/// Card displaying a Section and its Items.
///
/// A Section contains multiple Items (UnifiedObject with typeId='item').
/// Each Item has its own properties (key-value pairs).
/// The Section's `properties` field defines the Item template.
class ObjectCard extends ConsumerStatefulWidget {
  final UnifiedObject object;

  const ObjectCard({super.key, required this.object});

  @override
  ConsumerState<ObjectCard> createState() => _ObjectCardState();
}

class _ObjectCardState extends ConsumerState<ObjectCard> {
  static const String _historyFieldId = 'unified';
  final Set<String> _expandedHistoryItemIds = {};
  String? _editingItemId;
  bool _isAddingItem = false;
  bool _isExpanded = false;
  final Map<String, TextEditingController> _editControllers = {};

  void _disposeControllers() {
    for (final c in _editControllers.values) {
      c.dispose();
    }
    _editControllers.clear();
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

      // Other fields start empty
      for (final key in template.keys.skip(1)) {
        _editControllers[key] = TextEditingController(text: '');
      }
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

  Future<void> _deleteItem(String itemId) async {
    await ref.read(unifiedObjectProvider.notifier).deleteObject(itemId);
  }

  void _startEditingItem(UnifiedObject item) {
    setState(() {
      _editingItemId = item.id;
      _isAddingItem = false;
      _disposeControllers();
      _editControllers['__name__'] = TextEditingController(text: _itemDisplayTitle(item));
      for (final entry in item.properties.entries) {
        _editControllers[entry.key] = TextEditingController(
          text: _propertyValueToString(entry.value),
        );
      }
    });
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
        oldValues[entry.key] = _propertyValueToString(entry.value);
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

  String _propertyValueToString(PropertyValue value) {
    return switch (value) {
      TextProperty(:final text) => text,
      NumberProperty(:final value) => value?.toString() ?? '',
      DateProperty(:final isoDate) => isoDate ?? '',
      CheckboxProperty(:final checked) => checked ? 'Yes' : 'No',
      SelectProperty(:final selectedId) => selectedId ?? '',
      _ => '',
    };
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final icon = UnifiedObjectService.getIconFromName(widget.object.iconName);
    final items = ref.watch(childrenProvider(widget.object.id))
        .where((o) => o.typeId == 'item')
        .toList();
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
            // Header: icon + name + action buttons
            Row(
              children: [
                InkWell(
                  onTap: _changeIcon,
                  borderRadius: BorderRadius.circular(6),
                  child: Padding(
                    padding: const EdgeInsets.all(4),
                    child: Icon(icon, color: theme.colorScheme.primary, size: 20),
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    widget.object.name,
                    style: theme.textTheme.titleMedium?.copyWith(
                      fontWeight: FontWeight.w600,
                    ),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                IconButton(
                  icon: const Icon(Icons.edit_outlined, size: 18),
                  onPressed: _editObject,
                  tooltip: 'Edit',
                  padding: EdgeInsets.zero,
                  constraints: const BoxConstraints(minWidth: 28, minHeight: 28),
                  visualDensity: VisualDensity.compact,
                ),
                IconButton(
                  icon: const Icon(Icons.delete_outline, size: 18),
                  onPressed: _deleteObject,
                  tooltip: 'Delete',
                  padding: EdgeInsets.zero,
                  constraints: const BoxConstraints(minWidth: 28, minHeight: 28),
                  visualDensity: VisualDensity.compact,
                ),
                IconButton(
                  icon: const Icon(Icons.add, size: 18),
                  onPressed: _addItem,
                  tooltip: 'Add Item',
                  padding: EdgeInsets.zero,
                  constraints: const BoxConstraints(minWidth: 28, minHeight: 28),
                  visualDensity: VisualDensity.compact,
                ),
              ],
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
    return isEditing ? _buildItemEditMode(item) : _buildItemViewMode(item);
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

  Widget _buildItemViewMode(UnifiedObject item) {
    final theme = Theme.of(context);
    final isHistoryExpanded = _expandedHistoryItemIds.contains(item.id);
    final hasHistory = ref.watch(fieldHistoriesProvider).getHistory(item.id, _historyFieldId)?.entries.isNotEmpty == true;

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
                    ...item.properties.entries.where((e) => e.key != 'Title').map((entry) {
                      return Padding(
                        padding: const EdgeInsets.only(left: 8, bottom: 2),
                        child: Row(
                          children: [
                            SelectableText(
                              '${entry.key}: ',
                              style: theme.textTheme.bodySmall?.copyWith(
                                color: theme.colorScheme.onSurfaceVariant,
                              ),
                            ),
                            Expanded(
                              child: SelectableText(
                                _propertyValueToString(entry.value),
                                style: theme.textTheme.bodySmall,
                              ),
                            ),
                          ],
                        ),
                      );
                    }),
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
            Consumer(
              builder: (context, ref, child) {
                final histories = ref.watch(fieldHistoriesProvider);
                final history = histories.getHistory(item.id, _historyFieldId);
                return FieldHistoryView(
                  fieldName: _historyFieldId,
                  history: history,
                  initiallyExpanded: true,
                );
              },
            ),
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
          TextField(
            controller: _editControllers['__name__'],
            decoration: const InputDecoration(
              labelText: 'Title',
              border: OutlineInputBorder(),
            ),
          ),
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
                onPressed: _saveNewItem,
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
          TextField(
            controller: _editControllers['__name__'],
            decoration: const InputDecoration(
              labelText: 'Title',
              border: OutlineInputBorder(),
            ),
          ),
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
                onPressed: () => _saveEditItem(item.id),
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
        ],
      ),
      _ => TextField(
        controller: controller,
        decoration: InputDecoration(
          labelText: key,
          border: const OutlineInputBorder(),
        ),
        keyboardType: value is NumberProperty ? TextInputType.number : null,
      ),
    };
  }

  void _copyItem(UnifiedObject item) {
    final buffer = StringBuffer();
    buffer.writeln('${item.name}:');
    for (final entry in item.properties.entries) {
      buffer.writeln('  ${entry.key}: ${_propertyValueToString(entry.value)}');
    }
    Clipboard.setData(ClipboardData(text: buffer.toString()));
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(content: Text('Copied to clipboard')),
    );
  }
}
