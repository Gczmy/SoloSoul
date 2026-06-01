import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/scan/local_search_provider.dart';
import 'package:solosoul_flutter/presentation/providers/scan/local_search_state.dart';
import 'package:solosoul_flutter/presentation/widgets/scan_progress_banner.dart';

Widget wrap({required Widget child, required List overrides}) {
  return ProviderScope(
    overrides: overrides.cast(),
    child: MaterialApp(
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: Scaffold(body: child),
    ),
  );
}

void main() {
  group('ScanProgressBanner', () {
    testWidgets('shows banner when scanning', (tester) async {
      await tester.pumpWidget(wrap(
        child: const ScanProgressBanner(),
        overrides: [
          localSearchProvider.overrideWithValue(const LocalSearchState(
            isScanning: true,
            scannedCount: 10,
            foundCount: 3,
            skippedFiles: ['a.txt'],
          )),
        ],
      ));

      expect(find.text('Scanning'), findsOneWidget);
      expect(find.text('10'), findsOneWidget);
      expect(find.text('3'), findsOneWidget);
      expect(find.text('1'), findsOneWidget); // skippedFiles.length
      expect(find.byType(CircularProgressIndicator), findsOneWidget);
      expect(find.byIcon(Icons.stop_circle_outlined), findsOneWidget);
    });

    testWidgets('hides banner when not scanning', (tester) async {
      await tester.pumpWidget(wrap(
        child: const ScanProgressBanner(),
        overrides: [
          localSearchProvider.overrideWithValue(const LocalSearchState(
            isScanning: false,
          )),
        ],
      ));

      expect(find.text('Scanning'), findsNothing);
    });

    testWidgets('stop button is present', (tester) async {
      await tester.pumpWidget(wrap(
        child: const ScanProgressBanner(),
        overrides: [
          localSearchProvider.overrideWithValue(const LocalSearchState(
            isScanning: true,
            scannedCount: 5,
            foundCount: 1,
            skippedFiles: [],
          )),
        ],
      ));

      // Verify the stop button exists (tapping requires notifier override).
      expect(find.byIcon(Icons.stop_circle_outlined), findsOneWidget);
    });

    testWidgets('banner InkWell is present', (tester) async {
      await tester.pumpWidget(wrap(
        child: const ScanProgressBanner(),
        overrides: [
          localSearchProvider.overrideWithValue(const LocalSearchState(
            isScanning: true,
            scannedCount: 2,
            foundCount: 0,
            skippedFiles: [],
          )),
        ],
      ));

      // Verify the InkWell exists (tapping triggers GoRouter navigation).
      expect(find.byType(InkWell), findsWidgets);
    });
  });
}
