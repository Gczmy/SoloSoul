import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';
import 'package:path_provider/path_provider.dart';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';

/// Maximum character limits for form fields
const int kMaxFieldLength = 32;
const int kMaxNameLength = 16;

/// Sentinel value for copyWith to distinguish "not provided" from "explicitly null"
class _DeletedAtSentinel {
  const _DeletedAtSentinel();
}

/// Profile data types matching Rust profile.rs
class ProfileData {
  IdentityData? identity;
  TravelData? travel;
  FinancialData? financial;
  ProfessionalData? professional;

  ProfileData({this.identity, this.travel, this.financial, this.professional});

  factory ProfileData.fromJson(Map<String, dynamic> json) {
    return ProfileData(
      identity: json['identity'] != null
          ? IdentityData.fromJson(json['identity'])
          : null,
      travel: json['travel'] != null
          ? TravelData.fromJson(json['travel'])
          : null,
      financial: json['financial'] != null
          ? FinancialData.fromJson(json['financial'])
          : null,
      professional: json['professional'] != null
          ? ProfessionalData.fromJson(json['professional'])
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'identity': identity?.toJson(),
    'travel': travel?.toJson(),
    'financial': financial?.toJson(),
    'professional': professional?.toJson(),
  };

  ProfileData copyWith({
    IdentityData? identity,
    TravelData? travel,
    FinancialData? financial,
    ProfessionalData? professional,
  }) {
    return ProfileData(
      identity: identity ?? this.identity,
      travel: travel ?? this.travel,
      financial: financial ?? this.financial,
      professional: professional ?? this.professional,
    );
  }
}

class IdentityData {
  String? fullName;
  String? givenName;
  String? familyName;
  String? dateOfBirth;
  String? gender;
  String? nationality;
  List<IdCardData>? idCards;
  ContactData? contact;
  List<AddressData>? addresses;

  IdentityData({
    this.fullName,
    this.givenName,
    this.familyName,
    this.dateOfBirth,
    this.gender,
    this.nationality,
    this.idCards,
    this.contact,
    this.addresses,
  });

  factory IdentityData.fromJson(Map<String, dynamic> json) {
    return IdentityData(
      fullName: json['full_name'],
      givenName: json['given_name'],
      familyName: json['family_name'],
      dateOfBirth: json['date_of_birth'],
      gender: json['gender'],
      nationality: json['nationality'],
      idCards: json['id_cards'] != null
          ? (json['id_cards'] as List)
              .map((e) => IdCardData.fromJson(e))
              .toList()
          : null,
      contact: json['contact'] != null
          ? ContactData.fromJson(json['contact'])
          : null,
      addresses: json['addresses'] != null
          ? (json['addresses'] as List)
                .map((e) => AddressData.fromJson(e))
                .toList()
          : null,
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'full_name': fullName,
      'given_name': givenName,
      'family_name': familyName,
      'date_of_birth': dateOfBirth,
      'gender': gender,
      'nationality': nationality,
      'id_cards': idCards?.map((e) => e.toJson()).toList(),
      'contact': contact?.toJson(),
      'addresses': addresses?.map((e) => e.toJson()).toList(),
    };
  }

  IdentityData copyWith({
    String? fullName,
    String? givenName,
    String? familyName,
    String? dateOfBirth,
    String? gender,
    String? nationality,
    List<IdCardData>? idCards,
    ContactData? contact,
    List<AddressData>? addresses,
  }) {
    return IdentityData(
      fullName: fullName ?? this.fullName,
      givenName: givenName ?? this.givenName,
      familyName: familyName ?? this.familyName,
      dateOfBirth: dateOfBirth ?? this.dateOfBirth,
      gender: gender ?? this.gender,
      nationality: nationality ?? this.nationality,
      idCards: idCards ?? this.idCards,
      contact: contact ?? this.contact,
      addresses: addresses ?? this.addresses,
    );
  }

  /// Filter out soft-deleted items
  List<IdCardData> get activeIdCards =>
      idCards?.where((c) => !c.isDeleted).toList() ?? [];

  List<AddressData> get activeAddresses =>
      addresses?.where((a) => !a.isDeleted).toList() ?? [];
}

class ContactEntry {
  String label; // e.g., "Personal", "Work", "Emergency"
  String type; // "email", "phone", "mobile"
  String value;
  bool isDeleted;
  DateTime? deletedAt;

  ContactEntry({
    required this.label,
    required this.type,
    required this.value,
    this.isDeleted = false,
    this.deletedAt,
  });

  factory ContactEntry.fromJson(Map<String, dynamic> json) {
    return ContactEntry(
      label: json['label'] ?? '',
      type: json['type'] ?? 'email',
      value: json['value'] ?? '',
      isDeleted: json['is_deleted'] ?? false,
      deletedAt: json['deleted_at'] != null
          ? DateTime.tryParse(json['deleted_at'])
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'label': label,
    'type': type,
    'value': value,
    'is_deleted': isDeleted,
    'deleted_at': deletedAt?.toIso8601String(),
  };

  static const _sentinel = _DeletedAtSentinel();

  ContactEntry copyWith({
    String? label,
    String? type,
    String? value,
    bool? isDeleted,
    Object? deletedAt = _sentinel,
  }) {
    return ContactEntry(
      label: label ?? this.label,
      type: type ?? this.type,
      value: value ?? this.value,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: identical(deletedAt, _sentinel) ? this.deletedAt : deletedAt as DateTime?,
    );
  }
}

class ContactData {
  List<ContactEntry> entries;

  ContactData({this.entries = const []});

  factory ContactData.fromJson(Map<String, dynamic> json) {
    final entriesList = json['entries'] as List<dynamic>?;
    return ContactData(
      entries:
          entriesList
              ?.map((e) => ContactEntry.fromJson(e as Map<String, dynamic>))
              .toList() ??
          [],
    );
  }

  Map<String, dynamic> toJson() {
    return {'entries': entries.map((e) => e.toJson()).toList()};
  }

  ContactData copyWith({List<ContactEntry>? entries}) {
    return ContactData(entries: entries ?? this.entries);
  }

  /// Filter out soft-deleted entries
  List<ContactEntry> get activeEntries =>
      entries.where((e) => !e.isDeleted).toList();
}

class AddressData {
  String? label;
  String? street;
  String? city;
  String? state;
  String? postalCode;
  String? country;
  bool isDeleted;
  DateTime? deletedAt;

