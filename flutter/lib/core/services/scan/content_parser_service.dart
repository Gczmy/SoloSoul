import 'dart:async';
import 'dart:io';
import 'dart:convert';
import 'dart:typed_data';

import 'package:archive/archive.dart';
import 'package:xml/xml.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';

// =============================================================================
// Content Parser Service
// =============================================================================

/// Parses various file formats into plain text for fingerprint matching.
class ContentParserService {
  static const _maxFileSize = 10 * 1024 * 1024; // 10MB
  static const _maxTextLength = 1024 * 1024; // 1MB text limit

  /// Extract plain text from a file based on its extension.
  ///
  /// Returns null if the file doesn't exist, is unreadable, or parsing
  /// exceeds the 15-second timeout (prevents hanging on corrupted files).
  static Future<String?> extractText(String filePath) async {
    final file = File(filePath);
    if (!await file.exists()) return null;

    final stat = await file.stat();
    if (stat.size > _maxFileSize) {
      // For large files, only read the first chunk
      return _extractHead(filePath, stat.size);
    }

    final ext = _extension(filePath).toLowerCase();

    Future<String?> parse() async {
      switch (ext) {
        case '.txt':
        case '.md':
        case '.json':
        case '.csv':
        case '.xml':
        case '.html':
        case '.htm':
          return _readTextFile(filePath);
        case '.pdf':
          return _extractPdfText(filePath);
        case '.docx':
          return _extractDocxText(filePath);
        case '.xlsx':
          return _extractXlsxText(filePath);
        default:
          return _readTextFile(filePath);
      }
    }

    try {
      return await parse().timeout(const Duration(seconds: 15));
    } on TimeoutException {
      return null;
    }
  }

  /// Extract text from the beginning of a large file.
  static Future<String?> _extractHead(String filePath, int totalSize) async {
    final file = File(filePath);
    final maxRead = totalSize > _maxFileSize ? _maxFileSize : totalSize;
    final chunks = <int>[];
    await for (final chunk in file.openRead(0, maxRead)) {
      chunks.addAll(chunk);
    }
    final bytes = Uint8List.fromList(chunks);
    return _decodeBytes(bytes);
  }

  static String _extension(String path) {
    final idx = path.lastIndexOf('.');
    return idx >= 0 ? path.substring(idx) : '';
  }

  static Future<String?> _readTextFile(String path) async {
    try {
      final bytes = await File(path).readAsBytes();
      return _decodeBytes(bytes);
    } on Exception catch (_) {
      return null;
    }
  }

  static String? _decodeBytes(Uint8List bytes) {
    // Try UTF-8 first
    try {
      final text = utf8.decode(bytes, allowMalformed: true);
      if (text.length > _maxTextLength) {
        return text.substring(0, _maxTextLength);
      }
      return text;
    } on Exception catch (_) {
      return null;
    }
  }

  // ---------------------------------------------------------------------------
  // PDF Extraction
  // ---------------------------------------------------------------------------

  static Future<String?> _extractPdfText(String path) async {
    // Validate path to prevent command injection
    if (!_isSafePath(path)) {
      SoloLog.w('CONTENT_PARSER', 'Rejected unsafe path: $path');
      return null;
    }

    // Try pdftotext first (poppler-utils)
    try {
      final result = await Process.run('pdftotext', [path, '-'])
          .timeout(const Duration(seconds: 10));
      if (result.exitCode == 0 && result.stdout is String) {
        return _truncate(result.stdout as String);
      }
    } on Exception catch (e) {
      SoloLog.w('CONTENT_PARSER', 'pdftotext failed for $path', e);
    }

    // Fallback: strings command
    try {
      final result = await Process.run('strings', [path])
          .timeout(const Duration(seconds: 10));
      if (result.exitCode == 0 && result.stdout is String) {
        return _truncate(result.stdout as String);
      }
    } on Exception catch (e) {
      SoloLog.w('CONTENT_PARSER', 'strings failed for $path', e);
    }

    // Last resort: read raw and filter printable chars
    try {
      final bytes = await File(path).readAsBytes();
      final buffer = StringBuffer();
      for (final b in bytes) {
        if (b >= 32 && b < 127 || b == 10 || b == 13) {
          buffer.writeCharCode(b);
        }
      }
      return _truncate(buffer.toString());
    } on Exception catch (_) {
      return null;
    }
  }

  // ---------------------------------------------------------------------------
  // DOCX Extraction
  // ---------------------------------------------------------------------------

  static Future<String?> _extractDocxText(String path) async {
    try {
      final bytes = await File(path).readAsBytes();
      final archive = ZipDecoder().decodeBytes(bytes);

      // Find word/document.xml
      final docXml = archive.files.firstWhere(
        (f) => f.name == 'word/document.xml',
        orElse: () => throw Exception('document.xml not found'),
      );

      final content = utf8.decode(docXml.content as List<int>);
      final document = XmlDocument.parse(content);

      // Extract all w:t elements
      final texts = <String>[];
      for (final node in document.findAllElements('w:t')) {
        final text = node.innerText.trim();
        if (text.isNotEmpty) texts.add(text);
      }

      return _truncate(texts.join(' '));
    } on Exception catch (_) {
      return null;
    }
  }

  // ---------------------------------------------------------------------------
  // XLSX Extraction
  // ---------------------------------------------------------------------------

  static Future<String?> _extractXlsxText(String path) async {
    try {
      final bytes = await File(path).readAsBytes();
      final archive = ZipDecoder().decodeBytes(bytes);

      // Read shared strings if present
      final sharedStrings = <int, String>{};
      try {
        final ssFile = archive.files.firstWhere(
          (f) => f.name == 'xl/sharedStrings.xml',
          orElse: () => throw Exception('sharedStrings.xml not found'),
        );
        final ssContent = utf8.decode(ssFile.content as List<int>);
        final ssDoc = XmlDocument.parse(ssContent);
        var idx = 0;
        for (final node in ssDoc.findAllElements('si')) {
          sharedStrings[idx++] = node.innerText;
        }
      } on Exception catch (e) {
        SoloLog.w('CONTENT_PARSER', 'Excel sharedStrings parse failed', e);
      }

      // Read first worksheet
      final sheetFile = archive.files.firstWhere(
        (f) => f.name.startsWith('xl/worksheets/sheet') && f.name.endsWith('.xml'),
        orElse: () => throw Exception('No worksheet found'),
      );

      final sheetContent = utf8.decode(sheetFile.content as List<int>);
      final sheetDoc = XmlDocument.parse(sheetContent);

      final texts = <String>[];
      for (final cell in sheetDoc.findAllElements('c')) {
        final type = cell.getAttribute('t');
        final valueNode = cell.getElement('v');
        if (valueNode == null) continue;

        if (type == 's') {
          // Shared string reference
          final idx = int.tryParse(valueNode.innerText);
          if (idx != null && sharedStrings.containsKey(idx)) {
            texts.add(sharedStrings[idx]!);
          }
        } else {
          texts.add(valueNode.innerText);
        }
      }

      return _truncate(texts.join(' '));
    } on Exception catch (_) {
      return null;
    }
  }

  static String? _truncate(String text) {
    if (text.length > _maxTextLength) {
      return text.substring(0, _maxTextLength);
    }
    return text;
  }

  static bool _isSafePath(String path) {
    // Allow only alphanumeric, common path separators, whitespace, and
    // standard path characters (colon for drive letter, etc.).
    return RegExp(r'^[a-zA-Z0-9_:\\/\.\-\s]+$').hasMatch(path);
  }
}
