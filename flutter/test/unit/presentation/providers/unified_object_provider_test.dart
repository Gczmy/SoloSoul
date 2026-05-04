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

    UnifiedObject _makeObject({
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
          _makeObject(id: 'page1', name: 'Page', typeId: 'page'),
          _makeObject(
            id: 'sec1',
            name: 'Section',
            typeId: 'collection',
            parentId: 'page1',
          ),
          _makeObject(
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

    test('repairs orphan item with missing parent', () {
      final data = UnifiedObjectData(
        objects: [
          _makeObject(
            id: 'item1',
            name: 'Passport',
            typeId: 'travel_passport',
            parentId: 'missing_section',
          ),
        ],
        customTypes: const [],
      );

      final result = notifier.repairOrphanItems(data);
      // Should create page, section, and reparent item
      expect(result.objects.length, greaterThan(1));
      final item = result.objects.firstWhere((o) => o.id == 'item1');
      expect(item.parentId, isNot('missing_section'));
      expect(item.parentId, isNotNull);
    });

    test('creates missing page for orphan repair', () {
      final data = UnifiedObjectData(
        objects: [
          _makeObject(
            id: 'item1',
            name: 'Credit Card',
            typeId: 'financial_card',
            parentId: 'missing',
          ),
        ],
        customTypes: const [],
      );

      final result = notifier.repairOrphanItems(data);
      // A page should be created
      final pages = result.objects.where((o) => o.typeId == 'page').toList();
      expect(pages, isNotEmpty);
    });

    test('creates missing section for orphan repair', () {
      final data = UnifiedObjectData(
        objects: [
          _makeObject(
            id: 'item1',
            name: 'ID Card',
            typeId: 'profile_id_card',
            parentId: 'missing',
          ),
        ],
        customTypes: const [],
      );

      final result = notifier.repairOrphanItems(data);
      // A collection should be created
      final sections = result.objects.where((o) => o.typeId == 'collection').toList();
      expect(sections, isNotEmpty);
    });

    test('does not modify items with valid parents', () {
      final data = UnifiedObjectData(
        objects: [
          _makeObject(id: 'page1', name: 'Page', typeId: 'page'),
          _makeObject(
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
          _makeObject(
            id: 'page1',
            name: 'Orphan Page',
            typeId: 'page',
            parentId: 'missing',
          ),
          _makeObject(
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
          _makeObject(
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

    test('updates childrenIds on created sections and pages', () {
      final data = UnifiedObjectData(
        objects: [
          _makeObject(
            id: 'item1',
            name: 'Passport',
            typeId: 'travel_passport',
            parentId: 'missing',
          ),
        ],
        customTypes: const [],
      );

      final result = notifier.repairOrphanItems(data);
      // Find the section that now parents item1
      final item = result.objects.firstWhere((o) => o.id == 'item1');
      final section = result.objects.firstWhere((o) => o.id == item.parentId);
      expect(section.childrenIds, contains('item1'));
    });

    test('handles multiple orphans of same type', () {
      final data = UnifiedObjectData(
        objects: [
          _makeObject(
            id: 'item1',
            name: 'Passport A',
            typeId: 'travel_passport',
            parentId: 'missing',
          ),
          _makeObject(
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
      // Both should be reparented to the same section
      expect(items.first.parentId, items.last.parentId);
    });
  });

  group('Derived providers', () {
    test('rootObjectsProvider filters root and non-deleted', () {
      final container = ProviderContainer();
      final objects = [
        UnifiedObject(
          id: 'p1', typeId: 'page', name: 'Page1',
          iconName: 'article', parentId: null,
          childrenIds: const [], properties: const {},
          isDeleted: false, deletedAt: null,
          createdAt: 0, updatedAt: 0,
        ),
        UnifiedObject(
          id: 'c1', typeId: 'collection', name: 'Sec1',
          iconName: 'folder', parentId: 'p1',
          childrenIds: const [], properties: const {},
          isDeleted: false, deletedAt: null,
          createdAt: 0, updatedAt: 0,
        ),
        UnifiedObject(
          id: 'd1', typeId: 'page', name: 'Deleted',
          iconName: 'article', parentId: null,
          childrenIds: const [], properties: const {},
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
        UnifiedObject(
          id: 'a1', typeId: 'item', name: 'Active',
          iconName: 'folder', parentId: 'p1',
          childrenIds: const [], properties: const {},
          isDeleted: false, deletedAt: null,
          createdAt: 0, updatedAt: 0,
        ),
        UnifiedObject(
          id: 'd1', typeId: 'item', name: 'Deleted',
          iconName: 'folder', parentId: 'p1',
          childrenIds: const [], properties: const {},
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
        UnifiedObject(
          id: 'o1', typeId: 'item', name: 'One',
          iconName: 'folder', parentId: null,
          childrenIds: const [], properties: const {},
          isDeleted: false, deletedAt: null,
          createdAt: 0, updatedAt: 0,
        ),
        UnifiedObject(
          id: 'o2', typeId: 'item', name: 'Two',
          iconName: 'folder', parentId: null,
          childrenIds: const [], properties: const {},
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
        UnifiedObject(
          id: 'o1', typeId: 'passport', name: 'P1',
          iconName: 'folder', parentId: null,
          childrenIds: const [], properties: const {},
          isDeleted: false, deletedAt: null,
          createdAt: 0, updatedAt: 0,
        ),
        UnifiedObject(
          id: 'o2', typeId: 'credit_card', name: 'C1',
          iconName: 'folder', parentId: null,
          childrenIds: const [], properties: const {},
          isDeleted: false, deletedAt: null,
          createdAt: 0, updatedAt: 0,
        ),
        UnifiedObject(
          id: 'o3', typeId: 'passport', name: 'P2',
          iconName: 'folder', parentId: null,
          childrenIds: const [], properties: const {},
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
        UnifiedObject(
          id: 'p1', typeId: 'page', name: 'Page',
          iconName: 'article', parentId: null,
          childrenIds: const ['c1', 'i1'],
          properties: const {},
          isDeleted: false, deletedAt: null,
          createdAt: 0, updatedAt: 0,
        ),
        UnifiedObject(
          id: 'c1', typeId: 'collection', name: 'Sec',
          iconName: 'folder', parentId: 'p1',
          childrenIds: const [], properties: const {},
          isDeleted: false, deletedAt: null,
          createdAt: 0, updatedAt: 0,
        ),
        UnifiedObject(
          id: 'i1', typeId: 'item', name: 'Item',
          iconName: 'folder', parentId: 'p1',
          childrenIds: const [], properties: const {},
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
    });
  });
}
