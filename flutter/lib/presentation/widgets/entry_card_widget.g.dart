// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'entry_card_widget.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning
/// Provider for per-item history expanded state, keyed by itemId or title.

@ProviderFor(HistoryExpanded)
const historyExpandedProvider = HistoryExpandedFamily._();

/// Provider for per-item history expanded state, keyed by itemId or title.
final class HistoryExpandedProvider
    extends $NotifierProvider<HistoryExpanded, bool> {
  /// Provider for per-item history expanded state, keyed by itemId or title.
  const HistoryExpandedProvider._({
    required HistoryExpandedFamily super.from,
    required String super.argument,
  }) : super(
         retry: null,
         name: r'historyExpandedProvider',
         isAutoDispose: true,
         dependencies: null,
         $allTransitiveDependencies: null,
       );

  @override
  String debugGetCreateSourceHash() => _$historyExpandedHash();

  @override
  String toString() {
    return r'historyExpandedProvider'
        ''
        '($argument)';
  }

  @$internal
  @override
  HistoryExpanded create() => HistoryExpanded();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(bool value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<bool>(value),
    );
  }

  @override
  bool operator ==(Object other) {
    return other is HistoryExpandedProvider && other.argument == argument;
  }

  @override
  int get hashCode {
    return argument.hashCode;
  }
}

String _$historyExpandedHash() => r'4b7e7f55cc83fad3a8f026050d68b9272defd01e';

/// Provider for per-item history expanded state, keyed by itemId or title.

final class HistoryExpandedFamily extends $Family
    with $ClassFamilyOverride<HistoryExpanded, bool, bool, bool, String> {
  const HistoryExpandedFamily._()
    : super(
        retry: null,
        name: r'historyExpandedProvider',
        dependencies: null,
        $allTransitiveDependencies: null,
        isAutoDispose: true,
      );

  /// Provider for per-item history expanded state, keyed by itemId or title.

  HistoryExpandedProvider call(String key) =>
      HistoryExpandedProvider._(argument: key, from: this);

  @override
  String toString() => r'historyExpandedProvider';
}

/// Provider for per-item history expanded state, keyed by itemId or title.

abstract class _$HistoryExpanded extends $Notifier<bool> {
  late final _$args = ref.$arg as String;
  String get key => _$args;

  bool build(String key);
  @$mustCallSuper
  @override
  void runBuild() {
    final created = build(_$args);
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
