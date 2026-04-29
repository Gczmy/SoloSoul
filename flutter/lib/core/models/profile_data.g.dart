// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'profile_data.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ProfileData _$ProfileDataFromJson(Map<String, dynamic> json) => ProfileData(
  unifiedObjects: json['unified_objects'] == null
      ? null
      : UnifiedObjectData.fromJson(
          json['unified_objects'] as Map<String, dynamic>,
        ),
  schemaVersion: (json['schema_version'] as num?)?.toInt(),
);

Map<String, dynamic> _$ProfileDataToJson(ProfileData instance) =>
    <String, dynamic>{
      'unified_objects': instance.unifiedObjects?.toJson(),
      'schema_version': instance.schemaVersion,
    };
