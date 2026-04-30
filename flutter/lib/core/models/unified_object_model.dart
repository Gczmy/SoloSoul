import 'package:json_annotation/json_annotation.dart';
import 'base_models.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';

part 'unified_object_model.g.dart';

// =============================================================================
// Layout & Property Type Enums
// =============================================================================

/// Layout mode for how an object presents itself and its children in UI.
enum ObjectLayout {
  /// Document-style: free-form content with nested children (like Notion Page).
  @JsonValue('document')
  document,

  /// Collection-style: primarily an organizer of child objects (like Folder/DB).
  @JsonValue('collection')
  collection,
}

/// Supported property value types.
enum PropertyType {
  @JsonValue('text')
  text,
  @JsonValue('number')
  number,
  @JsonValue('date')
  date,
  @JsonValue('checkbox')
  checkbox,
  @JsonValue('select')
  select,
  @JsonValue('multiSelect')
  multiSelect,
  @JsonValue('relation')
  relation,
  @JsonValue('url')
  url,
}

// =============================================================================
// Schema Definitions (ObjectType + PropertyDefinition)
// =============================================================================

/// A single option for select / multi-select properties.
@JsonSerializable(explicitToJson: true)
class SelectOption {
  final String id;
  final String label;
  final int order;

  const SelectOption({
    required this.id,
    required this.label,
    required this.order,
  });

  factory SelectOption.fromJson(Map<String, dynamic> json) =>
      _$SelectOptionFromJson(json);

  Map<String, dynamic> toJson() => _$SelectOptionToJson(this);

  SelectOption copyWith({
    String? id,
    String? label,
    int? order,
  }) {
    return SelectOption(
      id: id ?? this.id,
      label: label ?? this.label,
      order: order ?? this.order,
    );
  }
}

/// Definition of a property (Schema layer).
/// Tells the UI what properties an object of a given type should have.
@JsonSerializable(explicitToJson: true)
class PropertyDefinition {
  final String id;
  final String name;
  final PropertyType type;

  /// Type-specific config:
  /// - select/multiSelect: {'options': [SelectOption, ...]}
  /// - number: {'decimalPlaces': 2}
  /// - relation: {'targetTypeId': 'page'}
  final Map<String, dynamic>? config;

  final bool required;
  final int order;

  const PropertyDefinition({
    required this.id,
    required this.name,
    required this.type,
    this.config,
    this.required = false,
    this.order = 0,
  });

  factory PropertyDefinition.fromJson(Map<String, dynamic> json) =>
      _$PropertyDefinitionFromJson(json);

  Map<String, dynamic> toJson() => _$PropertyDefinitionToJson(this);

  PropertyDefinition copyWith({
    String? id,
    String? name,
    PropertyType? type,
    Map<String, dynamic>? config,
    bool? required,
    int? order,
  }) {
    return PropertyDefinition(
      id: id ?? this.id,
      name: name ?? this.name,
      type: type ?? this.type,
      config: config ?? this.config,
      required: required ?? this.required,
      order: order ?? this.order,
    );
  }
}

/// Definition of an object type (Schema layer).
/// Built-in types are shipped as code constants; user-defined types are stored.
@JsonSerializable(explicitToJson: true)
class ObjectTypeDefinition {
  final String id;
  final String name;
  final String iconName;
  final String? description;
  final ObjectLayout defaultLayout;
  final List<PropertyDefinition> properties;

  const ObjectTypeDefinition({
    required this.id,
    required this.name,
    this.iconName = 'folder',
    this.description,
    this.defaultLayout = ObjectLayout.document,
    this.properties = const [],
  });

  factory ObjectTypeDefinition.fromJson(Map<String, dynamic> json) =>
      _$ObjectTypeDefinitionFromJson(json);

  Map<String, dynamic> toJson() => _$ObjectTypeDefinitionToJson(this);

  ObjectTypeDefinition copyWith({
    String? id,
    String? name,
    String? iconName,
    String? description,
    ObjectLayout? defaultLayout,
    List<PropertyDefinition>? properties,
  }) {
    return ObjectTypeDefinition(
      id: id ?? this.id,
      name: name ?? this.name,
      iconName: iconName ?? this.iconName,
      description: description ?? this.description,
      defaultLayout: defaultLayout ?? this.defaultLayout,
      properties: properties ?? this.properties,
    );
  }
}

