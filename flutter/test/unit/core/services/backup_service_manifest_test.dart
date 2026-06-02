import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/backup_service.dart';

void main() {
  group('BackupService manifest', () {
    late Directory tempDir;

    setUp(() {
      tempDir = Directory.systemTemp.createTempSync('backup_manifest_test_');
    });

    tearDown(() {
      if (tempDir.existsSync()) {
        tempDir.deleteSync(recursive: true);
      }
    });

    test('writeManifest creates manifest.json', () {
      final sidecar = '${tempDir.path}/sidecar';
      Directory(sidecar).createSync();

      BackupService.writeManifest(sidecar, ['a', 'b', 'c']);

      final manifestFile = File('$sidecar/manifest.json');
      expect(manifestFile.existsSync(), true);

      final content = manifestFile.readAsStringSync();
      expect(content, contains('"version":1'));
      expect(content, contains('"fileIds"'));
      expect(content, contains('"a"'));
      expect(content, contains('"b"'));
      expect(content, contains('"c"'));
    });

    test('readManifest returns fileIds', () {
      final sidecar = '${tempDir.path}/sidecar';
      Directory(sidecar).createSync();
      BackupService.writeManifest(sidecar, ['x', 'y']);

      final ids = BackupService.readManifest(sidecar);
      expect(ids, equals(['x', 'y']));
    });

    test('readManifest returns empty list when manifest missing', () {
      final sidecar = '${tempDir.path}/empty_sidecar';
      Directory(sidecar).createSync();

      final ids = BackupService.readManifest(sidecar);
      expect(ids, isEmpty);
    });

    test('readManifest returns empty list for corrupted manifest', () {
      final sidecar = '${tempDir.path}/corrupt';
      Directory(sidecar).createSync();
      File('$sidecar/manifest.json').writeAsStringSync('not json');

      final ids = BackupService.readManifest(sidecar);
      expect(ids, isEmpty);
    });

    test('writeManifest overwrites existing manifest', () {
      final sidecar = '${tempDir.path}/sidecar';
      Directory(sidecar).createSync();

      BackupService.writeManifest(sidecar, ['old1', 'old2']);
      var ids = BackupService.readManifest(sidecar);
      expect(ids, equals(['old1', 'old2']));

      BackupService.writeManifest(sidecar, ['new1']);
      ids = BackupService.readManifest(sidecar);
      expect(ids, equals(['new1']));
    });
  });

  group('BackupService.extractAccountIdFromBackupPath', () {
    test('extracts accountId from macOS/Linux path', () {
      final path = '/Users/foo/Library/solosoul_backups/acc123/backup_2026-01-01_00-00-00.backup';
      final accountId = BackupService.extractAccountIdFromBackupPath(path);
      expect(accountId, 'acc123');
    });

    test('extracts accountId from nested path', () {
      final path = '/data/solosoul_backups/my_account/special/my.backup';
      final accountId = BackupService.extractAccountIdFromBackupPath(path);
      expect(accountId, 'my_account');;
    });

    test('returns null when solosoul_backups not found', () {
      final path = '/some/other/path/backup_2026-01-01.backup';
      final accountId = BackupService.extractAccountIdFromBackupPath(path);
      expect(accountId, isNull);
    });

    test('handles path without trailing segments', () {
      final path = '/solosoul_backups/acc999';
      final accountId = BackupService.extractAccountIdFromBackupPath(path);
      expect(accountId, 'acc999');
    });
  });
}
