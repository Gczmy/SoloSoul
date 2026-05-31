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
import 'package:solosoul_flutter/presentation/widgets/object_editor/character_counter.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/core/utils/field_label_resolver.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart' show AppTheme;
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';
import 'package:solosoul_flutter/core/models/section_template.dart';
import 'package:solosoul_flutter/core/models/semantic_type_registry.dart';
import 'package:solosoul_flutter/presentation/pages/section_template_page.dart';
import 'package:solosoul_flutter/presentation/widgets/section_renderer_registry.dart';
import 'package:solosoul_flutter/presentation/widgets/semantic_type_picker.dart';


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
  bool isDeprecated;
  /// The actual stored value for this property (used by deprecated properties).
  String? storedValue;
  /// 语义类型 ID（如 "pet.name"）
  String? semanticType;

  _PropertyField({
    this.key = '',
    this.type = 'text',
    this.isDefaultName = false,
    this.sensitivity = SensitivityLevel.public,
    String? displayLabel,
    this.isDeprecated = false,
    this.storedValue,
    this.semanticType,
  }) : controller = TextEditingController(text: displayLabel ?? key);

  /// Current display label (synced with controller.text).
  String get displayLabel => controller.text.trim();
}

/// 属性构建结果，用于 `_saveObject` 方法间传递。
class _PropertyBuildResult {
  final Map<String, PropertyValue> properties;
  final Map<String, String> propertyLabels;
  final Map<String, String> semanticTypes;
  final List<String> propertyOrder;

  const _PropertyBuildResult({
    required this.properties,
    required this.propertyLabels,
    required this.semanticTypes,
    required this.propertyOrder,
  });
}

class _ObjectEditorPageState extends ConsumerState<ObjectEditorPage> {
  late final TextEditingController _nameController;
  late final TextEditingController _iconController;
  String? _selectedTypeId;
  String? _selectedParentId;
  final List<_PropertyField> _propertyFields = [];
  bool _fieldsInitialized = false;
  bool _showDeprecated = false;
  bool _localizedNameInitialized = false;

  bool get _isEditing => widget.objectId != null;

  String _getFieldKeyLabel(String key) {
    return FieldLabelResolver.resolve(key);
  }

  /// Whether we're editing an item (child of a section/collection).
  /// Items use parent section's properties as schema; sections use their own.
  bool get _isEditingItem {
    final obj = _existingObject;
    if (obj == null) return false;
    if (obj.typeId == 'item') {
      return true;
    }
    final parentId = obj.parentId;
    if (parentId != null) {
      final parent = ref.read(objectByIdProvider(parentId));
      if (parent != null && parent.typeId != 'page') {
        return true;
      }
    }
    return false;
  }

  /// Schema properties for the current editing context.
  /// For items: parent section's properties (the schema authority).
  /// For sections: the section's own properties.
  Map<String, PropertyValue> get _schemaProperties {
    if (_isEditingItem) {
      final parentId = _existingObject?.parentId;
      if (parentId != null) {
        final parent = ref.read(objectByIdProvider(parentId));
        if (parent != null && parent.typeId != 'page') {
          return _orderedProperties(parent);
        }
      }
    }
    return _orderedProperties(_existingObject);
  }

  void _handlePropertyReorder(int oldIndex, int newIndex) {
    setState(() {
      final normalIndices = <int>[];
      for (int i = 0; i < _propertyFields.length; i++) {
        if (!_propertyFields[i].isDeprecated && _propertyFields[i].isDefaultName != true) {
          normalIndices.add(i);
        }
      }
      if (oldIndex < 0 || oldIndex >= normalIndices.length) return;

      final oldFieldIndex = normalIndices[oldIndex];

      int newFieldIndex;
      if (newIndex >= normalIndices.length) {
        newFieldIndex = normalIndices.last + 1;
      } else {
        newFieldIndex = normalIndices[newIndex];
      }

      if (oldFieldIndex < newFieldIndex) {
        newFieldIndex--;
      }

      final field = _propertyFields.removeAt(oldFieldIndex);
      _propertyFields.insert(newFieldIndex, field);
    });
  }

