import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';

void main() {
  group('SelectOption', () {
    test('creates with required fields', () {
      const opt = SelectOption(id: 'opt-1', label: 'Option 1', order: 0);
      expect(opt.id, 'opt-1');
      expect(opt.label, 'Option 1');
      expect(opt.order, 0);
    });

    test('copyWith changes', () {
      const opt = SelectOption(id: 'opt-1', label: 'Old', order: 0);
      final copy = opt.copyWith(label: 'New');
      expect(copy.id, 'opt-1');
      expect(copy.label, 'New');
      expect(copy.order, 0);
    });
  });

  group('PropertyDefinition', () {
    test('creates with defaults', () {
      const def = PropertyDefinition(
        id: 'prop-1',
        name: 'Name',
        type: PropertyType.text,
      );
      expect(def.required, isFalse);
      expect(def.order, 0);
      expect(def.config, isNull);
    });

    test('copyWith changes', () {
      const def = PropertyDefinition(
        id: 'prop-1',
        name: 'Name',
        type: PropertyType.text,
      );
      final copy = def.copyWith(name: 'Email', type: PropertyType.url);
      expect(copy.name, 'Email');
      expect(copy.type, PropertyType.url);
      expect(copy.id, 'prop-1');
    });
  });

  group('ObjectTypeDefinition', () {
    test('creates with defaults', () {
      const def = ObjectTypeDefinition(id: 'type-1', name: 'Page');
      expect(def.iconName, 'folder');
      expect(def.description, isNull);
      expect(def.defaultLayout, ObjectLayout.document);
      expect(def.properties, isEmpty);
    });

    test('copyWith changes', () {
      const def = ObjectTypeDefinition(id: 'type-1', name: 'Page');
      final copy = def.copyWith(
        name: 'Collection',
        defaultLayout: ObjectLayout.collection,
      );
      expect(copy.name, 'Collection');
      expect(copy.defaultLayout, ObjectLayout.collection);
      expect(copy.id, 'type-1');
    });
  });

  group('PropertyValue subtypes', () {
    test('TextProperty has default sensitivity public', () {
      const prop = TextProperty(text: 'hello');
      expect(prop.sensitivity, SensitivityLevel.public);
    });

    test('NumberProperty with null value', () {
      const prop = NumberProperty();
      expect(prop.value, isNull);
      expect(prop.sensitivity, SensitivityLevel.public);
    });

    test('DateProperty with null isoDate', () {
      const prop = DateProperty();
      expect(prop.isoDate, isNull);
      expect(prop.includeTime, isFalse);
    });

    test('CheckboxProperty default is unchecked', () {
      const prop = CheckboxProperty();
      expect(prop.checked, isFalse);
    });

    test('SelectProperty with options', () {
      const prop = SelectProperty(
        options: [
          SelectOption(id: 'a', label: 'A', order: 0),
          SelectOption(id: 'b', label: 'B', order: 1),
        ],
        selectedId: 'a',
      );
      expect(prop.options, hasLength(2));
      expect(prop.selectedId, 'a');
    });

    test('MultiSelectProperty default selectedIds is empty', () {
      const prop = MultiSelectProperty(options: []);
      expect(prop.selectedIds, isEmpty);
    });

    test('RelationProperty with target', () {
      const prop = RelationProperty(
        targetTypeId: 'page',
        targetObjectId: 'obj-1',
      );
      expect(prop.targetTypeId, 'page');
      expect(prop.targetObjectId, 'obj-1');
    });

    test('UrlProperty with null url', () {
      const prop = UrlProperty();
      expect(prop.url, isNull);
    });
  });

  group('PropertyValue copyWith', () {
    test('TextProperty copyWith', () {
      const prop = TextProperty(text: 'old');
      final copy = prop.copyWith(text: 'new', sensitivity: SensitivityLevel.critical);
      expect(copy.text, 'new');
      expect(copy.sensitivity, SensitivityLevel.critical);
    });

    test('NumberProperty copyWith', () {
      const prop = NumberProperty(value: 1);
      final copy = prop.copyWith(value: 2);
      expect(copy.value, 2);
    });

    test('CheckboxProperty copyWith', () {
      const prop = CheckboxProperty(checked: false);
      final copy = prop.copyWith(checked: true);
      expect(copy.checked, isTrue);
    });
  });

  group('UnifiedObject', () {
    final now = DateTime.now().millisecondsSinceEpoch;

    test('creates with required fields', () {
      final obj = UnifiedObject(
        id: 'obj-1',
        name: 'Test',
        createdAt: now,
        updatedAt: now,
      );
      expect(obj.id, 'obj-1');
      expect(obj.name, 'Test');
      expect(obj.typeId, isNull);
      expect(obj.iconName, 'folder');
      expect(obj.parentId, isNull);
      expect(obj.childrenIds, isEmpty);
      expect(obj.properties, isEmpty);
      expect(obj.isDeleted, isFalse);
      expect(obj.deletedAt, isNull);
    });

    test('copyWith changes', () {
      final obj = UnifiedObject(
        id: 'obj-1',
        name: 'Old',
        createdAt: now,
        updatedAt: now,
      );
      final copy = obj.copyWith(name: 'New', isDeleted: true);
      expect(copy.id, 'obj-1');
      expect(copy.name, 'New');
      expect(copy.isDeleted, isTrue);
    });

    test('entryType returns UnifiedObject', () {
      final obj = UnifiedObject(
        id: 'obj-1',
        name: 'Test',
        createdAt: now,
        updatedAt: now,
      );
      expect(obj.entryType, 'UnifiedObject');
    });

    test('toMap includes properties as display strings', () {
      final obj = UnifiedObject(
        id: 'obj-1',
        name: 'Test',
        createdAt: now,
        updatedAt: now,
        properties: {
          'Title': const TextProperty(text: 'My Title'),
          'Done': const CheckboxProperty(checked: true),
        },
      );
      final map = obj.toMap();
      expect(map['Title'], 'My Title');
      expect(map['Done'], 'Yes');
      expect(map['id'], 'obj-1');
    });
  });

  group('UnifiedObjectData', () {
    test('default constructor has empty lists', () {
      const data = UnifiedObjectData();
      expect(data.objects, isEmpty);
      expect(data.customTypes, isEmpty);
    });

    test('copyWith changes', () {
      const data = UnifiedObjectData();
      final now = DateTime.now().millisecondsSinceEpoch;
      final copy = data.copyWith(
        objects: [
          UnifiedObject(id: '1', name: 'Test', createdAt: now, updatedAt: now),
        ],
      );
      expect(copy.objects, hasLength(1));
      expect(copy.customTypes, isEmpty);
    });
  });

  group('PropertyValueConverter', () {
    const converter = PropertyValueConverter();

    test('toJson adds type field for TextProperty', () {
      const prop = TextProperty(text: 'hello');
      final json = converter.toJson(prop);
      expect(json['type'], 'text');
      expect(json['text'], 'hello');
    });

    test('toJson adds type field for NumberProperty', () {
      const prop = NumberProperty(value: 42);
      final json = converter.toJson(prop);
      expect(json['type'], 'number');
    });

    test('toJson adds type field for CheckboxProperty', () {
      const prop = CheckboxProperty(checked: true);
      final json = converter.toJson(prop);
      expect(json['type'], 'checkbox');
      expect(json['checked'], true);
    });
  });

  group('ObjectLayout', () {
    test('has expected values', () {
      expect(ObjectLayout.values, hasLength(2));
      expect(ObjectLayout.values, contains(ObjectLayout.document));
      expect(ObjectLayout.values, contains(ObjectLayout.collection));
    });
  });

  group('PropertyType', () {
    test('has expected values', () {
      expect(PropertyType.values, hasLength(8));
      expect(PropertyType.values, contains(PropertyType.text));
      expect(PropertyType.values, contains(PropertyType.number));
      expect(PropertyType.values, contains(PropertyType.date));
      expect(PropertyType.values, contains(PropertyType.checkbox));
      expect(PropertyType.values, contains(PropertyType.select));
      expect(PropertyType.values, contains(PropertyType.multiSelect));
      expect(PropertyType.values, contains(PropertyType.relation));
      expect(PropertyType.values, contains(PropertyType.url));
    });
  });

  group('UnifiedObject semanticTypes', () {
    test('serializes semanticTypes as __semanticTypes', () {
      final now = DateTime.now().millisecondsSinceEpoch;
      final obj = UnifiedObject(
        id: 'section_1',
        name: 'Pet Dog',
        typeId: 'collection',
        properties: {
          'auto_a3f7d2e1': const TextProperty(text: 'Buddy'),
        },
        semanticTypes: {'auto_a3f7d2e1': 'pet.name'},
        createdAt: now,
        updatedAt: now,
      );
      final json = obj.toJson();
      expect(json['__semanticTypes'], {'auto_a3f7d2e1': 'pet.name'});
      expect(json.containsKey('semanticTypes'), isFalse);
    });

    test('deserializes __semanticTypes from JSON', () {
      final json = {
        'id': 'section_1',
        'name': 'Pet Dog',
        'typeId': 'collection',
        'createdAt': 1700000000000,
        'updatedAt': 1700000000000,
        'properties': {
          'auto_a3f7d2e1': {'type': 'text', 'text': 'Buddy'},
        },
        '__semanticTypes': {'auto_a3f7d2e1': 'pet.name'},
      };
      final obj = UnifiedObject.fromJson(json);
      expect(obj.semanticTypes, isNotNull);
      expect(obj.semanticTypes!['auto_a3f7d2e1'], 'pet.name');
    });

    test('copyWith updates semanticTypes', () {
      final now = DateTime.now().millisecondsSinceEpoch;
      final obj = UnifiedObject(
        id: 'section_1',
        name: 'Pet Dog',
        typeId: 'collection',
        properties: {},
        semanticTypes: {'auto_a3f7d2e1': 'pet.name'},
        createdAt: now,
        updatedAt: now,
      );
      final updated = obj.copyWith(
        semanticTypes: {'auto_a3f7d2e1': 'pet.name', 'auto_b2e18f4a': 'pet.breed'},
      );
      expect(updated.semanticTypes!.length, 2);
      expect(updated.semanticTypes!['auto_b2e18f4a'], 'pet.breed');
    });

    test('null semanticTypes serializes as null', () {
      final now = DateTime.now().millisecondsSinceEpoch;
      final obj1 = UnifiedObject(
        id: 'section_1',
        name: 'Pet Dog',
        typeId: 'collection',
        properties: {},
        createdAt: now,
        updatedAt: now,
      );
      // Generated toJson() includes null semanticTypes
      expect(obj1.toJson()['__semanticTypes'], isNull);
    });

    test('includes __semanticTypes when not empty', () {
      final now = DateTime.now().millisecondsSinceEpoch;
      final obj = UnifiedObject(
        id: 'section_1',
        name: 'Pet Dog',
        typeId: 'collection',
        properties: {},
        semanticTypes: {'auto_a3f7d2e1': 'pet.name'},
        createdAt: now,
        updatedAt: now,
      );
      expect(obj.toJson()['__semanticTypes'], {'auto_a3f7d2e1': 'pet.name'});
    });
  });
}
