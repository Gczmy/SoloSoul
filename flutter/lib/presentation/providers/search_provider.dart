import 'dart:async';
import 'dart:isolate';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';
import 'package:solosoul_flutter/presentation/models/search_models.dart';

/// Search notifier
class SearchNotifier extends Notifier<SearchState> {
  Timer? _debounceTimer;

  @override
  SearchState build() {
    ref.onDispose(() {
      _debounceTimer?.cancel();
      _debounceTimer = null;
    });
    return const SearchState();
  }

  void setQuery(String query) {
    state = state.copyWith(query: query);
    if (query.length >= 2) {
      _debounceSearch();
    } else {
      _cancelDebounce();
      state = state.copyWith(results: []);
    }
  }

  void _debounceSearch() {
    _debounceTimer?.cancel();
    _debounceTimer = Timer(const Duration(milliseconds: 300), _performSearch);
  }

  void _cancelDebounce() {
    _debounceTimer?.cancel();
    _debounceTimer = null;
  }

  void togglePublic() {
    state = state.copyWith(searchPublic: !state.searchPublic);
    if (state.query.length >= 2) _performSearch();
  }

  void toggleInternal() {
    state = state.copyWith(searchInternal: !state.searchInternal);
    if (state.query.length >= 2) _performSearch();
  }

  void toggleSensitive() {
    state = state.copyWith(searchSensitive: !state.searchSensitive);
    if (state.query.length >= 2) _performSearch();
  }

  void toggleRestricted() {
    state = state.copyWith(searchRestricted: !state.searchRestricted);
    if (state.query.length >= 2) _performSearch();
  }

  bool isFieldRevealed(String fieldPath, SensitivityLevel level) {
    final style = ref.read(accountStyleProvider).value;
    if (style == null || !style.revealedFields.contains(fieldPath)) return false;
    if (level == SensitivityLevel.critical) {
      return ref.read(isSensitiveAccessGrantedProvider);
    }
    return true;
  }

  Future<void> revealFieldWithContext(
    BuildContext context,
    WidgetRef ref,
    SensitivityLevel level,
    String fieldPath,
  ) async {
    if (level == SensitivityLevel.critical) {
      if (!ref.read(isSensitiveAccessGrantedProvider)) {
        final password = await showPasswordVerificationDialog(
          context: context,
          ref: ref,
          message: 'Restricted field. Enter your master password to view.',
          onVerify: (password) async {
            final authNotifier = ref.read(authNotifierProvider.notifier);
            return authNotifier.verifyPasswordForSensitiveData(password);
          },
        );
        if (password == null) return;
        ref.read(sensitivePageAccessProvider.notifier).markVerified();
      }
    }

    ref.read(accountStyleProvider.notifier).revealField(fieldPath);
  }

  Future<void> unlockAllRestricted(
    BuildContext context,
    WidgetRef ref,
  ) async {
    if (!ref.read(isSensitiveAccessGrantedProvider)) {
      final password = await showPasswordVerificationDialog(
        context: context,
        ref: ref,
        message: 'Restricted field. Enter your master password to view.',
        onVerify: (password) async {
          final authNotifier = ref.read(authNotifierProvider.notifier);
          return authNotifier.verifyPasswordForSensitiveData(password);
        },
      );
      if (password == null) return;
      ref.read(sensitivePageAccessProvider.notifier).markVerified();
    }

    final sensitiveNotifier = ref.read(accountStyleProvider.notifier);
    for (final result in state.results) {
      if (result.sensitivityLevel == SensitivityLevel.critical) {
        sensitiveNotifier.revealField(result.fieldPath);
      }
    }
  }

  Future<void> _performSearch() async {
    if (state.query.isEmpty) {
      state = state.copyWith(results: []);
      return;
    }

    state = state.copyWith(isSearching: true);

    final objects = ref.read(unifiedObjectProvider.select((d) => d.objects));

    final results = await _executeSearchInIsolate(
      objects: objects,
      query: state.query,
      searchPublic: state.searchPublic,
      searchInternal: state.searchInternal,
      searchSensitive: state.searchSensitive,
      searchRestricted: state.searchRestricted,
    );

    state = state.copyWith(results: results, isSearching: false);
  }

