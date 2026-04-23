import 'package:json_annotation/json_annotation.dart';

part 'field_history_models.g.dart';

/// Single historical value entry for a field.
/// Stores all field values at a point in time.
@JsonSerializable(explicitToJson: true)
class FieldHistoryEntry {
  final Map<String, String> values; // fieldName -> value
  final DateTime timestamp;

  const FieldHistoryEntry({
    required this.values,
    required this.timestamp,
  });

  factory FieldHistoryEntry.fromJson(Map<String, dynamic> json) {
    final valuesJson = json['values'] as Map<String, dynamic>? ?? {};
    final values = valuesJson.map((k, v) => MapEntry(k, v as String? ?? ''));
    return FieldHistoryEntry(
      values: values,
      timestamp: json['timestamp'] != null
          ? DateTime.parse(json['timestamp'] as String)
          : DateTime.now(),
    );
  }

  Map<String, dynamic> toJson() => {
        'values': values,
        'timestamp': timestamp.toIso8601String(),
      };

  String? getValue(String fieldName) => values[fieldName];
}

/// Complete history for a specific field on an item.
@JsonSerializable(explicitToJson: true)
class FieldHistory {
  final String fieldId;
  final String itemId;
  final List<FieldHistoryEntry> entries;

  const FieldHistory({
    required this.fieldId,
    required this.itemId,
    required this.entries,
  });

  factory FieldHistory.fromJson(Map<String, dynamic> json) {
    return FieldHistory(
      fieldId: json['field_id'] as String? ?? '',
      itemId: json['item_id'] as String? ?? '',
      entries: (json['entries'] as List<dynamic>?)
              ?.map((e) => FieldHistoryEntry.fromJson(e as Map<String, dynamic>))
              .toList() ??
          [],
    );
  }

  Map<String, dynamic> toJson() => {
        'field_id': fieldId,
        'item_id': itemId,
        'entries': entries.map((e) => e.toJson()).toList(),
      };

  FieldHistory copyWith({
    String? fieldId,
    String? itemId,
    List<FieldHistoryEntry>? entries,
  }) {
    return FieldHistory(
      fieldId: fieldId ?? this.fieldId,
      itemId: itemId ?? this.itemId,
      entries: entries ?? this.entries,
    );
  }
}

/// All field histories, keyed by item id and field id.
/// Generic - no profile-specific naming.
@JsonSerializable(explicitToJson: true)
class FormHistories {
  final Map<String, Map<String, FieldHistory>>
      histories; // itemId -> fieldId -> FieldHistory

  FormHistories({Map<String, Map<String, FieldHistory>>? histories})
      : histories = histories ?? {};

  factory FormHistories.fromJson(Map<String, dynamic> json) {
    final result = <String, Map<String, FieldHistory>>{};
    final data = json['histories'] as Map<String, dynamic>? ?? {};
    for (final itemEntry in data.entries) {
      final itemId = itemEntry.key;
      final fields = itemEntry.value as Map<String, dynamic>? ?? {};
      result[itemId] = {};
      for (final fieldEntry in fields.entries) {
        final fieldId = fieldEntry.key;
        final historyData = fieldEntry.value as Map<String, dynamic>? ?? {};
        result[itemId]![fieldId] = FieldHistory.fromJson(historyData);
      }
    }
    return FormHistories(histories: result);
  }

  Map<String, dynamic> toJson() => {
        'histories': histories.map(
          (itemId, fields) => MapEntry(
            itemId,
            fields.map((fieldId, h) => MapEntry(fieldId, h.toJson())),
          ),
        ),
      };

  /// Get history for a specific field.
  FieldHistory? getHistory(String itemId, String fieldId) {
    return histories[itemId]?[fieldId];
  }

  /// Get all histories for an item.
  Map<String, FieldHistory> getItemHistories(String itemId) {
    return histories[itemId] ?? {};
  }

  /// Add a new history entry for a field.
  FormHistories addEntry(String itemId, String fieldId, String value) {
    final entry = FieldHistoryEntry(
        values: {fieldId: value}, timestamp: DateTime.now());
    final newHistories = Map<String, Map<String, FieldHistory>>.from(
      histories.map((k, v) => MapEntry(k, Map<String, FieldHistory>.from(v))),
    );

    newHistories[itemId] ??= {};
    final existing = newHistories[itemId]![fieldId];
    if (existing != null) {
      newHistories[itemId]![fieldId] = existing.copyWith(
        entries: [...existing.entries, entry],
      );
    } else {
      newHistories[itemId]![fieldId] = FieldHistory(
        fieldId: fieldId,
        itemId: itemId,
        entries: [entry],
      );
    }

    return FormHistories(histories: newHistories);
  }

  /// Add a snapshot entry containing all field values at a point in time.
  FormHistories addSnapshot(
      String itemId, String fieldId, Map<String, String> values) {
    final entry = FieldHistoryEntry(values: values, timestamp: DateTime.now());
    final newHistories = Map<String, Map<String, FieldHistory>>.from(
      histories.map((k, v) => MapEntry(k, Map<String, FieldHistory>.from(v))),
    );

    newHistories[itemId] ??= {};
    final existing = newHistories[itemId]![fieldId];
    if (existing != null) {
      newHistories[itemId]![fieldId] = existing.copyWith(
        entries: [...existing.entries, entry],
      );
    } else {
      newHistories[itemId]![fieldId] = FieldHistory(
        fieldId: fieldId,
        itemId: itemId,
        entries: [entry],
      );
    }

    return FormHistories(histories: newHistories);
  }
}
