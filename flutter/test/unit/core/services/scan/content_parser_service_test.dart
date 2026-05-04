import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/scan/content_parser_service.dart';

void main() {
  group('ContentParserService', () {
    late Directory tempDir;

    setUp(() async {
      tempDir = await Directory.systemTemp.createTemp('scan_test_');
    });

    tearDown(() async {
      await tempDir.delete(recursive: true);
    });

    test('extracts text from txt file', () async {
      final file = File('${tempDir.path}/test.txt');
      await file.writeAsString('Hello World 13800138000');

      final text = await ContentParserService.extractText(file.path);
      expect(text, contains('Hello World'));
      expect(text, contains('13800138000'));
    });

    test('extracts text from json file', () async {
      final file = File('${tempDir.path}/test.json');
      await file.writeAsString('{"name": "Zhang San", "phone": "13800138000"}');

      final text = await ContentParserService.extractText(file.path);
      expect(text, contains('Zhang San'));
      expect(text, contains('13800138000'));
    });

    test('extracts text from csv file', () async {
      final file = File('${tempDir.path}/test.csv');
      await file.writeAsString('name,phone\nZhang San,13800138000');

      final text = await ContentParserService.extractText(file.path);
      expect(text, contains('Zhang San'));
      expect(text, contains('13800138000'));
    });

    test('returns null for non-existent file', () async {
      final text = await ContentParserService.extractText('${tempDir.path}/nonexistent.txt');
      expect(text, isNull);
    });

    test('extracts text from md file', () async {
      final file = File('${tempDir.path}/test.md');
      await file.writeAsString('# Resume\n\nName: Zhang San\nPhone: 13800138000');

      final text = await ContentParserService.extractText(file.path);
      expect(text, contains('Resume'));
      expect(text, contains('Zhang San'));
    });
  });
}
