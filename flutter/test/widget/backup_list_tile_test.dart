import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/backup_service.dart';
import 'package:solosoul_flutter/presentation/widgets/data_management/backup_list_tile.dart';

void main() {
  group('BackupListTile', () {
    final testEntry = BackupEntry(
      fileName: 'profile_2026-05-01.backup',
      createdAt: DateTime(2026, 5, 1, 10, 30),
      sizeBytes: 2048,
    );

    testWidgets('renders normal tile with backup icon and size',
        (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: BackupListTile(
              entry: testEntry,
              onPromote: () {},
              onRestore: () {},
              onDelete: () {},
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.backup_outlined), findsOneWidget);
      expect(find.text('2026-05-01 10:30:00'), findsOneWidget);
      expect(find.text('2.0 KB'), findsOneWidget);
    });

    testWidgets('renders special tile with star icon', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: BackupListTile(
              entry: testEntry,
              isSpecial: true,
              onRestore: () {},
              onDelete: () {},
              onRename: () {},
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.star), findsOneWidget);
      expect(find.text('profile_2026-05-01'), findsOneWidget);
    });

    testWidgets('shows promote, restore and delete buttons for normal tile',
        (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: BackupListTile(
              entry: testEntry,
              onPromote: () {},
              onRestore: () {},
              onDelete: () {},
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.star_outline), findsOneWidget);
      expect(find.byIcon(Icons.restore), findsOneWidget);
      expect(find.byIcon(Icons.delete_outline), findsOneWidget);
    });

    testWidgets('shows rename, restore and delete buttons for special tile',
        (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: BackupListTile(
              entry: testEntry,
              isSpecial: true,
              onRestore: () {},
              onDelete: () {},
              onRename: () {},
            ),
          ),
        ),
      );

      expect(find.byIcon(Icons.edit), findsOneWidget);
      expect(find.byIcon(Icons.restore), findsOneWidget);
      expect(find.byIcon(Icons.delete_outline), findsOneWidget);
    });

    testWidgets('calls onPromote when star button tapped', (tester) async {
      var promoted = false;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: BackupListTile(
              entry: testEntry,
              onPromote: () => promoted = true,
              onRestore: () {},
              onDelete: () {},
            ),
          ),
        ),
      );

      await tester.tap(find.byIcon(Icons.star_outline));
      await tester.pump();
      expect(promoted, true);
    });

    testWidgets('calls onRestore when restore button tapped', (tester) async {
      var restored = false;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: BackupListTile(
              entry: testEntry,
              onRestore: () => restored = true,
              onDelete: () {},
            ),
          ),
        ),
      );

      await tester.tap(find.byIcon(Icons.restore));
      await tester.pump();
      expect(restored, true);
    });

    testWidgets('calls onDelete when delete button tapped', (tester) async {
      var deleted = false;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: BackupListTile(
              entry: testEntry,
              onRestore: () {},
              onDelete: () => deleted = true,
            ),
          ),
        ),
      );

      await tester.tap(find.byIcon(Icons.delete_outline));
      await tester.pump();
      expect(deleted, true);
    });

    testWidgets('calls onRename when edit button tapped', (tester) async {
      var renamed = false;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: BackupListTile(
              entry: testEntry,
              isSpecial: true,
              onRestore: () {},
              onDelete: () {},
              onRename: () => renamed = true,
            ),
          ),
        ),
      );

      await tester.tap(find.byIcon(Icons.edit));
      await tester.pump();
      expect(renamed, true);
    });

    testWidgets('disables restore button when isRestoring', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: BackupListTile(
              entry: testEntry,
              isRestoring: true,
              onRestore: () {},
              onDelete: () {},
            ),
          ),
        ),
      );

      final restoreButton = tester.widget<IconButton>(
        find.widgetWithIcon(IconButton, Icons.restore),
      );
      expect(restoreButton.onPressed, isNull);
    });

    testWidgets('formats large bytes as MB', (tester) async {
      final largeEntry = BackupEntry(
        fileName: 'large.backup',
        createdAt: DateTime(2026, 5, 1),
        sizeBytes: 5 * 1024 * 1024,
      );

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: BackupListTile(
              entry: largeEntry,
              onRestore: () {},
              onDelete: () {},
            ),
          ),
        ),
      );

      expect(find.text('5.0 MB'), findsOneWidget);
    });
  });
}
