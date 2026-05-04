import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/profile_data.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';

void main() {
  group('ProfileStorageService.migrateIfNeeded', () {
    test('returns same profile when schema is current', () {
      final profile = ProfileData(
        unifiedObjects: const UnifiedObjectData(objects: [], customTypes: []),
        schemaVersion: ProfileStorageService.kSchemaVersion,
      );
      final result = ProfileStorageService.migrateIfNeeded(profile, {});
      expect(result.schemaVersion, ProfileStorageService.kSchemaVersion);
    });

    test('bumps schema version when older', () {
      final profile = ProfileData(
        unifiedObjects: const UnifiedObjectData(objects: [], customTypes: []),
        schemaVersion: 2,
      );
      final result = ProfileStorageService.migrateIfNeeded(profile, {});
      expect(result.schemaVersion, ProfileStorageService.kSchemaVersion);
    });

    test('bumps schema version when null', () {
      final profile = ProfileData(
        unifiedObjects: const UnifiedObjectData(objects: [], customTypes: []),
        schemaVersion: null,
      );
      final result = ProfileStorageService.migrateIfNeeded(profile, {});
      expect(result.schemaVersion, ProfileStorageService.kSchemaVersion);
    });
  });

  group('ProfileStorageService.validateAndRepairProfile', () {
    test('returns original when no objects', () {
      final profile = ProfileData(
        unifiedObjects: const UnifiedObjectData(objects: [], customTypes: []),
        schemaVersion: 4,
      );
      final (result, wasRepaired) = ProfileStorageService.validateAndRepairProfile(profile);
      expect(wasRepaired, isFalse);
      expect(result.unifiedObjects?.objects, isEmpty);
    });

    test('returns original when data is valid', () {
      final profile = ProfileData(
        unifiedObjects: UnifiedObjectData(
          objects: [
            UnifiedObject(
              createdAt: 1,
              updatedAt: 1,
              id: 'parent1',
              name: 'Parent',
              typeId: 'page',
              childrenIds: const ['child1'],
            ),
            UnifiedObject(
              createdAt: 1,
              updatedAt: 1,
              id: 'child1',
              name: 'Child',
              typeId: 'item',
              parentId: 'parent1',
              childrenIds: const [],
            ),
          ],
          customTypes: [],
        ),
        schemaVersion: 4,
      );
      final (result, wasRepaired) = ProfileStorageService.validateAndRepairProfile(profile);
      expect(wasRepaired, isFalse);
      expect(result.unifiedObjects?.objects.length, 2);
    });

    test('removes duplicate objects keeping first occurrence', () {
      final profile = ProfileData(
        unifiedObjects: UnifiedObjectData(
          objects: [
            UnifiedObject(
              createdAt: 1,
              updatedAt: 1,id: 'obj1', name: 'First', typeId: 'item'),
            UnifiedObject(
              createdAt: 1,
              updatedAt: 1,id: 'obj1', name: 'Duplicate', typeId: 'item'),
            UnifiedObject(
              createdAt: 1,
              updatedAt: 1,id: 'obj2', name: 'Second', typeId: 'item'),
          ],
          customTypes: [],
        ),
        schemaVersion: 4,
      );
      final (result, wasRepaired) = ProfileStorageService.validateAndRepairProfile(profile);
      expect(wasRepaired, isTrue);
      expect(result.unifiedObjects?.objects.length, 2);
      expect(result.unifiedObjects?.objects.first.name, 'First');
    });

    test('removes invalid childrenIds references', () {
      final profile = ProfileData(
        unifiedObjects: UnifiedObjectData(
          objects: [
            UnifiedObject(
              createdAt: 1,
              updatedAt: 1,
              id: 'parent1',
              name: 'Parent',
              typeId: 'page',
              childrenIds: const ['child1', 'nonexistent'],
            ),
            UnifiedObject(
              createdAt: 1,
              updatedAt: 1,
              id: 'child1',
              name: 'Child',
              typeId: 'item',
              parentId: 'parent1',
              childrenIds: const [],
            ),
          ],
          customTypes: [],
        ),
        schemaVersion: 4,
      );
      final (result, wasRepaired) = ProfileStorageService.validateAndRepairProfile(profile);
      expect(wasRepaired, isTrue);
      final parent = result.unifiedObjects!.objects.firstWhere((o) => o.id == 'parent1');
      expect(parent.childrenIds, ['child1']);
    });

    test('nulls out parentId when parent no longer exists', () {
      final profile = ProfileData(
        unifiedObjects: UnifiedObjectData(
          objects: [
            UnifiedObject(
              createdAt: 1,
              updatedAt: 1,
              id: 'orphan1',
              name: 'Orphan',
              typeId: 'item',
              parentId: 'deleted_parent',
              childrenIds: const [],
            ),
          ],
          customTypes: [],
        ),
        schemaVersion: 4,
      );
      final (result, wasRepaired) = ProfileStorageService.validateAndRepairProfile(profile);
      expect(wasRepaired, isTrue);
      final orphan = result.unifiedObjects!.objects.first;
      expect(orphan.parentId, isNull);
    });

    test('repairs all issues in combined profile', () {
      final profile = ProfileData(
        unifiedObjects: UnifiedObjectData(
          objects: [
            UnifiedObject(
              createdAt: 1,
              updatedAt: 1,
              id: 'parent1',
              name: 'Parent',
              typeId: 'page',
              childrenIds: const ['child1', 'missing_child'],
            ),
            UnifiedObject(
              createdAt: 1,
              updatedAt: 1,
              id: 'child1',
              name: 'Child',
              typeId: 'item',
              parentId: 'parent1',
              childrenIds: const [],
            ),
            UnifiedObject(
              createdAt: 1,
              updatedAt: 1,
              id: 'child1',
              name: 'Duplicate Child',
              typeId: 'item',
              parentId: 'parent1',
              childrenIds: const [],
            ),
            UnifiedObject(
              createdAt: 1,
              updatedAt: 1,
              id: 'orphan1',
              name: 'Orphan',
              typeId: 'item',
              parentId: 'missing_parent',
              childrenIds: const ['missing_child2'],
            ),
          ],
          customTypes: [],
        ),
        schemaVersion: 4,
      );
      final (result, wasRepaired) = ProfileStorageService.validateAndRepairProfile(profile);
      expect(wasRepaired, isTrue);
      // After dedup: parent1, child1, orphan1 = 3 objects
      expect(result.unifiedObjects?.objects.length, 3);

      final parent = result.unifiedObjects!.objects.firstWhere((o) => o.id == 'parent1');
      expect(parent.childrenIds, ['child1']); // missing_child removed

      final orphan = result.unifiedObjects!.objects.firstWhere((o) => o.id == 'orphan1');
      expect(orphan.parentId, isNull); // missing_parent nulled
      expect(orphan.childrenIds, isEmpty); // missing_child2 removed
    });

    test('kSchemaVersion is 4', () {
      expect(ProfileStorageService.kSchemaVersion, 4);
    });
  });
}