// =============================================================================
// PropertyValue Sealed Class Hierarchy
// =============================================================================

/// Base class for all property values.
sealed class PropertyValue {
  const PropertyValue();

  /// Sensitivity level for this property.
  SensitivityLevel get sensitivity;
}

/// Text property.
@JsonSerializable(explicitToJson: true)
class TextProperty extends PropertyValue {
  final String text;
  final int? maxLength;
  @override
  final SensitivityLevel sensitivity;

  const TextProperty({
    required this.text,
    this.maxLength,
    this.sensitivity = SensitivityLevel.public,
  });

  factory TextProperty.fromJson(Map<String, dynamic> json) =>
      _$TextPropertyFromJson(json);

  Map<String, dynamic> toJson() => _$TextPropertyToJson(this);

  TextProperty copyWith({
    String? text,
    int? maxLength,
    SensitivityLevel? sensitivity,
  }) {
    return TextProperty(
      text: text ?? this.text,
      maxLength: maxLength ?? this.maxLength,
      sensitivity: sensitivity ?? this.sensitivity,
    );
  }
}

/// Number property.
@JsonSerializable(explicitToJson: true)
class NumberProperty extends PropertyValue {
  final double? value;
  final int? decimalPlaces;
  @override
  final SensitivityLevel sensitivity;

  const NumberProperty({
    this.value,
    this.decimalPlaces,
    this.sensitivity = SensitivityLevel.public,
  });

  factory NumberProperty.fromJson(Map<String, dynamic> json) =>
      _$NumberPropertyFromJson(json);

  Map<String, dynamic> toJson() => _$NumberPropertyToJson(this);

  NumberProperty copyWith({
    double? value,
    int? decimalPlaces,
    SensitivityLevel? sensitivity,
  }) {
    return NumberProperty(
      value: value ?? this.value,
      decimalPlaces: decimalPlaces ?? this.decimalPlaces,
      sensitivity: sensitivity ?? this.sensitivity,
    );
  }
}

/// Date property (stored as ISO-8601 string for JSON compatibility).
@JsonSerializable(explicitToJson: true)
class DateProperty extends PropertyValue {
  final String? isoDate; // yyyy-MM-dd or full ISO
  final bool includeTime;
  @override
  final SensitivityLevel sensitivity;

  const DateProperty({
    this.isoDate,
    this.includeTime = false,
    this.sensitivity = SensitivityLevel.public,
  });

  factory DateProperty.fromJson(Map<String, dynamic> json) =>
      _$DatePropertyFromJson(json);

  Map<String, dynamic> toJson() => _$DatePropertyToJson(this);

  DateProperty copyWith({
    String? isoDate,
    bool? includeTime,
    SensitivityLevel? sensitivity,
  }) {
    return DateProperty(
      isoDate: isoDate ?? this.isoDate,
      includeTime: includeTime ?? this.includeTime,
      sensitivity: sensitivity ?? this.sensitivity,
    );
  }
}

/// Checkbox property.
@JsonSerializable(explicitToJson: true)
class CheckboxProperty extends PropertyValue {
  final bool checked;
  @override
  final SensitivityLevel sensitivity;

  const CheckboxProperty({
    this.checked = false,
    this.sensitivity = SensitivityLevel.public,
  });

  factory CheckboxProperty.fromJson(Map<String, dynamic> json) =>
      _$CheckboxPropertyFromJson(json);

  Map<String, dynamic> toJson() => _$CheckboxPropertyToJson(this);

  CheckboxProperty copyWith({
    bool? checked,
    SensitivityLevel? sensitivity,
  }) {
    return CheckboxProperty(
      checked: checked ?? this.checked,
      sensitivity: sensitivity ?? this.sensitivity,
    );
  }
}

/// Single-select property.
@JsonSerializable(explicitToJson: true)
class SelectProperty extends PropertyValue {
  final List<SelectOption> options;
  final String? selectedId;
  @override
  final SensitivityLevel sensitivity;

  const SelectProperty({
    required this.options,
    this.selectedId,
    this.sensitivity = SensitivityLevel.public,
  });

  factory SelectProperty.fromJson(Map<String, dynamic> json) =>
      _$SelectPropertyFromJson(json);

  Map<String, dynamic> toJson() => _$SelectPropertyToJson(this);

