import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/profile_data.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';

void main() {
  group('ProfileData', () {
    test('constructs with default values', () {
      const profile = ProfileData();
      expect(profile.unifiedObjects, isNull);
      expect(profile.schemaVersion, isNull);
    });

    test('copyWith updates fields', () {
      const profile = ProfileData(schemaVersion: 1);
      final updated = profile.copyWith(schemaVersion: 2);
      expect(updated.schemaVersion, 2);
      expect(updated.unifiedObjects, isNull);
    });

    test('collectAllItemIds returns empty when no objects', () {
      const profile = ProfileData();
      expect(profile.collectAllItemIds(), isEmpty);
    });

    test('collectAllItemIds collects all ids', () {
      final now = DateTime.now().millisecondsSinceEpoch;
      final obj1 = UnifiedObject(
        id: 'obj1',
        name: 'Object 1',
        createdAt: now,
        updatedAt: now,
      );
      final obj2 = UnifiedObject(
        id: 'obj2',
        name: 'Object 2',
        createdAt: now,
        updatedAt: now,
      );
      final profile = ProfileData(
        unifiedObjects: UnifiedObjectData(objects: [obj1, obj2]),
      );

      final ids = profile.collectAllItemIds();
      expect(ids, contains('obj1'));
      expect(ids, contains('obj2'));
      expect(ids.length, 2);
    });

    test('kMaxFieldLength is positive', () {
      expect(kMaxFieldLength, greaterThan(0));
    });
  });
}
