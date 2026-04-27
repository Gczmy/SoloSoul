import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/icon_picker_sheet.dart';
import 'package:solosoul_flutter/presentation/widgets/property_editor_factory.dart';

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

class _ObjectEditorPageState extends ConsumerState<ObjectEditorPage> {
  late final TextEditingController _nameController;
  late final TextEditingController _iconController;
  String? _selectedTypeId;
  String? _selectedParentId;
  Map<String, PropertyValue> _properties = {};

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
    _properties = Map.from(object.properties);
  }

  void _initPropertiesFromType(String typeId) {
    final type = ObjectTypeRegistry.getType(typeId);
    if (type == null) return;

    final newProperties = <String, PropertyValue>{};
    for (final propDef in type.properties) {
      newProperties[propDef.id] = _defaultValueForType(propDef);
    }
    _properties = newProperties;
  }

  PropertyValue _defaultValueForType(PropertyDefinition def) {
    return switch (def.type) {
      PropertyType.text => const TextProperty(text: ''),
      PropertyType.number => const NumberProperty(),
      PropertyType.date => const DateProperty(),
      PropertyType.checkbox => const CheckboxProperty(),
      PropertyType.select => SelectProperty(
          options: _parseOptions(def.config),
        ),
      PropertyType.multiSelect => MultiSelectProperty(
          options: _parseOptions(def.config),
        ),
      PropertyType.relation => const RelationProperty(),
      PropertyType.url => const UrlProperty(),
    };
  }

  List<SelectOption> _parseOptions(dynamic config) {
    if (config is! Map) return [];
    final optionsRaw = config['options'] as List<dynamic>? ?? [];
    return optionsRaw.map((e) {
      final m = e as Map<String, dynamic>;
      return SelectOption(
        id: m['id'] as String? ?? _generateTempId(),
        label: m['label'] as String? ?? '',
        order: (m['order'] as num?)?.toInt() ?? 0,
      );
    }).toList();
  }

  String _generateTempId() {
    return DateTime.now().millisecondsSinceEpoch.toString();
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
        title: Text(_isEditing ? 'Edit Page' : 'New Page'),
      ),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Name
            Text('Name', style: theme.textTheme.titleMedium),
            const SizedBox(height: 12),
            TextField(
              controller: _nameController,
              decoration: const InputDecoration(
                hintText: 'Enter object name',
                border: OutlineInputBorder(),
              ),
            ),
            const SizedBox(height: 24),

            // Icon
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

            // Parent
            Text('Parent', style: theme.textTheme.titleMedium),
            const SizedBox(height: 12),
            _ParentDropdown(
              selectedParentId: _selectedParentId,
              objectId: widget.objectId,
              onChanged: (value) {
                setState(() {
                  _selectedParentId = value;
                });
              },
            ),
            const SizedBox(height: 24),

            // Properties
            if (_properties.isNotEmpty) ...[
              Text('Properties', style: theme.textTheme.titleMedium),
              const SizedBox(height: 12),
              ..._properties.entries.map((entry) {
                return Card(
                  margin: const EdgeInsets.only(bottom: 12),
                  child: Padding(
                    padding: const EdgeInsets.all(16),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          entry.key,
                          style: theme.textTheme.titleSmall,
                        ),
                        const SizedBox(height: 8),
                        PropertyEditorFactory.buildEditor(
                          property: entry.value,
                          onChanged: (updated) {
                            setState(() {
                              _properties[entry.key] = updated;
                            });
                          },
                        ) ?? const SizedBox(),
                      ],
                    ),
                  ),
                );
              }),
            ],

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

  void _saveObject() async {
    if (_nameController.text.trim().isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Name is required')),
      );
      return;
    }

    final notifier = ref.read(unifiedObjectProvider.notifier);

    if (_isEditing && _existingObject != null) {
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
        properties: _properties,
      );
    } else {
      await notifier.createObject(
        name: _nameController.text.trim(),
        typeId: _selectedTypeId,
        parentId: _selectedParentId,
        iconName: _iconController.text.trim(),
        properties: _properties,
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
    final allTypes = ObjectTypeRegistry.getAllTypes();
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

class _ParentDropdown extends ConsumerWidget {
  final String? selectedParentId;
  final String? objectId;
  final ValueChanged<String?> onChanged;

  const _ParentDropdown({
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
