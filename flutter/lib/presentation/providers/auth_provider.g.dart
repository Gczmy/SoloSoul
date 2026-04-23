// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'auth_provider.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

String _$accountsVersionHash() => r'7537c353a8ed6b38596d0289ccb8ca55732d294d';

/// Provider that watches accountsVersion from AuthNotifier
///
/// Copied from [AccountsVersion].
@ProviderFor(AccountsVersion)
final accountsVersionProvider =
    AutoDisposeNotifierProvider<AccountsVersion, int>.internal(
      AccountsVersion.new,
      name: r'accountsVersionProvider',
      debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
          ? null
          : _$accountsVersionHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

typedef _$AccountsVersion = AutoDisposeNotifier<int>;
String _$isSensitiveAccessGrantedHash() =>
    r'68ecfdb376c379691709eb56adfe43ecd823a885';

/// Provider that checks if sensitive access is currently granted
///
/// Copied from [IsSensitiveAccessGranted].
@ProviderFor(IsSensitiveAccessGranted)
final isSensitiveAccessGrantedProvider =
    AutoDisposeNotifierProvider<IsSensitiveAccessGranted, bool>.internal(
      IsSensitiveAccessGranted.new,
      name: r'isSensitiveAccessGrantedProvider',
      debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
          ? null
          : _$isSensitiveAccessGrantedHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

typedef _$IsSensitiveAccessGranted = AutoDisposeNotifier<bool>;
// ignore_for_file: type=lint
// ignore_for_file: subtype_of_sealed_class, invalid_use_of_internal_member, invalid_use_of_visible_for_testing_member, deprecated_member_use_from_same_package