  /// 按 propertyOrder 排序返回 properties，保证字段顺序一致。
  Map<String, PropertyValue> _orderedProperties(UnifiedObject? obj) {
    if (obj == null || obj.propertyOrder.isEmpty) return obj?.properties ?? {};
    final ordered = <String, PropertyValue>{};
    for (final key in obj.propertyOrder) {
      if (obj.properties.containsKey(key)) {
        ordered[key] = obj.properties[key]!;
      }
    }
    // 追加不在 propertyOrder 中的字段
    for (final entry in obj.properties.entries) {
      if (!ordered.containsKey(entry.key)) {
        ordered[entry.key] = entry.value;
      }
    }
    return ordered;
  }

  void _initFieldDisplayLabels() {
    if (_fieldsInitialized) return;
    for (final field in _propertyFields) {
      final localized = _getFieldKeyLabel(field.key);
      if (localized != field.key) {
        field.controller.text = localized;
      }
    }
    _fieldsInitialized = true;
  }

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
    final objectId = widget.objectId;
    if (objectId == null) return;
    final object = ref.read(objectByIdProvider(objectId));
    if (object == null) return;

    _existingObject = object;
    _nameController.text = object.name;
    _iconController.text = object.iconName;
    // Name localization will be applied on first build via _initLocalizedName()
    _selectedTypeId = object.typeId;
    _selectedParentId = object.parentId;
    _propertyFields.clear();
    bool hasDefaultName = false;

    // Check if the object type has a custom title property key.
    final typeDef = ObjectTypeRegistry.getType(object.typeId ?? '');
    final typeTitleKey = typeDef?.titlePropertyKey;
    // When a type has a designated title key, only that key is treated as the title.
    // Otherwise fall back to the standard Title/title/Item Name detection.
    bool isTitleKey(String key) {
      if (typeTitleKey != null) return key == typeTitleKey;
      return key == 'Title' || key == 'title' || key == 'Item Name';
    }

    if (_isEditingItem) {
      // Item: use parent section's properties as schema authority.
      final schemaProps = _schemaProperties;
      final parentId = _existingObject?.parentId;
      final schemaObj = parentId != null
          ? ref.read(objectByIdProvider(parentId))
          : null;
      final parentLabels = schemaObj?.propertyLabels;

      // Add all schema properties (parent section's current properties).
      for (final entry in schemaProps.entries) {
        final sensitivity = entry.value.sensitivity;
        if (isTitleKey(entry.key)) {
          if (!hasDefaultName) {
            _propertyFields.add(_PropertyField(
              key: entry.key,
              type: 'text',
              isDefaultName: true,
              sensitivity: sensitivity,
            ));
            hasDefaultName = true;
          }
          continue;
        } else {
          final storedProp = object.properties[entry.key];
          _propertyFields.add(_PropertyField(
            key: entry.key,
            type: storedProp != null ? _inferTypeFromValue(storedProp) : _inferTypeFromValue(entry.value),
            sensitivity: sensitivity,
            displayLabel: parentLabels?[entry.key],
            storedValue: storedProp != null ? _propertyValueToString(storedProp) : null,
            semanticType: object.semanticTypes?[entry.key],
          ));
        }
      }

      // Add item properties NOT in parent schema as deprecated (data preserved).
      final schemaKeys = schemaProps.keys.toSet();
      for (final entry in object.properties.entries) {
        if (schemaKeys.contains(entry.key)) continue;
        if (isTitleKey(entry.key)) continue;
        _propertyFields.add(_PropertyField(
          key: entry.key,
          type: _inferTypeFromValue(entry.value),
          sensitivity: entry.value.sensitivity,
          displayLabel: parentLabels?[entry.key],
          storedValue: _propertyValueToString(entry.value),
          semanticType: object.semanticTypes?[entry.key],
          isDeprecated: true,
        ));
      }
    } else {
      // Section: load own properties directly (they ARE the schema).
      final orderedProps = _orderedProperties(object);
      for (final entry in orderedProps.entries) {
        final sensitivity = entry.value.sensitivity;
        if (isTitleKey(entry.key)) {
          if (!hasDefaultName) {
            _propertyFields.add(_PropertyField(
              key: entry.key,
              type: 'text',
              isDefaultName: true,
              sensitivity: sensitivity,
            ));
            hasDefaultName = true;
          }
          continue;
        } else {
          _propertyFields.add(_PropertyField(
            key: entry.key,
            type: _inferTypeFromValue(entry.value),
            sensitivity: sensitivity,
            displayLabel: object.propertyLabels?[entry.key],
            semanticType: object.semanticTypes?[entry.key],
          ));
        }
      }
    }

