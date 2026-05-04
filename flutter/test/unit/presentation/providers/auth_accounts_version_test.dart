import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_notifier.dart';

void main() {
  group('AccountsVersion provider', () {
    test('returns 0 initially', () async {
      final container = ProviderContainer();
      addTearDown(container.dispose);

      await container.read(authNotifierProvider.future);
      final version = container.read(accountsVersionProvider);
      expect(version, 0);
    });

    test('updates when selectAccount bumps version', () async {
      final container = ProviderContainer();
      addTearDown(container.dispose);

      await container.read(authNotifierProvider.future);
      final notifier = container.read(authNotifierProvider.notifier);

      await notifier.selectAccount(null);
      final version = container.read(accountsVersionProvider);
      expect(version, 1);
    });

    test('increments with multiple bumps', () async {
      final container = ProviderContainer();
      addTearDown(container.dispose);

      await container.read(authNotifierProvider.future);
      final notifier = container.read(authNotifierProvider.notifier);

      await notifier.selectAccount(null);
      await notifier.selectAccount(null);
      await notifier.selectAccount(null);

      final version = container.read(accountsVersionProvider);
      expect(version, 3);
    });
  });
}
