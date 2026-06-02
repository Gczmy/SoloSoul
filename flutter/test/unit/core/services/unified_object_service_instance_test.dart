import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';

void main() {
  group('UnifiedObjectService instance methods', () {
    late UnifiedObjectService service;

    setUp(() {
      service = UnifiedObjectService.instance;
    });

    group('createObject', () {
      test('creates object with required name', () {
        final obj = service.createObject(name: 'Test');
        expect(obj.name, 'Test');
        expect(obj.id, isNotEmpty);
        expect(obj.typeId, isNull);
        expect(obj.iconName, 'note'); // note is default type icon
        expect(obj.isDeleted, false);
        expect(obj.createdAt, isPositive);
        expect(obj.updatedAt, isPositive);
      });

      test('creates object with all optional fields', () {
        final obj = service.createObject(
          name: 'All Fields',
          typeId: 'task',
          parentId: 'parent1',
          iconName: 'star',
          properties: const {'done': CheckboxProperty(checked: true)},
          propertyLabels: const {'done': 'Done'},
          semanticTypes: const {'done': 'status'},
          propertyOrder: const ['done'],
          childrenIds: const ['child1'],
        );
        expect(obj.name, 'All Fields');
        expect(obj.typeId, 'task');
        expect(obj.parentId, 'parent1');
        expect(obj.iconName, 'star');
        expect(obj.properties, hasLength(1));
        expect(obj.propertyLabels, hasLength(1));
        expect(obj.semanticTypes, hasLength(1));
        expect(obj.propertyOrder, equals(['done']));
        expect(obj.childrenIds, equals(['child1']));
      });

      test('falls back to type icon when no iconName provided', () {
        final obj = service.createObject(name: 'Page', typeId: 'page');
        expect(obj.iconName, 'article'); // page type icon
      });

      test('falls back to folder icon for unknown type', () {
        final obj = service.createObject(name: 'Unknown', typeId: 'xyz');
        expect(obj.iconName, 'folder');
      });
    });

    group('updateObject', () {
      test('updates name', () {
        final obj = _makeObject(id: '1', name: 'Old');
        final updated = service.updateObject(obj, name: 'New');
        expect(updated.name, 'New');
        expect(updated.id, '1');
        expect(updated.updatedAt, greaterThanOrEqualTo(obj.updatedAt));
      });

      test('updates typeId', () {
        final obj = _makeObject(id: '1', name: 'A');
        final updated = service.updateObject(obj, typeId: 'task');
        expect(updated.typeId, 'task');
      });

      test('updates iconName', () {
        final obj = _makeObject(id: '1', name: 'A');
        final updated = service.updateObject(obj, iconName: 'star');
        expect(updated.iconName, 'star');
      });

      test('updates parentId', () {
        final obj = _makeObject(id: '1', name: 'A');
        final updated = service.updateObject(obj, parentId: 'p1');
        expect(updated.parentId, 'p1');
      });

      test('sets parentId to null explicitly', () {
        final obj = _makeObject(id: '1', name: 'A', parentId: 'p1');
        final updated = service.updateObject(
          obj,
          parentId: null,
        );
        expect(updated.parentId, isNull);
      });

      test('updates properties', () {
        final obj = _makeObject(id: '1', name: 'A');
        final updated = service.updateObject(
          obj,
          properties: const {'content': TextProperty(text: 'hello')},
        );
        expect(updated.properties, hasLength(1));
      });

      test('updates childrenIds', () {
        final obj = _makeObject(id: '1', name: 'A');
        final updated = service.updateObject(
          obj,
          childrenIds: const ['c1', 'c2'],
        );
        expect(updated.childrenIds, equals(['c1', 'c2']));
      });

      test('updates attachments', () {
        final obj = _makeObject(id: '1', name: 'A');
        const att = Attachment(
          id: 'att1',
          fileId: 'f1',
          fileName: 'doc.pdf',
          mimeType: 'application/pdf',
          size: 100,
          createdAt: 1,
        );
        final updated = service.updateObject(obj, attachments: [att]);
        expect(updated.attachments, hasLength(1));
        expect(updated.attachments.first.fileId, 'f1');
      });

      test('preserves unchanged fields', () {
        final obj = _makeObject(id: '1', name: 'A', typeId: 'note');
        final updated = service.updateObject(obj);
        expect(updated.name, 'A');
        expect(updated.typeId, 'note');
      });
    });

    group('deleteObject', () {
      test('marks object as deleted', () {
        final obj = _makeObject(id: '1', name: 'A');
        final deleted = service.deleteObject(obj);
        expect(deleted.isDeleted, true);
        expect(deleted.deletedAt, isNotNull);
        expect(deleted.updatedAt, greaterThanOrEqualTo(obj.updatedAt));
      });
    });

    group('restoreObject', () {
      test('restores deleted object', () {
        final obj = _makeObject(id: '1', name: 'A', isDeleted: true, deletedAt: DateTime.now());
        final restored = service.restoreObject(obj);
        expect(restored.isDeleted, false);
        expect(restored.deletedAt, isNull);
      });

      test('updates timestamp on restore', () {
        final obj = _makeObject(id: '1', name: 'A', isDeleted: true, deletedAt: DateTime.now());
        final restored = service.restoreObject(obj);
        expect(restored.updatedAt, greaterThanOrEqualTo(obj.updatedAt));
      });
    });

    group('getChildren', () {
      test('returns direct children in order', () {
        final parent = _makeObject(id: 'p', name: 'Parent', childrenIds: const ['c1', 'c2', 'c3']);
        final c1 = _makeObject(id: 'c1', name: 'C1', parentId: 'p');
        final c2 = _makeObject(id: 'c2', name: 'C2', parentId: 'p');
        final c3 = _makeObject(id: 'c3', name: 'C3', parentId: 'p');
        final objects = [parent, c1, c2, c3];

        final children = service.getChildren(objects, 'p');
        expect(children.map((o) => o.id).toList(), equals(['c1', 'c2', 'c3']));
      });

      test('excludes deleted children', () {
        final parent = _makeObject(id: 'p', name: 'P', childrenIds: const ['c1', 'c2']);
        final c1 = _makeObject(id: 'c1', name: 'C1', parentId: 'p');
        final c2 = _makeObject(id: 'c2', name: 'C2', parentId: 'p', isDeleted: true);
        final children = service.getChildren([parent, c1, c2], 'p');
        expect(children, hasLength(1));
        expect(children.first.id, 'c1');
      });

      test('returns empty list for unknown parent', () {
        expect(service.getChildren([], 'x'), isEmpty);
      });

      test('skips missing child IDs', () {
        final parent = _makeObject(id: 'p', name: 'P', childrenIds: const ['c1', 'missing']);
        final c1 = _makeObject(id: 'c1', name: 'C1', parentId: 'p');
        final children = service.getChildren([parent, c1], 'p');
        expect(children, hasLength(1));
        expect(children.first.id, 'c1');
      });
    });

    group('getRootObjects', () {
      test('returns objects without parent', () {
        final r1 = _makeObject(id: 'r1', name: 'R1');
        final r2 = _makeObject(id: 'r2', name: 'R2', parentId: 'p');
        final roots = service.getRootObjects([r1, r2]);
        expect(roots, hasLength(1));
        expect(roots.first.id, 'r1');
      });

      test('excludes deleted root objects', () {
        final r1 = _makeObject(id: 'r1', name: 'R1', isDeleted: true);
        expect(service.getRootObjects([r1]), isEmpty);
      });
    });

    group('getObjectById', () {
      test('finds object by ID', () {
        final obj = _makeObject(id: 'x', name: 'X');
        expect(service.getObjectById([obj], 'x'), isNotNull);
        expect(service.getObjectById([obj], 'x')!.name, 'X');
      });

      test('returns null for unknown ID', () {
        expect(service.getObjectById([], 'x'), isNull);
      });
    });

    group('getDescendantIds', () {
      test('returns all descendant IDs recursively', () {
        // p -> c1 -> g1
        // p -> c2
        final p = _makeObject(id: 'p', name: 'P', childrenIds: const ['c1', 'c2']);
        final c1 = _makeObject(id: 'c1', name: 'C1', parentId: 'p', childrenIds: const ['g1']);
        final c2 = _makeObject(id: 'c2', name: 'C2', parentId: 'p');
        final g1 = _makeObject(id: 'g1', name: 'G1', parentId: 'c1');

        final ids = service.getDescendantIds([p, c1, c2, g1], 'p');
        expect(ids, equals({'c1', 'c2', 'g1'}));
      });

      test('returns empty set for leaf node', () {
        final obj = _makeObject(id: 'x', name: 'X');
        expect(service.getDescendantIds([obj], 'x'), isEmpty);
      });

      test('handles cyclic references gracefully (won\'t infinite loop)', () {
        // a -> b -> a (cycle)
        final a = _makeObject(id: 'a', name: 'A', childrenIds: const ['b']);
        final b = _makeObject(id: 'b', name: 'B', childrenIds: const ['a']);
        final ids = service.getDescendantIds([a, b], 'a');
        expect(ids, equals({'a', 'b'}));
      });
    });

    group('moveObject', () {
      test('moves object to new parent', () {
        final p1 = _makeObject(id: 'p1', name: 'P1', childrenIds: const ['o']);
        final p2 = _makeObject(id: 'p2', name: 'P2');
        final o = _makeObject(id: 'o', name: 'O', parentId: 'p1');

        final result = service.moveObject([p1, p2, o], 'o', 'p2');
        final movedO = service.getObjectById(result, 'o')!;
        final newP1 = service.getObjectById(result, 'p1')!;
        final newP2 = service.getObjectById(result, 'p2')!;

        expect(movedO.parentId, 'p2');
        expect(newP1.childrenIds, isEmpty);
        expect(newP2.childrenIds, equals(['o']));
      });

      test('moves to root (null parent)', () {
        final p1 = _makeObject(id: 'p1', name: 'P1', childrenIds: const ['o']);
        final o = _makeObject(id: 'o', name: 'O', parentId: 'p1');

        final result = service.moveObject([p1, o], 'o', null);
        final movedO = service.getObjectById(result, 'o')!;
        final newP1 = service.getObjectById(result, 'p1')!;

        expect(movedO.parentId, isNull);
        expect(newP1.childrenIds, isEmpty);
      });

      test('prevents moving into own descendant', () {
        final p = _makeObject(id: 'p', name: 'P', childrenIds: const ['c']);
        final c = _makeObject(id: 'c', name: 'C', parentId: 'p', childrenIds: const ['g']);
        final g = _makeObject(id: 'g', name: 'G', parentId: 'c');

        // Try to move p into g
        final result = service.moveObject([p, c, g], 'p', 'g');
        final movedP = service.getObjectById(result, 'p')!;
        expect(movedP.parentId, isNull); // unchanged
      });

      test('prevents moving into self', () {
        final o = _makeObject(id: 'o', name: 'O');
        final result = service.moveObject([o], 'o', 'o');
        expect(service.getObjectById(result, 'o')!.parentId, isNull);
      });

      test('returns unchanged when moving within same parent', () {
        final p = _makeObject(id: 'p', name: 'P', childrenIds: const ['o']);
        final o = _makeObject(id: 'o', name: 'O', parentId: 'p');

        final result = service.moveObject([p, o], 'o', 'p');
        // Should not modify timestamps
        expect(result, hasLength(2));
      });

      test('returns unchanged for unknown object', () {
        final p = _makeObject(id: 'p', name: 'P');
        final result = service.moveObject([p], 'x', 'p');
        expect(result, hasLength(1));
      });
    });

    group('reorderChildren', () {
      test('reorders children within parent', () {
        final p = _makeObject(id: 'p', name: 'P', childrenIds: const ['a', 'b', 'c']);
        final result = service.reorderChildren([p], 'p', 0, 2);
        final updated = service.getObjectById(result, 'p')!;
        expect(updated.childrenIds, equals(['b', 'c', 'a']));
      });

      test('returns unchanged for invalid indices', () {
        final p = _makeObject(id: 'p', name: 'P', childrenIds: const ['a', 'b']);
        expect(service.reorderChildren([p], 'p', -1, 1), equals([p]));
        expect(service.reorderChildren([p], 'p', 0, 5), equals([p]));
      });

      test('returns unchanged for same index', () {
        final p = _makeObject(id: 'p', name: 'P', childrenIds: const ['a', 'b']);
        expect(service.reorderChildren([p], 'p', 0, 0), equals([p]));
      });

      test('returns unchanged for unknown parent', () {
        expect(service.reorderChildren([], 'x', 0, 1), isEmpty);
      });
    });

    group('addChild', () {
      test('adds child to parent', () {
        final p = _makeObject(id: 'p', name: 'P');
        final result = service.addChild([p], 'p', 'c1');
        final updated = service.getObjectById(result, 'p')!;
        expect(updated.childrenIds, equals(['c1']));
      });

      test('prevents duplicate child IDs', () {
        final p = _makeObject(id: 'p', name: 'P', childrenIds: const ['c1']);
        final result = service.addChild([p], 'p', 'c1');
        expect(result, equals([p]));
      });

      test('returns unchanged for unknown parent', () {
        final result = service.addChild([], 'x', 'c');
        expect(result, isEmpty);
      });
    });

    group('removeChild', () {
      test('removes child from parent', () {
        final p = _makeObject(id: 'p', name: 'P', childrenIds: const ['c1', 'c2']);
        final result = service.removeChild([p], 'p', 'c1');
        final updated = service.getObjectById(result, 'p')!;
        expect(updated.childrenIds, equals(['c2']));
      });

      test('returns unchanged for unknown parent', () {
        expect(service.removeChild([], 'x', 'c'), isEmpty);
      });

      test('handles missing child gracefully', () {
        final p = _makeObject(id: 'p', name: 'P', childrenIds: const ['c1']);
        final result = service.removeChild([p], 'p', 'missing');
        final updated = service.getObjectById(result, 'p')!;
        expect(updated.childrenIds, equals(['c1']));
      });
    });

    group('addObject', () {
      test('appends object to list', () {
        final o1 = _makeObject(id: '1', name: 'A');
        final o2 = _makeObject(id: '2', name: 'B');
        final result = service.addObject([o1], o2);
        expect(result, hasLength(2));
        expect(result.last.id, '2');
      });
    });

    group('replaceObject', () {
      test('replaces object by ID', () {
        final o1 = _makeObject(id: '1', name: 'A');
        final o2 = _makeObject(id: '1', name: 'Updated');
        final result = service.replaceObject([o1], o2);
        expect(result, hasLength(1));
        expect(result.first.name, 'Updated');
      });

      test('returns unchanged for unknown ID', () {
        final o1 = _makeObject(id: '1', name: 'A');
        final o2 = _makeObject(id: '2', name: 'B');
        final result = service.replaceObject([o1], o2);
        expect(result, equals([o1]));
      });
    });

    group('removeObject', () {
      test('removes object by ID', () {
        final o1 = _makeObject(id: '1', name: 'A');
        final o2 = _makeObject(id: '2', name: 'B');
        final result = service.removeObject([o1, o2], '1');
        expect(result, hasLength(1));
        expect(result.first.id, '2');
      });

      test('returns unchanged for unknown ID', () {
        final o1 = _makeObject(id: '1', name: 'A');
        final result = service.removeObject([o1], 'x');
        expect(result, equals([o1]));
      });
    });
  });
}

UnifiedObject _makeObject({
  required String id,
  required String name,
  String? typeId,
  String? parentId,
  List<String> childrenIds = const [],
  Map<String, PropertyValue> properties = const {},
  bool isDeleted = false,
  DateTime? deletedAt,
}) {
  final now = DateTime.now().millisecondsSinceEpoch;
  return UnifiedObject(
    id: id,
    name: name,
    typeId: typeId,
    parentId: parentId,
    childrenIds: childrenIds,
    properties: properties,
    createdAt: now,
    updatedAt: now,
    isDeleted: isDeleted,
    deletedAt: deletedAt,
  );
}
