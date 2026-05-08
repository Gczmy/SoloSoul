import 'dart:io';

import 'package:solosoul_flutter/core/models/scan/scan_result_model.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';

// =============================================================================
// Windows Search Service (Everything SDK)
// =============================================================================

/// Windows-specific file search using Everything SDK CLI.
/// Falls back to PowerShell if es.exe is not available.
class WindowsSearchService {
  /// Search for files matching extensions under a path.
  static Future<List<ScannedFile>> searchFiles(
    String rootPath,
    List<String> extensions, {
    int maxFiles = 500,
  }) async {
    // Try Everything SDK CLI first
    final esResults = await _searchWithEs(rootPath, extensions, maxFiles: maxFiles);
    if (esResults.isNotEmpty) return esResults;

    // Fallback to PowerShell
    return _searchWithPowerShell(rootPath, extensions, maxFiles: maxFiles);
  }

  /// Use Everything SDK es.exe if available.
  static Future<List<ScannedFile>> _searchWithEs(
    String rootPath,
    List<String> extensions, {
    int maxFiles = 500,
  }) async {
    final results = <ScannedFile>[];

    try {
      // Build extension query: ext:pdf|docx|xlsx
      final extQuery = extensions.map((e) => e.replaceFirst('.', '')).join('|');
      final result = await Process.run('es', [
        '-path', rootPath,
        '-ext', extQuery,
        '-size',
        '-dm',
        '-sort', 'date-modified',
        '-n', '200',
      ]);

      if (result.exitCode == 0 && result.stdout is String) {
        final lines = (result.stdout as String).trim().split('\n');
        for (final line in lines) {
          if (line.isEmpty) continue;
          if (results.length >= maxFiles) break;
          // es.exe output format: size date_modified path
          final parts = line.split('\t');
          if (parts.length >= 3) {
            final size = int.tryParse(parts[0].trim()) ?? 0;
            final dateStr = parts[1].trim();
            final path = parts[2].trim();
            final modifiedAt = _parseEsDate(dateStr);

            results.add(ScannedFile(
              path: path,
              name: path.split('\\').last,
              size: size,
              modifiedAt: modifiedAt,
              extension: _extension(path),
            ));
          }
        }
      }
    } on Exception catch (_) {
      // es.exe not available
    }

    return results;
  }

  /// Fallback: use PowerShell Get-ChildItem.
  static Future<List<ScannedFile>> _searchWithPowerShell(
    String rootPath,
    List<String> extensions, {
    int maxFiles = 500,
  }) async {
    // Validate path against injection — PowerShell command uses string
    // interpolation so we must reject metacharacters before building cmd.
    if (!_isSafePath(rootPath)) {
      SoloLog.w('WindowsSearch', 'Rejected unsafe path: $rootPath');
      return [];
    }

    final results = <ScannedFile>[];

    try {
      // Escape double-quotes in the validated path for PowerShell.
      final safePath = rootPath.replaceAll('"', '`"');
      final extList = extensions.map((e) => '*$e').join(',');
      final cmd = 'Get-ChildItem -Path "$safePath" -Recurse -Include $extList '
          '-File -ErrorAction SilentlyContinue | Select-Object -First 200 | '
          r'ForEach-Object { "$($_.FullName)|$($_.Length)|$($_.LastWriteTimeUtc.Ticks)" }';
      final result = await Process.run('powershell', [
        '-Command',
        cmd,
      ]);

      if (result.exitCode == 0 && result.stdout is String) {
        final lines = (result.stdout as String).trim().split('\n');
        for (final line in lines) {
          if (results.length >= maxFiles) break;
          final parts = line.split('|');
          if (parts.length >= 3) {
            final path = parts[0];
            final size = int.tryParse(parts[1]) ?? 0;
            final ticks = int.tryParse(parts[2]) ?? 0;
            results.add(ScannedFile(
              path: path,
              name: path.split('\\').last,
              size: size,
              modifiedAt: ticks ~/ 10000 - 62135596800000,
              extension: _extension(path),
            ));
          }
        }
      }
    } on Exception catch (e) {
      SoloLog.w('WINDOWS_SEARCH', 'es.exe search failed', e);
    }

    return results;
  }

  static int _parseEsDate(String dateStr) {
    try {
      // Everything date format: YYYY/MM/DD HH:MM:SS
      final parts = dateStr.split(' ');
      if (parts.length == 2) {
        final dateParts = parts[0].split('/');
        final timeParts = parts[1].split(':');
        if (dateParts.length == 3 && timeParts.length == 3) {
          final dt = DateTime(
            int.parse(dateParts[0]),
            int.parse(dateParts[1]),
            int.parse(dateParts[2]),
            int.parse(timeParts[0]),
            int.parse(timeParts[1]),
            int.parse(timeParts[2]),
          );
          return dt.millisecondsSinceEpoch;
        }
      }
    } on Exception catch (e) {
      SoloLog.w('WINDOWS_SEARCH', 'Date parse failed: $dateStr', e);
    }
    return 0;
  }

  static String _extension(String path) {
    final idx = path.lastIndexOf('.');
    return idx >= 0 ? path.substring(idx).toLowerCase() : '';
  }

  /// Reject paths containing PowerShell metacharacters to prevent injection.
  static bool _isSafePath(String path) {
    // Allow only alphanumeric, common path separators, whitespace, and
    // standard Windows path characters (colon for drive letter, etc.).
    return RegExp(r'^[a-zA-Z0-9_:\\\/\.\-\s]+$').hasMatch(path);
  }
}
