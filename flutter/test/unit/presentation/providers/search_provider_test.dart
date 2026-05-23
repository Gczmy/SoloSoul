import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/presentation/providers/search_provider.dart';

void main() {
  group('SearchNotifier.executeSearch', () {
    final now = DateTime.now().millisecondsSinceEpoch;

    UnifiedObject makeObject({
      required String id,
      required String name,
      String? typeId,
      String? parentId,
      Map<String, PropertyValue> properties = const {},
      bool isDeleted = false,
    }) {
      return UnifiedObject(
        id: id,
        typeId: typeId,
        name: name,
        iconName: 'folder',
        parentId: parentId,
        childrenIds: const [],
        properties: properties,
        isDeleted: isDeleted,
        deletedAt: null,
        createdAt: now,
        updatedAt: now,
      );
    }

    PropertyValue textProp(String text, SensitivityLevel sensitivity) {
      return TextProperty(text: text, sensitivity: sensitivity);
    }

    test('returns empty list for empty objects', () {
      final results = SearchNotifier.executeSearch(
        [], 'test', true, true, true, true,
      );
      expect(results, isEmpty);
    });

    test('finds object by name', () {
      final objects = [
        makeObject(id: 'o1', name: 'Passport'),
      ];
      final results = SearchNotifier.executeSearch(
        objects, 'pass', true, true, true, true,
      );
      expect(results.length, 1);
      expect(results.first.fieldName, 'name');
      expect(results.first.value, 'Passport');
      expect(results.first.sensitivityLevel, SensitivityLevel.public);
    });

    test('is case insensitive', () {
      final objects = [
        makeObject(id: 'o1', name: 'Bank Account'),
      ];
      final results = SearchNotifier.executeSearch(
        objects, 'BANK', true, true, true, true,
      );
      expect(results.length, 1);
      expect(results.first.value, 'Bank Account');
    });

    test('filters deleted objects', () {
      final objects = [
        makeObject(id: 'o1', name: 'Active'),
        makeObject(id: 'o2', name: 'Deleted', isDeleted: true),
      ];
      final results = SearchNotifier.executeSearch(
        objects, 'deleted', true, true, true, true,
      );
      expect(results, isEmpty);
    });

    test('finds object by typeId', () {
      final objects = [
        makeObject(id: 'o1', name: 'X', typeId: 'credit_card'),
      ];
      final results = SearchNotifier.executeSearch(
        objects, 'credit', true, true, true, true,
      );
      expect(results.length, 1);
      expect(results.first.fieldName, 'typeId');
      expect(results.first.value, 'credit_card');
    });

    test('finds property by value', () {
      final objects = [
        makeObject(
          id: 'o1',
          name: 'Item',
          properties: {'number': textProp('555-1234', SensitivityLevel.public)},
        ),
      ];
      final results = SearchNotifier.executeSearch(
        objects, '1234', true, true, true, true,
      );
      expect(results.length, 1);
      expect(results.first.fieldName, 'number');
      expect(results.first.value, '555-1234');
    });

    test('finds property by key', () {
      final objects = [
        makeObject(
          id: 'o1',
          name: 'Item',
          properties: {'passport_number': textProp('AB123', SensitivityLevel.public)},
        ),
      ];
      final results = SearchNotifier.executeSearch(
        objects, 'passport', true, true, true, true,
      );
      expect(results.length, 1);
      expect(results.first.fieldName, 'passport_number');
    });

    test('skips empty property values', () {
      final objects = [
        makeObject(
          id: 'o1',
          name: 'Item',
          properties: {'empty': textProp('', SensitivityLevel.public)},
        ),
      ];
      final results = SearchNotifier.executeSearch(
        objects, '', true, true, true, true,
      );
      // Empty query should still match name/typeId
      expect(results.any((r) => r.fieldName == 'name'), isTrue);
      // But empty property values are skipped
      expect(results.any((r) => r.fieldName == 'empty'), isFalse);
    });

    test('filters by sensitivity level - public only', () {
      final objects = [
        makeObject(
          id: 'o1',
          name: 'Item',
          properties: {
            'public': textProp('pub', SensitivityLevel.public),
            'internal': textProp('int', SensitivityLevel.internal),
          },
        ),
      ];
      final results = SearchNotifier.executeSearch(
        objects, 'pub', true, false, false, false,
      );
      expect(results.length, 1);
      expect(results.first.fieldName, 'public');
    });

    test('filters by sensitivity level - excludes public when disabled', () {
      final objects = [
        makeObject(id: 'o1', name: 'Test'),
      ];
      final results = SearchNotifier.executeSearch(
        objects, 'test', false, true, true, true,
      );
      // Name matches but public is disabled
      expect(results, isEmpty);
    });

    test('filters by sensitivity level - internal only', () {
      final objects = [
        makeObject(
          id: 'o1',
          name: 'Item',
          properties: {
            'internal': textProp('secret', SensitivityLevel.internal),
          },
        ),
      ];
      final results = SearchNotifier.executeSearch(
        objects, 'secret', false, true, false, false,
      );
      expect(results.length, 1);
      expect(results.first.sensitivityLevel, SensitivityLevel.internal);
    });

    test('filters by sensitivity level - sensitive only', () {
      final objects = [
        makeObject(
          id: 'o1',
          name: 'Item',
          properties: {
            'sensitive': textProp('hidden', SensitivityLevel.sensitive),
          },
        ),
      ];
      final results = SearchNotifier.executeSearch(
        objects, 'hidden', false, false, true, false,
      );
      expect(results.length, 1);
      expect(results.first.sensitivityLevel, SensitivityLevel.sensitive);
    });

    test('filters by sensitivity level - critical only', () {
      final objects = [
        makeObject(
          id: 'o1',
          name: 'Item',
          properties: {
            'critical': textProp('top', SensitivityLevel.critical),
          },
        ),
      ];
      final results = SearchNotifier.executeSearch(
        objects, 'top', false, false, false, true,
      );
      expect(results.length, 1);
      expect(results.first.sensitivityLevel, SensitivityLevel.critical);
    });

    test('resolves section name from parent chain', () {
      final objects = [
        makeObject(id: 'page1', name: 'Travel', typeId: 'page'),
        makeObject(
          id: 'sec1',
          name: 'Documents',
          typeId: 'collection',
          parentId: 'page1',
        ),
        makeObject(
          id: 'item1',
          name: 'Passport',
          typeId: 'item',
          parentId: 'sec1',
        ),
      ];
      final results = SearchNotifier.executeSearch(
        objects, 'passport', true, true, true, true,
      );
      expect(results.length, 1);
      expect(results.first.section, 'Documents');
    });

    test('resolves section name for root-level object', () {
      final objects = [
        makeObject(id: 'o1', name: 'Profile'),
      ];
      final results = SearchNotifier.executeSearch(
        objects, 'profile', true, true, true, true,
      );
      expect(results.first.section, 'Profile');
    });

    test('handles various property types', () {
      final objects = [
        makeObject(
          id: 'o1',
          name: 'Item',
          properties: {
            'num': const NumberProperty(value: 42, sensitivity: SensitivityLevel.public),
            'date': const DateProperty(isoDate: '2024-01-01', sensitivity: SensitivityLevel.public),
            'check': const CheckboxProperty(checked: true, sensitivity: SensitivityLevel.public),
            'select': const SelectProperty(options: [], selectedId: 'opt1', sensitivity: SensitivityLevel.public),
            'multi': const MultiSelectProperty(options: [], selectedIds: ['a', 'b'], sensitivity: SensitivityLevel.public),
            'url': const UrlProperty(url: 'https://x.com', sensitivity: SensitivityLevel.public),
            'relation': const RelationProperty(targetObjectId: 'ref1', sensitivity: SensitivityLevel.public),
          },
        ),
      ];

      expect(
        SearchNotifier.executeSearch(objects, '42', true, true, true, true).length,
        1,
      );
      expect(
        SearchNotifier.executeSearch(objects, '2024', true, true, true, true).length,
        1,
      );
      expect(
        SearchNotifier.executeSearch(objects, 'Yes', true, true, true, true).length,
        1,
      );
      expect(
        SearchNotifier.executeSearch(objects, 'opt1', true, true, true, true).length,
        1,
      );
      expect(
        SearchNotifier.executeSearch(objects, 'a, b', true, true, true, true).length,
        1,
      );
      expect(
        SearchNotifier.executeSearch(objects, 'x.com', true, true, true, true).length,
        1,
      );
      expect(
        SearchNotifier.executeSearch(objects, 'ref1', true, true, true, true).length,
        1,
      );
    });

    test('handles null values in properties', () {
      final objects = [
        makeObject(
          id: 'o1',
          name: 'Item',
          properties: {
            'nullNum': const NumberProperty(value: null, sensitivity: SensitivityLevel.public),
            'nullDate': const DateProperty(isoDate: null, sensitivity: SensitivityLevel.public),
          },
        ),
      ];
      final results = SearchNotifier.executeSearch(
        objects, 'null', true, true, true, true,
      );
      // null values become empty strings and are skipped
      expect(results, isEmpty);
    });

    test('returns multiple results for multiple matches', () {
      final objects = [
        makeObject(id: 'o1', name: 'Bank'),
        makeObject(id: 'o2', name: 'Bank Card'),
      ];
      final results = SearchNotifier.executeSearch(
        objects, 'bank', true, true, true, true,
      );
      expect(results.length, 2);
    });

    test('custom section fallback for broken parent chain', () {
      final objects = [
        makeObject(
          id: 'item1',
          name: 'Orphan',
          typeId: 'item',
          parentId: 'missing',
        ),
      ];
      final results = SearchNotifier.executeSearch(
        objects, 'orphan', true, true, true, true,
      );
      expect(results.length, 1);
      expect(results.first.section, 'Custom');
    });
  });
}
