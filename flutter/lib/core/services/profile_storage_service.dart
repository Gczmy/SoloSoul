import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:isolate';

import 'package:path_provider/path_provider.dart';
import 'package:solosoul_flutter/core/models/profile_data.dart';
export 'package:solosoul_flutter/core/models/profile_data.dart';

import 'package:solosoul_flutter/core/models/unified_object_model.dart';

import 'package:solosoul_flutter/core/services/debug_logger.dart';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';

/// Profile storage service - stores encrypted profile data locally
/// Delegates to RustVaultService via FFI for SQLCipher-encrypted storage
class ProfileStorageService {
  static ProfileStorageService? _instance;

  // Reference to Rust vault service
  final RustVaultService _rustVault = RustVaultService.instance;

  // In-memory cache of last loaded profile per account
  final Map<String, ProfileData> _profileCache = {};

  ProfileStorageService._();

  static ProfileStorageService get instance {
    _instance ??= ProfileStorageService._();
    return _instance!;
  }

  /// Get the storage directory for logs and other files
  /// Uses the app's documents directory
  Future<Directory> get storageDir async {
    final appDir = await getApplicationDocumentsDirectory();
    return Directory('${appDir.path}/solosoul_storage');
  }

  /// Current schema version for unified object model.
  static const int kSchemaVersion = 4;

  /// Migrate profile to latest schema if needed.
  /// Legacy migration paths (v0→v3, v3→v4) have been removed since
  /// all production data is now on v4+ unified object model.
  static ProfileData migrateIfNeeded(ProfileData profile, Map<String, dynamic> rawJson) {
    final currentVersion = profile.schemaVersion ?? 0;
    if (currentVersion >= kSchemaVersion) return profile;

    // For any older version, just bump schema version.
    // Legacy data fields (identity/travel/financial/professional) no longer exist.
    return profile.copyWith(schemaVersion: kSchemaVersion);
  }

  /// Validate and repair data integrity after migration.
  ///
  /// Checks performed:
  /// - Duplicate UnifiedObject IDs (keep first occurrence)
  /// - Invalid [childrenIds] references (remove IDs pointing to non-existent objects)
  /// - Invalid [parentId] references (set to null if parent no longer exists)
  ///
  /// Returns a repaired copy if fixes were applied, or the original if valid.
  static (ProfileData, bool) validateAndRepairProfile(ProfileData profile) {
    var repaired = profile;
    var wasRepaired = false;

    final unifiedObjects = repaired.unifiedObjects;
    if (unifiedObjects != null && unifiedObjects.objects.isNotEmpty) {
      final objectMap = <String, UnifiedObject>{};
      final seenIds = <String>{};

      // Pass 1: deduplicate by ID (keep first occurrence)
      for (final obj in unifiedObjects.objects) {
        if (seenIds.contains(obj.id)) {
          wasRepaired = true;
          continue;
        }
        seenIds.add(obj.id);
        objectMap[obj.id] = obj;
      }

      // Pass 2: repair childrenIds and parentId references
      final repairedObjects = <UnifiedObject>[];
      for (final obj in objectMap.values) {
        // Remove childrenIds that point to non-existent objects
        final validChildren = obj.childrenIds.where((id) => objectMap.containsKey(id)).toList();
        if (validChildren.length != obj.childrenIds.length) {
          wasRepaired = true;
        }

        // Null out parentId if parent no longer exists
        final validParentId = obj.parentId != null && objectMap.containsKey(obj.parentId!)
            ? obj.parentId
            : null;
        if (validParentId != obj.parentId) {
          wasRepaired = true;
        }

        // Direct construction because copyWith cannot set parentId to null
        // (its ?? operator treats null as "don't change").
        repairedObjects.add(UnifiedObject(
          id: obj.id,
          typeId: obj.typeId,
          name: obj.name,
          iconName: obj.iconName,
          parentId: validParentId,
          childrenIds: validChildren,
          properties: obj.properties,
          isDeleted: obj.isDeleted,
          deletedAt: obj.deletedAt,
          createdAt: obj.createdAt,
          updatedAt: obj.updatedAt,
        ));
      }

      if (wasRepaired) {
        repaired = repaired.copyWith(
          unifiedObjects: unifiedObjects.copyWith(objects: repairedObjects),
        );
      }
    }

    return (repaired, wasRepaired);
  }

  /// Load profile data for an account
  /// Decrypts via RustVaultService, deserializes JSON, migrates if needed
  Future<ProfileData?> loadProfile(String accountId) async {
    try {
      // Try to load from Rust vault
      final decrypted = await _rustVault.loadProfileDecrypted(accountId);
      if (decrypted == null) {
        return null;
      }

      final (profile, needsSave, logs) = await Isolate.run(() {
        final json = jsonDecode(decrypted) as Map<String, dynamic>;
        final profile = ProfileData.fromJson(json);
        final migratedProfile = ProfileStorageService.migrateIfNeeded(profile, json);
        final (repairedProfile, wasRepaired) = ProfileStorageService.validateAndRepairProfile(migratedProfile);
        final logs = <String>[];
        if (wasRepaired) {
          logs.add('Data integrity repairs applied during load');
        }
        return (repairedProfile, wasRepaired, logs);
      });

      // Replay isolate logs on main thread
      for (final msg in logs) {
        DebugLogger.instance.logInfo('PROFILE', msg);
      }

      // Persist repairs so they don't need to be re-applied next load
      if (needsSave) {
        unawaited(
          saveProfile(accountId, profile).catchError((e) {
            DebugLogger.instance.logError(
              'PROFILE',
              'Failed to persist repaired profile: $e',
            );
            return false;
          }),
        );
      }
      _profileCache[accountId] = profile;
      return profile;
    } on RemoteError catch (e) {
      DebugLogger.instance.logError(
        'PROFILE',
        'Profile load failed in isolate: ${e.toString()}',
      );
      return null;
    } on Exception catch (e, st) {
      DebugLogger.instance.logError('PROFILE', 'loadProfile failed: $e\n$st');
      return null;
    }
  }

  /// Save profile data for an account
  /// Encrypts and stores via RustVaultService
  Future<bool> saveProfile(String accountId, ProfileData profile) async {
    try {
      // Data protection: prevent accidental loss of unifiedObjects
      final existing = _profileCache[accountId];
      if (existing?.unifiedObjects != null && profile.unifiedObjects == null) {
        profile = profile.copyWith(unifiedObjects: existing!.unifiedObjects);
      }

      final json = await Isolate.run(() => jsonEncode(profile.toJson()));

      final result = await _rustVault.saveProfileEncrypted(accountId, json);

      if (result == null) {
        return false;
      }

      _profileCache[accountId] = profile;
      return true;
    } on Exception catch (e) {
      DebugLogger.instance
          .logError('PROFILE', 'saveProfile failed for $accountId: $e');
      return false;
    }
  }

  /// Delete profile data for an account
  Future<bool> deleteProfile(String accountId) async {
    try {
      final result = await _rustVault.deleteProfile(accountId);
      return result;
    } on Exception catch (e) {
      DebugLogger.instance
          .logError('PROFILE', 'deleteProfile failed for $accountId: $e');
      return false;
    }
  }
}
