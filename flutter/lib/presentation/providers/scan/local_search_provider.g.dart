// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'local_search_provider.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(LocalSearchNotifier)
const localSearchProvider = LocalSearchNotifierProvider._();

final class LocalSearchNotifierProvider
    extends $NotifierProvider<LocalSearchNotifier, LocalSearchState> {
  const LocalSearchNotifierProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'localSearchProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$localSearchNotifierHash();

  @$internal
  @override
  LocalSearchNotifier create() => LocalSearchNotifier();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(LocalSearchState value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<LocalSearchState>(value),
    );
  }
}

String _$localSearchNotifierHash() =>
    r'dd477108bcce7bf51c37828419d3c9f77a1d1d23';

abstract class _$LocalSearchNotifier extends $Notifier<LocalSearchState> {
  LocalSearchState build();
  @$mustCallSuper
  @override
  void runBuild() {
    final created = build();
    final ref = this.ref as $Ref<LocalSearchState, LocalSearchState>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<LocalSearchState, LocalSearchState>,
              LocalSearchState,
              Object?,
              Object?
            >;
    element.handleValue(ref, created);
  }
}