  AddressData({
    this.label,
    this.street,
    this.city,
    this.state,
    this.postalCode,
    this.country,
    this.isDeleted = false,
    this.deletedAt,
  });

  factory AddressData.fromJson(Map<String, dynamic> json) {
    return AddressData(
      label: json['label'],
      street: json['street'],
      city: json['city'],
      state: json['state'],
      postalCode: json['postal_code'],
      country: json['country'],
      isDeleted: json['is_deleted'] ?? false,
      deletedAt: json['deleted_at'] != null
          ? DateTime.tryParse(json['deleted_at'])
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'label': label,
    'street': street,
    'city': city,
    'state': state,
    'postal_code': postalCode,
    'country': country,
    'is_deleted': isDeleted,
    'deleted_at': deletedAt?.toIso8601String(),
  };

  static const _sentinel = _DeletedAtSentinel();

  AddressData copyWith({
    String? label,
    String? street,
    String? city,
    String? state,
    String? postalCode,
    String? country,
    bool? isDeleted,
    Object? deletedAt = _sentinel,
  }) {
    return AddressData(
      label: label ?? this.label,
      street: street ?? this.street,
      city: city ?? this.city,
      state: state ?? this.state,
      postalCode: postalCode ?? this.postalCode,
      country: country ?? this.country,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: identical(deletedAt, _sentinel) ? this.deletedAt : deletedAt as DateTime?,
    );
  }
}

class IdCardData {
  String? label;
  String? number;
  String? issueDate;
  String? expiryDate;
  String? holderName;
  String? country;
  bool isDeleted;
  DateTime? deletedAt;

  IdCardData({
    this.label,
    this.number,
    this.issueDate,
    this.expiryDate,
    this.holderName,
    this.country,
    this.isDeleted = false,
    this.deletedAt,
  });

  factory IdCardData.fromJson(Map<String, dynamic> json) {
    return IdCardData(
      label: json['label'],
      number: json['number'],
      issueDate: json['issue_date'],
      expiryDate: json['expiry_date'],
      holderName: json['holder_name'],
      country: json['country'],
      isDeleted: json['is_deleted'] ?? false,
      deletedAt: json['deleted_at'] != null
          ? DateTime.tryParse(json['deleted_at'])
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'label': label,
    'number': number,
    'issue_date': issueDate,
    'expiry_date': expiryDate,
    'holder_name': holderName,
    'country': country,
    'is_deleted': isDeleted,
    'deleted_at': deletedAt?.toIso8601String(),
  };

  static const _sentinel = _DeletedAtSentinel();

  IdCardData copyWith({
    String? label,
    String? number,
    String? issueDate,
    String? expiryDate,
    String? holderName,
    String? country,
    bool? isDeleted,
    Object? deletedAt = _sentinel,
  }) {
    return IdCardData(
      label: label ?? this.label,
      number: number ?? this.number,
      issueDate: issueDate ?? this.issueDate,
      expiryDate: expiryDate ?? this.expiryDate,
      holderName: holderName ?? this.holderName,
      country: country ?? this.country,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: identical(deletedAt, _sentinel) ? this.deletedAt : deletedAt as DateTime?,
    );
  }
}

class TravelHistoryData {
  String destination;
  String? date;
  bool isDeleted;
  DateTime? deletedAt;

  TravelHistoryData({
    required this.destination,
    this.date,
    this.isDeleted = false,
    this.deletedAt,
  });

  factory TravelHistoryData.fromJson(Map<String, dynamic> json) {
    return TravelHistoryData(
      destination: json['destination'] ?? '',
      date: json['date'],
      isDeleted: json['is_deleted'] ?? false,
      deletedAt: json['deleted_at'] != null
          ? DateTime.tryParse(json['deleted_at'])
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'destination': destination,
    'date': date,
    'is_deleted': isDeleted,
    'deleted_at': deletedAt?.toIso8601String(),
  };

  TravelHistoryData copyWith({
    String? destination,
    String? date,
    bool? isDeleted,
    DateTime? deletedAt,
  }) {
    return TravelHistoryData(
      destination: destination ?? this.destination,
      date: date ?? this.date,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: deletedAt ?? this.deletedAt,
    );
  }

  @override
  String toString() => destination;
}

class TravelData {
  List<PassportData> passports;
  List<VisaData> visas;
  List<TravelHistoryData> travelHistory;

  TravelData({
    this.passports = const [],
    this.visas = const [],
    this.travelHistory = const [],
  });

  /// Filter out soft-deleted items
  List<PassportData> get activePassports =>
      passports.where((p) => !p.isDeleted).toList();

  List<VisaData> get activeVisas =>
      visas.where((v) => !v.isDeleted).toList();

  List<TravelHistoryData> get activeTravelHistory =>
      travelHistory.where((t) => !t.isDeleted).toList();

  /// Get soft-deleted items only
  List<PassportData> get deletedPassports =>
      passports.where((p) => p.isDeleted).toList();

  List<VisaData> get deletedVisas =>
      visas.where((v) => v.isDeleted).toList();

  List<TravelHistoryData> get deletedTravelHistory =>
      travelHistory.where((t) => t.isDeleted).toList();

  factory TravelData.fromJson(Map<String, dynamic> json) {
    return TravelData(
      passports:
          (json['passports'] as List<dynamic>?)
              ?.map((e) => PassportData.fromJson(e))
              .toList() ??
          [],
      visas:
          (json['visas'] as List<dynamic>?)
              ?.map((e) => VisaData.fromJson(e))
              .toList() ??
          [],
      travelHistory:
          (json['travel_history'] as List<dynamic>?)
              ?.map((e) => TravelHistoryData.fromJson(e))
              .toList() ??
          [],
    );
  }

  Map<String, dynamic> toJson() => {
    'passports': passports.map((e) => e.toJson()).toList(),
    'visas': visas.map((e) => e.toJson()).toList(),
    'travel_history': travelHistory.map((e) => e.toJson()).toList(),
  };

  TravelData copyWith({
    List<PassportData>? passports,
    List<VisaData>? visas,
    List<TravelHistoryData>? travelHistory,
  }) {
    return TravelData(
      passports: passports ?? this.passports,
      visas: visas ?? this.visas,
      travelHistory: travelHistory ?? this.travelHistory,
    );
  }
}

class PassportData {
  String? number;
  String? country;
  String? issueDate;
  String? expiryDate;
  String? holderName;
  bool isDeleted;
  DateTime? deletedAt;

