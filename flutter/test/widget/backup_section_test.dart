import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/backup_service.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/data_management/backup_section.dart';

Widget wrap(Widget child) {
  return MaterialApp(
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(body: child),
  );
}

void main() {
  group('BackupSection', () {
    testWidgets('renders create backup button', (tester) async {
      await tester.pumpWidget(wrap(BackupSection(
        isCreating: false,
        backupProgress: 0,
        backups: const [],
        isRestoring: false,
        totalSize: 0,
        backupPoolSize: '',
        onCreateBackup: () {},
        onRestoreBackup: (_) {},
        onDeleteBackup: (_) {},
        onPromoteBackup: (_) {},
      )));

      expect(find.byType(FilledButton), findsOneWidget);
      expect(find.byIcon(Icons.backup), findsOneWidget);
    });

    testWidgets('shows progress indicator when creating', (tester) async {
      await tester.pumpWidget(wrap(BackupSection(
        isCreating: true,
        backupProgress: 0.5,
        backups: const [],
        isRestoring: false,
        totalSize: 0,
        backupPoolSize: '',
        onCreateBackup: () {},
        onRestoreBackup: (_) {},
        onDeleteBackup: (_) {},
        onPromoteBackup: (_) {},
      )));

      expect(find.byType(CircularProgressIndicator), findsOneWidget);
    });

    testWidgets('shows empty state when no backups', (tester) async {
      await tester.pumpWidget(wrap(BackupSection(
        isCreating: false,
        backupProgress: 0,
        backups: const [],
        isRestoring: false,
        totalSize: 0,
        backupPoolSize: '',
        onCreateBackup: () {},
        onRestoreBackup: (_) {},
        onDeleteBackup: (_) {},
        onPromoteBackup: (_) {},
      )));

      expect(find.byIcon(Icons.cloud_off_outlined), findsOneWidget);
    });

    testWidgets('renders backup list tiles', (tester) async {
      final backups = [
        BackupEntry(
          fileName: 'backup_2024-01-01_10-00-00.backup',
          createdAt: DateTime(2024, 1, 1),
          sizeBytes: 1024,
        ),
      ];

      await tester.pumpWidget(wrap(BackupSection(
        isCreating: false,
        backupProgress: 0,
        backups: backups,
        isRestoring: false,
        totalSize: 1024,
        backupPoolSize: '',
        onCreateBackup: () {},
        onRestoreBackup: (_) {},
        onDeleteBackup: (_) {},
        onPromoteBackup: (_) {},
      )));

      expect(find.text('2024-01-01 00:00:00'), findsOneWidget);
    });

    testWidgets('create backup button is disabled when creating', (tester) async {
      await tester.pumpWidget(wrap(BackupSection(
        isCreating: true,
        backupProgress: 0,
        backups: const [],
        isRestoring: false,
        totalSize: 0,
        backupPoolSize: '',
        onCreateBackup: () {},
        onRestoreBackup: (_) {},
        onDeleteBackup: (_) {},
        onPromoteBackup: (_) {},
      )));

      final button = tester.widget<FilledButton>(find.byType(FilledButton));
      expect(button.onPressed, isNull);
    });
  });
}
