import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/profile_data.dart';
import 'package:solosoul_flutter/core/services/backup_service.dart';

void main() {
  group('BackupService.backupFileName', () {
    test('generates correct filename without version', () {
      final dt = DateTime(2024, 3, 15, 14, 30, 45);
      final name = BackupService.backupFileName(dt);
      expect(name, 'backup_2024-03-15_14-30-45.backup');
    });

    test('generates correct filename with version', () {
      final dt = DateTime(2024, 12, 1, 9, 5, 0);
      final name = BackupService.backupFileName(dt, appVersion: '1.2.3');
      expect(name, 'backup_2024-12-01_09-05-00_v1.2.3.backup');
    });

    test('pads single digit components', () {
      final dt = DateTime(2024, 1, 2, 3, 4, 5);
      final name = BackupService.backupFileName(dt);
      expect(name, 'backup_2024-01-02_03-04-05.backup');
    });

    test('uses correct prefix and extension', () {
      final dt = DateTime(2024, 6, 15, 10, 0, 0);
      final name = BackupService.backupFileName(dt);
      expect(name.startsWith('backup_'), isTrue);
      expect(name.endsWith('.backup'), isTrue);
    });
  });

  group('BackupService.sanitizeSpecialName', () {
    test('removes forward slashes', () {
      expect(BackupService.sanitizeSpecialName('a/b/c'), 'a-b-c');
    });

    test('removes backslashes', () {
      expect(BackupService.sanitizeSpecialName('a\\b\\c'), 'a-b-c');
    });

    test('removes dot-dot sequences', () {
      expect(BackupService.sanitizeSpecialName('..hidden'), '-hidden');
      expect(BackupService.sanitizeSpecialName('foo..bar'), 'foo-bar');
    });

    test('trims whitespace', () {
      expect(BackupService.sanitizeSpecialName('  name  '), 'name');
    });

    test('allows safe characters', () {
      expect(BackupService.sanitizeSpecialName('My Backup v1.0'), 'My Backup v1.0');
    });

    test('handles empty string', () {
      expect(BackupService.sanitizeSpecialName(''), '');
    });

    test('handles complex path traversal attempt', () {
      final result = BackupService.sanitizeSpecialName('../../../etc/passwd');
      // All slashes and dot-dot sequences replaced with dashes
      expect(result.contains('/'), isFalse);
      expect(result.contains('\\'), isFalse);
      expect(result.contains('..'), isFalse);
      expect(result.endsWith('etc-passwd'), isTrue);
    });
  });

  group('BackupService static constants', () {
    test('maxBackupCount is 5', () {
      expect(BackupService.maxBackupCount, 5);
    });

    test('maxSpecialBackupCount is 5', () {
      expect(BackupService.maxSpecialBackupCount, 5);
    });
  });

  group('BackupService isolate helpers', () {
    test('_encodeProfileToBytes produces valid UTF-8 bytes', () {
      final json = <String, dynamic>{'key': 'value', 'number': 42};
      final bytes = BackupService.encodeProfileToBytes(json);
      expect(bytes, isA<Uint8List>());
      expect(bytes.isNotEmpty, isTrue);

      final decoded = utf8.decode(bytes);
      expect(decoded, contains('key'));
      expect(decoded, contains('value'));
    });

    test('_decodeProfileFromString round-trips ProfileData', () {
      final profile = ProfileData(
        unifiedObjects: null,
        schemaVersion: 4,
      );
      final jsonString = jsonEncode(profile.toJson());
      final decoded = BackupService.decodeProfileFromString(jsonString);
      expect(decoded.schemaVersion, 4);
    });

    test('_decodeProfileFromString handles complex data', () {
      final jsonString = jsonEncode({
        'unified_objects': null,
        'schema_version': 4,
        'custom_types': [],
      });
      final decoded = BackupService.decodeProfileFromString(jsonString);
      expect(decoded.schemaVersion, 4);
    });
  });

  group('BackupEntry', () {
    test('displayTime formats correctly', () {
      final entry = BackupEntry(
        fileName: 'test.backup',
        createdAt: DateTime(2024, 6, 15, 14, 30, 45),
        sizeBytes: 1024,
      );
      expect(entry.displayTime, '2024-06-15 14:30:45');
    });

    test('displayTime pads single digits', () {
      final entry = BackupEntry(
        fileName: 'test.backup',
        createdAt: DateTime(2024, 1, 2, 3, 4, 5),
        sizeBytes: 100,
      );
      expect(entry.displayTime, '2024-01-02 03:04:05');
    });
  });
}
