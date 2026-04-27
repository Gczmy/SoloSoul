// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'unified_object_provider.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning
/// All root-level objects (parentId == null), active only.

@ProviderFor(rootObjects)
const rootObjectsProvider = RootObjectsProvider._();

/// All root-level objects (parentId == null), active only.

final class RootObjectsProvider
    extends
        $FunctionalProvider<
          List<UnifiedObject>,
          List<UnifiedObject>,
          List<UnifiedObject>
        >
    with $Provider<List<UnifiedObject>> {
  /// All root-level objects (parentId == null), active only.
  const RootObjectsProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'rootObjectsProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$rootObjectsHash();

  @$internal
  @override
  $ProviderElement<List<UnifiedObject>> $createElement(
    $ProviderPointer pointer,
  ) => $ProviderElement(pointer);

  @override
  List<UnifiedObject> create(Ref ref) {
    return rootObjects(ref);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(List<UnifiedObject> value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<List<UnifiedObject>>(value),
    );
  }
}

String _$rootObjectsHash() => r'd3325302d65c21e7cd8e9080a6d528d4be0c19a3';

/// Direct children of a specific parent, in childrenIds order, active only.

@ProviderFor(children)
const childrenProvider = ChildrenFamily._();

/// Direct children of a specific parent, in childrenIds order, active only.

final class ChildrenProvider
    extends
        $FunctionalProvider<
          List<UnifiedObject>,
          List<UnifiedObject>,
          List<UnifiedObject>
        >
    with $Provider<List<UnifiedObject>> {
  /// Direct children of a specific parent, in childrenIds order, active only.
  const ChildrenProvider._({
    required ChildrenFamily super.from,
    required String super.argument,
  }) : super(
         retry: null,
         name: r'childrenProvider',
         isAutoDispose: true,
         dependencies: null,
         $allTransitiveDependencies: null,
       );

  @override
  String debugGetCreateSourceHash() => _$childrenHash();

  @override
  String toString() {
    return r'childrenProvider'
        ''
        '($argument)';
  }

  @$internal
  @override
  $ProviderElement<List<UnifiedObject>> $createElement(
    $ProviderPointer pointer,
  ) => $ProviderElement(pointer);

  @override
  List<UnifiedObject> create(Ref ref) {
    final argument = this.argument as String;
    return children(ref, argument);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(List<UnifiedObject> value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<List<UnifiedObject>>(value),
    );
  }

  @override
  bool operator ==(Object other) {
    return other is ChildrenProvider && other.argument == argument;
  }

  @override
  int get hashCode {
    return argument.hashCode;
  }
}

String _$childrenHash() => r'ed0b4fd238530b9492ae2d23594cf8a34255c232';

/// Direct children of a specific parent, in childrenIds order, active only.

final class ChildrenFamily extends $Family
    with $FunctionalFamilyOverride<List<UnifiedObject>, String> {
  const ChildrenFamily._()
    : super(
        retry: null,
        name: r'childrenProvider',
        dependencies: null,
        $allTransitiveDependencies: null,
        isAutoDispose: true,
      );

  /// Direct children of a specific parent, in childrenIds order, active only.

  ChildrenProvider call(String parentId) =>
      ChildrenProvider._(argument: parentId, from: this);

  @override
  String toString() => r'childrenProvider';
}

/// Get a specific object by ID.

@ProviderFor(objectById)
const objectByIdProvider = ObjectByIdFamily._();

/// Get a specific object by ID.

final class ObjectByIdProvider
    extends $FunctionalProvider<UnifiedObject?, UnifiedObject?, UnifiedObject?>
    with $Provider<UnifiedObject?> {
  /// Get a specific object by ID.
  const ObjectByIdProvider._({
    required ObjectByIdFamily super.from,
    required String super.argument,
  }) : super(
         retry: null,
         name: r'objectByIdProvider',
         isAutoDispose: true,
         dependencies: null,
         $allTransitiveDependencies: null,
       );

  @override
  String debugGetCreateSourceHash() => _$objectByIdHash();

  @override
  String toString() {
    return r'objectByIdProvider'
        ''
        '($argument)';
  }

  @$internal
  @override
  $ProviderElement<UnifiedObject?> $createElement($ProviderPointer pointer) =>
      $ProviderElement(pointer);

  @override
  UnifiedObject? create(Ref ref) {
    final argument = this.argument as String;
    return objectById(ref, argument);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(UnifiedObject? value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<UnifiedObject?>(value),
    );
  }

  @override
  bool operator ==(Object other) {
    return other is ObjectByIdProvider && other.argument == argument;
  }

  @override
  int get hashCode {
    return argument.hashCode;
  }
}

