// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'unified_object_model.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

SelectOption _$SelectOptionFromJson(Map<String, dynamic> json) => SelectOption(
  id: json['id'] as String,
  label: json['label'] as String,
  order: (json['order'] as num).toInt(),
);

Map<String, dynamic> _$SelectOptionToJson(SelectOption instance) =>
    <String, dynamic>{
      'id': instance.id,
      'label': instance.label,
      'order': instance.order,
    };

PropertyDefinition _$PropertyDefinitionFromJson(Map<String, dynamic> json) =>
    PropertyDefinition(
      id: json['id'] as String,
      name: json['name'] as String,
      type: $enumDecode(_$PropertyTypeEnumMap, json['type']),
      config: json['config'] as Map<String, dynamic>?,
      required: json['required'] as bool? ?? false,
      order: (json['order'] as num?)?.toInt() ?? 0,
      semanticType: json['semanticType'] as String?,
      isAutoKey: json['isAutoKey'] as bool? ?? false,
    );

Map<String, dynamic> _$PropertyDefinitionToJson(PropertyDefinition instance) =>
    <String, dynamic>{
      'id': instance.id,
      'name': instance.name,
      'type': _$PropertyTypeEnumMap[instance.type]!,
      'config': instance.config,
      'required': instance.required,
      'order': instance.order,
      'semanticType': instance.semanticType,
      'isAutoKey': instance.isAutoKey,
    };

const _$PropertyTypeEnumMap = {
  PropertyType.text: 'text',
  PropertyType.number: 'number',
  PropertyType.date: 'date',
  PropertyType.checkbox: 'checkbox',
  PropertyType.select: 'select',
  PropertyType.multiSelect: 'multiSelect',
  PropertyType.relation: 'relation',
  PropertyType.url: 'url',
};

ObjectTypeDefinition _$ObjectTypeDefinitionFromJson(
  Map<String, dynamic> json,
) => ObjectTypeDefinition(
  id: json['id'] as String,
  name: json['name'] as String,
  iconName: json['iconName'] as String? ?? 'folder',
  description: json['description'] as String?,
  defaultLayout:
      $enumDecodeNullable(_$ObjectLayoutEnumMap, json['defaultLayout']) ??
      ObjectLayout.document,
  properties:
      (json['properties'] as List<dynamic>?)
          ?.map((e) => PropertyDefinition.fromJson(e as Map<String, dynamic>))
          .toList() ??
      const [],
  schemaVersion: (json['schemaVersion'] as num?)?.toInt() ?? 1,
  deprecatedProperties:
      (json['deprecatedProperties'] as List<dynamic>?)
          ?.map((e) => e as String)
          .toList() ??
      const [],
  titlePropertyKey: json['titlePropertyKey'] as String?,
);

Map<String, dynamic> _$ObjectTypeDefinitionToJson(
  ObjectTypeDefinition instance,
) => <String, dynamic>{
  'id': instance.id,
  'name': instance.name,
  'iconName': instance.iconName,
  'description': instance.description,
  'defaultLayout': _$ObjectLayoutEnumMap[instance.defaultLayout]!,
  'properties': instance.properties.map((e) => e.toJson()).toList(),
  'schemaVersion': instance.schemaVersion,
  'deprecatedProperties': instance.deprecatedProperties,
  'titlePropertyKey': instance.titlePropertyKey,
};

const _$ObjectLayoutEnumMap = {
  ObjectLayout.document: 'document',
  ObjectLayout.collection: 'collection',
};

TextProperty _$TextPropertyFromJson(Map<String, dynamic> json) => TextProperty(
  text: json['text'] as String,
  maxLength: (json['maxLength'] as num?)?.toInt(),
  sensitivity:
      $enumDecodeNullable(_$SensitivityLevelEnumMap, json['sensitivity']) ??
      SensitivityLevel.public,
);

Map<String, dynamic> _$TextPropertyToJson(TextProperty instance) =>
    <String, dynamic>{
      'text': instance.text,
      'maxLength': instance.maxLength,
      'sensitivity': _$SensitivityLevelEnumMap[instance.sensitivity]!,
    };

const _$SensitivityLevelEnumMap = {
  SensitivityLevel.public: 'public',
  SensitivityLevel.internal: 'internal',
  SensitivityLevel.sensitive: 'sensitive',
  SensitivityLevel.critical: 'critical',
};

NumberProperty _$NumberPropertyFromJson(Map<String, dynamic> json) =>
    NumberProperty(
      value: (json['value'] as num?)?.toDouble(),
      decimalPlaces: (json['decimalPlaces'] as num?)?.toInt(),
      sensitivity:
          $enumDecodeNullable(_$SensitivityLevelEnumMap, json['sensitivity']) ??
          SensitivityLevel.public,
    );

