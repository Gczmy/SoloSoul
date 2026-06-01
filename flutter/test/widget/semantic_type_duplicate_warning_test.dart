import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/semantic_type_duplicate_warning.dart';

Widget _wrap(Widget child) => MaterialApp(
  localizationsDelegates: AppLocalizations.localizationsDelegates,
  supportedLocales: AppLocalizations.supportedLocales,
  home: Scaffold(body: child),
);

void main() {
  group('SemanticTypeDuplicateWarning', () {
    testWidgets('renders dialog with warning icon', (tester) async {
      await tester.pumpWidget(_wrap(
        SemanticTypeDuplicateWarning(
          semanticTypeId: 'email',
          existingFieldLabel: 'Work Email',
          languageCode: 'en',
          onContinue: () {},
          onCancel: () {},
        ),
      ));

      expect(find.byIcon(Icons.warning_amber), findsOneWidget);
      expect(find.byType(AlertDialog), findsOneWidget);
    });

    testWidgets('calls onCancel when cancel tapped', (tester) async {
      bool cancelled = false;

      await tester.pumpWidget(_wrap(
        SemanticTypeDuplicateWarning(
          semanticTypeId: 'email',
          existingFieldLabel: 'Work Email',
          languageCode: 'en',
          onContinue: () {},
          onCancel: () => cancelled = true,
        ),
      ));

      await tester.tap(find.widgetWithText(TextButton, 'Cancel'));
      await tester.pumpAndSettle();

      expect(cancelled, isTrue);
    });

    testWidgets('calls onContinue when continue tapped', (tester) async {
      bool continued = false;

      await tester.pumpWidget(_wrap(
        SemanticTypeDuplicateWarning(
          semanticTypeId: 'email',
          existingFieldLabel: 'Work Email',
          languageCode: 'en',
          onContinue: () => continued = true,
          onCancel: () {},
        ),
      ));

      await tester.tap(find.widgetWithText(FilledButton, 'Reassign Anyway'));
      await tester.pumpAndSettle();

      expect(continued, isTrue);
    });

    testWidgets('displays existing field label', (tester) async {
      await tester.pumpWidget(_wrap(
        SemanticTypeDuplicateWarning(
          semanticTypeId: 'email',
          existingFieldLabel: 'Work Email',
          languageCode: 'en',
          onContinue: () {},
          onCancel: () {},
        ),
      ));

      expect(find.textContaining('Work Email'), findsOneWidget);
    });
  });
}
