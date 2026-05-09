// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'trash_filter_provider.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning
/// Provider for the selected time filter in trash view.
/// null or 'all' means show all items regardless of time.

@ProviderFor(TrashTimeFilter)
const trashTimeFilterProvider = TrashTimeFilterProvider._();

/// Provider for the selected time filter in trash view.
/// null or 'all' means show all items regardless of time.
final class TrashTimeFilterProvider
    extends $NotifierProvider<TrashTimeFilter, String?> {
  /// Provider for the selected time filter in trash view.
  /// null or 'all' means show all items regardless of time.
  const TrashTimeFilterProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'trashTimeFilterProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$trashTimeFilterHash();

  @$internal
  @override
  TrashTimeFilter create() => TrashTimeFilter();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(String? value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<String?>(value),
    );
  }
}

String _$trashTimeFilterHash() => r'c4a4d8f1cd903a871272b3afed8bd53dfa04f297';

/// Provider for the selected time filter in trash view.
/// null or 'all' means show all items regardless of time.

abstract class _$TrashTimeFilter extends $Notifier<String?> {
  String? build();
  @$mustCallSuper
  @override
  void runBuild() {
    final created = build();
    final ref = this.ref as $Ref<String?, String?>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<String?, String?>,
              String?,
              Object?,
              Object?
            >;
    element.handleValue(ref, created);
  }
}

/// Provider for the selected type filters in trash view.
/// Empty set means show all types.

@ProviderFor(TrashTypeFilter)
const trashTypeFilterProvider = TrashTypeFilterProvider._();

/// Provider for the selected type filters in trash view.
/// Empty set means show all types.
final class TrashTypeFilterProvider
    extends $NotifierProvider<TrashTypeFilter, Set<String>> {
  /// Provider for the selected type filters in trash view.
  /// Empty set means show all types.
  const TrashTypeFilterProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'trashTypeFilterProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$trashTypeFilterHash();

  @$internal
  @override
  TrashTypeFilter create() => TrashTypeFilter();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(Set<String> value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<Set<String>>(value),
    );
  }
}

String _$trashTypeFilterHash() => r'44e8e0c4e5ff98b1d53561ce40297ea7c5322d36';

/// Provider for the selected type filters in trash view.
/// Empty set means show all types.

abstract class _$TrashTypeFilter extends $Notifier<Set<String>> {
  Set<String> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final created = build();
    final ref = this.ref as $Ref<Set<String>, Set<String>>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<Set<String>, Set<String>>,
              Set<String>,
              Object?,
              Object?
            >;
    element.handleValue(ref, created);
  }
}
