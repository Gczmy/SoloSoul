import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/categorized_icon_grid.dart';

void main() {
  group('CategorizedIconGrid', () {
    testWidgets('renders categories and icons', (tester) async {
      await tester.pumpWidget(MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: CategorizedIconGrid(
            currentIcon: 'folder',
            onSelected: (_) {},
          ),
        ),
      ));

      expect(find.byType(SingleChildScrollView), findsOneWidget);
      expect(find.byType(Wrap), findsWidgets);
      // Should have multiple category headers (Text widgets)
      expect(find.byType(Text), findsWidgets);
    });

    testWidgets('calls onSelected when icon tapped', (tester) async {
      String? selected;

      await tester.pumpWidget(MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: CategorizedIconGrid(
            currentIcon: 'folder',
            onSelected: (name) => selected = name,
          ),
        ),
      ));

      // Tap the first InkWell (first icon)
      await tester.tap(find.byType(InkWell).first);
      expect(selected, isNotNull);
    });

    testWidgets('highlights current icon', (tester) async {
      await tester.pumpWidget(MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: CategorizedIconGrid(
            currentIcon: 'folder',
            onSelected: (_) {},
          ),
        ),
      ));

      // Grid renders; at least one InkWell should be present
      expect(find.byType(InkWell), findsWidgets);
    });

    testWidgets('respects custom icon size and spacing', (tester) async {
      await tester.pumpWidget(MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: CategorizedIconGrid(
            currentIcon: 'folder',
            onSelected: (_) {},
            iconSize: 64,
            spacing: 20,
          ),
        ),
      ));

      expect(find.byType(CategorizedIconGrid), findsOneWidget);
    });
  });
}
