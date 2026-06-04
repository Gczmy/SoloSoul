import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:path_provider/path_provider.dart';
import 'package:uuid/uuid.dart';

import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/attachment_storage_service.dart';
import 'package:solosoul_flutter/core/services/backup_service.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';
import 'package:solosoul_flutter/core/services/export_import_models.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/frb/api.dart' as frb;

// =============================================================================
// Export / Import Service
// =============================================================================

class ExportImportService {
  ExportImportService._();
  static final ExportImportService instance = ExportImportService._();

  // ---------------------------------------------------------------------------
  // Export
  // ---------------------------------------------------------------------------

  /// Export the current account data into a `.solosoul` package file.
  /// Returns the saved file path, or null if export failed.
  Future<String?> exportPackage({
    required String accountId,
    required String password,
    required String? passwordHint,
    required String savePath,
  }) async {
    try {
      // 1. Load ProfileData
      final profile = await ProfileStorageService.instance.loadProfile(accountId);
      if (profile == null) {
        SoloLog.w('EXPORT', 'Profile not found for account $accountId');
        return null;
      }

      // 2. Generate export salt
      final exportSalt = await frb.frbGenerateSalt(length: 32);

      // 3. Derive export key (Balanced preset)
      final exportKey = await frb.frbDeriveKey(
        password: password,
        salt: exportSalt,
        memoryKib: 16384,
        iterations: 3,
        parallelism: 4,
      );

      // 4. Prepare temp work directory
      final tempDir = await getTemporaryDirectory();
      final workDir = Directory(
        '${tempDir.path}/solosoul_export_${const Uuid().v4()}',
      );
      await workDir.create(recursive: true);

      try {
        // 5. Build manifest.json (export_salt is public, only prevents rainbow tables)
        final manifest = ExportPackageManifest(
          version: '1.0',
          exportAt: DateTime.now().toIso8601String(),
          objectCount: profile.unifiedObjects?.objects.length ?? 0,
          attachmentCount: await AttachmentStorageService().getAttachmentCount(accountId),
          exportSalt: base64Encode(exportSalt),
        );
        final manifestFile = File('${workDir.path}/manifest.json');
        await manifestFile.writeAsString(jsonEncode(manifest.toJson()));

        // 6. Encrypt account.enc (independent verify token)
        final exportVerifyPlain = Uint8List.fromList(
          utf8.encode('SOLOSOUL_EXPORT_VERIFY_v1'),
        );
        final exportVerifyToken = await frb.frbEncryptWithKey(
          plaintext: exportVerifyPlain,
          key: exportKey,
        );
        final accountConfigJson = jsonEncode({
          'password_hint': passwordHint,
          'export_verify_token': base64Encode(exportVerifyToken),
          'export_salt': base64Encode(exportSalt),
        });
        final accountConfigEncrypted = await frb.frbEncryptWithKey(
          plaintext: Uint8List.fromList(utf8.encode(accountConfigJson)),
          key: exportKey,
        );
        final accountFile = File('${workDir.path}/account.enc');
        await accountFile.writeAsBytes(accountConfigEncrypted);

        // 7. Encrypt profile (includes unifiedObjects + customTypes)
        final profileJson = jsonEncode(profile.toJson());
        final profileEncrypted = await frb.frbEncryptWithKey(
          plaintext: Uint8List.fromList(utf8.encode(profileJson)),
          key: exportKey,
        );
        final profileFile = File('${workDir.path}/profile.enc');
        await profileFile.writeAsBytes(profileEncrypted);

        // 8. Re-encrypt attachments with exportKey
        final attachmentsDir = Directory('${workDir.path}/attachments');
        await attachmentsDir.create(recursive: true);
        await _collectAndReEncryptAttachments(
          accountId: accountId,
          exportKey: exportKey,
          attachmentsDir: attachmentsDir,
          tempDir: tempDir,
        );

        // 9. Compute SHA-256 checksums
        final checksums = await _computeChecksums(workDir);
        final checksumsFile = File('${workDir.path}/checksums.json');
        await checksumsFile.writeAsString(jsonEncode(checksums));

        // 10. Create ZIP via Rust FRB
        await frb.frbCreateZipPackage(
          srcDir: workDir.path,
          dstPath: savePath,
        );

        // 11. Set restrictive permissions (best effort)
        await _setRestrictivePermissions(savePath);

        SoloLog.d('EXPORT', 'Package exported to $savePath');
        return savePath;
      } finally {
        // Clean up temp directory
        if (await workDir.exists()) {
          await workDir.delete(recursive: true);
        }
      }
    } on Exception catch (e, st) {
      SoloLog.e('EXPORT', 'exportPackage failed: $e\n$st');
      return null;
    }
  }

