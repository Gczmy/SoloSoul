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
  final Map<String, TextEditingController> _editControllers = {};

  @override
  void dispose() {
    for (final c in _editControllers.values) {
      c.dispose();
    }
    super.dispose();
  }

  Future<void> _addItem() async {
    final template = widget.object.properties;
    final items = ref.read(childrenProvider(widget.object.id));
    final itemName = 'Item ${items.where((o) => o.typeId == 'item').length + 1}';

    await ref.read(unifiedObjectProvider.notifier).createObject(
      name: itemName,
      typeId: 'item',
      parentId: widget.object.id,
      properties: Map<String, PropertyValue>.from(template),
    );
  }

  Future<void> _deleteItem(String itemId) async {
    await ref.read(unifiedObjectProvider.notifier).deleteObject(itemId);
  }

  void _startEditingItem(UnifiedObject item) {
    setState(() {
      _editingItemId = item.id;
      _editControllers.clear();
      _editControllers['__name__'] = TextEditingController(text: item.name);
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
      for (final c in _editControllers.values) {
        c.dispose();
      }
      _editControllers.clear();
    });
  }

  Future<void> _saveEditItem(String itemId) async {
    final item = ref.read(objectByIdProvider(itemId));
    if (item == null) return;

    final name = _editControllers['__name__']?.text.trim() ?? item.name;
    final updatedProps = Map<String, PropertyValue>.from(item.properties);

    for (final key in item.properties.keys) {
      final controller = _editControllers[key];
      if (controller != null) {
        final oldValue = item.properties[key]!;
        updatedProps[key] = _parsePropertyValue(oldValue, controller.text);
      }
    }

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
      TextProperty() => TextProperty(text: newText),
      NumberProperty() => NumberProperty(value: double.tryParse(newText)),
      DateProperty() => DateProperty(isoDate: newText),
      CheckboxProperty() => CheckboxProperty(
          checked: newText.toLowerCase() == 'true' || newText == '1' || newText == 'yes',
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
            if (items.isEmpty)
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
            else
              ...items.map((item) => _buildItemTile(item)),


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
                    Text(
                      item.name,
                      style: theme.textTheme.bodyMedium?.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    const SizedBox(height: 4),
                    ...item.properties.entries.map((entry) {
                      return Padding(
                        padding: const EdgeInsets.only(left: 8, bottom: 2),
                        child: Row(
                          children: [
                            Text(
                              '${entry.key}: ',
                              style: theme.textTheme.bodySmall?.copyWith(
                                color: theme.colorScheme.onSurfaceVariant,
                              ),
                            ),
                            Expanded(
                              child: Text(
                                _propertyValueToString(entry.value),
                                style: theme.textTheme.bodySmall,
                                overflow: TextOverflow.ellipsis,
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

  Widget _buildItemEditMode(UnifiedObject item) {
    final theme = Theme.of(context);

    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Name input + action buttons
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Expanded(
                child: TextField(
                  controller: _editControllers['__name__'],
                  decoration: const InputDecoration(
                    labelText: 'Name',
                    isDense: true,
                    contentPadding: EdgeInsets.symmetric(horizontal: 10, vertical: 10),
                    border: OutlineInputBorder(),
                  ),
                  style: theme.textTheme.bodyMedium?.copyWith(fontWeight: FontWeight.w600),
                ),
              ),
              const SizedBox(width: 8),
              TextButton(
                onPressed: _cancelEditItem,
                child: const Text('Cancel'),
              ),
              FilledButton(
                onPressed: () => _saveEditItem(item.id),
                child: const Text('Save'),
              ),
            ],
          ),
          const SizedBox(height: 12),
          // Property inputs
          ...item.properties.keys.map((key) {
            return Padding(
              padding: const EdgeInsets.only(bottom: 8),
              child: _buildEditPropertyField(key, item.properties[key]!),
            );
          }),
          const Divider(height: 16),
        ],
      ),
    );
  }

  Widget _buildEditPropertyField(String key, PropertyValue value) {
    final theme = Theme.of(context);
    final controller = _editControllers[key];

    return Row(
      children: [
        Expanded(
          flex: 2,
          child: Text(
            key,
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
        ),
        const SizedBox(width: 8),
        Expanded(
          flex: 3,
          child: switch (value) {
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
              ],
            ),
            _ => TextField(
              controller: controller,
              decoration: const InputDecoration(
                isDense: true,
                contentPadding: EdgeInsets.symmetric(horizontal: 8, vertical: 6),
                border: OutlineInputBorder(),
              ),
              style: theme.textTheme.bodySmall,
              keyboardType: value is NumberProperty ? TextInputType.number : null,
            ),
          },
        ),
      ],
    );
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
