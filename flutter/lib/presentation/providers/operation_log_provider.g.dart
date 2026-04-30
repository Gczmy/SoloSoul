// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'operation_log_provider.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(OperationLogEntries)
const operationLogEntriesProvider = OperationLogEntriesProvider._();

final class OperationLogEntriesProvider
    extends $NotifierProvider<OperationLogEntries, List<OperationEntry>> {
  const OperationLogEntriesProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'operationLogEntriesProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$operationLogEntriesHash();

  @$internal
  @override
  OperationLogEntries create() => OperationLogEntries();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(List<OperationEntry> value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<List<OperationEntry>>(value),
    );
  }
}

String _$operationLogEntriesHash() =>
    r'1e17ae57c998a92c3b2d89168bed758ef388668d';

abstract class _$OperationLogEntries extends $Notifier<List<OperationEntry>> {
  List<OperationEntry> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final created = build();
    final ref = this.ref as $Ref<List<OperationEntry>, List<OperationEntry>>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<List<OperationEntry>, List<OperationEntry>>,
              List<OperationEntry>,
              Object?,
              Object?
            >;
    element.handleValue(ref, created);
  }
}

@ProviderFor(LogActionFilter)
const logActionFilterProvider = LogActionFilterProvider._();

final class LogActionFilterProvider
    extends $NotifierProvider<LogActionFilter, Set<String>> {
  const LogActionFilterProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'logActionFilterProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$logActionFilterHash();

  @$internal
  @override
  LogActionFilter create() => LogActionFilter();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(Set<String> value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<Set<String>>(value),
    );
  }
}

String _$logActionFilterHash() => r'b1f64df29a0c627c862ef52718ba5ee5e00a89c4';

abstract class _$LogActionFilter extends $Notifier<Set<String>> {
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

@ProviderFor(LogDeviceFilter)
const logDeviceFilterProvider = LogDeviceFilterProvider._();

final class LogDeviceFilterProvider
    extends $NotifierProvider<LogDeviceFilter, Set<String>> {
  const LogDeviceFilterProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'logDeviceFilterProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$logDeviceFilterHash();

  @$internal
  @override
  LogDeviceFilter create() => LogDeviceFilter();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(Set<String> value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<Set<String>>(value),
    );
  }
}

String _$logDeviceFilterHash() => r'dfcdd35cb3be966eb65cee9b2a25444b0bd85106';

abstract class _$LogDeviceFilter extends $Notifier<Set<String>> {
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

@ProviderFor(OperationLogFilteredEntries)
const operationLogFilteredEntriesProvider =
    OperationLogFilteredEntriesProvider._();

final class OperationLogFilteredEntriesProvider
    extends
        $NotifierProvider<OperationLogFilteredEntries, List<OperationEntry>> {
  const OperationLogFilteredEntriesProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'operationLogFilteredEntriesProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$operationLogFilteredEntriesHash();

  @$internal
  @override
  OperationLogFilteredEntries create() => OperationLogFilteredEntries();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(List<OperationEntry> value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<List<OperationEntry>>(value),
    );
  }
}

String _$operationLogFilteredEntriesHash() =>
    r'bfd793a2b82534ca01e259892c8539ab0c294c27';

abstract class _$OperationLogFilteredEntries
    extends $Notifier<List<OperationEntry>> {
  List<OperationEntry> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final created = build();
    final ref = this.ref as $Ref<List<OperationEntry>, List<OperationEntry>>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<List<OperationEntry>, List<OperationEntry>>,
              List<OperationEntry>,
              Object?,
              Object?
            >;
    element.handleValue(ref, created);
  }
}