    if (!hasDefaultName) {
      _propertyFields.insert(0, _PropertyField(
        key: typeTitleKey ?? 'Title',
        type: 'text',
        isDefaultName: true,
      ));
    }
    setState(() {});
  }

  void _initPropertiesFromType(String typeId) {
    _propertyFields.clear();
    final type = ObjectTypeRegistry.getType(typeId);
    final titleKey = type?.titlePropertyKey ?? 'Title';
    _propertyFields.add(_PropertyField(key: titleKey, type: 'text', isDefaultName: true));
    if (type == null) return;
    for (final propDef in type.properties) {
      if (propDef.id != titleKey && propDef.id != 'Title' && propDef.id != 'Item Name') {
        _propertyFields.add(_PropertyField(
          key: propDef.id,
          type: propDef.type.name,
          displayLabel: propDef.name.isNotEmpty ? propDef.name : null,
        ));
      }
    }
  }

  /// Convert snake_case keys to camelCase to align with ObjectTypeRegistry.
  static String _snakeToCamelCase(String key) {
    final parts = key.split('_');
    if (parts.length <= 1) return key;
    return parts[0] +
        parts.sublist(1).map((p) => p.isEmpty ? '' : p[0].toUpperCase() + p.substring(1)).join();
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

  /// 根据类型字符串和已有值创建 PropertyValue（保留 item 的存储数据）。
  static PropertyValue _createPropertyValueWithValue(
      String type, String value, SensitivityLevel sensitivity) {
    return switch (type) {
      'date' => DateProperty(isoDate: value.isEmpty ? null : value, sensitivity: sensitivity),
      'number' => NumberProperty(value: double.tryParse(value), sensitivity: sensitivity),
      'checkbox' => CheckboxProperty(checked: value == 'Yes', sensitivity: sensitivity),
      _ => TextProperty(text: value, sensitivity: sensitivity),
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

  void _initLocalizedName() {
    if (_localizedNameInitialized) return;
    final obj = _existingObject;
    if (obj == null) return;
    final l10n = AppLocalizations.of(context);
    final localizedName = getLocalizedObjectName(l10n, obj);
    if (localizedName != obj.name) {
      _nameController.text = localizedName;
    }
    _localizedNameInitialized = true;
  }

  @override
  Widget build(BuildContext context) {
    if (_isEditing) {
      _initFieldDisplayLabels();
      _initLocalizedName();
    }
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

                // 所属页面
                if (!_isEditingItem) ...[
                  Text(AppLocalizations.of(context).objectEditorParentPage, style: theme.textTheme.titleMedium),
                  const SizedBox(height: 12),
                  ObjectParentDropdown(
                    selectedParentId: _selectedParentId,
                    objectId: widget.objectId,
                    onChanged: (value) {
                      setState(() {
                        _selectedParentId = value;
                      });
                    },
                  ),
                  const SizedBox(height: 24),
                ],

                _PropertyFieldsSection(
                  fields: _propertyFields,
                  showDeprecated: _showDeprecated,
                  isItemEditor: _isEditingItem,
                  onToggleDeprecated: () => setState(() => _showDeprecated = !_showDeprecated),
                  onAdd: () => setState(() => _propertyFields.add(_PropertyField(key: '', type: 'text'))),
                  onDeleteConfirmed: (index, key) {
                    if (_isEditingItem) {
                      // Item: mark as deprecated (hide, preserve data)
                      setState(() => _propertyFields[index].isDeprecated = true);
                    } else {
                      // Section: truly remove from schema
                      setState(() {
                        final removed = _propertyFields.removeAt(index);
                        removed.controller.dispose();
                      });
                    }
                  },
                  onFieldChanged: () => setState(() {}),
                  onRestoreDeprecated: (key) {
                    setState(() {
                      final field = _propertyFields.firstWhere((f) => f.key == key);
                      field.isDeprecated = false;
                    });
                  },
                  onReorder: _handlePropertyReorder,
                ),
                const SizedBox(height: 24),

                // 模板入口 — liquid glass styled with hover effect
                const SizedBox(height: 16),
                _TemplateGlassButton(
                  onTap: () async {
                    final template = await Navigator.of(context).push<SectionTemplate>(
                      MaterialPageRoute(
                        builder: (context) => const SectionTemplatePage(),
                      ),
                    );
                    if (template != null) {
                      setState(() {
                        for (final field in template.fields) {
                          // Normalize template keys from snake_case to camelCase
                          // to match ObjectTypeRegistry conventions.
                          final normalizedKey = _snakeToCamelCase(field.key);
                          final existingKeys = _propertyFields.map((f) => f.key).toSet();
                          if (!existingKeys.contains(normalizedKey)) {
                            _propertyFields.add(_PropertyField(
                              key: normalizedKey,
                              type: field.type,
                              sensitivity: field.sensitivity,
                              displayLabel: _getFieldKeyLabel(normalizedKey),
                            ));
                          }
                        }
                      });
                    }
                  },
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
    final obj = _existingObject;
    if (obj == null) return;
    final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
    if (accountId == null) return;

    final allFieldValues = <String, String>{};
    for (final entry in obj.properties.entries) {
      allFieldValues[entry.key] = _propertyValueToString(entry.value);
    }

    await ref.read(fieldHistoriesProvider.notifier).recordSnapshot(
      accountId: accountId,
      itemId: obj.id,
      fieldIdPrefix: 'unified',
      allFieldValues: allFieldValues,
    );
  }

  /// 验证保存输入：检查空名称和重复属性键。
  /// 返回错误消息，验证通过返回 null。
  String? _validateSaveInput() {
    if (_nameController.text.trim().isEmpty) {
      return AppLocalizations.of(context).objectEditorNameRequired;
    }

    final keyCounts = <String, int>{};
    for (final field in _propertyFields) {
      if (field.isDeprecated) continue;
      final key = field.isDefaultName == true
          ? AppLocalizations.of(context).objectEditorDefaultFieldTitle
          : (field.key.trim().isEmpty ? field.displayLabel : field.key.trim());
      if (key.isNotEmpty) {
        keyCounts[key] = (keyCounts[key] ?? 0) + 1;
      }
    }
    final duplicates = keyCounts.entries.where((e) => e.value > 1).map((e) => e.key).toList();
    if (duplicates.isNotEmpty) {
      return AppLocalizations.of(context).objectEditorDuplicateProperties(duplicates.join(', '));
    }
    return null;
  }

  /// 从属性字段列表构建保存所需的属性映射。
  _PropertyBuildResult _buildProperties() {
    final properties = <String, PropertyValue>{};
    final propertyLabels = <String, String>{};
    final semanticTypes = <String, String>{};
    final propertyOrder = <String>[];

    for (final field in _propertyFields) {
      final key = field.isDefaultName == true && field.key.trim().isEmpty
          ? AppLocalizations.of(context).objectEditorDefaultFieldItemName
          : (field.key.trim().isEmpty ? field.displayLabel : field.key.trim());
      if (key.isEmpty) continue;

      if (field.isDefaultName == true) {
        properties[key] = const TextProperty(text: '', sensitivity: SensitivityLevel.public);
      } else if (_isEditingItem && field.isDeprecated) {
        properties[key] = _createPropertyValueWithValue(field.type, field.storedValue ?? '', field.sensitivity);
      } else if (_isEditingItem && field.storedValue != null) {
        properties[key] = _createPropertyValueWithValue(field.type, field.storedValue!, field.sensitivity);
      } else {
        properties[key] = _createEmptyPropertyValue(field.type, field.sensitivity);
      }

      final displayLabel = field.displayLabel;
      if (displayLabel.isNotEmpty && displayLabel != key) {
        propertyLabels[key] = displayLabel;
      }

      final semanticType = field.semanticType;
      if (semanticType != null && semanticType.isNotEmpty) {
        semanticTypes[key] = semanticType;
      }

      if (!field.isDeprecated) {
        propertyOrder.add(key);
      }
    }

    return _PropertyBuildResult(
      properties: properties,
      propertyLabels: propertyLabels,
      semanticTypes: semanticTypes,
      propertyOrder: propertyOrder,
    );
  }

  Future<void> _saveObject() async {
    try {
      final validationError = _validateSaveInput();
      if (validationError != null) {
        if (mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(content: Text(validationError)),
          );
        }
        return;
      }

      final built = _buildProperties();
      final notifier = ref.read(unifiedObjectProvider.notifier);
      final existing = _existingObject;

      if (_isEditing && existing != null) {
        await _recordHistory();
        if (existing.parentId != _selectedParentId) {
          await notifier.moveObject(existing.id, _selectedParentId);
        }
        await notifier.updateObject(
          existing.id,
          name: _nameController.text.trim(),
          typeId: _selectedTypeId,
          iconName: _iconController.text.trim(),
          properties: built.properties,
          propertyLabels: built.propertyLabels.isNotEmpty ? built.propertyLabels : null,
          semanticTypes: built.semanticTypes.isNotEmpty ? built.semanticTypes : null,
          propertyOrder: built.propertyOrder,
        );
      } else {
        await notifier.createObject(
          name: _nameController.text.trim(),
          typeId: _selectedTypeId,
          parentId: _selectedParentId,
          iconName: _iconController.text.trim(),
          properties: built.properties,
          propertyLabels: built.propertyLabels.isNotEmpty ? built.propertyLabels : null,
          semanticTypes: built.semanticTypes.isNotEmpty ? built.semanticTypes : null,
          propertyOrder: built.propertyOrder,
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
            final l10n = AppLocalizations.of(context);
            final displayName = SectionRendererRegistry.getConfig(type.id)?.l10nTitle(l10n) ?? type.name;
            return DropdownMenuItem(
              value: type.id,
              child: Row(
                children: [
                  Icon(
                    UnifiedObjectService.getIconFromName(type.iconName),
                    size: 20,
                  ),
                  const SizedBox(width: 12),
                  Text(displayName),
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

class _TitleFieldRow extends StatelessWidget {
  final ThemeData theme;
  final SensitivityLevel sensitivity;

  const _TitleFieldRow({required this.theme, required this.sensitivity});

  @override
  Widget build(BuildContext context) {
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
            child: Center(child: SensitivityTag(level: sensitivity)),
          ),
          const SizedBox(width: 40),
        ],
      ),
    );
  }
}
class ObjectParentDropdown extends ConsumerWidget {
  final String? selectedParentId;
  final String? objectId;
  final ValueChanged<String> onChanged;

  const ObjectParentDropdown({
    super.key,
    this.selectedParentId,
    this.objectId,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final allObjects = ref.watch(unifiedObjectProvider.select((d) => d.objects));
    final pages = allObjects
        .where((o) => !o.isDeleted && o.id != objectId && o.typeId == 'page')
        .toList();
    final effectiveParentId = pages.any((o) => o.id == selectedParentId)
        ? selectedParentId
        : (pages.isNotEmpty ? pages.first.id : null);

    return InputDecorator(
      decoration: const InputDecoration(
        border: OutlineInputBorder(),
      ),
      child: DropdownButtonHideUnderline(
        child: DropdownButton<String>(
          isExpanded: true,
          value: effectiveParentId,
          items: pages.map((o) {
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
                      getLocalizedObjectName(AppLocalizations.of(context), o),
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                ],
              ),
            );
          }).toList(),
          onChanged: (value) {
            if (value != null) onChanged(value);
          },
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
  final bool showDeprecated;
  final bool isItemEditor;
  final VoidCallback onToggleDeprecated;
  final VoidCallback onAdd;
  final void Function(int index, String key) onDeleteConfirmed;
  final VoidCallback onFieldChanged;
  final ValueChanged<String> onRestoreDeprecated;
  final void Function(int oldIndex, int newIndex) onReorder;

  const _PropertyFieldsSection({
    required this.fields,
    required this.showDeprecated,
    required this.isItemEditor,
    required this.onToggleDeprecated,
    required this.onAdd,
    required this.onDeleteConfirmed,
    required this.onFieldChanged,
    required this.onRestoreDeprecated,
    required this.onReorder,
  });

  @override
  Widget build(BuildContext context) {
    final deprecatedFields = fields.where((f) => f.isDeprecated).toList();
    final normalFields = fields.where((f) => !f.isDeprecated && f.isDefaultName != true).toList();
    final titleFieldList = fields.where((f) => f.isDefaultName == true).toList();
    final titleField = titleFieldList.isNotEmpty ? titleFieldList.first : null;
    final theme = Theme.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Text(AppLocalizations.of(context).objectEditorItemProperties, style: theme.textTheme.titleMedium),
            const Spacer(),
            if (deprecatedFields.isNotEmpty && isItemEditor)
              TextButton.icon(
                onPressed: onToggleDeprecated,
                icon: Icon(showDeprecated ? Icons.visibility_off : Icons.visibility, size: 16),
                label: Text(showDeprecated
                    ? AppLocalizations.of(context).objectEditorHideDeprecated
                    : AppLocalizations.of(context).objectEditorShowDeprecated(deprecatedFields.length)),
                style: TextButton.styleFrom(
                  visualDensity: VisualDensity.compact,
                  padding: const EdgeInsets.symmetric(horizontal: 8),
                ),
              ),
            IconButton(
              icon: const Icon(Icons.add, size: 20),
              tooltip: AppLocalizations.of(context).objectEditorAddProperty,
              onPressed: onAdd,
              visualDensity: VisualDensity.compact,
            ),
          ],
        ),
        const SizedBox(height: 12),

        // Title field
        if (titleField != null)
          _TitleFieldRow(theme: theme, sensitivity: titleField.sensitivity),

        // Normal property fields (reorderable)
        ReorderableListView.builder(
          shrinkWrap: true,
          physics: const NeverScrollableScrollPhysics(),
          itemCount: normalFields.length,
          onReorder: onReorder,
          buildDefaultDragHandles: false,
          itemBuilder: (context, index) {
            final field = normalFields[index];
            final fieldIndex = fields.indexOf(field);
            return _PropertyFieldRow(
              key: ObjectKey(field),
              field: field,
              index: fieldIndex,
              onDeleteConfirmed: onDeleteConfirmed,
              onFieldChanged: onFieldChanged,
              showDragHandle: true,
              dragIndex: index,
            );
          },
        ),

        // Deprecated property fields (collapsible)
        if (deprecatedFields.isNotEmpty && showDeprecated) ...[
          const SizedBox(height: 8),
          Divider(color: theme.colorScheme.outline.withValues(alpha: 0.2)),
          const SizedBox(height: 8),
          Text(
            AppLocalizations.of(context).objectEditorDeprecatedProperties,
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(height: 8),
          ...deprecatedFields.map((field) => _DeprecatedPropertyRow(
            field: field,
            onRestore: () => onRestoreDeprecated(field.key),
          )),
        ],
      ],
    );
  }
}

/// Read-only deprecated property row with restore action.
class _DeprecatedPropertyRow extends StatelessWidget {
  final _PropertyField field;
  final VoidCallback onRestore;

  const _DeprecatedPropertyRow({
    required this.field,
    required this.onRestore,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l = AppLocalizations.of(context);
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Row(
        children: [
          // Deprecated badge
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
            decoration: BoxDecoration(
              color: theme.colorScheme.errorContainer,
              borderRadius: BorderRadius.circular(4),
            ),
            child: Text(
              l.objectEditorDeprecatedBadge,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onErrorContainer,
                fontSize: 10,
              ),
            ),
          ),
          const SizedBox(width: 8),
          // Key label (localized)
          Expanded(
            flex: 2,
            child: Text(
              FieldLabelResolver.resolve(field.key),
              style: theme.textTheme.bodyMedium?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
                decoration: TextDecoration.lineThrough,
              ),
            ),
          ),
          // Value (read-only)
          Expanded(
            flex: 1,
            child: Container(
              height: 40,
              alignment: Alignment.centerLeft,
              padding: const EdgeInsets.symmetric(horizontal: 10),
              decoration: BoxDecoration(
                color: theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.5),
                borderRadius: BorderRadius.circular(4),
                border: Border.all(
                  color: theme.colorScheme.outline.withValues(alpha: 0.2),
                ),
              ),
              child: Text(
                _getPropertyDisplayValue(field),
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
                overflow: TextOverflow.ellipsis,
              ),
            ),
          ),
          const SizedBox(width: 8),
          // Restore button
          IconButton(
            icon: Icon(Icons.restore, size: 18, color: theme.colorScheme.primary),
            tooltip: l.objectEditorRestoreProperty,
            onPressed: onRestore,
            visualDensity: VisualDensity.compact,
          ),
        ],
      ),
    );
  }

  String _getPropertyDisplayValue(_PropertyField field) {
    final value = field.storedValue;
    if (value == null || value.isEmpty) return '—';
    return value;
  }
}

/// Single editable property field row.
class _PropertyFieldRow extends ConsumerWidget {
  final _PropertyField field;
  final int index;
  final void Function(int index, String key) onDeleteConfirmed;
  final VoidCallback onFieldChanged;
  final bool showDragHandle;
  /// Index within the ReorderableListView (not _propertyFields).
  final int? dragIndex;

  const _PropertyFieldRow({
    super.key,
    required this.field,
    required this.index,
    required this.onDeleteConfirmed,
    required this.onFieldChanged,
    this.showDragHandle = false,
    this.dragIndex,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context);
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          if (showDragHandle) ...[
            ReorderableDragStartListener(
              index: dragIndex ?? index,
              child: Icon(
                Icons.drag_handle,
                size: 20,
                color: Theme.of(context).colorScheme.onSurfaceVariant.withValues(alpha: 0.5),
              ),
            ),
            const SizedBox(width: 4),
          ],
          Expanded(
            flex: 2,
            child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // 显示标签输入框
              TextField(
                controller: field.controller,
                maxLength: 24,
                buildCounter: (context, {required currentLength, required isFocused, maxLength}) => null,
                decoration: InputDecoration(
                  hintText: l10n.objectEditorKeyName,
                  isDense: true,
                  contentPadding: const EdgeInsets.symmetric(horizontal: 10, vertical: 10),
                  border: const OutlineInputBorder(),
                ),
              ),
              // 语义类型 Chip
              if (field.semanticType != null)
                Padding(
                  padding: const EdgeInsets.only(top: 4),
                  child: _SemanticTypeChip(
                    semanticType: field.semanticType!,
                    onTap: () async {
                      final selected = await showModalBottomSheet<String?>(
                        context: context,
                        isScrollControlled: true,
                        backgroundColor: Colors.transparent,
                        builder: (ctx) => SemanticTypePickerSheet(
                          currentSemanticType: field.semanticType,
                          languageCode: Localizations.localeOf(context).languageCode,
                          onSelected: (type) => Navigator.of(ctx).pop(type),
                        ),
                      );
                      if (selected != null) {
                        field.semanticType = selected;
                        onFieldChanged();
                      }
                    },
                  ),
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
            return CharacterCounter(
              currentLength: value.text.length,
              maxLength: 24,
              maxLabel: AppLocalizations.of(context).objectEditorMaxLength(24),
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
                DropdownMenuItem(value: 'url', child: Text(AppLocalizations.of(context).objectEditorPropertyTypeUrl)),
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
        tooltip: l10n.commonDelete,
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
            onDeleteConfirmed(index, field.key);
          }
        },
        visualDensity: VisualDensity.compact,
      ),
    ],
  ),
);

  }
}

