import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/scan/scan_cache_service.dart';

void main() {
  group('ScanCacheService', () {
    late ScanCacheService cache;

    setUp(() {
      cache = ScanCacheService.instance;
      cache.clear();
    });

    tearDown(() {
      cache.clear();
    });

    group('isChanged', () {
      test('returns true for uncached file', () {
        expect(cache.isChanged('/path/to/file', 1000, 1024), isTrue);
      });

      test('returns false when mtime and size match', () {
        cache.update('/path/to/file', 1000, 1024);
        expect(cache.isChanged('/path/to/file', 1000, 1024), isFalse);
      });

      test('returns true when mtime differs', () {
        cache.update('/path/to/file', 1000, 1024);
        expect(cache.isChanged('/path/to/file', 2000, 1024), isTrue);
      });

      test('returns true when size differs', () {
        cache.update('/path/to/file', 1000, 1024);
        expect(cache.isChanged('/path/to/file', 1000, 2048), isTrue);
      });
    });

    group('update', () {
      test('adds new entry', () {
        cache.update('/new/file', 3000, 512);
        expect(cache.isChanged('/new/file', 3000, 512), isFalse);
      });

      test('overwrites existing entry', () {
        cache.update('/file', 1000, 1024);
        cache.update('/file', 2000, 2048);
        expect(cache.isChanged('/file', 2000, 2048), isFalse);
        expect(cache.isChanged('/file', 1000, 1024), isTrue);
      });
    });

    group('prune', () {
      test('removes entries not in existing paths', () {
        cache.update('/keep/this', 1000, 100);
        cache.update('/remove/this', 2000, 200);
        cache.prune({'/keep/this'});
        expect(cache.isChanged('/keep/this', 1000, 100), isFalse);
        expect(cache.isChanged('/remove/this', 2000, 200), isTrue);
      });

      test('removes all when empty set provided', () {
        cache.update('/file1', 1000, 100);
        cache.update('/file2', 2000, 200);
        cache.prune({});
        expect(cache.isChanged('/file1', 1000, 100), isTrue);
        expect(cache.isChanged('/file2', 2000, 200), isTrue);
      });
    });

    group('clear', () {
      test('removes all entries', () {
        cache.update('/file1', 1000, 100);
        cache.update('/file2', 2000, 200);
        cache.clear();
        expect(cache.isChanged('/file1', 1000, 100), isTrue);
        expect(cache.isChanged('/file2', 2000, 200), isTrue);
      });
    });

    group('ScanCacheEntry', () {
      test('fromJson parses correctly', () {
        final entry = ScanCacheEntry.fromJson({
          'path': '/test',
          'mtime': 1234,
          'size': 5678,
        });
        expect(entry.path, '/test');
        expect(entry.mtime, 1234);
        expect(entry.size, 5678);
      });

      test('toJson serializes correctly', () {
        final entry = ScanCacheEntry(path: '/test', mtime: 1234, size: 5678);
        final json = entry.toJson();
        expect(json['path'], '/test');
        expect(json['mtime'], 1234);
        expect(json['size'], 5678);
      });

      test('round-trip serialization', () {
        final original = ScanCacheEntry(path: '/test', mtime: 1234, size: 5678);
        final json = original.toJson();
        final restored = ScanCacheEntry.fromJson(json);
        expect(restored.path, original.path);
        expect(restored.mtime, original.mtime);
        expect(restored.size, original.size);
      });
    });
  });
}
