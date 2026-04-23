import 'package:solosoul_flutter/core/services/profile_storage_service.dart';

/// Centralized editor for profile section items.
///
/// Eliminates the 3 massive switch-case chains that previously duplicated
/// across travel/financial/professional/profile sections for:
/// - marking items deleted (soft-delete with timestamp)
/// - marking items restored (undo soft-delete)
/// - reading items by index
///
/// Each method returns `(ProfileData, bool)` where the bool indicates
/// whether the target item was found and modified.
class ProfileSectionEditor {
  ProfileSectionEditor._();

  // ===========================================================================
  // Soft-delete (mark as deleted)
  // ===========================================================================

  /// Marks the item at [index] in [section].[itemType] as deleted.
  /// Returns `(updatedProfile, wasFound)`.
  static (ProfileData, bool) markDeleted({
    required ProfileData current,
    required String section,
    required String itemType,
    required int index,
    required DateTime deletedAt,
  }) {
    switch (section) {
      case 'travel':
        return _markDeletedTravel(current, itemType, index, deletedAt);
      case 'financial':
        return _markDeletedFinancial(current, itemType, index, deletedAt);
      case 'professional':
        return _markDeletedProfessional(current, itemType, index, deletedAt);
      case 'profile':
        return _markDeletedProfile(current, itemType, index, deletedAt);
    }
    return (current, false);
  }

  static (ProfileData, bool) _markDeletedTravel(
    ProfileData current,
    String itemType,
    int index,
    DateTime deletedAt,
  ) {
    final travel = current.travel ?? TravelData();

    if (itemType == 'passport' && index < travel.passports.length) {
      final updated = List<PassportData>.from(travel.passports);
      updated[index] = updated[index].copyWith(
        isDeleted: true,
        deletedAt: deletedAt,
      );
      return (
        ProfileData(
          identity: current.identity,
          travel: TravelData(
            passports: updated,
            visas: travel.visas,
            travelHistory: travel.travelHistory,
          ),
          financial: current.financial,
          professional: current.professional,
        ),
        true,
      );
    } else if (itemType == 'visa' && index < travel.visas.length) {
      final updated = List<VisaData>.from(travel.visas);
      updated[index] = updated[index].copyWith(
        isDeleted: true,
        deletedAt: deletedAt,
      );
      return (
        ProfileData(
          identity: current.identity,
          travel: TravelData(
            passports: travel.passports,
            visas: updated,
            travelHistory: travel.travelHistory,
          ),
          financial: current.financial,
          professional: current.professional,
        ),
        true,
      );
    } else if (itemType == 'travel_history' &&
        index < travel.travelHistory.length) {
      final updated = List<TravelHistoryData>.from(travel.travelHistory);
      updated[index] = updated[index].copyWith(
        isDeleted: true,
        deletedAt: deletedAt,
      );
      return (
        ProfileData(
          identity: current.identity,
          travel: TravelData(
            passports: travel.passports,
            visas: travel.visas,
            travelHistory: updated,
          ),
          financial: current.financial,
          professional: current.professional,
        ),
        true,
      );
    }
    return (current, false);
  }

  static (ProfileData, bool) _markDeletedFinancial(
    ProfileData current,
    String itemType,
    int index,
    DateTime deletedAt,
  ) {
    final financial = current.financial ?? FinancialData();

    if (itemType == 'bank_account' &&
        index < financial.bankAccounts.length) {
      final updated = List<BankAccountData>.from(financial.bankAccounts);
      updated[index] = updated[index].copyWith(
        isDeleted: true,
        deletedAt: deletedAt,
      );
      return (
        ProfileData(
          identity: current.identity,
          travel: current.travel,
          financial: FinancialData(
            bankAccounts: updated,
            cards: financial.cards,
            taxIds: financial.taxIds,
          ),
          professional: current.professional,
        ),
        true,
      );
    } else if (itemType == 'card' && index < financial.cards.length) {
      final updated = List<CardData>.from(financial.cards);
      updated[index] = updated[index].copyWith(
        isDeleted: true,
        deletedAt: deletedAt,
      );
      return (
        ProfileData(
          identity: current.identity,
          travel: current.travel,
          financial: FinancialData(
            bankAccounts: financial.bankAccounts,
            cards: updated,
            taxIds: financial.taxIds,
          ),
          professional: current.professional,
        ),
        true,
      );
    } else if (itemType == 'tax_id' && index < financial.taxIds.length) {
      final updated = List<TaxIdData>.from(financial.taxIds);
      updated[index] = updated[index].copyWith(
        isDeleted: true,
        deletedAt: deletedAt,
      );
      return (
        ProfileData(
          identity: current.identity,
          travel: current.travel,
          financial: FinancialData(
            bankAccounts: financial.bankAccounts,
            cards: financial.cards,
            taxIds: updated,
          ),
          professional: current.professional,
        ),
        true,
      );
    }
    return (current, false);
  }

