// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'rust_vault_service.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

BridgeProfileSummary _$BridgeProfileSummaryFromJson(
  Map<String, dynamic> json,
) => BridgeProfileSummary(
  id: json['id'] as String,
  name: json['name'] as String,
  createdAt: json['created_at'] as String,
  updatedAt: json['updated_at'] as String,
  version: (json['version'] as num).toInt(),
);

Map<String, dynamic> _$BridgeProfileSummaryToJson(
  BridgeProfileSummary instance,
) => <String, dynamic>{
  'id': instance.id,
  'name': instance.name,
  'created_at': instance.createdAt,
  'updated_at': instance.updatedAt,
  'version': instance.version,
};