/// 语义类型选择 Chip
class _SemanticTypeChip extends StatelessWidget {
  final String semanticType;
  final VoidCallback onTap;

  const _SemanticTypeChip({required this.semanticType, required this.onTap});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final type = SemanticTypeRegistry.getType(semanticType);
    final label = type?.getLabel(Localizations.localeOf(context).languageCode) ?? semanticType;

    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(12),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
        decoration: BoxDecoration(
          color: theme.colorScheme.primaryContainer.withValues(alpha: 0.5),
          borderRadius: BorderRadius.circular(12),
          border: Border.all(
            color: theme.colorScheme.primary.withValues(alpha: 0.3),
            width: 1,
          ),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              type?.icon ?? Icons.label,
              size: 12,
              color: theme.colorScheme.primary,
            ),
            const SizedBox(width: 4),
            Text(
              label,
              style: theme.textTheme.bodySmall?.copyWith(
                fontSize: 11,
                color: theme.colorScheme.primary,
                fontWeight: FontWeight.w500,
              ),
            ),
            const SizedBox(width: 2),
            Icon(
              Icons.edit,
              size: 10,
              color: theme.colorScheme.primary.withValues(alpha: 0.7),
            ),
          ],
        ),
      ),
    );
  }
}

/// Liquid-glass template selector button with hover animation.
class _TemplateGlassButton extends StatefulWidget {
  final VoidCallback onTap;

