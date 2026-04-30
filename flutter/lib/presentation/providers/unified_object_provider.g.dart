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

String _$rootObjectsHash() => r'8f12e5f8b7fc264376c7e1a5e44065cddf7ace96';

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

String _$childrenHash() => r'2ecae6979da97d25d8594ed9799b19d17509cfbd';

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

String _$objectByIdHash() => r'ec9de1587c153eb828aeffe9dee4d1da8975689a';

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

String _$objectsByTypeHash() => r'75b217b280352b17c8d89fed05dbe846fa72d012';

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

String _$deletedObjectsHash() => r'8868c52a8045ca53d10f5f129797e58a6c51109e';

/// Get a default page by its fixed ID.

@ProviderFor(defaultPage)
const defaultPageProvider = DefaultPageFamily._();

/// Get a default page by its fixed ID.

final class DefaultPageProvider
    extends $FunctionalProvider<UnifiedObject?, UnifiedObject?, UnifiedObject?>
    with $Provider<UnifiedObject?> {
  /// Get a default page by its fixed ID.
  const DefaultPageProvider._({
    required DefaultPageFamily super.from,
    required String super.argument,
  }) : super(
         retry: null,
         name: r'defaultPageProvider',
         isAutoDispose: true,
         dependencies: null,
         $allTransitiveDependencies: null,
       );

  @override
  String debugGetCreateSourceHash() => _$defaultPageHash();

  @override
  String toString() {
    return r'defaultPageProvider'
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
    return defaultPage(ref, argument);
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
    return other is DefaultPageProvider && other.argument == argument;
  }

  @override
  int get hashCode {
    return argument.hashCode;
  }
}

String _$defaultPageHash() => r'd3e1b4bf247be2e14f2181917573e056fd39b9ee';

/// Get a default page by its fixed ID.

final class DefaultPageFamily extends $Family
    with $FunctionalFamilyOverride<UnifiedObject?, String> {
  const DefaultPageFamily._()
    : super(
        retry: null,
        name: r'defaultPageProvider',
        dependencies: null,
        $allTransitiveDependencies: null,
        isAutoDispose: true,
      );

  /// Get a default page by its fixed ID.

  DefaultPageProvider call(String pageId) =>
      DefaultPageProvider._(argument: pageId, from: this);

  @override
  String toString() => r'defaultPageProvider';
}

/// Get a default section by its fixed ID.

@ProviderFor(defaultSection)
const defaultSectionProvider = DefaultSectionFamily._();

/// Get a default section by its fixed ID.

final class DefaultSectionProvider
    extends $FunctionalProvider<UnifiedObject?, UnifiedObject?, UnifiedObject?>
    with $Provider<UnifiedObject?> {
  /// Get a default section by its fixed ID.
  const DefaultSectionProvider._({
    required DefaultSectionFamily super.from,
    required String super.argument,
  }) : super(
         retry: null,
         name: r'defaultSectionProvider',
         isAutoDispose: true,
         dependencies: null,
         $allTransitiveDependencies: null,
       );

  @override
  String debugGetCreateSourceHash() => _$defaultSectionHash();

  @override
  String toString() {
    return r'defaultSectionProvider'
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
    return defaultSection(ref, argument);
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
    return other is DefaultSectionProvider && other.argument == argument;
  }

  @override
  int get hashCode {
    return argument.hashCode;
  }
}

String _$defaultSectionHash() => r'9a31e3a03abd2146f35697b999c12ed2674a5163';

/// Get a default section by its fixed ID.

final class DefaultSectionFamily extends $Family
    with $FunctionalFamilyOverride<UnifiedObject?, String> {
  const DefaultSectionFamily._()
    : super(
        retry: null,
        name: r'defaultSectionProvider',
        dependencies: null,
        $allTransitiveDependencies: null,
        isAutoDispose: true,
      );

  /// Get a default section by its fixed ID.

  DefaultSectionProvider call(String sectionId) =>
      DefaultSectionProvider._(argument: sectionId, from: this);

  @override
  String toString() => r'defaultSectionProvider';
}

/// Get active items under a default section, ordered by section's childrenIds.

@ProviderFor(defaultPageItems)
const defaultPageItemsProvider = DefaultPageItemsFamily._();

/// Get active items under a default section, ordered by section's childrenIds.

final class DefaultPageItemsProvider
    extends
        $FunctionalProvider<
          List<UnifiedObject>,
          List<UnifiedObject>,
          List<UnifiedObject>
        >
    with $Provider<List<UnifiedObject>> {
  /// Get active items under a default section, ordered by section's childrenIds.
  const DefaultPageItemsProvider._({
    required DefaultPageItemsFamily super.from,
    required String super.argument,
  }) : super(
         retry: null,
         name: r'defaultPageItemsProvider',
         isAutoDispose: true,
         dependencies: null,
         $allTransitiveDependencies: null,
       );

  @override
  String debugGetCreateSourceHash() => _$defaultPageItemsHash();

  @override
  String toString() {
    return r'defaultPageItemsProvider'
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
    return defaultPageItems(ref, argument);
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
    return other is DefaultPageItemsProvider && other.argument == argument;
  }

  @override
  int get hashCode {
    return argument.hashCode;
  }
}

String _$defaultPageItemsHash() => r'7166602a8eaf662a2b6b015496731fe81bab7e9d';

/// Get active items under a default section, ordered by section's childrenIds.

final class DefaultPageItemsFamily extends $Family
    with $FunctionalFamilyOverride<List<UnifiedObject>, String> {
  const DefaultPageItemsFamily._()
    : super(
        retry: null,
        name: r'defaultPageItemsProvider',
        dependencies: null,
        $allTransitiveDependencies: null,
        isAutoDispose: true,
      );

  /// Get active items under a default section, ordered by section's childrenIds.

  DefaultPageItemsProvider call(String sectionId) =>
      DefaultPageItemsProvider._(argument: sectionId, from: this);

  @override
  String toString() => r'defaultPageItemsProvider';
}
