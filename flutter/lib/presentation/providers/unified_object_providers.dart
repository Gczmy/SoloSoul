part of 'unified_object_provider.dart';

// =============================================================================
// Derived Providers
// =============================================================================

/// All root-level objects (parentId == null), active only.
@riverpod
List<UnifiedObject> rootObjects(Ref ref) {
  final objects = ref.watch(unifiedObjectProvider.select((d) => d.objects));
  return objects
      .where((o) => o.parentId == null && !o.isDeleted)
      .toList();
}

/// Direct children of a specific parent, in childrenIds order, active only.
@riverpod
List<UnifiedObject> children(Ref ref, String parentId) {
  final cache = ref.watch(unifiedObjectCacheProvider);
  final parent = cache.objectById[parentId];
  if (parent == null) return [];
  return parent.childrenIds
      .map((id) => cache.objectById[id])
      .whereType<UnifiedObject>()
      .where((o) => !o.isDeleted)
      .toList();
}

/// Get a specific object by ID.
@riverpod
UnifiedObject? objectById(Ref ref, String id) {
  final cache = ref.watch(unifiedObjectCacheProvider);
  return cache.objectById[id];
}

/// Get all active objects of a given type.
@riverpod
List<UnifiedObject> objectsByType(Ref ref, String typeId) {
  final objects = ref.watch(unifiedObjectProvider.select((d) => d.objects));
  return objects
      .where((o) => o.typeId == typeId && !o.isDeleted)
      .toList();
}

/// Get all soft-deleted objects.
@riverpod
List<UnifiedObject> deletedObjects(Ref ref) {
  final objects = ref.watch(unifiedObjectProvider.select((d) => d.objects));
  return objects.where((o) => o.isDeleted).toList();
}

/// Get top-level deleted objects only — objects whose parent is not also deleted.
/// This powers the grouped trash view where deleted items live inside their
/// deleted section card rather than appearing as separate entries.
final trashRootDeletedObjectsProvider = Provider<List<UnifiedObject>>((ref) {
  final objects = ref.watch(unifiedObjectProvider.select((d) => d.objects));
  final deletedIds = objects.where((o) => o.isDeleted).map((o) => o.id).toSet();
  final result = objects
      .where((o) =>
          o.isDeleted &&
          (o.parentId == null || !deletedIds.contains(o.parentId)))
      .toList();
  // Sort by deletedAt descending (most recent first, nulls last)
  result.sort((a, b) {
    final aTime = a.deletedAt;
    final bTime = b.deletedAt;
    if (aTime == null && bTime == null) return 0;
    if (aTime == null) return 1;
    if (bTime == null) return -1;
    return bTime.compareTo(aTime);
  });
  return result;
});

/// Get deleted children of a specific parent (for section expand panel in trash).
/// Sorted by deletedAt descending (most recent first).
final deletedChildrenProvider =
    Provider.family<List<UnifiedObject>, String>((ref, parentId) {
  final objects = ref.watch(unifiedObjectProvider.select((d) => d.objects));
  final result =
      objects.where((o) => o.parentId == parentId && o.isDeleted).toList();
  result.sort((a, b) {
    final aTime = a.deletedAt;
    final bTime = b.deletedAt;
    if (aTime == null && bTime == null) return 0;
    if (aTime == null) return 1;
    if (bTime == null) return -1;
    return bTime.compareTo(aTime);
  });
  return result;
});

// =============================================================================
// Default Page Providers
// =============================================================================

/// Get a default page by its fixed ID.
@riverpod
UnifiedObject? defaultPage(Ref ref, String pageId) {
  final objects = ref.watch(unifiedObjectProvider.select((d) => d.objects));
  try {
    return objects.firstWhere((o) => o.id == pageId && !o.isDeleted);
  } on Object {
    return null;
  }
}

/// Get a default section by its fixed ID.
@riverpod
UnifiedObject? defaultSection(Ref ref, String sectionId) {
  final objects = ref.watch(unifiedObjectProvider.select((d) => d.objects));
  try {
    return objects.firstWhere((o) => o.id == sectionId && !o.isDeleted);
  } on Object {
    return null;
  }
}

/// Get active items under a default section, ordered by section's childrenIds.
@riverpod
List<UnifiedObject> defaultPageItems(Ref ref, String sectionId) {
  final cache = ref.watch(unifiedObjectCacheProvider);
  final section = cache.objectById[sectionId];
  if (section == null) return [];
  return section.childrenIds
      .map((id) => cache.objectById[id])
      .whereType<UnifiedObject>()
      .where((o) => !o.isDeleted)
      .toList();
}