  static (ProfileData, bool) _markDeletedProfessional(
    ProfileData current,
    String itemType,
    int index,
    DateTime deletedAt,
  ) {
    final professional = current.professional ?? ProfessionalData();

    if (itemType == 'education' && index < professional.education.length) {
      final updated = List<EducationData>.from(professional.education);
      updated[index] = updated[index].copyWith(
        isDeleted: true,
        deletedAt: deletedAt,
      );
      return (
        ProfileData(
          identity: current.identity,
          travel: current.travel,
          financial: current.financial,
          professional: ProfessionalData(
            education: updated,
            employment: professional.employment,
            skills: professional.skills,
            languages: professional.languages,
          ),
        ),
        true,
      );
    } else if (itemType == 'employment' &&
        index < professional.employment.length) {
      final updated = List<EmploymentData>.from(professional.employment);
      updated[index] = updated[index].copyWith(
        isDeleted: true,
        deletedAt: deletedAt,
      );
      return (
        ProfileData(
          identity: current.identity,
          travel: current.travel,
          financial: current.financial,
          professional: ProfessionalData(
            education: professional.education,
            employment: updated,
            skills: professional.skills,
            languages: professional.languages,
          ),
        ),
        true,
      );
    } else if (itemType == 'skill' && index < professional.skills.length) {
      final updated = List<SkillData>.from(professional.skills);
      updated[index] = updated[index].copyWith(
        isDeleted: true,
        deletedAt: deletedAt,
      );
      return (
        ProfileData(
          identity: current.identity,
          travel: current.travel,
          financial: current.financial,
          professional: ProfessionalData(
            education: professional.education,
            employment: professional.employment,
            skills: updated,
            languages: professional.languages,
          ),
        ),
        true,
      );
    } else if (itemType == 'language' &&
        index < professional.languages.length) {
      final updated = List<LanguageData>.from(professional.languages);
      updated[index] = updated[index].copyWith(
        isDeleted: true,
        deletedAt: deletedAt,
      );
      return (
        ProfileData(
          identity: current.identity,
          travel: current.travel,
          financial: current.financial,
          professional: ProfessionalData(
            education: professional.education,
            employment: professional.employment,
            skills: professional.skills,
            languages: updated,
          ),
        ),
        true,
      );
    }
    return (current, false);
  }

  static (ProfileData, bool) _markDeletedProfile(
    ProfileData current,
    String itemType,
    int index,
    DateTime deletedAt,
  ) {
    final identity = current.identity ?? IdentityData();

    if (itemType == 'contact' &&
        index < (identity.contact?.entries.length ?? 0)) {
      final entries =
          List<ContactEntry>.from(identity.contact!.entries);
      entries[index] = entries[index].copyWith(
        isDeleted: true,
        deletedAt: deletedAt,
      );
      return (
        ProfileData(
          identity: IdentityData(
            fullName: identity.fullName,
            givenName: identity.givenName,
            familyName: identity.familyName,
            dateOfBirth: identity.dateOfBirth,
            gender: identity.gender,
            nationality: identity.nationality,
            idCards: identity.idCards,
            contact: ContactData(entries: entries),
            addresses: identity.addresses,
          ),
          travel: current.travel,
          financial: current.financial,
          professional: current.professional,
        ),
        true,
      );
    } else if (itemType == 'idCard' &&
        index < (identity.idCards?.length ?? 0)) {
      final idCards = List<IdCardData>.from(identity.idCards!);
      idCards[index] = idCards[index].copyWith(
        isDeleted: true,
        deletedAt: deletedAt,
      );
      return (
        ProfileData(
          identity: IdentityData(
            fullName: identity.fullName,
            givenName: identity.givenName,
            familyName: identity.familyName,
            dateOfBirth: identity.dateOfBirth,
            gender: identity.gender,
            nationality: identity.nationality,
            idCards: idCards,
            contact: identity.contact,
            addresses: identity.addresses,
          ),
          travel: current.travel,
          financial: current.financial,
          professional: current.professional,
        ),
        true,
      );
    } else if (itemType == 'address' &&
        index < (identity.addresses?.length ?? 0)) {
      final addresses = List<AddressData>.from(identity.addresses!);
      addresses[index] = addresses[index].copyWith(
        isDeleted: true,
        deletedAt: deletedAt,
      );
      return (
        ProfileData(
          identity: IdentityData(
            fullName: identity.fullName,
            givenName: identity.givenName,
            familyName: identity.familyName,
            dateOfBirth: identity.dateOfBirth,
            gender: identity.gender,
            nationality: identity.nationality,
            idCards: identity.idCards,
            contact: identity.contact,
            addresses: addresses,
          ),
          travel: current.travel,
          financial: current.financial,
          professional: current.professional,
        ),
        true,
      );
    }
    return (current, false);
  }