  PassportData({
    this.number,
    this.country,
    this.issueDate,
    this.expiryDate,
    this.holderName,
    this.isDeleted = false,
    this.deletedAt,
  });

  factory PassportData.fromJson(Map<String, dynamic> json) {
    return PassportData(
      number: json['number'],
      country: json['country'],
      issueDate: json['issue_date'],
      expiryDate: json['expiry_date'],
      holderName: json['holder_name'],
      isDeleted: json['is_deleted'] ?? false,
      deletedAt: json['deleted_at'] != null
          ? DateTime.tryParse(json['deleted_at'])
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'number': number,
    'country': country,
    'issue_date': issueDate,
    'expiry_date': expiryDate,
    'holder_name': holderName,
    'is_deleted': isDeleted,
    'deleted_at': deletedAt?.toIso8601String(),
  };

  static const _sentinel = _DeletedAtSentinel();

  PassportData copyWith({
    String? number,
    String? country,
    String? issueDate,
    String? expiryDate,
    String? holderName,
    bool? isDeleted,
    Object? deletedAt = _sentinel,
  }) {
    return PassportData(
      number: number ?? this.number,
      country: country ?? this.country,
      issueDate: issueDate ?? this.issueDate,
      expiryDate: expiryDate ?? this.expiryDate,
      holderName: holderName ?? this.holderName,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: identical(deletedAt, _sentinel) ? this.deletedAt : deletedAt as DateTime?,
    );
  }
}

class VisaData {
  String? country;
  String? visaType;
  String? number;
  String? issueDate;
  String? expiryDate;
  bool isDeleted;
  DateTime? deletedAt;

  VisaData({
    this.country,
    this.visaType,
    this.number,
    this.issueDate,
    this.expiryDate,
    this.isDeleted = false,
    this.deletedAt,
  });

  factory VisaData.fromJson(Map<String, dynamic> json) {
    return VisaData(
      country: json['country'],
      visaType: json['visa_type'],
      number: json['number'],
      issueDate: json['issue_date'],
      expiryDate: json['expiry_date'],
      isDeleted: json['is_deleted'] ?? false,
      deletedAt: json['deleted_at'] != null
          ? DateTime.tryParse(json['deleted_at'])
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'country': country,
    'visa_type': visaType,
    'number': number,
    'issue_date': issueDate,
    'expiry_date': expiryDate,
    'is_deleted': isDeleted,
    'deleted_at': deletedAt?.toIso8601String(),
  };

  static const _sentinel = _DeletedAtSentinel();

  VisaData copyWith({
    String? country,
    String? visaType,
    String? number,
    String? issueDate,
    String? expiryDate,
    bool? isDeleted,
    Object? deletedAt = _sentinel,
  }) {
    return VisaData(
      country: country ?? this.country,
      visaType: visaType ?? this.visaType,
      number: number ?? this.number,
      issueDate: issueDate ?? this.issueDate,
      expiryDate: expiryDate ?? this.expiryDate,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: identical(deletedAt, _sentinel) ? this.deletedAt : deletedAt as DateTime?,
    );
  }
}

class FinancialData {
  List<BankAccountData> bankAccounts;
  List<CardData> cards;
  List<TaxIdData> taxIds;

  FinancialData({this.bankAccounts = const [], this.cards = const [], this.taxIds = const []});

  /// Filter out soft-deleted items
  List<BankAccountData> get activeBankAccounts =>
      bankAccounts.where((b) => !b.isDeleted).toList();

  List<CardData> get activeCards =>
      cards.where((c) => !c.isDeleted).toList();

  List<TaxIdData> get activeTaxIds =>
      taxIds.where((t) => !t.isDeleted).toList();

  /// Get soft-deleted items only
  List<BankAccountData> get deletedBankAccounts =>
      bankAccounts.where((b) => b.isDeleted).toList();

  List<CardData> get deletedCards =>
      cards.where((c) => c.isDeleted).toList();

  List<TaxIdData> get deletedTaxIds =>
      taxIds.where((t) => t.isDeleted).toList();

  factory FinancialData.fromJson(Map<String, dynamic> json) {
    return FinancialData(
      bankAccounts:
          (json['bank_accounts'] as List<dynamic>?)
              ?.map((e) => BankAccountData.fromJson(e))
              .toList() ??
          [],
      cards:
          (json['cards'] as List<dynamic>?)
              ?.map((e) => CardData.fromJson(e))
              .toList() ??
          [],
      taxIds:
          (json['tax_ids'] as List<dynamic>?)
              ?.map((e) => TaxIdData.fromJson(e))
              .toList() ??
          [],
    );
  }

  Map<String, dynamic> toJson() => {
    'bank_accounts': bankAccounts.map((e) => e.toJson()).toList(),
    'cards': cards.map((e) => e.toJson()).toList(),
    'tax_ids': taxIds.map((e) => e.toJson()).toList(),
  };

  FinancialData copyWith({
    List<BankAccountData>? bankAccounts,
    List<CardData>? cards,
    List<TaxIdData>? taxIds,
  }) {
    return FinancialData(
      bankAccounts: bankAccounts ?? this.bankAccounts,
      cards: cards ?? this.cards,
      taxIds: taxIds ?? this.taxIds,
    );
  }
}

class BankAccountData {
  String? bankName;
  String? accountNumber;
  String? currency;
  String? swiftBic;
  bool isDeleted;
  DateTime? deletedAt;

  BankAccountData({
    this.bankName,
    this.accountNumber,
    this.currency,
    this.swiftBic,
    this.isDeleted = false,
    this.deletedAt,
  });

