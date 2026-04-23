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
String _$educationItemsHash() => r'd35a2da739f6add6644373e9c8bb5a90d74c3dd6';

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
String _$bankAccountItemsHash() => r'463cd6415af68ba01457eb4a27d12f3f1654fd03';

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
String _$employmentItemsHash() => r'45027db71ce11b07141e7b8bf6e6f5ee8da43d66';

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
String _$skillItemsHash() => r'7232495484d4cec4429106fa83d0c418519b242b';

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
String _$taxIdItemsHash() => r'097d5870278148cf47bebfde558a0061f17821e5';

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
String _$passportItemsHash() => r'78f4ace3fb4e47cd72d693d46604050d2efc0c7c';

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
String _$visaItemsHash() => r'dc5c8b2d28d2d0844e097abe22310b8cf7b84580';

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
    r'87e120b7abdd825b4192a3a1217784157eff7794';

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
String _$cardItemsHash() => r'e3d08b3782be91eaf088175c45e18bd3679707bc';

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
String _$contactItemsHash() => r'f87b21b8b441b4bdae3899e7fe8ea9cd1c9098dc';

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
String _$languageItemsHash() => r'8adc02fcee63512ed827f2334bc35697f87738b3';

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
String _$awardItemsHash() => r'9874f70a56802bc192e625f68a7e6815996ea789';

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
String _$idCardItemsHash() => r'3cbfdff8cd0ba0ea4a47a861a13059203e4f6e6a';

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
String _$addressItemsHash() => r'0e3922c2e3d0d3b3cc2898121dfdfa319bccda82';

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
