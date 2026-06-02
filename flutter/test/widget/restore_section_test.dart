import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/backup_service.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/data_management/restore_section.dart';

Widget wrap(Widget child) {
  return MaterialApp(
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(body: child),
  );
}

void main() {
  group('RestoreSection', () {
    testWidgets('renders create special backup button', (tester) async {
      await tester.pumpWidget(wrap(RestoreSection(
        specialBackups: const [],
        isCreatingSpecial: false,
        specialBackupProgress: 0,
        isRestoring: false,
        onCreateSpecialBackup: () {},
        onRestoreSpecialBackup: (_) {},
        onDeleteSpecialBackup: (_) {},
        onRenameSpecialBackup: (_) {},
      )));

      expect(find.byType(TextButton), findsOneWidget);
      expect(find.byIcon(Icons.add), findsOneWidget);
    });

    testWidgets('shows empty state when no special backups', (tester) async {
      await tester.pumpWidget(wrap(RestoreSection(
        specialBackups: const [],
        isCreatingSpecial: false,
        specialBackupProgress: 0,
        isRestoring: false,
        onCreateSpecialBackup: () {},
        onRestoreSpecialBackup: (_) {},
        onDeleteSpecialBackup: (_) {},
        onRenameSpecialBackup: (_) {},
      )));

      expect(find.byType(Text), findsWidgets);
    });

    testWidgets('shows progress indicator when creating', (tester) async {
      await tester.pumpWidget(wrap(RestoreSection(
        specialBackups: const [],
        isCreatingSpecial: true,
        specialBackupProgress: 0.5,
        isRestoring: false,
        onCreateSpecialBackup: () {},
        onRestoreSpecialBackup: (_) {},
        onDeleteSpecialBackup: (_) {},
        onRenameSpecialBackup: (_) {},
      )));

      expect(find.byType(LinearProgressIndicator), findsOneWidget);
    });

    testWidgets('renders special backup list tiles', (tester) async {
      final backups = [
        BackupEntry(
          fileName: 'special_vacation.backup',
          createdAt: DateTime(2024, 6, 1),
          sizeBytes: 2048,
        ),
      ];

      await tester.pumpWidget(wrap(RestoreSection(
        specialBackups: backups,
        isCreatingSpecial: false,
        specialBackupProgress: 0,
        isRestoring: false,
        onCreateSpecialBackup: () {},
        onRestoreSpecialBackup: (_) {},
        onDeleteSpecialBackup: (_) {},
        onRenameSpecialBackup: (_) {},
      )));

      expect(find.byIcon(Icons.star), findsOneWidget);
    });

    testWidgets('create button is disabled when creating', (tester) async {
      await tester.pumpWidget(wrap(RestoreSection(
        specialBackups: const [],
        isCreatingSpecial: true,
        specialBackupProgress: 0,
        isRestoring: false,
        onCreateSpecialBackup: () {},
        onRestoreSpecialBackup: (_) {},
        onDeleteSpecialBackup: (_) {},
        onRenameSpecialBackup: (_) {},
      )));

      final button = tester.widget<TextButton>(find.byType(TextButton));
      expect(button.onPressed, isNull);
    });
  });
}
