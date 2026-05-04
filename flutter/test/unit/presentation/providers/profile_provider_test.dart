import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart';

void main() {
  group('ProfileNotifier', () {
    late ProviderContainer container;

    setUp(() {
      container = ProviderContainer();
    });

    tearDown(() => container.dispose());

    test('build returns null', () async {
      final state = await container.read(profileNotifierProvider.future);
      expect(state, isNull);
    });

    test('initial async state has null value', () async {
      await container.read(profileNotifierProvider.future);
      final asyncState = container.read(profileNotifierProvider);
      expect(asyncState.value, isNull);
    });

    test('isLoading is false initially', () async {
      await container.read(profileNotifierProvider.future);
      final notifier = container.read(profileNotifierProvider.notifier);
      expect(notifier.isLoading, isFalse);
    });

    test('clearProfile sets state to null', () async {
      await container.read(profileNotifierProvider.future);
      final notifier = container.read(profileNotifierProvider.notifier);
      await notifier.clearProfile();
      final asyncState = container.read(profileNotifierProvider);
      expect(asyncState.value, isNull);
    });
  });
}
