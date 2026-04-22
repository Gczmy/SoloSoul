import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/pages/travel_page.dart';

void main() {
  // Suppress flutter_animate timer warnings in tests
  setUp(() {
    FlutterError.onError = (FlutterErrorDetails details) {
      if (details.exceptionAsString().contains('Timer')) {
        return; // Ignore timer-related errors from animations
      }
      FlutterError.presentError(details);
    };
  });

  tearDown(() {
    FlutterError.onError = FlutterError.presentError;
  });

  group('TravelPage Widget Tests', () {
    testWidgets('renders travel page with scaffold', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: TravelPage(),
          ),
        ),
      );

      await tester.pumpAndSettle();

      expect(find.byType(Scaffold), findsOneWidget);
    });

    testWidgets('has app bar with Travel title', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: TravelPage(),
          ),
        ),
      );

      await tester.pumpAndSettle();

      expect(find.text('Travel'), findsOneWidget);
      expect(find.byType(AppBar), findsOneWidget);
    });

    testWidgets('shows OCR scan button', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: TravelPage(),
          ),
        ),
      );

      await tester.pumpAndSettle();

      expect(find.text('Scan Document with OCR'), findsOneWidget);
      expect(find.byIcon(Icons.document_scanner_outlined), findsOneWidget);
    });

    testWidgets('shows Passports section', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: TravelPage(),
          ),
        ),
      );

      await tester.pumpAndSettle();

      expect(find.text('Passports'), findsOneWidget);
      expect(find.byIcon(Icons.flight_outlined), findsOneWidget);
    });

    testWidgets('shows Visas section', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: TravelPage(),
          ),
        ),
      );

      await tester.pumpAndSettle();

      expect(find.text('Visas'), findsOneWidget);
      expect(find.byIcon(Icons.article_outlined), findsOneWidget);
    });

    testWidgets('shows Travel History section', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: TravelPage(),
          ),
        ),
      );

      await tester.pumpAndSettle();

      expect(find.text('Travel History'), findsOneWidget);
      expect(find.byIcon(Icons.history), findsOneWidget);
    });
  });

  group('TravelPage OCR Dialog Tests', () {
    testWidgets('shows OCR dialog when scan button tapped', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: TravelPage(),
          ),
        ),
      );

      await tester.pumpAndSettle();

      // Tap OCR scan button
      await tester.tap(find.text('Scan Document with OCR'));
      await tester.pumpAndSettle();

      // Verify dialog appears
      expect(find.text('OCR Scan'), findsOneWidget);
      expect(
        find.text('OCR document scanning will be available after PaddleOCR integration.'),
        findsOneWidget,
      );
    });

    testWidgets('can dismiss OCR dialog with OK button', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: TravelPage(),
          ),
        ),
      );

      await tester.pumpAndSettle();

      // Tap OCR scan button
      await tester.tap(find.text('Scan Document with OCR'));
      await tester.pumpAndSettle();

      // Verify dialog appears
      expect(find.text('OCR Scan'), findsOneWidget);

      // Tap OK to dismiss
      await tester.tap(find.text('OK'));
      await tester.pumpAndSettle();

      // Dialog should be dismissed
      expect(find.text('OCR Scan'), findsNothing);
    });

    testWidgets('OCR dialog has correct structure', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: TravelPage(),
          ),
        ),
      );

      await tester.pumpAndSettle();

      // Tap OCR scan button
      await tester.tap(find.text('Scan Document with OCR'));
      await tester.pumpAndSettle();

      // Verify dialog components
      expect(find.byType(AlertDialog), findsOneWidget);
      expect(find.byType(TextButton), findsOneWidget);
    });
  });
}
