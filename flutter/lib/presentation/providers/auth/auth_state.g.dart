// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'auth_state.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning
/// Provider that checks if sensitive access is currently granted

@ProviderFor(IsSensitiveAccessGranted)
const isSensitiveAccessGrantedProvider = IsSensitiveAccessGrantedProvider._();

/// Provider that checks if sensitive access is currently granted
final class IsSensitiveAccessGrantedProvider
    extends $NotifierProvider<IsSensitiveAccessGranted, bool> {
  /// Provider that checks if sensitive access is currently granted
  const IsSensitiveAccessGrantedProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'isSensitiveAccessGrantedProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$isSensitiveAccessGrantedHash();

  @$internal
  @override
  IsSensitiveAccessGranted create() => IsSensitiveAccessGranted();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(bool value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<bool>(value),
    );
  }
}

String _$isSensitiveAccessGrantedHash() =>
    r'68ecfdb376c379691709eb56adfe43ecd823a885';

/// Provider that checks if sensitive access is currently granted

abstract class _$IsSensitiveAccessGranted extends $Notifier<bool> {
  bool build();
  @$mustCallSuper
  @override
  void runBuild() {
    final created = build();
    final ref = this.ref as $Ref<bool, bool>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<bool, bool>,
              bool,
              Object?,
              Object?
            >;
    element.handleValue(ref, created);
  }
}
