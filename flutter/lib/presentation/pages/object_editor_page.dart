import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/core/services/field_history_service.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/icon_picker_sheet.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart' show AppTheme;
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';
import 'package:solosoul_flutter/core/models/section_template.dart';
import 'package:solosoul_flutter/presentation/pages/section_template_page.dart';


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
  bool? isDefaultName;
  SensitivityLevel sensitivity;
  final TextEditingController controller;

  _PropertyField({
    this.key = '',
    this.type = 'text',
    this.isDefaultName = false,
    this.sensitivity = SensitivityLevel.public,
  }) : controller = TextEditingController(text: key);
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
    bool hasDefaultName = false;
    for (final entry in object.properties.entries) {
      final sensitivity = entry.value.sensitivity;
      if (entry.key == 'Title' || entry.key == 'Item Name') {
        _propertyFields.add(_PropertyField(
          key: 'Title',
          type: 'text',
          isDefaultName: true,
          sensitivity: sensitivity,
        ));
        hasDefaultName = true;
      } else {
        _propertyFields.add(_PropertyField(
          key: entry.key,
          type: _inferTypeFromValue(entry.value),
          sensitivity: sensitivity,
        ));
      }
    }
    // 补全 Schema 中缺失的字段（兼容导入时只写入部分字段的历史数据）
    final type = ObjectTypeRegistry.getType(object.typeId ?? '');
    if (type != null) {
      final existingKeys = _propertyFields.map((f) => f.key).toSet();
      for (final propDef in type.properties) {
        if (propDef.id == 'Title' || propDef.id == 'Item Name') continue;
        if (!existingKeys.contains(propDef.id)) {
          _propertyFields.add(_PropertyField(
            key: propDef.id,
            type: propDef.type.name,
            sensitivity: SensitivityLevel.public,
          ));
        }
      }
    }

    if (!hasDefaultName) {
      _propertyFields.insert(0, _PropertyField(
        key: 'Title',
        type: 'text',
        isDefaultName: true,
      ));
    }
  }

  void _initPropertiesFromType(String typeId) {
    final type = ObjectTypeRegistry.getType(typeId);
    if (type == null) return;

    _propertyFields.clear();
    _propertyFields.add(_PropertyField(key: 'Title', type: 'text', isDefaultName: true));
    for (final propDef in type.properties) {
      if (propDef.id != 'Title' && propDef.id != 'Item Name') {
        _propertyFields.add(_PropertyField(
          key: propDef.id,
          type: propDef.type.name,
        ));
      }
    }
  }

  /// 从现有 PropertyValue 推断其字符串类型标识。
  static String _inferTypeFromValue(PropertyValue value) {
    return switch (value) {
      TextProperty() => 'text',
      NumberProperty() => 'number',
      DateProperty() => 'date',
      CheckboxProperty() => 'checkbox',
      SelectProperty() => 'select',
      MultiSelectProperty() => 'multiSelect',
      _ => 'text',
    };
  }

  /// 根据类型字符串创建空的 PropertyValue（用于 Section Schema 定义）。
  static PropertyValue _createEmptyPropertyValue(String type, SensitivityLevel sensitivity) {
    return switch (type) {
      'date' => DateProperty(isoDate: null, sensitivity: sensitivity),
      'number' => NumberProperty(value: null, sensitivity: sensitivity),
      'checkbox' => CheckboxProperty(checked: false, sensitivity: sensitivity),
      'select' => SelectProperty(options: [], selectedId: null, sensitivity: sensitivity),
      'multiSelect' => MultiSelectProperty(options: [], selectedIds: [], sensitivity: sensitivity),
      _ => TextProperty(text: '', sensitivity: sensitivity),
    };
  }

  @override
  void dispose() {
    _nameController.dispose();
    _iconController.dispose();
    for (final field in _propertyFields) {
      field.controller.dispose();
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Scaffold(
      appBar: SoloGlassAppBar(
        title: Text(_isEditing ? AppLocalizations.of(context).objectEditorEditSection : AppLocalizations.of(context).objectEditorNewSection),
      ),
      body: Stack(
        children: [
          SingleChildScrollView(
            padding: AppTheme.kPagePadding.copyWith(bottom: 96),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                _ObjectEditorHeader(
                  theme: theme,
                  iconController: _iconController,
                  nameController: _nameController,
                  onIconChanged: (icon) => setState(() => _iconController.text = icon),
                ),
                const SizedBox(height: 24),

                // Type — only shown when creating a new object
                if (!_isEditing) ...[
                  Text(AppLocalizations.of(context).objectEditorType, style: theme.textTheme.titleMedium),
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

                _PropertyFieldsSection(
                  fields: _propertyFields,
                  onAdd: () => setState(() => _propertyFields.add(_PropertyField(key: '', type: 'text'))),
                  onDeleteConfirmed: (index) => setState(() {
                    final removed = _propertyFields.removeAt(index);
                    removed.controller.dispose();
                  }),
                  onFieldChanged: () => setState(() {}),
                ),
                const SizedBox(height: 24),

                // 模板入口
                const SizedBox(height: 16),
                OutlinedButton.icon(
                  onPressed: () async {
                    final template = await Navigator.of(context).push<SectionTemplate>(
                      MaterialPageRoute(
                        builder: (context) => const SectionTemplatePage(),
                      ),
                    );
                    if (template != null) {
                      setState(() {
                        for (final field in template.fields) {
                          final existingKeys = _propertyFields.map((f) => f.key).toSet();
                          if (!existingKeys.contains(field.key)) {
                            _propertyFields.add(_PropertyField(
                              key: field.key,
                              type: field.type,
                              sensitivity: field.sensitivity,
                            ));
                          }
                        }
                      });
                    }
                  },
                  icon: const Icon(Icons.add_box_outlined, size: 18),
                  label: Text(AppLocalizations.of(context).sectionTemplateSelectButton),
                  style: OutlinedButton.styleFrom(
                    padding: const EdgeInsets.symmetric(vertical: 14),
                  ),
                ),
                const SizedBox(height: 24),
              ],
            ),
          ),
          _BottomSaveBar(onSave: _saveObject),
        ],
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
    try {
    if (_nameController.text.trim().isEmpty) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(AppLocalizations.of(context).objectEditorNameRequired)),
        );
      }
      return;
    }

    // Check for duplicate property keys
    final keyCounts = <String, int>{};
    for (final field in _propertyFields) {
      final key = field.isDefaultName == true ? AppLocalizations.of(context).objectEditorDefaultFieldTitle : field.controller.text.trim();
      if (key.isNotEmpty) {
        keyCounts[key] = (keyCounts[key] ?? 0) + 1;
      }
    }
    final duplicates = keyCounts.entries.where((e) => e.value > 1).map((e) => e.key).toList();
    if (duplicates.isNotEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(AppLocalizations.of(context).objectEditorDuplicateProperties(duplicates.join(', ')))),
      );
      return;
    }

    // Build properties from property fields
    final properties = <String, PropertyValue>{};
    for (final field in _propertyFields) {
      final key = field.isDefaultName == true && field.key.trim().isEmpty
          ? AppLocalizations.of(context).objectEditorDefaultFieldItemName
          : field.key.trim();
      if (key.isNotEmpty) {
        if (field.isDefaultName == true) {
          properties['Title'] = const TextProperty(
            text: '',
            sensitivity: SensitivityLevel.public,
          );
        } else {
          properties[key] = _createEmptyPropertyValue(field.type, field.sensitivity);
        }
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
    } on Exception catch (e, st) {
      DebugLogger.instance.logError('OBJECT_EDITOR', 'Failed to save object: $e\n$st');
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(AppLocalizations.of(context).objectEditorSaveFailed(e.toString()))),
        );
      }
    }
  }

}


