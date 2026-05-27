import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart';

/// Search result item
class SearchResultItem {
  final String fieldPath;
  final String fieldName;
  final String section;
  final String sectionDisplayName;
  final String value;
  final SensitivityLevel sensitivityLevel;
  final bool isDeleted;

  const SearchResultItem({
    required this.fieldPath,
    required this.fieldName,
    required this.section,
    required this.sectionDisplayName,
    required this.value,
    required this.sensitivityLevel,
    this.isDeleted = false,
  });
}

/// Search state
class SearchState {
  final String query;
  final bool searchPublic;
  final bool searchInternal;
  final bool searchSensitive;
  final bool searchCriticalOnly;
  final List<SearchResultItem> results;
  final bool isSearching;

  const SearchState({
    this.query = '',
    this.searchPublic = true,
    this.searchInternal = true,
    this.searchSensitive = true,
    this.searchCriticalOnly = true,
    this.results = const [],
    this.isSearching = false,
  });

  SearchState copyWith({
    String? query,
    bool? searchPublic,
    bool? searchInternal,
    bool? searchSensitive,
    bool? searchCriticalOnly,
    List<SearchResultItem>? results,
    bool? isSearching,
  }) {
    return SearchState(
      query: query ?? this.query,
      searchPublic: searchPublic ?? this.searchPublic,
      searchInternal: searchInternal ?? this.searchInternal,
      searchSensitive: searchSensitive ?? this.searchSensitive,
      searchCriticalOnly: searchCriticalOnly ?? this.searchCriticalOnly,
      results: results ?? this.results,
      isSearching: isSearching ?? this.isSearching,
    );
  }

  bool get hasActiveFilters =>
      searchPublic || searchInternal || searchSensitive || searchCriticalOnly;
}