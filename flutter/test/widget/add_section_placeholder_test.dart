import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/add_section_placeholder.dart';

Widget wrap(Widget child) {
  return MaterialApp(
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(body: child),
  );
}

void main() {
  group('AddSectionPlaceholder', () {
    testWidgets('renders add icon and label', (tester) async {
      await tester.pumpWidget(wrap(AddSectionPlaceholder(
        onTap: () {},
      )));

      expect(find.byIcon(Icons.add), findsOneWidget);
      expect(find.descendant(of: find.byType(AddSectionPlaceholder), matching: find.byType(CustomPaint)), findsOneWidget);
    });

    testWidgets('triggers onTap when tapped', (tester) async {
      bool tapped = false;
      await tester.pumpWidget(wrap(AddSectionPlaceholder(
        onTap: () => tapped = true,
      )));

      await tester.tap(find.byType(InkWell));
      await tester.pump();
      expect(tapped, isTrue);
    });
  });
}