String _$objectByIdHash() => r'a5a4217f08f726f4fc0bb4b7faf6d87b6f752547';

/// Get a specific object by ID.

final class ObjectByIdFamily extends $Family
    with $FunctionalFamilyOverride<UnifiedObject?, String> {
  const ObjectByIdFamily._()
    : super(
        retry: null,
        name: r'objectByIdProvider',
        dependencies: null,
        $allTransitiveDependencies: null,
        isAutoDispose: true,
      );

  /// Get a specific object by ID.

  ObjectByIdProvider call(String id) =>
      ObjectByIdProvider._(argument: id, from: this);

  @override
  String toString() => r'objectByIdProvider';
}

/// Get all active objects of a given type.

@ProviderFor(objectsByType)
const objectsByTypeProvider = ObjectsByTypeFamily._();

/// Get all active objects of a given type.

final class ObjectsByTypeProvider
    extends
        $FunctionalProvider<
          List<UnifiedObject>,
          List<UnifiedObject>,
          List<UnifiedObject>
        >
    with $Provider<List<UnifiedObject>> {
  /// Get all active objects of a given type.
  const ObjectsByTypeProvider._({
    required ObjectsByTypeFamily super.from,
    required String super.argument,
  }) : super(
         retry: null,
         name: r'objectsByTypeProvider',
         isAutoDispose: true,
         dependencies: null,
         $allTransitiveDependencies: null,
       );

  @override
  String debugGetCreateSourceHash() => _$objectsByTypeHash();

  @override
  String toString() {
    return r'objectsByTypeProvider'
        ''
        '($argument)';
  }

  @$internal
  @override
  $ProviderElement<List<UnifiedObject>> $createElement(
    $ProviderPointer pointer,
  ) => $ProviderElement(pointer);

  @override
  List<UnifiedObject> create(Ref ref) {
    final argument = this.argument as String;
    return objectsByType(ref, argument);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(List<UnifiedObject> value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<List<UnifiedObject>>(value),
    );
  }

  @override
  bool operator ==(Object other) {
    return other is ObjectsByTypeProvider && other.argument == argument;
  }

  @override
  int get hashCode {
    return argument.hashCode;
  }
}

String _$objectsByTypeHash() => r'fd74915173874f085bf80871f740a769ca58a011';

/// Get all active objects of a given type.

final class ObjectsByTypeFamily extends $Family
    with $FunctionalFamilyOverride<List<UnifiedObject>, String> {
  const ObjectsByTypeFamily._()
    : super(
        retry: null,
        name: r'objectsByTypeProvider',
        dependencies: null,
        $allTransitiveDependencies: null,
        isAutoDispose: true,
      );

  /// Get all active objects of a given type.

  ObjectsByTypeProvider call(String typeId) =>
      ObjectsByTypeProvider._(argument: typeId, from: this);

  @override
  String toString() => r'objectsByTypeProvider';
}

/// Get all soft-deleted objects.

@ProviderFor(deletedObjects)
const deletedObjectsProvider = DeletedObjectsProvider._();

/// Get all soft-deleted objects.

final class DeletedObjectsProvider
    extends
        $FunctionalProvider<
          List<UnifiedObject>,
          List<UnifiedObject>,
          List<UnifiedObject>
        >
    with $Provider<List<UnifiedObject>> {
  /// Get all soft-deleted objects.
  const DeletedObjectsProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'deletedObjectsProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$deletedObjectsHash();

  @$internal
  @override
  $ProviderElement<List<UnifiedObject>> $createElement(
    $ProviderPointer pointer,
  ) => $ProviderElement(pointer);

  @override
  List<UnifiedObject> create(Ref ref) {
    return deletedObjects(ref);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(List<UnifiedObject> value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<List<UnifiedObject>>(value),
    );
  }
}

String _$deletedObjectsHash() => r'95a00201d850789631313cedbc1ba2bc6ed3f03b';
