import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/services/profile_persistence_notifier.dart';
import 'package:solosoul_flutter/presentation/providers/services/operation_log_aggregator.dart';
import 'package:solosoul_flutter/presentation/providers/services/trash_manager.dart';
import 'package:solosoul_flutter/presentation/providers/services/section_mutators.dart';

// Re-export field history types for backward compatibility
export 'package:solosoul_flutter/core/services/field_history_service.dart'
    show fieldHistoriesProvider, FieldHistoriesNotifier;
export 'package:solosoul_flutter/core/models/field_history_models.dart'
    show FieldHistory, FormHistories;

part 'profile_provider.g.dart';

/// Profile notifier - facade that delegates to specialized services:
/// - ProfilePersistenceService: load/save with debounce
/// - OperationLogAggregator: change detection and summary
/// - TrashManager: soft delete/restore/permanent delete
/// - SectionMutators: domain model updates
class ProfileNotifier extends AsyncNotifier<ProfileData?> {
  late final ProfilePersistenceService _persistence;
  late final OperationLogAggregator _logAggregator;
  late final TrashManager _trashManager;
  late final SectionMutators _sectionMutators;

  @override
  Future<ProfileData?> build() async {
    _logAggregator = OperationLogAggregator();
    _persistence = ProfilePersistenceService(ref);
    _trashManager = TrashManager(ref, _logAggregator, _persistence);
    _sectionMutators = SectionMutators(ref, _logAggregator, _persistence);

    ref.onDispose(() {
      _persistence.dispose();
    });

    return await _loadFromStorage();
  }

  Future<ProfileData?> _loadFromStorage() async {
    final profile = await _persistence.loadProfile();
    return profile;
  }

  bool get isLoading => _persistence.isLoading;

  /// Clear profile state (when auth is locked or reset)
  Future<void> clearProfile() async {
    await _persistence.clearProfile();
    state = const AsyncData(null);
  }

  /// Reload profile (for manual refresh or auth state changes)
  Future<void> loadProfile() async {
    state = const AsyncLoading();
    try {
      final profile = await _loadFromStorage();
      state = AsyncData(profile);
    } on Object catch (e, st) {
      state = AsyncError(e, st);
    }
  }

  /// Save profile for the currently unlocked account (debounced)
  Future<bool> saveProfile(ProfileData profile, {bool immediate = false}) async {
    final result = await _persistence.saveProfile(profile, immediate: immediate);
    if (result) {
      state = AsyncData(profile);
    }
    return result;
  }

  /// Force an immediate save (bypasses debounce)
  Future<bool> saveProfileImmediate(ProfileData profile) async {
    final result = await _persistence.saveProfileImmediate(profile);
    if (result) {
      state = AsyncData(profile);
    }
    return result;
  }

  /// Update identity data
  Future<bool> updateIdentity(IdentityData identity) async {
    final currentProfile = state.value;
    return _sectionMutators.updateIdentity(identity, currentProfile, (p) => state = AsyncData(p));
  }

  /// Add or update travel data
  Future<bool> updateTravel(TravelData travel) async {
    final currentProfile = state.value;
    return _sectionMutators.updateTravel(travel, currentProfile, (p) => state = AsyncData(p));
  }

  /// Update travel data with immediate save (bypasses debounce)
  Future<bool> updateTravelImmediate(TravelData travel) async {
    final currentProfile = state.value;
    return _sectionMutators.updateTravelImmediate(travel, currentProfile, (p) => state = AsyncData(p));
  }

  /// Add or update financial data
  Future<bool> updateFinancial(FinancialData financial) async {
    final currentProfile = state.value;
    return _sectionMutators.updateFinancial(financial, currentProfile, (p) => state = AsyncData(p));
  }

  /// Update financial data with immediate save (bypasses debounce)
  Future<bool> updateFinancialImmediate(FinancialData financial) async {
    final currentProfile = state.value;
    return _sectionMutators.updateFinancialImmediate(financial, currentProfile, (p) => state = AsyncData(p));
  }

  /// Add or update professional data
  Future<bool> updateProfessional(ProfessionalData professional) async {
    final currentProfile = state.value;
    return _sectionMutators.updateProfessional(professional, currentProfile, (p) => state = AsyncData(p));
  }