  SelectProperty copyWith({
    List<SelectOption>? options,
    String? selectedId,
    SensitivityLevel? sensitivity,
  }) {
    return SelectProperty(
      options: options ?? this.options,
      selectedId: selectedId ?? this.selectedId,
      sensitivity: sensitivity ?? this.sensitivity,
    );
  }
}

/// Multi-select property.
@JsonSerializable(explicitToJson: true)
class MultiSelectProperty extends PropertyValue {
  final List<SelectOption> options;
  final List<String> selectedIds;
  @override
  final SensitivityLevel sensitivity;

  const MultiSelectProperty({
    required this.options,
    this.selectedIds = const [],
    this.sensitivity = SensitivityLevel.public,
  });

  factory MultiSelectProperty.fromJson(Map<String, dynamic> json) =>
      _$MultiSelectPropertyFromJson(json);

  Map<String, dynamic> toJson() => _$MultiSelectPropertyToJson(this);

  MultiSelectProperty copyWith({
    List<SelectOption>? options,
    List<String>? selectedIds,
    SensitivityLevel? sensitivity,
  }) {
    return MultiSelectProperty(
      options: options ?? this.options,
      selectedIds: selectedIds ?? this.selectedIds,
      sensitivity: sensitivity ?? this.sensitivity,
    );
  }
}

/// Relation property (reference to another UnifiedObject).
@JsonSerializable(explicitToJson: true)
class RelationProperty extends PropertyValue {
  final String? targetTypeId;
  final String? targetObjectId;
  @override
  final SensitivityLevel sensitivity;

  const RelationProperty({
    this.targetTypeId,
    this.targetObjectId,
    this.sensitivity = SensitivityLevel.public,
  });

  factory RelationProperty.fromJson(Map<String, dynamic> json) =>
      _$RelationPropertyFromJson(json);

  Map<String, dynamic> toJson() => _$RelationPropertyToJson(this);

  RelationProperty copyWith({
    String? targetTypeId,
    String? targetObjectId,
    SensitivityLevel? sensitivity,
  }) {
    return RelationProperty(
      targetTypeId: targetTypeId ?? this.targetTypeId,
      targetObjectId: targetObjectId ?? this.targetObjectId,
      sensitivity: sensitivity ?? this.sensitivity,
    );
  }
}

/// URL property.
@JsonSerializable(explicitToJson: true)
class UrlProperty extends PropertyValue {
  final String? url;
  @override
  final SensitivityLevel sensitivity;

  const UrlProperty({
    this.url,
    this.sensitivity = SensitivityLevel.public,
  });

  factory UrlProperty.fromJson(Map<String, dynamic> json) =>
      _$UrlPropertyFromJson(json);

  Map<String, dynamic> toJson() => _$UrlPropertyToJson(this);

  UrlProperty copyWith({
    String? url,
    SensitivityLevel? sensitivity,
  }) {
    return UrlProperty(
      url: url ?? this.url,
      sensitivity: sensitivity ?? this.sensitivity,
    );
  }
}

// =============================================================================
// JSON Converter for PropertyValue
// =============================================================================

class PropertyValueConverter
    implements JsonConverter<PropertyValue, Map<String, dynamic>> {
  const PropertyValueConverter();

  @override
  PropertyValue fromJson(Map<String, dynamic> json) {
    final type = json['type'] as String;
    switch (type) {
      case 'text':
        return TextProperty.fromJson(json);
      case 'number':
        return NumberProperty.fromJson(json);
      case 'date':
        return DateProperty.fromJson(json);
      case 'checkbox':
        return CheckboxProperty.fromJson(json);
      case 'select':
        return SelectProperty.fromJson(json);
      case 'multiSelect':
        return MultiSelectProperty.fromJson(json);
      case 'relation':
        return RelationProperty.fromJson(json);
      case 'url':
        return UrlProperty.fromJson(json);
      default:
        throw ArgumentError('Unknown PropertyValue type: $type');
    }
  }

  @override
  Map<String, dynamic> toJson(PropertyValue object) {
    return switch (object) {
      TextProperty() => {'type': 'text', ...object.toJson()},
      NumberProperty() => {'type': 'number', ...object.toJson()},
      DateProperty() => {'type': 'date', ...object.toJson()},
      CheckboxProperty() => {'type': 'checkbox', ...object.toJson()},
      SelectProperty() => {'type': 'select', ...object.toJson()},
      MultiSelectProperty() => {'type': 'multiSelect', ...object.toJson()},
      RelationProperty() => {'type': 'relation', ...object.toJson()},
      UrlProperty() => {'type': 'url', ...object.toJson()},
    };
  }
}

