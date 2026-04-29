// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'sensitivity_provider.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning
/// OPTIMIZED: Effective sensitivity level for a specific field.
/// Uses select() to narrow watch scope - only rebuilds when THIS fieldId changes.

@ProviderFor(effectiveSensitivity)
const effectiveSensitivityProvider = EffectiveSensitivityFamily._();

/// OPTIMIZED: Effective sensitivity level for a specific field.
/// Uses select() to narrow watch scope - only rebuilds when THIS fieldId changes.

final class EffectiveSensitivityProvider
    extends
        $FunctionalProvider<
          SensitivityLevel,
          SensitivityLevel,
          SensitivityLevel
        >
    with $Provider<SensitivityLevel> {
  /// OPTIMIZED: Effective sensitivity level for a specific field.
  /// Uses select() to narrow watch scope - only rebuilds when THIS fieldId changes.
  const EffectiveSensitivityProvider._({
    required EffectiveSensitivityFamily super.from,
    required String super.argument,
  }) : super(
         retry: null,
         name: r'effectiveSensitivityProvider',
         isAutoDispose: true,
         dependencies: null,
         $allTransitiveDependencies: null,
       );

  @override
  String debugGetCreateSourceHash() => _$effectiveSensitivityHash();

  @override
  String toString() {
    return r'effectiveSensitivityProvider'
        ''
        '($argument)';
  }

  @$internal
  @override
  $ProviderElement<SensitivityLevel> $createElement($ProviderPointer pointer) =>
      $ProviderElement(pointer);

  @override
  SensitivityLevel create(Ref ref) {
    final argument = this.argument as String;
    return effectiveSensitivity(ref, argument);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(SensitivityLevel value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<SensitivityLevel>(value),
    );
  }

  @override
  bool operator ==(Object other) {
    return other is EffectiveSensitivityProvider && other.argument == argument;
  }

  @override
  int get hashCode {
    return argument.hashCode;
  }
}

String _$effectiveSensitivityHash() =>
    r'81b7cd7e3a97f8372050db8db988b5c0cf3bb4c7';

/// OPTIMIZED: Effective sensitivity level for a specific field.
/// Uses select() to narrow watch scope - only rebuilds when THIS fieldId changes.

final class EffectiveSensitivityFamily extends $Family
    with $FunctionalFamilyOverride<SensitivityLevel, String> {
  const EffectiveSensitivityFamily._()
    : super(
        retry: null,
        name: r'effectiveSensitivityProvider',
        dependencies: null,
        $allTransitiveDependencies: null,
        isAutoDispose: true,
      );

  /// OPTIMIZED: Effective sensitivity level for a specific field.
  /// Uses select() to narrow watch scope - only rebuilds when THIS fieldId changes.

  EffectiveSensitivityProvider call(String fieldId) =>
      EffectiveSensitivityProvider._(argument: fieldId, from: this);

  @override
  String toString() => r'effectiveSensitivityProvider';
}

/// Provider for field metadata (name, section, etc.) for settings page display.

@ProviderFor(fieldMetadata)
const fieldMetadataProvider = FieldMetadataFamily._();

/// Provider for field metadata (name, section, etc.) for settings page display.

final class FieldMetadataProvider
    extends
        $FunctionalProvider<
          FieldSensitivity?,
          FieldSensitivity?,
          FieldSensitivity?
        >
    with $Provider<FieldSensitivity?> {
  /// Provider for field metadata (name, section, etc.) for settings page display.
  const FieldMetadataProvider._({
    required FieldMetadataFamily super.from,
    required String super.argument,
  }) : super(
         retry: null,
         name: r'fieldMetadataProvider',
         isAutoDispose: true,
         dependencies: null,
         $allTransitiveDependencies: null,
       );

  @override
  String debugGetCreateSourceHash() => _$fieldMetadataHash();

  @override
  String toString() {
    return r'fieldMetadataProvider'
        ''
        '($argument)';
  }

  @$internal
  @override
  $ProviderElement<FieldSensitivity?> $createElement(
    $ProviderPointer pointer,
  ) => $ProviderElement(pointer);

  @override
  FieldSensitivity? create(Ref ref) {
    final argument = this.argument as String;
    return fieldMetadata(ref, argument);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(FieldSensitivity? value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<FieldSensitivity?>(value),
    );
  }

  @override
  bool operator ==(Object other) {
    return other is FieldMetadataProvider && other.argument == argument;
  }

  @override
  int get hashCode {
    return argument.hashCode;
  }
}

String _$fieldMetadataHash() => r'e5696a5ec04520a1378dd80f72b9d2b70650e8ab';

/// Provider for field metadata (name, section, etc.) for settings page display.

final class FieldMetadataFamily extends $Family
    with $FunctionalFamilyOverride<FieldSensitivity?, String> {
  const FieldMetadataFamily._()
    : super(
        retry: null,
        name: r'fieldMetadataProvider',
        dependencies: null,
        $allTransitiveDependencies: null,
        isAutoDispose: true,
      );

  /// Provider for field metadata (name, section, etc.) for settings page display.

  FieldMetadataProvider call(String fieldId) =>
      FieldMetadataProvider._(argument: fieldId, from: this);

  @override
  String toString() => r'fieldMetadataProvider';
}

/// Aggregated sensitivity map for all trash item types.
/// Replaces per-item-type ref.watch loops in trash_page.dart with a single provider watch.

@ProviderFor(trashItemSensitivityMap)
const trashItemSensitivityMapProvider = TrashItemSensitivityMapProvider._();

/// Aggregated sensitivity map for all trash item types.
/// Replaces per-item-type ref.watch loops in trash_page.dart with a single provider watch.

final class TrashItemSensitivityMapProvider
    extends
        $FunctionalProvider<
          Map<String, SensitivityLevel>,
          Map<String, SensitivityLevel>,
          Map<String, SensitivityLevel>
        >
    with $Provider<Map<String, SensitivityLevel>> {
  /// Aggregated sensitivity map for all trash item types.
  /// Replaces per-item-type ref.watch loops in trash_page.dart with a single provider watch.
  const TrashItemSensitivityMapProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'trashItemSensitivityMapProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$trashItemSensitivityMapHash();

  @$internal
  @override
  $ProviderElement<Map<String, SensitivityLevel>> $createElement(
    $ProviderPointer pointer,
  ) => $ProviderElement(pointer);

  @override
  Map<String, SensitivityLevel> create(Ref ref) {
    return trashItemSensitivityMap(ref);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(Map<String, SensitivityLevel> value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<Map<String, SensitivityLevel>>(
        value,
      ),
    );
  }
}

String _$trashItemSensitivityMapHash() =>
    r'cd7e0abb44c7234aab3c5fd52bb278250c4fc2b6';
