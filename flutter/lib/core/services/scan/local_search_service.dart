import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:math';

import 'dart:typed_data';

import 'package:path_provider/path_provider.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/core/models/ocr_result.dart';
import 'package:solosoul_flutter/core/models/scan/scan_result_model.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/sensitivity_models.dart';
import 'package:solosoul_flutter/core/services/ocr_service.dart';
import 'package:solosoul_flutter/core/services/scan/content_parser_service.dart';
import 'package:solosoul_flutter/core/utils/mrz_parser.dart';
import 'package:solosoul_flutter/core/services/scan/windows_search_service.dart';
import 'package:solosoul_flutter/core/services/scan/scan_cache_service.dart';
import 'package:uuid/uuid.dart';

// =============================================================================
// Local Search Service
// =============================================================================

/// Simple cooperative cancellation token for long-running scan operations.
class CancelToken {
  bool _isCanceled = false;
  bool get isCanceled => _isCanceled;
  void cancel() => _isCanceled = true;
}

/// Scans local filesystem for files containing personal information.
/// Uses layered search strategy to avoid full-disk scanning.
class LocalSearchService {
  // ---------------------------------------------------------------------------
  // Configuration
  // ---------------------------------------------------------------------------

  /// Hot paths to scan by default.


  /// Target file extensions.
  static const List<String> _kTargetExtensions = [
    '.pdf', '.docx', '.xlsx', '.csv', '.json', '.txt', '.md',
    '.png', '.jpg', '.jpeg', '.webp', '.bmp', '.tiff',
  ];

  /// Image file extensions (subset of _kTargetExtensions).
  static const List<String> _kImageExtensions = [
    '.png', '.jpg', '.jpeg', '.webp', '.bmp', '.tiff',
  ];

  /// Filename keywords that suggest personal information.
  static const List<String> _kFilenameKeywords = [
    'resume', 'cv', '简历', 'passport', '护照', 'id_card', '身份证',
    'bank', '银行', 'card', '证书', 'credential', 'profile',
    'contact', 'address', 'tax', 'visa', 'education', 'employment',
  ];

  /// Content fingerprint regexes for personal information.
  static final Map<String, _Fingerprint> _kFingerprints = {
    'id_card': _Fingerprint(
      pattern: RegExp(r'(?<!\d)[1-9]\d{5}(?:19|20)\d{2}(?:0[1-9]|1[0-2])(?:0[1-9]|[12]\d|3[01])\d{3}[\dXx](?!\d)'),
      sensitivity: SensitivityLevel.critical,
    ),
    'phone': _Fingerprint(
      pattern: RegExp(r'(?<!\d)1[3-9]\d{9}(?!\d)'),
      sensitivity: SensitivityLevel.sensitive,
    ),
    'email': _Fingerprint(
      pattern: RegExp(r'[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}'),
      sensitivity: SensitivityLevel.internal,
    ),
    'passport': _Fingerprint(
      pattern: RegExp(r'(?<![A-Z0-9])[A-Z]\d{7,8}(?![A-Z0-9])'),
      sensitivity: SensitivityLevel.critical,
    ),
    'bank_card': _Fingerprint(
      pattern: RegExp(r'(?<!\d)\d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}(?!\d)'),
      sensitivity: SensitivityLevel.critical,
    ),
  };

  /// Section detection patterns: map section type to keywords in content.
  static final Map<String, List<String>> _kSectionKeywords = {
    'identity': ['姓名', '性别', '民族', '出生', '身份证', 'name', 'gender', 'nationality', 'date of birth'],
    'contact': ['电话', '手机', '邮箱', '地址', 'phone', 'email', 'address', 'contact'],
    'education': ['学校', '学历', '学位', '专业', 'university', 'college', 'degree', 'major', 'education'],
    'passport': ['护照', 'passport', '国籍', 'nationality', 'place of birth'],
    'visa': ['签证', 'visa', 'visa type', '签证类型'],
    'bankAccount': ['银行', '账户', '开户行', 'bank', 'account number', 'swift', 'sort code'],
    'card': ['信用卡', '借记卡', 'card number', 'cvv', 'expiry'],
    'employment': ['公司', '职位', '工作', 'company', 'employer', 'position', 'job title'],
  };

