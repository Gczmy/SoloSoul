// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'field_history_models.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

FieldHistoryEntry _$FieldHistoryEntryFromJson(Map<String, dynamic> json) =>
    FieldHistoryEntry(
      values: Map<String, String>.from(json['values'] as Map),
      timestamp: DateTime.parse(json['timestamp'] as String),
    );

Map<String, dynamic> _$FieldHistoryEntryToJson(FieldHistoryEntry instance) =>
    <String, dynamic>{
      'values': instance.values,
      'timestamp': instance.timestamp.toIso8601String(),
    };

FieldHistory _$FieldHistoryFromJson(Map<String, dynamic> json) => FieldHistory(
  fieldId: json['fieldId'] as String,
  itemId: json['itemId'] as String,
  entries: (json['entries'] as List<dynamic>)
      .map((e) => FieldHistoryEntry.fromJson(e as Map<String, dynamic>))
      .toList(),
);

Map<String, dynamic> _$FieldHistoryToJson(FieldHistory instance) =>
    <String, dynamic>{
      'fieldId': instance.fieldId,
      'itemId': instance.itemId,
      'entries': instance.entries.map((e) => e.toJson()).toList(),
    };

FormHistories _$FormHistoriesFromJson(Map<String, dynamic> json) =>
    FormHistories(
      histories: (json['histories'] as Map<String, dynamic>?)?.map(
        (k, e) => MapEntry(
          k,
          (e as Map<String, dynamic>).map(
            (k, e) =>
                MapEntry(k, FieldHistory.fromJson(e as Map<String, dynamic>)),
          ),
        ),
      ),
    );

Map<String, dynamic> _$FormHistoriesToJson(FormHistories instance) =>
    <String, dynamic>{
      'histories': instance.histories.map(
        (k, e) => MapEntry(k, e.map((k, e) => MapEntry(k, e.toJson()))),
      ),
    };
