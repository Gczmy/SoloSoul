import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/add_section_dialog.dart';

void main() {
  group('AddSectionDialog', () {
    testWidgets('renders dialog with title input', (tester) async {
      await tester.pumpWidget(MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: Builder(
            builder: (context) => ElevatedButton(
              onPressed: () => showDialog(
                context: context,
                builder: (_) => const AddSectionDialog(),
              ),
              child: const Text('add'),
            ),
          ),
        ),
      ));

      await tester.tap(find.text('add'));
      await tester.pumpAndSettle();

      expect(find.byType(AlertDialog), findsOneWidget);
      expect(find.byType(TextField), findsOneWidget);
    });

    testWidgets('cancel returns null', (tester) async {
      Map<String, String>? result;

      await tester.pumpWidget(MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: Builder(
            builder: (context) => ElevatedButton(
              onPressed: () async {
                result = await showDialog<Map<String, String>>(
                  context: context,
                  builder: (_) => const AddSectionDialog(),
                );
              },
              child: const Text('add'),
            ),
          ),
        ),
      ));

      await tester.tap(find.text('add'));
      await tester.pumpAndSettle();

      await tester.tap(find.widgetWithText(TextButton, 'Cancel'));
      await tester.pumpAndSettle();

      expect(result, isNull);
    });

    testWidgets('save returns title and icon when valid', (tester) async {
      Map<String, String>? result;

      await tester.pumpWidget(MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: Builder(
            builder: (context) => ElevatedButton(
              onPressed: () async {
                result = await showDialog<Map<String, String>>(
                  context: context,
                  builder: (_) => const AddSectionDialog(),
                );
              },
              child: const Text('add'),
            ),
          ),
        ),
      ));

      await tester.tap(find.text('add'));
      await tester.pumpAndSettle();

      await tester.enterText(find.byType(TextField), 'My Section');
      await tester.pump();

      await tester.tap(find.widgetWithText(FilledButton, 'Add Section'));
      await tester.pumpAndSettle();

      expect(result, isNotNull);
      expect(result!['title'], 'My Section');
      expect(result!['icon'], isNotNull);
    });

    testWidgets('save is disabled when title is empty', (tester) async {
      await tester.pumpWidget(MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: Builder(
            builder: (context) => ElevatedButton(
              onPressed: () => showDialog(
                context: context,
                builder: (_) => const AddSectionDialog(),
              ),
              child: const Text('add'),
            ),
          ),
        ),
      ));

      await tester.tap(find.text('add'));
      await tester.pumpAndSettle();

      // Empty title
      await tester.tap(find.widgetWithText(FilledButton, 'Add Section'));
      await tester.pump();

      // Dialog should still be open
      expect(find.byType(AlertDialog), findsOneWidget);
    });
  });
}