  factory BankAccountData.fromJson(Map<String, dynamic> json) {
    return BankAccountData(
      bankName: json['bank_name'],
      accountNumber: json['account_number'],
      currency: json['currency'],
      swiftBic: json['swift_bic'],
      isDeleted: json['is_deleted'] ?? false,
      deletedAt: json['deleted_at'] != null
          ? DateTime.tryParse(json['deleted_at'])
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'bank_name': bankName,
    'account_number': accountNumber,
    'currency': currency,
    'swift_bic': swiftBic,
    'is_deleted': isDeleted,
    'deleted_at': deletedAt?.toIso8601String(),
  };

  static const _sentinel = _DeletedAtSentinel();

  BankAccountData copyWith({
    String? bankName,
    String? accountNumber,
    String? currency,
    String? swiftBic,
    bool? isDeleted,
    Object? deletedAt = _sentinel,
  }) {
    return BankAccountData(
      bankName: bankName ?? this.bankName,
      accountNumber: accountNumber ?? this.accountNumber,
      currency: currency ?? this.currency,
      swiftBic: swiftBic ?? this.swiftBic,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: identical(deletedAt, _sentinel) ? this.deletedAt : deletedAt as DateTime?,
    );
  }
}

class CardData {
  String? cardNumber;
  String? cardType;
  String? expiryDate;
  String? holderName;
  bool isDeleted;
  DateTime? deletedAt;

  CardData({
    this.cardNumber,
    this.cardType,
    this.expiryDate,
    this.holderName,
    this.isDeleted = false,
    this.deletedAt,
  });

  factory CardData.fromJson(Map<String, dynamic> json) {
    return CardData(
      cardNumber: json['card_number'],
      cardType: json['card_type'],
      expiryDate: json['expiry_date'],
      holderName: json['holder_name'],
      isDeleted: json['is_deleted'] ?? false,
      deletedAt: json['deleted_at'] != null
          ? DateTime.tryParse(json['deleted_at'])
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'card_number': cardNumber,
    'card_type': cardType,
    'expiry_date': expiryDate,
    'holder_name': holderName,
    'is_deleted': isDeleted,
    'deleted_at': deletedAt?.toIso8601String(),
  };

  static const _sentinel = _DeletedAtSentinel();

  CardData copyWith({
    String? cardNumber,
    String? cardType,
    String? expiryDate,
    String? holderName,
    bool? isDeleted,
    Object? deletedAt = _sentinel,
  }) {
    return CardData(
      cardNumber: cardNumber ?? this.cardNumber,
      cardType: cardType ?? this.cardType,
      expiryDate: expiryDate ?? this.expiryDate,
      holderName: holderName ?? this.holderName,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: identical(deletedAt, _sentinel) ? this.deletedAt : deletedAt as DateTime?,
    );
  }
}

class TaxIdData {
  String? taxIdNumber;
  String? taxIdType;
  String? issuingAuthority;
  String? country;
  bool isDeleted;
  DateTime? deletedAt;

  TaxIdData({
    this.taxIdNumber,
    this.taxIdType,
    this.issuingAuthority,
    this.country,
    this.isDeleted = false,
    this.deletedAt,
  });

  factory TaxIdData.fromJson(Map<String, dynamic> json) {
    return TaxIdData(
      taxIdNumber: json['tax_id_number'],
      taxIdType: json['tax_id_type'],
      issuingAuthority: json['issuing_authority'],
      country: json['country'],
      isDeleted: json['is_deleted'] ?? false,
      deletedAt: json['deleted_at'] != null
          ? DateTime.tryParse(json['deleted_at'])
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'tax_id_number': taxIdNumber,
    'tax_id_type': taxIdType,
    'issuing_authority': issuingAuthority,
    'country': country,
    'is_deleted': isDeleted,
    'deleted_at': deletedAt?.toIso8601String(),
  };

  static const _sentinel = _DeletedAtSentinel();

  TaxIdData copyWith({
    String? taxIdNumber,
    String? taxIdType,
    String? issuingAuthority,
    String? country,
    bool? isDeleted,
    Object? deletedAt = _sentinel,
  }) {
    return TaxIdData(
      taxIdNumber: taxIdNumber ?? this.taxIdNumber,
      taxIdType: taxIdType ?? this.taxIdType,
      issuingAuthority: issuingAuthority ?? this.issuingAuthority,
      country: country ?? this.country,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: identical(deletedAt, _sentinel) ? this.deletedAt : deletedAt as DateTime?,
    );
  }
}

class SkillData {
  String name;
  String? level;
  bool isDeleted;
  DateTime? deletedAt;

  SkillData({
    required this.name,
    this.level,
    this.isDeleted = false,
    this.deletedAt,
  });

  factory SkillData.fromJson(Map<String, dynamic> json) {
    return SkillData(
      name: json['name'] ?? '',
      level: json['level'],
      isDeleted: json['is_deleted'] ?? false,
      deletedAt: json['deleted_at'] != null
          ? DateTime.tryParse(json['deleted_at'])
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'name': name,
    'level': level,
    'is_deleted': isDeleted,
    'deleted_at': deletedAt?.toIso8601String(),
  };

  SkillData copyWith({
    String? name,
    String? level,
    bool? isDeleted,
    DateTime? deletedAt,
  }) {
    return SkillData(
      name: name ?? this.name,
      level: level ?? this.level,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: deletedAt ?? this.deletedAt,
    );
  }

  @override
  String toString() => level != null && level!.isNotEmpty ? '$name ($level)' : name;
}

class LanguageData {
  String name;
  String? proficiency;
  bool isDeleted;
  DateTime? deletedAt;

  LanguageData({
    required this.name,
    this.proficiency,
    this.isDeleted = false,
    this.deletedAt,
  });

  factory LanguageData.fromJson(Map<String, dynamic> json) {
    return LanguageData(
      name: json['name'] ?? '',
      proficiency: json['proficiency'],
      isDeleted: json['is_deleted'] ?? false,
      deletedAt: json['deleted_at'] != null
          ? DateTime.tryParse(json['deleted_at'])
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'name': name,
    'proficiency': proficiency,
    'is_deleted': isDeleted,
    'deleted_at': deletedAt?.toIso8601String(),
  };

  LanguageData copyWith({
    String? name,
    String? proficiency,
    bool? isDeleted,
    DateTime? deletedAt,
  }) {
    return LanguageData(
      name: name ?? this.name,
      proficiency: proficiency ?? this.proficiency,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: deletedAt ?? this.deletedAt,
    );
  }

  @override
  String toString() => name;
}

class ProfessionalData {
  List<EducationData> education;
  List<EmploymentData> employment;
  List<SkillData> skills;
  List<LanguageData> languages;

  ProfessionalData({
    this.education = const [],
    this.employment = const [],
    this.skills = const [],
    this.languages = const [],
  });

