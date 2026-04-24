// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'profile_provider.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning
/// Identity section provider

@ProviderFor(ProfileIdentity)
const profileIdentityProvider = ProfileIdentityProvider._();

/// Identity section provider
final class ProfileIdentityProvider
    extends $NotifierProvider<ProfileIdentity, IdentityData?> {
  /// Identity section provider
  const ProfileIdentityProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'profileIdentityProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$profileIdentityHash();

  @$internal
  @override
  ProfileIdentity create() => ProfileIdentity();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(IdentityData? value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<IdentityData?>(value),
    );
  }
}

String _$profileIdentityHash() => r'f9ee523f55d247909309fc99cb0fcf83b2c26d48';

/// Identity section provider

abstract class _$ProfileIdentity extends $Notifier<IdentityData?> {
  IdentityData? build();
  @$mustCallSuper
  @override
  void runBuild() {
    final created = build();
    final ref = this.ref as $Ref<IdentityData?, IdentityData?>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<IdentityData?, IdentityData?>,
              IdentityData?,
              Object?,
              Object?
            >;
    element.handleValue(ref, created);
  }
}

/// Travel section provider

@ProviderFor(ProfileTravel)
const profileTravelProvider = ProfileTravelProvider._();

/// Travel section provider
final class ProfileTravelProvider
    extends $NotifierProvider<ProfileTravel, TravelData?> {
  /// Travel section provider
  const ProfileTravelProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'profileTravelProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$profileTravelHash();

  @$internal
  @override
  ProfileTravel create() => ProfileTravel();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(TravelData? value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<TravelData?>(value),
    );
  }
}

String _$profileTravelHash() => r'6102ea79d88174f93e81447a9572c49fc1eb8e8c';

/// Travel section provider

abstract class _$ProfileTravel extends $Notifier<TravelData?> {
  TravelData? build();
  @$mustCallSuper
  @override
  void runBuild() {
    final created = build();
    final ref = this.ref as $Ref<TravelData?, TravelData?>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<TravelData?, TravelData?>,
              TravelData?,
              Object?,
              Object?
            >;
    element.handleValue(ref, created);
  }
}

/// Financial section provider

@ProviderFor(ProfileFinancial)
const profileFinancialProvider = ProfileFinancialProvider._();

/// Financial section provider
final class ProfileFinancialProvider
    extends $NotifierProvider<ProfileFinancial, FinancialData?> {
  /// Financial section provider
  const ProfileFinancialProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'profileFinancialProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$profileFinancialHash();

  @$internal
  @override
  ProfileFinancial create() => ProfileFinancial();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(FinancialData? value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<FinancialData?>(value),
    );
  }
}

String _$profileFinancialHash() => r'6f867402030e0464eff2dac25e6c3352447c5f77';

/// Financial section provider

abstract class _$ProfileFinancial extends $Notifier<FinancialData?> {
  FinancialData? build();
  @$mustCallSuper
  @override
  void runBuild() {
    final created = build();
    final ref = this.ref as $Ref<FinancialData?, FinancialData?>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<FinancialData?, FinancialData?>,
              FinancialData?,
              Object?,
              Object?
            >;
    element.handleValue(ref, created);
  }
}

/// Professional section provider

@ProviderFor(ProfileProfessional)
const profileProfessionalProvider = ProfileProfessionalProvider._();

/// Professional section provider
final class ProfileProfessionalProvider
    extends $NotifierProvider<ProfileProfessional, ProfessionalData?> {
  /// Professional section provider
  const ProfileProfessionalProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'profileProfessionalProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$profileProfessionalHash();

  @$internal
  @override
  ProfileProfessional create() => ProfileProfessional();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(ProfessionalData? value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<ProfessionalData?>(value),
    );
  }
}

String _$profileProfessionalHash() =>
    r'e6afa824dc6ba8a40299357de3e7d0e8269b0e3f';

/// Professional section provider

abstract class _$ProfileProfessional extends $Notifier<ProfessionalData?> {
  ProfessionalData? build();
  @$mustCallSuper
  @override
  void runBuild() {
    final created = build();
    final ref = this.ref as $Ref<ProfessionalData?, ProfessionalData?>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<ProfessionalData?, ProfessionalData?>,
              ProfessionalData?,
              Object?,
              Object?
            >;
    element.handleValue(ref, created);
  }
}

/// Education items provider - derives sorted EducationData from profileNotifierProvider.

@ProviderFor(EducationItems)
const educationItemsProvider = EducationItemsProvider._();