  // ===========================================================================
  // Restore (undo soft-delete)
  // ===========================================================================

  /// Marks the item at [index] in [section].[itemType] as restored.
  /// Returns `(updatedProfile, wasFound)`.
  static (ProfileData, bool) markRestored({
    required ProfileData current,
    required String section,
    required String itemType,
    required int index,
  }) {
    switch (section) {
      case 'travel':
        return _markRestoredTravel(current, itemType, index);
      case 'financial':
        return _markRestoredFinancial(current, itemType, index);
      case 'professional':
        return _markRestoredProfessional(current, itemType, index);
      case 'profile':
        return _markRestoredProfile(current, itemType, index);
    }
    return (current, false);
  }

  static (ProfileData, bool) _markRestoredTravel(
    ProfileData current,
    String itemType,
    int index,
  ) {
    final travel = current.travel ?? TravelData();

    if (itemType == 'passport' && index < travel.passports.length) {
      final updated = List<PassportData>.from(travel.passports);
      updated[index] = updated[index].copyWith(
        isDeleted: false,
        deletedAt: null,
      );
      return (
        ProfileData(
          identity: current.identity,
          travel: TravelData(
            passports: updated,
            visas: travel.visas,
            travelHistory: travel.travelHistory,
          ),
          financial: current.financial,
          professional: current.professional,
        ),
        true,
      );
    } else if (itemType == 'visa' && index < travel.visas.length) {
      final updated = List<VisaData>.from(travel.visas);
      updated[index] = updated[index].copyWith(
        isDeleted: false,
        deletedAt: null,
      );
      return (
        ProfileData(
          identity: current.identity,
          travel: TravelData(
            passports: travel.passports,
            visas: updated,
            travelHistory: travel.travelHistory,
          ),
          financial: current.financial,
          professional: current.professional,
        ),
        true,
      );
    } else if (itemType == 'travel_history' &&
        index < travel.travelHistory.length) {
      final updated = List<TravelHistoryData>.from(travel.travelHistory);
      updated[index] = updated[index].copyWith(
        isDeleted: false,
        deletedAt: null,
      );
      return (
        ProfileData(
          identity: current.identity,
          travel: TravelData(
            passports: travel.passports,
            visas: travel.visas,
            travelHistory: updated,
          ),
          financial: current.financial,
          professional: current.professional,
        ),
        true,
      );
    }
    return (current, false);
  }

  static (ProfileData, bool) _markRestoredFinancial(
    ProfileData current,
    String itemType,
    int index,
  ) {
    final financial = current.financial ?? FinancialData();

    if (itemType == 'bank_account' &&
        index < financial.bankAccounts.length) {
      final updated = List<BankAccountData>.from(financial.bankAccounts);
      updated[index] = updated[index].copyWith(
        isDeleted: false,
        deletedAt: null,
      );
      return (
        ProfileData(
          identity: current.identity,
          travel: current.travel,
          financial: FinancialData(
            bankAccounts: updated,
            cards: financial.cards,
            taxIds: financial.taxIds,
          ),
          professional: current.professional,
        ),
        true,
      );
    } else if (itemType == 'card' && index < financial.cards.length) {
      final updated = List<CardData>.from(financial.cards);
      updated[index] = updated[index].copyWith(
        isDeleted: false,
        deletedAt: null,
      );
      return (
        ProfileData(
          identity: current.identity,
          travel: current.travel,
          financial: FinancialData(
            bankAccounts: financial.bankAccounts,
            cards: updated,
            taxIds: financial.taxIds,
          ),
          professional: current.professional,
        ),
        true,
      );
    } else if (itemType == 'tax_id' && index < financial.taxIds.length) {
      final updated = List<TaxIdData>.from(financial.taxIds);
      updated[index] = updated[index].copyWith(
        isDeleted: false,
        deletedAt: null,
      );
      return (
        ProfileData(
          identity: current.identity,
          travel: current.travel,
          financial: FinancialData(
            bankAccounts: financial.bankAccounts,
            cards: financial.cards,
            taxIds: updated,
          ),
          professional: current.professional,
        ),
        true,
      );
    }
    return (current, false);
  }