  const _TemplateGlassButton({required this.onTap});

  @override
  State<_TemplateGlassButton> createState() => _TemplateGlassButtonState();
}

class _TemplateGlassButtonState extends State<_TemplateGlassButton> {
  bool _isHovered = false;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final isDark = Theme.of(context).brightness == Brightness.dark;
    final textColor = isDark ? Colors.white : const Color(0xFF1F1F1F);

    return MouseRegion(
      onEnter: (_) => setState(() => _isHovered = true),
      onExit: (_) => setState(() => _isHovered = false),
      cursor: SystemMouseCursors.click,
      child: AnimatedScale(
        scale: _isHovered ? 1.015 : 1.0,
        duration: const Duration(milliseconds: 200),
        curve: Curves.easeOut,
        child: GlassButton.custom(
          onTap: widget.onTap,
          width: double.infinity,
          shape: const LiquidRoundedSuperellipse(borderRadius: 12),
          glowOpacity: _isHovered ? 0.15 : null,
          child: Padding(
            padding: const EdgeInsets.symmetric(vertical: 14, horizontal: 16),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                AnimatedDefaultTextStyle(
                  duration: const Duration(milliseconds: 200),
                  curve: Curves.easeOut,
                  style: TextStyle(
                    fontSize: 18,
                    color: _isHovered
                        ? textColor
                        : textColor.withValues(alpha: 0.7),
                  ),
                  child: const Text('✦'),
                ),
                const SizedBox(width: 10),
                AnimatedDefaultTextStyle(
                  duration: const Duration(milliseconds: 200),
                  curve: Curves.easeOut,
                  style: TextStyle(
                    fontSize: 15,
                    fontWeight: FontWeight.w600,
                    color: _isHovered
                        ? textColor
                        : textColor.withValues(alpha: 0.7),
                  ),
                  child: Text(l10n.sectionTemplateSelectButton),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

