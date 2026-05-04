import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/widgets/search_empty_state.dart';

void main() {
  group('SearchEmptyState', () {
    testWidgets('renders search icon and hint text', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(body: SearchEmptyState()),
        ),
      );

      expect(find.byIcon(Icons.search), findsOneWidget);
      expect(
        find.text('Enter at least 2 characters to search'),
        findsOneWidget,
      );
    });
  });

  group('SearchLoadingState', () {
    testWidgets('renders CircularProgressIndicator', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(body: SearchLoadingState()),
        ),
      );

      expect(find.byType(CircularProgressIndicator), findsOneWidget);
    });
  });

  group('SearchNoResultsState', () {
    testWidgets('renders search_off icon and messages', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(body: SearchNoResultsState()),
        ),
      );

      expect(find.byIcon(Icons.search_off), findsOneWidget);
      expect(find.text('No results found'), findsOneWidget);
      expect(
        find.text('Try adjusting your filters or search terms'),
        findsOneWidget,
      );
    });
  });
}
