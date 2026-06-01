import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/search_empty_state.dart';

Widget _wrap(Widget child) => MaterialApp(
  localizationsDelegates: AppLocalizations.localizationsDelegates,
  supportedLocales: AppLocalizations.supportedLocales,
  home: Scaffold(body: child),
);

void main() {
  group('SearchEmptyState', () {
    testWidgets('renders search icon and message', (tester) async {
      await tester.pumpWidget(_wrap(const SearchEmptyState()));

      expect(find.byIcon(Icons.search), findsOneWidget);
      expect(find.byType(Text), findsOneWidget);
    });
  });

  group('SearchLoadingState', () {
    testWidgets('renders circular progress indicator', (tester) async {
      await tester.pumpWidget(_wrap(const SearchLoadingState()));

      expect(find.byType(CircularProgressIndicator), findsOneWidget);
    });
  });

  group('SearchNoResultsState', () {
    testWidgets('renders search_off icon and message', (tester) async {
      await tester.pumpWidget(_wrap(const SearchNoResultsState()));

      expect(find.byIcon(Icons.search_off), findsOneWidget);
      expect(find.byIcon(Icons.search), findsNothing);
    });
  });
}