  static (ProfileData, bool) _markRestoredProfessional(
    ProfileData current,
    String itemType,
    int index,
  ) {
    final professional = current.professional ?? ProfessionalData();

    if (itemType == 'education' && index < professional.education.length) {
      final updated = List<EducationData>.from(professional.education);
      updated[index] = updated[index].copyWith(
        isDeleted: false,
        deletedAt: null,
      );
      return (
        ProfileData(
          identity: current.identity,
          travel: current.travel,
          financial: current.financial,
          professional: ProfessionalData(
            education: updated,
            employment: professional.employment,
            skills: professional.skills,
            languages: professional.languages,
          ),
        ),
        true,
      );
    } else if (itemType == 'employment' &&
        index < professional.employment.length) {
      final updated = List<EmploymentData>.from(professional.employment);
      updated[index] = updated[index].copyWith(
        isDeleted: false,
        deletedAt: null,
      );
      return (
        ProfileData(
          identity: current.identity,
          travel: current.travel,
          financial: current.financial,
          professional: ProfessionalData(
            education: professional.education,
            employment: updated,
            skills: professional.skills,
            languages: professional.languages,
          ),
        ),
        true,
      );
    } else if (itemType == 'skill' && index < professional.skills.length) {
      final updated = List<SkillData>.from(professional.skills);
      updated[index] = updated[index].copyWith(
        isDeleted: false,
        deletedAt: null,
      );
      return (
        ProfileData(
          identity: current.identity,
          travel: current.travel,
          financial: current.financial,
          professional: ProfessionalData(
            education: professional.education,
            employment: professional.employment,
            skills: updated,
            languages: professional.languages,
          ),
        ),
        true,
      );
    } else if (itemType == 'language' &&
        index < professional.languages.length) {
      final updated = List<LanguageData>.from(professional.languages);
      updated[index] = updated[index].copyWith(
        isDeleted: false,
        deletedAt: null,
      );
      return (
        ProfileData(
          identity: current.identity,
          travel: current.travel,
          financial: current.financial,
          professional: ProfessionalData(
            education: professional.education,
            employment: professional.employment,
            skills: professional.skills,
            languages: updated,
          ),
        ),
        true,
      );
    }
    return (current, false);
  }

  static (ProfileData, bool) _markRestoredProfile(
    ProfileData current,
    String itemType,
    int index,
  ) {
    final identity = current.identity ?? IdentityData();

    if (itemType == 'contact' &&
        index < (identity.contact?.entries.length ?? 0)) {
      final entries = List<ContactEntry>.from(identity.contact!.entries);
      entries[index] = entries[index].copyWith(
        isDeleted: false,
        deletedAt: null,
      );
      return (
        ProfileData(
          identity: IdentityData(
            fullName: identity.fullName,
            givenName: identity.givenName,
            familyName: identity.familyName,
            dateOfBirth: identity.dateOfBirth,
            gender: identity.gender,
            nationality: identity.nationality,
            idCards: identity.idCards,
            contact: ContactData(entries: entries),
            addresses: identity.addresses,
          ),
          travel: current.travel,
          financial: current.financial,
          professional: current.professional,
        ),
        true,
      );
    } else if (itemType == 'idCard' &&
        index < (identity.idCards?.length ?? 0)) {
      final idCards = List<IdCardData>.from(identity.idCards!);
      idCards[index] = idCards[index].copyWith(
        isDeleted: false,
        deletedAt: null,
      );
      return (
        ProfileData(
          identity: IdentityData(
            fullName: identity.fullName,
            givenName: identity.givenName,
            familyName: identity.familyName,
            dateOfBirth: identity.dateOfBirth,
            gender: identity.gender,
            nationality: identity.nationality,
            idCards: idCards,
            contact: identity.contact,
            addresses: identity.addresses,
          ),
          travel: current.travel,
          financial: current.financial,
          professional: current.professional,
        ),
        true,
      );
    } else if (itemType == 'address' &&
        index < (identity.addresses?.length ?? 0)) {
      final addresses = List<AddressData>.from(identity.addresses!);
      addresses[index] = addresses[index].copyWith(
        isDeleted: false,
        deletedAt: null,
      );
      return (
        ProfileData(
          identity: IdentityData(
            fullName: identity.fullName,
            givenName: identity.givenName,
            familyName: identity.familyName,
            dateOfBirth: identity.dateOfBirth,
            gender: identity.gender,
            nationality: identity.nationality,
            idCards: identity.idCards,
            contact: identity.contact,
            addresses: addresses,
          ),
          travel: current.travel,
          financial: current.financial,
          professional: current.professional,
        ),
        true,
      );
    }
    return (current, false);
  }

