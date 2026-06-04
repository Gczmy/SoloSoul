import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:path_provider/path_provider.dart';
import 'package:collection/collection.dart';
import 'package:uuid/uuid.dart';

import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/attachment_storage_service.dart';
import 'package:solosoul_flutter/core/services/backup_service.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';
import 'package:solosoul_flutter/core/services/export_import_models.dart';
import 'package:solosoul_flutter/core/services/page_section_link_registry.dart';
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

  /// Decrypt and verify password, returning the exported [DecryptedImportData].
  Future<DecryptedImportData?> decryptAndVerify(
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
      final profile = ProfileData.fromJson(profileJson);
      return DecryptedImportData(profile: profile, exportKey: exportKey);
    } on WrongPasswordException {
      rethrow;
      // ignore: avoid_catches_without_on_clauses
    } catch (e, st) {
      // NOTE: FRB throws plain String on Rust Err(String), not Exception.
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

      // Derive original parent page info
      final originalPageInfo = _deriveOriginalParentPage(col, objects);

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
        targetPageId: originalPageInfo.pageId,
        originalParentPageId: originalPageInfo.pageId,
        originalParentPageName: originalPageInfo.pageName,
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
        // Determine target page
        final originalPageId = selection.targetPageId ??
            selection.originalParentPageId ??
            _findDefaultPage(currentObjects);
        if (originalPageId == null) continue;

        // Ensure target page exists (create if missing, deduplicate via idMapping)
        final actualPageId = _ensurePageExists(
          originalPageId: originalPageId,
          originalPageName: selection.originalParentPageName,
          currentObjects: currentObjects,
          idMapping: idMapping,
        );

        // Find or create matching section in target page
        final targetSectionId = _findOrCreateMatchingSection(
          currentObjects: currentObjects,
          pageId: actualPageId,
          importCollection: selection,
          idMapping: idMapping,
        );

        // Process items
        for (final obj in selection.items) {
          // Skip collection root object — it's represented by targetSectionId
          if (obj.id == selection.originalId) continue;

          // Ensure object has a new ID
          if (!idMapping.containsKey(obj.id)) {
            idMapping[obj.id] = const Uuid().v4();
          }
          final newId = idMapping[obj.id]!;

          // Skip if already added (handles nested collections appearing in
          // multiple parent collections)
          final alreadyAdded = currentObjects.any((o) => o.id == newId);
          if (alreadyAdded) continue;

          // Determine new parent:
          // - Direct child of this collection → targetSectionId
          // - Child of another imported object → mapped ID
          String newParentId;
          if (obj.parentId == null || obj.parentId == selection.originalId) {
            newParentId = targetSectionId;
          } else {
            newParentId = idMapping[obj.parentId] ?? targetSectionId;
          }

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

          // Update parent's childrenIds
          _updateParentChildrenIds(currentObjects, newParentId, newId);
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

          // Decrypt with exportKey (attachments were encrypted with frbEncryptWithKey)
          final encBytes = await srcFile.readAsBytes();
          final decBytes = await frb.frbDecryptWithKey(
            ciphertext: encBytes,
            key: exportKey,
          );
          await File(decryptedPath).writeAsBytes(decBytes);

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

  /// Derive the original parent page ID and name for a collection.
  /// For preset sections, uses [PageSectionLinkRegistry].
  /// For custom sections, walks up the parent chain to find the page.
  ({String? pageId, String? pageName}) _deriveOriginalParentPage(
    UnifiedObject col,
    List<UnifiedObject> allObjects,
  ) {
    // 1. Try preset section → default page mapping
    final defaultPageId = PageSectionLinkRegistry.getDefaultPageIdForSection(col.id);
    if (defaultPageId != null) {
      final page = allObjects.firstWhereOrNull(
        (o) => o.id == defaultPageId && o.typeId == 'page',
      );
      return (pageId: defaultPageId, pageName: page?.name);
    }

    // 2. Walk up parent chain for custom sections
    String? currentId = col.parentId;
    final visited = <String>{};
    while (currentId != null && !visited.contains(currentId)) {
      visited.add(currentId);
      final parent = allObjects.firstWhereOrNull((o) => o.id == currentId);
      if (parent == null) break;
      if (parent.typeId == 'page') {
        return (pageId: parent.id, pageName: parent.name);
      }
      currentId = parent.parentId;
    }

    return (pageId: null, pageName: null);
  }

  /// Find the default target page for imported collections.
  String? _findDefaultPage(List<UnifiedObject> currentObjects) {
    final pages = currentObjects.where((o) => o.typeId == 'page').toList();
    if (pages.isEmpty) return null;
    return pages.first.id;
  }

  /// Ensure the target page exists. If missing, create it.
  /// Uses [idMapping] to deduplicate when multiple collections
  /// reference the same original page.
  String _ensurePageExists({
    required String originalPageId,
    required String? originalPageName,
    required List<UnifiedObject> currentObjects,
    required Map<String, String> idMapping,
  }) {
    // If already mapped (another collection created this page), return the new ID
    if (idMapping.containsKey(originalPageId)) {
      return idMapping[originalPageId]!;
    }

    // Check if page already exists in current objects
    final existingPage = currentObjects.firstWhereOrNull(
      (o) => o.id == originalPageId && o.typeId == 'page',
    );
    if (existingPage != null) {
      idMapping[originalPageId] = originalPageId;
      return originalPageId;
    }

    // Create new page
    final newPageId = const Uuid().v4();
    final now = DateTime.now().millisecondsSinceEpoch;
    final newPage = UnifiedObject(
      id: newPageId,
      typeId: 'page',
      name: originalPageName ?? 'Imported Page',
      iconName: 'article',
      parentId: null,
      childrenIds: [],
      createdAt: now,
      updatedAt: now,
    );
    currentObjects.add(newPage);
    idMapping[originalPageId] = newPageId;
    return newPageId;
  }

  /// Find an existing section in [pageId] matching [importCollection] by name,
  /// or create a new section. Updates [idMapping] for the collection's originalId.
  String _findOrCreateMatchingSection({
    required List<UnifiedObject> currentObjects,
    required String pageId,
    required ImportCollection importCollection,
    required Map<String, String> idMapping,
  }) {
    // Get existing sections under the target page
    final page = currentObjects.firstWhereOrNull((o) => o.id == pageId);
    final existingSectionIds = page?.childrenIds ?? [];
    final existingSections = existingSectionIds
        .map((id) => currentObjects.firstWhereOrNull((o) => o.id == id))
        .whereType<UnifiedObject>()
        .where((o) => o.typeId != 'page')
        .toList();

    // Match by name (case-insensitive, trimmed)
    final importName = importCollection.name.trim().toLowerCase();
    final match = existingSections.firstWhereOrNull(
      (s) => s.name.trim().toLowerCase() == importName,
    );

    if (match != null) {
      idMapping[importCollection.originalId] = match.id;
      return match.id;
    }

    // Create new section
    final newSectionId = const Uuid().v4();
    idMapping[importCollection.originalId] = newSectionId;
    final now = DateTime.now().millisecondsSinceEpoch;

    final newSection = UnifiedObject(
      id: newSectionId,
      typeId: 'collection',
      name: importCollection.name,
      iconName: importCollection.iconName,
      parentId: pageId,
      childrenIds: [],
      properties: {},
      createdAt: now,
      updatedAt: now,
    );
    currentObjects.add(newSection);

    // Update page's childrenIds
    final pageIndex = currentObjects.indexWhere((o) => o.id == pageId);
    if (pageIndex >= 0) {
      final page = currentObjects[pageIndex];
      if (!page.childrenIds.contains(newSectionId)) {
        currentObjects[pageIndex] = page.copyWith(
          childrenIds: [...page.childrenIds, newSectionId],
        );
      }
    }

    return newSectionId;
  }

  /// Add [newChildId] to [parentId]'s childrenIds if not already present.
  void _updateParentChildrenIds(
    List<UnifiedObject> objects,
    String parentId,
    String newChildId,
  ) {
    final parentIndex = objects.indexWhere((o) => o.id == parentId);
    if (parentIndex < 0) return;

    final parent = objects[parentIndex];
    if (!parent.childrenIds.contains(newChildId)) {
      objects[parentIndex] = parent.copyWith(
        childrenIds: [...parent.childrenIds, newChildId],
      );
    }
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