/// Icon picker + section name input row.
class _ObjectEditorHeader extends StatelessWidget {
  final ThemeData theme;
  final TextEditingController iconController;
  final TextEditingController nameController;
  final ValueChanged<String> onIconChanged;

  const _ObjectEditorHeader({
    required this.theme,
    required this.iconController,
    required this.nameController,
    required this.onIconChanged,
  });

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Text(AppLocalizations.of(context).objectEditorIcon, style: theme.textTheme.titleMedium),
            const SizedBox(width: 16 + 56 + 16),
            Expanded(
              child: Text(AppLocalizations.of(context).objectEditorName, style: theme.textTheme.titleMedium),
            ),
          ],
        ),
        const SizedBox(height: 12),
        Row(
          crossAxisAlignment: CrossAxisAlignment.center,
          children: [
            InkWell(
              onTap: () async {
                final result = await showModalBottomSheet<String>(
                  context: context,
                  builder: (ctx) => IconPickerSheet(
                    currentIcon: iconController.text.isEmpty
                        ? 'folder'
                        : iconController.text,
                  ),
                );
                if (result != null) {
                  onIconChanged(result);
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
                  UnifiedObjectService.getIconFromName(iconController.text),
                  color: theme.colorScheme.primary,
                  size: 28,
                ),
              ),
            ),
            const SizedBox(width: 16),
            Expanded(
              child: SizedBox(
                height: 56,
                child: TextField(
                  controller: nameController,
                  expands: true,
                  minLines: null,
                  maxLines: null,
                  textAlignVertical: TextAlignVertical.center,
                  decoration: InputDecoration(
                    hintText: AppLocalizations.of(context).objectEditorEnterSectionName,
                    border: const OutlineInputBorder(),
                    contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 16),
                  ),
                ),
              ),
            ),
          ],
        ),
      ],
    );
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
      decoration: InputDecoration(
        border: const OutlineInputBorder(),
        hintText: AppLocalizations.of(context).objectEditorSelectType,
      ),
      child: DropdownButtonHideUnderline(
        child: DropdownButton<String>(
          isExpanded: true,
          value: effectiveTypeId,
          hint: Text(AppLocalizations.of(context).objectEditorSelectType),
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
    final allObjects = ref.watch(unifiedObjectProvider.select((d) => d.objects));
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
      decoration: InputDecoration(
        border: const OutlineInputBorder(),
        hintText: AppLocalizations.of(context).objectEditorNoParent,
      ),
      child: DropdownButtonHideUnderline(
        child: DropdownButton<String?>(
          isExpanded: true,
          value: effectiveParentId,
          hint: Text(AppLocalizations.of(context).objectEditorNoParent),
          items: [
            DropdownMenuItem(
              value: null,
              child: Text(AppLocalizations.of(context).objectEditorNoParent),
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


/// Fixed bottom save bar for object editor.
class _BottomSaveBar extends StatelessWidget {
  final VoidCallback onSave;

  const _BottomSaveBar({required this.onSave});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Positioned(
      left: 0,
      right: 0,
      bottom: 0,
      child: Container(
        color: theme.scaffoldBackgroundColor,
        padding: AppTheme.kPagePadding,
        child: Center(
          child: OutlinedButton(
            onPressed: onSave,
            child: Text(AppLocalizations.of(context).commonSave),
          ),
        ),
      ),
    );
  }
}

/// Item Properties section with dynamic field list.
class _PropertyFieldsSection extends StatelessWidget {
  final List<_PropertyField> fields;
  final VoidCallback onAdd;
  final ValueChanged<int> onDeleteConfirmed;
  final VoidCallback onFieldChanged;

  const _PropertyFieldsSection({
    required this.fields,
    required this.onAdd,
    required this.onDeleteConfirmed,
    required this.onFieldChanged,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Text(AppLocalizations.of(context).objectEditorItemProperties, style: theme.textTheme.titleMedium),
            const Spacer(),
            IconButton(
              icon: const Icon(Icons.add, size: 20),
              tooltip: AppLocalizations.of(context).objectEditorAddProperty,
              onPressed: onAdd,
              visualDensity: VisualDensity.compact,
            ),
          ],
        ),
        const SizedBox(height: 12),
        ...fields.asMap().entries.map((entry) {
          final index = entry.key;
          final field = entry.value;
          if (field.isDefaultName == true) {
            return Padding(
              padding: const EdgeInsets.only(bottom: 8),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.center,
                children: [
                  Expanded(
                    flex: 2,
                    child: Container(
                      height: 40,
                      alignment: Alignment.centerLeft,
                      padding: const EdgeInsets.symmetric(horizontal: 12),
                      decoration: BoxDecoration(
                        border: Border.all(color: theme.colorScheme.outline.withValues(alpha: 0.3)),
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: Text(
                        AppLocalizations.of(context).objectEditorDefaultFieldTitle,
                        style: theme.textTheme.bodyMedium?.copyWith(
                          color: theme.colorScheme.onSurface,
                        ),
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  const SizedBox(width: 40),
                  const SizedBox(width: 8),
                  const Expanded(
                    flex: 1,
                    child: SizedBox(height: 40),
                  ),
                  const SizedBox(width: 8),
                  SizedBox(
                    width: 72,
                    child: Center(child: SensitivityTag(level: field.sensitivity)),
                  ),
                  const SizedBox(width: 40),
                ],
              ),
            );
          }
          return _PropertyFieldRow(
            field: field,
            index: index,
            onDeleteConfirmed: onDeleteConfirmed,
            onFieldChanged: onFieldChanged,
          );
        }),
      ],
    );
  }
}

/// Single editable property field row.
class _PropertyFieldRow extends ConsumerWidget {
  final _PropertyField field;
  final int index;
  final ValueChanged<int> onDeleteConfirmed;
  final VoidCallback onFieldChanged;

  const _PropertyFieldRow({
    required this.field,
    required this.index,
    required this.onDeleteConfirmed,
    required this.onFieldChanged,
  });

  String _getFieldKeyLabel(String key, AppLocalizations l) {
    switch (key) {
      case 'bank_name':
        return l.fieldBankName;
      case 'account_number':
        return l.fieldAccountNumber;
      case 'account_holder':
        return l.fieldAccountHolder;
      case 'branch_name':
        return l.fieldBranchName;
      case 'sort_code':
        return l.fieldSortCode;
      case 'iban':
        return l.fieldIban;
      case 'routing_number':
        return l.fieldRoutingNumber;
      case 'account_type':
        return l.fieldAccountType;
      default:
        return key;
    }
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final l = AppLocalizations.of(context);
    return Padding(
  padding: const EdgeInsets.only(bottom: 8),
  child: Row(
    crossAxisAlignment: CrossAxisAlignment.center,
    children: [
      Expanded(
        flex: 2,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Localized label for template field keys
            if (_getFieldKeyLabel(field.key, l) != field.key)
              Padding(
                padding: const EdgeInsets.only(bottom: 2, left: 4),
                child: Text(
                  _getFieldKeyLabel(field.key, l),
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.primary,
                    fontWeight: FontWeight.w500,
                  ),
                ),
              ),
            TextField(
              controller: field.controller,
              maxLength: 24,
              buildCounter: (context, {required currentLength, required isFocused, maxLength}) => null,
              decoration: InputDecoration(
                hintText: AppLocalizations.of(context).objectEditorKeyName,
                isDense: true,
                contentPadding: const EdgeInsets.symmetric(horizontal: 10, vertical: 10),
                border: const OutlineInputBorder(),
              ),
              onChanged: (value) {
                field.key = value;
              },
            ),
          ],
        ),
      ),
      const SizedBox(width: 8),
      SizedBox(
        width: 40,
        child: ValueListenableBuilder<TextEditingValue>(
          valueListenable: field.controller,
          builder: (context, value, child) {
            final len = value.text.length;
            return Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                SizedBox(
                  width: 16,
                  child: Text(
                    '$len',
                    textAlign: TextAlign.right,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: len >= 24 ? theme.colorScheme.error : theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ),
                Text(
                  '/',
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: len >= 24 ? theme.colorScheme.error : theme.colorScheme.onSurfaceVariant,
                  ),
                ),
                SizedBox(
                  width: 16,
                  child: Text(
                    AppLocalizations.of(context).objectEditorMaxLength(24),
                    textAlign: TextAlign.left,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: len >= 24 ? theme.colorScheme.error : theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ),
              ],
            );
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
              isExpanded: true,
              value: field.type,
              items: [
                DropdownMenuItem(value: 'text', child: Text(AppLocalizations.of(context).objectEditorPropertyTypeText)),
                DropdownMenuItem(value: 'date', child: Text(AppLocalizations.of(context).objectEditorPropertyTypeDate)),
                DropdownMenuItem(value: 'number', child: Text(AppLocalizations.of(context).objectEditorPropertyTypeNumber)),
                DropdownMenuItem(value: 'checkbox', child: Text(AppLocalizations.of(context).objectEditorPropertyTypeCheckbox)),
                DropdownMenuItem(value: 'select', child: Text(AppLocalizations.of(context).objectEditorPropertyTypeSelect)),
                DropdownMenuItem(value: 'multiSelect', child: Text(AppLocalizations.of(context).objectEditorPropertyTypeMultiSelect)),
              ],
              onChanged: (value) {
                if (value != null) {
                  field.type = value;
                  onFieldChanged();
                }
              },
            ),
          ),
        ),
      ),
      const SizedBox(width: 8),
      SizedBox(
        width: 72,
        child: PopupMenuButton<SensitivityLevel>(
          tooltip: AppLocalizations.of(context).objectEditorSensitivity,
          child: Center(
            child: SensitivityTag(level: field.sensitivity),
          ),
          itemBuilder: (context) => SensitivityLevel.values.map((level) {
            return PopupMenuItem(
              value: level,
              child: Row(
                children: [
                  Icon(Icons.circle, color: getSensitivityColor(level), size: 10),
                  const SizedBox(width: 8),
                  Text(level.label),
                ],
              ),
            );
          }).toList(),
          onSelected: (level) {
            field.sensitivity = level;
            onFieldChanged();
          },
        ),
      ),
      IconButton(
        icon: Icon(Icons.delete_outline, size: 20, color: theme.colorScheme.error),
        tooltip: AppLocalizations.of(context).commonDelete,
        onPressed: () async {
          final keyName = field.key.trim().isNotEmpty ? field.key.trim() : AppLocalizations.of(context).objectEditorItemProperties;
          final confirmed = await showDialog<bool>(
            context: context,
            builder: (context) => AlertDialog(
              title: Text(AppLocalizations.of(context).objectEditorDeletePropertyTitle),
              content: Text(AppLocalizations.of(context).objectEditorDeletePropertyConfirm(keyName)),
              actions: [
                TextButton(
                  onPressed: () => Navigator.of(context).pop(false),
                  child: Text(AppLocalizations.of(context).commonCancel),
                ),
                TextButton(
                  onPressed: () => Navigator.of(context).pop(true),
                  child: Text(AppLocalizations.of(context).commonDelete, style: const TextStyle(color: AppTheme.errorColor)),
                ),
              ],
            ),
          );
          if (confirmed == true) {
            onDeleteConfirmed(index);
          }
        },
        visualDensity: VisualDensity.compact,
      ),
    ],
  ),
);

  }
}

