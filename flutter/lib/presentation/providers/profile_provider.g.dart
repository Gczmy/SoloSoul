// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'profile_provider.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

String _$profileIdentityHash() => r'f9ee523f55d247909309fc99cb0fcf83b2c26d48';

/// Identity section provider
///
/// Copied from [ProfileIdentity].
@ProviderFor(ProfileIdentity)
final profileIdentityProvider =
    AutoDisposeNotifierProvider<ProfileIdentity, IdentityData?>.internal(
      ProfileIdentity.new,
      name: r'profileIdentityProvider',
      debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
          ? null
          : _$profileIdentityHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

typedef _$ProfileIdentity = AutoDisposeNotifier<IdentityData?>;
String _$profileTravelHash() => r'6102ea79d88174f93e81447a9572c49fc1eb8e8c';

/// Travel section provider
///
/// Copied from [ProfileTravel].
@ProviderFor(ProfileTravel)
final profileTravelProvider =
    AutoDisposeNotifierProvider<ProfileTravel, TravelData?>.internal(
      ProfileTravel.new,
      name: r'profileTravelProvider',
      debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
          ? null
          : _$profileTravelHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

typedef _$ProfileTravel = AutoDisposeNotifier<TravelData?>;
String _$profileFinancialHash() => r'6f867402030e0464eff2dac25e6c3352447c5f77';

/// Financial section provider
///
/// Copied from [ProfileFinancial].
@ProviderFor(ProfileFinancial)
final profileFinancialProvider =
    AutoDisposeNotifierProvider<ProfileFinancial, FinancialData?>.internal(
      ProfileFinancial.new,
      name: r'profileFinancialProvider',
      debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
          ? null
          : _$profileFinancialHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

typedef _$ProfileFinancial = AutoDisposeNotifier<FinancialData?>;
String _$profileProfessionalHash() =>
    r'e6afa824dc6ba8a40299357de3e7d0e8269b0e3f';

/// Professional section provider
///
/// Copied from [ProfileProfessional].
@ProviderFor(ProfileProfessional)
final profileProfessionalProvider =
    AutoDisposeNotifierProvider<
      ProfileProfessional,
      ProfessionalData?
    >.internal(
      ProfileProfessional.new,
      name: r'profileProfessionalProvider',
      debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
          ? null
          : _$profileProfessionalHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

typedef _$ProfileProfessional = AutoDisposeNotifier<ProfessionalData?>;
String _$educationItemsHash() => r'f32830cafab9b2e32ffaf7752619468740f48275';

/// Education items provider - derives sorted EducationData from profileNotifierProvider.
///
/// Copied from [EducationItems].
@ProviderFor(EducationItems)
final educationItemsProvider =
    AutoDisposeNotifierProvider<EducationItems, List<EducationData>>.internal(
      EducationItems.new,
      name: r'educationItemsProvider',
      debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
          ? null
          : _$educationItemsHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

typedef _$EducationItems = AutoDisposeNotifier<List<EducationData>>;
String _$bankAccountItemsHash() => r'a698ddd4c21884e35ee10ab104f5f0e337415033';

/// Bank account items provider
///
/// Copied from [BankAccountItems].
@ProviderFor(BankAccountItems)
final bankAccountItemsProvider =
    AutoDisposeNotifierProvider<
      BankAccountItems,
      List<BankAccountData>
    >.internal(
      BankAccountItems.new,
      name: r'bankAccountItemsProvider',
      debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
          ? null
          : _$bankAccountItemsHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

typedef _$BankAccountItems = AutoDisposeNotifier<List<BankAccountData>>;
String _$employmentItemsHash() => r'3b7636caed5b88d38a29119609f165078eb0dfbf';

/// Employment items provider
///
/// Copied from [EmploymentItems].
@ProviderFor(EmploymentItems)
final employmentItemsProvider =
    AutoDisposeNotifierProvider<EmploymentItems, List<EmploymentData>>.internal(
      EmploymentItems.new,
      name: r'employmentItemsProvider',
      debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
          ? null
          : _$employmentItemsHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

typedef _$EmploymentItems = AutoDisposeNotifier<List<EmploymentData>>;
String _$skillItemsHash() => r'1e7c9e000684fa7522195d94d502907d969cb0e9';

/// Skill items provider
///
/// Copied from [SkillItems].
@ProviderFor(SkillItems)
final skillItemsProvider =
    AutoDisposeNotifierProvider<SkillItems, List<SkillData>>.internal(
      SkillItems.new,
      name: r'skillItemsProvider',
      debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
          ? null
          : _$skillItemsHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

typedef _$SkillItems = AutoDisposeNotifier<List<SkillData>>;
String _$taxIdItemsHash() => r'53c9cba0d641fd562cc540fd586aea05f1eb008c';

/// Tax ID items provider
///
/// Copied from [TaxIdItems].
@ProviderFor(TaxIdItems)
final taxIdItemsProvider =
    AutoDisposeNotifierProvider<TaxIdItems, List<TaxIdData>>.internal(
      TaxIdItems.new,
      name: r'taxIdItemsProvider',
      debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
          ? null
          : _$taxIdItemsHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

typedef _$TaxIdItems = AutoDisposeNotifier<List<TaxIdData>>;
String _$passportItemsHash() => r'15ae06710a083b5c733bb4bae7431ce254b583ae';

/// Passport items provider
///
/// Copied from [PassportItems].
@ProviderFor(PassportItems)
final passportItemsProvider =
    AutoDisposeNotifierProvider<PassportItems, List<PassportData>>.internal(
      PassportItems.new,
      name: r'passportItemsProvider',
      debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
          ? null
          : _$passportItemsHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

typedef _$PassportItems = AutoDisposeNotifier<List<PassportData>>;
String _$visaItemsHash() => r'20bb9d2611487139bc3a401d1fa3cc9e4d82e7d7';

/// Visa items provider
///
/// Copied from [VisaItems].
@ProviderFor(VisaItems)
final visaItemsProvider =
    AutoDisposeNotifierProvider<VisaItems, List<VisaData>>.internal(
      VisaItems.new,
      name: r'visaItemsProvider',
      debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
          ? null
          : _$visaItemsHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

typedef _$VisaItems = AutoDisposeNotifier<List<VisaData>>;
String _$travelHistoryItemsHash() =>
    r'cdec59130fd93dfc5381de0fef6ac73a1f053d05';

/// Travel history items provider
///
/// Copied from [TravelHistoryItems].
@ProviderFor(TravelHistoryItems)
final travelHistoryItemsProvider =
    AutoDisposeNotifierProvider<
      TravelHistoryItems,
      List<TravelHistoryData>
    >.internal(
      TravelHistoryItems.new,
      name: r'travelHistoryItemsProvider',
      debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
          ? null
          : _$travelHistoryItemsHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

typedef _$TravelHistoryItems = AutoDisposeNotifier<List<TravelHistoryData>>;
String _$cardItemsHash() => r'dda1cf907ee4f1fe1650cd85b8b646affdffc744';

/// Card items provider
///
/// Copied from [CardItems].
@ProviderFor(CardItems)
final cardItemsProvider =
    AutoDisposeNotifierProvider<CardItems, List<CardData>>.internal(
      CardItems.new,
      name: r'cardItemsProvider',
      debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
          ? null
          : _$cardItemsHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

typedef _$CardItems = AutoDisposeNotifier<List<CardData>>;
String _$contactItemsHash() => r'688c5d062c0b254c679d9c28d9d4dd8bb6a06f63';

/// Contact items provider
///
/// Copied from [ContactItems].
@ProviderFor(ContactItems)
final contactItemsProvider =
    AutoDisposeNotifierProvider<ContactItems, List<ContactEntry>>.internal(
      ContactItems.new,
      name: r'contactItemsProvider',
      debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
          ? null
          : _$contactItemsHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

typedef _$ContactItems = AutoDisposeNotifier<List<ContactEntry>>;
String _$languageItemsHash() => r'98f70e70f82f87425f478183b11c3ef3d40b51e0';

/// Language items provider
///
/// Copied from [LanguageItems].
@ProviderFor(LanguageItems)
final languageItemsProvider =
    AutoDisposeNotifierProvider<LanguageItems, List<LanguageData>>.internal(
      LanguageItems.new,
      name: r'languageItemsProvider',
      debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
          ? null
          : _$languageItemsHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

typedef _$LanguageItems = AutoDisposeNotifier<List<LanguageData>>;
String _$awardItemsHash() => r'59aa035bad5b95c2723b14b0e859b1e6769015da';

/// Award items provider
///
/// Copied from [AwardItems].
@ProviderFor(AwardItems)
final awardItemsProvider =
    AutoDisposeNotifierProvider<AwardItems, List<AwardData>>.internal(
      AwardItems.new,
      name: r'awardItemsProvider',
      debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
          ? null
          : _$awardItemsHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

typedef _$AwardItems = AutoDisposeNotifier<List<AwardData>>;
String _$idCardItemsHash() => r'59d9e16129af5640c779e0aa1a32cdf4d7e546c0';

/// ID card items provider
///
/// Copied from [IdCardItems].
@ProviderFor(IdCardItems)
final idCardItemsProvider =
    AutoDisposeNotifierProvider<IdCardItems, List<IdCardData>>.internal(
      IdCardItems.new,
      name: r'idCardItemsProvider',
      debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
          ? null
          : _$idCardItemsHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

typedef _$IdCardItems = AutoDisposeNotifier<List<IdCardData>>;
String _$addressItemsHash() => r'1f3d7b79e02896eb86467ea2f7b93c1be19f7d9b';

/// Address items provider
///
/// Copied from [AddressItems].
@ProviderFor(AddressItems)
final addressItemsProvider =
    AutoDisposeNotifierProvider<AddressItems, List<AddressData>>.internal(
      AddressItems.new,
      name: r'addressItemsProvider',
      debugGetCreateSourceHash: const bool.fromEnvironment('dart.vm.product')
          ? null
          : _$addressItemsHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

typedef _$AddressItems = AutoDisposeNotifier<List<AddressData>>;
// ignore_for_file: type=lint
// ignore_for_file: subtype_of_sealed_class, invalid_use_of_internal_member, invalid_use_of_visible_for_testing_member, deprecated_member_use_from_same_package
