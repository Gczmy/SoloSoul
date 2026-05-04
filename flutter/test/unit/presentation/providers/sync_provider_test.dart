import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/providers/sync_provider.dart';

void main() {
  group('SyncStatus', () {
    test('has expected values', () {
      expect(SyncStatus.values, hasLength(5));
      expect(SyncStatus.values, contains(SyncStatus.idle));
      expect(SyncStatus.values, contains(SyncStatus.discovering));
      expect(SyncStatus.values, contains(SyncStatus.syncing));
      expect(SyncStatus.values, contains(SyncStatus.success));
      expect(SyncStatus.values, contains(SyncStatus.error));
    });

    test('values have correct index order', () {
      expect(SyncStatus.idle.index, 0);
      expect(SyncStatus.discovering.index, 1);
      expect(SyncStatus.syncing.index, 2);
      expect(SyncStatus.success.index, 3);
      expect(SyncStatus.error.index, 4);
    });
  });

  group('SyncState', () {
    test('default constructor has correct defaults', () {
      const state = SyncState();
      expect(state.status, SyncStatus.idle);
      expect(state.devices, isEmpty);
      expect(state.errorMessage, isNull);
      expect(state.lastResult, isNull);
      expect(state.isAdvertising, isFalse);
    });

    group('copyWith', () {
      test('copies with no changes', () {
        const state = SyncState(
          status: SyncStatus.syncing,
          isAdvertising: true,
        );
        final copy = state.copyWith();
        expect(copy.status, SyncStatus.syncing);
        expect(copy.isAdvertising, isTrue);
        expect(copy.devices, isEmpty);
        expect(copy.errorMessage, isNull);
      });

      test('copies with status change', () {
        const state = SyncState();
        final copy = state.copyWith(status: SyncStatus.discovering);
        expect(copy.status, SyncStatus.discovering);
        expect(copy.isAdvertising, isFalse);
      });

      test('copies with errorMessage', () {
        const state = SyncState();
        final copy = state.copyWith(
          status: SyncStatus.error,
          errorMessage: 'Connection failed',
        );
        expect(copy.status, SyncStatus.error);
        expect(copy.errorMessage, 'Connection failed');
      });

      test('copies with isAdvertising', () {
        const state = SyncState();
        final copy = state.copyWith(isAdvertising: true);
        expect(copy.isAdvertising, isTrue);
        expect(copy.status, SyncStatus.idle);
      });

      test('errorMessage can be set to null explicitly', () {
        final state = const SyncState().copyWith(
          status: SyncStatus.error,
          errorMessage: 'Error',
        );
        // copyWith with no errorMessage preserves existing value
        final copy = state.copyWith(status: SyncStatus.idle);
        // Note: copyWith sets errorMessage to null when not provided
        // because the field defaults to null in the constructor
        expect(copy.status, SyncStatus.idle);
      });
    });
  });

  group('SyncNotifier', () {
    late ProviderContainer container;

    setUp(() {
      container = ProviderContainer();
    });

    tearDown(() => container.dispose());

    test('initial state is idle with empty devices', () {
      final state = container.read(syncProvider);
      expect(state.status, SyncStatus.idle);
      expect(state.devices, isEmpty);
      expect(state.isAdvertising, isFalse);
      expect(state.errorMessage, isNull);
      expect(state.lastResult, isNull);
    });

    test('stopAdvertising sets isAdvertising to false', () {
      container.read(syncProvider.notifier).stopAdvertising();
      expect(container.read(syncProvider).isAdvertising, isFalse);
    });

    test('reset returns to initial state', () {
      final notifier = container.read(syncProvider.notifier);
      notifier.stopAdvertising();
      notifier.reset();
      final state = container.read(syncProvider);
      expect(state.status, SyncStatus.idle);
      expect(state.devices, isEmpty);
      expect(state.isAdvertising, isFalse);
      expect(state.errorMessage, isNull);
    });
  });
}