  /// Filter out soft-deleted items
  List<EducationData> get activeEducation =>
      education.where((e) => !e.isDeleted).toList();

  List<EmploymentData> get activeEmployment =>
      employment.where((e) => !e.isDeleted).toList();

  List<SkillData> get activeSkills =>
      skills.where((s) => !s.isDeleted).toList();

  List<LanguageData> get activeLanguages =>
      languages.where((l) => !l.isDeleted).toList();

  /// Get soft-deleted items only
  List<EducationData> get deletedEducation =>
      education.where((e) => e.isDeleted).toList();

  List<EmploymentData> get deletedEmployment =>
      employment.where((e) => e.isDeleted).toList();

  List<SkillData> get deletedSkills =>
      skills.where((s) => s.isDeleted).toList();

  List<LanguageData> get deletedLanguages =>
      languages.where((l) => l.isDeleted).toList();

  factory ProfessionalData.fromJson(Map<String, dynamic> json) {
    return ProfessionalData(
      education:
          (json['education'] as List<dynamic>?)
              ?.map((e) => EducationData.fromJson(e))
              .toList() ??
          [],
      employment:
          (json['employment'] as List<dynamic>?)
              ?.map((e) => EmploymentData.fromJson(e))
              .toList() ??
          [],
      skills:
          (json['skills'] as List<dynamic>?)
              ?.map((e) => SkillData.fromJson(e))
              .toList() ??
          [],
      languages:
          (json['languages'] as List<dynamic>?)
              ?.map((e) => LanguageData.fromJson(e))
              .toList() ??
          [],
    );
  }

  Map<String, dynamic> toJson() => {
    'education': education.map((e) => e.toJson()).toList(),
    'employment': employment.map((e) => e.toJson()).toList(),
    'skills': skills.map((s) => s.toJson()).toList(),
    'languages': languages.map((l) => l.toJson()).toList(),
  };

  ProfessionalData copyWith({
    List<EducationData>? education,
    List<EmploymentData>? employment,
    List<SkillData>? skills,
    List<LanguageData>? languages,
  }) {
    return ProfessionalData(
      education: education ?? this.education,
      employment: employment ?? this.employment,
      skills: skills ?? this.skills,
      languages: languages ?? this.languages,
    );
  }
}

class EducationData {
  String? institution;
  String? degree;
  String? field;
  String? startDate;
  String? endDate;
  bool isDeleted;
  DateTime? deletedAt;

  EducationData({
    this.institution,
    this.degree,
    this.field,
    this.startDate,
    this.endDate,
    this.isDeleted = false,
    this.deletedAt,
  });

  factory EducationData.fromJson(Map<String, dynamic> json) {
    return EducationData(
      institution: json['institution'],
      degree: json['degree'],
      field: json['field'],
      startDate: json['start_date'],
      endDate: json['end_date'],
      isDeleted: json['is_deleted'] ?? false,
      deletedAt: json['deleted_at'] != null
          ? DateTime.tryParse(json['deleted_at'])
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'institution': institution,
    'degree': degree,
    'field': field,
    'start_date': startDate,
    'end_date': endDate,
    'is_deleted': isDeleted,
    'deleted_at': deletedAt?.toIso8601String(),
  };

  static const _sentinel = _DeletedAtSentinel();

  EducationData copyWith({
    String? institution,
    String? degree,
    String? field,
    String? startDate,
    String? endDate,
    bool? isDeleted,
    Object? deletedAt = _sentinel,
  }) {
    return EducationData(
      institution: institution ?? this.institution,
      degree: degree ?? this.degree,
      field: field ?? this.field,
      startDate: startDate ?? this.startDate,
      endDate: endDate ?? this.endDate,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: identical(deletedAt, _sentinel) ? this.deletedAt : deletedAt as DateTime?,
    );
  }
}

class EmploymentData {
  String? company;
  String? position;
  String? startDate;
  String? endDate;
  bool isDeleted;
  DateTime? deletedAt;

  EmploymentData({
    this.company,
    this.position,
    this.startDate,
    this.endDate,
    this.isDeleted = false,
    this.deletedAt,
  });

  factory EmploymentData.fromJson(Map<String, dynamic> json) {
    return EmploymentData(
      company: json['company'],
      position: json['position'],
      startDate: json['start_date'],
      endDate: json['end_date'],
      isDeleted: json['is_deleted'] ?? false,
      deletedAt: json['deleted_at'] != null
          ? DateTime.tryParse(json['deleted_at'])
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'company': company,
    'position': position,
    'start_date': startDate,
    'end_date': endDate,
    'is_deleted': isDeleted,
    'deleted_at': deletedAt?.toIso8601String(),
  };

  static const _sentinel = _DeletedAtSentinel();

  EmploymentData copyWith({
    String? company,
    String? position,
    String? startDate,
    String? endDate,
    bool? isDeleted,
    Object? deletedAt = _sentinel,
  }) {
    return EmploymentData(
      company: company ?? this.company,
      position: position ?? this.position,
      startDate: startDate ?? this.startDate,
      endDate: endDate ?? this.endDate,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: identical(deletedAt, _sentinel) ? this.deletedAt : deletedAt as DateTime?,
    );
  }
}

/// Info about a soft-deleted item for trash view
class DeletedItemInfo {
  final String section;  // 'travel', 'financial', 'professional'
  final String itemType;  // 'passport', 'visa', 'bank_account', 'card', 'education', 'employment'
  final int index;        // index in the list
  final String itemLabel; // display name for UI
  final DateTime deletedAt;

  const DeletedItemInfo({
    required this.section,
    required this.itemType,
    required this.index,
    required this.itemLabel,
    required this.deletedAt,
  });
}

/// Profile storage service - stores encrypted profile data locally
/// Delegates to RustVaultService via FFI for SQLCipher-encrypted storage
class ProfileStorageService {
  static ProfileStorageService? _instance;

  // Reference to Rust vault service
  final RustVaultService _rustVault = RustVaultService.instance;

  ProfileStorageService._();

  static ProfileStorageService get instance {
    _instance ??= ProfileStorageService._();
    return _instance!;
  }

  /// Set the encryption key (derived from master password)
  /// Also sets it on the RustVaultService
  void setEncryptionKey(Uint8List key) {
    _rustVault.setEncryptionKey(key);
  }