// =============================================================================
// UnifiedObject & UnifiedObjectData
// =============================================================================

/// The single unit of data in SoloSoul. Everything is a UnifiedObject.
///
/// There is no fixed hierarchy (page/section/item). Any object may be a parent
/// or child of any other object via [parentId] and [childrenIds].
@JsonSerializable(explicitToJson: true)
class UnifiedObject with FormattableEntry implements IdentifiableItem {
  @override
  final String id;

  /// Reference to [ObjectTypeDefinition.id]. Built-in types use hard-coded IDs
  /// such as "page", "collection", "note", "task".
  final String? typeId;

  final String name;
  final String iconName;

  /// Parent object ID. Null means root-level object.
  final String? parentId;

  /// Ordered list of child object IDs. This makes tree rendering O(k)
  /// instead of O(n) scan over all objects.
  final List<String> childrenIds;

  @PropertyValueConverter()
  final Map<String, PropertyValue> properties;

  final bool isDeleted;
  final DateTime? deletedAt;
  final int createdAt;
  final int updatedAt;

  const UnifiedObject({
    required this.id,
    this.typeId,
    required this.name,
    this.iconName = 'folder',
    this.parentId,
    this.childrenIds = const [],
    this.properties = const {},
    this.isDeleted = false,
    this.deletedAt,
    required this.createdAt,
    required this.updatedAt,
  });

  @override
  String get entryType => 'UnifiedObject';

  @override
  Map<String, dynamic> toMap() => {
        'id': id,
        'typeId': typeId,
        'name': name,
        'iconName': iconName,
        'parentId': parentId,
        'childrenIds': childrenIds,
        'isDeleted': isDeleted,
        'createdAt': createdAt,
        'updatedAt': updatedAt,
      };

  factory UnifiedObject.fromJson(Map<String, dynamic> json) =>
      _$UnifiedObjectFromJson(json);

  Map<String, dynamic> toJson() => _$UnifiedObjectToJson(this);

  UnifiedObject copyWith({
    String? id,
    String? typeId,
    String? name,
    String? iconName,
    String? parentId,
    List<String>? childrenIds,
    Map<String, PropertyValue>? properties,
    bool? isDeleted,
    DateTime? deletedAt,
    int? createdAt,
    int? updatedAt,
  }) {
    return UnifiedObject(
      id: id ?? this.id,
      typeId: typeId ?? this.typeId,
      name: name ?? this.name,
      iconName: iconName ?? this.iconName,
      parentId: parentId ?? this.parentId,
      childrenIds: childrenIds ?? this.childrenIds,
      properties: properties ?? this.properties,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: deletedAt ?? this.deletedAt,
      createdAt: createdAt ?? this.createdAt,
      updatedAt: updatedAt ?? this.updatedAt,
    );
  }
}

/// Container for all unified objects and custom type definitions.
@JsonSerializable(explicitToJson: true)
class UnifiedObjectData {
  final List<UnifiedObject> objects;

  /// User-defined object types. Built-in types are not stored here;
  /// they are provided by [ObjectTypeRegistry] in the service layer.
  final List<ObjectTypeDefinition> customTypes;

  const UnifiedObjectData({
    this.objects = const [],
    this.customTypes = const [],
  });

  factory UnifiedObjectData.fromJson(Map<String, dynamic> json) =>
      _$UnifiedObjectDataFromJson(json);

  Map<String, dynamic> toJson() => _$UnifiedObjectDataToJson(this);

  /// Fixes the mismatch between generated fromJson (expects 'custom_types')
  /// and toJson (outputs 'customTypes') by normalizing the key.
  factory UnifiedObjectData.fromJsonCompat(Map<String, dynamic> json) {
    final normalized = Map<String, dynamic>.from(json);
    if (normalized.containsKey('customTypes') && !normalized.containsKey('custom_types')) {
      normalized['custom_types'] = normalized.remove('customTypes');
    }
    return _$UnifiedObjectDataFromJson(normalized);
  }

  UnifiedObjectData copyWith({
    List<UnifiedObject>? objects,
    List<ObjectTypeDefinition>? customTypes,
  }) {
    return UnifiedObjectData(
      objects: objects ?? this.objects,
      customTypes: customTypes ?? this.customTypes,
    );
  }
}
