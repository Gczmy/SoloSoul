import 'package:solosoul_flutter/core/models/scan/scan_result_model.dart';
import 'package:solosoul_flutter/core/services/scan/scan_import_service.dart';

// =============================================================================
// Local Search State
// =============================================================================

class LocalSearchState {
  final bool isScanning;
  final bool wasCanceled;
  final int scanProgress;
  final int scannedCount;
  final int foundCount;
  final String currentPath;
  final String? scanError;

  final List<String> paths;
  final List<String> extensions;
  final String scanDepth;
  final Map<String, int> maxFileSizeByExtension; // 各文件类型大小限制（MB）

  final List<ScanResult> scanResults;
  final List<String> scannedFiles;     // 已扫描的文件路径
  final List<String> foundFiles;       // 命中个人信息的文件路径
  final List<String> skippedFiles;     // 被跳过的文件路径（大小超限/缓存未变更/不匹配）
  final List<ImportCandidate> importCandidates;
  final List<ImportConflict> importConflicts;
  final ScanImportResult? importResult;

  // AI 字段映射状态
  final AiMappingStatus aiMappingStatus;
  final String? aiMappingError;

  const LocalSearchState({
    this.isScanning = false,
    this.wasCanceled = false,
    this.scanProgress = 0,
    this.scannedCount = 0,
    this.foundCount = 0,
    this.currentPath = '',
    this.scanError,
    this.paths = const [],
    this.extensions = const [
      '.pdf', '.docx', '.xlsx', '.csv', '.json', '.txt', '.md',
      '.png', '.jpg', '.jpeg', '.webp', '.bmp', '.tiff',
    ],
    this.scanDepth = 'fingerprint',
    this.maxFileSizeByExtension = const {
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
    },
    this.scanResults = const [],
    this.scannedFiles = const [],
    this.foundFiles = const [],
    this.skippedFiles = const [],
    this.importCandidates = const [],
    this.importConflicts = const [],
    this.importResult,
    this.aiMappingStatus = AiMappingStatus.idle,
    this.aiMappingError,
  });

  LocalSearchState copyWith({
    bool? isScanning,
    bool? wasCanceled,
    int? scanProgress,
    int? scannedCount,
    int? foundCount,
    String? currentPath,
    String? scanError,
    List<String>? paths,
    List<String>? extensions,
    String? scanDepth,
    Map<String, int>? maxFileSizeByExtension,
    List<ScanResult>? scanResults,
    List<String>? scannedFiles,
    List<String>? foundFiles,
    List<String>? skippedFiles,
    List<ImportCandidate>? importCandidates,
    List<ImportConflict>? importConflicts,
    ScanImportResult? importResult,
    AiMappingStatus? aiMappingStatus,
    String? aiMappingError,
  }) {
    return LocalSearchState(
      isScanning: isScanning ?? this.isScanning,
      wasCanceled: wasCanceled ?? this.wasCanceled,
      scanProgress: scanProgress ?? this.scanProgress,
      scannedCount: scannedCount ?? this.scannedCount,
      foundCount: foundCount ?? this.foundCount,
      currentPath: currentPath ?? this.currentPath,
      scanError: scanError == null ? this.scanError : (scanError.isEmpty ? null : scanError),
      paths: paths ?? this.paths,
      extensions: extensions ?? this.extensions,
      scanDepth: scanDepth ?? this.scanDepth,
      maxFileSizeByExtension: maxFileSizeByExtension ?? this.maxFileSizeByExtension,
      scanResults: scanResults ?? this.scanResults,
      scannedFiles: scannedFiles ?? this.scannedFiles,
      foundFiles: foundFiles ?? this.foundFiles,
      skippedFiles: skippedFiles ?? this.skippedFiles,
      importCandidates: importCandidates ?? this.importCandidates,
      importConflicts: importConflicts ?? this.importConflicts,
      importResult: importResult ?? this.importResult,
      aiMappingStatus: aiMappingStatus ?? this.aiMappingStatus,
      aiMappingError: aiMappingError == null
          ? this.aiMappingError
          : (aiMappingError.isEmpty ? null : aiMappingError),
    );
  }
}

// =============================================================================
// AI Mapping Status
// =============================================================================

enum AiMappingStatus {
  idle,
  loading,
  success,
  error,
}

extension AiMappingStatusExtension on AiMappingStatus {
  bool get isIdle => this == AiMappingStatus.idle;
  bool get isLoading => this == AiMappingStatus.loading;
  bool get isSuccess => this == AiMappingStatus.success;
  bool get isError => this == AiMappingStatus.error;
}
