import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/core/services/field_history_service.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/icon_picker_sheet.dart';


/// Generic editor for creating or editing any UnifiedObject.
class ObjectEditorPage extends ConsumerStatefulWidget {
  final String? objectId;
  final String? parentId;

  const ObjectEditorPage({
    super.key,
    this.objectId,
    this.parentId,
  });

  @override
  ConsumerState<ObjectEditorPage> createState() => _ObjectEditorPageState();
}

/// Editable property row for Item Properties editor.
class _PropertyField {
  String key;
  String type;
  _PropertyField({required this.key, this.type = 'text'});
}

class _ObjectEditorPageState extends ConsumerState<ObjectEditorPage> {
  late final TextEditingController _nameController;
  late final TextEditingController _iconController;
  String? _selectedTypeId;
  String? _selectedParentId;
  final List<_PropertyField> _propertyFields = [];

  bool get _isEditing => widget.objectId != null;
  UnifiedObject? _existingObject;

  @override
  void initState() {
    super.initState();
    _nameController = TextEditingController();
    _iconController = TextEditingController(text: 'folder');
    _selectedParentId = widget.parentId;

    if (_isEditing) {
      _loadExistingObject();
    } else {
      _selectedTypeId = 'note';
      _initPropertiesFromType('note');
    }
  }

  void _loadExistingObject() {
    final object = ref.read(objectByIdProvider(widget.objectId!));
    if (object == null) return;

    _existingObject = object;
    _nameController.text = object.name;
    _iconController.text = object.iconName;
    _selectedTypeId = object.typeId;
    _selectedParentId = object.parentId;
    _propertyFields.clear();
    for (final entry in object.properties.entries) {
      _propertyFields.add(_PropertyField(key: entry.key, type: 'text'));
    }
  }

  void _initPropertiesFromType(String typeId) {
    final type = ObjectTypeRegistry.getType(typeId);
    if (type == null) return;

    _propertyFields.clear();
    for (final propDef in type.properties) {
      _propertyFields.add(_PropertyField(key: propDef.id, type: 'text'));
    }
  }

  @override
  void dispose() {
    _nameController.dispose();
    _iconController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(
        title: Text(_isEditing ? 'Edit Section' : 'New Section'),
      ),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Icon + Name
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text('Icon', style: theme.textTheme.titleMedium),
                    const SizedBox(height: 12),
                    InkWell(
                      onTap: () async {
                        final result = await showModalBottomSheet<String>(
                          context: context,
                          builder: (ctx) => IconPickerSheet(
                            currentIcon: _iconController.text.isEmpty
                                ? 'folder'
                                : _iconController.text,
                          ),
                        );
                        if (result != null) {
                          setState(() {
                            _iconController.text = result;
                          });
                        }
                      },
                      borderRadius: BorderRadius.circular(12),
                      child: Container(
                        width: 56,
                        height: 56,
                        decoration: BoxDecoration(
                          color: theme.colorScheme.primary.withValues(alpha: 0.1),
                          borderRadius: BorderRadius.circular(12),
                        ),
                        child: Icon(
                          UnifiedObjectService.getIconFromName(_iconController.text),
                          color: theme.colorScheme.primary,
                          size: 28,
                        ),
                      ),
                    ),
                  ],
                ),
                const SizedBox(width: 16),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text('Name', style: theme.textTheme.titleMedium),
                      const SizedBox(height: 12),
                      TextField(
                        controller: _nameController,
                        decoration: const InputDecoration(
                          hintText: 'Enter section name',
                          border: OutlineInputBorder(),
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
            const SizedBox(height: 24),

            // Type — only shown when creating a new object
            if (!_isEditing) ...[
              Text('Type', style: theme.textTheme.titleMedium),
              const SizedBox(height: 12),
              _TypeDropdown(
                selectedTypeId: _selectedTypeId,
                onChanged: (value) {
                  if (value == null) return;
                  setState(() {
                    _selectedTypeId = value;
                    _initPropertiesFromType(value);
                  });
                },
              ),
              const SizedBox(height: 24),
            ],

            // Item Properties
            Card(
              margin: const EdgeInsets.only(bottom: 12),
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Text('Item Properties', style: theme.textTheme.titleMedium),
                        const Spacer(),
                        IconButton(
                          icon: const Icon(Icons.add, size: 20),
                          tooltip: 'Add Property',
                          onPressed: () {
                            setState(() {
                              _propertyFields.add(_PropertyField(key: ''));
                            });
                          },
                          visualDensity: VisualDensity.compact,
                        ),
                      ],
                    ),
                    const SizedBox(height: 12),
                    ..._propertyFields.asMap().entries.map((entry) {
                      final index = entry.key;
                      final field = entry.value;
                      return Padding(
                        padding: const EdgeInsets.only(bottom: 8),
                        child: Row(
                          children: [
                            Expanded(
                              flex: 2,
                              child: TextField(
                                controller: TextEditingController(text: field.key),
                                decoration: const InputDecoration(
                                  hintText: 'Key name',
                                  isDense: true,
                                  contentPadding: EdgeInsets.symmetric(horizontal: 10, vertical: 10),
                                  border: OutlineInputBorder(),
                                ),
                                onChanged: (value) {
                                  field.key = value;
                                },
                              ),
                            ),
                            const SizedBox(width: 8),
                            Expanded(
                              flex: 1,
                              child: InputDecorator(
                                decoration: const InputDecoration(
                                  isDense: true,
                                  contentPadding: EdgeInsets.symmetric(horizontal: 10, vertical: 10),
                                  border: OutlineInputBorder(),
                                ),
                                child: DropdownButtonHideUnderline(
                                  child: DropdownButton<String>(
                                    isDense: true,
                                    value: field.type,
                                    items: const [
                                      DropdownMenuItem(value: 'text', child: Text('Text')),
                                    ],
                                    onChanged: (value) {
                                      if (value != null) {
                                        setState(() {
                                          field.type = value;
                                        });
                                      }
                                    },
                                  ),
                                ),
                              ),
                            ),
                            const SizedBox(width: 4),
                            IconButton(
                              icon: Icon(Icons.delete_outline, size: 20, color: theme.colorScheme.error),
                              tooltip: 'Delete',
                              onPressed: () {
                                setState(() {
                                  _propertyFields.removeAt(index);
                                });
                              },
                              visualDensity: VisualDensity.compact,
                            ),
                          ],
                        ),
                      );
                    }),
                  ],
                ),
              ),
            ),

            const SizedBox(height: 32),
            Center(
              child: OutlinedButton(
                onPressed: _saveObject,
                child: const Text('Save'),
              ),
            ),
            const SizedBox(height: 16),
          ],
        ),
      ),
    );
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

  Future<void> _recordHistory() async {
    if (_existingObject == null) return;
    final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
    if (accountId == null) return;

    final allFieldValues = <String, String>{};
    for (final entry in _existingObject!.properties.entries) {
      allFieldValues[entry.key] = _propertyValueToString(entry.value);
    }

    await ref.read(fieldHistoriesProvider.notifier).recordSnapshot(
      accountId: accountId,
      itemId: _existingObject!.id,
      fieldIdPrefix: 'unified',
      allFieldValues: allFieldValues,
    );
  }

  void _saveObject() async {
    if (_nameController.text.trim().isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Name is required')),
      );
      return;
    }

    // Build properties from property fields
    final properties = <String, PropertyValue>{};
    for (final field in _propertyFields) {
      if (field.key.trim().isNotEmpty) {
        properties[field.key.trim()] = const TextProperty(text: '');
      }
    }

    final notifier = ref.read(unifiedObjectProvider.notifier);

    if (_isEditing && _existingObject != null) {
      // Record history before update
      await _recordHistory();

      // Handle parent change: if parent changed, use moveObject
      final oldParentId = _existingObject!.parentId;
      final newParentId = _selectedParentId;

      if (oldParentId != newParentId) {
        await notifier.moveObject(_existingObject!.id, newParentId);
      }

      await notifier.updateObject(
        _existingObject!.id,
        name: _nameController.text.trim(),
        typeId: _selectedTypeId,
        iconName: _iconController.text.trim(),
        properties: properties,
      );
    } else {
      await notifier.createObject(
        name: _nameController.text.trim(),
        typeId: _selectedTypeId,
        parentId: _selectedParentId,
        iconName: _iconController.text.trim(),
        properties: properties,
      );
    }

    if (mounted) {
      context.pop();
    }
  }
}


