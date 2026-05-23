import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/models/field_history_models.dart';
import 'package:solosoul_flutter/core/services/field_history_service.dart';

void main() {
  group('FieldHistoriesNotifier', () {
    late ProviderContainer container;
    late FieldHistoriesNotifier notifier;

    setUp(() {
      container = ProviderContainer();
      notifier = container.read(fieldHistoriesProvider.notifier);
    });

    tearDown(() => container.dispose());

    test('initial state is empty', () {
      expect(notifier.state.histories, isEmpty);
      expect(notifier.histories.histories, isEmpty);
    });

    test('allChangesSorted returns empty when no histories', () {
      expect(notifier.allChangesSorted, isEmpty);
    });

    test('allChangesSorted returns changes sorted by timestamp newest first',
        () {
      final older = DateTime(2024, 1, 1);
      final newer = DateTime(2024, 1, 2);
      notifier.state = FormHistories(histories: {
        'item1': {
          'field1': FieldHistory(
            fieldId: 'field1',
            itemId: 'item1',
            entries: [
              FieldHistoryEntry(
                values: {'field1': 'old'},
                timestamp: older,
              ),
              FieldHistoryEntry(
                values: {'field1': 'new'},
                timestamp: newer,
              ),
            ],
          ),
        },
      });
      final changes = notifier.allChangesSorted;
      expect(changes.length, 2);
      expect(changes.first.timestamp, newer);
      expect(changes.last.timestamp, older);
      expect(changes.first.itemId, 'item1');
      expect(changes.first.fieldId, 'field1');
    });

    test('allChangesSorted caches results', () {
      notifier.state = FormHistories(histories: {
        'item1': {
          'field1': FieldHistory(
            fieldId: 'field1',
            itemId: 'item1',
            entries: [
              FieldHistoryEntry(
                values: {'field1': 'v'},
                timestamp: DateTime(2024),
              ),
            ],
          ),
        },
      });
      final first = notifier.allChangesSorted;
      final second = notifier.allChangesSorted;
      expect(identical(first, second), isTrue);
    });

    test('allChangesSorted invalidates cache when state changes', () {
      notifier.state = FormHistories(histories: {
        'item1': {
          'field1': FieldHistory(
            fieldId: 'field1',
            itemId: 'item1',
            entries: [
              FieldHistoryEntry(
                values: {'field1': 'v'},
                timestamp: DateTime(2024),
              ),
            ],
          ),
        },
      });
      final first = notifier.allChangesSorted;
      notifier.state = FormHistories(histories: {});
      final second = notifier.allChangesSorted;
      expect(identical(first, second), isFalse);
      expect(second, isEmpty);
    });

    test('getHistory returns history for existing item and field', () {
      final history = FieldHistory(
        fieldId: 'field1',
        itemId: 'item1',
        entries: [
          FieldHistoryEntry(
            values: {'field1': 'v'},
            timestamp: DateTime(2024),
          ),
        ],
      );
      notifier.state = FormHistories(histories: {
        'item1': {'field1': history},
      });
      final result = notifier.getHistory('item1', 'field1');
      expect(result, isNotNull);
      expect(result!.fieldId, 'field1');
      expect(result.entries.length, 1);
    });

    test('getHistory returns null for missing item', () {
      notifier.state = FormHistories(histories: {});
      final result = notifier.getHistory('missing', 'field');
      expect(result, isNull);
    });

    test('getHistory returns null for missing field', () {
      notifier.state = FormHistories(histories: {
        'item1': {
          'field1': const FieldHistory(
            fieldId: 'field1',
            itemId: 'item1',
            entries: [],
          ),
        },
      });
      final result = notifier.getHistory('item1', 'missing');
      expect(result, isNull);
    });

    test('clear resets state', () {
      notifier.state = FormHistories(histories: {
        'item1': {
          'field1': const FieldHistory(
            fieldId: 'field1',
            itemId: 'item1',
            entries: [],
          ),
        },
      });
      notifier.clear();
      expect(notifier.state.histories, isEmpty);
      expect(notifier.allChangesSorted, isEmpty);
    });
  });
}
