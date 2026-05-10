import 'dart:async';
import 'dart:io';

import 'package:path_provider/path_provider.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/scan/scan_result_model.dart';
import 'package:solosoul_flutter/core/models/sensitivity_models.dart';
import 'package:solosoul_flutter/core/services/scan/content_parser_service.dart';
import 'package:solosoul_flutter/core/services/scan/cancel_token.dart';
import 'package:solosoul_flutter/core/services/scan/scan_cache_service.dart';
import 'package:solosoul_flutter/core/services/scan/scan_file_lister.dart';
import 'package:solosoul_flutter/core/services/scan/scan_image_scanner.dart';
import 'package:solosoul_flutter/core/services/scan/scan_section_detector.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:uuid/uuid.dart';

// Export for backwards compatibility
export 'cancel_token.dart' show CancelToken;
export 'scan_section_detector.dart' show ScanSectionDetector;

// =============================================================================
// Local Search Service
// =============================================================================

/// Scans local filesystem for files containing personal information.
/// Uses layered search strategy to avoid full-disk scanning.
class LocalSearchService {
  // ---------------------------------------------------------------------------
  // Public API
  // ---------------------------------------------------------------------------

  /// Scan paths for files containing personal information.
  ///
  /// [paths] overrides default hot paths if provided.
  /// [extensions] overrides default target extensions.
  /// [scanDepth] controls how deep content is parsed:
  ///   - 'filename': only check filenames
  ///   - 'fingerprint': filename + content fingerprint matching
  ///   - 'full': full text parsing (slowest)
  static Stream<ScanResult> scan({
    List<String>? paths,
    List<String>? extensions,
    String scanDepth = 'fingerprint',
    void Function(int scanned, int found, int skipped, String currentPath)? onProgress,
    void Function(String path)? onScanned,
    void Function(String path)? onFound,
    void Function(String path)? onSkipped,
    bool useCache = true,
    Map<String, int>? maxFileSizeByExtension,
    CancelToken? cancelToken,
  }) async* {
    final targetPaths = paths ?? await _resolveHotPaths();
    final targetExts = extensions ?? kTargetExtensions;
    final sizeLimits = maxFileSizeByExtension ?? kDefaultSizeLimits;
    var scannedCount = 0;
    var foundCount = 0;
    var skippedCount = 0;

    final cache = ScanCacheService.instance;
    if (useCache) await cache.load();

    final allPaths = <String>{};

    for (final rootPath in targetPaths) {
      if (cancelToken?.isCanceled ?? false) return;
      final files = await ScanFileLister.listFiles(
        rootPath,
        targetExts,
        maxFiles: kMaxFilesPerPath,
        cancelToken: cancelToken,
      );

      for (final file in files) {
        if (cancelToken?.isCanceled ?? false) break;
        scannedCount++;
        allPaths.add(file.path);
        onScanned?.call(file.path);

        final skipReason = _shouldSkipFile(file, sizeLimits, cache, useCache, scanDepth);
        if (skipReason != null) {
          skippedCount++;
          if (skipReason != 'cache') cache.update(file.path, file.modifiedAt, file.size);
          onSkipped?.call(file.path);
          onProgress?.call(scannedCount, foundCount, skippedCount, file.path);
          continue;
        }

        if (cancelToken?.isCanceled ?? false) break;
        final result = await _scanFile(file, scanDepth);
        cache.update(file.path, file.modifiedAt, file.size);

        if (result != null && result.sections.isNotEmpty) {
          foundCount++;
          onFound?.call(file.path);
          yield result;
        }

        onProgress?.call(scannedCount, foundCount, skippedCount, file.path);
      }
    }

    if (useCache) {
      cache.prune(allPaths);
      await cache.save();
    }
  }

  // ---------------------------------------------------------------------------
  // File-level helpers (extracted from scan() for readability)
  // ---------------------------------------------------------------------------

  /// Returns a skip reason string, or null if the file should be scanned.
  static String? _shouldSkipFile(
    ScannedFile file,
    Map<String, int> sizeLimits,
    ScanCacheService cache,
    bool useCache,
    String scanDepth,
  ) {
    final extLimitMb = sizeLimits[file.extension] ?? kDefaultMaxFileSizeMb;
    if (file.size > extLimitMb * 1024 * 1024) return 'size';
    if (useCache && !cache.isChanged(file.path, file.modifiedAt, file.size)) return 'cache';
    if (scanDepth != 'full' && !ScanSectionDetector.filenameHintsPersonal(file.name)) return 'filename';
    return null;
  }

