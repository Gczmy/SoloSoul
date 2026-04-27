import 'package:flutter/material.dart';
import 'package:uuid/uuid.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';

const _uuid = Uuid();

/// Generate a new unique ID using UUID v4
String _generateId() => _uuid.v4();

/// Returns current timestamp in milliseconds since epoch
int _currentTimestamp() => DateTime.now().millisecondsSinceEpoch;

// =============================================================================
// Built-in Object Type Definitions
// =============================================================================

/// Registry of built-in object types shipped with the app.
/// User-defined types are stored in [UnifiedObjectData.customTypes].
class ObjectTypeRegistry {
  static final Map<String, ObjectTypeDefinition> _builtins = {
    for (final t in _kBuiltinTypes) t.id: t,
  };

  /// Get a type definition by ID. Checks built-ins first.
  static ObjectTypeDefinition? getType(
    String id, {
    List<ObjectTypeDefinition> customTypes = const [],
  }) {
    if (_builtins.containsKey(id)) return _builtins[id];
    try {
      return customTypes.firstWhere((t) => t.id == id);
    } on Object {
      return null;
    }
  }

  /// Get all available type definitions (built-in + custom).
  static List<ObjectTypeDefinition> getAllTypes({
    List<ObjectTypeDefinition> customTypes = const [],
  }) {
    return [..._builtins.values, ...customTypes];
  }

  /// Default type for generic objects.
  static ObjectTypeDefinition get defaultType => _builtins['note']!;
}

/// Built-in type definitions.
final List<ObjectTypeDefinition> _kBuiltinTypes = [
  const ObjectTypeDefinition(
    id: 'page',
    name: 'Page',
    iconName: 'article',
    defaultLayout: ObjectLayout.document,
  ),
  const ObjectTypeDefinition(
    id: 'collection',
    name: 'Collection',
    iconName: 'folder',
    defaultLayout: ObjectLayout.collection,
  ),
  const ObjectTypeDefinition(
    id: 'note',
    name: 'Note',
    iconName: 'note',
    defaultLayout: ObjectLayout.document,
    properties: [
      PropertyDefinition(
        id: 'content',
        name: 'Content',
        type: PropertyType.text,
      ),
    ],
  ),
  const ObjectTypeDefinition(
    id: 'task',
    name: 'Task',
    iconName: 'check_circle',
    defaultLayout: ObjectLayout.document,
    properties: [
      PropertyDefinition(
        id: 'done',
        name: 'Done',
        type: PropertyType.checkbox,
      ),
      PropertyDefinition(
        id: 'dueDate',
        name: 'Due Date',
        type: PropertyType.date,
      ),
    ],
  ),
  const ObjectTypeDefinition(
    id: 'contact',
    name: 'Contact',
    iconName: 'person',
    defaultLayout: ObjectLayout.document,
    properties: [
      PropertyDefinition(
        id: 'phone',
        name: 'Phone',
        type: PropertyType.text,
      ),
      PropertyDefinition(
        id: 'email',
        name: 'Email',
        type: PropertyType.url,
      ),
    ],
  ),
  const ObjectTypeDefinition(
    id: 'item',
    name: 'Item',
    iconName: 'list_item',
    defaultLayout: ObjectLayout.collection,
  ),
];

// =============================================================================
// UnifiedObjectService
// =============================================================================

/// Service for managing unified objects.
/// All operations are pure functions returning new instances (immutable style).
class UnifiedObjectService {
  static UnifiedObjectService? _instance;
  UnifiedObjectService._();
  static UnifiedObjectService get instance {
    _instance ??= UnifiedObjectService._();
    return _instance!;
  }

  // ---------------------------------------------------------------------------
  // Create
  // ---------------------------------------------------------------------------

  /// Create a new object.
  UnifiedObject createObject({
    required String name,
    String? typeId,
    String? parentId,
    String? iconName,
    Map<String, PropertyValue>? properties,
    List<String>? childrenIds,
  }) {
    final now = _currentTimestamp();
    final effectiveIcon = iconName ??
        ObjectTypeRegistry.getType(typeId ?? 'note')?.iconName ??
        'folder';
    return UnifiedObject(
      id: _generateId(),
      typeId: typeId,
      name: name,
      iconName: effectiveIcon,
      parentId: parentId,
      childrenIds: childrenIds ?? const [],
      properties: properties ?? const {},
      createdAt: now,
      updatedAt: now,
    );
  }