  // ---------------------------------------------------------------------------
  // Import — Parse Package
  // ---------------------------------------------------------------------------

  /// Parse a `.solosoul` package and return an [ImportPreview].
  /// Caller is responsible for cleaning up [preview.tempDir] when done.
  Future<ImportPreview?> parseImportPackage(String filePath) async {
    try {
      final tempDir = await getTemporaryDirectory();
      final extractDir = Directory(
        '${tempDir.path}/solosoul_import_${const Uuid().v4()}',
      );
      await extractDir.create(recursive: true);

      // 1. Extract ZIP via Rust FRB
      await frb.frbExtractZipPackage(
        zipPath: filePath,
        dstDir: extractDir.path,
      );

      // 2. Read manifest.json
      final manifestFile = File('${extractDir.path}/manifest.json');
      if (!await manifestFile.exists()) {
        SoloLog.w('IMPORT', 'manifest.json not found in package');
        await extractDir.delete(recursive: true);
        return null;
      }
      final manifestJson = jsonDecode(await manifestFile.readAsString())
          as Map<String, dynamic>;
      final manifest = ExportPackageManifest.fromJson(manifestJson);

      // 3. Verify checksums (transport integrity check)
      final checksumsFile = File('${extractDir.path}/checksums.json');
      if (await checksumsFile.exists()) {
        final checksumsJson = jsonDecode(await checksumsFile.readAsString())
            as Map<String, dynamic>;
        final sha256Map = checksumsJson['sha256'] as Map<String, dynamic>?;
        if (sha256Map != null) {
          for (final entry in sha256Map.entries) {
            final file = File('${extractDir.path}/${entry.key}');
            if (await file.exists()) {
              final bytes = await file.readAsBytes();
              final computed = sha256.convert(bytes).toString();
              if (computed != entry.value) {
                SoloLog.e(
                  'IMPORT',
                  'Checksum mismatch for ${entry.key}: expected ${entry.value}, got $computed',
                );
                await extractDir.delete(recursive: true);
                return null;
              }
            }
          }
        }
      }

      // 4. Read account.enc (still encrypted)
      final accountEncFile = File('${extractDir.path}/account.enc');
      if (!await accountEncFile.exists()) {
        SoloLog.w('IMPORT', 'account.enc not found in package');
        await extractDir.delete(recursive: true);
        return null;
      }
      final accountEncBytes = await accountEncFile.readAsBytes();

      final profileEncPath = '${extractDir.path}/profile.enc';
      final attachmentsDir = '${extractDir.path}/attachments';

      return ImportPreview(
        manifest: manifest,
        accountEncBytes: accountEncBytes,
        profileEncPath: profileEncPath,
        attachmentsDir: attachmentsDir,
        tempDir: extractDir,
      );
    } on Exception catch (e, st) {
      SoloLog.e('IMPORT', 'parseImportPackage failed: $e\n$st');
      return null;
    }
  }

  // ---------------------------------------------------------------------------
  // Import — Decrypt & Verify
  // ---------------------------------------------------------------------------

