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
  if (e.degreeCustom != null && e.degreeCustom!.isNotEmpty && !degreeOrder.contains(degree)) {
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
    final professional = ref.watch(profileNotifierProvider.select((p) => p.value?.professional));
    if (professional == null) return [];

    final items = professional.activeEducation.map((e) => EducationData(
      id: e.id,
      institution: e.institution,
      degree: e.degree,
      degreeCustom: e.degreeCustom,
      field: e.field,
      startDate: e.startDate,
      endDate: e.endDate,
      updatedAt: e.updatedAt,
      isDeleted: e.isDeleted,
      deletedAt: e.deletedAt,
    )).toList();

    const degreeOrder = ['PhD', 'Master', 'Bachelor', 'Senior High', 'Junior High', 'Elementary'];
    items.sort((a, b) {
      final aOrder = _degreeSortOrder(a, degreeOrder);
      final bOrder = _degreeSortOrder(b, degreeOrder);
      return aOrder.compareTo(bOrder);
    });

    return items;
  }
}

/// Bank account items provider
@riverpod
class BankAccountItems extends _$BankAccountItems {
  @override
  List<BankAccountData> build() {
    final financial = ref.watch(profileNotifierProvider.select((p) => p.value?.financial));
    if (financial == null) return [];

    return financial.activeBankAccounts.map((b) => BankAccountData(
      id: b.id,
      title: b.title,
      bankName: b.bankName,
      accountNumber: b.accountNumber,
      currency: b.currency,
      swiftBic: b.swiftBic,
      sortCode: b.sortCode,
      updatedAt: b.updatedAt,
      isDeleted: b.isDeleted,
      deletedAt: b.deletedAt,
    )).toList();
  }
}

/// Employment items provider
@riverpod
class EmploymentItems extends _$EmploymentItems {
  @override
  List<EmploymentData> build() {
    final professional = ref.watch(profileNotifierProvider.select((p) => p.value?.professional));
    if (professional == null) return [];

    return professional.activeEmployment.map((e) => EmploymentData(
      id: e.id,
      company: e.company,
      position: e.position,
      responsibilities: e.responsibilities,
      startDate: e.startDate,
      endDate: e.endDate,
      updatedAt: e.updatedAt,
      isDeleted: e.isDeleted,
      deletedAt: e.deletedAt,
    )).toList();
  }
}

/// Skill items provider
@riverpod
class SkillItems extends _$SkillItems {
  @override
  List<SkillData> build() {
    final professional = ref.watch(profileNotifierProvider.select((p) => p.value?.professional));
    if (professional == null) return [];

    return professional.activeSkills.map((s) => SkillData(
      id: s.id,
      name: s.name,
      level: s.level,
      updatedAt: s.updatedAt,
      isDeleted: s.isDeleted,
      deletedAt: s.deletedAt,
    )).toList();
  }
}

/// Tax ID items provider
@riverpod
class TaxIdItems extends _$TaxIdItems {
  @override
  List<TaxIdData> build() {
    final financial = ref.watch(profileNotifierProvider.select((p) => p.value?.financial));
    if (financial == null) return [];

    return financial.activeTaxIds.map((t) => TaxIdData(
      id: t.id,
      title: t.title,
      taxIdNumber: t.taxIdNumber,
      taxIdType: t.taxIdType,
      issuingAuthority: t.issuingAuthority,
      country: t.country,
      updatedAt: t.updatedAt,
      isDeleted: t.isDeleted,
      deletedAt: t.deletedAt,
    )).toList();
  }
}

/// Passport items provider
@riverpod
class PassportItems extends _$PassportItems {
  @override
  List<PassportData> build() {
    final travel = ref.watch(profileNotifierProvider.select((p) => p.value?.travel));
    if (travel == null) return [];

    return travel.activePassports.map((p) => PassportData(
      id: p.id,
      title: p.title,
      number: p.number,
      country: p.country,
      countryCode: p.countryCode,
      issueDate: p.issueDate,
      placeOfIssue: p.placeOfIssue,
      expiryDate: p.expiryDate,
      dateOfBirth: p.dateOfBirth,
      placeOfBirth: p.placeOfBirth,
      sex: p.sex,
      nationality: p.nationality,
      authority: p.authority,
      holderName: p.holderName,
      updatedAt: p.updatedAt,
      isDeleted: p.isDeleted,
      deletedAt: p.deletedAt,
    )).toList();
  }
}

