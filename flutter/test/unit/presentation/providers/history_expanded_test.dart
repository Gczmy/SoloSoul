import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/widgets/entry_card_widget.dart'
    show historyExpandedProvider;

void main() {
  group('HistoryExpanded provider', () {
    test('build returns false by default', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);

      expect(container.read(historyExpandedProvider('item-1')), false);
    });

    test('toggle flips state', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);

      container.read(historyExpandedProvider('item-1').notifier).toggle();
      expect(container.read(historyExpandedProvider('item-1')), true);

      container.read(historyExpandedProvider('item-1').notifier).toggle();
      expect(container.read(historyExpandedProvider('item-1')), false);
    });

    test('expand sets state to true', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);

      container.read(historyExpandedProvider('item-1').notifier).expand();
      expect(container.read(historyExpandedProvider('item-1')), true);

      // expand again stays true
      container.read(historyExpandedProvider('item-1').notifier).expand();
      expect(container.read(historyExpandedProvider('item-1')), true);
    });

    test('collapse sets state to false', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);

      container.read(historyExpandedProvider('item-1').notifier).expand();
      expect(container.read(historyExpandedProvider('item-1')), true);

      container.read(historyExpandedProvider('item-1').notifier).collapse();
      expect(container.read(historyExpandedProvider('item-1')), false);
    });

    test('keys are independent', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);

      container.read(historyExpandedProvider('a').notifier).expand();
      expect(container.read(historyExpandedProvider('a')), true);
      expect(container.read(historyExpandedProvider('b')), false);

      container.read(historyExpandedProvider('b').notifier).toggle();
      expect(container.read(historyExpandedProvider('a')), true);
      expect(container.read(historyExpandedProvider('b')), true);
    });
  });
}
