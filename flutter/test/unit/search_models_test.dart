import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/presentation/models/search_models.dart';

void main() {
  group('SearchResultItem', () {
    test('constructs with required fields', () {
      const item = SearchResultItem(
        fieldPath: 'identity.name',
        fieldName: 'Name',
        section: 'identity',
        sectionDisplayName: 'Identity',
        value: 'John',
        sensitivityLevel: SensitivityLevel.public,
      );
      expect(item.fieldPath, 'identity.name');
      expect(item.value, 'John');
      expect(item.isDeleted, false);
    });

    test('constructs with isDeleted true', () {
      const item = SearchResultItem(
        fieldPath: 'identity.name',
        fieldName: 'Name',
        section: 'identity',
        sectionDisplayName: 'Identity',
        value: 'John',
        sensitivityLevel: SensitivityLevel.public,
        isDeleted: true,
      );
      expect(item.isDeleted, isTrue);
    });
  });

  group('SearchState', () {
    test('has default values', () {
      const state = SearchState();
      expect(state.query, '');
      expect(state.searchPublic, isTrue);
      expect(state.searchInternal, isTrue);
      expect(state.searchSensitive, isTrue);
      expect(state.searchCriticalOnly, isTrue);
      expect(state.results, isEmpty);
      expect(state.isSearching, isFalse);
    });

    test('copyWith updates fields', () {
      const state = SearchState();
      final updated = state.copyWith(
        query: 'test',
        isSearching: true,
        results: const [
          SearchResultItem(
            fieldPath: 'identity.name',
            fieldName: 'Name',
            section: 'identity',
            sectionDisplayName: 'Identity',
            value: 'John',
            sensitivityLevel: SensitivityLevel.public,
          ),
        ],
      );
      expect(updated.query, 'test');
      expect(updated.isSearching, isTrue);
      expect(updated.results.length, 1);
      expect(updated.searchPublic, isTrue); // unchanged
    });

    test('copyWith preserves unchanged fields', () {
      const state = SearchState(query: 'hello');
      final updated = state.copyWith(isSearching: true);
      expect(updated.query, 'hello');
      expect(updated.isSearching, isTrue);
    });

    test('hasActiveFilters returns true when any filter enabled', () {
      const state = SearchState();
      expect(state.hasActiveFilters, isTrue);
    });

    test('hasActiveFilters returns true with some filters disabled', () {
      const state = SearchState(
        searchPublic: false,
        searchInternal: false,
        searchSensitive: true,
        searchCriticalOnly: false,
      );
      expect(state.hasActiveFilters, isTrue);
    });

    test('hasActiveFilters returns false when all filters disabled', () {
      const state = SearchState(
        searchPublic: false,
        searchInternal: false,
        searchSensitive: false,
        searchCriticalOnly: false,
      );
      expect(state.hasActiveFilters, isFalse);
    });
  });
}
