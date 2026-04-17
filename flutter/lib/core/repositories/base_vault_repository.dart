import 'dart:convert';
import 'dart:typed_data';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';

/// Base class for vault repositories providing common JSON relay operations.
/// All section repositories (Identity, Travel, Financial, etc.) extend this.
abstract class BaseVaultRepository {
  final RustVaultService _rustVault = RustVaultService.instance;

  /// Profile storage for full profile operations
  final ProfileStorageService _profileStorage = ProfileStorageService.instance;

  /// Current account ID - subclasses should call setAccountId on login
  String? _accountId;

  /// Set the current account ID after unlock
  void setAccountId(String? accountId) {
    _accountId = accountId;
  }

  /// Get the current account ID
  String? get accountId => _accountId;

  /// Load full profile data
  Future<ProfileData?> loadProfile() async {
    if (_accountId == null) return null;
    return _profileStorage.loadProfile(_accountId!);
  }

  /// Save full profile data
  Future<bool> saveProfile(ProfileData profile) async {
    if (_accountId == null) return false;
    return _profileStorage.saveProfile(_accountId!, profile);
  }

  /// Vault operations - for when sections need independent storage

  /// List profiles in vault (returns profile metadata, not content)
  Future<List<BridgeProfileSummary>> listVaultProfiles() async {
    return _rustVault.listProfiles();
  }

  /// Save data to a named vault profile
  Future<BridgeProfileSummary?> saveToVault(String name, Map<String, dynamic> data) async {
    final jsonStr = jsonEncode(data);
    final jsonBytes = Uint8List.fromList(jsonStr.codeUnits);
    return _rustVault.saveProfile(name, jsonBytes);
  }

  /// Load data from a vault profile by ID
  Future<Map<String, dynamic>?> loadFromVault(String profileId) async {
    final data = await _rustVault.loadProfile(profileId);
    if (data == null) return null;
    try {
      final jsonStr = String.fromCharCodes(data);
      return jsonDecode(jsonStr) as Map<String, dynamic>;
    } catch (_) {
      return null;
    }
  }

  /// Delete a vault profile by ID
  Future<bool> deleteFromVault(String profileId) async {
    return _rustVault.deleteProfile(profileId);
  }

  /// Get vault statistics
  Map<String, dynamic>? getVaultStats() {
    return _rustVault.getVaultStats();
  }

  /// Check if vault is unlocked
  bool get isVaultUnlocked => _rustVault.isVaultUnlocked();
}
