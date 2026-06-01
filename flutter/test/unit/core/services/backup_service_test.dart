import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/backup_service.dart';

void main() {
  group('BackupEntry', () {
    test('displayTime formats correctly', () {
      final entry = BackupEntry(
        fileName: 'test.backup',
        createdAt: DateTime(2025, 6, 15, 9, 5, 3),
        sizeBytes: 1024,
      );

      expect(entry.displayTime, '2025-06-15 09:05:03');
    });

    test('displayTime pads single digit values', () {
      final entry = BackupEntry(
        fileName: 'test.backup',
        createdAt: DateTime(2025, 1, 2, 3, 4, 5),
        sizeBytes: 0,
      );

      expect(entry.displayTime, '2025-01-02 03:04:05');
    });
  });

  group('BackupService.backupFileName', () {
    test('generates filename without version', () {
      final dt = DateTime(2025, 6, 15, 9, 5, 3);
      final name = BackupService.backupFileName(dt);

      expect(name, startsWith('backup_'));
      expect(name, endsWith('.backup'));
      expect(name, contains('2025-06-15_09-05-03'));
    });

    test('generates filename with version suffix', () {
      final dt = DateTime(2025, 6, 15, 9, 5, 3);
      final name = BackupService.backupFileName(dt, appVersion: '1.2.3');

      expect(name, 'backup_2025-06-15_09-05-03_v1.2.3.backup');
    });

    test('pads single digit date/time components', () {
      final dt = DateTime(2025, 1, 2, 3, 4, 5);
      final name = BackupService.backupFileName(dt);

      expect(name, 'backup_2025-01-02_03-04-05.backup');
    });
  });

  group('BackupService.sanitizeSpecialName', () {
    test('replaces forward slashes', () {
      expect(BackupService.sanitizeSpecialName('a/b/c'), 'a-b-c');
    });

    test('replaces backslashes', () {
      expect(BackupService.sanitizeSpecialName('a\\b\\c'), 'a-b-c');
    });

    test('replaces double dots', () {
      expect(BackupService.sanitizeSpecialName('..hidden'), '-hidden');
    });

    test('trims whitespace', () {
      expect(BackupService.sanitizeSpecialName('  name  '), 'name');
    });

    test('handles complex path traversal attempt', () {
      expect(
        BackupService.sanitizeSpecialName('../../../etc/passwd'),
        '------etc-passwd',
      );
    });

    test('returns plain name unchanged', () {
      expect(BackupService.sanitizeSpecialName('My Backup'), 'My Backup');
    });
  });
}
