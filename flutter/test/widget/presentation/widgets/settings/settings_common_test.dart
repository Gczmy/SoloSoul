import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/widgets/settings/settings_common.dart';

void main() {
  group('InfoTile', () {
    testWidgets('renders icon, title and value', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: InfoTile(
              icon: Icons.person,
              title: 'Account',
              value: 'test@example.com',
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.person), findsOneWidget);
      expect(find.text('Account'), findsOneWidget);
      expect(find.text('test@example.com'), findsOneWidget);
    });

    testWidgets('renders subtitle when provided', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: InfoTile(
              icon: Icons.info,
              title: 'Status',
              value: 'Active',
              subtitle: 'Since 2024',
            ),
          ),
        ),
      );

      expect(find.text('Since 2024'), findsOneWidget);
    });

    testWidgets('does not render subtitle when null', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: InfoTile(
              icon: Icons.info,
              title: 'Status',
              value: 'Active',
            ),
          ),
        ),
      );

      expect(find.text('Since 2024'), findsNothing);
    });
  });

  group('VersionInfoTile', () {
    testWidgets('renders icon, title and value', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: VersionInfoTile(
              icon: Icons.app_shortcut,
              title: 'Version',
              value: '1.0.0',
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.app_shortcut), findsOneWidget);
      expect(find.text('Version'), findsOneWidget);
      expect(find.text('1.0.0'), findsOneWidget);
    });

    testWidgets('renders trailing widget when provided', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: VersionInfoTile(
              icon: Icons.update,
              title: 'Update',
              value: 'Available',
              trailing: Icon(Icons.arrow_forward),
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.arrow_forward), findsOneWidget);
    });

    testWidgets('does not render trailing when null', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: VersionInfoTile(
              icon: Icons.update,
              title: 'Update',
              value: 'Available',
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.arrow_forward), findsNothing);
    });
  });
}
