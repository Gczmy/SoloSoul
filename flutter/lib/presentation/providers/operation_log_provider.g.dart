// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'operation_log_provider.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

String _$operationLogEntriesHash() =>
    r'1e17ae57c998a92c3b2d89168bed758ef388668d';

/// See also [OperationLogEntries].
@ProviderFor(OperationLogEntries)
final operationLogEntriesProvider =
    AutoDisposeNotifierProvider<
      OperationLogEntries,
      List<OperationEntry>
    >.internal(
      OperationLogEntries.new,
      name: r'operationLogEntriesProvider',
      debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
          ? null
          : _$operationLogEntriesHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

typedef _$OperationLogEntries = AutoDisposeNotifier<List<OperationEntry>>;
String _$logActionFilterHash() => r'b1f64df29a0c627c862ef52718ba5ee5e00a89c4';

/// See also [LogActionFilter].
@ProviderFor(LogActionFilter)
final logActionFilterProvider =
    AutoDisposeNotifierProvider<LogActionFilter, Set<String>>.internal(
      LogActionFilter.new,
      name: r'logActionFilterProvider',
      debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
          ? null
          : _$logActionFilterHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

typedef _$LogActionFilter = AutoDisposeNotifier<Set<String>>;
String _$logDeviceFilterHash() => r'dfcdd35cb3be966eb65cee9b2a25444b0bd85106';

/// See also [LogDeviceFilter].
@ProviderFor(LogDeviceFilter)
final logDeviceFilterProvider =
    AutoDisposeNotifierProvider<LogDeviceFilter, Set<String>>.internal(
      LogDeviceFilter.new,
      name: r'logDeviceFilterProvider',
      debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
          ? null
          : _$logDeviceFilterHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

typedef _$LogDeviceFilter = AutoDisposeNotifier<Set<String>>;
String _$logSensitivityFilterHash() =>
    r'755ea84831ffba79c18191659137fedd08b1ebef';

/// See also [LogSensitivityFilter].
@ProviderFor(LogSensitivityFilter)
final logSensitivityFilterProvider =
    AutoDisposeNotifierProvider<
      LogSensitivityFilter,
      Set<SensitivityLevel>
    >.internal(
      LogSensitivityFilter.new,
      name: r'logSensitivityFilterProvider',
      debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
          ? null
          : _$logSensitivityFilterHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

typedef _$LogSensitivityFilter = AutoDisposeNotifier<Set<SensitivityLevel>>;
String _$operationLogFilteredEntriesHash() =>
    r'ce3000ce5f848218a7554a5417f03b5eaf11abb6';

/// See also [OperationLogFilteredEntries].
@ProviderFor(OperationLogFilteredEntries)
final operationLogFilteredEntriesProvider =
    AutoDisposeNotifierProvider<
      OperationLogFilteredEntries,
      List<OperationEntry>
    >.internal(
      OperationLogFilteredEntries.new,
      name: r'operationLogFilteredEntriesProvider',
      debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
          ? null
          : _$operationLogFilteredEntriesHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

typedef _$OperationLogFilteredEntries =
    AutoDisposeNotifier<List<OperationEntry>>;
// ignore_for_file: type=lint
// ignore_for_file: subtype_of_sealed_class, invalid_use_of_internal_member, invalid_use_of_visible_for_testing_member, deprecated_member_use_from_same_package