  /// Property mapping: section -> key patterns -> propertyId
  static final Map<String, Map<String, String>> _kPropertyMapping = {
    'identity': {
      'fullName|姓名|名字': 'fullName',
      'givenName|given': 'givenName',
      'familyName|姓|姓氏': 'familyName',
      'dateOfBirth|出生日期|birth': 'dateOfBirth',
      'gender|性别|sex': 'gender',
      'nationality|国籍|民族': 'nationality',
    },
    'passport': {
      'country|国家|签发国': 'country',
      'countryCode|代码': 'countryCode',
      'number|号码|编号|passport': 'number',
      'issueDate|签发日期|date of issue': 'issueDate',
      'placeOfIssue|签发地点': 'placeOfIssue',
      'expiryDate|有效期|date of expiry': 'expiryDate',
      'holderName|持有人|姓名': 'holderName',
      'dateOfBirth|出生日期': 'dateOfBirth',
      'placeOfBirth|出生地': 'placeOfBirth',
      'sex|性别': 'sex',
      'nationality|国籍': 'nationality',
      'authority|签发机关': 'authority',
    },
    'education': {
      'institution|学校|大学|院校|university|college|school': 'institution',
      'degree|学位|学历': 'degree',
      'field|专业|领域|major': 'field',
      'startDate|开始日期|入学': 'startDate',
      'endDate|结束日期|毕业': 'endDate',
    },
    'bankAccount': {
      'bankName|银行名称|开户行': 'bankName',
      'accountNumber|账号|账户': 'accountNumber',
      'currency|货币': 'currency',
      'swiftBic|swift|bic': 'swiftBic',
      'sortCode|sort': 'sortCode',
      'accountHolderName|持有人': 'accountHolderName',
      'routingNumber|routing': 'routingNumber',
    },
    'idCard': {
      'number|号码|编号|id': 'number',
      'holderName|持有人|姓名|name': 'holderName',
      'country|国家|签发国': 'country',
      'dateOfBirth|出生日期|birth': 'dateOfBirth',
      'sex|性别': 'sex',
      'expiryDate|有效期|date of expiry': 'expiryDate',
    },
  };

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
  static const int _kMaxFilesPerPath = 500; // 限制每个路径返回的文件数
  static const int _kDefaultMaxFileSizeMb = 10;

  /// Per-extension default size limits (MB).
  static const Map<String, int> _kDefaultSizeLimits = {
    '.pdf': 5,
    '.docx': 1,
    '.xlsx': 1,
    '.csv': 1,
    '.json': 1,
    '.txt': 1,
    '.md': 1,
    '.png': 5,
    '.jpg': 5,
    '.jpeg': 5,
    '.webp': 5,
    '.bmp': 5,
    '.tiff': 10,
  };

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
    final targetExts = extensions ?? _kTargetExtensions;
    final sizeLimits = maxFileSizeByExtension ?? _kDefaultSizeLimits;
    var scannedCount = 0;
    var foundCount = 0;
    var skippedCount = 0;

    final cache = ScanCacheService.instance;
    if (useCache) await cache.load();

    final allPaths = <String>{};

    for (final rootPath in targetPaths) {
      if (cancelToken?.isCanceled ?? false) return;
      final files = await _listFiles(rootPath, targetExts, maxFiles: _kMaxFilesPerPath, cancelToken: cancelToken);

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
    final extLimitMb = sizeLimits[file.extension] ?? _kDefaultMaxFileSizeMb;
    if (file.size > extLimitMb * 1024 * 1024) return 'size';
    if (useCache && !cache.isChanged(file.path, file.modifiedAt, file.size)) return 'cache';
    if (scanDepth != 'full' && !filenameHintsPersonal(file.name)) return 'filename';
    return null;
  }

