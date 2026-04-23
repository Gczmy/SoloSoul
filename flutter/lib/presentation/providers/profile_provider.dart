import 'dart:async';
import 'dart:convert';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/field_history_service.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/core/services/operation_logger.dart';
import 'package:solosoul_flutter/core/services/log_section_config.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';
import 'package:solosoul_flutter/presentation/pages/operation_log_page.dart';
import 'package:solosoul_flutter/presentation/providers/profile_section_editor.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart'
    show formFieldRegistryProvider;

// Re-export field history types for backward compatibility
export 'package:solosoul_flutter/core/services/field_history_service.dart'
    show fieldHistoriesProvider, FieldHistoriesNotifier;
export 'package:solosoul_flutter/core/models/field_history_models.dart'
    show FieldHistory, FormHistories;

/// Debounce duration for profile saves (500ms)
const _kSaveDebounceDuration = Duration(milliseconds: 500);

/// Profile notifier for loading and saving profile data
class ProfileNotifier extends StateNotifier<ProfileData?> {
  final ProfileStorageService _storage = ProfileStorageService.instance;
  final Ref _ref;
  bool _isLoading = false;

  /// Timer for debouncing saves
  Timer? _saveDebounceTimer;

  /// Last saved JSON string (for change detection)
  String? _lastSavedJson;

  ProfileNotifier(this._ref) : super(null);

  @override
  void dispose() {
    _saveDebounceTimer?.cancel();
    _saveDebounceTimer = null;
    super.dispose();
  }

  bool get isLoading => _isLoading;

  /// Clear profile state (when auth is locked or reset)
  Future<void> clearProfile() async {
    // Cancel any pending debounced save
    _saveDebounceTimer?.cancel();
    _saveDebounceTimer = null;
    _lastSavedJson = null;
    state = null;
    // Clear operation logs for current account (waits for pending saves to complete)
    await OperationLogService.instance.clearForCurrentAccount();
  }

  /// Load profile for the currently unlocked account
  Future<void> loadProfile() async {

    // Prevent concurrent loads
    if (_isLoading) {
      return;
    }
    _isLoading = true;

    try {
      // Check auth state is still unlocked (may have changed while awaiting)
      final authState = _ref.read(authNotifierProvider);
      if (authState != AuthState.unlocked) {
        return;
      }

      final authNotifier = _ref.read(authNotifierProvider.notifier);
      final accountId = authNotifier.selectedAccountId;
      if (accountId == null) {
        return;
      }

      // Sync encryption key to OperationLogService (logs use same encryption)
      final encryptionKey = _storage.encryptionKey;
      if (encryptionKey != null) {
        OperationLogService.instance.setEncryptionKey(encryptionKey);
      } else {
        // Vault is locked, cannot load profile without encryption key
        return;
      }

      // Initialize operation log service for this account
      await OperationLogService.instance.initializeForAccount(accountId);

      // Re-check auth state before loading (may have changed during awaits)
      final authStateBeforeLoad = _ref.read(authNotifierProvider);
      if (authStateBeforeLoad != AuthState.unlocked) {
        return;
      }

      final profile = await _storage.loadProfile(accountId);

      // Auto-purge items deleted more than 30 days ago (using already-loaded profile)
      await _storage.purgeOldDeletedItemsIfNeeded(accountId, existingProfile: profile);

      // Final auth state check before updating state
      final authStateAfterLoad = _ref.read(authNotifierProvider);
      if (authStateAfterLoad != AuthState.unlocked) {
        return;
      }

      // Only update state if profile was successfully loaded
      if (profile != null) {
        state = profile;
        // Initialize lastSavedJson for change detection
        _lastSavedJson = jsonEncode(profile.toJson());
        // Load field histories after profile loads
        _ref.read(fieldHistoriesProvider.notifier).loadHistories(accountId);
      } else {
      }
    } finally {
      _isLoading = false;
    }
  }