  /// Update professional data with immediate save (bypasses debounce)
  Future<bool> updateProfessionalImmediate(ProfessionalData professional) async {
    final currentProfile = state.value;
    return _sectionMutators.updateProfessionalImmediate(professional, currentProfile, (p) => state = AsyncData(p));
  }

  /// Soft delete an item (marks as deleted but doesn't remove)
  Future<void> softDelete({
    required String section,
    required String itemType,
    required int index,
    required dynamic deletedItem,
  }) async {
    final currentProfile = state.value;
    if (currentProfile == null) return;

    final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
    if (accountId == null) return;

    final actualIndex = _trashManager.findIndexById(currentProfile, section, itemType, deletedItem.id);
    if (actualIndex < 0) return;

    await _trashManager.softDelete(
      currentProfile: currentProfile,
      section: section,
      itemType: itemType,
      index: actualIndex,
      deletedItem: deletedItem,
      onStateUpdate: (profile) => state = AsyncData(profile),
    );
  }

  /// Restore a soft-deleted item
  Future<void> restore({
    required String section,
    required String itemType,
    required String id,
  }) async {
    final currentProfile = state.value;
    if (currentProfile == null) {
      throw Exception('No profile loaded');
    }

    final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
    if (accountId == null) {
      throw Exception('No account selected');
    }

    await _trashManager.restore(
      currentProfile: currentProfile,
      section: section,
      itemType: itemType,
      id: id,
      onStateUpdate: (profile) => state = AsyncData(profile),
    );
  }

  /// Permanently delete an item (remove from list completely)
  Future<void> permanentDelete({
    required String section,
    required String itemType,
    required String id,
  }) async {
    final currentProfile = state.value;
    if (currentProfile == null) {
      throw Exception('No profile loaded');
    }

    final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
    if (accountId == null) {
      throw Exception('No account selected');
    }

    await _trashManager.permanentDelete(
      currentProfile: currentProfile,
      section: section,
      itemType: itemType,
      id: id,
      onStateUpdate: (profile) => state = AsyncData(profile),
    );
  }

  /// Empty all trash (permanent delete all soft-deleted items)
  Future<void> emptyAllTrash() async {
    final currentProfile = state.value;
    if (currentProfile == null) return;

    final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
    if (accountId == null) return;

    await _trashManager.emptyAllTrash(
      currentProfile: currentProfile,
      onStateUpdate: (profile) => state = AsyncData(profile),
    );
  }
}

/// Auto-loading profile provider that loads when auth state is unlocked
final profileNotifierProvider = AsyncNotifierProvider<ProfileNotifier, ProfileData?>(() {
  return ProfileNotifier();
});

// =============================================================================
// Section Providers (converted to @riverpod)
// =============================================================================

/// Identity section provider
@riverpod
class ProfileIdentity extends _$ProfileIdentity {
  @override
  IdentityData? build() {
    final identity = ref.watch(
      profileNotifierProvider.select((p) => p.value?.identity),
    );
    return identity;
  }
}

/// Travel section provider
@riverpod
class ProfileTravel extends _$ProfileTravel {
  @override
  TravelData? build() {
    final travel = ref.watch(
      profileNotifierProvider.select((p) => p.value?.travel),
    );
    return travel;
  }
}

/// Financial section provider
@riverpod
class ProfileFinancial extends _$ProfileFinancial {
  @override
  FinancialData? build() {
    final financial = ref.watch(
      profileNotifierProvider.select((p) => p.value?.financial),
    );
    return financial;
  }
}

/// Professional section provider
@riverpod
class ProfileProfessional extends _$ProfileProfessional {
  @override
  ProfessionalData? build() {
    final professional = ref.watch(
      profileNotifierProvider.select((p) => p.value?.professional),
    );
    return professional;
  }
}

// =============================================================================
// Section Item Providers (Pilot: reduce ref.read usage)
// =============================================================================

int _degreeSortOrder(EducationData e, List<String> degreeOrder) {
  final degree = e.degree ?? '';
  final degreeCustom = e.degreeCustom;
  if (degreeCustom != null && degreeCustom.isNotEmpty && !degreeOrder.contains(degree)) {
    return -1;
  }
  final index = degreeOrder.indexOf(degree);
  return index >= 0 ? index : degreeOrder.length;
}

