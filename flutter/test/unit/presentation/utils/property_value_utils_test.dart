import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/utils/property_value_utils.dart';

void main() {
  group('propValueToString', () {
    test('converts TextProperty', () {
      expect(propValueToString(const TextProperty(text: 'hello')), 'hello');
      expect(propValueToString(const TextProperty(text: '')), '');
    });

    test('converts NumberProperty', () {
      // value is double?, so 42 → 42.0
      expect(propValueToString(const NumberProperty(value: 42)), '42.0');
      expect(propValueToString(const NumberProperty(value: 3.14)), '3.14');
      expect(propValueToString(const NumberProperty()), '');
    });

    test('converts DateProperty', () {
      expect(
        propValueToString(const DateProperty(isoDate: '2024-01-15')),
        '2024-01-15',
      );
      expect(propValueToString(const DateProperty()), '');
    });

    test('converts CheckboxProperty', () {
      expect(propValueToString(const CheckboxProperty(checked: true)), 'Yes');
      expect(propValueToString(const CheckboxProperty(checked: false)), 'No');
    });

    test('converts SelectProperty', () {
      expect(
        propValueToString(
          const SelectProperty(options: [], selectedId: 'opt1'),
        ),
        'opt1',
      );
      expect(
        propValueToString(const SelectProperty(options: [])),
        '',
      );
    });

    test('converts MultiSelectProperty', () {
      expect(
        propValueToString(
          const MultiSelectProperty(
            options: [],
            selectedIds: ['a', 'b', 'c'],
          ),
        ),
        'a, b, c',
      );
      expect(
        propValueToString(const MultiSelectProperty(options: [])),
        '',
      );
    });

    test('converts RelationProperty', () {
      expect(
        propValueToString(
          const RelationProperty(targetObjectId: 'obj-123'),
        ),
        'obj-123',
      );
      expect(propValueToString(const RelationProperty()), '');
    });

    test('converts UrlProperty', () {
      expect(
        propValueToString(
          const UrlProperty(url: 'https://example.com'),
        ),
        'https://example.com',
      );
      expect(propValueToString(const UrlProperty()), '');
    });
  });

  group('wrapEveryNChars', () {
    test('returns text with colon for short text', () {
      expect(wrapEveryNChars('hello', 10), 'hello: ');
    });

    test('wraps long text at specified interval', () {
      final result = wrapEveryNChars('abcdefghij', 5);
      expect(result, 'abcde\nfghij: ');
    });

    test('handles text exactly at boundary', () {
      final result = wrapEveryNChars('abcde', 5);
      expect(result, 'abcde: ');
    });

    test('handles text not evenly divisible', () {
      final result = wrapEveryNChars('abcdefg', 3);
      expect(result, 'abc\ndef\ng: ');
    });
  });

  group('fieldPrefixForTypeId', () {
    test('returns correct prefix for known types', () {
      expect(fieldPrefixForTypeId('profile_identity'), 'identity');
      expect(fieldPrefixForTypeId('profile_contact'), 'contact');
      expect(fieldPrefixForTypeId('profile_id_card'), 'idCard');
      expect(fieldPrefixForTypeId('profile_address'), 'address');
      expect(fieldPrefixForTypeId('travel_passport'), 'passport');
      expect(fieldPrefixForTypeId('travel_visa'), 'visa');
      expect(fieldPrefixForTypeId('travel_history'), 'travel');
      expect(fieldPrefixForTypeId('financial_bank_account'), 'bankAccount');
      expect(fieldPrefixForTypeId('financial_card'), 'card');
      expect(fieldPrefixForTypeId('financial_tax_id'), 'taxId');
      expect(fieldPrefixForTypeId('professional_education'), 'education');
      expect(fieldPrefixForTypeId('professional_employment'), 'employment');
      expect(fieldPrefixForTypeId('professional_skill'), 'skill');
      expect(fieldPrefixForTypeId('professional_language'), 'language');
      expect(fieldPrefixForTypeId('professional_award'), 'award');
    });

    test('returns typeId as-is for unknown types', () {
      expect(fieldPrefixForTypeId('custom_type'), 'custom_type');
      expect(fieldPrefixForTypeId('unknown'), 'unknown');
    });
  });

  group('objectItemDisplayTitle', () {
    final now = DateTime.now().millisecondsSinceEpoch;

    test('returns title from Title property', () {
      final obj = UnifiedObject(
        id: '1',
        name: 'fallback',
        createdAt: now,
        updatedAt: now,
        properties: {
          'Title': const TextProperty(text: 'My Title'),
        },
      );
      expect(objectItemDisplayTitle(obj), 'My Title');
    });

    test('falls back to Item Name property', () {
      final obj = UnifiedObject(
        id: '1',
        name: 'fallback',
        createdAt: now,
        updatedAt: now,
        properties: {
          'Item Name': const TextProperty(text: 'Old Name'),
        },
      );
      expect(objectItemDisplayTitle(obj), 'Old Name');
    });

    test('falls back to object name', () {
      final obj = UnifiedObject(
        id: '1',
        name: 'Object Name',
        createdAt: now,
        updatedAt: now,
      );
      expect(objectItemDisplayTitle(obj), 'Object Name');
    });

    test('uses nameExtractor when provided', () {
      final obj = UnifiedObject(
        id: '1',
        name: 'fallback',
        createdAt: now,
        updatedAt: now,
        properties: {
          'firstName': const TextProperty(text: 'John'),
          'lastName': const TextProperty(text: 'Doe'),
        },
      );
      final result = objectItemDisplayTitle(
        obj,
        nameExtractor: (props) =>
            '${props['firstName']} ${props['lastName']}',
      );
      expect(result, 'John Doe');
    });

    test('falls back when nameExtractor returns empty', () {
      final obj = UnifiedObject(
        id: '1',
        name: 'fallback',
        createdAt: now,
        updatedAt: now,
        properties: {
          'Title': const TextProperty(text: 'Real Title'),
        },
      );
      final result = objectItemDisplayTitle(
        obj,
        nameExtractor: (props) => '',
      );
      expect(result, 'Real Title');
    });
  });
}
