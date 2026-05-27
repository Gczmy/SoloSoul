import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/models/search_models.dart';
import 'package:solosoul_flutter/presentation/providers/search_provider.dart';
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart';

void main() {
  group('SearchResultItem', () {
    test('creates with required fields', () {
      const item = SearchResultItem(
        fieldPath: 'identity.name',
        fieldName: 'name',
        section: 'identity',
        sectionDisplayName: 'Identity',
        value: 'John',
        sensitivityLevel: SensitivityLevel.public,
      );
      expect(item.fieldPath, 'identity.name');
      expect(item.fieldName, 'name');
      expect(item.section, 'identity');
      expect(item.sectionDisplayName, 'Identity');
      expect(item.value, 'John');
      expect(item.sensitivityLevel, SensitivityLevel.public);
      expect(item.isDeleted, isFalse);
    });

    test('creates with isDeleted true', () {
      const item = SearchResultItem(
        fieldPath: 'test',
        fieldName: 'test',
        section: 'test',
        sectionDisplayName: 'Test',
        value: 'val',
        sensitivityLevel: SensitivityLevel.sensitive,
        isDeleted: true,
      );
      expect(item.isDeleted, isTrue);
    });
  });

  group('SearchState', () {
    test('default constructor has correct defaults', () {
      const state = SearchState();
      expect(state.query, '');
      expect(state.searchPublic, isTrue);
      expect(state.searchInternal, isTrue);
      expect(state.searchSensitive, isTrue);
      expect(state.searchCriticalOnly, isTrue);
      expect(state.results, isEmpty);
      expect(state.isSearching, isFalse);
    });

    test('hasActiveFilters is true when any filter is on', () {
      const state = SearchState(
        searchPublic: true,
        searchInternal: false,
        searchSensitive: false,
        searchCriticalOnly: false,
      );
      expect(state.hasActiveFilters, isTrue);
    });

    test('hasActiveFilters is false when all filters off', () {
      const state = SearchState(
        searchPublic: false,
        searchInternal: false,
        searchSensitive: false,
        searchCriticalOnly: false,
      );
      expect(state.hasActiveFilters, isFalse);
    });

    group('copyWith', () {
      test('copies with no changes', () {
        const state = SearchState(query: 'test');
        final copy = state.copyWith();
        expect(copy.query, 'test');
        expect(copy.searchPublic, isTrue);
        expect(copy.results, isEmpty);
      });

      test('copies with changes', () {
        const state = SearchState(query: 'old');
        final copy = state.copyWith(
          query: 'new',
          isSearching: true,
          searchPublic: false,
        );
        expect(copy.query, 'new');
        expect(copy.isSearching, isTrue);
        expect(copy.searchPublic, isFalse);
        // Unchanged
        expect(copy.searchInternal, isTrue);
      });

      test('copies with results', () {
        const state = SearchState();
        final results = [
          const SearchResultItem(
            fieldPath: 'a',
            fieldName: 'b',
            section: 'c',
            sectionDisplayName: 'C',
            value: 'v',
            sensitivityLevel: SensitivityLevel.public,
          ),
        ];
        final copy = state.copyWith(results: results);
        expect(copy.results, hasLength(1));
      });
    });

    group('SearchNotifier', () {
      late ProviderContainer container;

      setUp(() {
        container = ProviderContainer();
      });

      tearDown(() => container.dispose());

      test('initial state is empty', () {
        final state = container.read(searchProvider);
        expect(state.query, '');
        expect(state.results, isEmpty);
        expect(state.isSearching, isFalse);
        expect(state.searchPublic, isTrue);
        expect(state.searchInternal, isTrue);
        expect(state.searchSensitive, isTrue);
        expect(state.searchCriticalOnly, isTrue);
      });

      test('setQuery updates query', () {
        container.read(searchProvider.notifier).setQuery('test');
        expect(container.read(searchProvider).query, 'test');
      });

      test('setQuery with short query clears results', () {
        final notifier = container.read(searchProvider.notifier);
        notifier.setQuery('ab');
        notifier.setQuery('a');
        expect(container.read(searchProvider).results, isEmpty);
        expect(container.read(searchProvider).query, 'a');
      });

      test('togglePublic flips state', () {
        final notifier = container.read(searchProvider.notifier);
        expect(container.read(searchProvider).searchPublic, isTrue);
        notifier.togglePublic();
        expect(container.read(searchProvider).searchPublic, isFalse);
        notifier.togglePublic();
        expect(container.read(searchProvider).searchPublic, isTrue);
      });

      test('toggleInternal flips state', () {
        final notifier = container.read(searchProvider.notifier);
        expect(container.read(searchProvider).searchInternal, isTrue);
        notifier.toggleInternal();
        expect(container.read(searchProvider).searchInternal, isFalse);
      });

      test('toggleSensitive flips state', () {
        final notifier = container.read(searchProvider.notifier);
        expect(container.read(searchProvider).searchSensitive, isTrue);
        notifier.toggleSensitive();
        expect(container.read(searchProvider).searchSensitive, isFalse);
      });

      test('toggleCriticalOnly flips state', () {
        final notifier = container.read(searchProvider.notifier);
        expect(container.read(searchProvider).searchCriticalOnly, isTrue);
        notifier.toggleCriticalOnly();
        expect(container.read(searchProvider).searchCriticalOnly, isFalse);
      });
    });
  });
}