/// Education items provider - derives sorted EducationData from profileNotifierProvider.
final class EducationItemsProvider
    extends $NotifierProvider<EducationItems, List<EducationData>> {
  /// Education items provider - derives sorted EducationData from profileNotifierProvider.
  const EducationItemsProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'educationItemsProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$educationItemsHash();

  @$internal
  @override
  EducationItems create() => EducationItems();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(List<EducationData> value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<List<EducationData>>(value),
    );
  }
}

String _$educationItemsHash() => r'f32830cafab9b2e32ffaf7752619468740f48275';

/// Education items provider - derives sorted EducationData from profileNotifierProvider.

abstract class _$EducationItems extends $Notifier<List<EducationData>> {
  List<EducationData> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final created = build();
    final ref = this.ref as $Ref<List<EducationData>, List<EducationData>>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<List<EducationData>, List<EducationData>>,
              List<EducationData>,
              Object?,
              Object?
            >;
    element.handleValue(ref, created);
  }
}

/// Bank account items provider

@ProviderFor(BankAccountItems)
const bankAccountItemsProvider = BankAccountItemsProvider._();

/// Bank account items provider
final class BankAccountItemsProvider
    extends $NotifierProvider<BankAccountItems, List<BankAccountData>> {
  /// Bank account items provider
  const BankAccountItemsProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'bankAccountItemsProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$bankAccountItemsHash();

  @$internal
  @override
  BankAccountItems create() => BankAccountItems();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(List<BankAccountData> value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<List<BankAccountData>>(value),
    );
  }
}

String _$bankAccountItemsHash() => r'a698ddd4c21884e35ee10ab104f5f0e337415033';

/// Bank account items provider

abstract class _$BankAccountItems extends $Notifier<List<BankAccountData>> {
  List<BankAccountData> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final created = build();
    final ref = this.ref as $Ref<List<BankAccountData>, List<BankAccountData>>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<List<BankAccountData>, List<BankAccountData>>,
              List<BankAccountData>,
              Object?,
              Object?
            >;
    element.handleValue(ref, created);
  }
}

/// Employment items provider

@ProviderFor(EmploymentItems)
const employmentItemsProvider = EmploymentItemsProvider._();

/// Employment items provider
final class EmploymentItemsProvider
    extends $NotifierProvider<EmploymentItems, List<EmploymentData>> {
  /// Employment items provider
  const EmploymentItemsProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'employmentItemsProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$employmentItemsHash();

  @$internal
  @override
  EmploymentItems create() => EmploymentItems();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(List<EmploymentData> value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<List<EmploymentData>>(value),
    );
  }
}

String _$employmentItemsHash() => r'3b7636caed5b88d38a29119609f165078eb0dfbf';

/// Employment items provider

abstract class _$EmploymentItems extends $Notifier<List<EmploymentData>> {
  List<EmploymentData> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final created = build();
    final ref = this.ref as $Ref<List<EmploymentData>, List<EmploymentData>>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<List<EmploymentData>, List<EmploymentData>>,
              List<EmploymentData>,
              Object?,
              Object?
            >;
    element.handleValue(ref, created);
  }
}

/// Skill items provider

@ProviderFor(SkillItems)
const skillItemsProvider = SkillItemsProvider._();

/// Skill items provider
final class SkillItemsProvider
    extends $NotifierProvider<SkillItems, List<SkillData>> {
  /// Skill items provider
  const SkillItemsProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'skillItemsProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$skillItemsHash();

  @$internal
  @override
  SkillItems create() => SkillItems();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(List<SkillData> value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<List<SkillData>>(value),
    );
  }
}

String _$skillItemsHash() => r'1e7c9e000684fa7522195d94d502907d969cb0e9';

/// Skill items provider

abstract class _$SkillItems extends $Notifier<List<SkillData>> {
  List<SkillData> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final created = build();
    final ref = this.ref as $Ref<List<SkillData>, List<SkillData>>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<List<SkillData>, List<SkillData>>,
              List<SkillData>,
              Object?,
              Object?
            >;
    element.handleValue(ref, created);
  }
}

/// Tax ID items provider

@ProviderFor(TaxIdItems)
const taxIdItemsProvider = TaxIdItemsProvider._();

/// Tax ID items provider
final class TaxIdItemsProvider
    extends $NotifierProvider<TaxIdItems, List<TaxIdData>> {
  /// Tax ID items provider
  const TaxIdItemsProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'taxIdItemsProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$taxIdItemsHash();

  @$internal
  @override
  TaxIdItems create() => TaxIdItems();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(List<TaxIdData> value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<List<TaxIdData>>(value),
    );
  }
}

String _$taxIdItemsHash() => r'53c9cba0d641fd562cc540fd586aea05f1eb008c';

/// Tax ID items provider

