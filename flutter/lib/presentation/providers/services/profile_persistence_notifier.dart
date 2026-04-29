import 'dart:async';
import 'dart:convert';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/core/services/field_history_service.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';
import 'package:solosoul_flutter/presentation/providers/operation_log_provider.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';

/// Debounce duration for profile saves (500ms)
const kSaveDebounceDuration = Duration(milliseconds: 500);

/// Service responsible for profile data loading and saving with debounce.
/// This class handles:
/// - Loading profile from storage
/// - Debounced saving to storage
/// - Change detection via JSON comparison
class ProfilePersistenceService {
  final ProfileStorageService _storage = ProfileStorageService.instance;
  final Ref _ref;
  bool _isLoading = false;

  /// Timer for debouncing saves
  Timer? _saveDebounceTimer;

  /// Last saved JSON string (for change detection)
  String? _lastSavedJson;

  ProfilePersistenceService(this._ref);

  bool get isLoading => _isLoading;

  /// Clear profile state (when auth is locked or reset)
  Future<void> clearProfile() async {
    _saveDebounceTimer?.cancel();
    _saveDebounceTimer = null;
    _lastSavedJson = null;
    await OperationLogService.instance.clearForCurrentAccount();
  }

  /// Load profile for the currently unlocked account
  Future<ProfileData?> loadProfile() async {
    if (_isLoading) return null;
    _isLoading = true;

    try {
      final authState = _ref.read(authNotifierProvider).value;
      if (authState != AuthState.unlocked) return null;

      final authNotifier = _ref.read(authNotifierProvider.notifier);
      final accountId = authNotifier.selectedAccountId;
      if (accountId == null) return null;

      final encryptionKey = _storage.encryptionKey;
      if (encryptionKey != null) {
        OperationLogService.instance.setEncryptionKey(encryptionKey);
      } else {
        return null;
      }

      await OperationLogService.instance.initializeForAccount(accountId);

      final authStateBeforeLoad = _ref.read(authNotifierProvider).value;
      if (authStateBeforeLoad != AuthState.unlocked) return null;

      final profile = await _storage.loadProfile(accountId);
      await _storage.purgeOldDeletedItemsIfNeeded(accountId, existingProfile: profile);

      final authStateAfterLoad = _ref.read(authNotifierProvider).value;
      if (authStateAfterLoad != AuthState.unlocked) return null;

      if (profile != null) {
        _lastSavedJson = jsonEncode(profile.toJson());
        unawaited(_ref.read(fieldHistoriesProvider.notifier).loadHistories(accountId));
        // Cleanup field histories for items that no longer exist
        unawaited(
          FieldHistoryService.instance.cleanupOrphanHistories(
            accountId: accountId,
            validItemIds: profile.collectAllItemIds(),
          ),
        );
      }
      return profile;
    } finally {
      _isLoading = false;
    }
  }

  /// Save profile for the currently unlocked account (debounced)
  /// Returns true if save was queued or completed successfully
  Future<bool> saveProfile(ProfileData profile, {bool immediate = false}) async {
    final newJson = jsonEncode(profile.toJson());

    if (newJson == _lastSavedJson) return true;

    final authState = _ref.read(authNotifierProvider).value;
    if (authState != AuthState.unlocked) return false;

    final encryptionKey = _storage.encryptionKey;
    if (encryptionKey == null) return false;

    final authNotifier = _ref.read(authNotifierProvider.notifier);
    final accountId = authNotifier.selectedAccountId;
    if (accountId == null) return false;

    Future<bool> doSave() async {
      final result = await _storage.saveProfile(accountId, profile);
      if (result) {
        _lastSavedJson = newJson;
      }
      return result;
    }

    if (immediate) {
      _saveDebounceTimer?.cancel();
      _saveDebounceTimer = null;
      return doSave();
    }

    _saveDebounceTimer?.cancel();
    _saveDebounceTimer = Timer(kSaveDebounceDuration, () async {
      _saveDebounceTimer = null;
      try {
        await doSave();
      } catch (e, st) {
        DebugLogger.instance.logError('PROFILE', 'Debounced save failed: $e $st');
        rethrow;
      }
    });

    return true;
  }

  /// Force an immediate save (bypasses debounce)
  Future<bool> saveProfileImmediate(ProfileData profile) async {
    return saveProfile(profile, immediate: true);
  }

  /// Reload profile from storage
  Future<ProfileData?> reloadProfile(String accountId) async {
    return _storage.loadProfile(accountId);
  }

  void dispose() {
    _saveDebounceTimer?.cancel();
    _saveDebounceTimer = null;
  }
}
