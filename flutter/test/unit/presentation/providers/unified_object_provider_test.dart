import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';

void main() {
  group('UnifiedObjectNotifier', () {
    late ProviderContainer container;

    setUp(() {
      container = ProviderContainer();
    });

    tearDown(() => container.dispose());

    test('build returns empty data when profile is null', () async {
      // Ensure profile provider is built (returns null)
      await container.read(profileNotifierProvider.future);
      final state = container.read(unifiedObjectProvider);
      expect(state.objects, isEmpty);
      expect(state.customTypes, isEmpty);
    });

    test('reset sets state to empty', () async {
      await container.read(profileNotifierProvider.future);
      final notifier = container.read(unifiedObjectProvider.notifier);
      notifier.reset();
      final state = container.read(unifiedObjectProvider);
      expect(state.objects, isEmpty);
      expect(state.customTypes, isEmpty);
    });

    test('loadFromProfile resets when profile is null', () async {
      await container.read(profileNotifierProvider.future);
      final notifier = container.read(unifiedObjectProvider.notifier);
      notifier.loadFromProfile();
      final state = container.read(unifiedObjectProvider);
      expect(state.objects, isEmpty);
      expect(state.customTypes, isEmpty);
    });
  });

  group('UnifiedObjectCache', () {
    test('constructs with correct fields', () {
      const cache = UnifiedObjectCache(
        objectById: {},
        workspaceChildren: {},
        itemChildren: {},
        rootObjects: [],
      );
      expect(cache.objectById, isEmpty);
      expect(cache.workspaceChildren, isEmpty);
      expect(cache.itemChildren, isEmpty);
      expect(cache.rootObjects, isEmpty);
    });
  });

  group('rootObjectsProvider', () {
    test('returns empty when no objects', () async {
      final container = ProviderContainer();
      await container.read(profileNotifierProvider.future);
      final roots = container.read(rootObjectsProvider);
      expect(roots, isEmpty);
      container.dispose();
    });
  });

  group('deletedObjectsProvider', () {
    test('returns empty when no deleted objects', () async {
      final container = ProviderContainer();
      await container.read(profileNotifierProvider.future);
      final deleted = container.read(deletedObjectsProvider);
      expect(deleted, isEmpty);
      container.dispose();
    });
  });
}
