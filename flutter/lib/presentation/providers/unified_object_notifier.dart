part of 'unified_object_provider.dart';

/// Notifier for managing all unified objects.
class UnifiedObjectNotifier extends Notifier<UnifiedObjectData> {
  late final UnifiedObjectService _service;
  Timer? _saveTimer;
  static const _saveDebounceDuration = Duration(milliseconds: 300);

  @override
  UnifiedObjectData build() {
    _service = UnifiedObjectService.instance;

    ref.onDispose(() {
      _saveTimer?.cancel();
    });

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

  /// Public accessor for the current list of objects (avoids direct state access
  /// from outside this library).
  List<UnifiedObject> get currentObjects => state.objects;

  // ---------------------------------------------------------------------------
  // Persistence
  // ---------------------------------------------------------------------------

  /// Load unified objects from the current profile.
  Future<void> loadFromProfile() async {
    final profile = ref.read(profileNotifierProvider).value;
    if (profile == null) {
      // 新账号：创建默认结构并立即保存
      final defaultData = _createDefaultStructure(
        const UnifiedObjectData(objects: [], customTypes: []),
      );
      state = defaultData;
      final updatedProfile = ProfileData(
        unifiedObjects: defaultData,
        schemaVersion: ProfileStorageService.kSchemaVersion,
      );
      await ref.read(profileNotifierProvider.notifier).saveProfileImmediate(updatedProfile);
      return;
    }

    final data = profile.unifiedObjects;
    if (data == null) {
      // Profile 存在但 unifiedObjects 缺失：同样创建默认结构
      final defaultData = _createDefaultStructure(
        const UnifiedObjectData(objects: [], customTypes: []),
      );
      state = defaultData;
      final updatedProfile = profile.copyWith(unifiedObjects: defaultData);
      await ref.read(profileNotifierProvider.notifier).saveProfileImmediate(updatedProfile);
      return;
    }

    // 数据完整性修复：挂载孤儿 item 到默认 section
    var repaired = repairOrphanItems(data);

    // 如果数据为空（首次启动），自动创建默认 page + section（带 schema）
    if (repaired.objects.isEmpty) {
      repaired = _createDefaultStructure(repaired);
    } else {
      // 已有数据：为默认分区迁移 schema（如果 properties 为空）
      repaired = _migrateDefaultSectionSchemas(repaired);
    }

    state = repaired;
  }

  /// Reset state to empty (called on lock or account switch)
  void reset() {
    state = const UnifiedObjectData(objects: [], customTypes: []);
  }

  /// 启动时数据完整性检查：将 parentId 指向不存在的 section 的孤儿 item
  /// 重新挂载到对应默认 section，但不再自动创建缺失的 section/page。
  /// 如果目标 section 不存在，孤儿 item 的 parentId 会被清空（成为根级对象）。
  /// Public for testing.
  UnifiedObjectData repairOrphanItems(UnifiedObjectData data) {
    final objects = List<UnifiedObject>.from(data.objects);
    final now = DateTime.now().millisecondsSinceEpoch;
    var changed = false;

    // Pre-build Set for O(1) parent existence checks
    final activeIds = {
      for (final o in data.objects)
        if (!o.isDeleted) o.id,
    };

    for (var i = 0; i < objects.length; i++) {
      final obj = objects[i];
      if (obj.parentId == null) continue;
      if (obj.typeId == 'page' || obj.typeId == 'collection') continue;

      final parentExists = activeIds.contains(obj.parentId);
      if (parentExists) continue;

      // Parent missing — try to find the default section for this item type
      final itemTypeId = obj.typeId;
      if (itemTypeId == null) continue;
      final targetSectionId = getDefaultSectionIdForItemType(itemTypeId);

      if (targetSectionId != null &&
          data.objects.any((o) => o.id == targetSectionId && !o.isDeleted)) {
        // Target section exists — reparent
        objects[i] = obj.copyWith(parentId: targetSectionId, updatedAt: now);
        changed = true;
      } else {
        // No valid target — clear parentId so item becomes root-level
        objects[i] = obj.copyWith(parentId: null, updatedAt: now);
        changed = true;
      }
    }

    return changed ? data.copyWith(objects: objects) : data;
  }

  /// 首次启动时创建完整的默认 page + section 结构，并为每个 section 复制 builtin schema。
  UnifiedObjectData _createDefaultStructure(UnifiedObjectData data) {
    final objects = List<UnifiedObject>.from(data.objects);
    final now = DateTime.now().millisecondsSinceEpoch;

    for (final entry in PageSectionLinkRegistry.allDefaultLinks.entries) {
      final pageId = entry.key;
      final sectionIds = entry.value;

      // Ensure page exists
      final pageExists = objects.any((o) => o.id == pageId);
      if (!pageExists) {
        objects.add(UnifiedObject(
          id: pageId,
          typeId: 'page',
          name: pageNameFromId(pageId),
          iconName: pageIconNameFromId(pageId),
          parentId: null,
          childrenIds: const [],
          properties: const {},
          isDeleted: false,
          deletedAt: null,
          createdAt: now,
          updatedAt: now,
        ));
      }

      for (final sectionId in sectionIds) {
        final itemTypeId = getItemTypeIdForSection(sectionId);
        final config = SectionRendererRegistry.getConfigBySectionId(sectionId);

        // Build schema from builtin type definition
        final schemaProps = itemTypeId != null
            ? ObjectTypeRegistry.buildPropertiesFromType(itemTypeId)
            : <String, PropertyValue>{};
        final propertyLabels = itemTypeId != null
            ? ObjectTypeRegistry.buildPropertyLabelsFromType(itemTypeId)
            : <String, String>{};

        objects.add(UnifiedObject(
          id: sectionId,
          typeId: 'collection',
          name: config?.defaultName ?? sectionId,
          iconName: config?.iconName ?? 'folder',
          parentId: pageId,
          childrenIds: const [],
          properties: schemaProps,
          propertyLabels: propertyLabels.isNotEmpty ? propertyLabels : null,
          isDeleted: false,
          deletedAt: null,
          createdAt: now,
          updatedAt: now,
        ));

        // Add section to page's childrenIds
        final pageIndex = objects.indexWhere((o) => o.id == pageId);
        if (pageIndex >= 0) {
          final page = objects[pageIndex];
          objects[pageIndex] = page.copyWith(
            childrenIds: [...page.childrenIds, sectionId],
            updatedAt: now,
          );
        }
      }
    }

    return data.copyWith(objects: objects);
  }

  /// 为已有数据迁移：修复默认页面 iconName、为空分区填充 schema、
  /// 并创建从未存在过的缺失默认页面/分区。
  UnifiedObjectData _migrateDefaultSectionSchemas(UnifiedObjectData data) {
    final objects = List<UnifiedObject>.from(data.objects);
    final now = DateTime.now().millisecondsSinceEpoch;
    var changed = false;

    for (var i = 0; i < objects.length; i++) {
      final obj = objects[i];

      // Migrate default page icon names (only for built-in default pages)
      if (obj.typeId == 'page' && _isDefaultPageId(obj.id)) {
        final expectedIcon = pageIconNameFromId(obj.id);
        if (obj.iconName != expectedIcon) {
          objects[i] = obj.copyWith(iconName: expectedIcon, updatedAt: now);
          changed = true;
        }
        continue;
      }

      // Migrate default section schemas
      if (obj.typeId != 'collection') continue;
      if (obj.properties.isNotEmpty) continue;

      final itemTypeId = getItemTypeIdForSection(obj.id);
      if (itemTypeId == null) continue;

      final schemaProps = ObjectTypeRegistry.buildPropertiesFromType(itemTypeId);
      if (schemaProps.isEmpty) continue;
      final propertyLabels = ObjectTypeRegistry.buildPropertyLabelsFromType(itemTypeId);

      objects[i] = obj.copyWith(
        properties: schemaProps,
        propertyLabels: propertyLabels.isNotEmpty ? propertyLabels : null,
        updatedAt: now,
      );
      changed = true;
    }

    // Create missing default pages (never existed before)
    for (final pageId in [
      DefaultPageIds.profile,
      DefaultPageIds.travel,
      DefaultPageIds.financial,
      DefaultPageIds.professional,
    ]) {
      if (objects.any((o) => o.id == pageId)) continue;

      objects.add(UnifiedObject(
        id: pageId,
        typeId: 'page',
        name: pageNameFromId(pageId),
        iconName: pageIconNameFromId(pageId),
        parentId: null,
        childrenIds: const [],
        properties: const {},
        isDeleted: false,
        deletedAt: null,
        createdAt: now,
        updatedAt: now,
      ));
      changed = true;
    }

    // Create missing default sections (never existed before)
    for (final entry in PageSectionLinkRegistry.allDefaultLinks.entries) {
      final pageId = entry.key;
      for (final sectionId in entry.value) {
        if (objects.any((o) => o.id == sectionId)) continue;

        final itemTypeId = getItemTypeIdForSection(sectionId);
        final config = SectionRendererRegistry.getConfigBySectionId(sectionId);
        final schemaProps = itemTypeId != null
            ? ObjectTypeRegistry.buildPropertiesFromType(itemTypeId)
            : <String, PropertyValue>{};
        final propertyLabels = itemTypeId != null
            ? ObjectTypeRegistry.buildPropertyLabelsFromType(itemTypeId)
            : <String, String>{};

        objects.add(UnifiedObject(
          id: sectionId,
          typeId: 'collection',
          name: config?.defaultName ?? sectionId,
          iconName: config?.iconName ?? 'folder',
          parentId: pageId,
          childrenIds: const [],
          properties: schemaProps,
          propertyLabels: propertyLabels.isNotEmpty ? propertyLabels : null,
          isDeleted: false,
          deletedAt: null,
          createdAt: now,
          updatedAt: now,
        ));

        // Add section to page's childrenIds
        final pageIndex = objects.indexWhere((o) => o.id == pageId);
        if (pageIndex >= 0) {
          final page = objects[pageIndex];
          if (!page.childrenIds.contains(sectionId)) {
            objects[pageIndex] = page.copyWith(
              childrenIds: [...page.childrenIds, sectionId],
              updatedAt: now,
            );
          }
        }

        changed = true;
      }
    }

    return changed ? data.copyWith(objects: objects) : data;
  }

  /// Save current state back to profile.
  Future<bool> _save({String? operationDesc}) async {
    final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
    if (accountId == null) return false;

    final profile = ref.read(profileNotifierProvider).value;

    // 防御：如果当前 state 为空且现有数据非空，拒绝覆盖（防止未加载完成时误写）
    final unifiedObjects = profile?.unifiedObjects;
    if (state.objects.isEmpty &&
        state.customTypes.isEmpty &&
        unifiedObjects != null &&
        (unifiedObjects.objects.isNotEmpty ||
            unifiedObjects.customTypes.isNotEmpty)) {
      // 可能处于未加载完成的空状态，禁止保存空数据覆盖已有数据
      return false;
    }

    // 如果 profile 为 null（新账号或首次使用），创建一个新的 ProfileData
    final updatedProfile = profile?.copyWith(unifiedObjects: state) ??
        ProfileData(
          unifiedObjects: state,
          schemaVersion: ProfileStorageService.kSchemaVersion,
        );
    final profileNotifier = ref.read(profileNotifierProvider.notifier);
    final result = await profileNotifier.saveProfileImmediate(updatedProfile);
    if (result && operationDesc != null) {
      unawaited(ref.read(authNotifierProvider.notifier).updateOperation(operationDesc));
    }
    return result;
  }

  /// Debounced save — batches rapid mutations into a single disk write.
  /// Returns a Future that completes when the save actually executes.
  Future<bool> _saveDebounced({String? operationDesc}) {
    final completer = Completer<bool>();
    _saveTimer?.cancel();
    _saveTimer = Timer(_saveDebounceDuration, () async {
      final result = await _save(operationDesc: operationDesc);
      completer.complete(result);
    });
    return completer.future;
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
    Map<String, String>? propertyLabels,
    Map<String, String>? semanticTypes,
    List<String>? propertyOrder,
  }) async {
    final object = _service.createObject(
      name: name,
      typeId: typeId,
      parentId: parentId,
      iconName: iconName,
      properties: properties,
      propertyLabels: propertyLabels,
      semanticTypes: semanticTypes,
      propertyOrder: propertyOrder,
    );

    var updatedObjects = _service.addObject(state.objects, object);

    // If parent specified, add child reference to parent's childrenIds
    if (parentId != null) {
      updatedObjects = _service.addChild(updatedObjects, parentId, object.id);
    }

    state = state.copyWith(objects: updatedObjects);
    return _saveDebounced(operationDesc: 'Created object');
  }

