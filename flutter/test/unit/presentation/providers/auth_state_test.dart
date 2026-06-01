import 'package:fake_async/fake_async.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_state.dart';

void main() {
  group('SensitivePageAccessNotifier', () {
    late ProviderContainer container;

    setUp(() {
      container = ProviderContainer();
    });

    tearDown(() => container.dispose());

    test('build returns default state', () {
      final state = container.read(sensitivePageAccessProvider);
      expect(state.lastVerified, isNull);
      expect(state.isValid, isFalse);
    });

    test('markVerified sets lastVerified and starts timer', () {
      fakeAsync((async) {
        final notifier = container.read(sensitivePageAccessProvider.notifier);
        notifier.markVerified();
        final state = container.read(sensitivePageAccessProvider);
        expect(state.lastVerified, isNotNull);
        expect(state.isValid, isTrue);
      });
    });

    test('timer expiry resets state to invalid', () {
      fakeAsync((async) {
        final notifier = container.read(sensitivePageAccessProvider.notifier);
        notifier.markVerified();
        expect(container.read(sensitivePageAccessProvider).isValid, isTrue);

        async.elapse(const Duration(minutes: 1));
        expect(container.read(sensitivePageAccessProvider).isValid, isFalse);
      });
    });

    test('clear cancels timer and resets state', () {
      fakeAsync((async) {
        final notifier = container.read(sensitivePageAccessProvider.notifier);
        notifier.markVerified();
        notifier.clear();
        final state = container.read(sensitivePageAccessProvider);
        expect(state.lastVerified, isNull);
        expect(state.isValid, isFalse);

        // Ensure timer does not fire after clear
        async.elapse(const Duration(minutes: 2));
        expect(container.read(sensitivePageAccessProvider).isValid, isFalse);
      });
    });

    test('markVerified restarts timer on repeated calls', () {
      fakeAsync((async) {
        final notifier = container.read(sensitivePageAccessProvider.notifier);
        notifier.markVerified();
        async.elapse(const Duration(seconds: 30));
        expect(container.read(sensitivePageAccessProvider).isValid, isTrue);

        notifier.markVerified(); // Restart timer
        async.elapse(const Duration(seconds: 30));
        expect(container.read(sensitivePageAccessProvider).isValid, isTrue);

        async.elapse(const Duration(seconds: 30));
        expect(container.read(sensitivePageAccessProvider).isValid, isFalse);
      });
    });
  });

  group('SensitivePageAccessState', () {
    test('default constructor has null lastVerified', () {
      const state = SensitivePageAccessState();
      expect(state.lastVerified, isNull);
      expect(state.isValid, isFalse);
    });

    test('isValid is true when recently verified', () {
      final state = SensitivePageAccessState(
        lastVerified: DateTime.now(),
      );
      expect(state.isValid, isTrue);
    });

    test('isValid is false when verification expired', () {
      final state = SensitivePageAccessState(
        lastVerified: DateTime.now().subtract(
          const Duration(minutes: 2),
        ),
      );
      expect(state.isValid, isFalse);
    });

    test('isValid is true just before timeout', () {
      final state = SensitivePageAccessState(
        lastVerified: DateTime.now().subtract(
          const Duration(seconds: 50),
        ),
      );
      expect(state.isValid, isTrue);
    });

    test('copyWith no changes', () {
      final now = DateTime(2024, 6, 15);
      final state = SensitivePageAccessState(lastVerified: now);
      final copy = state.copyWith();
      expect(copy.lastVerified, now);
    });

    test('copyWith changes', () {
      final old = DateTime(2024, 1, 1);
      final updated = DateTime(2024, 6, 15);
      final state = SensitivePageAccessState(lastVerified: old);
      final copy = state.copyWith(lastVerified: updated);
      expect(copy.lastVerified, updated);
    });
  });

  group('kSensitiveAccessTimeout', () {
    test('is 1 minute', () {
      expect(kSensitiveAccessTimeout, const Duration(minutes: 1));
    });
  });

  group('BackoffNotifier', () {
    late ProviderContainer container;

    setUp(() {
      container = ProviderContainer();
    });

    tearDown(() => container.dispose());

    test('build returns default state', () {
      final state = container.read(backoffProvider);
      expect(state.remainingSeconds, 0);
      expect(state.isLockedOut, false);
    });

    test('onBackoffException sets locked out state', () {
      fakeAsync((async) {
        final notifier = container.read(backoffProvider.notifier);
        notifier.onBackoffException(30, true);
        final state = container.read(backoffProvider);
        expect(state.remainingSeconds, 30);
        expect(state.isLockedOut, true);
      });
    });

    test('onBackoffException starts countdown when not locked out', () {
      fakeAsync((async) {
        final notifier = container.read(backoffProvider.notifier);
        notifier.onBackoffException(3, false);
        expect(container.read(backoffProvider).remainingSeconds, 3);

        async.elapse(const Duration(seconds: 1));
        expect(container.read(backoffProvider).remainingSeconds, 2);

        async.elapse(const Duration(seconds: 1));
        expect(container.read(backoffProvider).remainingSeconds, 1);

        async.elapse(const Duration(seconds: 1));
        final state = container.read(backoffProvider);
        expect(state.remainingSeconds, 0);
        expect(state.isLockedOut, false);
      });
    });

    test('clear resets state and cancels timer', () {
      fakeAsync((async) {
        final notifier = container.read(backoffProvider.notifier);
        notifier.onBackoffException(10, false);
        expect(container.read(backoffProvider).remainingSeconds, 10);

        notifier.clear();
        final state = container.read(backoffProvider);
        expect(state.remainingSeconds, 0);
        expect(state.isLockedOut, false);

        // Ensure timer does not fire after clear
        async.elapse(const Duration(seconds: 5));
        expect(container.read(backoffProvider).remainingSeconds, 0);
      });
    });

    test('countdown reaches zero and resets state', () {
      fakeAsync((async) {
        final notifier = container.read(backoffProvider.notifier);
        notifier.onBackoffException(2, false);

        async.elapse(const Duration(seconds: 2));
        final state = container.read(backoffProvider);
        expect(state.remainingSeconds, 0);
        expect(state.isLockedOut, false);
      });
    });
  });
}