  // ---------------------------------------------------------------------------
  // Update
  // ---------------------------------------------------------------------------

  UnifiedObject updateObject(
    UnifiedObject object, {
    String? name,
    String? typeId,
    String? iconName,
    String? parentId,
    Map<String, PropertyValue>? properties,
    List<String>? childrenIds,
  }) {
    return object.copyWith(
      name: name,
      typeId: typeId,
      iconName: iconName,
      parentId: parentId,
      properties: properties,
      childrenIds: childrenIds,
      updatedAt: _currentTimestamp(),
    );
  }

  // ---------------------------------------------------------------------------
  // Soft Delete / Restore
  // ---------------------------------------------------------------------------

  UnifiedObject deleteObject(UnifiedObject object) {
    return object.copyWith(
      isDeleted: true,
      deletedAt: DateTime.now(),
      updatedAt: _currentTimestamp(),
    );
  }

  UnifiedObject restoreObject(UnifiedObject object) {
    return object.copyWith(
      isDeleted: false,
      deletedAt: null,
      updatedAt: _currentTimestamp(),
    );
  }

  // ---------------------------------------------------------------------------
  // Tree Operations
  // ---------------------------------------------------------------------------

  /// Get direct children of [parentId] in the order defined by parent's [childrenIds].
  List<UnifiedObject> getChildren(
    List<UnifiedObject> objects,
    String parentId,
  ) {
    final parent = getObjectById(objects, parentId);
    if (parent == null) return [];
    final map = {for (final o in objects) o.id: o};
    return parent.childrenIds
        .where((id) => map.containsKey(id))
        .map((id) => map[id]!)
        .where((o) => !o.isDeleted)
        .toList();
  }

  /// Get all root-level objects (parentId == null).
  List<UnifiedObject> getRootObjects(List<UnifiedObject> objects) {
    return objects
        .where((o) => o.parentId == null && !o.isDeleted)
        .toList();
  }

  /// Find object by ID.
  UnifiedObject? getObjectById(List<UnifiedObject> objects, String id) {
    try {
      return objects.firstWhere((o) => o.id == id);
    } on Object {
      return null;
    }
  }

  /// Recursively collect all descendant IDs of [objectId].
  Set<String> getDescendantIds(List<UnifiedObject> objects, String objectId) {
    final result = <String>{};
    final map = {for (final o in objects) o.id: o};
    void collect(String id) {
      final obj = map[id];
      if (obj == null) return;
      for (final childId in obj.childrenIds) {
        if (!result.contains(childId)) {
          result.add(childId);
          collect(childId);
        }
      }
    }
    collect(objectId);
    return result;
  }

  /// Move [objectId] to a new parent. Updates both old and new parent's childrenIds.
  List<UnifiedObject> moveObject(
    List<UnifiedObject> objects,
    String objectId,
    String? newParentId,
  ) {
    final object = getObjectById(objects, objectId);
    if (object == null) return objects;

    // Prevent moving an object into its own descendant
    if (newParentId != null) {
      final descendants = getDescendantIds(objects, objectId);
      if (newParentId == objectId || descendants.contains(newParentId)) {
        return objects;
      }
    }

    final oldParentId = object.parentId;
    var updated = List<UnifiedObject>.from(objects);

    // Remove from old parent's childrenIds
    if (oldParentId != null) {
      updated = updated.map((o) {
        if (o.id == oldParentId) {
          return o.copyWith(
            childrenIds: o.childrenIds.where((id) => id != objectId).toList(),
            updatedAt: _currentTimestamp(),
          );
        }
        return o;
      }).toList();
    }

    // Add to new parent's childrenIds
    if (newParentId != null) {
      updated = updated.map((o) {
        if (o.id == newParentId && !o.childrenIds.contains(objectId)) {
          return o.copyWith(
            childrenIds: [...o.childrenIds, objectId],
            updatedAt: _currentTimestamp(),
          );
        }
        return o;
      }).toList();
    }

    // Update object's parentId
    updated = updated.map((o) {
      if (o.id == objectId) {
        return o.copyWith(
          parentId: newParentId,
          updatedAt: _currentTimestamp(),
        );
      }
      return o;
    }).toList();

    return updated;
  }

