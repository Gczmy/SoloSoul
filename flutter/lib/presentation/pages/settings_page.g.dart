// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'settings_page.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(DebugMode)
const debugModeProvider = DebugModeProvider._();

final class DebugModeProvider extends $NotifierProvider<DebugMode, bool> {
  const DebugModeProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'debugModeProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$debugModeHash();

  @$internal
  @override
  DebugMode create() => DebugMode();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(bool value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<bool>(value),
    );
  }
}

String _$debugModeHash() => r'85c81a1e4961c3dd722908743dd3a8e0441f078e';

abstract class _$DebugMode extends $Notifier<bool> {
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
