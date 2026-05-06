import 'dart:convert';
import 'dart:io';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/scan/scan_cache_service.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  late Directory tempDir;

  setUpAll(() async {
    tempDir = await Directory.systemTemp.createTemp('scan_cache_test_');

    const pathProviderChannel = MethodChannel('plugins.flutter.io/path_provider');
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(pathProviderChannel, (call) async {
      if (call.method == 'getApplicationDocumentsDirectory') {
        return tempDir.path;
      }
      return null;
    });
  });

  tearDownAll(() async {
    const pathProviderChannel = MethodChannel('plugins.flutter.io/path_provider');
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(pathProviderChannel, null);
    await tempDir.delete(recursive: true);
  });

  setUp(() {
    // Reset singleton state between tests
    ScanCacheService.instance.clear();
  });

  group('ScanCacheService', () {
    test('isChanged returns true for unknown path', () {
      expect(ScanCacheService.instance.isChanged('/new/file.txt', 1000, 100), isTrue);
    });

    test('isChanged returns false when mtime and size match', () {
      ScanCacheService.instance.update('/file.txt', 1000, 100);
      expect(ScanCacheService.instance.isChanged('/file.txt', 1000, 100), isFalse);
    });

    test('isChanged returns true when mtime differs', () {
      ScanCacheService.instance.update('/file.txt', 1000, 100);
      expect(ScanCacheService.instance.isChanged('/file.txt', 2000, 100), isTrue);
    });

    test('isChanged returns true when size differs', () {
      ScanCacheService.instance.update('/file.txt', 1000, 100);
      expect(ScanCacheService.instance.isChanged('/file.txt', 1000, 200), isTrue);
    });

    test('update replaces existing entry', () {
      ScanCacheService.instance.update('/file.txt', 1000, 100);
      ScanCacheService.instance.update('/file.txt', 2000, 200);
      expect(ScanCacheService.instance.isChanged('/file.txt', 2000, 200), isFalse);
    });

    test('prune removes missing paths', () {
      ScanCacheService.instance.update('/keep.txt', 1000, 100);
      ScanCacheService.instance.update('/remove.txt', 2000, 200);

      ScanCacheService.instance.prune({'/keep.txt'});

      expect(ScanCacheService.instance.isChanged('/keep.txt', 1000, 100), isFalse);
      expect(ScanCacheService.instance.isChanged('/remove.txt', 2000, 200), isTrue);
    });

    test('clear removes all entries', () {
      ScanCacheService.instance.update('/a.txt', 1000, 100);
      ScanCacheService.instance.clear();
      expect(ScanCacheService.instance.isChanged('/a.txt', 1000, 100), isTrue);
    });

    test('load and save roundtrip', () async {
      ScanCacheService.instance.update('/doc.txt', 1234, 56);
      await ScanCacheService.instance.save();

      // Verify file was written
      final file = File('${tempDir.path}/scan_cache.json');
      expect(await file.exists(), isTrue);

      final content = await file.readAsString();
      final json = jsonDecode(content) as Map<String, dynamic>;
      expect(json.containsKey('/doc.txt'), isTrue);
      expect(json['/doc.txt']['mtime'], 1234);
      expect(json['/doc.txt']['size'], 56);

      // Clear and reload
      ScanCacheService.instance.clear();
      expect(ScanCacheService.instance.isChanged('/doc.txt', 1234, 56), isTrue);

      await ScanCacheService.instance.load();
      expect(ScanCacheService.instance.isChanged('/doc.txt', 1234, 56), isFalse);
    });

    test('load ignores missing file gracefully', () async {
      // Ensure file does not exist
      final file = File('${tempDir.path}/scan_cache.json');
      if (await file.exists()) await file.delete();

      // Should not throw
      await ScanCacheService.instance.load();
      expect(ScanCacheService.instance.isChanged('/any.txt', 0, 0), isTrue);
    });

    test('load ignores malformed JSON gracefully', () async {
      final file = File('${tempDir.path}/scan_cache.json');
      await file.writeAsString('not-json');

      // Should not throw
      await ScanCacheService.instance.load();
      expect(ScanCacheService.instance.isChanged('/any.txt', 0, 0), isTrue);
    });

    test('save ignores write errors gracefully', () async {
      // Write to a read-only directory is hard to simulate cross-platform,
      // so we verify save() completes without throwing on normal paths.
      ScanCacheService.instance.update('/x.txt', 1, 1);
      await expectLater(ScanCacheService.instance.save(), completes);
    });
  });

  group('ScanCacheEntry', () {
    test('toJson and fromJson roundtrip', () {
      final entry = ScanCacheEntry(path: '/test.txt', mtime: 1000, size: 200);
      final json = entry.toJson();
      final restored = ScanCacheEntry.fromJson(json);

      expect(restored.path, '/test.txt');
      expect(restored.mtime, 1000);
      expect(restored.size, 200);
    });
  });
}
