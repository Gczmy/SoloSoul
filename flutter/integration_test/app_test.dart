import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/main.dart';
import 'package:solosoul_flutter/presentation/pages/login_page.dart';
import 'package:solosoul_flutter/presentation/pages/home_page.dart';
import 'package:solosoul_flutter/presentation/pages/profile_page.dart';
import 'package:solosoul_flutter/presentation/pages/travel_page.dart';

// Note: Integration tests require a device/emulator to run.
// Use: flutter test integration_test/app_test.dart
// Or: flutter drive --target=integration_test/app_test.dart

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  group('SoloSoul App Integration Tests', () {
    group('App Launch', () {
      testWidgets('app launches and shows splash screen',
          (WidgetTester tester) async {
        await tester.pumpWidget(
          const ProviderScope(
            child: SoloSoulApp(),
          ),
        );

        // Verify splash screen shows app name
        expect(find.text('SoloSoul'), findsOneWidget);
      });
    });

    group('Navigation Flow', () {
      testWidgets('can navigate to profile page',
          (WidgetTester tester) async {
        await tester.pumpWidget(
          ProviderScope(
            child: MaterialApp(
              home: Builder(
                builder: (context) => Scaffold(
                  body: Column(
                    children: [
                      TextButton(
                        onPressed: () {
                          Navigator.of(context).push(
                            MaterialPageRoute(
                              builder: (_) => const ProfilePage(),
                            ),
                          );
                        },
                        child: const Text('Go to Profile'),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        );

        await tester.pumpAndSettle();

        // Navigate to profile
        await tester.tap(find.text('Go to Profile'));
        await tester.pumpAndSettle();

        // Verify profile page
        expect(find.text('Profile'), findsOneWidget);
      });

      testWidgets('can navigate to travel page',
          (WidgetTester tester) async {
        await tester.pumpWidget(
          ProviderScope(
            child: MaterialApp(
              home: Builder(
                builder: (context) => Scaffold(
                  body: Column(
                    children: [
                      TextButton(
                        onPressed: () {
                          Navigator.of(context).push(
                            MaterialPageRoute(
                              builder: (_) => const TravelPage(),
                            ),
                          );
                        },
                        child: const Text('Go to Travel'),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        );

        await tester.pumpAndSettle();

        // Navigate to travel
        await tester.tap(find.text('Go to Travel'));
        await tester.pumpAndSettle();

        // Verify travel page
        expect(find.text('Travel'), findsOneWidget);
      });
    });

    group('Profile Page Integration', () {
      testWidgets('profile page renders all sections',
          (WidgetTester tester) async {
        await tester.pumpWidget(
          const ProviderScope(
            child: MaterialApp(
              home: ProfilePage(),
            ),
          ),
        );

        await tester.pump();

        // Verify main sections are present
        expect(find.text('Identity Profile'), findsOneWidget);
        expect(find.text('Contact Information'), findsOneWidget);
        expect(find.text('Identity Documents'), findsOneWidget);
        expect(find.text('Addresses'), findsOneWidget);
        expect(find.text('End-to-End Encrypted'), findsOneWidget);
      });
    });

    group('Travel Page Integration', () {
      testWidgets('travel page renders all sections',
          (WidgetTester tester) async {
        await tester.pumpWidget(
          const ProviderScope(
            child: MaterialApp(
              home: TravelPage(),
            ),
          ),
        );

        await tester.pump();

        // Verify all main sections are present
        expect(find.text('Passports'), findsOneWidget);
        expect(find.text('Visas'), findsOneWidget);
        expect(find.text('Travel History'), findsOneWidget);
        expect(find.text('Scan Document with OCR'), findsOneWidget);
      });

      testWidgets('travel page OCR dialog interaction',
          (WidgetTester tester) async {
        await tester.pumpWidget(
          const ProviderScope(
            child: MaterialApp(
              home: TravelPage(),
            ),
          ),
        );

        await tester.pump();

        // Tap OCR scan button
        await tester.tap(find.text('Scan Document with OCR'));
        await tester.pumpAndSettle();

        // Verify dialog appears
        expect(find.text('OCR Scan'), findsOneWidget);
      });
    });
  });
}