  /// Wraps [_executeSearch] in an isolate.
  /// Must be static so the closure sent to [Isolate.run] does not capture `this`.
  static Future<List<SearchResultItem>> _executeSearchInIsolate({
    required List<UnifiedObject> objects,
    required String query,
    required bool searchPublic,
    required bool searchInternal,
    required bool searchSensitive,
    required bool searchRestricted,
  }) {
    return Isolate.run(() => _executeSearch(
      objects,
      query,
      searchPublic,
      searchInternal,
      searchSensitive,
      searchRestricted,
    ));
  }

  /// Pure search function — runs in a background isolate.
  static List<SearchResultItem> _executeSearch(
    List<UnifiedObject> objects,
    String query,
    bool searchPublic,
    bool searchInternal,
    bool searchSensitive,
    bool searchRestricted,
  ) {
    final results = <SearchResultItem>[];
    final lowerQuery = query.toLowerCase();
    final objectMap = {for (final o in objects) o.id: o};

    bool matchesQuery(String text) => text.toLowerCase().contains(lowerQuery);

    SensitivityLevel? checkLevel(SensitivityLevel level) {
      switch (level) {
        case SensitivityLevel.public:
          if (!searchPublic) return null;
          break;
        case SensitivityLevel.internal:
          if (!searchInternal) return null;
          break;
        case SensitivityLevel.sensitive:
          if (!searchSensitive) return null;
          break;
        case SensitivityLevel.critical:
          if (!searchRestricted) return null;
          break;
      }
      return level;
    }

    /// Walks up the parent chain to find the nearest container (collection or root page).
    String resolveSectionName(UnifiedObject obj) {
      if (obj.parentId == null || obj.typeId == 'collection') {
        return obj.name;
      }
      String? currentId = obj.parentId;
      while (currentId != null) {
        final parent = objectMap[currentId];
        if (parent == null) break;
        if (parent.parentId == null || parent.typeId == 'collection') {
          return parent.name;
        }
        currentId = parent.parentId;
      }
      return 'Custom';
    }

    for (final obj in objects) {
      if (obj.isDeleted) continue;

      final section = resolveSectionName(obj);

      // Search object name
      if (matchesQuery(obj.name)) {
        final level = checkLevel(SensitivityLevel.public);
        if (level != null) {
          results.add(SearchResultItem(
            fieldPath: obj.id,
            fieldName: 'name',
            section: section,
            sectionDisplayName: section,
            value: obj.name,
            sensitivityLevel: level,
          ));
        }
      }

      // Search object typeId
      if (obj.typeId != null && matchesQuery(obj.typeId!)) {
        final level = checkLevel(SensitivityLevel.public);
        if (level != null) {
          results.add(SearchResultItem(
            fieldPath: obj.id,
            fieldName: 'typeId',
            section: section,
            sectionDisplayName: section,
            value: obj.typeId!,
            sensitivityLevel: level,
          ));
        }
      }

      // Search properties
      for (final entry in obj.properties.entries) {
        final prop = entry.value;
        final valueStr = switch (prop) {
          TextProperty(:final text) => text,
          NumberProperty(:final value) => value?.toString() ?? '',
          DateProperty(:final isoDate) => isoDate ?? '',
          CheckboxProperty(:final checked) => checked ? 'Yes' : 'No',
          SelectProperty(:final selectedId) => selectedId ?? '',
          MultiSelectProperty(:final selectedIds) => selectedIds.join(', '),
          RelationProperty(:final targetObjectId) => targetObjectId ?? '',
          UrlProperty(:final url) => url ?? '',
        };

        if (valueStr.isEmpty) continue;
        if (!matchesQuery(valueStr) && !matchesQuery(entry.key)) continue;

        final level = checkLevel(prop.sensitivity);
        if (level != null) {
          results.add(SearchResultItem(
            fieldPath: obj.id,
            fieldName: entry.key,
            section: section,
            sectionDisplayName: section,
            value: valueStr,
            sensitivityLevel: level,
          ));
        }
      }
    }

    return results;
  }
}

/// Search provider
final searchProvider = NotifierProvider<SearchNotifier, SearchState>(() {
  return SearchNotifier();
});
