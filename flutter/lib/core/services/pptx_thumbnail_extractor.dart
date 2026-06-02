import 'dart:io';
import 'dart:typed_data';

import 'package:archive/archive.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';

// =============================================================================
// PPTX Thumbnail Extractor
// =============================================================================

/// 从 Office Open XML 文件（PPTX/DOCX/XLSX）字节中提取内置缩略图（docProps/thumbnail.jpeg）。
///
/// PPTX/DOCX/XLSX 本质上是 ZIP 文件，此方法只读取 ZIP 目录结构并解压
/// 缩略图文件，不需要将整个文件解压到内存。
///
/// 超过 [_maxExtractSize] 的文件直接返回 null，避免内存峰值过高。
/// 旧格式（PPT/DOC/XLS 等二进制 OLE2 文件）不是 ZIP，会直接返回 null。
class PptxThumbnailExtractor {
  /// 大于此阈值的 PPTX 文件直接跳过缩略图提取，避免内存峰值过高。
  static const int _maxExtractSize = 20 * 1024 * 1024; // 20 MB

  /// 从 PPTX 字节数据中提取缩略图（docProps/thumbnail.jpeg）。
  ///
  /// 超过 [_maxExtractSize] 的文件直接返回 null，调用方应 fallback 到
  /// 系统应用打开，以避免 OOM。
  ///
  /// 返回缩略图的 JPEG 字节，如果没有缩略图或文件过大则返回 null。
  static Uint8List? extractThumbnail(Uint8List pptxBytes) {
    if (pptxBytes.length > _maxExtractSize) {
      SoloLog.d(
        'PptxThumbnail',
        'File too large (${pptxBytes.length} bytes), skipping thumbnail extraction',
      );
      return null;
    }
    if (!_isZip(pptxBytes)) return null;
    try {
      final archive = ZipDecoder().decodeBytes(pptxBytes);
      return _findThumbnailInArchive(archive);
    } on Exception catch (e) {
      SoloLog.w('PptxThumbnail', 'Failed to extract thumbnail: $e');
      return null;
    }
  }

  /// 从 PPTX 文件路径提取缩略图。
  ///
  /// 适用于已解密到临时文件的场景，避免先加载完整 bytes 到内存。
  /// 文件大小超过 [_maxExtractSize] 时返回 null。
  static Uint8List? extractThumbnailFromPath(String filePath) {
    try {
      final file = File(filePath);
      if (!file.existsSync()) return null;
      final size = file.lengthSync();
      if (size > _maxExtractSize) {
        SoloLog.d('PptxThumbnail',
            'File too large ($size bytes), skipping thumbnail extraction');
        return null;
      }
      final bytes = file.readAsBytesSync();
      if (!_isZip(bytes)) return null;
      final archive = ZipDecoder().decodeBytes(bytes);
      return _findThumbnailInArchive(archive);
    } on Exception catch (e) {
      SoloLog.w('PptxThumbnail', 'Failed to extract thumbnail from path: $e');
      return null;
    }
  }

  static Uint8List? _findThumbnailInArchive(Archive archive) {
    final thumbnailFile = archive.findFile('docProps/thumbnail.jpeg');
    if (thumbnailFile != null) {
      return Uint8List.fromList(thumbnailFile.content);
    }
    // 某些 PowerPoint 版本可能使用 .jpg 扩展名
    final thumbnailFileJpg = archive.findFile('docProps/thumbnail.jpg');
    if (thumbnailFileJpg != null) {
      return Uint8List.fromList(thumbnailFileJpg.content);
    }
    return null;
  }

  /// 判断字节数据是否为 ZIP 格式（以 PK 开头）。
  static bool _isZip(Uint8List bytes) {
    return bytes.length >= 2 && bytes[0] == 0x50 && bytes[1] == 0x4B;
  }

  /// 判断字节数据是否为有效的 PPTX（ZIP 格式且包含 [Content_Types].xml）。
  /// 当 MIME 类型缺失时可用作备用检测。
  static bool isPptx(Uint8List bytes) {
    if (bytes.length < 4) return false;
    if (!_isZip(bytes)) return false;
    try {
      final archive = ZipDecoder().decodeBytes(bytes);
      return archive.findFile('[Content_Types].xml') != null;
    } on Exception {
      return false;
    }
  }
}
