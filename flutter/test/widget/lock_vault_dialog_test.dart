import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/lock_vault_dialog.dart';

void main() {
  group('showLockVaultDialog', () {
    testWidgets('shows dialog with lock icon and message', (tester) async {
      await tester.pumpWidget(MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: Builder(
            builder: (context) => ElevatedButton(
              onPressed: () => showLockVaultDialog(context),
              child: const Text('lock'),
            ),
          ),
        ),
      ));

      await tester.tap(find.text('lock'));
      await tester.pumpAndSettle();

      expect(find.byType(AlertDialog), findsOneWidget);
      expect(find.byIcon(Icons.lock_outline), findsOneWidget);
    });

    testWidgets('returns false when cancel tapped', (tester) async {
      bool? result;

      await tester.pumpWidget(MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: Builder(
            builder: (context) => ElevatedButton(
              onPressed: () async {
                result = await showLockVaultDialog(context);
              },
              child: const Text('lock'),
            ),
          ),
        ),
      ));

      await tester.tap(find.text('lock'));
      await tester.pumpAndSettle();

      await tester.tap(find.widgetWithText(TextButton, 'Cancel'));
      await tester.pumpAndSettle();

      expect(result, false);
    });

    testWidgets('returns true when lock tapped', (tester) async {
      bool? result;

      await tester.pumpWidget(MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: Builder(
            builder: (context) => ElevatedButton(
              onPressed: () async {
                result = await showLockVaultDialog(context);
              },
              child: const Text('lock'),
            ),
          ),
        ),
      ));

      await tester.tap(find.text('lock'));
      await tester.pumpAndSettle();

      await tester.tap(find.widgetWithText(FilledButton, 'Lock'));
      await tester.pumpAndSettle();

      expect(result, true);
    });
  });
}