Map<String, dynamic> _$NumberPropertyToJson(NumberProperty instance) =>
    <String, dynamic>{
      'value': instance.value,
      'decimalPlaces': instance.decimalPlaces,
      'sensitivity': _$SensitivityLevelEnumMap[instance.sensitivity]!,
    };

DateProperty _$DatePropertyFromJson(Map<String, dynamic> json) => DateProperty(
  isoDate: json['isoDate'] as String?,
  includeTime: json['includeTime'] as bool? ?? false,
  sensitivity:
      $enumDecodeNullable(_$SensitivityLevelEnumMap, json['sensitivity']) ??
      SensitivityLevel.public,
);

Map<String, dynamic> _$DatePropertyToJson(DateProperty instance) =>
    <String, dynamic>{
      'isoDate': instance.isoDate,
      'includeTime': instance.includeTime,
      'sensitivity': _$SensitivityLevelEnumMap[instance.sensitivity]!,
    };

CheckboxProperty _$CheckboxPropertyFromJson(Map<String, dynamic> json) =>
    CheckboxProperty(
      checked: json['checked'] as bool? ?? false,
      sensitivity:
          $enumDecodeNullable(_$SensitivityLevelEnumMap, json['sensitivity']) ??
          SensitivityLevel.public,
    );

Map<String, dynamic> _$CheckboxPropertyToJson(CheckboxProperty instance) =>
    <String, dynamic>{
      'checked': instance.checked,
      'sensitivity': _$SensitivityLevelEnumMap[instance.sensitivity]!,
    };

SelectProperty _$SelectPropertyFromJson(Map<String, dynamic> json) =>
    SelectProperty(
      options: (json['options'] as List<dynamic>)
          .map((e) => SelectOption.fromJson(e as Map<String, dynamic>))
          .toList(),
      selectedId: json['selectedId'] as String?,
      sensitivity:
          $enumDecodeNullable(_$SensitivityLevelEnumMap, json['sensitivity']) ??
          SensitivityLevel.public,
    );

Map<String, dynamic> _$SelectPropertyToJson(SelectProperty instance) =>
    <String, dynamic>{
      'options': instance.options.map((e) => e.toJson()).toList(),
      'selectedId': instance.selectedId,
      'sensitivity': _$SensitivityLevelEnumMap[instance.sensitivity]!,
    };

MultiSelectProperty _$MultiSelectPropertyFromJson(Map<String, dynamic> json) =>
    MultiSelectProperty(
      options: (json['options'] as List<dynamic>)
          .map((e) => SelectOption.fromJson(e as Map<String, dynamic>))
          .toList(),
      selectedIds:
          (json['selectedIds'] as List<dynamic>?)
              ?.map((e) => e as String)
              .toList() ??
          const [],
      sensitivity:
          $enumDecodeNullable(_$SensitivityLevelEnumMap, json['sensitivity']) ??
          SensitivityLevel.public,
    );

Map<String, dynamic> _$MultiSelectPropertyToJson(
  MultiSelectProperty instance,
) => <String, dynamic>{
  'options': instance.options.map((e) => e.toJson()).toList(),
  'selectedIds': instance.selectedIds,
  'sensitivity': _$SensitivityLevelEnumMap[instance.sensitivity]!,
};

RelationProperty _$RelationPropertyFromJson(Map<String, dynamic> json) =>
    RelationProperty(
      targetTypeId: json['targetTypeId'] as String?,
      targetObjectId: json['targetObjectId'] as String?,
      sensitivity:
          $enumDecodeNullable(_$SensitivityLevelEnumMap, json['sensitivity']) ??
          SensitivityLevel.public,
    );

Map<String, dynamic> _$RelationPropertyToJson(RelationProperty instance) =>
    <String, dynamic>{
      'targetTypeId': instance.targetTypeId,
      'targetObjectId': instance.targetObjectId,
      'sensitivity': _$SensitivityLevelEnumMap[instance.sensitivity]!,
    };

UrlProperty _$UrlPropertyFromJson(Map<String, dynamic> json) => UrlProperty(
  url: json['url'] as String?,
  sensitivity:
      $enumDecodeNullable(_$SensitivityLevelEnumMap, json['sensitivity']) ??
      SensitivityLevel.public,
);

Map<String, dynamic> _$UrlPropertyToJson(UrlProperty instance) =>
    <String, dynamic>{
      'url': instance.url,
      'sensitivity': _$SensitivityLevelEnumMap[instance.sensitivity]!,
    };

