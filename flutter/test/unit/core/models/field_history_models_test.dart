import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/field_history_models.dart';

void main() {
  group('HistoryChangeItem', () {
    test('stores all fields', () {
      final now = DateTime(2024, 6, 15);
      final item = HistoryChangeItem(
        itemId: 'item-1',
        fieldId: 'field-1',
        values: {'name': 'John'},
        timestamp: now,
      );
      expect(item.itemId, 'item-1');
      expect(item.fieldId, 'field-1');
      expect(item.values, {'name': 'John'});
      expect(item.timestamp, now);
    });
  });

  group('FieldHistoryEntry', () {
    test('getValue returns value for known field', () {
      final entry = FieldHistoryEntry(
        values: {'email': 'test@example.com', 'phone': '123'},
        timestamp: DateTime(2024),
      );
      expect(entry.getValue('email'), 'test@example.com');
      expect(entry.getValue('phone'), '123');
    });

    test('getValue returns null for unknown field', () {
      final entry = FieldHistoryEntry(
        values: {'email': 'test@example.com'},
        timestamp: DateTime(2024),
      );
      expect(entry.getValue('unknown'), isNull);
    });
  });

  group('FieldHistory', () {
    test('copyWith no changes', () {
      final history = FieldHistory(
        fieldId: 'f1',
        itemId: 'i1',
        entries: [],
      );
      final copy = history.copyWith();
      expect(copy.fieldId, 'f1');
      expect(copy.itemId, 'i1');
      expect(copy.entries, isEmpty);
    });

    test('copyWith changes', () {
      final history = FieldHistory(
        fieldId: 'f1',
        itemId: 'i1',
        entries: [],
      );
      final copy = history.copyWith(fieldId: 'f2');
      expect(copy.fieldId, 'f2');
      expect(copy.itemId, 'i1');
    });
  });

  group('FormHistories', () {
    test('default constructor has empty histories', () {
      final form = FormHistories();
      expect(form.histories, isEmpty);
    });

    test('getHistory returns null for missing item/field', () {
      final form = FormHistories();
      expect(form.getHistory('item-1', 'field-1'), isNull);
    });

    test('getItemHistories returns empty map for missing item', () {
      final form = FormHistories();
      expect(form.getItemHistories('item-1'), isEmpty);
    });

    test('addEntry creates new history for new item/field', () {
      final form = FormHistories();
      final updated = form.addEntry('item-1', 'field-1', 'value1');

      final history = updated.getHistory('item-1', 'field-1');
      expect(history, isNotNull);
      expect(history!.fieldId, 'field-1');
      expect(history.itemId, 'item-1');
      expect(history.entries, hasLength(1));
      expect(history.entries.first.values, {'field-1': 'value1'});
    });

    test('addEntry appends to existing history', () {
      var form = FormHistories();
      form = form.addEntry('item-1', 'field-1', 'value1');
      form = form.addEntry('item-1', 'field-1', 'value2');

      final history = form.getHistory('item-1', 'field-1');
      expect(history!.entries, hasLength(2));
      expect(history.entries.last.values, {'field-1': 'value2'});
    });

    test('addEntry does not mutate original', () {
      final form = FormHistories();
      final updated = form.addEntry('item-1', 'field-1', 'value1');

      expect(form.histories, isEmpty);
      expect(updated.histories, isNotEmpty);
    });

    test('addSnapshot creates history with full values', () {
      final form = FormHistories();
      final updated = form.addSnapshot('item-1', 'field-1', {
        'name': 'John',
        'email': 'john@test.com',
      });

      final history = updated.getHistory('item-1', 'field-1');
      expect(history!.entries, hasLength(1));
      expect(history.entries.first.values, {
        'name': 'John',
        'email': 'john@test.com',
      });
    });

    test('addSnapshot appends to existing history', () {
      var form = FormHistories();
      form = form.addSnapshot('item-1', 'field-1', {'name': 'John'});
      form = form.addSnapshot('item-1', 'field-1', {'name': 'Jane'});

      final history = form.getHistory('item-1', 'field-1');
      expect(history!.entries, hasLength(2));
    });

    test('getItemHistories returns all histories for an item', () {
      var form = FormHistories();
      form = form.addEntry('item-1', 'field-1', 'v1');
      form = form.addEntry('item-1', 'field-2', 'v2');
      form = form.addEntry('item-2', 'field-1', 'v3');

      final item1Histories = form.getItemHistories('item-1');
      expect(item1Histories, hasLength(2));
      expect(item1Histories.containsKey('field-1'), isTrue);
      expect(item1Histories.containsKey('field-2'), isTrue);
    });
  });
}
