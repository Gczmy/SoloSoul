import 'package:riverpod_annotation/riverpod_annotation.dart';

part 'trash_filter_provider.g.dart';

/// Time filter options for trash items.
/// Each option is a tuple of (id, label).
const timeFilterOptions = [
  ('all', 'All'),
  ('10days', '10 days ago'),
  ('1day', '1 day ago'),
  ('6hours', '6 hours ago'),
  ('1hour', '1 hour ago'),
];

/// Object type IDs to display name mapping for trash filter.
/// 'page' and 'collection' map directly to UnifiedObject.typeId values.
/// 'item' is a special category that matches all predefined section item types
/// (e.g., __preset_passport, __preset_identity, __preset_bank_account, etc.).
const objectTypeIds = {
  'page': 'Page',
  'collection': 'Section',
  'item': 'Item',
};

/// Generic type IDs that are not item types.
const _genericTypeIds = {'page', 'collection', 'note', 'task', 'contact'};

/// Returns true if the typeId represents an item (not a page, collection,
/// or other generic type). Items include predefined section types like
/// __preset_passport, __preset_identity, __preset_bank_account, etc.
bool isItemType(String? typeId) {
  if (typeId == null) return false;
  if (_genericTypeIds.contains(typeId)) return false;
  return true;
}

/// Provider for the selected time filter in trash view.
/// null or 'all' means show all items regardless of time.
@riverpod
class TrashTimeFilter extends _$TrashTimeFilter {
  @override
  String? build() => null;

  void setFilter(String? filterId) {
    state = filterId == 'all' ? null : filterId;
  }

  void clear() {
    state = null;
  }
}

/// Provider for the selected type filters in trash view.
/// Empty set means show all types.
@riverpod
class TrashTypeFilter extends _$TrashTypeFilter {
  @override
  Set<String> build() => {};

  void toggle(String typeId) {
    if (state.contains(typeId)) {
      state = {...state}..remove(typeId);
    } else {
      state = {...state, typeId};
    }
  }

  void setFilters(Set<String> filters) {
    state = filters;
  }

  void clear() {
    state = {};
  }
}

/// Returns the cutoff DateTime for a given time filter id.
/// Returns null if filter is 'all' or null.
DateTime? getTimeFilterCutoff(String? filterId) {
  if (filterId == null || filterId == 'all') return null;

  final now = DateTime.now();
  switch (filterId) {
    case '10days':
      return now.subtract(const Duration(days: 10));
    case '1day':
      return now.subtract(const Duration(days: 1));
    case '6hours':
      return now.subtract(const Duration(hours: 6));
    case '1hour':
      return now.subtract(const Duration(hours: 1));
    default:
      return null;
  }
}