abstract class _$TaxIdItems extends $Notifier<List<TaxIdData>> {
  List<TaxIdData> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final created = build();
    final ref = this.ref as $Ref<List<TaxIdData>, List<TaxIdData>>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<List<TaxIdData>, List<TaxIdData>>,
              List<TaxIdData>,
              Object?,
              Object?
            >;
    element.handleValue(ref, created);
  }
}

/// Passport items provider

@ProviderFor(PassportItems)
const passportItemsProvider = PassportItemsProvider._();

/// Passport items provider
final class PassportItemsProvider
    extends $NotifierProvider<PassportItems, List<PassportData>> {
  /// Passport items provider
  const PassportItemsProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'passportItemsProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$passportItemsHash();

  @$internal
  @override
  PassportItems create() => PassportItems();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(List<PassportData> value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<List<PassportData>>(value),
    );
  }
}

String _$passportItemsHash() => r'15ae06710a083b5c733bb4bae7431ce254b583ae';

/// Passport items provider

abstract class _$PassportItems extends $Notifier<List<PassportData>> {
  List<PassportData> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final created = build();
    final ref = this.ref as $Ref<List<PassportData>, List<PassportData>>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<List<PassportData>, List<PassportData>>,
              List<PassportData>,
              Object?,
              Object?
            >;
    element.handleValue(ref, created);
  }
}

/// Visa items provider

@ProviderFor(VisaItems)
const visaItemsProvider = VisaItemsProvider._();

/// Visa items provider
final class VisaItemsProvider
    extends $NotifierProvider<VisaItems, List<VisaData>> {
  /// Visa items provider
  const VisaItemsProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'visaItemsProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$visaItemsHash();

  @$internal
  @override
  VisaItems create() => VisaItems();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(List<VisaData> value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<List<VisaData>>(value),
    );
  }
}

String _$visaItemsHash() => r'20bb9d2611487139bc3a401d1fa3cc9e4d82e7d7';

/// Visa items provider

abstract class _$VisaItems extends $Notifier<List<VisaData>> {
  List<VisaData> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final created = build();
    final ref = this.ref as $Ref<List<VisaData>, List<VisaData>>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<List<VisaData>, List<VisaData>>,
              List<VisaData>,
              Object?,
              Object?
            >;
    element.handleValue(ref, created);
  }
}

/// Travel history items provider

@ProviderFor(TravelHistoryItems)
const travelHistoryItemsProvider = TravelHistoryItemsProvider._();

/// Travel history items provider
final class TravelHistoryItemsProvider
    extends $NotifierProvider<TravelHistoryItems, List<TravelHistoryData>> {
  /// Travel history items provider
  const TravelHistoryItemsProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'travelHistoryItemsProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$travelHistoryItemsHash();

  @$internal
  @override
  TravelHistoryItems create() => TravelHistoryItems();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(List<TravelHistoryData> value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<List<TravelHistoryData>>(value),
    );
  }
}

String _$travelHistoryItemsHash() =>
    r'cdec59130fd93dfc5381de0fef6ac73a1f053d05';

/// Travel history items provider

abstract class _$TravelHistoryItems extends $Notifier<List<TravelHistoryData>> {
  List<TravelHistoryData> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final created = build();
    final ref =
        this.ref as $Ref<List<TravelHistoryData>, List<TravelHistoryData>>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<List<TravelHistoryData>, List<TravelHistoryData>>,
              List<TravelHistoryData>,
              Object?,
              Object?
            >;
    element.handleValue(ref, created);
  }
}

/// Card items provider

@ProviderFor(CardItems)
const cardItemsProvider = CardItemsProvider._();

/// Card items provider
final class CardItemsProvider
    extends $NotifierProvider<CardItems, List<CardData>> {
  /// Card items provider
  const CardItemsProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'cardItemsProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$cardItemsHash();

  @$internal
  @override
  CardItems create() => CardItems();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(List<CardData> value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<List<CardData>>(value),
    );
  }
}

String _$cardItemsHash() => r'dda1cf907ee4f1fe1650cd85b8b646affdffc744';

/// Card items provider

abstract class _$CardItems extends $Notifier<List<CardData>> {
  List<CardData> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final created = build();
    final ref = this.ref as $Ref<List<CardData>, List<CardData>>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<List<CardData>, List<CardData>>,
              List<CardData>,
              Object?,
              Object?
            >;
    element.handleValue(ref, created);
  }
}

/// Contact items provider

@ProviderFor(ContactItems)
const contactItemsProvider = ContactItemsProvider._();