  /// Get the encryption key (for use by OperationLogService)
  Uint8List? get encryptionKey => _rustVault.encryptionKey;

  /// Get the storage directory for logs and other files
  /// Uses the app's documents directory
  Future<Directory> get storageDir async {
    final appDir = await getApplicationDocumentsDirectory();
    return Directory('${appDir.path}/solosoul_storage');
  }

  /// Clear the encryption key (on lock)
  void clearEncryptionKey() {
    _rustVault.clearEncryptionKey();
  }

  /// Load profile data for an account
  /// Returns ProfileData with all fields decrypted, or null if not found
  Future<ProfileData?> loadProfile(String accountId) async {
    // Try to load from Rust vault
    final decrypted = await _rustVault.loadProfileDecrypted(accountId);
    if (decrypted == null) return null;

    try {
      final json = jsonDecode(decrypted) as Map<String, dynamic>;
      return ProfileData.fromJson(json);
    } catch (_) {
      return null;
    }
  }

  /// Save profile data for an account
  /// Encrypts and stores via RustVaultService
  Future<bool> saveProfile(String accountId, ProfileData profile) async {
    try {
      final json = jsonEncode(profile.toJson());
      final result = await _rustVault.saveProfileEncrypted(accountId, json);
      return result != null;
    } catch (_) {
      return false;
    }
  }

  /// Get all soft-deleted items across all sections
  List<DeletedItemInfo> getDeletedItems(ProfileData profile) {
    final items = <DeletedItemInfo>[];

    if (profile.travel != null) {
      for (var i = 0; i < profile.travel!.passports.length; i++) {
        final p = profile.travel!.passports[i];
        if (p.isDeleted) {
          items.add(DeletedItemInfo(
            section: 'travel',
            itemType: 'passport',
            index: i,
            itemLabel: p.country ?? 'Passport',
            deletedAt: p.deletedAt ?? DateTime.now(),
          ));
        }
      }
    }

    // Visa loop (separate from passport loop above)
    if (profile.travel != null) {
      for (var i = 0; i < profile.travel!.visas.length; i++) {
        final v = profile.travel!.visas[i];
        if (v.isDeleted) {
          items.add(DeletedItemInfo(
            section: 'travel',
            itemType: 'visa',
            index: i,
            itemLabel: v.country ?? 'Visa',
            deletedAt: v.deletedAt ?? DateTime.now(),
          ));
        }
      }
      for (var i = 0; i < profile.travel!.travelHistory.length; i++) {
        final t = profile.travel!.travelHistory[i];
        if (t.isDeleted) {
          items.add(DeletedItemInfo(
            section: 'travel',
            itemType: 'travel_history',
            index: i,
            itemLabel: t.destination,
            deletedAt: t.deletedAt ?? DateTime.now(),
          ));
        }
      }
    }

    // Financial section
    if (profile.financial != null) {
      for (var i = 0; i < profile.financial!.bankAccounts.length; i++) {
        final b = profile.financial!.bankAccounts[i];
        if (b.isDeleted) {
          items.add(DeletedItemInfo(
            section: 'financial',
            itemType: 'bank_account',
            index: i,
            itemLabel: b.bankName ?? 'Bank Account',
            deletedAt: b.deletedAt ?? DateTime.now(),
          ));
        }
      }
      for (var i = 0; i < profile.financial!.cards.length; i++) {
        final c = profile.financial!.cards[i];
        if (c.isDeleted) {
          items.add(DeletedItemInfo(
            section: 'financial',
            itemType: 'card',
            index: i,
            itemLabel: c.cardType ?? 'Card',
            deletedAt: c.deletedAt ?? DateTime.now(),
          ));
        }
      }
      for (var i = 0; i < profile.financial!.taxIds.length; i++) {
        final t = profile.financial!.taxIds[i];
        if (t.isDeleted) {
          items.add(DeletedItemInfo(
            section: 'financial',
            itemType: 'tax_id',
            index: i,
            itemLabel: t.taxIdType ?? 'Tax ID',
            deletedAt: t.deletedAt ?? DateTime.now(),
          ));
        }
      }
    }

    // Professional section
    if (profile.professional != null) {
      for (var i = 0; i < profile.professional!.education.length; i++) {
        final e = profile.professional!.education[i];
        if (e.isDeleted) {
          items.add(DeletedItemInfo(
            section: 'professional',
            itemType: 'education',
            index: i,
            itemLabel: e.institution ?? 'Education',
            deletedAt: e.deletedAt ?? DateTime.now(),
          ));
        }
      }
      for (var i = 0; i < profile.professional!.employment.length; i++) {
        final emp = profile.professional!.employment[i];
        if (emp.isDeleted) {
          items.add(DeletedItemInfo(
            section: 'professional',
            itemType: 'employment',
            index: i,
            itemLabel: emp.company ?? 'Employment',
            deletedAt: emp.deletedAt ?? DateTime.now(),
          ));
        }
      }
      for (var i = 0; i < profile.professional!.skills.length; i++) {
        final s = profile.professional!.skills[i];
        if (s.isDeleted) {
          items.add(DeletedItemInfo(
            section: 'professional',
            itemType: 'skill',
            index: i,
            itemLabel: s.toString(),
            deletedAt: s.deletedAt ?? DateTime.now(),
          ));
        }
      }
      for (var i = 0; i < profile.professional!.languages.length; i++) {
        final l = profile.professional!.languages[i];
        if (l.isDeleted) {
          items.add(DeletedItemInfo(
            section: 'professional',
            itemType: 'language',
            index: i,
            itemLabel: l.toString(),
            deletedAt: l.deletedAt ?? DateTime.now(),
          ));
        }
      }
    }

    // Profile/Identity section - contact entries
    if (profile.identity?.contact != null) {
      for (var i = 0; i < profile.identity!.contact!.entries.length; i++) {
        final e = profile.identity!.contact!.entries[i];
        if (e.isDeleted) {
          items.add(DeletedItemInfo(
            section: 'profile',
            itemType: 'contact',
            index: i,
            itemLabel: e.label.isNotEmpty ? '${e.label} - ${e.value}' : e.value,
            deletedAt: e.deletedAt ?? DateTime.now(),
          ));
        }
      }
    }

    // Profile/Identity section - ID cards
    if (profile.identity?.idCards != null) {
      for (var i = 0; i < profile.identity!.idCards!.length; i++) {
        final c = profile.identity!.idCards![i];
        if (c.isDeleted) {
          items.add(DeletedItemInfo(
            section: 'profile',
            itemType: 'idCard',
            index: i,
            itemLabel: c.label ?? c.number ?? 'ID Card',
            deletedAt: c.deletedAt ?? DateTime.now(),
          ));
        }
      }
    }

    // Profile/Identity section - addresses
    if (profile.identity?.addresses != null) {
      for (var i = 0; i < profile.identity!.addresses!.length; i++) {
        final a = profile.identity!.addresses![i];
        if (a.isDeleted) {
          items.add(DeletedItemInfo(
            section: 'profile',
            itemType: 'address',
            index: i,
            itemLabel: a.label ?? 'Address',
            deletedAt: a.deletedAt ?? DateTime.now(),
          ));
        }
      }
    }

    // Sort by deletedAt descending (most recent first)
    items.sort((a, b) => b.deletedAt.compareTo(a.deletedAt));
    return items;
  }

