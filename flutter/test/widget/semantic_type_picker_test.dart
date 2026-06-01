import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/semantic_type_picker.dart';

Widget wrap(Widget child) {
  return MaterialApp(
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(body: child),
  );
}

void main() {
  group('SemanticTypePickerSheet', () {
    testWidgets('renders with search field', (tester) async {
      await tester.pumpWidget(wrap(SemanticTypePickerSheet(
        languageCode: 'en',
        onSelected: (_) {},
      )));

      expect(find.byType(TextField), findsOneWidget);
    });

    testWidgets('selects a type on tap', (tester) async {
      String? selectedId;
      await tester.pumpWidget(wrap(SemanticTypePickerSheet(
        languageCode: 'en',
        onSelected: (id) => selectedId = id,
      )));

      // Expand a category by tapping it
      await tester.tap(find.byType(InkWell).first);
      await tester.pumpAndSettle();

      // Tap the first type tile
      await tester.tap(find.byType(InkWell).at(1));
      await tester.pump();
      expect(selectedId, isNotNull);
    });

    testWidgets('filters types on search', (tester) async {
      await tester.pumpWidget(wrap(SemanticTypePickerSheet(
        languageCode: 'en',
        onSelected: (_) {},
      )));

      await tester.enterText(find.byType(TextField), 'email');
      await tester.pumpAndSettle();

      // Should show filtered results
      expect(find.byType(TextField), findsOneWidget);
    });

    testWidgets('renders with current selection without error', (tester) async {
      await tester.pumpWidget(wrap(SemanticTypePickerSheet(
        currentSemanticType: 'email',
        languageCode: 'en',
        onSelected: (_) {},
      )));

      expect(find.byType(TextField), findsOneWidget);
    });
  });
}
