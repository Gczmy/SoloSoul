import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart';

part 'unified_object_provider.g.dart';

/// Notifier for managing all unified objects.
class UnifiedObjectNotifier extends Notifier<UnifiedObjectData> {
  late final UnifiedObjectService _service;

  @override
  UnifiedObjectData build() {
    _service = UnifiedObjectService.instance;

    // Auto-load unified objects when profile data is loaded from storage.
    ref.listen(profileNotifierProvider, (previous, next) {
      if (next.hasValue && next.value != null) {
        loadFromProfile();
      }
    });

    return const UnifiedObjectData(objects: [], customTypes: []);
  }

  // ---------------------------------------------------------------------------
  // Persistence
  // ---------------------------------------------------------------------------

  /// Load unified objects from the current profile.
  Future<void> loadFromProfile() async {
    final profile = ref.read(profileNotifierProvider).value;
    if (profile == null) return;

    final data = profile.unifiedObjects;
    if (data == null) return; // Don't overwrite state if unifiedObjects is missing

    state = data;
  }

  /// Save current state back to profile.
  Future<bool> _save() async {
    final profile = ref.read(profileNotifierProvider).value;
    if (profile == null) return false;

    final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
    if (accountId == null) return false;

    final updatedProfile = profile.copyWith(unifiedObjects: state);
    final profileNotifier = ref.read(profileNotifierProvider.notifier);
    return profileNotifier.saveProfileImmediate(updatedProfile);
  }

  // ---------------------------------------------------------------------------
  // Create
  // ---------------------------------------------------------------------------

  /// Create a new object and optionally attach it to a parent.
  Future<bool> createObject({
    required String name,
    String? typeId,
    String? parentId,
    String? iconName,
    Map<String, PropertyValue>? properties,
  }) async {
    final object = _service.createObject(
      name: name,
      typeId: typeId,
      parentId: parentId,
      iconName: iconName,
      properties: properties,
    );

    var updatedObjects = _service.addObject(state.objects, object);

    // If parent specified, add child reference to parent's childrenIds
    if (parentId != null) {
      updatedObjects = _service.addChild(updatedObjects, parentId, object.id);
    }

    state = state.copyWith(objects: updatedObjects);
    return _save();
  }

  // ---------------------------------------------------------------------------
  // Update
  // ---------------------------------------------------------------------------

  /// Update an existing object by ID.
  Future<bool> updateObject(
    String id, {
    String? name,
    String? typeId,
    String? iconName,
    String? parentId,
    Map<String, PropertyValue>? properties,
    List<String>? childrenIds,
  }) async {
    final object = _service.getObjectById(state.objects, id);
    if (object == null) return false;

    final updated = _service.updateObject(
      object,
      name: name,
      typeId: typeId,
      iconName: iconName,
      parentId: parentId,
      properties: properties,
      childrenIds: childrenIds,
    );

    state = state.copyWith(
      objects: _service.replaceObject(state.objects, updated),
    );
    return _save();
  }

  /// Move an object to a new parent.
  Future<bool> moveObject(String objectId, String? newParentId) async {
    final updatedObjects = _service.moveObject(state.objects, objectId, newParentId);
    state = state.copyWith(objects: updatedObjects);
    return _save();
  }

  // ---------------------------------------------------------------------------
  // Delete / Restore
  // ---------------------------------------------------------------------------

  /// Soft delete an object by ID.
  /// Also removes it from its parent's childrenIds.
  /// Recursively soft-deletes all descendants.
  Future<bool> deleteObject(String id) async {
    final object = _service.getObjectById(state.objects, id);
    if (object == null) return false;

    var updatedObjects = List<UnifiedObject>.from(state.objects);

    // Remove from parent's childrenIds
    if (object.parentId != null) {
      updatedObjects = _service.removeChild(
        updatedObjects,
        object.parentId!,
        id,
      );
    }

    // Soft delete the object itself and all descendants
    final descendantIds = _service.getDescendantIds(state.objects, id);
    final idsToDelete = {id, ...descendantIds};

    for (final deleteId in idsToDelete) {
      final obj = _service.getObjectById(updatedObjects, deleteId);
      if (obj != null && !obj.isDeleted) {
        final deleted = _service.deleteObject(obj);
        updatedObjects = _service.replaceObject(updatedObjects, deleted);
      }
    }

    state = state.copyWith(objects: updatedObjects);
    return _save();
  }

