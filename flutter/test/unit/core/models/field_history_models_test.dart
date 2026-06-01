import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/field_history_models.dart';

void main() {
  group('FieldHistoryEntry', () {
    test('getValue returns value for existing field', () {
      final entry = FieldHistoryEntry(
        values: {'name': 'Alice', 'age': '30'},
        timestamp: DateTime(2025, 1, 1),
      );
      expect(entry.getValue('name'), 'Alice');
      expect(entry.getValue('age'), '30');
    });

    test('getValue returns null for missing field', () {
      final entry = FieldHistoryEntry(
        values: {'name': 'Alice'},
        timestamp: DateTime(2025, 1, 1),
      );
      expect(entry.getValue('missing'), isNull);
    });

    test('round-trip serialization', () {
      final original = FieldHistoryEntry(
        values: {'a': '1', 'b': '2'},
        timestamp: DateTime(2025, 6, 15, 10, 30),
      );
      final json = original.toJson();
      final restored = FieldHistoryEntry.fromJson(json);
      expect(restored.values, original.values);
      expect(restored.timestamp, original.timestamp);
    });
  });

  group('FieldHistory', () {
    test('copyWith updates fields', () {
      final history = FieldHistory(
        fieldId: 'email',
        itemId: 'item1',
        entries: [
          FieldHistoryEntry(values: {'email': 'a@b.com'}, timestamp: DateTime(2025, 1, 1)),
        ],
      );
      final copy = history.copyWith(fieldId: 'phone');
      expect(copy.fieldId, 'phone');
      expect(copy.itemId, 'item1');
      expect(copy.entries, hasLength(1));
    });

    test('copyWith preserves fields when null', () {
      final history = FieldHistory(
        fieldId: 'email',
        itemId: 'item1',
        entries: [],
      );
      final copy = history.copyWith();
      expect(copy.fieldId, 'email');
      expect(copy.itemId, 'item1');
      expect(copy.entries, isEmpty);
    });
  });

  group('FormHistories', () {
    test('getHistory returns existing history', () {
      final histories = FormHistories(histories: {
        'item1': {
          'email': FieldHistory(
            fieldId: 'email',
            itemId: 'item1',
            entries: [],
          ),
        },
      });
      final result = histories.getHistory('item1', 'email');
      expect(result, isNotNull);
      expect(result!.fieldId, 'email');
    });

    test('getHistory returns null for missing item', () {
      final histories = FormHistories();
      expect(histories.getHistory('missing', 'field'), isNull);
    });

    test('getHistory returns null for missing field', () {
      final histories = FormHistories(histories: {
        'item1': {},
      });
      expect(histories.getHistory('item1', 'missing'), isNull);
    });

    test('getItemHistories returns all field histories', () {
      final histories = FormHistories(histories: {
        'item1': {
          'email': FieldHistory(fieldId: 'email', itemId: 'item1', entries: []),
          'phone': FieldHistory(fieldId: 'phone', itemId: 'item1', entries: []),
        },
      });
      final items = histories.getItemHistories('item1');
      expect(items, hasLength(2));
    });

    test('getItemHistories returns empty for missing item', () {
      final histories = FormHistories();
      expect(histories.getItemHistories('missing'), isEmpty);
    });

    group('addEntry', () {
      test('adds first entry for field', () {
        final histories = FormHistories();
        final updated = histories.addEntry('item1', 'email', 'a@b.com');
        final history = updated.getHistory('item1', 'email');
        expect(history, isNotNull);
        expect(history!.entries, hasLength(1));
        expect(history.entries.first.getValue('email'), 'a@b.com');
      });

      test('appends entry to existing field history', () {
        var histories = FormHistories();
        histories = histories.addEntry('item1', 'email', 'a@b.com');
        histories = histories.addEntry('item1', 'email', 'c@d.com');
        final history = histories.getHistory('item1', 'email');
        expect(history!.entries, hasLength(2));
        expect(history.entries[0].getValue('email'), 'a@b.com');
        expect(history.entries[1].getValue('email'), 'c@d.com');
      });

      test('does not mutate original', () {
        final original = FormHistories();
        final updated = original.addEntry('item1', 'email', 'a@b.com');
        expect(original.getHistory('item1', 'email'), isNull);
        expect(updated.getHistory('item1', 'email'), isNotNull);
      });
    });

    group('addSnapshot', () {
      test('adds first snapshot for field', () {
        final histories = FormHistories();
        final updated = histories.addSnapshot('item1', 'profile', {
          'name': 'Alice',
          'age': '30',
        });
        final history = updated.getHistory('item1', 'profile');
        expect(history, isNotNull);
        expect(history!.entries, hasLength(1));
        expect(history.entries.first.values, {'name': 'Alice', 'age': '30'});
      });

      test('appends snapshot to existing field history', () {
        var histories = FormHistories();
        histories = histories.addSnapshot('item1', 'profile', {'name': 'Alice'});
        histories = histories.addSnapshot('item1', 'profile', {'name': 'Bob'});
        final history = histories.getHistory('item1', 'profile');
        expect(history!.entries, hasLength(2));
      });

      test('does not mutate original', () {
        final original = FormHistories();
        final updated = original.addSnapshot('item1', 'profile', {'name': 'Alice'});
        expect(original.getHistory('item1', 'profile'), isNull);
        expect(updated.getHistory('item1', 'profile'), isNotNull);
      });
    });

    test('round-trip serialization with empty histories', () {
      final original = FormHistories();
      final json = original.toJson();
      final restored = FormHistories.fromJson(json);
      expect(restored.histories, isEmpty);
    });

    test('round-trip serialization with entries', () {
      var original = FormHistories();
      original = original.addEntry('item1', 'email', 'test@example.com');
      final json = original.toJson();
      final restored = FormHistories.fromJson(json);
      final history = restored.getHistory('item1', 'email');
      expect(history, isNotNull);
      expect(history!.entries, hasLength(1));
      expect(history.entries.first.getValue('email'), 'test@example.com');
    });
  });
}
