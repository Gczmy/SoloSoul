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
  createdAt: json['createdAt'] as String,
  updatedAt: json['updatedAt'] as String,
  version: (json['version'] as num).toInt(),
);

Map<String, dynamic> _$BridgeProfileSummaryToJson(
  BridgeProfileSummary instance,
) => <String, dynamic>{
  'id': instance.id,
  'name': instance.name,
  'createdAt': instance.createdAt,
  'updatedAt': instance.updatedAt,
  'version': instance.version,
};
