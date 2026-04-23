import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/core/services/log_section_config.dart';
import 'package:solosoul_flutter/presentation/pages/operation_log_page.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/profile_section_editor.dart';
import 'package:solosoul_flutter/presentation/providers/services/operation_log_aggregator.dart';
import 'package:solosoul_flutter/presentation/providers/services/profile_persistence_notifier.dart';

/// Service responsible for trash management (soft delete, restore, permanent delete).
/// This class handles:
/// - Soft delete (marking items as deleted)
/// - Restore (undoing soft delete)
/// - Permanent delete (removing items completely)
/// - Empty all trash (garbage collection)
class TrashManager {
  final Ref _ref;
  final ProfileStorageService _storage = ProfileStorageService.instance;
  final OperationLogAggregator _logAggregator;
  final ProfilePersistenceService _persistence;

  TrashManager(this._ref, this._logAggregator, this._persistence);

  /// Check if an item is deleted
  bool isItemDeleted(dynamic item) {
    if (item == null) return false;
    try {
      return item.isDeleted == true;
    } on Exception catch (_) {
      return false;
    }
  }

  /// Soft delete an item (marks as deleted but doesn't remove)
  Future<void> softDelete({
    required ProfileData currentProfile,
    required String section,
    required String itemType,
    required int index,
    required dynamic deletedItem,
    required void Function(ProfileData) onStateUpdate,
  }) async {
    final now = DateTime.now();

    final (newProfile, deletedFound) = ProfileSectionEditor.markDeleted(
      current: currentProfile,
      section: section,
      itemType: itemType,
      index: index,
      deletedAt: now,
    );

    // Log the delete operation
    _logSoftDelete(section, itemType, deletedItem);

    // Save FIRST (immediate to invalidate cache), then update state only on success
    final saved = await _persistence.saveProfile(newProfile, immediate: true);
    if (!saved) {
      _logAggregator.addLogEntry(
        section: LogSectionConfig.getLogSection(section, itemType),
        action: LogAction.update,
        description: 'ERROR: Failed to save soft delete for $itemType',
      );
      return;
    }

    onStateUpdate(newProfile);

    // Update account operation metadata
    await _ref
        .read(authNotifierProvider.notifier)
        .updateOperation('Deleted $itemType');
  }

  /// Log a soft delete operation
  void _logSoftDelete(String section, String itemType, dynamic deletedItem) {
    final logSection = LogSectionConfig.getLogSection(section, itemType);
    final itemLabel = LogSectionConfig.getItemLabel(
      section,
      itemType,
      deletedItem,
    );

    _logAggregator.addLogEntry(
      section: logSection,
      action: LogAction.delete,
      description: 'Moved $itemType to trash: $itemLabel',
    );
  }

  /// Restore a soft-deleted item
  Future<void> restore({
    required ProfileData currentProfile,
    required String section,
    required String itemType,
    required String id,
    required void Function(ProfileData) onStateUpdate,
  }) async {
    // Find the actual index by ID - this is stable even if indices shift
    final actualIndex = findIndexById(currentProfile, section, itemType, id);
    if (actualIndex == -1) return;

    // Verify item exists and is deleted at this index
    final itemAtIndex = ProfileSectionEditor.getItem(
      profile: currentProfile,
      section: section,
      itemType: itemType,
      index: actualIndex,
    );
    if (itemAtIndex == null || !isItemDeleted(itemAtIndex)) {
      return;
    }

    final (newProfile, restoredFound) = ProfileSectionEditor.markRestored(
      current: currentProfile,
      section: section,
      itemType: itemType,
      index: actualIndex,
    );

    // Log the restore operation
    _logRestore(section, itemType, actualIndex);

    // Save FIRST (immediate to invalidate cache), then update state only on success
    final saved = await _persistence.saveProfile(newProfile, immediate: true);
    if (!saved) {
      _logAggregator.addLogEntry(
        section: LogSectionConfig.getLogSection(section, itemType),
        action: LogAction.update,
        description: 'ERROR: Failed to save restore for $itemType',
      );
      return;
    }

    onStateUpdate(newProfile);

    // Update account operation metadata
    await _ref
        .read(authNotifierProvider.notifier)
        .updateOperation('Restored $itemType');
  }

  /// Log a restore operation
  void _logRestore(String section, String itemType, int index) {
    final logSection = LogSectionConfig.getLogSection(section, itemType);

    final description = 'Restored $itemType from trash';
    _logAggregator.addLogEntry(
      section: logSection,
      action: LogAction.restore,
      description: description,
    );
  }

