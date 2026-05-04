import 'dart:convert';
import 'dart:io';

import 'package:path_provider/path_provider.dart';

// =============================================================================
// Scan Cache Service
// =============================================================================

/// Caches scan metadata to avoid re-parsing unchanged files.
/// Stores {path, mtime, size} records in app documents directory.
class ScanCacheService {
  static const _cacheFileName = 'scan_cache.json';
  static ScanCacheService? _instance;
  static ScanCacheService get instance => _instance ??= ScanCacheService._();
  ScanCacheService._();

  Map<String, ScanCacheEntry> _cache = {};
  bool _loaded = false;

  /// Load cache from disk.
  Future<void> load() async {
    if (_loaded) return;
    try {
      final dir = await getApplicationDocumentsDirectory();
      final file = File('${dir.path}/$_cacheFileName');
      if (await file.exists()) {
        final json = jsonDecode(await file.readAsString()) as Map<String, dynamic>;
        _cache = {
          for (final entry in json.entries)
            entry.key: ScanCacheEntry.fromJson(entry.value as Map<String, dynamic>),
        };
      }
    } on Exception catch (_) {
      // Ignore load errors
    }
    _loaded = true;
  }

  /// Save cache to disk.
  Future<void> save() async {
    try {
      final dir = await getApplicationDocumentsDirectory();
      final file = File('${dir.path}/$_cacheFileName');
      final json = {
        for (final entry in _cache.entries) entry.key: entry.value.toJson(),
      };
      await file.writeAsString(jsonEncode(json));
    } on Exception catch (_) {
      // Ignore save errors
    }
  }

  /// Check if a file has changed since last scan.
  bool isChanged(String path, int mtime, int size) {
    final entry = _cache[path];
    if (entry == null) return true;
    return entry.mtime != mtime || entry.size != size;
  }

  /// Update cache entry for a file.
  void update(String path, int mtime, int size) {
    _cache[path] = ScanCacheEntry(path: path, mtime: mtime, size: size);
  }

  /// Remove entries that no longer exist.
  void prune(Set<String> existingPaths) {
    _cache.removeWhere((path, _) => !existingPaths.contains(path));
  }

  /// Clear all cache entries.
  void clear() {
    _cache.clear();
  }
}

// =============================================================================
// Scan Cache Entry
// =============================================================================

class ScanCacheEntry {
  final String path;
  final int mtime;
  final int size;

  ScanCacheEntry({
    required this.path,
    required this.mtime,
    required this.size,
  });

  factory ScanCacheEntry.fromJson(Map<String, dynamic> json) {
    return ScanCacheEntry(
      path: json['path'] as String,
      mtime: json['mtime'] as int,
      size: json['size'] as int,
    );
  }

  Map<String, dynamic> toJson() => {
        'path': path,
        'mtime': mtime,
        'size': size,
      };
}
