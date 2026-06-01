import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/pages/settings_page.dart';
import 'package:solosoul_flutter/presentation/widgets/settings/version_sheet.dart';

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
  group('VersionSheet', () {
    testWidgets('renders version info', (tester) async {
      final packageInfo = AsyncValue.data(PackageInfo(
        appName: 'SoloSoul',
        packageName: 'com.solosoul.app',
        version: '1.0.0',
        buildNumber: '100',
      ));

      await tester.pumpWidget(wrap(
        child: VersionSheet(
          packageInfo: packageInfo,
          onDebugActivationRequested: () async {}
        ),
        overrides: [
          latestVersionProvider.overrideWithValue(const AsyncValue.data('1.0.0')),
        ],
      ));

      expect(find.text('SoloSoul'), findsOneWidget);
      expect(find.text('1.0.0'), findsWidgets);
    });

    testWidgets('shows update available when latest is newer', (tester) async {
      final packageInfo = AsyncValue.data(PackageInfo(
        appName: 'SoloSoul',
        packageName: 'com.solosoul.app',
        version: '1.0.0',
        buildNumber: '100',
      ));

      await tester.pumpWidget(wrap(
        child: VersionSheet(
          packageInfo: packageInfo,
          onDebugActivationRequested: () async {}
        ),
        overrides: [
          latestVersionProvider.overrideWithValue(const AsyncValue.data('1.1.0')),
        ],
      ));

      expect(find.byType(OutlinedButton), findsOneWidget);
    });

    testWidgets('shows up to date when versions match', (tester) async {
      final packageInfo = AsyncValue.data(PackageInfo(
        appName: 'SoloSoul',
        packageName: 'com.solosoul.app',
        version: '1.0.0',
        buildNumber: '100',
      ));

      await tester.pumpWidget(wrap(
        child: VersionSheet(
          packageInfo: packageInfo,
          onDebugActivationRequested: () async {}
        ),
        overrides: [
          latestVersionProvider.overrideWithValue(const AsyncValue.data('1.0.0')),
        ],
      ));

      expect(find.byIcon(Icons.check_circle_outline), findsOneWidget);
    });

    testWidgets('renders platform info', (tester) async {
      final packageInfo = AsyncValue.data(PackageInfo(
        appName: 'SoloSoul',
        packageName: 'com.solosoul.app',
        version: '1.0.0',
        buildNumber: '100',
      ));

      await tester.pumpWidget(wrap(
        child: VersionSheet(
          packageInfo: packageInfo,
          onDebugActivationRequested: () async {},
        ),
        overrides: [
          latestVersionProvider.overrideWithValue(const AsyncValue.data('1.0.0')),
        ],
      ));

      expect(find.byIcon(Icons.laptop_mac), findsOneWidget);
    });
  });
}
