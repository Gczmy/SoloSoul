import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations_en.dart';
import 'package:solosoul_flutter/presentation/widgets/home/quick_action.dart';

void main() {
  group('QuickAction', () {
    final l10n = AppLocalizationsEn();

    test('localizedLabel returns custom label for custom actions', () {
      const action = QuickAction(
        icon: Icons.star,
        label: 'My Custom',
        route: '/custom',
        color: Colors.blue,
        isCustom: true,
      );
      expect(action.localizedLabel(l10n), 'My Custom');
    });

    test('localizedLabel returns l10n for default routes', () {
      const action = QuickAction(
        icon: Icons.person,
        label: 'Profile',
        route: '/profile',
        color: Colors.blue,
      );
      expect(action.localizedLabel(l10n), isNotEmpty);
    });

    test('localizedLabel falls back for unknown route', () {
      const action = QuickAction(
        icon: Icons.abc,
        label: 'Fallback',
        route: '/unknown',
        color: Colors.grey,
      );
      expect(action.localizedLabel(l10n), 'Fallback');
    });

    test('constructs with default isCustom false', () {
      const action = QuickAction(
        icon: Icons.home,
        label: 'Home',
        route: '/home',
        color: Colors.red,
      );
      expect(action.isCustom, isFalse);
    });
  });
}