Attachment _$AttachmentFromJson(Map<String, dynamic> json) => Attachment(
  id: json['id'] as String,
  fileId: json['fileId'] as String,
  fileName: json['fileName'] as String,
  mimeType: json['mimeType'] as String,
  size: (json['size'] as num).toInt(),
  thumbnail: json['thumbnail'] as String?,
  createdAt: (json['createdAt'] as num).toInt(),
  isDeleted: json['isDeleted'] as bool? ?? false,
  deletedAt: (json['deletedAt'] as num?)?.toInt(),
);

Map<String, dynamic> _$AttachmentToJson(Attachment instance) =>
    <String, dynamic>{
      'id': instance.id,
      'fileId': instance.fileId,
      'fileName': instance.fileName,
      'mimeType': instance.mimeType,
      'size': instance.size,
      'thumbnail': instance.thumbnail,
      'createdAt': instance.createdAt,
      'isDeleted': instance.isDeleted,
      'deletedAt': instance.deletedAt,
    };

UnifiedObject _$UnifiedObjectFromJson(Map<String, dynamic> json) =>
    UnifiedObject(
      id: json['id'] as String,
      typeId: json['typeId'] as String?,
      name: json['name'] as String,
      iconName: json['iconName'] as String? ?? 'folder',
      parentId: json['parentId'] as String?,
      childrenIds:
          (json['childrenIds'] as List<dynamic>?)
              ?.map((e) => e as String)
              .toList() ??
          const [],
      properties:
          (json['properties'] as Map<String, dynamic>?)?.map(
            (k, e) => MapEntry(
              k,
              const PropertyValueConverter().fromJson(
                e as Map<String, dynamic>,
              ),
            ),
          ) ??
          const {},
      propertyLabels: (json['propertyLabels'] as Map<String, dynamic>?)?.map(
        (k, e) => MapEntry(k, e as String),
      ),
      semanticTypes: (json['semanticTypes'] as Map<String, dynamic>?)?.map(
        (k, e) => MapEntry(k, e as String),
      ),
      propertyOrder:
          (json['propertyOrder'] as List<dynamic>?)
              ?.map((e) => e as String)
              .toList() ??
          const [],
      attachments:
          (json['attachments'] as List<dynamic>?)
              ?.map((e) => Attachment.fromJson(e as Map<String, dynamic>))
              .toList() ??
          const [],
      isDeleted: json['isDeleted'] as bool? ?? false,
      deletedAt: json['deletedAt'] == null
          ? null
          : DateTime.parse(json['deletedAt'] as String),
      createdAt: (json['createdAt'] as num).toInt(),
      updatedAt: (json['updatedAt'] as num).toInt(),
      schemaVersionWhenSaved: (json['schemaVersionWhenSaved'] as num?)?.toInt(),
    );

Map<String, dynamic> _$UnifiedObjectToJson(UnifiedObject instance) =>
    <String, dynamic>{
      'id': instance.id,
      'typeId': instance.typeId,
      'name': instance.name,
      'iconName': instance.iconName,
      'parentId': instance.parentId,
      'childrenIds': instance.childrenIds,
      'properties': instance.properties.map(
        (k, e) => MapEntry(k, const PropertyValueConverter().toJson(e)),
      ),
      'propertyLabels': instance.propertyLabels,
      'semanticTypes': instance.semanticTypes,
      'propertyOrder': instance.propertyOrder,
      'attachments': instance.attachments.map((e) => e.toJson()).toList(),
      'isDeleted': instance.isDeleted,
      'deletedAt': instance.deletedAt?.toIso8601String(),
      'createdAt': instance.createdAt,
      'updatedAt': instance.updatedAt,
      'schemaVersionWhenSaved': instance.schemaVersionWhenSaved,
    };

UnifiedObjectData _$UnifiedObjectDataFromJson(Map<String, dynamic> json) =>
    UnifiedObjectData(
      objects:
          (json['objects'] as List<dynamic>?)
              ?.map((e) => UnifiedObject.fromJson(e as Map<String, dynamic>))
              .toList() ??
          const [],
      customTypes:
          (json['customTypes'] as List<dynamic>?)
              ?.map(
                (e) => ObjectTypeDefinition.fromJson(e as Map<String, dynamic>),
              )
              .toList() ??
          const [],
    );

Map<String, dynamic> _$UnifiedObjectDataToJson(UnifiedObjectData instance) =>
    <String, dynamic>{
      'objects': instance.objects.map((e) => e.toJson()).toList(),
      'customTypes': instance.customTypes.map((e) => e.toJson()).toList(),
    };
