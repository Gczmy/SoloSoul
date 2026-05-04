import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/presentation/models/operation_log_models.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/widgets/operation_tile.dart';

void main() {
  group('OperationTile', () {
    testWidgets('renders create action with correct icon and color',
        (tester) async {
      final entry = OperationEntry(
        timestamp: DateTime.now().subtract(const Duration(minutes: 5)),
        action: 'create',
        section: 'identity',
        description: 'Created profile',
        device: 'macos',
      );

      await tester.pumpWidget(
        MaterialApp(home: Scaffold(body: OperationTile(entry: entry))),
      );

      expect(find.byIcon(Icons.add_circle_outline), findsOneWidget);
      expect(find.text('Created'), findsOneWidget);
      expect(find.text('IDENTITY'), findsOneWidget);
      expect(find.text('Created profile'), findsOneWidget);
    });

    testWidgets('renders update action with correct icon', (tester) async {
      final entry = OperationEntry(
        timestamp: DateTime.now().subtract(const Duration(hours: 2)),
        action: 'update',
        section: 'travel',
        description: 'Updated passport',
        device: 'ios',
      );

      await tester.pumpWidget(
        MaterialApp(home: Scaffold(body: OperationTile(entry: entry))),
      );

      expect(find.byIcon(Icons.edit_outlined), findsOneWidget);
      expect(find.text('Updated'), findsOneWidget);
    });

    testWidgets('renders delete action with correct icon', (tester) async {
      final entry = OperationEntry(
        timestamp: DateTime.now(),
        action: 'delete',
        section: 'financial',
        description: 'Deleted entry',
        device: 'android',
      );

      await tester.pumpWidget(
        MaterialApp(home: Scaffold(body: OperationTile(entry: entry))),
      );

      expect(find.byIcon(Icons.delete_outline), findsOneWidget);
      expect(find.text('Deleted'), findsOneWidget);
    });

    testWidgets('renders restore action with correct icon', (tester) async {
      final entry = OperationEntry(
        timestamp: DateTime.now(),
        action: 'restore',
        section: 'trash',
        description: 'Restored item',
        device: 'windows',
      );

      await tester.pumpWidget(
        MaterialApp(home: Scaffold(body: OperationTile(entry: entry))),
      );

      expect(find.byIcon(Icons.restore), findsOneWidget);
      expect(find.text('Restored'), findsOneWidget);
    });

    testWidgets('renders purge action with correct icon', (tester) async {
      final entry = OperationEntry(
        timestamp: DateTime.now(),
        action: 'purge',
        section: 'trash',
        description: 'Purged item',
        device: 'linux',
      );

      await tester.pumpWidget(
        MaterialApp(home: Scaffold(body: OperationTile(entry: entry))),
      );

      expect(find.byIcon(Icons.delete_forever), findsOneWidget);
      expect(find.text('Purged'), findsOneWidget);
    });

    testWidgets('renders unknown action with info icon', (tester) async {
      final entry = OperationEntry(
        timestamp: DateTime.now(),
        action: 'unknown',
        section: 'system',
        description: 'Unknown action',
        device: 'web',
      );

      await tester.pumpWidget(
        MaterialApp(home: Scaffold(body: OperationTile(entry: entry))),
      );

      expect(find.byIcon(Icons.info_outline), findsWidgets);
      expect(find.text('unknown'), findsOneWidget);
    });

    testWidgets('renders device label and icon', (tester) async {
      final entry = OperationEntry(
        timestamp: DateTime.now(),
        action: 'create',
        section: 'identity',
        description: 'Test',
        device: 'macos',
      );

      await tester.pumpWidget(
        MaterialApp(home: Scaffold(body: OperationTile(entry: entry))),
      );

      expect(find.text('macOS'), findsOneWidget);
      expect(find.byIcon(Icons.laptop_mac), findsOneWidget);
    });

    testWidgets('renders fallback device for unknown platform',
        (tester) async {
      final entry = OperationEntry(
        timestamp: DateTime.now(),
        action: 'create',
        section: 'identity',
        description: 'Test',
        device: 'custom',
      );

      await tester.pumpWidget(
        MaterialApp(home: Scaffold(body: OperationTile(entry: entry))),
      );

      expect(find.text('custom'), findsOneWidget);
      expect(find.byIcon(Icons.devices), findsOneWidget);
    });

    testWidgets('shows detail dialog when info button tapped', (tester) async {
      final entry = OperationEntry(
        timestamp: DateTime(2026, 5, 1, 12, 30, 0),
        action: 'create',
        section: 'identity',
        description: 'Created profile',
        device: 'macos',
        fieldPath: 'profile.name',
      );

      await tester.pumpWidget(
        MaterialApp(home: Scaffold(body: OperationTile(entry: entry))),
      );

      await tester.tap(find.byIcon(Icons.info_outline).last);
      await tester.pumpAndSettle();

      expect(find.text('Operation Details'), findsOneWidget);
      expect(find.text('Created profile'), findsAtLeastNWidgets(1));
      expect(find.text('profile.name'), findsOneWidget);
      expect(find.text('macOS'), findsAtLeastNWidgets(1));
    });

    testWidgets('shows properties in detail dialog', (tester) async {
      final entry = OperationEntry(
        timestamp: DateTime.now(),
        action: 'update',
        section: 'travel',
        description: 'Updated visa',
        device: 'ios',
        properties: {'number': 'V12345', 'country': 'USA'},
        propertyLevels: {'number': 'sensitive', 'country': 'public'},
      );

      await tester.pumpWidget(
        MaterialApp(home: Scaffold(body: OperationTile(entry: entry))),
      );

      await tester.tap(find.byIcon(Icons.info_outline).last);
      await tester.pumpAndSettle();

      expect(find.text('Property Snapshot'), findsOneWidget);
      expect(find.text('V12345'), findsOneWidget);
      expect(find.text('USA'), findsOneWidget);
      expect(find.text('Sensitive'), findsOneWidget);
      expect(find.text('Public'), findsOneWidget);
    });

    testWidgets('shows empty placeholder for empty property value',
        (tester) async {
      final entry = OperationEntry(
        timestamp: DateTime.now(),
        action: 'create',
        section: 'identity',
        description: 'Test',
        device: 'macos',
        properties: {'note': ''},
      );

      await tester.pumpWidget(
        MaterialApp(home: Scaffold(body: OperationTile(entry: entry))),
      );

      await tester.tap(find.byIcon(Icons.info_outline).last);
      await tester.pumpAndSettle();

      expect(find.text('(empty)'), findsOneWidget);
    });

    testWidgets('formats time relative to now', (tester) async {
      final justNow = OperationEntry(
        timestamp: DateTime.now().subtract(const Duration(seconds: 30)),
        action: 'create',
        section: 'identity',
        description: 'Test',
        device: 'macos',
      );

      await tester.pumpWidget(
        MaterialApp(home: Scaffold(body: OperationTile(entry: justNow))),
      );

      expect(find.text('Just now'), findsOneWidget);
    });

    testWidgets('formats minutes ago', (tester) async {
      final entry = OperationEntry(
        timestamp: DateTime.now().subtract(const Duration(minutes: 30)),
        action: 'create',
        section: 'identity',
        description: 'Test',
        device: 'macos',
      );

      await tester.pumpWidget(
        MaterialApp(home: Scaffold(body: OperationTile(entry: entry))),
      );

      expect(find.textContaining('m ago'), findsOneWidget);
    });

    testWidgets('formats hours ago', (tester) async {
      final entry = OperationEntry(
        timestamp: DateTime.now().subtract(const Duration(hours: 5)),
        action: 'create',
        section: 'identity',
        description: 'Test',
        device: 'macos',
      );

      await tester.pumpWidget(
        MaterialApp(home: Scaffold(body: OperationTile(entry: entry))),
      );

      expect(find.textContaining('h ago'), findsOneWidget);
    });

    testWidgets('formats days ago', (tester) async {
      final entry = OperationEntry(
        timestamp: DateTime.now().subtract(const Duration(days: 3)),
        action: 'create',
        section: 'identity',
        description: 'Test',
        device: 'macos',
      );

      await tester.pumpWidget(
        MaterialApp(home: Scaffold(body: OperationTile(entry: entry))),
      );

      expect(find.textContaining('d ago'), findsOneWidget);
    });

    testWidgets('formats old date as absolute', (tester) async {
      final entry = OperationEntry(
        timestamp: DateTime.now().subtract(const Duration(days: 10)),
        action: 'create',
        section: 'identity',
        description: 'Test',
        device: 'macos',
      );

      await tester.pumpWidget(
        MaterialApp(home: Scaffold(body: OperationTile(entry: entry))),
      );

      // Should show day/month/year format, not relative
      expect(find.textContaining('d ago'), findsNothing);
      expect(find.textContaining('h ago'), findsNothing);
      expect(find.textContaining('m ago'), findsNothing);
      expect(find.text('Just now'), findsNothing);
    });
  });
}