  /// Restore a soft-deleted object.
  Future<bool> restoreObject(String id) async {
    final object = _service.getObjectById(state.objects, id);
    if (object == null) return false;

    var restored = _service.restoreObject(object);
    var updatedObjects = _service.replaceObject(state.objects, restored);

    // Re-add to parent's childrenIds if parent exists
    if (restored.parentId != null) {
      updatedObjects = _service.addChild(updatedObjects, restored.parentId!, restored.id);
    }

    state = state.copyWith(objects: updatedObjects);
    return _save();
  }

  /// Permanently delete an object and all its descendants.
  Future<bool> permanentlyDeleteObject(String id) async {
    final object = _service.getObjectById(state.objects, id);
    final descendantIds = _service.getDescendantIds(state.objects, id);
    final idsToRemove = {id, ...descendantIds};

    var updatedObjects = state.objects
        .where((o) => !idsToRemove.contains(o.id))
        .toList();

    // Remove from parent's childrenIds
    if (object?.parentId != null) {
      updatedObjects = _service.removeChild(updatedObjects, object!.parentId!, id);
    }

    state = state.copyWith(objects: updatedObjects);
    return _save();
  }

  // ---------------------------------------------------------------------------
  // Reorder
  // ---------------------------------------------------------------------------

  /// Reorder children within a parent.
  Future<bool> reorderChildren(
    String parentId,
    int oldIndex,
    int newIndex,
  ) async {
    final updatedObjects = _service.reorderChildren(
      state.objects,
      parentId,
      oldIndex,
      newIndex,
    );
    state = state.copyWith(objects: updatedObjects);
    return _save();
  }

  // ---------------------------------------------------------------------------
  // Custom Types
  // ---------------------------------------------------------------------------

  /// Add or update a custom object type definition.
  Future<bool> saveCustomType(ObjectTypeDefinition type) async {
    final existingIndex = state.customTypes.indexWhere((t) => t.id == type.id);
    final updatedTypes = List<ObjectTypeDefinition>.from(state.customTypes);
    if (existingIndex >= 0) {
      updatedTypes[existingIndex] = type;
    } else {
      updatedTypes.add(type);
    }
    state = state.copyWith(customTypes: updatedTypes);
    return _save();
  }

  /// Delete a custom object type.
  Future<bool> deleteCustomType(String typeId) async {
    final updatedTypes = state.customTypes.where((t) => t.id != typeId).toList();
    state = state.copyWith(customTypes: updatedTypes);
    return _save();
  }
}

/// Provider for unified object state management.
final unifiedObjectProvider =
    NotifierProvider<UnifiedObjectNotifier, UnifiedObjectData>(() {
  return UnifiedObjectNotifier();
});

// =============================================================================
// Derived Providers
// =============================================================================

/// All root-level objects (parentId == null), active only.
@riverpod
List<UnifiedObject> rootObjects(Ref ref) {
  final data = ref.watch(unifiedObjectProvider);
  return data.objects
      .where((o) => o.parentId == null && !o.isDeleted)
      .toList();
}

/// Direct children of a specific parent, in childrenIds order, active only.
@riverpod
List<UnifiedObject> children(Ref ref, String parentId) {
  final data = ref.watch(unifiedObjectProvider);
  final parentIndex = data.objects.indexWhere((o) => o.id == parentId);
  if (parentIndex == -1) return [];
  final parent = data.objects[parentIndex];
  final map = {for (final o in data.objects) o.id: o};
  return parent.childrenIds
      .where((id) => map.containsKey(id))
      .map((id) => map[id]!)
      .where((o) => !o.isDeleted)
      .toList();
}

/// Get a specific object by ID.
@riverpod
UnifiedObject? objectById(Ref ref, String id) {
  final data = ref.watch(unifiedObjectProvider);
  try {
    return data.objects.firstWhere((o) => o.id == id);
  } on Object {
    return null;
  }
}

/// Get all active objects of a given type.
@riverpod
List<UnifiedObject> objectsByType(Ref ref, String typeId) {
  final data = ref.watch(unifiedObjectProvider);
  return data.objects
      .where((o) => o.typeId == typeId && !o.isDeleted)
      .toList();
}

/// Get all soft-deleted objects.
@riverpod
List<UnifiedObject> deletedObjects(Ref ref) {
  final data = ref.watch(unifiedObjectProvider);
  return data.objects.where((o) => o.isDeleted).toList();
}

// =============================================================================
// Extensions
// =============================================================================

extension UnifiedObjectDataExtension on UnifiedObjectData {
  /// Build a quick lookup map by object ID.
  Map<String, UnifiedObject> toMap() => {
        for (final o in objects) o.id: o,
      };
}
