import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/profile_data.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';

void main() {
  group('ProfileData', () {
    test('default constructor has null fields', () {
      const data = ProfileData();
      expect(data.unifiedObjects, isNull);
      expect(data.schemaVersion, isNull);
    });

    test('creates with values', () {
      const data = ProfileData(
        unifiedObjects: UnifiedObjectData(),
        schemaVersion: 1,
      );
      expect(data.unifiedObjects, isNotNull);
      expect(data.schemaVersion, 1);
    });

    group('collectAllItemIds', () {
      test('returns empty set when unifiedObjects is null', () {
        const data = ProfileData();
        expect(data.collectAllItemIds(), isEmpty);
      });

      test('returns empty set when no objects', () {
        const data = ProfileData(
          unifiedObjects: UnifiedObjectData(),
        );
        expect(data.collectAllItemIds(), isEmpty);
      });

      test('collects all object ids', () {
        final now = DateTime.now().millisecondsSinceEpoch;
        final data = ProfileData(
          unifiedObjects: UnifiedObjectData(
            objects: [
              UnifiedObject(id: 'obj-1', name: 'A', createdAt: now, updatedAt: now),
              UnifiedObject(id: 'obj-2', name: 'B', createdAt: now, updatedAt: now),
              UnifiedObject(id: 'obj-3', name: 'C', createdAt: now, updatedAt: now),
            ],
          ),
        );
        final ids = data.collectAllItemIds();
        expect(ids, {'obj-1', 'obj-2', 'obj-3'});
      });
    });

    group('copyWith', () {
      test('copies with no changes', () {
        const data = ProfileData(schemaVersion: 1);
        final copy = data.copyWith();
        expect(copy.schemaVersion, 1);
        expect(copy.unifiedObjects, isNull);
      });

      test('copies with changes', () {
        const data = ProfileData(schemaVersion: 1);
        final copy = data.copyWith(
          unifiedObjects: const UnifiedObjectData(),
          schemaVersion: 2,
        );
        expect(copy.unifiedObjects, isNotNull);
        expect(copy.schemaVersion, 2);
      });
    });
  });

  group('kMaxFieldLength', () {
    test('is 32', () {
      expect(kMaxFieldLength, 32);
    });
  });
}