  /// Restore a soft-deleted item
  Future<void> restoreItem(
    ProfileData profile,
    String accountId,
    String section,
    String itemType,
    int index,
  ) async {
    switch (section) {
      case 'travel':
        if (profile.travel == null) return;
        if (itemType == 'passport' && index < profile.travel!.passports.length) {
          profile.travel!.passports[index] = profile.travel!.passports[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
        } else if (itemType == 'visa' && index < profile.travel!.visas.length) {
          profile.travel!.visas[index] = profile.travel!.visas[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
        }
        break;
      case 'financial':
        if (profile.financial == null) return;
        if (itemType == 'bank_account' && index < profile.financial!.bankAccounts.length) {
          profile.financial!.bankAccounts[index] = profile.financial!.bankAccounts[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
        } else if (itemType == 'card' && index < profile.financial!.cards.length) {
          profile.financial!.cards[index] = profile.financial!.cards[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
        } else if (itemType == 'tax_id' && index < profile.financial!.taxIds.length) {
          profile.financial!.taxIds[index] = profile.financial!.taxIds[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
        }
        break;
      case 'professional':
        if (profile.professional == null) return;
        if (itemType == 'education' && index < profile.professional!.education.length) {
          profile.professional!.education[index] = profile.professional!.education[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
        } else if (itemType == 'employment' && index < profile.professional!.employment.length) {
          profile.professional!.employment[index] = profile.professional!.employment[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
        } else if (itemType == 'skill' && index < profile.professional!.skills.length) {
          profile.professional!.skills[index] = profile.professional!.skills[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
        } else if (itemType == 'language' && index < profile.professional!.languages.length) {
          profile.professional!.languages[index] = profile.professional!.languages[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
        }
        break;
      case 'profile':
        if (profile.identity == null) return;
        if (itemType == 'contact' && index < (profile.identity!.contact?.entries.length ?? 0)) {
          final entries = List<ContactEntry>.from(profile.identity!.contact!.entries);
          entries[index] = entries[index].copyWith(isDeleted: false, deletedAt: null);
          profile.identity = profile.identity!.copyWith(
            contact: ContactData(entries: entries),
          );
        } else if (itemType == 'idCard' && index < (profile.identity!.idCards?.length ?? 0)) {
          final idCards = List<IdCardData>.from(profile.identity!.idCards!);
          idCards[index] = idCards[index].copyWith(isDeleted: false, deletedAt: null);
          profile.identity = profile.identity!.copyWith(idCards: idCards);
        } else if (itemType == 'address' && index < (profile.identity!.addresses?.length ?? 0)) {
          final addresses = List<AddressData>.from(profile.identity!.addresses!);
          addresses[index] = addresses[index].copyWith(isDeleted: false, deletedAt: null);
          profile.identity = profile.identity!.copyWith(addresses: addresses);
        }
        break;
    }
    await saveProfile(accountId, profile);
  }

  /// Permanently delete a specific item (removes from list completely)
  Future<void> permanentDeleteItem(
    ProfileData profile,
    String accountId,
    String section,
    String itemType,
    int index,
  ) async {
    switch (section) {
      case 'travel':
        if (profile.travel == null) return;
        if (itemType == 'passport' && index < profile.travel!.passports.length) {
          final updated = List<PassportData>.from(profile.travel!.passports);
          updated.removeAt(index);
          profile.travel = profile.travel!.copyWith(passports: updated);
        } else if (itemType == 'visa' && index < profile.travel!.visas.length) {
          final updated = List<VisaData>.from(profile.travel!.visas);
          updated.removeAt(index);
          profile.travel = profile.travel!.copyWith(visas: updated);
        }
        break;
      case 'financial':
        if (profile.financial == null) return;
        if (itemType == 'bank_account' && index < profile.financial!.bankAccounts.length) {
          final updated = List<BankAccountData>.from(profile.financial!.bankAccounts);
          updated.removeAt(index);
          profile.financial = profile.financial!.copyWith(bankAccounts: updated);
        } else if (itemType == 'card' && index < profile.financial!.cards.length) {
          final updated = List<CardData>.from(profile.financial!.cards);
          updated.removeAt(index);
          profile.financial = profile.financial!.copyWith(cards: updated);
        } else if (itemType == 'tax_id' && index < profile.financial!.taxIds.length) {
          final updated = List<TaxIdData>.from(profile.financial!.taxIds);
          updated.removeAt(index);
          profile.financial = profile.financial!.copyWith(taxIds: updated);
        }
        break;
      case 'professional':
        if (profile.professional == null) return;
        if (itemType == 'education' && index < profile.professional!.education.length) {
          final updated = List<EducationData>.from(profile.professional!.education);
          updated.removeAt(index);
          profile.professional = profile.professional!.copyWith(education: updated);
        } else if (itemType == 'employment' && index < profile.professional!.employment.length) {
          final updated = List<EmploymentData>.from(profile.professional!.employment);
          updated.removeAt(index);
          profile.professional = profile.professional!.copyWith(employment: updated);
        } else if (itemType == 'skill' && index < profile.professional!.skills.length) {
          final updated = List<SkillData>.from(profile.professional!.skills);
          updated.removeAt(index);
          profile.professional = profile.professional!.copyWith(skills: updated);
        } else if (itemType == 'language' && index < profile.professional!.languages.length) {
          final updated = List<LanguageData>.from(profile.professional!.languages);
          updated.removeAt(index);
          profile.professional = profile.professional!.copyWith(languages: updated);
        }
        break;
      case 'profile':
        if (profile.identity == null) return;
        if (itemType == 'contact' && index < (profile.identity!.contact?.entries.length ?? 0)) {
          final entries = List<ContactEntry>.from(profile.identity!.contact!.entries);
          entries.removeAt(index);
          profile.identity = profile.identity!.copyWith(
            contact: ContactData(entries: entries),
          );
        } else if (itemType == 'idCard' && index < (profile.identity!.idCards?.length ?? 0)) {
          final idCards = List<IdCardData>.from(profile.identity!.idCards!);
          idCards.removeAt(index);
          profile.identity = profile.identity!.copyWith(idCards: idCards);
        } else if (itemType == 'address' && index < (profile.identity!.addresses?.length ?? 0)) {
          final addresses = List<AddressData>.from(profile.identity!.addresses!);
          addresses.removeAt(index);
          profile.identity = profile.identity!.copyWith(addresses: addresses);
        }
        break;
    }
    await saveProfile(accountId, profile);
  }

  /// Permanently delete items older than 30 days
  Future<void> purgeOldDeletedItems(ProfileData profile, String accountId) async {
    final cutoff = DateTime.now().subtract(const Duration(days: 30));

    // Travel section
    if (profile.travel != null) {
      profile.travel!.passports.removeWhere(
        (p) => p.isDeleted && p.deletedAt != null && p.deletedAt!.isBefore(cutoff),
      );
      profile.travel!.visas.removeWhere(
        (v) => v.isDeleted && v.deletedAt != null && v.deletedAt!.isBefore(cutoff),
      );
    }

    // Financial section
    if (profile.financial != null) {
      profile.financial!.bankAccounts.removeWhere(
        (b) => b.isDeleted && b.deletedAt != null && b.deletedAt!.isBefore(cutoff),
      );
      profile.financial!.cards.removeWhere(
        (c) => c.isDeleted && c.deletedAt != null && c.deletedAt!.isBefore(cutoff),
      );
      profile.financial!.taxIds.removeWhere(
        (t) => t.isDeleted && t.deletedAt != null && t.deletedAt!.isBefore(cutoff),
      );
    }

    // Professional section
    if (profile.professional != null) {
      profile.professional!.education.removeWhere(
        (e) => e.isDeleted && e.deletedAt != null && e.deletedAt!.isBefore(cutoff),
      );
      profile.professional!.employment.removeWhere(
        (emp) => emp.isDeleted && emp.deletedAt != null && emp.deletedAt!.isBefore(cutoff),
      );
    }

    await saveProfile(accountId, profile);
  }

  /// Check and purge old deleted items (called on app startup)
  Future<void> purgeOldDeletedItemsIfNeeded(String accountId) async {
    final profile = await loadProfile(accountId);
    if (profile == null) return;

    final cutoff = DateTime.now().subtract(const Duration(days: 30));
    bool hasOldItems = false;

    // Check if any deleted items are older than 30 days
    if (profile.travel != null) {
      hasOldItems = hasOldItems ||
          profile.travel!.passports.any(
            (p) => p.isDeleted && p.deletedAt != null && p.deletedAt!.isBefore(cutoff),
          ) ||
          profile.travel!.visas.any(
            (v) => v.isDeleted && v.deletedAt != null && v.deletedAt!.isBefore(cutoff),
          );
    }
    if (profile.financial != null) {
      hasOldItems = hasOldItems ||
          profile.financial!.bankAccounts.any(
            (b) => b.isDeleted && b.deletedAt != null && b.deletedAt!.isBefore(cutoff),
          ) ||
          profile.financial!.cards.any(
            (c) => c.isDeleted && c.deletedAt != null && c.deletedAt!.isBefore(cutoff),
          ) ||
          profile.financial!.taxIds.any(
            (t) => t.isDeleted && t.deletedAt != null && t.deletedAt!.isBefore(cutoff),
          );
    }
    if (profile.professional != null) {
      hasOldItems = hasOldItems ||
          profile.professional!.education.any(
            (e) => e.isDeleted && e.deletedAt != null && e.deletedAt!.isBefore(cutoff),
          ) ||
          profile.professional!.employment.any(
            (emp) => emp.isDeleted && emp.deletedAt != null && emp.deletedAt!.isBefore(cutoff),
          );
    }

    if (hasOldItems) {
      await purgeOldDeletedItems(profile, accountId);
    }
  }

  /// Manually empty all trash (permanent delete all soft-deleted items)
  Future<void> emptyAllTrash(ProfileData profile, String accountId) async {
    // Travel section
    if (profile.travel != null) {
      profile.travel!.passports.removeWhere((p) => p.isDeleted);
      profile.travel!.visas.removeWhere((v) => v.isDeleted);
    }

    // Financial section
    if (profile.financial != null) {
      profile.financial!.bankAccounts.removeWhere((b) => b.isDeleted);
      profile.financial!.cards.removeWhere((c) => c.isDeleted);
      profile.financial!.taxIds.removeWhere((t) => t.isDeleted);
    }

    // Professional section
    if (profile.professional != null) {
      profile.professional!.education.removeWhere((e) => e.isDeleted);
      profile.professional!.employment.removeWhere((emp) => emp.isDeleted);
      profile.professional!.skills.removeWhere((s) => s.isDeleted);
      profile.professional!.languages.removeWhere((l) => l.isDeleted);
    }

    // Profile/Identity section
    if (profile.identity?.contact != null) {
      final entries = profile.identity!.contact!.entries.where((e) => !e.isDeleted).toList();
      profile.identity = profile.identity!.copyWith(contact: ContactData(entries: entries));
    }
    if (profile.identity?.idCards != null) {
      final idCards = profile.identity!.idCards!.where((c) => !c.isDeleted).toList();
      profile.identity = profile.identity!.copyWith(idCards: idCards);
    }
    if (profile.identity?.addresses != null) {
      final addresses = profile.identity!.addresses!.where((a) => !a.isDeleted).toList();
      profile.identity = profile.identity!.copyWith(addresses: addresses);
    }

    await saveProfile(accountId, profile);
  }
}