  // ===========================================================================
  // Read item at index
  // ===========================================================================

  /// Gets the item at [index] in [section].[itemType], or null if not found.
  static dynamic getItem({
    required ProfileData profile,
    required String section,
    required String itemType,
    required int index,
  }) {
    switch (section) {
      case 'travel':
        return _getItemTravel(profile.travel, itemType, index);
      case 'financial':
        return _getItemFinancial(profile.financial, itemType, index);
      case 'professional':
        return _getItemProfessional(profile.professional, itemType, index);
      case 'profile':
        return _getItemProfile(profile.identity, itemType, index);
    }
    return null;
  }

  static dynamic _getItemTravel(
    TravelData? travel,
    String itemType,
    int index,
  ) {
    if (travel == null) return null;
    switch (itemType) {
      case 'passport':
        if (index >= 0 && index < travel.passports.length) {
          return travel.passports[index];
        }
        break;
      case 'visa':
        if (index >= 0 && index < travel.visas.length) {
          return travel.visas[index];
        }
        break;
      case 'travel_history':
        if (index >= 0 && index < travel.travelHistory.length) {
          return travel.travelHistory[index];
        }
        break;
    }
    return null;
  }

  static dynamic _getItemFinancial(
    FinancialData? financial,
    String itemType,
    int index,
  ) {
    if (financial == null) return null;
    switch (itemType) {
      case 'bank_account':
        if (index >= 0 && index < financial.bankAccounts.length) {
          return financial.bankAccounts[index];
        }
        break;
      case 'card':
        if (index >= 0 && index < financial.cards.length) {
          return financial.cards[index];
        }
        break;
      case 'tax_id':
        if (index >= 0 && index < financial.taxIds.length) {
          return financial.taxIds[index];
        }
        break;
    }
    return null;
  }

  static dynamic _getItemProfessional(
    ProfessionalData? professional,
    String itemType,
    int index,
  ) {
    if (professional == null) return null;
    switch (itemType) {
      case 'education':
        if (index >= 0 && index < professional.education.length) {
          return professional.education[index];
        }
        break;
      case 'employment':
        if (index >= 0 && index < professional.employment.length) {
          return professional.employment[index];
        }
        break;
      case 'skill':
        if (index >= 0 && index < professional.skills.length) {
          return professional.skills[index];
        }
        break;
      case 'language':
        if (index >= 0 && index < professional.languages.length) {
          return professional.languages[index];
        }
        break;
    }
    return null;
  }

  static dynamic _getItemProfile(
    IdentityData? identity,
    String itemType,
    int index,
  ) {
    if (identity == null) return null;
    switch (itemType) {
      case 'contact':
        if (identity.contact != null &&
            index >= 0 &&
            index < identity.contact!.entries.length) {
          return identity.contact!.entries[index];
        }
        break;
      case 'idCard':
        if (identity.idCards != null &&
            index >= 0 &&
            index < identity.idCards!.length) {
          return identity.idCards![index];
        }
        break;
      case 'address':
        if (identity.addresses != null &&
            index >= 0 &&
            index < identity.addresses!.length) {
          return identity.addresses![index];
        }
        break;
    }
    return null;
  }
}