  /// Save profile for the currently unlocked account (debounced)
  /// Returns true if save was queued or completed successfully
  Future<bool> saveProfile(ProfileData profile, {bool immediate = false}) async {
    // Build JSON for comparison
    final newJson = jsonEncode(profile.toJson());

    // Check if data has actually changed
    if (newJson == _lastSavedJson) {
      return true;
    }

    // Verify vault is unlocked before saving
    final authState = _ref.read(authNotifierProvider);
    if (authState != AuthState.unlocked) {
      return false;
    }

    final encryptionKey = _storage.encryptionKey;
    if (encryptionKey == null) {
      return false;
    }

    final authNotifier = _ref.read(authNotifierProvider.notifier);
    final accountId = authNotifier.selectedAccountId;
    if (accountId == null) {
      return false;
    }

    // Helper to perform the actual save
    Future<bool> doSave() async {
      final result = await _storage.saveProfile(accountId, profile);
      if (result) {
        _lastSavedJson = newJson;
      } else {
      }
      return result;
    }

    if (immediate) {
      // Cancel any pending debounced save and save now
      _saveDebounceTimer?.cancel();
      _saveDebounceTimer = null;
      return doSave();
    }

    // Debounce: cancel previous timer and schedule new save
    _saveDebounceTimer?.cancel();
    _saveDebounceTimer = Timer(_kSaveDebounceDuration, () async {
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

  /// Update identity data
  Future<bool> updateIdentity(IdentityData identity) async {
    final current = state ?? ProfileData();
    final oldIdentity = current.identity;

    // Log changes to identity fields
    _logIdentityChanges(oldIdentity, identity);

    // Determine if this is a create or update for operation tracking
    final isCreate = oldIdentity == null;

    // Create new ProfileData to ensure state change is detected by Riverpod
    final newProfile = ProfileData(
      identity: identity,
      travel: current.travel,
      financial: current.financial,
      professional: current.professional,
    );

    // Save FIRST, then update state only on success
    final result = await saveProfile(newProfile);
    if (!result) {
      return false;
    }

    state = newProfile;

    // Update account operation metadata
    await _ref
        .read(authNotifierProvider.notifier)
        .updateOperation(_summarizeIdentityChanges(oldIdentity, identity, isCreate));

    return true;
  }

  /// Summarize identity changes into a human-readable operation string.
  /// e.g. "Updated Profile — Contact Information: added 'Email'"
  String _summarizeIdentityChanges(IdentityData? old, IdentityData? newData, bool isCreate) {
    if (isCreate) return 'Created Profile';

    final changes = <String>[];

    // Check simple fields
    if (old?.fullName != newData?.fullName) {
      changes.add('Full Name');
    }
    if (old?.givenName != newData?.givenName) changes.add('Given Name');
    if (old?.familyName != newData?.familyName) changes.add('Family Name');
    if (old?.dateOfBirth != newData?.dateOfBirth) changes.add('Date of Birth');
    if (old?.gender != newData?.gender) changes.add('Gender');
    if (old?.nationality != newData?.nationality) changes.add('Nationality');

    // Check list fields
    final oldContacts = old?.contact?.entries ?? [];
    final newContacts = newData?.contact?.entries ?? [];
    if (newContacts.length > oldContacts.length) {
      changes.add('Contact Information (${newContacts.length - oldContacts.length} added)');
    } else if (newContacts.length < oldContacts.length) {
      changes.add('Contact Information (${oldContacts.length - newContacts.length} removed)');
    } else if (oldContacts.length != newContacts.length ||
        (oldContacts.isNotEmpty && newContacts.isNotEmpty &&
         (oldContacts.first.title != newContacts.first.title ||
          oldContacts.first.value != newContacts.first.value))) {
      changes.add('Contact Information');
    }

    final oldIdCards = old?.idCards ?? [];
    final newIdCards = newData?.idCards ?? [];
    if (newIdCards.length > oldIdCards.length) {
      changes.add('ID Card (${newIdCards.length - oldIdCards.length} added)');
    } else if (newIdCards.length < oldIdCards.length) {
      changes.add('ID Card (${oldIdCards.length - newIdCards.length} removed)');
    } else if (newIdCards.isNotEmpty && oldIdCards.isNotEmpty &&
        (oldIdCards.first.title != newIdCards.first.title ||
         oldIdCards.first.number != newIdCards.first.number)) {
      changes.add('ID Card');
    }

    final oldAddresses = old?.addresses ?? [];
    final newAddresses = newData?.addresses ?? [];
    if (newAddresses.length > oldAddresses.length) {
      changes.add('Address (${newAddresses.length - oldAddresses.length} added)');
    } else if (newAddresses.length < oldAddresses.length) {
      changes.add('Address (${oldAddresses.length - newAddresses.length} removed)');
    } else if (newAddresses.isNotEmpty && oldAddresses.isNotEmpty &&
        (oldAddresses.first.title != newAddresses.first.title ||
         oldAddresses.first.city != newAddresses.first.city)) {
      changes.add('Address');
    }

    if (changes.isEmpty) return 'Updated Profile';

    // Truncate if too many items
    if (changes.length <= 3) {
      return 'Updated Profile — ${changes.join(', ')}';
    }
    return 'Updated Profile — ${changes.take(3).join(', ')} (+${changes.length - 3} more)';
  }

  /// Summarize travel changes into a human-readable operation string.
  String _summarizeTravelChanges(TravelData? old, TravelData? newData, bool isCreate) {
    if (isCreate) return 'Created Travel Data';

    final changes = <String>[];
    final oldPassports = old?.passports ?? [];
    final newPassports = newData?.passports ?? [];
    if (newPassports.length > oldPassports.length) {
      changes.add('Passport (${newPassports.length - oldPassports.length} added)');
    } else if (newPassports.length < oldPassports.length) {
      changes.add('Passport (${oldPassports.length - newPassports.length} removed)');
    }

    final oldVisas = old?.visas ?? [];
    final newVisas = newData?.visas ?? [];
    if (newVisas.length > oldVisas.length) {
      changes.add('Visa (${newVisas.length - oldVisas.length} added)');
    } else if (newVisas.length < oldVisas.length) {
      changes.add('Visa (${oldVisas.length - newVisas.length} removed)');
    }

    final oldHistory = old?.travelHistory ?? [];
    final newHistory = newData?.travelHistory ?? [];
    if (newHistory.length > oldHistory.length) {
      changes.add('Travel History (${newHistory.length - oldHistory.length} added)');
    } else if (newHistory.length < oldHistory.length) {
      changes.add('Travel History (${oldHistory.length - newHistory.length} removed)');
    }

    if (changes.isEmpty) return 'Updated Travel Data';
    return 'Updated Travel Data — ${changes.join(', ')}';
  }

  /// Summarize financial changes into a human-readable operation string.
  String _summarizeFinancialChanges(FinancialData? old, FinancialData? newData, bool isCreate) {
    if (isCreate) return 'Created Financial Data';

    final changes = <String>[];
    final oldAccounts = old?.bankAccounts ?? [];
    final newAccounts = newData?.bankAccounts ?? [];
    if (newAccounts.length > oldAccounts.length) {
      changes.add('Bank Account (${newAccounts.length - oldAccounts.length} added)');
    } else if (newAccounts.length < oldAccounts.length) {
      changes.add('Bank Account (${oldAccounts.length - newAccounts.length} removed)');
    }

    final oldCards = old?.cards ?? [];
    final newCards = newData?.cards ?? [];
    if (newCards.length > oldCards.length) {
      changes.add('Card (${newCards.length - oldCards.length} added)');
    } else if (newCards.length < oldCards.length) {
      changes.add('Card (${oldCards.length - newCards.length} removed)');
    }

    if (changes.isEmpty) return 'Updated Financial Data';
    return 'Updated Financial Data — ${changes.join(', ')}';
  }

  /// Summarize professional changes into a human-readable operation string.
  String _summarizeProfessionalChanges(ProfessionalData? old, ProfessionalData? newData, bool isCreate) {
    if (isCreate) return 'Created Professional Data';

    final changes = <String>[];
    final oldEdu = old?.education ?? [];
    final newEdu = newData?.education ?? [];
    if (newEdu.length > oldEdu.length) {
      changes.add('Education (${newEdu.length - oldEdu.length} added)');
    } else if (newEdu.length < oldEdu.length) {
      changes.add('Education (${oldEdu.length - newEdu.length} removed)');
    }

    final oldEmp = old?.employment ?? [];
    final newEmp = newData?.employment ?? [];
    if (newEmp.length > oldEmp.length) {
      changes.add('Employment (${newEmp.length - oldEmp.length} added)');
    } else if (newEmp.length < oldEmp.length) {
      changes.add('Employment (${oldEmp.length - newEmp.length} removed)');
    }

    final oldSkills = old?.skills ?? [];
    final newSkills = newData?.skills ?? [];
    if (newSkills.length > oldSkills.length) {
      changes.add('Skill (${newSkills.length - oldSkills.length} added)');
    } else if (newSkills.length < oldSkills.length) {
      changes.add('Skill (${oldSkills.length - newSkills.length} removed)');
    }

    final oldLang = old?.languages ?? [];
    final newLang = newData?.languages ?? [];
    if (newLang.length > oldLang.length) {
      changes.add('Language (${newLang.length - oldLang.length} added)');
    } else if (newLang.length < oldLang.length) {
      changes.add('Language (${oldLang.length - newLang.length} removed)');
    }

    if (changes.isEmpty) return 'Updated Professional Data';
    return 'Updated Professional Data — ${changes.join(', ')}';
  }

  /// Log changes between old and new identity data
  void _logIdentityChanges(IdentityData? old, IdentityData? newData) {
    if (old == null && newData != null) {
      // New identity created
      OperationLogService.instance.addEntry(
        OperationLogger.logIdentity(
          action: LogAction.create,
          description: 'Created identity profile',
          fieldPath: 'identity',
        ),
      );
      return;
    }
    if (old != null && newData == null) {
      // Identity deleted
      OperationLogService.instance.addEntry(
        OperationLogger.logIdentity(
          action: LogAction.delete,
          description: 'Deleted identity profile',
          fieldPath: 'identity',
        ),
      );
      return;
    }
    if (old == null || newData == null) return;

    // Compare full name (with before→after)
    if (old.fullName != newData.fullName) {
      final action = old.fullName == null ? LogAction.create : LogAction.update;
      String description;
      if (action == LogAction.update) {
        description = 'Updated fullName: ${old.fullName} → ${newData.fullName}';
      } else {
        description = 'Added fullName: ${newData.fullName}';
      }
      OperationLogService.instance.addEntry(
        OperationLogger.logIdentity(
          action: action,
          description: description,
          fieldPath: 'fullName',
        ),
      );
    }

    // Compare contacts (detailed comparison with before→after)
    final oldContacts = old.contact?.entries ?? [];
    final newContacts = newData.contact?.entries ?? [];
    _logIdentityListChanges(
      oldList: oldContacts,
      newList: newContacts,
      section: LogSection.contactInformation,
      itemType: 'contact',
      compareEntry: (oldEntry, newEntry) =>
          oldEntry.title != newEntry.title ||
          oldEntry.type != newEntry.type ||
          oldEntry.value != newEntry.value,
      getLabel: (entry) => entry.title.isNotEmpty ? entry.title : entry.value,
      showDiff: true,
    );

    // Compare ID cards (with before→after)
    final oldIdCards = old.idCards ?? [];
    final newIdCards = newData.idCards ?? [];
    _logIdentityListChanges(
      oldList: oldIdCards,
      newList: newIdCards,
      section: LogSection.idCard,
      itemType: 'ID card',
      compareEntry: (oldEntry, newEntry) =>
          oldEntry.title != newEntry.title ||
          oldEntry.number != newEntry.number,
      getLabel: (entry) => entry.title ?? entry.number ?? 'ID card',
      showDiff: true,
    );

    // Compare addresses (with before→after)
    final oldAddresses = old.addresses ?? [];
    final newAddresses = newData.addresses ?? [];
    _logIdentityListChanges(
      oldList: oldAddresses,
      newList: newAddresses,
      section: LogSection.address,
      itemType: 'address',
      compareEntry: (oldEntry, newEntry) =>
          oldEntry.title != newEntry.title ||
          oldEntry.country != newEntry.country ||
          oldEntry.city != newEntry.city ||
          oldEntry.street != newEntry.street ||
          oldEntry.postalCode != newEntry.postalCode,
      getLabel: (entry) => entry.title ?? 'Address',
      showDiff: true,
    );
  }

  /// Log identity list changes with detailed field comparison
  /// Logs adds, deletes, and updates when individual fields change
  /// If showDiff is true, includes before→after in description
  void _logIdentityListChanges<T>({
    required List<T> oldList,
    required List<T> newList,
    required LogSection section,
    required String itemType,
    required bool Function(T oldEntry, T newEntry) compareEntry,
    required String Function(T entry) getLabel,
    bool showDiff = false,
  }) {
    final oldLen = oldList.length;
    final newLen = newList.length;

    if (oldLen < newLen) {
      // Items added - always use create action regardless of oldList count
      final count = newLen - oldLen;
      final description = count == 1
          ? 'Added $itemType${newLen > 0 ? ': ${getLabel(newList.first)}' : ''}'
          : 'Added $count $itemType items';
      _addLogEntry(
        section: section,
        action: LogAction.create,
        description: description,
      );
    } else if (oldLen > newLen) {
      // Items deleted
      final count = oldLen - newLen;
      final description = count == 1
          ? 'Deleted $itemType${oldLen > 0 ? ': ${getLabel(oldList.first)}' : ''}'
          : 'Deleted $count $itemType items';
      _addLogEntry(
        section: section,
        action: LogAction.delete,
        description: description,
      );
    } else if (oldLen > 0) {
      // Same count - check if any item was modified
      // Log ALL changes, not just the first one
      for (var i = 0; i < oldLen; i++) {
        if (compareEntry(oldList[i], newList[i])) {
          String description;
          if (showDiff) {
            description =
                'Updated $itemType: ${getLabel(oldList[i])} → ${getLabel(newList[i])}';
          } else {
            description = 'Updated $itemType: ${getLabel(newList[i])}';
          }
          _addLogEntry(
            section: section,
            action: LogAction.update,
            description: description,
          );
          // Don't break - log ALL changes that occurred
        }
      }
    }
  }

  /// Log simple list changes based on count difference
  void _logSimpleListChanges({
    required int oldList,
    required int newList,
    required LogSection section,
    required String itemType,
    String? itemLabel,
  }) {
    if (oldList < newList) {
      // Items added - always use create action regardless of oldList count
      final count = newList - oldList;
      final description = count == 1
          ? 'Added $itemType${itemLabel != null ? ': $itemLabel' : ''}'
          : 'Added $count $itemType items';
      _addLogEntry(
        section: section,
        action: LogAction.create,
        description: description,
      );
    } else if (oldList > newList) {
      // Items deleted
      final count = oldList - newList;
      final description = count == 1
          ? 'Deleted $itemType${itemLabel != null ? ': $itemLabel' : ''}'
          : 'Deleted $count $itemType items';
      _addLogEntry(
        section: section,
        action: LogAction.delete,
        description: description,
      );
    }
    // If same count, items may have been updated - for now we don't log this
  }

  /// Add a log entry based on section
  void _addLogEntry({
    required LogSection section,
    required LogAction action,
    required String description,
  }) {
    switch (section) {
      case LogSection.identity:
        OperationLogService.instance.addEntry(
          OperationLogger.logIdentity(action: action, description: description),
        );
        break;
      case LogSection.contactInformation:
        OperationLogService.instance.addEntry(
          OperationLogger.logContactInformation(
            action: action,
            description: description,
          ),
        );
        break;
      case LogSection.address:
        OperationLogService.instance.addEntry(
          OperationLogger.logAddress(action: action, description: description),
        );
        break;
      case LogSection.idCard:
        OperationLogService.instance.addEntry(
          OperationLogger.logIdCard(action: action, description: description),
        );
        break;
      case LogSection.passport:
        OperationLogService.instance.addEntry(
          OperationLogger.logPassport(action: action, description: description),
        );
        break;
      case LogSection.visa:
        OperationLogService.instance.addEntry(
          OperationLogger.logVisa(action: action, description: description),
        );
        break;
      case LogSection.travelHistory:
        OperationLogService.instance.addEntry(
          OperationLogger.logTravelHistory(
            action: action,
            description: description,
          ),
        );
        break;
      case LogSection.bankAccount:
        OperationLogService.instance.addEntry(
          OperationLogger.logBankAccount(
            action: action,
            description: description,
          ),
        );
        break;
      case LogSection.card:
        OperationLogService.instance.addEntry(
          OperationLogger.logCard(action: action, description: description),
        );
        break;
      case LogSection.education:
        OperationLogService.instance.addEntry(
          OperationLogger.logEducation(
            action: action,
            description: description,
          ),
        );
        break;
      case LogSection.employment:
        OperationLogService.instance.addEntry(
          OperationLogger.logEmployment(
            action: action,
            description: description,
          ),
        );
        break;
      case LogSection.skill:
        OperationLogService.instance.addEntry(
          OperationLogger.logSkill(action: action, description: description),
        );
        break;
      case LogSection.language:
        OperationLogService.instance.addEntry(
          OperationLogger.logLanguage(action: action, description: description),
        );
        break;
      case LogSection.travel:
        OperationLogService.instance.addEntry(
          OperationLogger.logTravel(action: action, description: description),
        );
        break;
      case LogSection.financial:
        OperationLogService.instance.addEntry(
          OperationLogger.logFinancial(
            action: action,
            description: description,
          ),
        );
        break;
      case LogSection.professional:
        OperationLogService.instance.addEntry(
          OperationLogger.logProfessional(
            action: action,
            description: description,
          ),
        );
        break;
      case LogSection.sensitivitySettings:
        OperationLogService.instance.addEntry(
          OperationLogger.logSensitivitySettings(
            action: action,
            description: description,
          ),
        );
        break;
    }
  }

  /// Add or update travel data
  Future<bool> updateTravel(TravelData travel) async {
    final current = state ?? ProfileData();
    final oldTravel = current.travel;

    // Log travel changes
    _logTravelChanges(oldTravel, travel);

    // Determine if this is a create or update for operation tracking
    final isCreate = oldTravel == null;

    final updated = current.copyWith(travel: travel);

    // DEBUG logging

    // Save FIRST, then update state only on success (FIX: was incorrectly mutating state before save)
    final result = await saveProfile(updated);


    if (!result) {
      return false;
    }

    state = updated;

    // Update account operation metadata
    await _ref
        .read(authNotifierProvider.notifier)
        .updateOperation(_summarizeTravelChanges(oldTravel, travel, isCreate));

    return result;
  }

  /// Update travel data with immediate save (bypasses debounce)
  Future<bool> updateTravelImmediate(TravelData travel) async {
    final current = state ?? ProfileData();
    final oldTravel = current.travel;

    _logTravelChanges(oldTravel, travel);
    final isCreate = oldTravel == null;

    final updated = current.copyWith(travel: travel);
    final result = await saveProfileImmediate(updated);

    if (!result) {
      return false;
    }

    state = updated;
    await _ref
        .read(authNotifierProvider.notifier)
        .updateOperation(_summarizeTravelChanges(oldTravel, travel, isCreate));

    return result;
  }

  /// Log changes between old and new travel data
  void _logTravelChanges(TravelData? old, TravelData? newData) {
    if (old == null && newData != null) {
      _addLogEntry(
        section: LogSection.travel,
        action: LogAction.create,
        description: 'Added travel data',
      );
      return;
    }
    if (old != null && newData == null) {
      _addLogEntry(
        section: LogSection.travel,
        action: LogAction.delete,
        description: 'Deleted travel data',
      );
      return;
    }
    if (old == null || newData == null) return;

    // Compare passports
    _logSimpleListChanges(
      oldList: old.passports.length,
      newList: newData.passports.length,
      section: LogSection.passport,
      itemType: 'passport',
      itemLabel: newData.passports.isNotEmpty ? newData.passports.first.number : null,
    );

    // Compare visas
    _logSimpleListChanges(
      oldList: old.visas.length,
      newList: newData.visas.length,
      section: LogSection.visa,
      itemType: 'visa',
      itemLabel: newData.visas.isNotEmpty ? newData.visas.first.country : null,
    );

    // Compare travel history
    _logSimpleListChanges(
      oldList: old.travelHistory.length,
      newList: newData.travelHistory.length,
      section: LogSection.travelHistory,
      itemType: 'travel history entry',
      itemLabel: newData.travelHistory.isNotEmpty ? newData.travelHistory.first.destination : null,
    );
  }

  /// Add or update financial data
  Future<bool> updateFinancial(FinancialData financial) async {
    final current = state ?? ProfileData();
    final oldFinancial = current.financial;

    // Log financial changes
    _logFinancialChanges(oldFinancial, financial);

    // Determine if this is a create or update for operation tracking
    final isCreate = oldFinancial == null;

    final updated = current.copyWith(financial: financial);

    // DEBUG logging

    // Save FIRST, then update state only on success (FIX: was incorrectly mutating state before save)
    final result = await saveProfile(updated);


    if (!result) {
      return false;
    }

    state = updated;

    // Update account operation metadata
    await _ref
        .read(authNotifierProvider.notifier)
        .updateOperation(_summarizeFinancialChanges(oldFinancial, financial, isCreate));

    return result;
  }

  /// Update financial data with immediate save (bypasses debounce)
  Future<bool> updateFinancialImmediate(FinancialData financial) async {
    final current = state ?? ProfileData();
    final oldFinancial = current.financial;

    _logFinancialChanges(oldFinancial, financial);
    final isCreate = oldFinancial == null;

    final updated = current.copyWith(financial: financial);
    final result = await saveProfileImmediate(updated);

    if (!result) {
      return false;
    }

    state = updated;
    await _ref
        .read(authNotifierProvider.notifier)
        .updateOperation(_summarizeFinancialChanges(oldFinancial, financial, isCreate));

    return result;
  }

  /// Log changes between old and new financial data
  void _logFinancialChanges(FinancialData? old, FinancialData? newData) {
    if (old == null && newData != null) {
      _addLogEntry(
        section: LogSection.financial,
        action: LogAction.create,
        description: 'Added financial data',
      );
      return;
    }
    if (old != null && newData == null) {
      _addLogEntry(
        section: LogSection.financial,
        action: LogAction.delete,
        description: 'Deleted financial data',
      );
      return;
    }
    if (old == null || newData == null) return;

    // Compare bank accounts
    _logSimpleListChanges(
      oldList: old.bankAccounts.length,
      newList: newData.bankAccounts.length,
      section: LogSection.bankAccount,
      itemType: 'bank account',
      itemLabel: newData.bankAccounts.isNotEmpty ? newData.bankAccounts.first.bankName : null,
    );

    // Compare cards
    _logSimpleListChanges(
      oldList: old.cards.length,
      newList: newData.cards.length,
      section: LogSection.card,
      itemType: 'card',
      itemLabel: newData.cards.isNotEmpty ? newData.cards.first.cardType : null,
    );
  }

  /// Add or update professional data
  Future<bool> updateProfessional(ProfessionalData professional) async {
    final current = state ?? ProfileData();
    final oldProfessional = current.professional;

    // Log professional changes
    _logProfessionalChanges(oldProfessional, professional);

    // Determine if this is a create or update for operation tracking
    final isCreate = oldProfessional == null;

    final updated = current.copyWith(professional: professional);

    // DEBUG logging

    // Save FIRST, then update state only on success (FIX: was incorrectly mutating state before save)
    final result = await saveProfile(updated);


    if (!result) {
      return false;
    }

    state = updated;

    // Update account operation metadata
    await _ref
        .read(authNotifierProvider.notifier)
        .updateOperation(_summarizeProfessionalChanges(oldProfessional, professional, isCreate));

    return result;
  }

  /// Update professional data with immediate save (bypasses debounce)
  Future<bool> updateProfessionalImmediate(ProfessionalData professional) async {
    final current = state ?? ProfileData();
    final oldProfessional = current.professional;

    _logProfessionalChanges(oldProfessional, professional);
    final isCreate = oldProfessional == null;

    final updated = current.copyWith(professional: professional);
    final result = await saveProfileImmediate(updated);

    if (!result) {
      return false;
    }

    state = updated;
    await _ref
        .read(authNotifierProvider.notifier)
        .updateOperation(_summarizeProfessionalChanges(oldProfessional, professional, isCreate));

    return result;
  }

  /// Log changes between old and new professional data
  void _logProfessionalChanges(
    ProfessionalData? old,
    ProfessionalData? newData,
  ) {
    if (old == null && newData != null) {
      _addLogEntry(
        section: LogSection.professional,
        action: LogAction.create,
        description: 'Added professional data',
      );
      return;
    }
    if (old != null && newData == null) {
      _addLogEntry(
        section: LogSection.professional,
        action: LogAction.delete,
        description: 'Deleted professional data',
      );
      return;
    }
    if (old == null || newData == null) return;

    // Compare education
    _logSimpleListChanges(
      oldList: old.education.length,
      newList: newData.education.length,
      section: LogSection.education,
      itemType: 'education entry',
      itemLabel: newData.education.isNotEmpty
          ? newData.education.first.institution
          : null,
    );

    // Compare employment
    _logSimpleListChanges(
      oldList: old.employment.length,
      newList: newData.employment.length,
      section: LogSection.employment,
      itemType: 'employment entry',
      itemLabel: newData.employment.isNotEmpty ? newData.employment.first.company : null,
    );

    // Compare skills
    _logSimpleListChanges(
      oldList: old.skills.length,
      newList: newData.skills.length,
      section: LogSection.skill,
      itemType: 'skill',
      itemLabel: newData.skills.isNotEmpty ? newData.skills.first.toString() : null,
    );

    // Compare languages
    _logSimpleListChanges(
      oldList: old.languages.length,
      newList: newData.languages.length,
      section: LogSection.language,
      itemType: 'language',
      itemLabel: newData.languages.isNotEmpty ? newData.languages.first.toString() : null,
    );
  }

  /// Soft delete an item (marks as deleted but doesn't remove)
  Future<void> softDelete({
    required String section,
    required String itemType,
    required int index,
    required dynamic deletedItem,
  }) async {
    if (state == null) {
      return;
    }

    final accountId = _ref
        .read(authNotifierProvider.notifier)
        .selectedAccountId;
    if (accountId == null) {
      return;
    }

    final current = state!;
    final now = DateTime.now();

    // Find index by ID (robust, no content comparison needed)
    final actualIndex = _findIndexById(current, section, itemType, deletedItem.id);
    if (actualIndex < 0) {
      return;
    }
    final (newProfile, deletedFound) = ProfileSectionEditor.markDeleted(
      current: current,
      section: section,
      itemType: itemType,
      index: actualIndex,
      deletedAt: now,
    );

    // Log the delete operation
    _logSoftDelete(section, itemType, deletedItem);

    // Save FIRST (immediate to invalidate cache), then update state only on success
    final saved = await saveProfileImmediate(newProfile);
    if (!saved) {
      _addLogEntry(
        section: LogSectionConfig.getLogSection(section, itemType),
        action: LogAction.update,
        description: 'ERROR: Failed to save soft delete for $itemType',
      );
      return;
    }

    state = newProfile;

    // Update account operation metadata
    await _ref
        .read(authNotifierProvider.notifier)
        .updateOperation('Deleted $itemType');
  }


  /// Log a soft delete operation
  void _logSoftDelete(String section, String itemType, dynamic deletedItem) {
    // Use centralized config to determine LogSection and itemLabel
    final logSection = LogSectionConfig.getLogSection(section, itemType);
    final itemLabel = LogSectionConfig.getItemLabel(
      section,
      itemType,
      deletedItem,
    );

    _addLogEntry(
      section: logSection,
      action: LogAction.delete,
      description: 'Moved $itemType to trash: $itemLabel',
    );
  }

  /// Restore a soft-deleted item
  Future<void> restore({
    required String section,
    required String itemType,
    required String id,
  }) async {
    if (state == null) {
      throw Exception('No profile loaded');
    }

    final accountId = _ref
        .read(authNotifierProvider.notifier)
        .selectedAccountId;
    if (accountId == null) {
      throw Exception('No account selected');
    }

    // Find the actual index by ID - this is stable even if indices shift
    final actualIndex = _findIndexById(state!, section, itemType, id);
    if (actualIndex == -1) {
      return;
    }

    // Verify item exists and is deleted at this index
    final itemAtIndex = ProfileSectionEditor.getItem(
      profile: state!,
      section: section,
      itemType: itemType,
      index: actualIndex,
    );
    if (itemAtIndex == null || !_isItemDeleted(itemAtIndex)) {
      // Item doesn't exist or was already restored - silently return
      return;
    }

    final current = state!;
    final (newProfile, restoredFound) = ProfileSectionEditor.markRestored(
      current: current,
      section: section,
      itemType: itemType,
      index: actualIndex,
    );

    // Log the restore operation
    _logRestore(section, itemType, actualIndex);

    // Save FIRST (immediate to invalidate cache), then update state only on success
    final saved = await saveProfileImmediate(newProfile);
    if (!saved) {
      _addLogEntry(
        section: LogSectionConfig.getLogSection(section, itemType),
        action: LogAction.update,
        description: 'ERROR: Failed to save restore for $itemType',
      );
      return;
    }

    state = newProfile;

    // Update account operation metadata
    await _ref
        .read(authNotifierProvider.notifier)
        .updateOperation('Restored $itemType');
  }

  /// Check if an item is deleted
  bool _isItemDeleted(dynamic item) {
    if (item == null) return false;
    // Check for isDeleted field via reflection-like approach
    // Each data type has isDeleted property
    try {
      return item.isDeleted == true;
    } catch (_) {
      return false;
    }
  }


  /// Permanently delete an item (remove from list completely)
  Future<void> permanentDelete({
    required String section,
    required String itemType,
    required String id,
  }) async {
    if (state == null) {
      throw Exception('No profile loaded');
    }

    final accountId = _ref
        .read(authNotifierProvider.notifier)
        .selectedAccountId;
    if (accountId == null) {
      throw Exception('No account selected');
    }

    // Find the actual index by ID
    final actualIndex = _findIndexById(state!, section, itemType, id);
    if (actualIndex == -1) {
      throw Exception('$itemType not found by id=$id');
    }

    // Verify item exists and is deleted at this index
    final itemAtIndex = ProfileSectionEditor.getItem(
      profile: state!,
      section: section,
      itemType: itemType,
      index: actualIndex,
    );
    if (itemAtIndex == null) {
      throw Exception('$itemType not found at index $actualIndex');
    }
    if (!_isItemDeleted(itemAtIndex)) {
      throw Exception('$itemType is not in trash');
    }

    // Get item label BEFORE deletion (since item will be removed)
    final itemLabel = _getItemLabel(section, itemType, actualIndex);

    final current = state!;
    await _storage.permanentDeleteItem(
      current,
      accountId,
      section,
      itemType,
      actualIndex,
    );

    // Log the permanent delete operation
    _logPermanentDelete(section, itemType, itemLabel);

    // Reload profile to get updated state
    final updatedProfile = await _storage.loadProfile(accountId);
    if (updatedProfile != null) {
      state = updatedProfile;
    }

    // Update account operation metadata
    await _ref
        .read(authNotifierProvider.notifier)
        .updateOperation('Purged $itemType');
  }


  /// Log a restore operation
  void _logRestore(String section, String itemType, int index) {
    // Use centralized config to determine LogSection
    final logSection = LogSectionConfig.getLogSection(section, itemType);

    final description = 'Restored $itemType from trash';
    _addLogEntry(
      section: logSection,
      action: LogAction.restore,
      description: description,
    );
  }

  /// Log a permanent delete operation
  void _logPermanentDelete(String section, String itemType, String itemLabel) {
    // Use centralized config to determine LogSection
    final logSection = LogSectionConfig.getLogSection(section, itemType);

    final description = 'Permanently deleted $itemType: $itemLabel';
    _addLogEntry(
      section: logSection,
      action: LogAction.purge,
      description: description,
    );
  }

  /// Empty all trash (permanent delete all soft-deleted items)
  Future<void> emptyAllTrash() async {
    if (state == null) return;

    final accountId = _ref
        .read(authNotifierProvider.notifier)
        .selectedAccountId;
    if (accountId == null) return;

    // Collect and log all soft-deleted items before purging
    final profile = state!;

    // Travel section
    if (profile.travel != null) {
      for (var i = 0; i < profile.travel!.passports.length; i++) {
        if (profile.travel!.passports[i].isDeleted) {
          _logPermanentDelete(
            'travel',
            'passport',
            profile.travel!.passports[i].country ?? 'Passport',
          );
        }
      }
      for (var i = 0; i < profile.travel!.visas.length; i++) {
        if (profile.travel!.visas[i].isDeleted) {
          _logPermanentDelete(
            'travel',
            'visa',
            profile.travel!.visas[i].country ?? 'Visa',
          );
        }
      }
      for (var i = 0; i < profile.travel!.travelHistory.length; i++) {
        if (profile.travel!.travelHistory[i].isDeleted) {
          _logPermanentDelete(
            'travel',
            'travel_history',
            profile.travel!.travelHistory[i].destination,
          );
        }
      }
    }

    // Financial section
    if (profile.financial != null) {
      for (var i = 0; i < profile.financial!.bankAccounts.length; i++) {
        if (profile.financial!.bankAccounts[i].isDeleted) {
          _logPermanentDelete(
            'financial',
            'bank_account',
            profile.financial!.bankAccounts[i].bankName ?? 'Bank Account',
          );
        }
      }
      for (var i = 0; i < profile.financial!.cards.length; i++) {
        if (profile.financial!.cards[i].isDeleted) {
          _logPermanentDelete(
            'financial',
            'card',
            profile.financial!.cards[i].cardType ?? 'Card',
          );
        }
      }
    }

    // Professional section
    if (profile.professional != null) {
      for (var i = 0; i < profile.professional!.education.length; i++) {
        if (profile.professional!.education[i].isDeleted) {
          _logPermanentDelete(
            'professional',
            'education',
            profile.professional!.education[i].institution ?? 'Education',
          );
        }
      }
      for (var i = 0; i < profile.professional!.employment.length; i++) {
        if (profile.professional!.employment[i].isDeleted) {
          _logPermanentDelete(
            'professional',
            'employment',
            profile.professional!.employment[i].company ?? 'Employment',
          );
        }
      }
      for (var i = 0; i < profile.professional!.skills.length; i++) {
        if (profile.professional!.skills[i].isDeleted) {
          _logPermanentDelete(
            'professional',
            'skill',
            profile.professional!.skills[i].toString(),
          );
        }
      }
      for (var i = 0; i < profile.professional!.languages.length; i++) {
        if (profile.professional!.languages[i].isDeleted) {
          _logPermanentDelete(
            'professional',
            'language',
            profile.professional!.languages[i].toString(),
          );
        }
      }
    }

    // Profile/Identity section
    if (profile.identity?.contact != null) {
      for (var i = 0; i < profile.identity!.contact!.entries.length; i++) {
        if (profile.identity!.contact!.entries[i].isDeleted) {
          final entry = profile.identity!.contact!.entries[i];
          final label = entry.title.isNotEmpty
              ? '${entry.title} - ${entry.value}'
              : entry.value;
          _logPermanentDelete('profile', 'contact', label);
        }
      }
    }
    if (profile.identity?.idCards != null) {
      for (var i = 0; i < profile.identity!.idCards!.length; i++) {
        if (profile.identity!.idCards![i].isDeleted) {
          _logPermanentDelete(
            'profile',
            'idCard',
            profile.identity!.idCards![i].title ??
                profile.identity!.idCards![i].number ??
                'ID Card',
          );
        }
      }
    }
    if (profile.identity?.addresses != null) {
      for (var i = 0; i < profile.identity!.addresses!.length; i++) {
        if (profile.identity!.addresses![i].isDeleted) {
          _logPermanentDelete(
            'profile',
            'address',
            profile.identity!.addresses![i].title ?? 'Address',
          );
        }
      }
    }

    // Purge all soft-deleted items
    await _storage.emptyAllTrash(state!, accountId);

    // Ensure all purge logs are flushed to disk before returning
    await OperationLogService.instance.flush();

    // Reload profile to get updated state
    final updatedProfile = await _storage.loadProfile(accountId);
    if (updatedProfile != null) {
      state = updatedProfile;
    }

    // Update account operation metadata
    await _ref
        .read(authNotifierProvider.notifier)
        .updateOperation('Emptied trash');
  }

  /// Get item label for logging purposes
  String _getItemLabel(String section, String itemType, int index) {
    if (state == null) return 'Unknown';

    switch (section) {
      case 'travel':
        if (itemType == 'passport' &&
            index < (state!.travel?.passports.length ?? 0)) {
          return state!.travel!.passports[index].country ?? 'Passport';
        } else if (itemType == 'visa' &&
            index < (state!.travel?.visas.length ?? 0)) {
          return state!.travel!.visas[index].country ?? 'Visa';
        } else if (itemType == 'travel_history' &&
            index < (state!.travel?.travelHistory.length ?? 0)) {
          return state!.travel!.travelHistory[index].destination;
        }
        break;
      case 'financial':
        if (itemType == 'bank_account' &&
            index < (state!.financial?.bankAccounts.length ?? 0)) {
          return state!.financial!.bankAccounts[index].bankName ??
              'Bank Account';
        } else if (itemType == 'card' &&
            index < (state!.financial?.cards.length ?? 0)) {
          return state!.financial!.cards[index].cardType ?? 'Card';
        }
        break;
      case 'professional':
        if (itemType == 'education' &&
            index < (state!.professional?.education.length ?? 0)) {
          return state!.professional!.education[index].institution ??
              'Education';
        } else if (itemType == 'employment' &&
            index < (state!.professional?.employment.length ?? 0)) {
          return state!.professional!.employment[index].company ?? 'Employment';
        } else if (itemType == 'skill' &&
            index < (state!.professional?.skills.length ?? 0)) {
          return state!.professional!.skills[index].name;
        } else if (itemType == 'language' &&
            index < (state!.professional?.languages.length ?? 0)) {
          return state!.professional!.languages[index].name;
        }
        break;
      case 'profile':
        if (itemType == 'contact' &&
            index < (state!.identity?.contact?.entries.length ?? 0)) {
          final entry = state!.identity!.contact!.entries[index];
          return entry.title.isNotEmpty
              ? '${entry.title} - ${entry.value}'
              : entry.value;
        } else if (itemType == 'idCard' &&
            index < (state!.identity?.idCards?.length ?? 0)) {
          return state!.identity!.idCards![index].title ??
              state!.identity!.idCards![index].number ??
              'ID Card';
        } else if (itemType == 'address' &&
            index < (state!.identity?.addresses?.length ?? 0)) {
          return state!.identity!.addresses![index].title ?? 'Address';
        }
        break;
    }
    return itemType;
  }

  /// Finds index by ID - the primary and reliable method for item location.
  /// Returns -1 if not found.
  int _findIndexById(
    ProfileData current,
    String section,
    String itemType,
    String id,
  ) {
    switch (section) {
      case 'travel':
        final list = current.travel;
        if (list == null) return -1;
        if (itemType == 'passport') {
          return list.passports.indexWhere((p) => p.id == id);
        } else if (itemType == 'visa') {
          return list.visas.indexWhere((v) => v.id == id);
        } else if (itemType == 'travel_history') {
          return list.travelHistory.indexWhere((t) => t.id == id);
        }
        return -1;
      case 'financial':
        final list = current.financial;
        if (list == null) return -1;
        if (itemType == 'bank_account') {
          return list.bankAccounts.indexWhere((b) => b.id == id);
        } else if (itemType == 'card') {
          return list.cards.indexWhere((c) => c.id == id);
        } else if (itemType == 'tax_id') {
          return list.taxIds.indexWhere((t) => t.id == id);
        }
        return -1;
      case 'professional':
        final list = current.professional;
        if (list == null) return -1;
        if (itemType == 'education') {
          return list.education.indexWhere((e) => e.id == id);
        } else if (itemType == 'employment') {
          return list.employment.indexWhere((e) => e.id == id);
        } else if (itemType == 'skill') {
          return list.skills.indexWhere((s) => s.id == id);
        } else if (itemType == 'language') {
          return list.languages.indexWhere((l) => l.id == id);
        }
        return -1;
      case 'profile':
        final list = current.identity;
        if (list == null) return -1;
        if (itemType == 'contact') {
          return list.contact?.entries.indexWhere((e) => e.id == id) ?? -1;
        } else if (itemType == 'idCard') {
          return list.idCards?.indexWhere((c) => c.id == id) ?? -1;
        } else if (itemType == 'address') {
          return list.addresses?.indexWhere((a) => a.id == id) ?? -1;
        }
        return -1;
      default:
        return -1;
    }
  }

}

/// Auto-loading profile provider that loads when auth state is unlocked
final profileNotifierProvider =
    StateNotifierProvider<ProfileNotifier, ProfileData?>((ref) {
      final notifier = ProfileNotifier(ref);

      // Watch auth state and auto-load when unlocked
      ref.listen<AuthState>(authNotifierProvider, (previous, next) {
        if (next == AuthState.unlocked) {
          notifier.loadProfile();
          // Pre-register all form fields so sensitivity settings page has full list
          ref.read(formFieldRegistryProvider.notifier).registerAllForms();
        } else if (previous == AuthState.unlocked &&
                   (next == AuthState.locked || next == AuthState.initial)) {
          // Only clear profile when transitioning FROM unlocked to locked/initial.
          // This prevents clearing profile data on FAILED unlock attempts
          // (which go: unlocked → loading → locked, not locked → unlocked).
          notifier.clearProfile();
        }
      });

      // If already unlocked when provider is first created (e.g., on hot reload)
      final authState = ref.read(authNotifierProvider);
      if (authState == AuthState.unlocked) {
        notifier.loadProfile();
      }

      return notifier;
    });

/// Convenience providers for individual data sections
final identityProvider = Provider<IdentityData?>((ref) {
  final profile = ref.watch(profileNotifierProvider);
  return profile?.identity;
});

final travelProvider = Provider<TravelData?>((ref) {
  final profile = ref.watch(profileNotifierProvider);
  return profile?.travel;
});

final financialProvider = Provider<FinancialData?>((ref) {
  final profile = ref.watch(profileNotifierProvider);
  return profile?.financial;
});

final professionalProvider = Provider<ProfessionalData?>((ref) {
  final profile = ref.watch(profileNotifierProvider);
  return profile?.professional;
});
