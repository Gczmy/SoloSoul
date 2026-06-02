import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';

void main() {
  group('ProfileStorageService', () {
    group('migrateIfNeeded', () {
      test('returns profile unchanged when schema version is current', () {
        final profile = ProfileData(
          schemaVersion: ProfileStorageService.kSchemaVersion,
          unifiedObjects: UnifiedObjectData(
            objects: [
              _makeObject(id: '1', name: 'Test', typeId: 'page'),
            ],
          ),
        );

        final migrated = ProfileStorageService.migrateIfNeeded(profile, <String, dynamic>{});

        expect(migrated.schemaVersion, ProfileStorageService.kSchemaVersion);
        expect(migrated.unifiedObjects?.objects.length, 1);
      });

      test('migrates v4 typeIds to v5 preset typeIds', () {
        final profile = ProfileData(
          schemaVersion: 4,
          unifiedObjects: UnifiedObjectData(
            objects: [
              _makeObject(id: '1', name: 'Identity', typeId: 'profile_identity'),
              _makeObject(id: '2', name: 'Bank', typeId: 'financial_bank_account'),
              _makeObject(id: '3', name: 'Custom', typeId: 'my_custom_type'),
            ],
          ),
        );

        final migrated = ProfileStorageService.migrateIfNeeded(profile, <String, dynamic>{});

        expect(migrated.unifiedObjects?.objects[0].typeId, '__preset_identity');
        expect(migrated.unifiedObjects?.objects[1].typeId, '__preset_bank_account');
        expect(migrated.unifiedObjects?.objects[2].typeId, 'my_custom_type');
        expect(migrated.schemaVersion, ProfileStorageService.kSchemaVersion);
      });

      test('clears builtin propertyLabels during v5 to v6 migration', () {
        final profile = ProfileData(
          schemaVersion: 5,
          unifiedObjects: UnifiedObjectData(
            objects: [
              _makeObject(
                id: '1',
                name: 'Identity',
                typeId: '__preset_identity',
                propertyLabels: {'name': 'Name', 'email': 'Email'},
              ),
              _makeObject(
                id: '2',
                name: 'Custom',
                typeId: 'custom_type',
                propertyLabels: {'field': 'Label'},
              ),
            ],
          ),
        );

        final migrated = ProfileStorageService.migrateIfNeeded(profile, <String, dynamic>{});

        // Builtin type labels cleared
        expect(migrated.unifiedObjects?.objects[0].propertyLabels?.isEmpty, isTrue);
        // Custom type labels preserved
        expect(migrated.unifiedObjects?.objects[1].propertyLabels, {'field': 'Label'});
        expect(migrated.schemaVersion, ProfileStorageService.kSchemaVersion);
      });

      test('page/collection/note/task/contact/item builtin types also get labels cleared', () {
        final profile = ProfileData(
          schemaVersion: 5,
          unifiedObjects: UnifiedObjectData(
            objects: [
              _makeObject(
                id: '1',
                name: 'Note',
                typeId: 'note',
                propertyLabels: {'content': 'Content'},
              ),
              _makeObject(
                id: '2',
                name: 'Contact',
                typeId: 'contact',
                propertyLabels: {'phone': 'Phone'},
              ),
            ],
          ),
        );

        final migrated = ProfileStorageService.migrateIfNeeded(profile, <String, dynamic>{});

        expect(migrated.unifiedObjects?.objects[0].propertyLabels?.isEmpty, isTrue);
        expect(migrated.unifiedObjects?.objects[1].propertyLabels?.isEmpty, isTrue);
      });
    });

    group('validateAndRepairProfile', () {
      test('returns original when data is valid', () {
        final profile = ProfileData(
          schemaVersion: 6,
          unifiedObjects: UnifiedObjectData(
            objects: [
              _makeObject(id: '1', name: 'Parent'),
              _makeObject(id: '2', name: 'Child', parentId: '1'),
            ],
          ),
        );

        final (repaired, wasRepaired) = ProfileStorageService.validateAndRepairProfile(profile);

        expect(wasRepaired, isFalse);
        expect(repaired.unifiedObjects?.objects.length, 2);
      });

      test('removes orphaned childrenIds references', () {
        final profile = ProfileData(
          unifiedObjects: UnifiedObjectData(
            objects: [
              _makeObject(id: '1', name: 'Parent', childrenIds: ['2', '3']),
              _makeObject(id: '2', name: 'Real Child'),
            ],
          ),
        );

        final (repaired, wasRepaired) = ProfileStorageService.validateAndRepairProfile(profile);

        expect(wasRepaired, isTrue);
        expect(repaired.unifiedObjects?.objects[0].childrenIds, ['2']);
      });

      test('sets invalid parentId to null', () {
        final profile = ProfileData(
          unifiedObjects: UnifiedObjectData(
            objects: [
              _makeObject(id: '1', name: 'Orphan', parentId: 'nonexistent'),
            ],
          ),
        );

        final (repaired, wasRepaired) = ProfileStorageService.validateAndRepairProfile(profile);

        expect(wasRepaired, isTrue);
        expect(repaired.unifiedObjects?.objects[0].parentId, isNull);
      });

      test('valid parentId is kept intact', () {
        final profile = ProfileData(
          unifiedObjects: UnifiedObjectData(
            objects: [
              _makeObject(id: 'parent', name: 'Parent'),
              _makeObject(id: 'child', name: 'Child', parentId: 'parent'),
            ],
          ),
        );

        final (repaired, wasRepaired) = ProfileStorageService.validateAndRepairProfile(profile);

        expect(wasRepaired, isFalse);
        expect(repaired.unifiedObjects?.objects[1].parentId, 'parent');
      });

      test('removes duplicate objects by ID keeping first', () {
        final profile = ProfileData(
          unifiedObjects: UnifiedObjectData(
            objects: [
              _makeObject(id: 'dup', name: 'First'),
              _makeObject(id: 'dup', name: 'Second'),
              _makeObject(id: 'unique', name: 'Unique'),
            ],
          ),
        );

        final (repaired, wasRepaired) = ProfileStorageService.validateAndRepairProfile(profile);

        expect(wasRepaired, isTrue);
        expect(repaired.unifiedObjects?.objects.length, 2);
        expect(repaired.unifiedObjects?.objects[0].name, 'First');
        expect(repaired.unifiedObjects?.objects[1].name, 'Unique');
      });

      test('handles null unifiedObjects gracefully', () {
        const profile = ProfileData(schemaVersion: 6);

        final (repaired, wasRepaired) = ProfileStorageService.validateAndRepairProfile(profile);

        expect(wasRepaired, isFalse);
        expect(repaired.unifiedObjects, isNull);
      });

      test('handles empty objects list gracefully', () {
        const profile = ProfileData(
          schemaVersion: 6,
          unifiedObjects: UnifiedObjectData(objects: []),
        );

        final (repaired, wasRepaired) = ProfileStorageService.validateAndRepairProfile(profile);

        expect(wasRepaired, isFalse);
        expect(repaired.unifiedObjects?.objects.isEmpty, isTrue);
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
  Map<String, String>? propertyLabels,
}) {
  return UnifiedObject(
    id: id,
    typeId: typeId,
    name: name,
    parentId: parentId,
    childrenIds: childrenIds,
    propertyLabels: propertyLabels,
    createdAt: 0,
    updatedAt: 0,
  );
}