/// Education items provider - derives sorted EducationData from profileNotifierProvider.
@riverpod
class EducationItems extends _$EducationItems {
  @override
  List<EducationData> build() {
    final education = ref.watch(
      profileNotifierProvider.select((p) => p.value?.professional?.activeEducation),
    );
    if (education == null || education.isEmpty) return const [];

    final sorted = [...education];
    const degreeOrder = ['PhD', 'Master', 'Bachelor', 'Senior High', 'Junior High', 'Elementary'];
    sorted.sort((a, b) {
      final aOrder = _degreeSortOrder(a, degreeOrder);
      final bOrder = _degreeSortOrder(b, degreeOrder);
      return aOrder.compareTo(bOrder);
    });

    return sorted;
  }
}

/// Bank account items provider
@riverpod
class BankAccountItems extends _$BankAccountItems {
  @override
  List<BankAccountData> build() {
    final accounts = ref.watch(
      profileNotifierProvider.select((p) => p.value?.financial?.activeBankAccounts),
    );
    return accounts ?? const [];
  }
}

/// Employment items provider
@riverpod
class EmploymentItems extends _$EmploymentItems {
  @override
  List<EmploymentData> build() {
    final employment = ref.watch(
      profileNotifierProvider.select((p) => p.value?.professional?.activeEmployment),
    );
    return employment ?? const [];
  }
}

/// Skill items provider
@riverpod
class SkillItems extends _$SkillItems {
  @override
  List<SkillData> build() {
    final skills = ref.watch(
      profileNotifierProvider.select((p) => p.value?.professional?.activeSkills),
    );
    return skills ?? const [];
  }
}

/// Tax ID items provider
@riverpod
class TaxIdItems extends _$TaxIdItems {
  @override
  List<TaxIdData> build() {
    final taxIds = ref.watch(
      profileNotifierProvider.select((p) => p.value?.financial?.activeTaxIds),
    );
    return taxIds ?? const [];
  }
}

/// Passport items provider
@riverpod
class PassportItems extends _$PassportItems {
  @override
  List<PassportData> build() {
    final passports = ref.watch(
      profileNotifierProvider.select((p) => p.value?.travel?.activePassports),
    );
    return passports ?? const [];
  }
}

/// Visa items provider
@riverpod
class VisaItems extends _$VisaItems {
  @override
  List<VisaData> build() {
    final visas = ref.watch(
      profileNotifierProvider.select((p) => p.value?.travel?.activeVisas),
    );
    return visas ?? const [];
  }
}

/// Travel history items provider
@riverpod
class TravelHistoryItems extends _$TravelHistoryItems {
  @override
  List<TravelHistoryData> build() {
    final history = ref.watch(
      profileNotifierProvider.select((p) => p.value?.travel?.activeTravelHistory),
    );
    return history ?? const [];
  }
}

/// Card items provider
@riverpod
class CardItems extends _$CardItems {
  @override
  List<CardData> build() {
    final cards = ref.watch(
      profileNotifierProvider.select((p) => p.value?.financial?.activeCards),
    );
    return cards ?? const [];
  }
}

/// Contact items provider
@riverpod
class ContactItems extends _$ContactItems {
  @override
  List<ContactEntry> build() {
    final entries = ref.watch(
      profileNotifierProvider.select((p) => p.value?.identity?.contact?.activeEntries),
    );
    return entries ?? const [];
  }
}

/// Language items provider
@riverpod
class LanguageItems extends _$LanguageItems {
  @override
  List<LanguageData> build() {
    final languages = ref.watch(
      profileNotifierProvider.select((p) => p.value?.professional?.activeLanguages),
    );
    return languages ?? const [];
  }
}

/// Award items provider
@riverpod
class AwardItems extends _$AwardItems {
  @override
  List<AwardData> build() {
    final awards = ref.watch(
      profileNotifierProvider.select((p) => p.value?.professional?.activeAwards),
    );
    return awards ?? const [];
  }
}

/// ID card items provider
@riverpod
class IdCardItems extends _$IdCardItems {
  @override
  List<IdCardData> build() {
    final idCards = ref.watch(
      profileNotifierProvider.select((p) => p.value?.identity?.activeIdCards),
    );
    return idCards ?? const [];
  }
}

/// Address items provider
@riverpod
class AddressItems extends _$AddressItems {
  @override
  List<AddressData> build() {
    final addresses = ref.watch(
      profileNotifierProvider.select((p) => p.value?.identity?.activeAddresses),
    );
    return addresses ?? const [];
  }
}
