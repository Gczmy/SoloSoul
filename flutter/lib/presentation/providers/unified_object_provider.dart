import 'package:flutter/material.dart' show immutable;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';
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
      if (next.hasValue) {
        if (next.value != null) {
          loadFromProfile();
        } else {
          // Profile cleared (e.g., on lock/account switch) - reset to empty
          state = const UnifiedObjectData(objects: [], customTypes: []);
        }
      }
    });

    // 如果 profile 已经加载，立即同步，避免 ref.listen 因值未变化而错过
    final profile = ref.read(profileNotifierProvider).value;
    final unifiedObjects = profile?.unifiedObjects;
    if (unifiedObjects != null) {
      return unifiedObjects;
    }

    return const UnifiedObjectData(objects: [], customTypes: []);
  }

  // ---------------------------------------------------------------------------
  // Persistence
  // ---------------------------------------------------------------------------

  /// Load unified objects from the current profile.
  Future<void> loadFromProfile() async {
    final profile = ref.read(profileNotifierProvider).value;
    if (profile == null) {
      // 新账号或旧数据没有 profile 时，重置 state 避免显示其他账号的数据
      state = const UnifiedObjectData(objects: [], customTypes: []);
      return;
    }

    final data = profile.unifiedObjects;
    if (data == null) {
      // 新账号或旧数据没有 unifiedObjects 时，重置 state 避免显示其他账号的数据
      if (state.objects.isNotEmpty || state.customTypes.isNotEmpty) {
        state = const UnifiedObjectData(objects: [], customTypes: []);
      }
      return;
    }

    // 避免无意义的 state 覆盖（引用相等时跳过，防止级联重建）
    if (identical(state, data)) return;

    // 数据完整性修复：挂载孤儿 item 到默认 section
    final repaired = _repairOrphanItems(data);

    state = repaired;
  }

  /// Reset state to empty (called on lock or account switch)
  void reset() {
    state = const UnifiedObjectData(objects: [], customTypes: []);
  }

  /// 启动时数据完整性检查：将所有 parentId 指向不存在的 section 的 item
  /// 自动挂载到对应的默认 section（section 不存在则自动创建）。
  UnifiedObjectData _repairOrphanItems(UnifiedObjectData data) {
    var objects = List<UnifiedObject>.from(data.objects);
    final objectMap = {for (final o in objects) o.id: o};
    var modified = false;

    for (final obj in objects) {
      // 只检查 item 级别对象（有 parentId 且不是 page/collection）
      if (obj.parentId == null) continue;
      if (obj.typeId == 'page' || obj.typeId == 'collection') continue;

      final parent = objectMap[obj.parentId];
      if (parent != null) continue; // parent 存在，不是孤儿

      // 孤儿 item：尝试找到对应的默认 section
      final itemTypeId = obj.typeId;
      if (itemTypeId == null) continue;
      final targetSectionId = getDefaultSectionIdForItemType(itemTypeId);
      if (targetSectionId == null) continue;

      // 确保 section 存在
      final sectionExists = objectMap[targetSectionId] != null;
      if (!sectionExists) {
        final meta = getSectionMeta(targetSectionId);
        if (meta == null) continue;

        // 确保 page 存在（使用固定 ID）
        final pageExists = objectMap[meta.parentPageId] != null;
        if (!pageExists) {
          final now = DateTime.now().millisecondsSinceEpoch;
          final page = UnifiedObject(
            id: meta.parentPageId,
            typeId: 'page',
            name: _pageNameFromId(meta.parentPageId),
            iconName: 'article',
            parentId: null,
            childrenIds: const [],
            properties: const {},
            isDeleted: false,
            deletedAt: null,
            createdAt: now,
            updatedAt: now,
          );
          objects = _service.addObject(objects, page);
          objectMap[page.id] = page;
        }

        // 创建 section（使用固定 ID）
        final now = DateTime.now().millisecondsSinceEpoch;
        final section = UnifiedObject(
          id: targetSectionId,
          typeId: 'collection',
          name: meta.name,
          iconName: meta.iconName,
          parentId: meta.parentPageId,
          childrenIds: const [],
          properties: const {},
          isDeleted: false,
          deletedAt: null,
          createdAt: now,
          updatedAt: now,
        );
        objects = _service.addObject(objects, section);
        objects = _service.addChild(objects, meta.parentPageId, section.id);
        objectMap[section.id] = section;
        modified = true;
      }

      // 将孤儿 item 挂载到目标 section
      objects = objects.map((o) {
        if (o.id == obj.id) {
          return o.copyWith(parentId: targetSectionId, updatedAt: DateTime.now().millisecondsSinceEpoch);
        }
        return o;
      }).toList();
      objects = _service.addChild(objects, targetSectionId, obj.id);
      objectMap[obj.id] = objects.firstWhere((o) => o.id == obj.id);
      modified = true;
    }

    return modified ? data.copyWith(objects: objects) : data;
  }

  /// Save current state back to profile.
  Future<bool> _save() async {
    final profile = ref.read(profileNotifierProvider).value;
    if (profile == null) return false;

    final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
    if (accountId == null) return false;

    // 防御：如果当前 state 为空且现有数据非空，拒绝覆盖（防止未加载完成时误写）
    if (state.objects.isEmpty &&
        state.customTypes.isEmpty &&
        profile.unifiedObjects != null &&
        (profile.unifiedObjects!.objects.isNotEmpty ||
            profile.unifiedObjects!.customTypes.isNotEmpty)) {
      // 可能处于未加载完成的空状态，禁止保存空数据覆盖已有数据
      return false;
    }

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

    // Protect default pages from deletion
    if (id == DefaultPageIds.profile ||
        id == DefaultPageIds.travel ||
        id == DefaultPageIds.financial ||
        id == DefaultPageIds.professional) {
      DebugLogger.instance.logWarning('UNIFIED', 'Blocked deletion of default page: $id');
      return false;
    }

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
    final parentId = object?.parentId;
    if (parentId != null) {
      updatedObjects = _service.removeChild(updatedObjects, parentId, id);
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

  // ---------------------------------------------------------------------------
  // Default Page Item Operations (predefined schema)
  // ---------------------------------------------------------------------------

  /// Create a new default-page item under a section.
  /// The item's schema is predefined by its [typeId]; users can only edit values.
  /// 如果目标 section 不存在，会自动创建（连带创建其所属 page）。
  Future<bool> createDefaultItem({
    required String sectionId,
    required String typeId,
    required String name,
    required Map<String, String> values,
  }) async {
    var objects = state.objects;

    // 防御：如果 section 不存在，自动创建（连带 page）
    final sectionExists = _service.getObjectById(objects, sectionId) != null;
    if (!sectionExists) {
      final meta = getSectionMeta(sectionId);
      if (meta != null) {
        final now = DateTime.now().millisecondsSinceEpoch;
        // 确保 page 存在（使用固定 ID）
        final pageExists = _service.getObjectById(objects, meta.parentPageId) != null;
        if (!pageExists) {
          final page = UnifiedObject(
            id: meta.parentPageId,
            typeId: 'page',
            name: _pageNameFromId(meta.parentPageId),
            iconName: 'article',
            parentId: null,
            childrenIds: const [],
            properties: const {},
            isDeleted: false,
            deletedAt: null,
            createdAt: now,
            updatedAt: now,
          );
          objects = _service.addObject(objects, page);
        }
        // 创建 section（使用固定 ID）
        final section = UnifiedObject(
          id: sectionId,
          typeId: 'collection',
          name: meta.name,
          iconName: meta.iconName,
          parentId: meta.parentPageId,
          childrenIds: const [],
          properties: const {},
          isDeleted: false,
          deletedAt: null,
          createdAt: now,
          updatedAt: now,
        );
        objects = _service.addObject(objects, section);
        objects = _service.addChild(objects, meta.parentPageId, section.id);
      }
    }

    final properties = <String, PropertyValue>{};
    for (final entry in values.entries) {
      properties[entry.key] = TextProperty(text: entry.value);
    }

    final object = _service.createObject(
      name: name,
      typeId: typeId,
      parentId: sectionId,
      properties: properties,
    );

    var updatedObjects = _service.addObject(objects, object);
    updatedObjects = _service.addChild(updatedObjects, sectionId, object.id);

    state = state.copyWith(objects: updatedObjects);
    return _save();
  }

  String _pageNameFromId(String pageId) {
    return switch (pageId) {
      DefaultPageIds.profile => 'Profile',
      DefaultPageIds.travel => 'Travel',
      DefaultPageIds.financial => 'Financial',
      DefaultPageIds.professional => 'Professional',
      _ => 'Page',
    };
  }

  /// Update an existing default-page item's name and property values.
  Future<bool> updateDefaultItem(
    String itemId, {
    required String name,
    required Map<String, String> values,
  }) async {
    final object = _service.getObjectById(state.objects, itemId);
    if (object == null) return false;

    final properties = <String, PropertyValue>{};
    for (final entry in values.entries) {
      properties[entry.key] = TextProperty(text: entry.value);
    }

    final updated = _service.updateObject(
      object,
      name: name,
      properties: properties,
    );

    state = state.copyWith(
      objects: _service.replaceObject(state.objects, updated),
    );
    return _save();
  }

  /// Soft-delete a default-page item. Removes it from its section's childrenIds.
  Future<bool> deleteDefaultItem(String itemId) async {
    final object = _service.getObjectById(state.objects, itemId);
    if (object == null) return false;

    var updatedObjects = List<UnifiedObject>.from(state.objects);

    // Remove from section's childrenIds
    if (object.parentId != null) {
      updatedObjects = _service.removeChild(
        updatedObjects,
        object.parentId!,
        itemId,
      );
    }

    // Soft delete the object
    final deleted = _service.deleteObject(object);
    updatedObjects = _service.replaceObject(updatedObjects, deleted);

    state = state.copyWith(objects: updatedObjects);
    return _save();
  }

  /// Restore a soft-deleted default-page item. Re-adds it to its section.
  Future<bool> restoreDefaultItem(String itemId) async {
    final object = _service.getObjectById(state.objects, itemId);
    if (object == null) return false;

    var restored = _service.restoreObject(object);
    var updatedObjects = _service.replaceObject(state.objects, restored);

    // Re-add to section's childrenIds
    if (restored.parentId != null) {
      updatedObjects = _service.addChild(
        updatedObjects,
        restored.parentId!,
        restored.id,
      );
    }

    state = state.copyWith(objects: updatedObjects);
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
  final objects = ref.watch(unifiedObjectProvider.select((d) => d.objects));
  return objects
      .where((o) => o.parentId == null && !o.isDeleted)
      .toList();
}

/// Direct children of a specific parent, in childrenIds order, active only.
@riverpod
List<UnifiedObject> children(Ref ref, String parentId) {
  final objects = ref.watch(unifiedObjectProvider.select((d) => d.objects));
  final map = {for (final o in objects) o.id: o};
  final parent = map[parentId];
  if (parent == null) return [];
  return parent.childrenIds
      .map((id) => map[id])
      .whereType<UnifiedObject>()
      .where((o) => !o.isDeleted)
      .toList();
}

/// Get a specific object by ID.
@riverpod
UnifiedObject? objectById(Ref ref, String id) {
  final objects = ref.watch(unifiedObjectProvider.select((d) => d.objects));
  final map = {for (final o in objects) o.id: o};
  return map[id];
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
  final objects = ref.watch(unifiedObjectProvider.select((d) => d.objects));
  final map = {for (final o in objects) o.id: o};
  final section = map[sectionId];
  if (section == null) return [];
  return section.childrenIds
      .map((id) => map[id])
      .whereType<UnifiedObject>()
      .where((o) => !o.isDeleted)
      .toList();
}

// =============================================================================
// Pre-computed Cache
// =============================================================================

/// 预计算缓存：所有对象的工作区内容一次性算好，点击页面时直接读取，无需现场遍历。
@immutable
class UnifiedObjectCache {
  final Map<String, UnifiedObject> objectById;

  /// parentId → 该 parent 下非 page 类型的子对象列表（workspace 显示用）
  final Map<String, List<UnifiedObject>> workspaceChildren;

  /// parentId → 该 parent 下 type=='item' 的子对象列表（ObjectCard 显示用）
  final Map<String, List<UnifiedObject>> itemChildren;

  /// 根级对象列表（parentId == null，未删除）
  final List<UnifiedObject> rootObjects;

  const UnifiedObjectCache({
    required this.objectById,
    required this.workspaceChildren,
    required this.itemChildren,
    required this.rootObjects,
  });
}

/// 全局预计算缓存 Provider：只监听 objects 列表，数据变化时一次性重建全部索引。
final unifiedObjectCacheProvider = Provider<UnifiedObjectCache>((ref) {
  final objects = ref.watch(unifiedObjectProvider.select((d) => d.objects));
  final map = {for (final o in objects) o.id: o};

  final objectById = <String, UnifiedObject>{};
  final workspaceChildren = <String, List<UnifiedObject>>{};
  final itemChildren = <String, List<UnifiedObject>>{};

  for (final obj in objects) {
    if (obj.isDeleted) continue;
    objectById[obj.id] = obj;

    final childList = obj.childrenIds
        .map((id) => map[id])
        .whereType<UnifiedObject>()
        .where((o) => !o.isDeleted)
        .toList();

    workspaceChildren[obj.id] = childList.where((c) => c.typeId != 'page').toList();
    itemChildren[obj.id] = childList.where((c) => c.typeId == 'item').toList();
  }

  final rootObjects = objects
      .where((o) => o.parentId == null && !o.isDeleted)
      .toList();

  return UnifiedObjectCache(
    objectById: objectById,
    workspaceChildren: workspaceChildren,
    itemChildren: itemChildren,
    rootObjects: rootObjects,
  );
});

// =============================================================================
// Extensions
// =============================================================================