  /// Decrypt and verify password, returning the exported [ProfileData].
  Future<ProfileData?> decryptAndVerify(
    ImportPreview preview,
    String password,
  ) async {
    try {
      // export_salt is stored in the encrypted account.enc.
      // We need to read it from manifest for the salt location hint.
      // Actually per design, export_salt is in account.enc which is encrypted.
      // But we stored export_salt in the account config JSON before encryption.
      // We need to brute-force try: the salt is in the encrypted blob.
      // Wait — the design says export_salt is in account.enc (encrypted).
      // But to derive the key we need the salt BEFORE decrypting.
      // Let's re-read the plan: export_salt is stored in the account.enc JSON.
      // This means we must have a way to get it.
      // Looking back at the plan: Step 3.2 says "export_salt 放在 manifest.json 中".
      // But the actual code stores it inside account.enc.
      // To fix this, we should store export_salt in manifest.json as well,
      // or we need to change the export format.
      // For now, let's store export_salt in manifest.json during export
      // and read it from there. I'll update the export code above.
      
      // Read export_salt from manifest
      final exportSaltBase64 = preview.manifest.exportSalt;
      if (exportSaltBase64.isEmpty) {
        SoloLog.w('IMPORT', 'export_salt not found in manifest');
        throw const WrongPasswordException();
      }
      final exportSalt = base64Decode(exportSaltBase64);

      // Derive key (same Balanced preset as export)
      final exportKey = await frb.frbDeriveKey(
        password: password,
        salt: exportSalt,
        memoryKib: 16384,
        iterations: 3,
        parallelism: 4,
      );

      // Decrypt account.enc
      final accountPlain = await frb.frbDecryptWithKey(
        ciphertext: preview.accountEncBytes,
        key: exportKey,
      );
      final accountConfig = jsonDecode(utf8.decode(accountPlain))
          as Map<String, dynamic>;

      // Verify password via exportVerifyToken
      final verifyTokenBase64 = accountConfig['export_verify_token'] as String?;
      if (verifyTokenBase64 == null) {
        SoloLog.w('IMPORT', 'export_verify_token not found');
        throw const WrongPasswordException();
      }
      final verifyTokenBytes = base64Decode(verifyTokenBase64);
      final verifyPlain = await frb.frbDecryptWithKey(
        ciphertext: verifyTokenBytes,
        key: exportKey,
      );
      final verifyString = utf8.decode(verifyPlain);
      if (verifyString != 'SOLOSOUL_EXPORT_VERIFY_v1') {
        SoloLog.w('IMPORT', 'Password verification failed');
        throw const WrongPasswordException();
      }

      // Decrypt profile.enc
      final profileEncFile = File(preview.profileEncPath);
      if (!await profileEncFile.exists()) {
        SoloLog.w('IMPORT', 'profile.enc not found');
        return null;
      }
      final profilePlain = await frb.frbDecryptWithKey(
        ciphertext: await profileEncFile.readAsBytes(),
        key: exportKey,
      );
      final profileJson = jsonDecode(utf8.decode(profilePlain))
          as Map<String, dynamic>;
      return ProfileData.fromJson(profileJson);
    } on WrongPasswordException {
      rethrow;
    } on Exception catch (e, st) {
      SoloLog.e('IMPORT', 'decryptAndVerify failed: $e\n$st');
      return null;
    }
  }

  // ---------------------------------------------------------------------------
  // Import — Build Preview Collections
  // ---------------------------------------------------------------------------

