import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';

void main() {
  group('UnifiedObjectNotifier.pageNameFromId', () {
    late UnifiedObjectNotifier notifier;

    setUp(() {
      final container = ProviderContainer();
      notifier = container.read(unifiedObjectProvider.notifier);
    });

    test('returns Profile for profile page id', () {
      expect(notifier.pageNameFromId(DefaultPageIds.profile), 'Profile');
    });

    test('returns Travel for travel page id', () {
      expect(notifier.pageNameFromId(DefaultPageIds.travel), 'Travel');
    });

    test('returns Financial for financial page id', () {
      expect(notifier.pageNameFromId(DefaultPageIds.financial), 'Financial');
    });

    test('returns Professional for professional page id', () {
      expect(notifier.pageNameFromId(DefaultPageIds.professional), 'Professional');
    });

    test('returns Page for unknown id', () {
      expect(notifier.pageNameFromId('custom_123'), 'Page');
    });
  });

  group('UnifiedObjectNotifier.repairOrphanItems', () {
    late UnifiedObjectNotifier notifier;

    setUp(() {
      final container = ProviderContainer();
      notifier = container.read(unifiedObjectProvider.notifier);
    });

    final now = DateTime.now().millisecondsSinceEpoch;

    UnifiedObject makeObject({
      required String id,
      required String name,
      String? typeId,
      String? parentId,
      List<String> childrenIds = const [],
      Map<String, PropertyValue> properties = const {},
    }) {
      return UnifiedObject(
        id: id,
        typeId: typeId,
        name: name,
        iconName: 'folder',
        parentId: parentId,
        childrenIds: childrenIds,
        properties: properties,
        isDeleted: false,
        deletedAt: null,
        createdAt: now,
        updatedAt: now,
      );
    }

    test('returns data unchanged when no orphans', () {
      final data = UnifiedObjectData(
        objects: [
          makeObject(id: 'page1', name: 'Page', typeId: 'page'),
          makeObject(
            id: 'sec1',
            name: 'Section',
            typeId: 'collection',
            parentId: 'page1',
          ),
          makeObject(
            id: 'item1',
            name: 'Item',
            typeId: 'item',
            parentId: 'sec1',
          ),
        ],
        customTypes: const [],
      );

      final result = notifier.repairOrphanItems(data);
      expect(result.objects.length, 3);
      expect(identical(result, data), isTrue);
    });

    test('repairs orphan item with missing parent by clearing parentId', () {
      final data = UnifiedObjectData(
        objects: [
          makeObject(
            id: 'item1',
            name: 'Passport',
            typeId: 'travel_passport',
            parentId: 'missing_section',
          ),
        ],
        customTypes: const [],
      );

      final result = notifier.repairOrphanItems(data);
      // Target section does not exist — parentId is cleared (becomes root-level)
      expect(result.objects.length, 1);
      final item = result.objects.firstWhere((o) => o.id == 'item1');
      expect(item.parentId, isNull);
    });

    test('does not create missing page for orphan repair', () {
      final data = UnifiedObjectData(
        objects: [
          makeObject(
            id: 'item1',
            name: 'Credit Card',
            typeId: 'financial_card',
            parentId: 'missing',
          ),
        ],
        customTypes: const [],
      );

      final result = notifier.repairOrphanItems(data);
      // No page should be created when target section is missing
      final pages = result.objects.where((o) => o.typeId == 'page').toList();
      expect(pages, isEmpty);
    });

    test('does not create missing section for orphan repair', () {
      final data = UnifiedObjectData(
        objects: [
          makeObject(
            id: 'item1',
            name: 'ID Card',
            typeId: 'profile_id_card',
            parentId: 'missing',
          ),
        ],
        customTypes: const [],
      );

      final result = notifier.repairOrphanItems(data);
      // No collection should be created when target section is missing
      final sections = result.objects.where((o) => o.typeId == 'collection').toList();
      expect(sections, isEmpty);
    });

    test('does not modify items with valid parents', () {
      final data = UnifiedObjectData(
        objects: [
          makeObject(id: 'page1', name: 'Page', typeId: 'page'),
          makeObject(
            id: 'item1',
            name: 'Item',
            typeId: 'item',
            parentId: 'page1',
          ),
        ],
        customTypes: const [],
      );

      final result = notifier.repairOrphanItems(data);
      final item = result.objects.firstWhere((o) => o.id == 'item1');
      expect(item.parentId, 'page1');
    });

    test('skips page and collection objects in orphan detection', () {
      final data = UnifiedObjectData(
        objects: [
          makeObject(
            id: 'page1',
            name: 'Orphan Page',
            typeId: 'page',
            parentId: 'missing',
          ),
          makeObject(
            id: 'sec1',
            name: 'Orphan Section',
            typeId: 'collection',
            parentId: 'missing',
          ),
        ],
        customTypes: const [],
      );

      final result = notifier.repairOrphanItems(data);
      // Pages and collections with missing parents are NOT repaired
      expect(identical(result, data), isTrue);
    });

    test('skips items with null typeId', () {
      final data = UnifiedObjectData(
        objects: [
          makeObject(
            id: 'item1',
            name: 'Unknown',
            typeId: null,
            parentId: 'missing',
          ),
        ],
        customTypes: const [],
      );

      final result = notifier.repairOrphanItems(data);
      expect(identical(result, data), isTrue);
    });

    test('clears parentId when target section is missing', () {
      final data = UnifiedObjectData(
        objects: [
          makeObject(
            id: 'item1',
            name: 'Passport',
            typeId: 'travel_passport',
            parentId: 'missing',
          ),
        ],
        customTypes: const [],
      );

      final result = notifier.repairOrphanItems(data);
      // parentId is cleared since target section does not exist
      final item = result.objects.firstWhere((o) => o.id == 'item1');
      expect(item.parentId, isNull);
    });

    test('handles multiple orphans of same type by clearing parentIds', () {
      final data = UnifiedObjectData(
        objects: [
          makeObject(
            id: 'item1',
            name: 'Passport A',
            typeId: 'travel_passport',
            parentId: 'missing',
          ),
          makeObject(
            id: 'item2',
            name: 'Passport B',
            typeId: 'travel_passport',
            parentId: 'missing',
          ),
        ],
        customTypes: const [],
      );

      final result = notifier.repairOrphanItems(data);
      final items = result.objects.where((o) => o.id.startsWith('item')).toList();
      expect(items.length, 2);
      // Both should have parentId cleared
      expect(items.first.parentId, isNull);
      expect(items.last.parentId, isNull);
    });
  });

  group('Derived providers', () {
    test('rootObjectsProvider filters root and non-deleted', () {
      final container = ProviderContainer();
      final objects = [
        const UnifiedObject(
          id: 'p1', typeId: 'page', name: 'Page1',
          iconName: 'article', parentId: null,
          childrenIds: [], properties: {},
          isDeleted: false, deletedAt: null,
          createdAt: 0, updatedAt: 0,
        ),
        const UnifiedObject(
          id: 'c1', typeId: 'collection', name: 'Sec1',
          iconName: 'folder', parentId: 'p1',
          childrenIds: [], properties: {},
          isDeleted: false, deletedAt: null,
          createdAt: 0, updatedAt: 0,
        ),
        const UnifiedObject(
          id: 'd1', typeId: 'page', name: 'Deleted',
          iconName: 'article', parentId: null,
          childrenIds: [], properties: {},
          isDeleted: true, deletedAt: null,
          createdAt: 0, updatedAt: 0,
        ),
      ];

      final notifier = container.read(unifiedObjectProvider.notifier);
      notifier.state = UnifiedObjectData(objects: objects, customTypes: const []);

      final roots = container.read(rootObjectsProvider);
      expect(roots.length, 1);
      expect(roots.first.id, 'p1');
    });

    test('deletedObjectsProvider returns only deleted', () {
      final container = ProviderContainer();
      final objects = [
        const UnifiedObject(
          id: 'a1', typeId: 'item', name: 'Active',
          iconName: 'folder', parentId: 'p1',
          childrenIds: [], properties: {},
          isDeleted: false, deletedAt: null,
          createdAt: 0, updatedAt: 0,
        ),
        const UnifiedObject(
          id: 'd1', typeId: 'item', name: 'Deleted',
          iconName: 'folder', parentId: 'p1',
          childrenIds: [], properties: {},
          isDeleted: true, deletedAt: null,
          createdAt: 0, updatedAt: 0,
        ),
      ];

      final notifier = container.read(unifiedObjectProvider.notifier);
      notifier.state = UnifiedObjectData(objects: objects, customTypes: const []);

      final deleted = container.read(deletedObjectsProvider);
      expect(deleted.length, 1);
      expect(deleted.first.id, 'd1');
    });

    test('objectByIdProvider returns correct object', () {
      final container = ProviderContainer();
      final objects = [
        const UnifiedObject(
          id: 'o1', typeId: 'item', name: 'One',
          iconName: 'folder', parentId: null,
          childrenIds: [], properties: {},
          isDeleted: false, deletedAt: null,
          createdAt: 0, updatedAt: 0,
        ),
        const UnifiedObject(
          id: 'o2', typeId: 'item', name: 'Two',
          iconName: 'folder', parentId: null,
          childrenIds: [], properties: {},
          isDeleted: false, deletedAt: null,
          createdAt: 0, updatedAt: 0,
        ),
      ];

      final notifier = container.read(unifiedObjectProvider.notifier);
      notifier.state = UnifiedObjectData(objects: objects, customTypes: const []);

      expect(container.read(objectByIdProvider('o1'))?.name, 'One');
      expect(container.read(objectByIdProvider('o2'))?.name, 'Two');
      expect(container.read(objectByIdProvider('missing')), isNull);
    });

    test('objectsByTypeProvider filters by type', () {
      final container = ProviderContainer();
      final objects = [
        const UnifiedObject(
          id: 'o1', typeId: 'passport', name: 'P1',
          iconName: 'folder', parentId: null,
          childrenIds: [], properties: {},
          isDeleted: false, deletedAt: null,
          createdAt: 0, updatedAt: 0,
        ),
        const UnifiedObject(
          id: 'o2', typeId: 'credit_card', name: 'C1',
          iconName: 'folder', parentId: null,
          childrenIds: [], properties: {},
          isDeleted: false, deletedAt: null,
          createdAt: 0, updatedAt: 0,
        ),
        const UnifiedObject(
          id: 'o3', typeId: 'passport', name: 'P2',
          iconName: 'folder', parentId: null,
          childrenIds: [], properties: {},
          isDeleted: true, deletedAt: null,
          createdAt: 0, updatedAt: 0,
        ),
      ];

      final notifier = container.read(unifiedObjectProvider.notifier);
      notifier.state = UnifiedObjectData(objects: objects, customTypes: const []);

      final passports = container.read(objectsByTypeProvider('passport'));
      expect(passports.length, 1);
      expect(passports.first.name, 'P1');
    });

    test('unifiedObjectCacheProvider builds indexes', () {
      final container = ProviderContainer();
      final objects = [
        const UnifiedObject(
          id: 'p1', typeId: 'page', name: 'Page',
          iconName: 'article', parentId: null,
          childrenIds: ['c1', 'i1'],
          properties: {},
          isDeleted: false, deletedAt: null,
          createdAt: 0, updatedAt: 0,
        ),
        const UnifiedObject(
          id: 'c1', typeId: 'collection', name: 'Sec',
          iconName: 'folder', parentId: 'p1',
          childrenIds: [], properties: {},
          isDeleted: false, deletedAt: null,
          createdAt: 0, updatedAt: 0,
        ),
        const UnifiedObject(
          id: 'i1', typeId: 'item', name: 'Item',
          iconName: 'folder', parentId: 'p1',
          childrenIds: [], properties: {},
          isDeleted: false, deletedAt: null,
          createdAt: 0, updatedAt: 0,
        ),
      ];

      final notifier = container.read(unifiedObjectProvider.notifier);
      notifier.state = UnifiedObjectData(objects: objects, customTypes: const []);

      final cache = container.read(unifiedObjectCacheProvider);
      expect(cache.objectById['p1']?.name, 'Page');
      expect(cache.workspaceChildren['p1']?.length, 2);
      expect(cache.itemChildren['p1']?.length, 1);
      expect(cache.rootObjects.length, 1);
      addTearDown(container.dispose);
    });
  });

  group('UnifiedObjectNotifier CRUD operations', () {
    test('createObject adds new object to state', () async {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final notifier = container.read(unifiedObjectProvider.notifier);

      // Fire-and-forget; state updates synchronously before _saveDebounced
      unawaited(notifier.createObject(name: 'New Object'));

      expect(notifier.state.objects, hasLength(1));
      expect(notifier.state.objects.first.name, 'New Object');
    });

    test('createObject with parentId adds child reference', () async {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final notifier = container.read(unifiedObjectProvider.notifier);
      final parent = UnifiedObject(
        id: 'p1', name: 'Parent', typeId: 'page',
        iconName: 'article', createdAt: 0, updatedAt: 0,
      );
      notifier.state = UnifiedObjectData(objects: [parent], customTypes: const []);

      unawaited(notifier.createObject(name: 'Child', parentId: 'p1'));

      expect(notifier.state.objects, hasLength(2));
      final updatedParent = notifier.state.objects.firstWhere((o) => o.id == 'p1');
      expect(updatedParent.childrenIds, hasLength(1));
    });

    test('deleteObject marks object as deleted', () async {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final notifier = container.read(unifiedObjectProvider.notifier);
      final obj = UnifiedObject(
        id: 'o1', name: 'ToDelete', typeId: 'note',
        iconName: 'note', createdAt: 0, updatedAt: 0,
      );
      notifier.state = UnifiedObjectData(objects: [obj], customTypes: const []);

      unawaited(notifier.deleteObject('o1'));

      final deleted = notifier.state.objects.firstWhere((o) => o.id == 'o1');
      expect(deleted.isDeleted, true);
      expect(deleted.deletedAt, isNotNull);
    });

    test('deleteObject removes child reference from parent', () async {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final notifier = container.read(unifiedObjectProvider.notifier);
      final parent = UnifiedObject(
        id: 'p1', name: 'Parent', typeId: 'page',
        iconName: 'article', childrenIds: const ['c1'],
        createdAt: 0, updatedAt: 0,
      );
      final child = UnifiedObject(
        id: 'c1', name: 'Child', typeId: 'note',
        iconName: 'note', parentId: 'p1',
        createdAt: 0, updatedAt: 0,
      );
      notifier.state = UnifiedObjectData(objects: [parent, child], customTypes: const []);

      unawaited(notifier.deleteObject('c1'));

      final updatedParent = notifier.state.objects.firstWhere((o) => o.id == 'p1');
      expect(updatedParent.childrenIds, isEmpty);
    });

    test('restoreObject marks object as not deleted', () async {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final notifier = container.read(unifiedObjectProvider.notifier);
      final obj = UnifiedObject(
        id: 'o1', name: 'Restored', typeId: 'note',
        iconName: 'note', isDeleted: true, deletedAt: DateTime.now(),
        createdAt: 0, updatedAt: 0,
      );
      notifier.state = UnifiedObjectData(objects: [obj], customTypes: const []);

      unawaited(notifier.restoreObject('o1'));

      final restored = notifier.state.objects.firstWhere((o) => o.id == 'o1');
      expect(restored.isDeleted, false);
      expect(restored.deletedAt, isNull);
    });

    test('moveObject updates parentId and childrenIds', () async {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final notifier = container.read(unifiedObjectProvider.notifier);
      final p1 = UnifiedObject(
        id: 'p1', name: 'P1', typeId: 'page',
        iconName: 'article', createdAt: 0, updatedAt: 0,
      );
      final p2 = UnifiedObject(
        id: 'p2', name: 'P2', typeId: 'page',
        iconName: 'article', createdAt: 0, updatedAt: 0,
      );
      final child = UnifiedObject(
        id: 'c1', name: 'Child', typeId: 'note',
        iconName: 'note', parentId: 'p1',
        createdAt: 0, updatedAt: 0,
      );
      notifier.state = UnifiedObjectData(objects: [p1, p2, child], customTypes: const []);

      unawaited(notifier.moveObject('c1', 'p2'));

      final moved = notifier.state.objects.firstWhere((o) => o.id == 'c1');
      expect(moved.parentId, 'p2');
      final oldParent = notifier.state.objects.firstWhere((o) => o.id == 'p1');
      expect(oldParent.childrenIds, isEmpty);
      final newParent = notifier.state.objects.firstWhere((o) => o.id == 'p2');
      expect(newParent.childrenIds, contains('c1'));
    });

    test('reorderChildren reorders within parent', () async {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final notifier = container.read(unifiedObjectProvider.notifier);
      final parent = UnifiedObject(
        id: 'p1', name: 'Parent', typeId: 'page',
        iconName: 'article', childrenIds: const ['a', 'b', 'c'],
        createdAt: 0, updatedAt: 0,
      );
      final a = UnifiedObject(
        id: 'a', name: 'A', typeId: 'note', parentId: 'p1',
        iconName: 'note', createdAt: 0, updatedAt: 0,
      );
      final b = UnifiedObject(
        id: 'b', name: 'B', typeId: 'note', parentId: 'p1',
        iconName: 'note', createdAt: 0, updatedAt: 0,
      );
      final c = UnifiedObject(
        id: 'c', name: 'C', typeId: 'note', parentId: 'p1',
        iconName: 'note', createdAt: 0, updatedAt: 0,
      );
      notifier.state = UnifiedObjectData(objects: [parent, a, b, c], customTypes: const []);

      unawaited(notifier.reorderChildren('p1', 0, 2));

      final updated = notifier.state.objects.firstWhere((o) => o.id == 'p1');
      expect(updated.childrenIds, equals(['b', 'c', 'a']));
    });

    test('updateObject modifies existing object', () async {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final notifier = container.read(unifiedObjectProvider.notifier);
      final obj = UnifiedObject(
        id: 'o1', name: 'Old', typeId: 'note',
        iconName: 'note', createdAt: 0, updatedAt: 0,
      );
      notifier.state = UnifiedObjectData(objects: [obj], customTypes: const []);

      unawaited(notifier.updateObject('o1', name: 'New'));

      final updated = notifier.state.objects.firstWhere((o) => o.id == 'o1');
      expect(updated.name, 'New');
    });

    test('saveCustomType adds new custom type', () async {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final notifier = container.read(unifiedObjectProvider.notifier);

      const typeDef = ObjectTypeDefinition(
        id: 'custom1',
        name: 'Custom Type',
        iconName: 'star',
        defaultLayout: ObjectLayout.document,
      );
      unawaited(notifier.saveCustomType(typeDef));

      expect(notifier.state.customTypes, hasLength(1));
      expect(notifier.state.customTypes.first.id, 'custom1');
    });

    test('deleteCustomType removes custom type', () async {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final notifier = container.read(unifiedObjectProvider.notifier);
      const typeDef = ObjectTypeDefinition(
        id: 'custom1',
        name: 'Custom Type',
        iconName: 'star',
        defaultLayout: ObjectLayout.document,
      );
      notifier.state = UnifiedObjectData(objects: [], customTypes: [typeDef]);

      unawaited(notifier.deleteCustomType('custom1'));

      expect(notifier.state.customTypes, isEmpty);
    });

    test('currentObjects accessor returns state objects', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final notifier = container.read(unifiedObjectProvider.notifier);
      final obj = UnifiedObject(
        id: 'o1', name: 'A', typeId: 'note',
        iconName: 'note', createdAt: 0, updatedAt: 0,
      );
      notifier.state = UnifiedObjectData(objects: [obj], customTypes: const []);

      expect(notifier.currentObjects, hasLength(1));
      expect(notifier.currentObjects.first.id, 'o1');
    });
  });
}
