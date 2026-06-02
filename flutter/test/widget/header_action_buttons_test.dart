import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';

void main() {
  group('HeaderActionButtons', () {
    testWidgets('renders nothing when sensitive access not granted', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            supportedLocales: AppLocalizations.supportedLocales,
            home: Scaffold(body: HeaderActionButtons()),
          ),
        ),
      );

      expect(find.byType(SizedBox), findsOneWidget);
      expect(find.byType(IconButton), findsNothing);
    });

    testWidgets('renders lock icon when sensitive access granted', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            sensitivePageAccessProvider.overrideWith(() => _MockSensitiveAccessNotifier(true)),
          ],
          child: const MaterialApp(
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            supportedLocales: AppLocalizations.supportedLocales,
            home: Scaffold(body: HeaderActionButtons()),
          ),
        ),
      );

      expect(find.byType(IconButton), findsOneWidget);
      expect(find.byIcon(Icons.lock_open_outlined), findsOneWidget);
    });

    testWidgets('renders lock icon with success color', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            sensitivePageAccessProvider.overrideWith(() => _MockSensitiveAccessNotifier(true)),
          ],
          child: const MaterialApp(
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            supportedLocales: AppLocalizations.supportedLocales,
            home: Scaffold(body: HeaderActionButtons()),
          ),
        ),
      );

      final iconButton = tester.widget<IconButton>(find.byType(IconButton));
      expect((iconButton.icon as Icon).icon, Icons.lock_open_outlined);
    });
  });
}

class _MockSensitiveAccessNotifier extends SensitivePageAccessNotifier {
  bool _granted;
  bool cleared = false;

  _MockSensitiveAccessNotifier(this._granted);

  @override
  SensitivePageAccessState build() {
    return SensitivePageAccessState(
      lastVerified: _granted ? DateTime.now() : null,
    );
  }

  @override
  void clear() {
    cleared = true;
    _granted = false;
  }

  @override
  void markVerified() {
    _granted = true;
  }
}
