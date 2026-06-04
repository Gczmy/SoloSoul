import 'dart:io';
import 'dart:typed_data';

import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/profile_data.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';

// =============================================================================
// Exceptions
// =============================================================================

class WrongPasswordException implements Exception {
  const WrongPasswordException();

  @override
  String toString() => 'WrongPasswordException';
}

// =============================================================================
// Export Package Manifest
// =============================================================================

/// 导出包的 manifest.json 内容（明文）
class ExportPackageManifest {
  final String version;
  final String exportAt;
  final int objectCount;
  final int attachmentCount;
  final String exportSalt;

  const ExportPackageManifest({
    required this.version,
    required this.exportAt,
    required this.objectCount,
    required this.attachmentCount,
    required this.exportSalt,
  });

  Map<String, dynamic> toJson() => {
    'version': version,
    'exportAt': exportAt,
    'objectCount': objectCount,
    'attachmentCount': attachmentCount,
    'export_salt': exportSalt,
  };

  factory ExportPackageManifest.fromJson(Map<String, dynamic> json) {
    return ExportPackageManifest(
      version: json['version'] as String? ?? '1.0',
      exportAt: json['exportAt'] as String? ?? '',
      objectCount: json['objectCount'] as int? ?? 0,
      attachmentCount: json['attachmentCount'] as int? ?? 0,
      exportSalt: json['export_salt'] as String? ?? '',
    );
  }
}

// =============================================================================
// Attachment Entry
// =============================================================================

/// 附件条目，用于导出/导入过程中追踪附件
class AttachmentEntry {
  final String fileId;
  final String encryptedPath;

  const AttachmentEntry({
    required this.fileId,
    required this.encryptedPath,
  });
}

// =============================================================================
// Import Preview
// =============================================================================

/// 导入预览数据
class ImportPreview {
  final ExportPackageManifest manifest;
  final Uint8List accountEncBytes;
  final String profileEncPath;
  final String attachmentsDir;
  final Directory tempDir;

  ImportPreview({
    required this.manifest,
    required this.accountEncBytes,
    required this.profileEncPath,
    required this.attachmentsDir,
    required this.tempDir,
  });
}

// =============================================================================
// Decrypted Import Data
// =============================================================================

/// Holds the decrypted profile and the export key needed for attachment decryption.
class DecryptedImportData {
  final ProfileData profile;
  final Uint8List exportKey;

  const DecryptedImportData({
    required this.profile,
    required this.exportKey,
  });
}

// =============================================================================
// Import Collection
// =============================================================================

/// 导入预览中的单个 collection（分区）
class ImportCollection {
  final String originalId;
  final String name;
  final String iconName;
  final int itemCount;
  final SensitivityLevel highestSensitivity;
  final List<UnifiedObject> items;
  final List<Attachment> attachments;
  final List<ObjectTypeDefinition> exportedCustomTypes;
  final int relationPropertyCount;
  final int crossPartitionRelationCount;
  bool selected;
  String? targetPageId;

  /// 导出时该 collection 所属的原始页面 ID
  /// 默认分区：通过 PageSectionLinkRegistry 推导
  /// 自定义分区：通过导出数据的 parentId 获取
  final String? originalParentPageId;

  /// 原始页面名称（用于显示，导出时的名称）
  final String? originalParentPageName;

  ImportCollection({
    required this.originalId,
    required this.name,
    required this.iconName,
    required this.itemCount,
    required this.highestSensitivity,
    required this.items,
    this.attachments = const [],
    this.exportedCustomTypes = const [],
    this.relationPropertyCount = 0,
    this.crossPartitionRelationCount = 0,
    this.selected = true,
    this.targetPageId,
    this.originalParentPageId,
    this.originalParentPageName,
  });
}

// =============================================================================
// Import Target Page Option
// =============================================================================

/// 导入预览对话框中的目标页面选项
class ImportTargetPageOption {
  final String pageId;
  final String name;
  final String displayName;
  final bool exists;

  const ImportTargetPageOption({
    required this.pageId,
    required this.name,
    required this.displayName,
    required this.exists,
  });
}