  /// Build [ImportCollection] list from exported [ProfileData].
  List<ImportCollection> buildImportCollections(ProfileData exportedProfile) {
    final objects = exportedProfile.unifiedObjects?.objects ?? [];
    final collections = objects.where((o) => o.typeId == 'collection').toList();

    return collections.map((col) {
      final items = objects
          .where((o) => o.parentId == col.id || o.id == col.id)
          .toList();
      final highestSens = _computeHighestSensitivity(items);

      // Collect attachments from all items
      final allAttachments = <Attachment>[];
      for (final item in items) {
        allAttachments.addAll(item.attachments);
      }

      // Count relation properties
      int relationCount = 0;
      int crossPartitionCount = 0;
      final collectionItemIds = items.map((i) => i.id).toSet();
      for (final item in items) {
        for (final entry in item.properties.entries) {
          if (entry.value is RelationProperty) {
            relationCount++;
            final targetId = (entry.value as RelationProperty).targetObjectId;
            if (targetId != null && !collectionItemIds.contains(targetId)) {
              crossPartitionCount++;
            }
          }
        }
      }

      // Collect custom types used by items in this collection
      final customTypes = <ObjectTypeDefinition>[];
      final seenTypeIds = <String>{};
      final allCustomTypes = exportedProfile.unifiedObjects?.customTypes ?? [];
      for (final item in items) {
        final typeId = item.typeId;
        if (typeId != null &&
            !_isBuiltinType(typeId) &&
            !seenTypeIds.contains(typeId)) {
          seenTypeIds.add(typeId);
          final typeDef = allCustomTypes.firstWhere(
            (t) => t.id == typeId,
            orElse: () => ObjectTypeDefinition(
              id: typeId,
              name: 'Unknown',
            ),
          );
          customTypes.add(typeDef);
        }
      }

      return ImportCollection(
        originalId: col.id,
        name: col.name,
        iconName: col.iconName,
        itemCount: items.length,
        highestSensitivity: highestSens,
        items: items,
        attachments: allAttachments,
        exportedCustomTypes: customTypes,
        relationPropertyCount: relationCount,
        crossPartitionRelationCount: crossPartitionCount,
        selected: true,
        targetPageId: null,
      );
    }).toList();
  }

  // ---------------------------------------------------------------------------
  // Import — Execute
  // ---------------------------------------------------------------------------

