import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations_en.dart';

void main() {
  group('UnifiedObject.toMap', () {
    test('includes basic fields', () {
      const obj = UnifiedObject(
        id: '1',
        name: 'Test',
        typeId: 'note',
        createdAt: 1000,
        updatedAt: 2000,
      );
      final map = obj.toMap();
      expect(map['id'], '1');
      expect(map['name'], 'Test');
      expect(map['typeId'], 'note');
      expect(map['isDeleted'], false);
    });

    test('includes propertyLabels when present', () {
      const obj = UnifiedObject(
        id: '1',
        name: 'Test',
        propertyLabels: {'name': 'Full Name'},
        createdAt: 0,
        updatedAt: 0,
      );
      final map = obj.toMap();
      expect(map['__propertyLabels'], equals({'name': 'Full Name'}));
    });

    test('excludes propertyLabels when empty', () {
      const obj = UnifiedObject(
        id: '1',
        name: 'Test',
        propertyLabels: {},
        createdAt: 0,
        updatedAt: 0,
      );
      final map = obj.toMap();
      expect(map.containsKey('__propertyLabels'), isFalse);
    });

    test('includes semanticTypes when present', () {
      const obj = UnifiedObject(
        id: '1',
        name: 'Test',
        semanticTypes: {'name': 'personName'},
        createdAt: 0,
        updatedAt: 0,
      );
      final map = obj.toMap();
      expect(map['__semanticTypes'], equals({'name': 'personName'}));
    });

    test('includes propertyOrder when non-empty', () {
      const obj = UnifiedObject(
        id: '1',
        name: 'Test',
        propertyOrder: ['a', 'b'],
        createdAt: 0,
        updatedAt: 0,
      );
      final map = obj.toMap();
      expect(map['propertyOrder'], equals(['a', 'b']));
    });

    test('converts properties to display strings', () {
      const obj = UnifiedObject(
        id: '1',
        name: 'Test',
        properties: {
          'text': TextProperty(text: 'hello'),
          'num': NumberProperty(value: 42.5),
          'date': DateProperty(isoDate: '2024-01-01'),
          'checked': CheckboxProperty(checked: true),
          'selected': SelectProperty(options: [], selectedId: 'opt1'),
          'multi': MultiSelectProperty(options: [], selectedIds: ['a', 'b']),
          'rel': RelationProperty(targetObjectId: 'ref1'),
          'url': UrlProperty(url: 'https://example.com'),
        },
        createdAt: 0,
        updatedAt: 0,
      );
      final map = obj.toMap();
      expect(map['text'], 'hello');
      expect(map['num'], '42.5');
      expect(map['date'], '2024-01-01');
      expect(map['checked'], 'Yes');
      expect(map['selected'], 'opt1');
      expect(map['multi'], 'a, b');
      expect(map['rel'], 'ref1');
      expect(map['url'], 'https://example.com');
    });

    test('uses l10n for checkbox display', () {
      const obj = UnifiedObject(
        id: '1',
        name: 'Test',
        properties: {
          'checked': CheckboxProperty(checked: false),
        },
        createdAt: 0,
        updatedAt: 0,
      );
      final map = obj.toMap();
      expect(map['checked'], 'No');
    });
  });

  group('UnifiedObject.getDisplayLabelFor', () {
    test('returns propertyLabels override when present', () {
      const obj = UnifiedObject(
        id: '1',
        name: 'Test',
        propertyLabels: {'fullName': 'Legal Name'},
        createdAt: 0,
        updatedAt: 0,
      );
      final l10n = AppLocalizationsEn();
      expect(obj.getDisplayLabelFor('fullName', l10n), 'Legal Name');
    });

    test('falls back to translateFieldLabel when no propertyLabels', () {
      const obj = UnifiedObject(
        id: '1',
        name: 'Test',
        createdAt: 0,
        updatedAt: 0,
      );
      final l10n = AppLocalizationsEn();
      expect(obj.getDisplayLabelFor('fullName', l10n), l10n.fieldFullName);
    });
  });

  group('Attachment', () {
    test('copyWith changes fields', () {
      const att = Attachment(
        id: '1',
        fileId: 'f1',
        fileName: 'doc.pdf',
        mimeType: 'application/pdf',
        size: 100,
        createdAt: 0,
      );
      final copy = att.copyWith(fileName: 'new.pdf', size: 200);
      expect(copy.fileName, 'new.pdf');
      expect(copy.size, 200);
      expect(copy.id, '1');
    });

    test('default isDeleted is false', () {
      const att = Attachment(
        id: '1',
        fileId: 'f1',
        fileName: 'doc.pdf',
        mimeType: 'application/pdf',
        size: 100,
        createdAt: 0,
      );
      expect(att.isDeleted, false);
      expect(att.deletedAt, isNull);
    });
  });

  group('UnifiedObjectData', () {
    test('fromJsonCompat normalizes customTypes key', () {
      final data = UnifiedObjectData.fromJsonCompat({
        'objects': [],
        'customTypes': [],
      });
      expect(data.objects, isEmpty);
      expect(data.customTypes, isEmpty);
    });

    test('copyWith updates fields', () {
      const data = UnifiedObjectData(objects: [], customTypes: []);
      const obj = UnifiedObject(
        id: '1',
        name: 'A',
        createdAt: 0,
        updatedAt: 0,
      );
      final copy = data.copyWith(objects: [obj]);
      expect(copy.objects, hasLength(1));
      expect(copy.customTypes, isEmpty);
    });
  });

  group('PropertyValue.copyWith', () {
    test('TextProperty', () {
      const p = TextProperty(text: 'hello');
      final copy = p.copyWith(text: 'world');
      expect(copy.text, 'world');
      expect(copy.sensitivity, SensitivityLevel.public);
    });

    test('NumberProperty', () {
      const p = NumberProperty(value: 10.0);
      final copy = p.copyWith(value: 20.0);
      expect(copy.value, 20.0);
    });

    test('DateProperty', () {
      const p = DateProperty(isoDate: '2024-01-01');
      final copy = p.copyWith(isoDate: '2024-12-31');
      expect(copy.isoDate, '2024-12-31');
    });

    test('CheckboxProperty', () {
      const p = CheckboxProperty(checked: false);
      final copy = p.copyWith(checked: true);
      expect(copy.checked, true);
    });

    test('SelectProperty', () {
      const p = SelectProperty(options: [], selectedId: 'a');
      final copy = p.copyWith(selectedId: 'b');
      expect(copy.selectedId, 'b');
    });

    test('MultiSelectProperty', () {
      const p = MultiSelectProperty(options: [], selectedIds: ['a']);
      final copy = p.copyWith(selectedIds: ['b', 'c']);
      expect(copy.selectedIds, equals(['b', 'c']));
    });

    test('RelationProperty', () {
      const p = RelationProperty(targetObjectId: 'x');
      final copy = p.copyWith(targetObjectId: 'y');
      expect(copy.targetObjectId, 'y');
    });

    test('UrlProperty', () {
      const p = UrlProperty(url: 'http://a.com');
      final copy = p.copyWith(url: 'http://b.com');
      expect(copy.url, 'http://b.com');
    });
  });
}
