import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/widgets/settings/settings_tile.dart';

void main() {
  group('SettingsTile', () {
    testWidgets('renders icon, title and subtitle', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SettingsTile(
              icon: Icons.person,
              title: 'Account',
              subtitle: 'Manage your account',
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.person), findsOneWidget);
      expect(find.text('Account'), findsOneWidget);
      expect(find.text('Manage your account'), findsOneWidget);
    });

    testWidgets('shows chevron when onTap is provided', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: const Scaffold(
            body: SettingsTile(
              icon: Icons.settings,
              title: 'Settings',
              subtitle: 'App settings',
              onTap: _noop,
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.chevron_right), findsOneWidget);
    });

    testWidgets('hides chevron when no onTap', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SettingsTile(
              icon: Icons.info,
              title: 'About',
              subtitle: 'Version info',
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.chevron_right), findsNothing);
    });

    testWidgets('shows trailing widget instead of chevron', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: const Scaffold(
            body: SettingsTile(
              icon: Icons.notifications,
              title: 'Notifications',
              subtitle: 'Push settings',
              onTap: _noop,
              trailing: Switch(value: true, onChanged: _noopBool),
            ),
          ),
        ),
      );

      expect(find.byType(Switch), findsOneWidget);
      expect(find.byIcon(Icons.chevron_right), findsNothing);
    });

    testWidgets('calls onTap when tapped', (tester) async {
      var tapped = false;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SettingsTile(
              icon: Icons.lock,
              title: 'Security',
              subtitle: 'Password & biometrics',
              onTap: () => tapped = true,
            ),
          ),
        ),
      );

      await tester.tap(find.byType(SettingsTile));
      await tester.pump();
      expect(tapped, true);
    });
  });
}

void _noop() {}
void _noopBool(bool _) {}