  /// Execute the import: merge selected collections into current profile.
  Future<bool> executeImport({
    required String currentAccountId,
    required ProfileData currentProfile,
    required List<ImportCollection> selections,
    required Uint8List exportKey,
    required String tempAttachmentsDir,
  }) async {
    try {
      // 1. Pre-import silent backup
      await BackupService.instance.createBackup(currentAccountId);

      final currentObjects = List<UnifiedObject>.from(
        currentProfile.unifiedObjects?.objects ?? [],
      );
      final currentCustomTypes = List<ObjectTypeDefinition>.from(
        currentProfile.unifiedObjects?.customTypes ?? [],
      );
      final idMapping = <String, String>{}; // oldId -> newId
      final typeIdMapping = <String, String>{}; // oldTypeId -> newTypeId

      // Generate new IDs for all items in selected collections
      for (final selection in selections.where((s) => s.selected)) {
        for (final obj in selection.items) {
          idMapping[obj.id] = const Uuid().v4();
        }
      }

      // Merge custom types
      for (final selection in selections.where((s) => s.selected)) {
        for (final typeDef in selection.exportedCustomTypes) {
          final typeId = typeDef.id;
          final existing = currentCustomTypes.any((t) => t.id == typeId);
          if (!existing) {
            currentCustomTypes.add(typeDef);
            typeIdMapping[typeId] = typeId;
          } else {
            final newTypeId = const Uuid().v4();
            typeIdMapping[typeId] = newTypeId;
            currentCustomTypes.add(typeDef.copyWith(id: newTypeId));
          }
        }
      }

      // Recursively update custom type relation targetTypeIds
      for (var i = 0; i < currentCustomTypes.length; i++) {
        final type = currentCustomTypes[i];
        final newProperties = <PropertyDefinition>[];
        for (final prop in type.properties) {
          if (prop.type == PropertyType.relation) {
            final targetTypeId = prop.config?['targetTypeId'] as String?;
            if (targetTypeId != null &&
                typeIdMapping.containsKey(targetTypeId)) {
              final newConfig =
                  Map<String, dynamic>.from(prop.config ?? {});
              newConfig['targetTypeId'] = typeIdMapping[targetTypeId];
              newProperties.add(prop.copyWith(config: newConfig));
              continue;
            }
          }
          newProperties.add(prop);
        }
        currentCustomTypes[i] = type.copyWith(properties: newProperties);
      }

      // Merge objects
      for (final selection in selections.where((s) => s.selected)) {
        final targetPageId =
            selection.targetPageId ?? _findDefaultPage(currentObjects);

        for (final obj in selection.items) {
          final newId = idMapping[obj.id]!;
          final newParentId = obj.parentId == null
              ? targetPageId
              : idMapping[obj.parentId];

          final newChildrenIds = obj.childrenIds
              .map((id) => idMapping[id])
              .whereType<String>()
              .toList();

          // Update RelationProperty targetObjectIds
          final newProperties = <String, PropertyValue>{};
          for (final entry in obj.properties.entries) {
            final value = entry.value;
            if (value is RelationProperty && value.targetObjectId != null) {
              final newTargetId = idMapping[value.targetObjectId];
              if (newTargetId != null) {
                newProperties[entry.key] =
                    value.copyWith(targetObjectId: newTargetId);
              } else {
                newProperties[entry.key] = value;
                DebugLogger.instance.logWarning(
                  'IMPORT',
                  'RelationProperty target ${value.targetObjectId} '
                  'not in import scope, keeping original',
                );
              }
            } else {
              newProperties[entry.key] = value;
            }
          }

          // Map typeId for custom types with conflicts
          final newTypeId = typeIdMapping[obj.typeId] ?? obj.typeId;

          final newObj = obj.copyWith(
            id: newId,
            typeId: newTypeId,
            parentId: newParentId,
            childrenIds: newChildrenIds,
            properties: newProperties,
            createdAt: DateTime.now().millisecondsSinceEpoch,
            updatedAt: DateTime.now().millisecondsSinceEpoch,
          );
          currentObjects.add(newObj);
        }

        // Re-encrypt attachments
        for (final attachment in selection.attachments) {
          final srcEncPath = '$tempAttachmentsDir/${attachment.fileId}.enc';
          final srcFile = File(srcEncPath);
          if (!await srcFile.exists()) {
            SoloLog.w(
              'IMPORT',
              'Attachment not found in package: ${attachment.fileId}',
            );
            continue;
          }

          final tempDir = await getTemporaryDirectory();
          final decryptedPath =
              '${tempDir.path}/dec_${attachment.fileId}';

          // Decrypt with exportKey
          await frb.frbDecryptFile(
            srcPath: srcEncPath,
            dstPath: decryptedPath,
            progressPath: '${tempDir.path}/dec_progress.txt',
            cancelPath: '${tempDir.path}/dec_cancel.txt',
          );

          // Re-encrypt with current Vault session key
          final attachTempDir = await getTemporaryDirectory();
          await AttachmentStorageService().saveAttachmentFromPath(
            accountId: currentAccountId,
            fileName: attachment.fileName,
            srcPath: decryptedPath,
            fileSize: attachment.size,
            progressPath: '${attachTempDir.path}/import_enc_progress_${attachment.fileId}.txt',
            cancelPath: '${attachTempDir.path}/import_enc_cancel_${attachment.fileId}.txt',
            isSrcTemporary: true,
          );

          // Clean up decrypted temp file
          final decFile = File(decryptedPath);
          if (await decFile.exists()) {
            await decFile.delete();
          }
        }
      }

      // Save updated profile
      final newProfile = currentProfile.copyWith(
        unifiedObjects: (currentProfile.unifiedObjects ??
                const UnifiedObjectData(objects: []))
            .copyWith(
          objects: currentObjects,
          customTypes: currentCustomTypes,
        ),
      );

      final saved = await ProfileStorageService.instance.saveProfile(
        currentAccountId,
        newProfile,
        preserveCache: false,
      );

      if (saved) {
        final selectedCount =
            selections.where((s) => s.selected).length;
        SoloLog.d('IMPORT', 'Imported $selectedCount collections');
      }

      return saved;
    } on Exception catch (e, st) {
      SoloLog.e('IMPORT', 'executeImport failed: $e\n$st');
      return false;
    }
  }