  /// Reorder children within a parent by moving from [oldIndex] to [newIndex].
  List<UnifiedObject> reorderChildren(
    List<UnifiedObject> objects,
    String parentId,
    int oldIndex,
    int newIndex,
  ) {
    final parent = getObjectById(objects, parentId);
    if (parent == null) return objects;
    final children = List<String>.from(parent.childrenIds);
    if (oldIndex < 0 || oldIndex >= children.length) return objects;
    if (newIndex < 0 || newIndex >= children.length) return objects;
    if (oldIndex == newIndex) return objects;

    final item = children.removeAt(oldIndex);
    children.insert(newIndex, item);

    return objects.map((o) {
      if (o.id == parentId) {
        return o.copyWith(
          childrenIds: children,
          updatedAt: _currentTimestamp(),
        );
      }
      return o;
    }).toList();
  }

  /// Add a child reference to a parent's childrenIds.
  List<UnifiedObject> addChild(
    List<UnifiedObject> objects,
    String parentId,
    String childId,
  ) {
    return objects.map((o) {
      if (o.id == parentId && !o.childrenIds.contains(childId)) {
        return o.copyWith(
          childrenIds: [...o.childrenIds, childId],
          updatedAt: _currentTimestamp(),
        );
      }
      return o;
    }).toList();
  }

  /// Remove a child reference from a parent's childrenIds.
  List<UnifiedObject> removeChild(
    List<UnifiedObject> objects,
    String parentId,
    String childId,
  ) {
    return objects.map((o) {
      if (o.id == parentId) {
        return o.copyWith(
          childrenIds: o.childrenIds.where((id) => id != childId).toList(),
          updatedAt: _currentTimestamp(),
        );
      }
      return o;
    }).toList();
  }

  // ---------------------------------------------------------------------------
  // Batch / List Helpers
  // ---------------------------------------------------------------------------

  List<UnifiedObject> addObject(
    List<UnifiedObject> objects,
    UnifiedObject newObject,
  ) {
    return [...objects, newObject];
  }

  List<UnifiedObject> replaceObject(
    List<UnifiedObject> objects,
    UnifiedObject updatedObject,
  ) {
    return objects.map((o) => o.id == updatedObject.id ? updatedObject : o).toList();
  }

  List<UnifiedObject> removeObject(
    List<UnifiedObject> objects,
    String id,
  ) {
    return objects.where((o) => o.id != id).toList();
  }

  // ---------------------------------------------------------------------------
  // Icon Mapping
  // ---------------------------------------------------------------------------

  static IconData getIconFromName(String iconName) {
    return switch (iconName) {
      'article' => Icons.article_outlined,
      'folder' => Icons.folder_outlined,
      'note' => Icons.note_outlined,
      'check_circle' => Icons.check_circle_outlined,
      'person' => Icons.person_outlined,
      'flight' => Icons.flight,
      'work' => Icons.work,
      'school' => Icons.school,
      'account_balance' => Icons.account_balance,
      'credit_card' => Icons.credit_card,
      'home' => Icons.home,
      'language' => Icons.language,
      'star' => Icons.star,
      'badge' => Icons.badge,
      'history' => Icons.history,
      'book' => Icons.book,
      'favorite' => Icons.favorite,
      'security' => Icons.security,
      'vpn_key' => Icons.vpn_key,
      'medical_services' => Icons.medical_services,
      'car_rental' => Icons.car_rental,
      'hotel' => Icons.hotel,
      'restaurant' => Icons.restaurant,
      'shopping_bag' => Icons.shopping_bag,
      'sports' => Icons.sports,
      'music_note' => Icons.music_note,
      'movie' => Icons.movie,
      'camera' => Icons.camera_alt,
      'pets' => Icons.pets,
      'fitness_center' => Icons.fitness_center,
      'local_hospital' => Icons.local_hospital,
      'phone' => Icons.phone,
      'email' => Icons.email,
      'link' => Icons.link,
      'description' => Icons.description,
      'add' => Icons.add,
      _ => Icons.folder_outlined,
    };
  }
}
