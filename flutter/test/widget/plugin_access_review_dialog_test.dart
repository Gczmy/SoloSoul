import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/plugin_access_review_dialog.dart';

void main() {
  group('PluginAccessReviewDialog', () {
    testWidgets('renders dialog with plugin name', (tester) async {
      await tester.pumpWidget(MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: PluginAccessReviewDialog(
            pluginName: 'Test Plugin',
            fieldStatuses: const [],
            onModifySensitivity: () {},
            onCreateMissingFields: () {},
            onContinueInstall: () {},
            onCancel: () {},
          ),
        ),
      ));

      expect(find.byType(AlertDialog), findsOneWidget);
      expect(find.byIcon(Icons.extension), findsOneWidget);
    });

    testWidgets('calls onCancel when cancel tapped', (tester) async {
      bool cancelled = false;

      await tester.pumpWidget(MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: PluginAccessReviewDialog(
            pluginName: 'Test Plugin',
            fieldStatuses: const [],
            onModifySensitivity: () {},
            onCreateMissingFields: () {},
            onContinueInstall: () {},
            onCancel: () => cancelled = true,
          ),
        ),
      ));

      await tester.tap(find.widgetWithText(TextButton, 'Cancel'));
      await tester.pumpAndSettle();

      expect(cancelled, isTrue);
    });

    testWidgets('calls onContinueInstall when continue tapped', (tester) async {
      bool continued = false;

      await tester.pumpWidget(MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: PluginAccessReviewDialog(
            pluginName: 'Test Plugin',
            fieldStatuses: const [],
            onModifySensitivity: () {},
            onCreateMissingFields: () {},
            onContinueInstall: () => continued = true,
            onCancel: () {},
          ),
        ),
      ));

      await tester.tap(find.widgetWithText(FilledButton, 'Continue Install'));
      await tester.pumpAndSettle();

      expect(continued, isTrue);
    });

    testWidgets('shows exceeded warning when fields exceeded', (tester) async {
      await tester.pumpWidget(MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: PluginAccessReviewDialog(
            pluginName: 'Test Plugin',
            fieldStatuses: [
              FieldAccessStatus(
                fieldKey: 'test',
                status: AccessStatus.exceeded,
              ),
            ],
            onModifySensitivity: () {},
            onCreateMissingFields: () {},
            onContinueInstall: () {},
            onCancel: () {},
          ),
        ),
      ));

      expect(find.byIcon(Icons.warning_amber), findsOneWidget);
    });

    testWidgets('shows create missing button when fields missing', (tester) async {
      await tester.pumpWidget(MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: PluginAccessReviewDialog(
            pluginName: 'Test Plugin',
            fieldStatuses: [
              FieldAccessStatus(
                fieldKey: 'test',
                status: AccessStatus.missing,
              ),
            ],
            onModifySensitivity: () {},
            onCreateMissingFields: () {},
            onContinueInstall: () {},
            onCancel: () {},
          ),
        ),
      ));

      expect(find.byIcon(Icons.add), findsOneWidget);
    });
  });
}