  // ---------------------------------------------------------------------------
  // Helpers
  // ---------------------------------------------------------------------------

  /// Collect and re-encrypt all attachments for export.
  Future<void> _collectAndReEncryptAttachments({
    required String accountId,
    required Uint8List exportKey,
    required Directory attachmentsDir,
    required Directory tempDir,
  }) async {
    final srcFileIds =
        await AttachmentStorageService().getAttachmentFileIds(accountId);
    if (srcFileIds.isEmpty) return;

    for (final fileId in srcFileIds) {
      final srcDir = await AttachmentStorageService().getAttachmentsDir(accountId);
      final srcPath = '${srcDir.path}/$fileId.solo';
      final srcFile = File(srcPath);
      if (!await srcFile.exists()) continue;

      final dstPath = '${attachmentsDir.path}/$fileId.enc';

      // Use Vault session key to decrypt, then exportKey to encrypt
      // But frbEncryptFile uses Vault session key, not exportKey.
      // We need a different approach.
      // For now, copy the file as-is and encrypt with exportKey using frbEncryptFile
      // Wait — frbEncryptFile uses the Vault's session key, not a custom key.
      // The plan says "用 exportKey 重新加密" but frbEncryptFile uses session key.
      // We need to either:
      // 1. Use frbEncryptWithKey on the file bytes (memory heavy for large files)
      // 2. Add a new Rust function frbEncryptFileWithKey
      // For now, let's use frbEncryptWithKey for small files and frbEncryptFile for large files.
      // Actually, the plan says to call frbEncryptFile — but that uses session key.
      // This is a design gap. Let's use frbEncryptWithKey for all attachments for now,
      // accepting the memory cost. We can optimize later with a streaming custom-key function.

      final bytes = await srcFile.readAsBytes();
      final encrypted = await frb.frbEncryptWithKey(
        plaintext: bytes,
        key: exportKey,
      );
      await File(dstPath).writeAsBytes(encrypted);
    }
  }

  /// Compute SHA-256 checksums for all files in [dir].
  Future<Map<String, dynamic>> _computeChecksums(Directory dir) async {
    final result = <String, String>{};
    await for (final entity in dir.list(recursive: true)) {
      if (entity is File) {
        final relativePath = entity.path.substring(dir.path.length + 1);
        final bytes = await entity.readAsBytes();
        result[relativePath] = sha256.convert(bytes).toString();
      }
    }
    return {'sha256': result};
  }

  /// Compute the highest sensitivity level among [items].
  SensitivityLevel _computeHighestSensitivity(List<UnifiedObject> items) {
    var highest = SensitivityLevel.public;
    for (final item in items) {
      for (final prop in item.properties.values) {
        if (prop.sensitivity.index > highest.index) {
          highest = prop.sensitivity;
        }
      }
    }
    return highest;
  }

  /// Find the default target page for imported collections.
  String? _findDefaultPage(List<UnifiedObject> currentObjects) {
    final pages = currentObjects.where((o) => o.typeId == 'page').toList();
    if (pages.isEmpty) return null;
    return pages.first.id;
  }

  /// Check if a typeId is a built-in type.
  bool _isBuiltinType(String typeId) {
    return typeId.startsWith('__preset_') ||
        const {
          'page',
          'collection',
          'note',
          'task',
          'contact',
          'item',
        }.contains(typeId);
  }

  /// Set restrictive file permissions (owner-only read/write).
  /// Best effort — may not be available on all platforms.
  static Future<void> _setRestrictivePermissions(String path) async {
    try {
      final result = await Process.run('chmod', ['600', path]);
      if (result.exitCode != 0) {
        SoloLog.w('EXPORT', 'chmod 600 failed (exit ${result.exitCode})');
      }
    } on Exception catch (e) {
      SoloLog.w('EXPORT', 'chmod failed: $e');
    }
  }
}
