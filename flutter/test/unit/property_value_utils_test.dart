import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/presentation/utils/property_value_utils.dart';

void main() {
  group('propValueToString', () {
    test('converts TextProperty', () {
      expect(propValueToString(const TextProperty(text: 'hello')), 'hello');
    });

    test('converts NumberProperty', () {
      expect(propValueToString(const NumberProperty(value: 42)), '42.0');
    });

    test('converts NumberProperty with null', () {
      expect(propValueToString(const NumberProperty()), '');
    });

    test('converts DateProperty', () {
      expect(propValueToString(const DateProperty(isoDate: '2024-01-01')), '2024-01-01');
    });

    test('converts DateProperty with null', () {
      expect(propValueToString(const DateProperty()), '');
    });

    test('converts CheckboxProperty true', () {
      expect(
        propValueToString(const CheckboxProperty(checked: true)),
        'Yes',
      );
    });

    test('converts CheckboxProperty false', () {
      expect(
        propValueToString(const CheckboxProperty(checked: false)),
        'No',
      );
    });

    test('converts CheckboxProperty with custom labels', () {
      expect(
        propValueToString(const CheckboxProperty(checked: true), yesLabel: '是', noLabel: '否'),
        '是',
      );
    });

    test('converts SelectProperty', () {
      expect(
        propValueToString(const SelectProperty(options: [], selectedId: 'opt1')),
        'opt1',
      );
    });

    test('converts MultiSelectProperty', () {
      expect(
        propValueToString(const MultiSelectProperty(
          options: [],
          selectedIds: ['a', 'b'],
        )),
        'a, b',
      );
    });

    test('converts RelationProperty', () {
      expect(propValueToString(const RelationProperty(targetObjectId: 'obj1')), 'obj1');
    });

    test('converts UrlProperty', () {
      expect(propValueToString(const UrlProperty(url: 'https://example.com')), 'https://example.com');
    });
  });

  group('objectItemDisplayTitle', () {
    test('uses nameExtractor when provided', () {
      const item = UnifiedObject(
        id: '1',
        name: 'Fallback',
        createdAt: 0,
        updatedAt: 0,
        properties: {
          'Title': TextProperty(text: 'TitleValue'),
        },
      );
      expect(
        objectItemDisplayTitle(item, nameExtractor: (p) => p['Title']!),
        'TitleValue',
      );
    });

    test('falls back to titlePropertyKey', () {
      const item = UnifiedObject(
        id: '1',
        name: 'Fallback',
        createdAt: 0,
        updatedAt: 0,
        properties: {
          'Title': TextProperty(text: 'My Title'),
        },
      );
      expect(objectItemDisplayTitle(item), 'My Title');
    });

    test('falls back to item.name when no title', () {
      const item = UnifiedObject(
        id: '1',
        name: 'Fallback',
        createdAt: 0,
        updatedAt: 0,
      );
      expect(objectItemDisplayTitle(item), 'Fallback');
    });

    test('falls back to legacy Item Name property', () {
      const item = UnifiedObject(
        id: '1',
        name: 'Fallback',
        createdAt: 0,
        updatedAt: 0,
        properties: {
          'Item Name': TextProperty(text: 'Legacy Name'),
        },
      );
      expect(objectItemDisplayTitle(item), 'Legacy Name');
    });

    test('nameExtractor returning Untitled falls back', () {
      const item = UnifiedObject(
        id: '1',
        name: 'Real Name',
        createdAt: 0,
        updatedAt: 0,
      );
      expect(
        objectItemDisplayTitle(item, nameExtractor: (_) => 'Untitled'),
        'Real Name',
      );
    });
  });
}
