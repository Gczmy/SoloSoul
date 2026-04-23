// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'sensitivity_provider.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

String _$effectiveSensitivityHash() =>
    r'81b7cd7e3a97f8372050db8db988b5c0cf3bb4c7';

/// Copied from Dart SDK
class _SystemHash {
  _SystemHash._();

  static int combine(int hash, int value) {
    // ignore: parameter_assignments
    hash = 0x1fffffff & (hash + value);
    // ignore: parameter_assignments
    hash = 0x1fffffff & (hash + ((0x0007ffff & hash) << 10));
    return hash ^ (hash >> 6);
  }

  static int finish(int hash) {
    // ignore: parameter_assignments
    hash = 0x1fffffff & (hash + ((0x03ffffff & hash) << 3));
    // ignore: parameter_assignments
    hash = hash ^ (hash >> 11);
    return 0x1fffffff & (hash + ((0x00003fff & hash) << 15));
  }
}

/// OPTIMIZED: Effective sensitivity level for a specific field.
/// Uses select() to narrow watch scope - only rebuilds when THIS fieldId changes.
///
/// Copied from [effectiveSensitivity].
@ProviderFor(effectiveSensitivity)
const effectiveSensitivityProvider = EffectiveSensitivityFamily();

/// OPTIMIZED: Effective sensitivity level for a specific field.
/// Uses select() to narrow watch scope - only rebuilds when THIS fieldId changes.
///
/// Copied from [effectiveSensitivity].
class EffectiveSensitivityFamily extends Family<SensitivityLevel> {
  /// OPTIMIZED: Effective sensitivity level for a specific field.
  /// Uses select() to narrow watch scope - only rebuilds when THIS fieldId changes.
  ///
  /// Copied from [effectiveSensitivity].
  const EffectiveSensitivityFamily();

  /// OPTIMIZED: Effective sensitivity level for a specific field.
  /// Uses select() to narrow watch scope - only rebuilds when THIS fieldId changes.
  ///
  /// Copied from [effectiveSensitivity].
  EffectiveSensitivityProvider call(String fieldId) {
    return EffectiveSensitivityProvider(fieldId);
  }

  @override
  EffectiveSensitivityProvider getProviderOverride(
    covariant EffectiveSensitivityProvider provider,
  ) {
    return call(provider.fieldId);
  }

  static const Iterable<ProviderOrFamily>? _dependencies = null;

  @override
  Iterable<ProviderOrFamily>? get dependencies => _dependencies;

  static const Iterable<ProviderOrFamily>? _allTransitiveDependencies = null;

  @override
  Iterable<ProviderOrFamily>? get allTransitiveDependencies =>
      _allTransitiveDependencies;

  @override
  String? get name => r'effectiveSensitivityProvider';
}

/// OPTIMIZED: Effective sensitivity level for a specific field.
/// Uses select() to narrow watch scope - only rebuilds when THIS fieldId changes.
///
/// Copied from [effectiveSensitivity].
class EffectiveSensitivityProvider
    extends AutoDisposeProvider<SensitivityLevel> {
  /// OPTIMIZED: Effective sensitivity level for a specific field.
  /// Uses select() to narrow watch scope - only rebuilds when THIS fieldId changes.
  ///
  /// Copied from [effectiveSensitivity].
  EffectiveSensitivityProvider(String fieldId)
    : this._internal(
        (ref) => effectiveSensitivity(ref as EffectiveSensitivityRef, fieldId),
        from: effectiveSensitivityProvider,
        name: r'effectiveSensitivityProvider',
        debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
            ? null
            : _$effectiveSensitivityHash,
        dependencies: EffectiveSensitivityFamily._dependencies,
        allTransitiveDependencies:
            EffectiveSensitivityFamily._allTransitiveDependencies,
        fieldId: fieldId,
      );

  EffectiveSensitivityProvider._internal(
    super._createNotifier, {
    required super.name,
    required super.dependencies,
    required super.allTransitiveDependencies,
    required super.debugGetCreateSourceHash,
    required super.from,
    required this.fieldId,
  }) : super.internal();

  final String fieldId;

  @override
  Override overrideWith(
    SensitivityLevel Function(EffectiveSensitivityRef provider) create,
  ) {
    return ProviderOverride(
      origin: this,
      override: EffectiveSensitivityProvider._internal(
        (ref) => create(ref as EffectiveSensitivityRef),
        from: from,
        name: null,
        dependencies: null,
        allTransitiveDependencies: null,
        debugGetCreateSourceHash: null,
        fieldId: fieldId,
      ),
    );
  }

  @override
  AutoDisposeProviderElement<SensitivityLevel> createElement() {
    return _EffectiveSensitivityProviderElement(this);
  }

  @override
  bool operator ==(Object other) {
    return other is EffectiveSensitivityProvider && other.fieldId == fieldId;
  }

  @override
  int get hashCode {
    var hash = _SystemHash.combine(0, runtimeType.hashCode);
    hash = _SystemHash.combine(hash, fieldId.hashCode);

    return _SystemHash.finish(hash);
  }
}