  /// Scan a single file according to [scanDepth]. Returns null on timeout.
  static Future<ScanResult?> _scanFile(ScannedFile file, String scanDepth) async {
    try {
      // 图片文件走 OCR 识别路径
      if (_kImageExtensions.contains(file.extension)) {
        return await _scanImageFile(file).timeout(const Duration(seconds: 20));
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
    final sections = _detectSectionsFromFilename(file.name);
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

    final sections = _detectSections(text);
    if (sections.isEmpty) return null;

    // Calculate confidence based on fingerprint hits
    final totalFields = sections.fold<int>(0, (sum, s) => sum + s.fields.length);
    final confidence = min(0.5 + (totalFields * 0.1), 1.0);

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
  // Image OCR Scan
  // ---------------------------------------------------------------------------

  /// 对图片文件执行 OCR 识别，并智能判断是否为 MRZ 文档。
  ///
  /// 流程：
  /// 1. 读取图片文件为 bytes
  /// 2. 通用 OCR 识别文本
  /// 3. 尝试从 OCR 结果中提取 MRZ 候选行并解析
  /// 4. 若 MRZ 解析成功 → 生成 passport / idCard 类型的 ScanResult
  /// 5. 若 MRZ 失败 → 将 OCR 文本作为普通文本进行 section 检测
  static Future<ScanResult?> _scanImageFile(ScannedFile file) async {
    try {
      final fileHandle = File(file.path);
      if (!await fileHandle.exists()) return null;

      final bytes = await fileHandle.readAsBytes();
      if (bytes.isEmpty) return null;

      // 通用 OCR 识别
      final ocrResult = await OcrService.recognizeText(Uint8List.fromList(bytes));
      if (ocrResult.rawText.trim().isEmpty) return null;

      // 尝试 MRZ 提取（从候选行中精确筛选标准格式行）
      final mrzCandidates = OcrService.extractMrzLinesFromResult(ocrResult);
      MrzData? mrzData;
      if (mrzCandidates.isNotEmpty) {
        // TD3 护照: 2 行 × 44 字符
        final td3Lines = mrzCandidates.where((l) => l.length == 44).toList();
        if (td3Lines.length >= 2) {
          mrzData = MrzParser.parse(td3Lines.sublist(td3Lines.length - 2));
        }
        // TD1 身份证: 3 行 × 30 字符
        if (mrzData == null) {
          final td1Lines = mrzCandidates.where((l) => l.length == 30).toList();
          if (td1Lines.length >= 3) {
            mrzData = MrzParser.parse(td1Lines.sublist(td1Lines.length - 3));
          }
        }
        // TD2: 2 行 × 36 字符
        if (mrzData == null) {
          final td2Lines = mrzCandidates.where((l) => l.length == 36).toList();
          if (td2Lines.length >= 2) {
            mrzData = MrzParser.parse(td2Lines.sublist(td2Lines.length - 2));
          }
        }

        if (mrzData != null) {
          SoloLog.d('LocalSearchService',
              'MRZ detected in image: ${file.name} type=${mrzData.documentType}');
          return _buildMrzScanResult(file, mrzData);
        }
      }

      // MRZ 未识别到，将 OCR 文本作为普通文档处理
      SoloLog.d('LocalSearchService',
          'No MRZ in image: ${file.name}, falling back to text detection');
      return _buildTextScanResultFromOcr(file, ocrResult);
    } on Exception catch (e) {
      SoloLog.w('LocalSearchService', 'Image OCR failed: ${file.name}', e);
      return null;
    }
  }

  /// 从 MRZ 数据构建 ScanResult
  static ScanResult _buildMrzScanResult(ScannedFile file, MrzData mrzData) {
    final isPassport = mrzData.documentType.startsWith('P');
    final sectionId = isPassport ? 'passport' : 'idCard';
    final displayName = isPassport ? 'Passport' : 'ID Card';

    final fields = <ScanField>[
      ScanField(
        key: 'number',
        value: mrzData.documentNumber,
        sensitivity: SensitivityLevel.critical,
        confidence: mrzData.confidence,
      ),
      ScanField(
        key: 'holderName',
        value: '${mrzData.surname} ${mrzData.givenNames}'.trim(),
        sensitivity: SensitivityLevel.public,
        confidence: mrzData.confidence,
      ),
      ScanField(
        key: 'country',
        value: mrzData.country,
        sensitivity: SensitivityLevel.public,
        confidence: mrzData.confidence,
      ),
      ScanField(
        key: 'dateOfBirth',
        value: mrzData.dateOfBirth,
        sensitivity: SensitivityLevel.sensitive,
        confidence: mrzData.confidence,
      ),
      ScanField(
        key: 'sex',
        value: mrzData.sex,
        sensitivity: SensitivityLevel.public,
        confidence: mrzData.confidence,
      ),
      ScanField(
        key: 'expiryDate',
        value: mrzData.expiryDate,
        sensitivity: SensitivityLevel.sensitive,
        confidence: mrzData.confidence,
      ),
    ];

    if (isPassport) {
      fields.add(ScanField(
        key: 'nationality',
        value: mrzData.nationality,
        sensitivity: SensitivityLevel.public,
        confidence: mrzData.confidence,
      ));
    }

    return ScanResult(
      meta: ScanMeta(
        scanId: const Uuid().v4(),
        createdAt: DateTime.now().millisecondsSinceEpoch,
        sourceFile: file.path,
        confidence: mrzData.confidence,
        fileType: file.extension,
      ),
      sections: [
        ScanSection(
          section: sectionId,
          display: displayName,
          fields: fields,
        ),
      ],
    );
  }

  /// 从通用 OCR 结果构建 ScanResult（将 OCR 文本当作普通文档文本处理）
  static ScanResult? _buildTextScanResultFromOcr(
    ScannedFile file,
    OcrResult ocrResult,
  ) {
    final sections = _detectSections(ocrResult.rawText);
    if (sections.isEmpty) return null;

    final totalFields = sections.fold<int>(0, (sum, s) => sum + s.fields.length);
    final confidence = min(ocrResult.confidence + (totalFields * 0.05), 1.0);

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

  // ---------------------------------------------------------------------------
  // File Discovery
  // ---------------------------------------------------------------------------

  /// List files matching target extensions under a path.
  static Future<List<ScannedFile>> _listFiles(
    String rootPath,
    List<String> extensions, {
    int maxFiles = _kMaxFilesPerPath,
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
    int maxFiles = _kMaxFilesPerPath,
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
    int maxFiles = _kMaxFilesPerPath,
    CancelToken? cancelToken,
  }) async {
    if (cancelToken?.isCanceled ?? false) return <ScannedFile>[];
    return WindowsSearchService.searchFiles(rootPath, extensions, maxFiles: maxFiles);
  }

  /// Generic: Dart directory traversal.
  static Future<List<ScannedFile>> _listFilesGeneric(
    String rootPath,
    List<String> extensions, {
    int maxFiles = _kMaxFilesPerPath,
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

  // ---------------------------------------------------------------------------
  // Detection Logic
  // ---------------------------------------------------------------------------

  static bool filenameHintsPersonal(String filename) {
    final lower = filename.toLowerCase();
    return _kFilenameKeywords.any((kw) => lower.contains(kw.toLowerCase()));
  }

  static List<ScanSection> _detectSectionsFromFilename(String filename) {
    final lower = filename.toLowerCase();
    final sections = <ScanSection>[];

    if (lower.contains('resume') || lower.contains('cv') || lower.contains('简历')) {
      sections.add(const ScanSection(
        section: 'identity',
        display: 'Personal Information',
        fields: [],
      ));
      sections.add(const ScanSection(
        section: 'education',
        display: 'Education',
        fields: [],
      ));
    }
    if (lower.contains('passport') || lower.contains('护照')) {
      sections.add(const ScanSection(
        section: 'passport',
        display: 'Passport',
        fields: [],
      ));
    }
    if (lower.contains('bank') || lower.contains('银行')) {
      sections.add(const ScanSection(
        section: 'bankAccount',
        display: 'Bank Account',
        fields: [],
      ));
    }

    return sections;
  }

  static List<ScanSection> _detectSections(String text) {
    final sections = <String, ScanSection>{};
    final lowerText = text.toLowerCase();

    // Detect which sections are present based on keywords
    for (final entry in _kSectionKeywords.entries) {
      final sectionId = entry.key;
      final keywords = entry.value;

      final matched = keywords.any((kw) => lowerText.contains(kw.toLowerCase()));
      if (!matched) continue;

      final fields = <ScanField>[];

      // Run fingerprints for this section
      if (sectionId == 'identity') {
        fields.addAll(_extractIdentityFields(text));
      } else if (sectionId == 'passport') {
        fields.addAll(_extractPassportFields(text));
      } else if (sectionId == 'education') {
        fields.addAll(_extractEducationFields(text));
      } else if (sectionId == 'bankAccount') {
        fields.addAll(_extractBankAccountFields(text));
      } else if (sectionId == 'contact') {
        fields.addAll(_extractContactFields(text));
      } else if (sectionId == 'employment') {
        fields.addAll(_extractEmploymentFields(text));
      }

      if (fields.isNotEmpty) {
        sections[sectionId] = ScanSection(
          section: sectionId,
          display: _sectionDisplayName(sectionId),
          fields: fields,
        );
      }
    }

    return sections.values.toList();
  }

  static String _sectionDisplayName(String sectionId) {
    const names = {
      'identity': 'Personal Information',
      'contact': 'Contact',
      'education': 'Education',
      'passport': 'Passport',
      'visa': 'Visa',
      'bankAccount': 'Bank Account',
      'card': 'Card',
      'employment': 'Employment',
    };
    return names[sectionId] ?? sectionId;
  }

  // ---------------------------------------------------------------------------
  // Field Extractors
  // ---------------------------------------------------------------------------

  static List<ScanField> _extractIdentityFields(String text) {
    final fields = <ScanField>[];

    // ID Card
    final idMatch = _kFingerprints['id_card']!.pattern.firstMatch(text);
    if (idMatch != null) {
      fields.add(ScanField(
        key: 'idCard',
        value: idMatch.group(0)!,
        sensitivity: SensitivityLevel.critical,
        confidence: 0.99,
      ));
    }

    // Phone
    final phoneMatches = _kFingerprints['phone']!.pattern.allMatches(text);
    if (phoneMatches.isNotEmpty) {
      fields.add(ScanField(
        key: 'phone',
        value: phoneMatches.first.group(0)!,
        sensitivity: SensitivityLevel.sensitive,
        confidence: 0.98,
      ));
    }

    // Email
    final emailMatches = _kFingerprints['email']!.pattern.allMatches(text);
    if (emailMatches.isNotEmpty) {
      fields.add(ScanField(
        key: 'email',
        value: emailMatches.first.group(0)!,
        sensitivity: SensitivityLevel.internal,
        confidence: 0.97,
      ));
    }

    // Try to extract name (heuristic: look for patterns like "Name: Zhang San")
    final nameMatch = RegExp(r'[Nn]ame[\s:：]+([\u4e00-\u9fa5]{2,4}|[A-Z][a-z]+\s[A-Z][a-z]+)').firstMatch(text);
    if (nameMatch != null) {
      fields.add(ScanField(
        key: 'fullName',
        value: nameMatch.group(1)!,
        sensitivity: SensitivityLevel.public,
        confidence: 0.85,
      ));
    }

    return fields;
  }

  static List<ScanField> _extractPassportFields(String text) {
    final fields = <ScanField>[];

    final passportMatch = _kFingerprints['passport']!.pattern.firstMatch(text);
    if (passportMatch != null) {
      fields.add(ScanField(
        key: 'number',
        value: passportMatch.group(0)!,
        sensitivity: SensitivityLevel.critical,
        confidence: 0.95,
      ));
    }

    // Country (heuristic)
    final countryMatch = RegExp(r'[Cc]ountry[\s:：]+([A-Za-z\u4e00-\u9fa5 ]{2,30})').firstMatch(text);
    if (countryMatch != null) {
      fields.add(ScanField(
        key: 'country',
        value: countryMatch.group(1)!.trim(),
        sensitivity: SensitivityLevel.public,
        confidence: 0.80,
      ));
    }

    // Holder name
    final holderMatch = RegExp(r'[Nn]ame[\s:：]+([\u4e00-\u9fa5]{2,4}|[A-Z][a-z]+\s[A-Z][a-z]+)').firstMatch(text);
    if (holderMatch != null) {
      fields.add(ScanField(
        key: 'holderName',
        value: holderMatch.group(1)!,
        sensitivity: SensitivityLevel.sensitive,
        confidence: 0.80,
      ));
    }

    return fields;
  }

  static List<ScanField> _extractEducationFields(String text) {
    final fields = <ScanField>[];

    // Institution (heuristic)
    final instMatch = RegExp(
      r'([\u4e00-\u9fa5]{2,10}(?:大学|学院|学校)|[A-Z][a-zA-Z\s]+(?:University|College|Institute|School))',
    ).firstMatch(text);
    if (instMatch != null) {
      fields.add(ScanField(
        key: 'institution',
        value: instMatch.group(1)!,
        sensitivity: SensitivityLevel.public,
        confidence: 0.75,
      ));
    }

    // Degree
    final degreeMatch = RegExp(
      r'(Bachelor|Master|Ph\.?D|博士|硕士|学士|本科|研究生)',
      caseSensitive: false,
    ).firstMatch(text);
    if (degreeMatch != null) {
      fields.add(ScanField(
        key: 'degree',
        value: degreeMatch.group(1)!,
        sensitivity: SensitivityLevel.public,
        confidence: 0.70,
      ));
    }

    return fields;
  }

  static List<ScanField> _extractBankAccountFields(String text) {
    final fields = <ScanField>[];

    // Bank name heuristic
    final bankMatch = RegExp(
      r'([\u4e00-\u9fa5]{2,8}银行|[A-Z][a-zA-Z\s]+Bank)',
    ).firstMatch(text);
    if (bankMatch != null) {
      fields.add(ScanField(
        key: 'bankName',
        value: bankMatch.group(1)!,
        sensitivity: SensitivityLevel.sensitive,
        confidence: 0.80,
      ));
    }

    // Account number (generic 16-19 digit pattern)
    final acctMatch = RegExp(r'\b\d{16,19}\b').firstMatch(text);
    if (acctMatch != null) {
      fields.add(ScanField(
        key: 'accountNumber',
        value: acctMatch.group(0)!,
        sensitivity: SensitivityLevel.critical,
        confidence: 0.90,
      ));
    }

    // SWIFT/BIC
    final swiftMatch = RegExp(r'\b[A-Z]{6}[A-Z0-9]{2}([A-Z0-9]{3})?\b').firstMatch(text);
    if (swiftMatch != null) {
      fields.add(ScanField(
        key: 'swiftBic',
        value: swiftMatch.group(0)!,
        sensitivity: SensitivityLevel.critical,
        confidence: 0.85,
      ));
    }

    return fields;
  }

  static List<ScanField> _extractContactFields(String text) {
    final fields = <ScanField>[];

    final phoneMatches = _kFingerprints['phone']!.pattern.allMatches(text);
    if (phoneMatches.isNotEmpty) {
      fields.add(ScanField(
        key: 'value',
        value: phoneMatches.first.group(0)!,
        sensitivity: SensitivityLevel.internal,
        confidence: 0.95,
      ));
    }

    final emailMatches = _kFingerprints['email']!.pattern.allMatches(text);
    if (emailMatches.isNotEmpty) {
      fields.add(ScanField(
        key: 'value',
        value: emailMatches.first.group(0)!,
        sensitivity: SensitivityLevel.internal,
        confidence: 0.95,
      ));
    }

    return fields;
  }

  static List<ScanField> _extractEmploymentFields(String text) {
    final fields = <ScanField>[];

    final companyMatch = RegExp(
      r'([\u4e00-\u9fa5]{2,20}(?:公司|集团|企业)|[A-Z][a-zA-Z0-9\s&]+(?:Inc|Ltd|LLC|Corp|Company))',
    ).firstMatch(text);
    if (companyMatch != null) {
      fields.add(ScanField(
        key: 'company',
        value: companyMatch.group(1)!,
        sensitivity: SensitivityLevel.public,
        confidence: 0.70,
      ));
    }

    return fields;
  }

  // ---------------------------------------------------------------------------
  // Utilities
  // ---------------------------------------------------------------------------

  static String _extension(String path) {
    final idx = path.lastIndexOf('.');
    return idx >= 0 ? path.substring(idx).toLowerCase() : '';
  }

  static const List<String> _kHotPaths = ['Documents', 'Desktop', 'Downloads'];

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
      for (final relative in _kHotPaths) {
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
    final sectionMap = _kPropertyMapping[sectionId];
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

/// Internal class wrapping a regex fingerprint.
class _Fingerprint {
  final RegExp pattern;
  final SensitivityLevel sensitivity;

  _Fingerprint({required this.pattern, required this.sensitivity});
}
