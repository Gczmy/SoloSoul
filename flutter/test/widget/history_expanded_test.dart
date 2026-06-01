import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/widgets/entry_card_widget.dart';

void main() {
  group('HistoryExpanded', () {
    test('build returns false', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final state = container.read(historyExpandedProvider('key1'));
      expect(state, false);
    });

    test('toggle flips state', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final notifier = container.read(historyExpandedProvider('key1').notifier);

      notifier.toggle();
      expect(container.read(historyExpandedProvider('key1')), true);

      notifier.toggle();
      expect(container.read(historyExpandedProvider('key1')), false);
    });

    test('expand sets state to true', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final notifier = container.read(historyExpandedProvider('key1').notifier);

      notifier.expand();
      expect(container.read(historyExpandedProvider('key1')), true);
    });

    test('collapse sets state to false', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final notifier = container.read(historyExpandedProvider('key1').notifier);

      notifier.expand();
      expect(container.read(historyExpandedProvider('key1')), true);

      notifier.collapse();
      expect(container.read(historyExpandedProvider('key1')), false);
    });

    test('different keys have independent state', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);

      container.read(historyExpandedProvider('a').notifier).expand();
      expect(container.read(historyExpandedProvider('a')), true);
      expect(container.read(historyExpandedProvider('b')), false);
    });
  });
}
