import 'package:flutter_riverpod/flutter_riverpod.dart';
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
    } on Exception catch (e, st) {
      state = AsyncError(e, st);
    }
  }

  /// Save profile for the currently unlocked account (debounced)
  Future<bool> saveProfile(ProfileData profile, {bool immediate = false}) async {
    return _persistence.saveProfile(profile, immediate: immediate);
  }

  /// Force an immediate save (bypasses debounce)
  Future<bool> saveProfileImmediate(ProfileData profile) async {
    return _persistence.saveProfileImmediate(profile);
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

/// Convenience providers for individual data sections
final identityProvider = Provider<IdentityData?>((ref) {
  final profile = ref.watch(profileNotifierProvider);
  return profile.value?.identity;
});

final travelProvider = Provider<TravelData?>((ref) {
  final profile = ref.watch(profileNotifierProvider);
  return profile.value?.travel;
});

final financialProvider = Provider<FinancialData?>((ref) {
  final profile = ref.watch(profileNotifierProvider);
  return profile.value?.financial;
});

final professionalProvider = Provider<ProfessionalData?>((ref) {
  final profile = ref.watch(profileNotifierProvider);
  return profile.value?.professional;
});

// =============================================================================
// Section Item Providers (Pilot: reduce ref.read usage)
// =============================================================================

/// Education items provider - derives sorted EducationData from profileNotifierProvider.
final educationItemsProvider = Provider.autoDispose<List<EducationData>>((ref) {
  final profile = ref.watch(profileNotifierProvider);
  final professional = profile.value?.professional;
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
});

int _degreeSortOrder(EducationData e, List<String> degreeOrder) {
  final degree = e.degree ?? '';
  if (e.degreeCustom != null && e.degreeCustom!.isNotEmpty && !degreeOrder.contains(degree)) {
    return -1;
  }
  final index = degreeOrder.indexOf(degree);
  return index >= 0 ? index : degreeOrder.length;
}

/// Bank account items provider
final bankAccountItemsProvider = Provider.autoDispose<List<BankAccountData>>((ref) {
  final profile = ref.watch(profileNotifierProvider);
  final financial = profile.value?.financial;
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
});

/// Employment items provider
final employmentItemsProvider = Provider.autoDispose<List<EmploymentData>>((ref) {
  final profile = ref.watch(profileNotifierProvider);
  final professional = profile.value?.professional;
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
});

/// Skill items provider
final skillItemsProvider = Provider.autoDispose<List<SkillData>>((ref) {
  final profile = ref.watch(profileNotifierProvider);
  final professional = profile.value?.professional;
  if (professional == null) return [];

  return professional.activeSkills.map((s) => SkillData(
    id: s.id,
    name: s.name,
    level: s.level,
    updatedAt: s.updatedAt,
    isDeleted: s.isDeleted,
    deletedAt: s.deletedAt,
  )).toList();
});

/// Tax ID items provider
final taxIdItemsProvider = Provider.autoDispose<List<TaxIdData>>((ref) {
  final profile = ref.watch(profileNotifierProvider);
  final financial = profile.value?.financial;
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
});

/// Passport items provider
final passportItemsProvider = Provider.autoDispose<List<PassportData>>((ref) {
  final profile = ref.watch(profileNotifierProvider);
  final travel = profile.value?.travel;
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
});

/// Visa items provider
final visaItemsProvider = Provider.autoDispose<List<VisaData>>((ref) {
  final profile = ref.watch(profileNotifierProvider);
  final travel = profile.value?.travel;
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
});

/// Travel history items provider
final travelHistoryItemsProvider = Provider.autoDispose<List<TravelHistoryData>>((ref) {
  final profile = ref.watch(profileNotifierProvider);
  final travel = profile.value?.travel;
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
});

/// Card items provider
final cardItemsProvider = Provider.autoDispose<List<CardData>>((ref) {
  final profile = ref.watch(profileNotifierProvider);
  final financial = profile.value?.financial;
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
});

/// Contact items provider
final contactItemsProvider = Provider.autoDispose<List<ContactEntry>>((ref) {
  final profile = ref.watch(profileNotifierProvider);
  final contact = profile.value?.identity?.contact;
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
});

/// Language items provider
final languageItemsProvider = Provider.autoDispose<List<LanguageData>>((ref) {
  final profile = ref.watch(profileNotifierProvider);
  final professional = profile.value?.professional;
  if (professional == null) return [];
  return professional.activeLanguages.toList();
});

/// Award items provider
final awardItemsProvider = Provider.autoDispose<List<AwardData>>((ref) {
  final profile = ref.watch(profileNotifierProvider);
  final professional = profile.value?.professional;
  if (professional == null) return [];
  return professional.activeAwards.toList();
});

/// ID card items provider
final idCardItemsProvider = Provider.autoDispose<List<IdCardData>>((ref) {
  final profile = ref.watch(profileNotifierProvider);
  final identity = profile.value?.identity;
  if (identity == null) return [];
  return identity.activeIdCards.toList();
});

/// Address items provider
final addressItemsProvider = Provider.autoDispose<List<AddressData>>((ref) {
  final profile = ref.watch(profileNotifierProvider);
  final identity = profile.value?.identity;
  if (identity == null) return [];
  return identity.activeAddresses.toList();
});