  /// Scan a single file according to [scanDepth]. Returns null on timeout.
  static Future<ScanResult?> _scanFile(ScannedFile file, String scanDepth) async {
    try {
      // 图片文件走 OCR 识别路径
      if (kImageExtensions.contains(file.extension)) {
        return await ScanImageScanner.scanImage(file).timeout(const Duration(seconds: 20));
      }

      if (scanDepth == 'filename') {
        return _scanFilenameOnly(file);
      } else if (scanDepth == 'fingerprint') {
        return await _scanWithFingerprint(file).timeout(const Duration(seconds: 15));
      } else {
        return await _scanFull(file).timeout(const Duration(seconds: 15));
      }
    } on TimeoutException {
      return null;
    }
  }

  /// Quick scan: only check if filenames match keywords.
  static ScanResult? _scanFilenameOnly(ScannedFile file) {
    final sections = ScanSectionDetector.detectSectionsFromFilename(file.name);
    if (sections.isEmpty) return null;

    return ScanResult(
      meta: ScanMeta(
        scanId: const Uuid().v4(),
        createdAt: DateTime.now().millisecondsSinceEpoch,
        sourceFile: file.path,
        confidence: 0.5,
        fileType: file.extension,
      ),
      sections: sections,
    );
  }

  /// Fingerprint scan: parse file content and match regexes.
  static Future<ScanResult?> _scanWithFingerprint(ScannedFile file) async {
    final text = await ContentParserService.extractText(file.path);
    if (text == null || text.isEmpty) return null;

    final sections = ScanSectionDetector.detectSections(text);
    if (sections.isEmpty) return null;

    // Calculate confidence based on fingerprint hits
    final totalFields = sections.fold<int>(0, (sum, s) => sum + s.fields.length);
    final confidence = (0.5 + (totalFields * 0.1)).clamp(0.0, 1.0);

    return ScanResult(
      meta: ScanMeta(
        scanId: const Uuid().v4(),
        createdAt: DateTime.now().millisecondsSinceEpoch,
        sourceFile: file.path,
        confidence: confidence,
        fileType: file.extension,
      ),
      sections: sections,
    );
  }

  /// Full scan: parse all content and extract structured fields.
  static Future<ScanResult?> _scanFull(ScannedFile file) async {
    // Same as fingerprint for now; can be extended with NLP
    return _scanWithFingerprint(file);
  }

  // ---------------------------------------------------------------------------
  // Utilities
  // ---------------------------------------------------------------------------

  static Future<List<String>> _resolveHotPaths() async {
    final results = <String>[];
    try {
      final docs = await getApplicationDocumentsDirectory();
      results.add(docs.path);
    } on Exception catch (e) {
      SoloLog.w('LOCAL_SEARCH', 'getApplicationDocumentsDirectory failed', e);
    }

    // Fallback to expanding ~
    final home = Platform.environment['HOME'] ?? Platform.environment['USERPROFILE'];
    if (home != null) {
      for (final relative in kHotPaths) {
        final path = '$home${Platform.pathSeparator}$relative';
        if (await Directory(path).exists()) {
          results.add(path);
        }
      }
    }

    return results.toSet().toList();
  }

  /// Map a scan section to the corresponding item type ID.
  static String? mapSectionToTypeId(String sectionId) {
    const mapping = {
      'identity': 'profile_identity',
      'contact': 'profile_contact',
      'idCard': 'profile_id_card',
      'address': 'profile_address',
      'passport': 'travel_passport',
      'visa': 'travel_visa',
      'travel': 'travel_history',
      'bankAccount': 'financial_bank_account',
      'card': 'financial_card',
      'taxId': 'financial_tax_id',
      'education': 'professional_education',
      'employment': 'professional_employment',
      'skill': 'professional_skill',
      'language': 'professional_language',
      'award': 'professional_award',
    };
    return mapping[sectionId];
  }

  /// Map a scan field key to the corresponding property ID.
  static String? mapFieldToPropertyId(String sectionId, String fieldKey) {
    final sectionMap = kPropertyMapping[sectionId];
    if (sectionMap == null) return null;

    for (final entry in sectionMap.entries) {
      final patterns = entry.key.split('|');
      for (final pattern in patterns) {
        if (fieldKey.toLowerCase().contains(pattern.toLowerCase())) {
          return entry.value;
        }
      }
    }
    return null;
  }

  /// Get default sensitivity for a field based on the registry.
  /// Delegates to [FieldRegistry] as the single source of truth.
  static SensitivityLevel getDefaultSensitivity(String sectionId, String propertyId) {
    final fieldId = '$sectionId.$propertyId';
    final field = firstWhereOrNull(
      FieldRegistry.defaultFields,
      (f) => f.fieldId == fieldId,
    );
    return field?.level ?? SensitivityLevel.public;
  }
}
