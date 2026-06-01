import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/presentation/utils/property_value_utils.dart';

void main() {
  group('propValueToString', () {
    test('converts TextProperty', () {
      expect(propValueToString(const TextProperty(text: 'hello')), 'hello');
    });

    test('converts NumberProperty', () {
      expect(propValueToString(NumberProperty(value: 42)), '42.0');
    });

    test('converts NumberProperty with null', () {
      expect(propValueToString(NumberProperty()), '');
    });

    test('converts DateProperty', () {
      expect(propValueToString(DateProperty(isoDate: '2024-01-01')), '2024-01-01');
    });

    test('converts DateProperty with null', () {
      expect(propValueToString(DateProperty()), '');
    });

    test('converts CheckboxProperty true', () {
      expect(
        propValueToString(CheckboxProperty(checked: true)),
        'Yes',
      );
    });

    test('converts CheckboxProperty false', () {
      expect(
        propValueToString(CheckboxProperty(checked: false)),
        'No',
      );
    });

    test('converts CheckboxProperty with custom labels', () {
      expect(
        propValueToString(CheckboxProperty(checked: true), yesLabel: '是', noLabel: '否'),
        '是',
      );
    });

    test('converts SelectProperty', () {
      expect(
        propValueToString(SelectProperty(options: const [], selectedId: 'opt1')),
        'opt1',
      );
    });

    test('converts MultiSelectProperty', () {
      expect(
        propValueToString(MultiSelectProperty(
          options: const [],
          selectedIds: const ['a', 'b'],
        )),
        'a, b',
      );
    });

    test('converts RelationProperty', () {
      expect(propValueToString(RelationProperty(targetObjectId: 'obj1')), 'obj1');
    });

    test('converts UrlProperty', () {
      expect(propValueToString(UrlProperty(url: 'https://example.com')), 'https://example.com');
    });
  });

  group('wrapEveryNChars', () {
    test('returns text with colon when under limit', () {
      expect(wrapEveryNChars('hello', 10), 'hello: ');
    });

    test('wraps text at N chars', () {
      expect(wrapEveryNChars('abcdef', 3), 'abc\ndef: ');
    });

    test('handles exact multiple', () {
      expect(wrapEveryNChars('abcd', 2), 'ab\ncd: ');
    });
  });

  group('objectItemDisplayTitle', () {
    test('uses nameExtractor when provided', () {
      final item = UnifiedObject(
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
      final item = UnifiedObject(
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
      final item = UnifiedObject(
        id: '1',
        name: 'Fallback',
        createdAt: 0,
        updatedAt: 0,
      );
      expect(objectItemDisplayTitle(item), 'Fallback');
    });

    test('falls back to legacy Item Name property', () {
      final item = UnifiedObject(
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
      final item = UnifiedObject(
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
