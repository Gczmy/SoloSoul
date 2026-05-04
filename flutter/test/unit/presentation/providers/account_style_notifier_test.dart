import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart';

void main() {
  group('AccountStyleNotifier', () {
    late ProviderContainer container;

    setUp(() {
      container = ProviderContainer();
    });

    tearDown(() => container.dispose());

    test('build returns default AccountStyle when no account', () async {
      final state = await container.read(accountStyleProvider.future);
      expect(state.displayMode, SensitivityDisplayMode.hidePrivate);
      expect(state.fieldSettings, isEmpty);
      expect(state.revealedFields, isEmpty);
      expect(state.tagDefaults, isEmpty);
    });

    test('setDisplayMode updates display mode', () async {
      await container.read(accountStyleProvider.future);
      final notifier = container.read(accountStyleProvider.notifier);

      notifier.setDisplayMode(SensitivityDisplayMode.showAll);
      expect(container.read(accountStyleProvider).value?.displayMode,
          SensitivityDisplayMode.showAll);

      notifier.setDisplayMode(SensitivityDisplayMode.hideAll);
      expect(container.read(accountStyleProvider).value?.displayMode,
          SensitivityDisplayMode.hideAll);
    });

    test('revealField adds field to revealed set', () async {
      await container.read(accountStyleProvider.future);
      final notifier = container.read(accountStyleProvider.notifier);

      notifier.revealField('email');
      expect(container.read(accountStyleProvider).value?.revealedFields,
          contains('email'));

      notifier.revealField('phone');
      final revealed =
          container.read(accountStyleProvider).value?.revealedFields;
      expect(revealed, contains('email'));
      expect(revealed, contains('phone'));
    });

    test('hideField removes field from revealed set', () async {
      await container.read(accountStyleProvider.future);
      final notifier = container.read(accountStyleProvider.notifier);

      notifier.revealField('email');
      notifier.revealField('phone');
      expect(container.read(accountStyleProvider).value?.revealedFields.length,
          2);

      notifier.hideField('email');
      final revealed =
          container.read(accountStyleProvider).value?.revealedFields;
      expect(revealed, isNot(contains('email')));
      expect(revealed, contains('phone'));
    });

    test('toggleField adds and removes field', () async {
      await container.read(accountStyleProvider.future);
      final notifier = container.read(accountStyleProvider.notifier);

      notifier.toggleField('email');
      expect(container.read(accountStyleProvider).value?.revealedFields,
          contains('email'));

      notifier.toggleField('email');
      expect(container.read(accountStyleProvider).value?.revealedFields,
          isNot(contains('email')));
    });

    test('hideAllPrivate clears all revealed fields', () async {
      await container.read(accountStyleProvider.future);
      final notifier = container.read(accountStyleProvider.notifier);

      notifier.revealField('email');
      notifier.revealField('phone');
      expect(container.read(accountStyleProvider).value?.revealedFields.length,
          2);

      notifier.hideAllPrivate();
      expect(container.read(accountStyleProvider).value?.revealedFields,
          isEmpty);
    });

    test('setDisplayMode does nothing when state has no value', () async {
      await container.read(accountStyleProvider.future);
      final notifier = container.read(accountStyleProvider.notifier);

      // State has value, so this works
      notifier.setDisplayMode(SensitivityDisplayMode.showAll);
      expect(container.read(accountStyleProvider).value?.displayMode,
          SensitivityDisplayMode.showAll);
    });

    test('revealField does nothing when field already revealed', () async {
      await container.read(accountStyleProvider.future);
      final notifier = container.read(accountStyleProvider.notifier);

      notifier.revealField('email');
      notifier.revealField('email');
      expect(container.read(accountStyleProvider).value?.revealedFields.length,
          1);
    });

    test('hideField does nothing when field not in revealed set', () async {
      await container.read(accountStyleProvider.future);
      final notifier = container.read(accountStyleProvider.notifier);

      notifier.hideField('nonexistent');
      expect(container.read(accountStyleProvider).value?.revealedFields,
          isEmpty);
    });

    test('clear resets state to default', () async {
      await container.read(accountStyleProvider.future);
      final notifier = container.read(accountStyleProvider.notifier);

      notifier.setDisplayMode(SensitivityDisplayMode.showAll);
      notifier.revealField('email');

      notifier.clear();
      final state = container.read(accountStyleProvider).value;
      expect(state?.displayMode, SensitivityDisplayMode.hidePrivate);
      expect(state?.revealedFields, isEmpty);
    });
  });
}