@Deprecated('Will be removed in 3.0. Use Ref instead')
// ignore: unused_element
mixin EffectiveSensitivityRef on AutoDisposeProviderRef<SensitivityLevel> {
  /// The parameter `fieldId` of this provider.
  String get fieldId;
}

class _EffectiveSensitivityProviderElement
    extends AutoDisposeProviderElement<SensitivityLevel>
    with EffectiveSensitivityRef {
  _EffectiveSensitivityProviderElement(super.provider);

  @override
  String get fieldId => (origin as EffectiveSensitivityProvider).fieldId;
}

String _$fieldMetadataHash() => r'e5696a5ec04520a1378dd80f72b9d2b70650e8ab';

/// Provider for field metadata (name, section, etc.) for settings page display.
///
/// Copied from [fieldMetadata].
@ProviderFor(fieldMetadata)
const fieldMetadataProvider = FieldMetadataFamily();

/// Provider for field metadata (name, section, etc.) for settings page display.
///
/// Copied from [fieldMetadata].
class FieldMetadataFamily extends Family<FieldSensitivity?> {
  /// Provider for field metadata (name, section, etc.) for settings page display.
  ///
  /// Copied from [fieldMetadata].
  const FieldMetadataFamily();

  /// Provider for field metadata (name, section, etc.) for settings page display.
  ///
  /// Copied from [fieldMetadata].
  FieldMetadataProvider call(String fieldId) {
    return FieldMetadataProvider(fieldId);
  }

  @override
  FieldMetadataProvider getProviderOverride(
    covariant FieldMetadataProvider provider,
  ) {
    return call(provider.fieldId);
  }

  static const Iterable<ProviderOrFamily>? _dependencies = null;

  @override
  Iterable<ProviderOrFamily>? get dependencies => _dependencies;

  static const Iterable<ProviderOrFamily>? _allTransitiveDependencies = null;

  @override
  Iterable<ProviderOrFamily>? get allTransitiveDependencies =>
      _allTransitiveDependencies;

  @override
  String? get name => r'fieldMetadataProvider';
}

/// Provider for field metadata (name, section, etc.) for settings page display.
///
/// Copied from [fieldMetadata].
class FieldMetadataProvider extends AutoDisposeProvider<FieldSensitivity?> {
  /// Provider for field metadata (name, section, etc.) for settings page display.
  ///
  /// Copied from [fieldMetadata].
  FieldMetadataProvider(String fieldId)
    : this._internal(
        (ref) => fieldMetadata(ref as FieldMetadataRef, fieldId),
        from: fieldMetadataProvider,
        name: r'fieldMetadataProvider',
        debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
            ? null
            : _$fieldMetadataHash,
        dependencies: FieldMetadataFamily._dependencies,
        allTransitiveDependencies:
            FieldMetadataFamily._allTransitiveDependencies,
        fieldId: fieldId,
      );

  FieldMetadataProvider._internal(
    super._createNotifier, {
    required super.name,
    required super.dependencies,
    required super.allTransitiveDependencies,
    required super.debugGetCreateSourceHash,
    required super.from,
    required this.fieldId,
  }) : super.internal();

  final String fieldId;

  @override
  Override overrideWith(
    FieldSensitivity? Function(FieldMetadataRef provider) create,
  ) {
    return ProviderOverride(
      origin: this,
      override: FieldMetadataProvider._internal(
        (ref) => create(ref as FieldMetadataRef),
        from: from,
        name: null,
        dependencies: null,
        allTransitiveDependencies: null,
        debugGetCreateSourceHash: null,
        fieldId: fieldId,
      ),
    );
  }

  @override
  AutoDisposeProviderElement<FieldSensitivity?> createElement() {
    return _FieldMetadataProviderElement(this);
  }

  @override
  bool operator ==(Object other) {
    return other is FieldMetadataProvider && other.fieldId == fieldId;
  }

  @override
  int get hashCode {
    var hash = _SystemHash.combine(0, runtimeType.hashCode);
    hash = _SystemHash.combine(hash, fieldId.hashCode);

    return _SystemHash.finish(hash);
  }
}

@Deprecated('Will be removed in 3.0. Use Ref instead')
// ignore: unused_element
mixin FieldMetadataRef on AutoDisposeProviderRef<FieldSensitivity?> {
  /// The parameter `fieldId` of this provider.
  String get fieldId;
}

class _FieldMetadataProviderElement
    extends AutoDisposeProviderElement<FieldSensitivity?>
    with FieldMetadataRef {
  _FieldMetadataProviderElement(super.provider);

  @override
  String get fieldId => (origin as FieldMetadataProvider).fieldId;
}

// ignore_for_file: type=lint
// ignore_for_file: subtype_of_sealed_class, invalid_use_of_internal_member, invalid_use_of_visible_for_testing_member, deprecated_member_use_from_same_package
