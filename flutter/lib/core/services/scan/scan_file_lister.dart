import 'dart:convert';
import 'dart:io';

import 'package:solosoul_flutter/core/models/scan/scan_result_model.dart';
import 'package:solosoul_flutter/core/services/scan/cancel_token.dart';
import 'package:solosoul_flutter/core/services/scan/windows_search_service.dart';

// =============================================================================
// Scan File Lister
// =============================================================================

/// Handles file discovery across different platforms.
class ScanFileLister {
  /// List files matching target extensions under a path.
  static Future<List<ScannedFile>> listFiles(
    String rootPath,
    List<String> extensions, {
    int maxFiles = 500,
    CancelToken? cancelToken,
  }) async {
    final results = <ScannedFile>[];

    // Use platform-specific fast listing
    if (Platform.isMacOS) {
      results.addAll(await _listFilesMacOS(rootPath, extensions, maxFiles: maxFiles, cancelToken: cancelToken));
    } else if (Platform.isWindows) {
      results.addAll(await _listFilesWindows(rootPath, extensions, maxFiles: maxFiles, cancelToken: cancelToken));
    } else {
      results.addAll(await _listFilesGeneric(rootPath, extensions, maxFiles: maxFiles, cancelToken: cancelToken));
    }

    return results;
  }

  /// macOS: use find command.
  static Future<List<ScannedFile>> _listFilesMacOS(
    String rootPath,
    List<String> extensions, {
    int maxFiles = 500,
    CancelToken? cancelToken,
  }) async {
    final results = <ScannedFile>[];

    // Build extension filter for find
    final extArgs = <String>[];
    for (var i = 0; i < extensions.length; i++) {
      if (i > 0) extArgs.add('-o');
      extArgs.addAll(['-iname', '*${extensions[i]}']);
    }

    try {
      final process = await Process.start('find', [
        rootPath,
        '-maxdepth',
        '3',
        '-type',
        'f',
        '(',
        ...extArgs,
        ')',
      ]);

      final stdoutLines = process.stdout.transform(utf8.decoder).transform(const LineSplitter());
      await for (final line in stdoutLines) {
        if (cancelToken?.isCanceled ?? false) {
          process.kill();
          return results;
        }
        if (line.isEmpty) continue;
        if (results.length >= maxFiles) {
          process.kill();
          break;
        }
        final file = File(line);
        final stat = await file.stat();
        if (stat.type == FileSystemEntityType.file) {
          results.add(ScannedFile(
            path: line,
            name: line.split('/').last,
            size: stat.size,
            modifiedAt: stat.modified.millisecondsSinceEpoch,
            extension: _extension(line),
          ));
        }
      }

      // Ensure process exits; kill if still running after stream ends.
      if (cancelToken?.isCanceled ?? false) {
        process.kill();
      } else {
        await process.exitCode.timeout(const Duration(seconds: 30), onTimeout: () {
          process.kill();
          return -1;
        });
      }
    } on Exception catch (_) {
      // Fallback to generic
      return _listFilesGeneric(rootPath, extensions, maxFiles: maxFiles, cancelToken: cancelToken);
    }

    return results;
  }

  /// Windows: use Everything SDK or PowerShell.
  static Future<List<ScannedFile>> _listFilesWindows(
    String rootPath,
    List<String> extensions, {
    int maxFiles = 500,
    CancelToken? cancelToken,
  }) async {
    if (cancelToken?.isCanceled ?? false) return <ScannedFile>[];
    return WindowsSearchService.searchFiles(rootPath, extensions, maxFiles: maxFiles);
  }

  /// Generic: Dart directory traversal.
  static Future<List<ScannedFile>> _listFilesGeneric(
    String rootPath,
    List<String> extensions, {
    int maxFiles = 500,
    CancelToken? cancelToken,
  }) async {
    final results = <ScannedFile>[];
    final dir = Directory(rootPath);
    if (!await dir.exists()) return results;

    try {
      await for (final entity in dir.list(recursive: true, followLinks: false)) {
        if (cancelToken?.isCanceled ?? false) break;
        if (entity is File) {
          final ext = _extension(entity.path).toLowerCase();
          if (extensions.contains(ext)) {
            final stat = await entity.stat();
            results.add(ScannedFile(
              path: entity.path,
              name: entity.path.split(Platform.pathSeparator).last,
              size: stat.size,
              modifiedAt: stat.modified.millisecondsSinceEpoch,
              extension: ext,
            ));
            if (results.length >= maxFiles) break; // Limit to prevent overload
          }
        }
      }
    } on Exception catch (_) {
      // Ignore permission errors
    }

    return results;
  }

  static String _extension(String path) {
    final idx = path.lastIndexOf('.');
    return idx >= 0 ? path.substring(idx).toLowerCase() : '';
  }
}
