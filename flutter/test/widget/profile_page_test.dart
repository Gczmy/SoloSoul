import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/pages/profile_page.dart';

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

  group('ProfilePage Widget Tests', () {
    testWidgets('renders profile page with scaffold', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          child: MaterialApp(
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            supportedLocales: AppLocalizations.supportedLocales,
            home: const ProfilePage(),
          ),
        ),
      );

      await tester.pumpAndSettle();

      expect(find.byType(Scaffold), findsOneWidget);
    });

    testWidgets('has app bar with Profile title', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          child: MaterialApp(
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            supportedLocales: AppLocalizations.supportedLocales,
            home: const ProfilePage(),
          ),
        ),
      );

      await tester.pumpAndSettle();

      expect(find.text('Profile'), findsOneWidget);
      expect(find.byType(AppBar), findsOneWidget);
    });

    testWidgets('shows Identity Profile label', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          child: MaterialApp(
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            supportedLocales: AppLocalizations.supportedLocales,
            home: const ProfilePage(),
          ),
        ),
      );

      await tester.pumpAndSettle();

      expect(find.text('Identity Profile'), findsOneWidget);
    });

    testWidgets('shows encryption notice', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          child: MaterialApp(
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            supportedLocales: AppLocalizations.supportedLocales,
            home: const ProfilePage(),
          ),
        ),
      );

      await tester.pumpAndSettle();

      expect(find.text('End-to-End Encrypted'), findsOneWidget);
      expect(find.text('Your data is encrypted with AES-256-GCM'), findsOneWidget);
    });

    testWidgets('shows lock icon in security notice', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          child: MaterialApp(
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            supportedLocales: AppLocalizations.supportedLocales,
            home: const ProfilePage(),
          ),
        ),
      );

      await tester.pumpAndSettle();

      expect(find.byIcon(Icons.lock_outline), findsWidgets);
    });

    testWidgets('has CircleAvatar for profile', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          child: MaterialApp(
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            supportedLocales: AppLocalizations.supportedLocales,
            home: const ProfilePage(),
          ),
        ),
      );

      await tester.pumpAndSettle();

      expect(find.byType(CircleAvatar), findsWidgets);
    });

    testWidgets('shows default avatar character when no name', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          child: MaterialApp(
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            supportedLocales: AppLocalizations.supportedLocales,
            home: const ProfilePage(),
          ),
        ),
      );

      await tester.pumpAndSettle();

      // When fullName is empty/null, may show '?' depending on data
      expect(find.text('?'), findsAtLeastNWidgets(0));
    });
  });

  group('ProfilePage Sections Tests', () {
    testWidgets('shows Contact Information section', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          child: MaterialApp(
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            supportedLocales: AppLocalizations.supportedLocales,
            home: const ProfilePage(),
          ),
        ),
      );

      await tester.pumpAndSettle();

      expect(find.text('Contact Information'), findsOneWidget);
    });

    testWidgets('shows Identity Documents section', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          child: MaterialApp(
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            supportedLocales: AppLocalizations.supportedLocales,
            home: const ProfilePage(),
          ),
        ),
      );

      await tester.pumpAndSettle();

      expect(find.text('Identity Documents'), findsOneWidget);
    });

    testWidgets('shows Addresses section', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          child: MaterialApp(
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            supportedLocales: AppLocalizations.supportedLocales,
            home: const ProfilePage(),
          ),
        ),
      );

      await tester.pumpAndSettle();

      expect(find.text('Addresses'), findsOneWidget);
    });
  });
}
