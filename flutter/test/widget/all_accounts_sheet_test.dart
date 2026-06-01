import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_types.dart';
import 'package:solosoul_flutter/presentation/widgets/settings/all_accounts_sheet.dart';

Widget wrap(Widget child) {
  return MaterialApp(
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(body: child),
  );
}

void main() {
  group('AllAccountsSheet', () {
    testWidgets('renders account count', (tester) async {
      final accounts = [
        const AccountInfo(id: 'a1', name: 'Alice'),
        const AccountInfo(id: 'a2', name: 'Bob'),
      ];

      await tester.pumpWidget(wrap(AllAccountsSheet(
        accounts: accounts,
        selectedAccountId: 'a1',
        onSelectAccount: (_) async {},
      )));

      expect(find.text('Alice'), findsOneWidget);
      expect(find.text('Bob'), findsOneWidget);
    });

    testWidgets('marks selected account with active badge', (tester) async {
      final accounts = [
        const AccountInfo(id: 'a1', name: 'Alice'),
      ];

      await tester.pumpWidget(wrap(AllAccountsSheet(
        accounts: accounts,
        selectedAccountId: 'a1',
        onSelectAccount: (_) async {},
      )));

      expect(find.byIcon(Icons.account_circle), findsWidgets);
    });

    testWidgets('non-selected account shows chevron', (tester) async {
      final accounts = [
        const AccountInfo(id: 'a1', name: 'Alice'),
        const AccountInfo(id: 'a2', name: 'Bob'),
      ];

      await tester.pumpWidget(wrap(AllAccountsSheet(
        accounts: accounts,
        selectedAccountId: 'a1',
        onSelectAccount: (_) async {},
      )));

      expect(find.byIcon(Icons.chevron_right), findsOneWidget);
    });

    testWidgets('selected account is not tappable', (tester) async {
      final accounts = [
        const AccountInfo(id: 'a1', name: 'Alice'),
      ];

      await tester.pumpWidget(wrap(AllAccountsSheet(
        accounts: accounts,
        selectedAccountId: 'a1',
        onSelectAccount: (_) async {},
      )));

      // The selected account's InkWell should have null onTap
      final inkWells = tester.widgetList<InkWell>(find.byType(InkWell));
      for (final inkWell in inkWells) {
        expect(inkWell.onTap, isNull);
      }
    });

    testWidgets('tapping non-selected account triggers callback', (tester) async {
      final accounts = [
        const AccountInfo(id: 'a1', name: 'Alice'),
        const AccountInfo(id: 'a2', name: 'Bob'),
      ];

      String? selectedId;
      await tester.pumpWidget(wrap(AllAccountsSheet(
        accounts: accounts,
        selectedAccountId: 'a1',
        onSelectAccount: (id) async => selectedId = id,
      )));

      // Tap the non-selected account (Bob)
      await tester.tap(find.text('Bob'));
      await tester.pump();
      expect(selectedId, equals('a2'));
    });

    testWidgets('shows device icon when recentDevices present', (tester) async {
      final accounts = [
        AccountInfo(
          id: 'a1',
          name: 'Alice',
          recentDevices: [
            DeviceInfo(deviceName: 'MacBook', lastUsed: DateTime(2024, 6, 1)),
          ],
        ),
      ];

      await tester.pumpWidget(wrap(AllAccountsSheet(
        accounts: accounts,
        selectedAccountId: 'a1',
        onSelectAccount: (_) async {},
      )));

      expect(find.byIcon(Icons.laptop_mac), findsOneWidget);
    });
  });
}
