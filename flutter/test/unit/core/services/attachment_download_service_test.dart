import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/attachment_download_service.dart';

void main() {
  group('AttachmentDownloadService.resolveUniquePath', () {
    late AttachmentDownloadService service;
    late Directory tempDir;

    setUp(() {
      service = AttachmentDownloadService();
      tempDir = Directory.systemTemp.createTempSync('solosoul_test_');
    });

    tearDown(() {
      tempDir.deleteSync(recursive: true);
    });

    test('returns original path when file does not exist', () {
      final result = service.resolveUniquePath(tempDir.path, 'report.pdf');
      expect(result, endsWith('report.pdf'));
    });

    test('appends (1) when file exists', () {
      File('${tempDir.path}/report.pdf').createSync();
      final result = service.resolveUniquePath(tempDir.path, 'report.pdf');
      expect(result, endsWith('report (1).pdf'));
    });

    test('increments number for multiple conflicts', () {
      File('${tempDir.path}/report.pdf').createSync();
      File('${tempDir.path}/report (1).pdf').createSync();
      File('${tempDir.path}/report (2).pdf').createSync();
      final result = service.resolveUniquePath(tempDir.path, 'report.pdf');
      expect(result, endsWith('report (3).pdf'));
    });

    test('increments from existing numbered file', () {
      File('${tempDir.path}/report (2).pdf').createSync();
      final result = service.resolveUniquePath(tempDir.path, 'report (2).pdf');
      expect(result, endsWith('report (3).pdf'));
    });

    test('handles files without extension', () {
      File('${tempDir.path}/README').createSync();
      final result = service.resolveUniquePath(tempDir.path, 'README');
      expect(result, endsWith('README (1)'));
    });

    test('handles multiple conflicts without extension', () {
      File('${tempDir.path}/README').createSync();
      File('${tempDir.path}/README (1)').createSync();
      final result = service.resolveUniquePath(tempDir.path, 'README');
      expect(result, endsWith('README (2)'));
    });

    test('handles complex extensions', () {
      File('${tempDir.path}/archive.tar.gz').createSync();
      final result = service.resolveUniquePath(tempDir.path, 'archive.tar.gz');
      // path.basenameWithoutExtension returns 'archive.tar', extension returns '.gz'
      expect(result, endsWith('archive.tar (1).gz'));
    });

    test('handles files with spaces in name', () {
      File('${tempDir.path}/my report.pdf').createSync();
      final result = service.resolveUniquePath(tempDir.path, 'my report.pdf');
      expect(result, endsWith('my report (1).pdf'));
    });
  });
}