  /// Permanently delete an item (remove from list completely)
  Future<void> permanentDelete({
    required ProfileData currentProfile,
    required String section,
    required String itemType,
    required String id,
    required void Function(ProfileData) onStateUpdate,
  }) async {
    final accountId = _ref.read(authNotifierProvider.notifier).selectedAccountId;
    if (accountId == null) throw Exception('No account selected');

    // Find the actual index by ID
    final actualIndex = findIndexById(currentProfile, section, itemType, id);
    if (actualIndex == -1) {
      throw Exception('$itemType not found by id=$id');
    }

    // Verify item exists and is deleted at this index
    final itemAtIndex = ProfileSectionEditor.getItem(
      profile: currentProfile,
      section: section,
      itemType: itemType,
      index: actualIndex,
    );
    if (itemAtIndex == null) {
      throw Exception('$itemType not found at index $actualIndex');
    }
    if (!isItemDeleted(itemAtIndex)) {
      throw Exception('$itemType is not in trash');
    }

    // Get item label BEFORE deletion (since item will be removed)
    final itemLabel = _getItemLabel(currentProfile, section, itemType, actualIndex);

    await _storage.permanentDeleteItem(
      currentProfile,
      accountId,
      section,
      itemType,
      actualIndex,
    );

    // Log the permanent delete operation
    _logPermanentDelete(section, itemType, itemLabel);

    // Reload profile to get updated state
    final updatedProfile = await _persistence.reloadProfile(accountId);
    if (updatedProfile != null) {
      onStateUpdate(updatedProfile);
    }

    // Update account operation metadata
    await _ref
        .read(authNotifierProvider.notifier)
        .updateOperation('Purged $itemType');
  }

  /// Log a permanent delete operation
  void _logPermanentDelete(String section, String itemType, String itemLabel) {
    final logSection = LogSectionConfig.getLogSection(section, itemType);

    final description = 'Permanently deleted $itemType: $itemLabel';
    _logAggregator.addLogEntry(
      section: logSection,
      action: LogAction.purge,
      description: description,
    );
  }

  /// Empty all trash (permanent delete all soft-deleted items)
  Future<void> emptyAllTrash({
    required ProfileData currentProfile,
    required void Function(ProfileData) onStateUpdate,
  }) async {
    final accountId = _ref.read(authNotifierProvider.notifier).selectedAccountId;
    if (accountId == null) return;

    final profile = currentProfile;

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
    await _storage.emptyAllTrash(profile, accountId);

    // Ensure all purge logs are flushed to disk before returning
    await OperationLogService.instance.flush();

    // Reload profile to get updated state
    final updatedProfile = await _persistence.reloadProfile(accountId);
    if (updatedProfile != null) {
      onStateUpdate(updatedProfile);
    }

    // Update account operation metadata
    await _ref
        .read(authNotifierProvider.notifier)
        .updateOperation('Emptied trash');
  }

  /// Get item label for logging purposes
  String _getItemLabel(ProfileData? profile, String section, String itemType, int index) {
    if (profile == null) return 'Unknown';

    switch (section) {
      case 'travel':
        if (itemType == 'passport' &&
            index < (profile.travel?.passports.length ?? 0)) {
          return profile.travel!.passports[index].country ?? 'Passport';
        } else if (itemType == 'visa' &&
            index < (profile.travel?.visas.length ?? 0)) {
          return profile.travel!.visas[index].country ?? 'Visa';
        } else if (itemType == 'travel_history' &&
            index < (profile.travel?.travelHistory.length ?? 0)) {
          return profile.travel!.travelHistory[index].destination;
        }
        break;
      case 'financial':
        if (itemType == 'bank_account' &&
            index < (profile.financial?.bankAccounts.length ?? 0)) {
          return profile.financial!.bankAccounts[index].bankName ?? 'Bank Account';
        } else if (itemType == 'card' &&
            index < (profile.financial?.cards.length ?? 0)) {
          return profile.financial!.cards[index].cardType ?? 'Card';
        }
        break;
      case 'professional':
        if (itemType == 'education' &&
            index < (profile.professional?.education.length ?? 0)) {
          return profile.professional!.education[index].institution ?? 'Education';
        } else if (itemType == 'employment' &&
            index < (profile.professional?.employment.length ?? 0)) {
          return profile.professional!.employment[index].company ?? 'Employment';
        } else if (itemType == 'skill' &&
            index < (profile.professional?.skills.length ?? 0)) {
          return profile.professional!.skills[index].name;
        } else if (itemType == 'language' &&
            index < (profile.professional?.languages.length ?? 0)) {
          return profile.professional!.languages[index].name;
        }
        break;
      case 'profile':
        if (itemType == 'contact' &&
            index < (profile.identity?.contact?.entries.length ?? 0)) {
          final entry = profile.identity!.contact!.entries[index];
          return entry.title.isNotEmpty
              ? '${entry.title} - ${entry.value}'
              : entry.value;
        } else if (itemType == 'idCard' &&
            index < (profile.identity?.idCards?.length ?? 0)) {
          return profile.identity!.idCards![index].title ??
              profile.identity!.idCards![index].number ??
              'ID Card';
        } else if (itemType == 'address' &&
            index < (profile.identity?.addresses?.length ?? 0)) {
          return profile.identity!.addresses![index].title ?? 'Address';
        }
        break;
    }
    return itemType;
  }

  /// Finds index by ID - the primary and reliable method for item location.
  /// Returns -1 if not found.
  int findIndexById(
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