  /// Creates a new object and returns its ID on success, or null on failure.
  /// This is identical to [createObject] but provides the generated object ID
  /// for follow-up operations (e.g. attaching files).
  Future<String?> createObjectAndReturnId({
    required String name,
    String? typeId,
    String? parentId,
    String? iconName,
    Map<String, PropertyValue>? properties,
    Map<String, String>? propertyLabels,
    Map<String, String>? semanticTypes,
    List<String>? propertyOrder,
  }) async {
    final object = _service.createObject(
      name: name,
      typeId: typeId,
      parentId: parentId,
      iconName: iconName,
      properties: properties,
      propertyLabels: propertyLabels,
      semanticTypes: semanticTypes,
      propertyOrder: propertyOrder,
    );

    var updatedObjects = _service.addObject(state.objects, object);

    if (parentId != null) {
      updatedObjects = _service.addChild(updatedObjects, parentId, object.id);
    }

    state = state.copyWith(objects: updatedObjects);
    final saved = await _saveDebounced(operationDesc: 'Created object');
    return saved ? object.id : null;
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
    Object? parentId = UnifiedObject.kNullSentinel,
    Map<String, PropertyValue>? properties,
    Map<String, String>? propertyLabels,
    Map<String, String>? semanticTypes,
    List<String>? propertyOrder,
    List<String>? childrenIds,
    List<Attachment>? attachments,
    int? schemaVersionWhenSaved,
  }) async {
    final object = _service.getObjectById(state.objects, id);
    if (object == null) return false;

    // Cleanup removed attachment files before updating state
    if (attachments != null) {
      final oldFileIds = object.attachments.map((a) => a.fileId).toSet();
      final newFileIds = attachments.map((a) => a.fileId).toSet();
      final removedFileIds = oldFileIds.difference(newFileIds);
      if (removedFileIds.isNotEmpty) {
        final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
        if (accountId != null) {
          await Future.wait(
            removedFileIds.map(
              (fileId) => AttachmentStorageService().deleteAttachment(
                accountId: accountId,
                fileId: fileId,
              ),
            ),
          );
        }
      }
    }

    final updated = _service.updateObject(
      object,
      name: name,
      typeId: typeId,
      iconName: iconName,
      parentId: parentId,
      properties: properties,
      propertyLabels: propertyLabels,
      semanticTypes: semanticTypes,
      propertyOrder: propertyOrder,
      childrenIds: childrenIds,
      attachments: attachments,
      schemaVersionWhenSaved: schemaVersionWhenSaved,
    );

    state = state.copyWith(
      objects: _service.replaceObject(state.objects, updated),
    );
    return _saveDebounced(operationDesc: 'Updated object');
  }

  // ---------------------------------------------------------------------------
  // Attachment lifecycle
  // ---------------------------------------------------------------------------

  /// Soft-delete an attachment by marking [isDeleted] = true.
  /// The encrypted file on disk is preserved.
  Future<bool> softDeleteAttachment(String objectId, String attachmentId) async {
    final object = _service.getObjectById(state.objects, objectId);
    if (object == null) return false;

    final now = DateTime.now().millisecondsSinceEpoch;
    final updatedAttachments = object.attachments.map((a) {
      if (a.id == attachmentId) {
        return a.copyWith(isDeleted: true, deletedAt: now);
      }
      return a;
    }).toList();

    return updateObject(
      objectId,
      attachments: updatedAttachments,
    );
  }

  /// Restore a soft-deleted attachment by clearing [isDeleted] and [deletedAt].
  Future<bool> restoreAttachment(String objectId, String attachmentId) async {
    final object = _service.getObjectById(state.objects, objectId);
    if (object == null) return false;

    final updatedAttachments = object.attachments.map((a) {
      if (a.id == attachmentId) {
        return a.copyWith(isDeleted: false, deletedAt: null);
      }
      return a;
    }).toList();

    return updateObject(
      objectId,
      attachments: updatedAttachments,
    );
  }

  /// Permanently delete an attachment: remove from metadata list and delete
  /// the encrypted file from disk.
  Future<bool> permanentlyDeleteAttachment(
    String objectId,
    String attachmentId, {
    String? accountId,
  }) async {
    final object = _service.getObjectById(state.objects, objectId);
    if (object == null) return false;

    final attachment = object.attachments.firstWhere(
      (a) => a.id == attachmentId,
      orElse: () => throw Exception('Attachment not found'),
    );

    // Delete encrypted file first
    if (accountId == null) {
      throw Exception('accountId is required for permanent attachment deletion');
    }
    try {
      await AttachmentStorageService().deleteAttachment(
        accountId: accountId,
        fileId: attachment.fileId,
      );
    } on Exception catch (_) {
      // Allow metadata removal even if file deletion fails (orphan file)
    }

    final updatedAttachments =
        object.attachments.where((a) => a.id != attachmentId).toList();

    return updateObject(
      objectId,
      attachments: updatedAttachments,
    );
  }

  /// Move an object to a new parent.
  Future<bool> moveObject(String objectId, String? newParentId) async {
    final updatedObjects = _service.moveObject(state.objects, objectId, newParentId);
    state = state.copyWith(objects: updatedObjects);
    return _saveDebounced(operationDesc: 'Moved object');
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
    return _save(operationDesc: 'Deleted object');
  }

  /// Restore a soft-deleted object and all its descendants.
  Future<bool> restoreObject(String id) async {
    final object = _service.getObjectById(state.objects, id);
    if (object == null) return false;

    var updatedObjects = state.objects;

    // Restore the object itself and all descendants (mirrors deleteObject recursion)
    final descendantIds = _service.getDescendantIds(state.objects, id);
    final idsToRestore = {id, ...descendantIds};

    for (final restoreId in idsToRestore) {
      final obj = _service.getObjectById(updatedObjects, restoreId);
      if (obj != null && obj.isDeleted) {
        final restored = _service.restoreObject(obj);
        updatedObjects = _service.replaceObject(updatedObjects, restored);
      }
    }

    // Re-add to parent's childrenIds if parent exists
    if (object.parentId != null) {
      updatedObjects = _service.addChild(updatedObjects, object.parentId!, id);
    }

    state = state.copyWith(objects: updatedObjects);
    return _save(operationDesc: 'Restored object');
  }

  /// Permanently delete an object and all its descendants.
  /// If [accountId] is provided, associated attachment files are also deleted.
  Future<bool> permanentlyDeleteObject(String id, {String? accountId}) async {
    final object = _service.getObjectById(state.objects, id);
    final descendantIds = _service.getDescendantIds(state.objects, id);
    final idsToRemove = {id, ...descendantIds};

    // Cleanup attachment files before removing from state
    if (accountId != null) {
      final futures = <Future<void>>[];
      for (final removeId in idsToRemove) {
        final obj = _service.getObjectById(state.objects, removeId);
        if (obj != null && obj.attachments.isNotEmpty) {
          futures.add(
            AttachmentStorageService().deleteAttachments(
              accountId: accountId,
              attachments: obj.attachments,
            ),
          );
        }
      }
      await Future.wait(futures);
    }

    var updatedObjects = state.objects
        .where((o) => !idsToRemove.contains(o.id))
        .toList();

    // Remove from parent's childrenIds
    final parentId = object?.parentId;
    if (parentId != null) {
      updatedObjects = _service.removeChild(updatedObjects, parentId, id);
    }

    state = state.copyWith(objects: updatedObjects);
    return _save(operationDesc: 'Permanently deleted object');
  }

  /// Permanently delete multiple objects in a single save operation.
  /// More efficient than calling permanentlyDeleteObject in a loop.
  /// If [accountId] is provided, associated attachment files are also deleted.
  Future<bool> permanentlyDeleteMultiple(List<String> ids, {String? accountId}) async {
    if (ids.isEmpty) return true;

    final idsToRemove = <String>{};
    for (final id in ids) {
      idsToRemove.add(id);
      idsToRemove.addAll(_service.getDescendantIds(state.objects, id));
    }

    // Cleanup attachment files before removing from state
    if (accountId != null) {
      final futures = <Future<void>>[];
      for (final removeId in idsToRemove) {
        final obj = _service.getObjectById(state.objects, removeId);
        if (obj != null && obj.attachments.isNotEmpty) {
          futures.add(
            AttachmentStorageService().deleteAttachments(
              accountId: accountId,
              attachments: obj.attachments,
            ),
          );
        }
      }
      await Future.wait(futures);
    }

    var updatedObjects = state.objects
        .where((o) => !idsToRemove.contains(o.id))
        .toList();

    // Remove from parent's childrenIds for each top-level deleted object
    for (final id in ids) {
      final object = _service.getObjectById(state.objects, id);
      final parentId = object?.parentId;
      if (parentId != null) {
        updatedObjects = _service.removeChild(updatedObjects, parentId, id);
      }
    }

    state = state.copyWith(objects: updatedObjects);
    return _save(operationDesc: 'Permanently deleted ${ids.length} objects');
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
    return _saveDebounced(operationDesc: 'Reordered objects');
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
    // Type changes must be saved immediately (not debounced) to avoid data loss
    return _save(operationDesc: 'Saved custom type');
  }

  /// Delete a custom object type.
  Future<bool> deleteCustomType(String typeId) async {
    final updatedTypes = state.customTypes.where((t) => t.id != typeId).toList();
    state = state.copyWith(customTypes: updatedTypes);
    return _save(operationDesc: 'Deleted custom type');
  }

  // ---------------------------------------------------------------------------
  // Default Page Item Operations (predefined schema)
  // ---------------------------------------------------------------------------

  /// Create a new default-page item under a section.
  /// The item's schema is predefined by its [typeId]; users can only edit values.
  /// If the target section does not exist, it is auto-created (with schema).
  Future<bool> createDefaultItem({
    required String sectionId,
    required String typeId,
    required String name,
    required Map<String, PropertyValue> properties,
  }) async {
    var objects = state.objects;

    // Ensure section exists (create with schema if missing)
    final sectionExists = _service.getObjectById(objects, sectionId) != null;
    if (!sectionExists) {
      final meta = getSectionMeta(sectionId);
      if (meta != null) {
        final now = DateTime.now().millisecondsSinceEpoch;
        // Ensure page exists
        final pageExists = _service.getObjectById(objects, meta.parentPageId) != null;
        if (!pageExists) {
          final page = UnifiedObject(
            id: meta.parentPageId,
            typeId: 'page',
            name: pageNameFromId(meta.parentPageId),
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
        final schemaProps = ObjectTypeRegistry.buildPropertiesFromType(typeId);
        final propertyLabels = ObjectTypeRegistry.buildPropertyLabelsFromType(typeId);
        final section = UnifiedObject(
          id: sectionId,
          typeId: 'collection',
          name: meta.name,
          iconName: meta.iconName,
          parentId: meta.parentPageId,
          childrenIds: const [],
          properties: schemaProps,
          propertyLabels: propertyLabels.isNotEmpty ? propertyLabels : null,
          isDeleted: false,
          deletedAt: null,
          createdAt: now,
          updatedAt: now,
        );
        objects = _service.addObject(objects, section);
        objects = _service.addChild(objects, meta.parentPageId, sectionId);
      }
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
    return _saveDebounced(operationDesc: 'Created item');
  }

  /// Public for testing.
  String pageNameFromId(String pageId) {
    return switch (pageId) {
      DefaultPageIds.profile => 'Profile',
      DefaultPageIds.travel => 'Travel',
      DefaultPageIds.financial => 'Financial',
      DefaultPageIds.professional => 'Professional',
      _ => 'Page',
    };
  }

  /// Whether the given page ID is one of the built-in default pages.
  bool _isDefaultPageId(String pageId) =>
      pageId == DefaultPageIds.profile ||
      pageId == DefaultPageIds.travel ||
      pageId == DefaultPageIds.financial ||
      pageId == DefaultPageIds.professional;

  /// Icon name for a default page (used by sidebar and home quick actions).
  String pageIconNameFromId(String pageId) {
    return switch (pageId) {
      DefaultPageIds.profile => 'person',
      DefaultPageIds.travel => 'flight',
      DefaultPageIds.financial => 'account_balance',
      DefaultPageIds.professional => 'work',
      _ => 'article',
    };
  }

  /// Create all default sections for a given page, plus the page itself if missing.
  /// Used by "Restore defaults" button when a default page has no sections.
  Future<bool> createDefaultSectionsForPage(String pageId) async {
    final sectionIds = getDefaultSectionIdsForPage(pageId);
    if (sectionIds.isEmpty) return false;

    var objects = List<UnifiedObject>.from(state.objects);
    final now = DateTime.now().millisecondsSinceEpoch;

    // Ensure page exists
    final pageExists = _service.getObjectById(objects, pageId) != null;
    if (!pageExists) {
      final page = UnifiedObject(
        id: pageId,
        typeId: 'page',
        name: pageNameFromId(pageId),
        iconName: pageIconNameFromId(pageId),
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

    for (final sectionId in sectionIds) {
      final existing = _service.getObjectById(objects, sectionId);
      if (existing != null && !existing.isDeleted) continue;

      final meta = getSectionMeta(sectionId);
      if (meta == null) continue;

      final itemTypeId = getItemTypeIdForSection(sectionId);
      final schemaProps = itemTypeId != null
          ? ObjectTypeRegistry.buildPropertiesFromType(itemTypeId)
          : <String, PropertyValue>{};

      // If soft-deleted, restore it instead of creating
      if (existing != null && existing.isDeleted) {
        final restored = _service.restoreObject(existing);
        objects = _service.replaceObject(objects, restored.copyWith(
          properties: schemaProps,
          updatedAt: now,
        ));
      } else {
        final section = UnifiedObject(
          id: sectionId,
          typeId: 'collection',
          name: meta.name,
          iconName: meta.iconName,
          parentId: meta.parentPageId,
          childrenIds: const [],
          properties: schemaProps,
          isDeleted: false,
          deletedAt: null,
          createdAt: now,
          updatedAt: now,
        );
        objects = _service.addObject(objects, section);
      }
      objects = _service.addChild(objects, meta.parentPageId, sectionId);
    }

    state = state.copyWith(objects: objects);
    return _save(operationDesc: 'Restored default sections');
  }

  /// Update an existing default-page item's name and property values.
  Future<bool> updateDefaultItem(
    String itemId, {
    required String name,
    required Map<String, PropertyValue> properties,
  }) async {
    final object = _service.getObjectById(state.objects, itemId);
    if (object == null) return false;

    final updated = _service.updateObject(
      object,
      name: name,
      properties: properties,
    );

    state = state.copyWith(
      objects: _service.replaceObject(state.objects, updated),
    );
    return _saveDebounced(operationDesc: 'Updated item');
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
    return _save(operationDesc: 'Deleted item');
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
    return _saveDebounced(operationDesc: 'Restored item');
  }
}