/// Visa items provider
@riverpod
class VisaItems extends _$VisaItems {
  @override
  List<VisaData> build() {
    final travel = ref.watch(profileNotifierProvider.select((p) => p.value?.travel));
    if (travel == null) return [];

    return travel.activeVisas.map((v) => VisaData(
      id: v.id,
      title: v.title,
      country: v.country,
      visaType: v.visaType,
      number: v.number,
      issueDate: v.issueDate,
      expiryDate: v.expiryDate,
      updatedAt: v.updatedAt,
      isDeleted: v.isDeleted,
      deletedAt: v.deletedAt,
    )).toList();
  }
}

/// Travel history items provider
@riverpod
class TravelHistoryItems extends _$TravelHistoryItems {
  @override
  List<TravelHistoryData> build() {
    final travel = ref.watch(profileNotifierProvider.select((p) => p.value?.travel));
    if (travel == null) return [];

    return travel.activeTravelHistory.map((t) => TravelHistoryData(
      id: t.id,
      destination: t.destination,
      date: t.date,
      departureCity: t.departureCity,
      departureTime: t.departureTime,
      arrivalTime: t.arrivalTime,
      flightNumber: t.flightNumber,
      ticketPrice: t.ticketPrice,
      airline: t.airline,
      travelType: t.travelType,
      updatedAt: t.updatedAt,
      isDeleted: t.isDeleted,
      deletedAt: t.deletedAt,
    )).toList();
  }
}

/// Card items provider
@riverpod
class CardItems extends _$CardItems {
  @override
  List<CardData> build() {
    final financial = ref.watch(profileNotifierProvider.select((p) => p.value?.financial));
    if (financial == null) return [];

    return financial.activeCards.map((c) => CardData(
      id: c.id,
      title: c.title,
      cardNumber: c.cardNumber,
      cardType: c.cardType,
      expiryDate: c.expiryDate,
      holderName: c.holderName,
      cvv: c.cvv,
      updatedAt: c.updatedAt,
      isDeleted: c.isDeleted,
      deletedAt: c.deletedAt,
    )).toList();
  }
}

/// Contact items provider
@riverpod
class ContactItems extends _$ContactItems {
  @override
  List<ContactEntry> build() {
    final contact = ref.watch(profileNotifierProvider.select((p) => p.value?.identity?.contact));
    if (contact == null) return [];

    return contact.activeEntries.map((e) => ContactEntry(
      id: e.id,
      title: e.title,
      type: e.type,
      value: e.value,
      updatedAt: e.updatedAt,
      isDeleted: e.isDeleted,
      deletedAt: e.deletedAt,
    )).toList();
  }
}

/// Language items provider
@riverpod
class LanguageItems extends _$LanguageItems {
  @override
  List<LanguageData> build() {
    final professional = ref.watch(profileNotifierProvider.select((p) => p.value?.professional));
    if (professional == null) return [];
    return professional.activeLanguages.toList();
  }
}

/// Award items provider
@riverpod
class AwardItems extends _$AwardItems {
  @override
  List<AwardData> build() {
    final professional = ref.watch(profileNotifierProvider.select((p) => p.value?.professional));
    if (professional == null) return [];
    return professional.activeAwards.toList();
  }
}

/// ID card items provider
@riverpod
class IdCardItems extends _$IdCardItems {
  @override
  List<IdCardData> build() {
    final identity = ref.watch(profileNotifierProvider.select((p) => p.value?.identity));
    if (identity == null) return [];
    return identity.activeIdCards.toList();
  }
}

/// Address items provider
@riverpod
class AddressItems extends _$AddressItems {
  @override
  List<AddressData> build() {
    final identity = ref.watch(profileNotifierProvider.select((p) => p.value?.identity));
    if (identity == null) return [];
    return identity.activeAddresses.toList();
  }
}