class _TypeDropdown extends ConsumerWidget {
  final String? selectedTypeId;
  final ValueChanged<String?> onChanged;

  const _TypeDropdown({
    required this.selectedTypeId,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final allTypes = ObjectTypeRegistry.getAllTypes()
        .where((t) => t.id != 'page' && t.id != 'item')
        .toList();
    final effectiveTypeId = allTypes.any((t) => t.id == selectedTypeId)
        ? selectedTypeId
        : null;

    return InputDecorator(
      decoration: const InputDecoration(
        border: OutlineInputBorder(),
        hintText: 'Select type',
      ),
      child: DropdownButtonHideUnderline(
        child: DropdownButton<String>(
          isExpanded: true,
          value: effectiveTypeId,
          hint: const Text('Select type'),
          items: allTypes.map((type) {
            return DropdownMenuItem(
              value: type.id,
              child: Row(
                children: [
                  Icon(
                    UnifiedObjectService.getIconFromName(type.iconName),
                    size: 20,
                  ),
                  const SizedBox(width: 12),
                  Text(type.name),
                ],
              ),
            );
          }).toList(),
          onChanged: onChanged,
        ),
      ),
    );
  }
}

class ObjectParentDropdown extends ConsumerWidget {
  final String? selectedParentId;
  final String? objectId;
  final ValueChanged<String?> onChanged;

  const ObjectParentDropdown({
    super.key,
    required this.selectedParentId,
    this.objectId,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final allObjects = ref.watch(unifiedObjectProvider).objects;
    final validParentIds = <String?>[null];
    for (final o in allObjects) {
      if (!o.isDeleted && o.id != objectId && o.typeId == 'page') {
        validParentIds.add(o.id);
      }
    }
    final effectiveParentId = validParentIds.contains(selectedParentId)
        ? selectedParentId
        : null;

    return InputDecorator(
      decoration: const InputDecoration(
        border: OutlineInputBorder(),
        hintText: 'No parent (root)',
      ),
      child: DropdownButtonHideUnderline(
        child: DropdownButton<String?>(
          isExpanded: true,
          value: effectiveParentId,
          hint: const Text('No parent (root)'),
          items: [
            const DropdownMenuItem(
              value: null,
              child: Text('No parent (root)'),
            ),
            ...allObjects
                .where((o) => !o.isDeleted && o.id != objectId && o.typeId == 'page')
                .map((o) {
              return DropdownMenuItem(
                value: o.id,
                child: Row(
                  children: [
                    Icon(
                      UnifiedObjectService.getIconFromName(o.iconName),
                      size: 18,
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        o.name,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                  ],
                ),
              );
            }),
          ],
          onChanged: onChanged,
        ),
      ),
    );
  }
}
