import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_types.dart';
import 'package:solosoul_flutter/presentation/widgets/settings/current_account_sheet.dart';

Widget wrap(Widget child) {
  return MaterialApp(
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(body: child),
  );
}

void main() {
  group('CurrentAccountSheet', () {
    testWidgets('renders account name and id', (tester) async {
      const account = AccountInfo(
        id: 'acc_123',
        name: 'Alice',
        createdAt: null,
        lastLoginAt: null,
      );

      await tester.pumpWidget(wrap(const CurrentAccountSheet(account: account)));

      expect(find.text('Alice'), findsOneWidget);
      expect(find.textContaining('acc_123'), findsOneWidget);
    });

    testWidgets('renders N/A for missing dates', (tester) async {
      const account = AccountInfo(
        id: 'acc_123',
        name: 'Alice',
        createdAt: null,
        lastLoginAt: null,
      );

      await tester.pumpWidget(wrap(const CurrentAccountSheet(account: account)));

      expect(find.text('N/A'), findsWidgets);
    });

    testWidgets('formats dates correctly', (tester) async {
      final account = AccountInfo(
        id: 'acc_123',
        name: 'Alice',
        createdAt: DateTime(2024, 1, 15, 10, 30),
        lastLoginAt: DateTime(2024, 6, 20, 14, 45),
      );

      await tester.pumpWidget(wrap(CurrentAccountSheet(account: account)));

      expect(find.text('2024-01-15 10:30'), findsOneWidget);
      expect(find.text('2024-06-20 14:45'), findsOneWidget);
    });

    testWidgets('translates known operation descriptions', (tester) async {
      final account = AccountInfo(
        id: 'acc_123',
        name: 'Alice',
        lastOperationDesc: 'Created account',
        lastOperationAt: DateTime(2024, 3, 1),
      );

      await tester.pumpWidget(wrap(CurrentAccountSheet(account: account)));
    });

    testWidgets('shows device count when devices present', (tester) async {
      final account = AccountInfo(
        id: 'acc_123',
        name: 'Alice',
        recentDevices: [
          DeviceInfo(deviceName: 'MacBook', lastUsed: DateTime(2024, 6, 1)),
          DeviceInfo(deviceName: 'iPhone', lastUsed: DateTime(2024, 6, 2)),
        ],
      );

      await tester.pumpWidget(wrap(CurrentAccountSheet(account: account)));

      expect(find.byIcon(Icons.laptop_mac), findsOneWidget);
      expect(find.byIcon(Icons.phone_iphone), findsOneWidget);
    });

    testWidgets('shows no devices message when empty', (tester) async {
      const account = AccountInfo(
        id: 'acc_123',
        name: 'Alice',
        recentDevices: [],
      );

      await tester.pumpWidget(wrap(const CurrentAccountSheet(account: account)));

      expect(find.byIcon(Icons.devices_outlined), findsOneWidget);
    });
  });
}