/// Contact items provider
final class ContactItemsProvider
    extends $NotifierProvider<ContactItems, List<ContactEntry>> {
  /// Contact items provider
  const ContactItemsProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'contactItemsProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$contactItemsHash();

  @$internal
  @override
  ContactItems create() => ContactItems();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(List<ContactEntry> value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<List<ContactEntry>>(value),
    );
  }
}

String _$contactItemsHash() => r'688c5d062c0b254c679d9c28d9d4dd8bb6a06f63';

/// Contact items provider

abstract class _$ContactItems extends $Notifier<List<ContactEntry>> {
  List<ContactEntry> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final created = build();
    final ref = this.ref as $Ref<List<ContactEntry>, List<ContactEntry>>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<List<ContactEntry>, List<ContactEntry>>,
              List<ContactEntry>,
              Object?,
              Object?
            >;
    element.handleValue(ref, created);
  }
}

/// Language items provider

@ProviderFor(LanguageItems)
const languageItemsProvider = LanguageItemsProvider._();

/// Language items provider
final class LanguageItemsProvider
    extends $NotifierProvider<LanguageItems, List<LanguageData>> {
  /// Language items provider
  const LanguageItemsProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'languageItemsProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$languageItemsHash();

  @$internal
  @override
  LanguageItems create() => LanguageItems();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(List<LanguageData> value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<List<LanguageData>>(value),
    );
  }
}

String _$languageItemsHash() => r'98f70e70f82f87425f478183b11c3ef3d40b51e0';

/// Language items provider

abstract class _$LanguageItems extends $Notifier<List<LanguageData>> {
  List<LanguageData> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final created = build();
    final ref = this.ref as $Ref<List<LanguageData>, List<LanguageData>>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<List<LanguageData>, List<LanguageData>>,
              List<LanguageData>,
              Object?,
              Object?
            >;
    element.handleValue(ref, created);
  }
}

/// Award items provider

@ProviderFor(AwardItems)
const awardItemsProvider = AwardItemsProvider._();

/// Award items provider
final class AwardItemsProvider
    extends $NotifierProvider<AwardItems, List<AwardData>> {
  /// Award items provider
  const AwardItemsProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'awardItemsProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$awardItemsHash();

  @$internal
  @override
  AwardItems create() => AwardItems();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(List<AwardData> value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<List<AwardData>>(value),
    );
  }
}

String _$awardItemsHash() => r'59aa035bad5b95c2723b14b0e859b1e6769015da';

/// Award items provider

abstract class _$AwardItems extends $Notifier<List<AwardData>> {
  List<AwardData> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final created = build();
    final ref = this.ref as $Ref<List<AwardData>, List<AwardData>>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<List<AwardData>, List<AwardData>>,
              List<AwardData>,
              Object?,
              Object?
            >;
    element.handleValue(ref, created);
  }
}

/// ID card items provider

@ProviderFor(IdCardItems)
const idCardItemsProvider = IdCardItemsProvider._();

/// ID card items provider
final class IdCardItemsProvider
    extends $NotifierProvider<IdCardItems, List<IdCardData>> {
  /// ID card items provider
  const IdCardItemsProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'idCardItemsProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$idCardItemsHash();

  @$internal
  @override
  IdCardItems create() => IdCardItems();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(List<IdCardData> value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<List<IdCardData>>(value),
    );
  }
}

String _$idCardItemsHash() => r'59d9e16129af5640c779e0aa1a32cdf4d7e546c0';

/// ID card items provider

abstract class _$IdCardItems extends $Notifier<List<IdCardData>> {
  List<IdCardData> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final created = build();
    final ref = this.ref as $Ref<List<IdCardData>, List<IdCardData>>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<List<IdCardData>, List<IdCardData>>,
              List<IdCardData>,
              Object?,
              Object?
            >;
    element.handleValue(ref, created);
  }
}

/// Address items provider

@ProviderFor(AddressItems)
const addressItemsProvider = AddressItemsProvider._();

/// Address items provider
final class AddressItemsProvider
    extends $NotifierProvider<AddressItems, List<AddressData>> {
  /// Address items provider
  const AddressItemsProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'addressItemsProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$addressItemsHash();

  @$internal
  @override
  AddressItems create() => AddressItems();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(List<AddressData> value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<List<AddressData>>(value),
    );
  }
}

String _$addressItemsHash() => r'1f3d7b79e02896eb86467ea2f7b93c1be19f7d9b';

/// Address items provider

abstract class _$AddressItems extends $Notifier<List<AddressData>> {
  List<AddressData> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final created = build();
    final ref = this.ref as $Ref<List<AddressData>, List<AddressData>>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<List<AddressData>, List<AddressData>>,
              List<AddressData>,
              Object?,
              Object?
            >;
    element.handleValue(ref, created);
  }
}
