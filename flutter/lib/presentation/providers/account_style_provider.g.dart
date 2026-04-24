// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'account_style_provider.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning
/// Provider for display mode (reuses existing sensitivity settings).

@ProviderFor(DisplayMode)
const displayModeProvider = DisplayModeProvider._();

/// Provider for display mode (reuses existing sensitivity settings).
final class DisplayModeProvider
    extends $NotifierProvider<DisplayMode, SensitivityDisplayMode> {
  /// Provider for display mode (reuses existing sensitivity settings).
  const DisplayModeProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'displayModeProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$displayModeHash();

  @$internal
  @override
  DisplayMode create() => DisplayMode();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(SensitivityDisplayMode value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<SensitivityDisplayMode>(value),
    );
  }
}

String _$displayModeHash() => r'6def24011dc300242c9f74705e6dcb34a4c65c0b';

/// Provider for display mode (reuses existing sensitivity settings).

abstract class _$DisplayMode extends $Notifier<SensitivityDisplayMode> {
  SensitivityDisplayMode build();
  @$mustCallSuper
  @override
  void runBuild() {
    final created = build();
    final ref =
        this.ref as $Ref<SensitivityDisplayMode, SensitivityDisplayMode>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<SensitivityDisplayMode, SensitivityDisplayMode>,
              SensitivityDisplayMode,
              Object?,
              Object?
            >;
    element.handleValue(ref, created);
  }
}
