// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'auth_notifier.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning
/// Provider that watches accountsVersion from AuthNotifier

@ProviderFor(AccountsVersion)
const accountsVersionProvider = AccountsVersionProvider._();

/// Provider that watches accountsVersion from AuthNotifier
final class AccountsVersionProvider
    extends $NotifierProvider<AccountsVersion, int> {
  /// Provider that watches accountsVersion from AuthNotifier
  const AccountsVersionProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'accountsVersionProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$accountsVersionHash();

  @$internal
  @override
  AccountsVersion create() => AccountsVersion();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(int value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<int>(value),
    );
  }
}

String _$accountsVersionHash() => r'673536db557e37cf0c88b3b41a22d7c9d7f27221';

/// Provider that watches accountsVersion from AuthNotifier

abstract class _$AccountsVersion extends $Notifier<int> {
  int build();
  @$mustCallSuper
  @override
  void runBuild() {
    final created = build();
    final ref = this.ref as $Ref<int, int>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<int, int>,
              int,
              Object?,
              Object?
            >;
    element.handleValue(ref, created);
  }
}
