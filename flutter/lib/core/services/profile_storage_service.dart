import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';
import 'package:flutter/material.dart';
import 'package:json_annotation/json_annotation.dart';
import 'package:path_provider/path_provider.dart';
import 'package:uuid/uuid.dart';
import 'package:solosoul_flutter/core/models/base_models.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/models/field_history_models.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/models/sensitivity_models.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';

part 'profile_storage_service.g.dart';

// Re-export for backward compatibility
typedef ProfileFieldHistories = FormHistories;

/// Maximum character limits for form fields
const int kMaxFieldLength = 32;
const int kMaxNameLength = 16;

const _uuid = Uuid();

/// Sentinel value for copyWith to distinguish "not provided" from "explicitly null"
class _DeletedAtSentinel {
  const _DeletedAtSentinel();
}

/// Generates a new unique ID using UUID v4
String generateEntryId() => _uuid.v4();

/// Returns current timestamp in milliseconds since epoch
int currentTimestamp() => DateTime.now().millisecondsSinceEpoch;

/// Profile data types matching Rust profile.rs
@JsonSerializable(explicitToJson: true)
class ProfileData {
  final IdentityData? identity;
  final TravelData? travel;
  final FinancialData? financial;
  final ProfessionalData? professional;
  final UnifiedObjectData? unifiedObjects;
  final int? schemaVersion;

  const ProfileData({
    this.identity,
    this.travel,
    this.financial,
    this.professional,
    this.unifiedObjects,
    this.schemaVersion,
  });

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
      unifiedObjects: json['unified_objects'] != null
          ? UnifiedObjectData.fromJson(json['unified_objects'])
          : null,
      schemaVersion: json['schema_version'] as int?,
    );
  }

  Map<String, dynamic> toJson() => {
    'identity': identity?.toJson(),
    'travel': travel?.toJson(),
    'financial': financial?.toJson(),
    'professional': professional?.toJson(),
    'unified_objects': unifiedObjects?.toJson(),
    'schema_version': schemaVersion,
  };

  ProfileData copyWith({
    IdentityData? identity,
    TravelData? travel,
    FinancialData? financial,
    ProfessionalData? professional,
    UnifiedObjectData? unifiedObjects,
    int? schemaVersion,
  }) {
    return ProfileData(
      identity: identity ?? this.identity,
      travel: travel ?? this.travel,
      financial: financial ?? this.financial,
      professional: professional ?? this.professional,
      unifiedObjects: unifiedObjects ?? this.unifiedObjects,
      schemaVersion: schemaVersion ?? this.schemaVersion,
    );
  }
}

class IdentityData {
  final String? fullName;
  final String? givenName;
  final String? familyName;
  final String? dateOfBirth;
  final String? gender;
  final String? nationality;
  final List<IdCardData>? idCards;
  final ContactData? contact;
  final List<AddressData>? addresses;

  const IdentityData({
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

@JsonSerializable()
class ContactEntry with FormattableEntry implements IdentifiableItem {
  @override
  final String id;
  final String title; // e.g., "Personal", "Work", "Emergency"
  final String type; // "email", "phone", "mobile"
  final String value;
  final int updatedAt;
  final bool isDeleted;
  final DateTime? deletedAt;

  @override
  String get entryType => 'Contact';

  @override
  Map<String, dynamic> toMap() => {
    'title': title,
    'type': type,
    'value': value,
  };

  ContactEntry({
    required this.id,
    required this.title,
    required this.type,
    required this.value,
    int? updatedAt,
    this.isDeleted = false,
    this.deletedAt,
  }) : updatedAt = updatedAt ?? currentTimestamp();

  /// Deprecated: using generated fromJson
  @Deprecated('Use the generated fromJson instead')
  factory ContactEntry.fromJson(Map<String, dynamic> json) =>
      _$ContactEntryFromJson(json);

  /// Deprecated: using generated toJson
  @Deprecated('Use the generated toJson instead')
  Map<String, dynamic> toJson() => _$ContactEntryToJson(this);

  static const _sentinel = _DeletedAtSentinel();

  ContactEntry copyWith({
    String? id,
    String? title,
    String? type,
    String? value,
    int? updatedAt,
    bool? isDeleted,
    Object? deletedAt = _sentinel,
  }) {
    return ContactEntry(
      id: id ?? this.id,
      title: title ?? this.title,
      type: type ?? this.type,
      value: value ?? this.value,
      updatedAt: updatedAt ?? this.updatedAt,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: identical(deletedAt, _sentinel)
          ? this.deletedAt
          : deletedAt as DateTime?,
    );
  }
}

@JsonSerializable(explicitToJson: true)
class ContactData {
  final List<ContactEntry> entries;

  ContactData({this.entries = const []});

  /// Deprecated: using generated fromJson
  @Deprecated('Use the generated fromJson instead')
  factory ContactData.fromJson(Map<String, dynamic> json) =>
      _$ContactDataFromJson(json);

  /// Deprecated: using generated toJson
  @Deprecated('Use the generated toJson instead')
  Map<String, dynamic> toJson() => _$ContactDataToJson(this);

  ContactData copyWith({List<ContactEntry>? entries}) {
    return ContactData(entries: entries ?? this.entries);
  }

  /// Filter out soft-deleted entries
  List<ContactEntry> get activeEntries =>
      entries.where((e) => !e.isDeleted).toList();
}

@JsonSerializable()
class AddressData with FormattableEntry implements IdentifiableItem {
  @override
  final String id;
  final String? title;
  final String? street;
  final String? city;
  final String? state;
  final String? postalCode;
  final String? country;
  final int updatedAt;
  final bool isDeleted;
  final DateTime? deletedAt;

  @override
  String get entryType => 'Address';

  @override
  Map<String, dynamic> toMap() => {
    'title': title,
    'street': street,
    'city': city,
    'state': state,
    'postalCode': postalCode,
    'country': country,
  };

  AddressData({
    required this.id,
    this.title,
    this.street,
    this.city,
    this.state,
    this.postalCode,
    this.country,
    int? updatedAt,
    this.isDeleted = false,
    this.deletedAt,
  }) : updatedAt = updatedAt ?? currentTimestamp();

  /// Deprecated: using generated fromJson
  @Deprecated('Use the generated fromJson instead')
  factory AddressData.fromJson(Map<String, dynamic> json) =>
      _$AddressDataFromJson(json);

  /// Deprecated: using generated toJson
  @Deprecated('Use the generated toJson instead')
  Map<String, dynamic> toJson() => _$AddressDataToJson(this);

  static const _sentinel = _DeletedAtSentinel();

  AddressData copyWith({
    String? id,
    String? title,
    String? street,
    String? city,
    String? state,
    String? postalCode,
    String? country,
    int? updatedAt,
    bool? isDeleted,
    Object? deletedAt = _sentinel,
  }) {
    return AddressData(
      id: id ?? this.id,
      title: title ?? this.title,
      street: street ?? this.street,
      city: city ?? this.city,
      state: state ?? this.state,
      postalCode: postalCode ?? this.postalCode,
      country: country ?? this.country,
      updatedAt: updatedAt ?? this.updatedAt,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: identical(deletedAt, _sentinel)
          ? this.deletedAt
          : deletedAt as DateTime?,
    );
  }
}

@JsonSerializable()
class IdCardData with FormattableEntry implements IdentifiableItem {
  @override
  final String id;
  final String? title;
  final String? number;
  final String? issueDate;
  final String? expiryDate;
  final String? holderName;
  final String? country;
  final int updatedAt;
  final bool isDeleted;
  final DateTime? deletedAt;

  @override
  String get entryType => 'IdCard';

  @override
  Map<String, dynamic> toMap() => {
    'title': title,
    'number': number,
    'issueDate': issueDate,
    'expiryDate': expiryDate,
    'holderName': holderName,
    'country': country,
  };

  IdCardData({
    required this.id,
    this.title,
    this.number,
    this.issueDate,
    this.expiryDate,
    this.holderName,
    this.country,
    int? updatedAt,
    this.isDeleted = false,
    this.deletedAt,
  }) : updatedAt = updatedAt ?? currentTimestamp();

  /// Deprecated: using generated fromJson
  @Deprecated('Use the generated fromJson instead')
  factory IdCardData.fromJson(Map<String, dynamic> json) =>
      _$IdCardDataFromJson(json);

  /// Deprecated: using generated toJson
  @Deprecated('Use the generated toJson instead')
  Map<String, dynamic> toJson() => _$IdCardDataToJson(this);

  static const _sentinel = _DeletedAtSentinel();

  IdCardData copyWith({
    String? id,
    String? title,
    String? number,
    String? issueDate,
    String? expiryDate,
    String? holderName,
    String? country,
    int? updatedAt,
    bool? isDeleted,
    Object? deletedAt = _sentinel,
  }) {
    return IdCardData(
      id: id ?? this.id,
      title: title ?? this.title,
      number: number ?? this.number,
      issueDate: issueDate ?? this.issueDate,
      expiryDate: expiryDate ?? this.expiryDate,
      holderName: holderName ?? this.holderName,
      country: country ?? this.country,
      updatedAt: updatedAt ?? this.updatedAt,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: identical(deletedAt, _sentinel)
          ? this.deletedAt
          : deletedAt as DateTime?,
    );
  }
}

@JsonSerializable()
class TravelHistoryData with FormattableEntry implements IdentifiableItem {
  @override
  final String id;
  final String destination;
  final String? date;
  final String? departureCity;
  final String? departureTime;
  final String? arrivalTime;
  final String? flightNumber;
  final String? ticketPrice;
  final String? airline;
  final String? travelType; // Airplane, Train, Bus, Taxi, Drive, Other
  final int updatedAt;
  final bool isDeleted;
  final DateTime? deletedAt;

  @override
  String get entryType => 'TravelHistory';

  @override
  Map<String, dynamic> toMap() => {
    'destination': destination,
    'date': date,
    'departureCity': departureCity,
    'departureTime': departureTime,
    'arrivalTime': arrivalTime,
    'flightNumber': flightNumber,
    'ticketPrice': ticketPrice,
    'airline': airline,
    'travelType': travelType,
  };

  TravelHistoryData({
    required this.id,
    required this.destination,
    this.date,
    this.departureCity,
    this.departureTime,
    this.arrivalTime,
    this.flightNumber,
    this.ticketPrice,
    this.airline,
    this.travelType,
    int? updatedAt,
    this.isDeleted = false,
    this.deletedAt,
  }) : updatedAt = updatedAt ?? currentTimestamp();

  /// Deprecated: using generated fromJson
  @Deprecated('Use the generated fromJson instead')
  factory TravelHistoryData.fromJson(Map<String, dynamic> json) =>
      _$TravelHistoryDataFromJson(json);

  /// Deprecated: using generated toJson
  @Deprecated('Use the generated toJson instead')
  Map<String, dynamic> toJson() => _$TravelHistoryDataToJson(this);

  static const _sentinel = _DeletedAtSentinel();

  TravelHistoryData copyWith({
    String? id,
    String? destination,
    String? date,
    String? departureCity,
    String? departureTime,
    String? arrivalTime,
    String? flightNumber,
    String? ticketPrice,
    String? airline,
    String? travelType,
    int? updatedAt,
    bool? isDeleted,
    Object? deletedAt = _sentinel,
  }) {
    return TravelHistoryData(
      id: id ?? this.id,
      destination: destination ?? this.destination,
      date: date ?? this.date,
      departureCity: departureCity ?? this.departureCity,
      departureTime: departureTime ?? this.departureTime,
      arrivalTime: arrivalTime ?? this.arrivalTime,
      flightNumber: flightNumber ?? this.flightNumber,
      ticketPrice: ticketPrice ?? this.ticketPrice,
      airline: airline ?? this.airline,
      travelType: travelType ?? this.travelType,
      updatedAt: updatedAt ?? this.updatedAt,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: identical(deletedAt, _sentinel)
          ? this.deletedAt
          : deletedAt as DateTime?,
    );
  }

  @override
  String toString() => destination;
}

@JsonSerializable(explicitToJson: true)
class TravelData {
  final List<PassportData> passports;
  final List<VisaData> visas;
  final List<TravelHistoryData> travelHistory;

  TravelData({
    this.passports = const [],
    this.visas = const [],
    this.travelHistory = const [],
  });

  /// Filter out soft-deleted items
  List<PassportData> get activePassports =>
      passports.where((p) => !p.isDeleted).toList();

  List<VisaData> get activeVisas => visas.where((v) => !v.isDeleted).toList();

  List<TravelHistoryData> get activeTravelHistory =>
      travelHistory.where((t) => !t.isDeleted).toList();

  /// Get soft-deleted items only
  List<PassportData> get deletedPassports =>
      passports.where((p) => p.isDeleted).toList();

  List<VisaData> get deletedVisas => visas.where((v) => v.isDeleted).toList();

  List<TravelHistoryData> get deletedTravelHistory =>
      travelHistory.where((t) => t.isDeleted).toList();

  /// Deprecated: using generated fromJson
  @Deprecated('Use the generated fromJson instead')
  factory TravelData.fromJson(Map<String, dynamic> json) =>
      _$TravelDataFromJson(json);

  /// Deprecated: using generated toJson
  @Deprecated('Use the generated toJson instead')
  Map<String, dynamic> toJson() => _$TravelDataToJson(this);

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

@JsonSerializable()
class PassportData with FormattableEntry implements IdentifiableItem {
  @override
  final String id;
  final String? title;
  final String? number;
  final String? country;
  final String? countryCode;
  final String? issueDate;
  final String? placeOfIssue;
  final String? expiryDate;
  final String? dateOfBirth;
  final String? placeOfBirth;
  final String? sex;
  final String? nationality;
  final String? authority;
  final String? holderName;
  final int updatedAt;
  final bool isDeleted;
  final DateTime? deletedAt;

  @override
  String get entryType => 'Passport';

  @override
  Map<String, dynamic> toMap() => {
    'title': title,
    'country': country,
    'countryCode': countryCode,
    'number': number,
    'issueDate': issueDate,
    'placeOfIssue': placeOfIssue,
    'expiryDate': expiryDate,
    'dateOfBirth': dateOfBirth,
    'placeOfBirth': placeOfBirth,
    'sex': sex,
    'nationality': nationality,
    'authority': authority,
    'holderName': holderName,
  };

  PassportData({
    required this.id,
    this.title,
    this.number,
    this.country,
    this.countryCode,
    this.issueDate,
    this.placeOfIssue,
    this.expiryDate,
    this.dateOfBirth,
    this.placeOfBirth,
    this.sex,
    this.nationality,
    this.authority,
    this.holderName,
    int? updatedAt,
    this.isDeleted = false,
    this.deletedAt,
  }) : updatedAt = updatedAt ?? currentTimestamp();

  /// Deprecated: using generated fromJson
  @Deprecated('Use the generated fromJson instead')
  factory PassportData.fromJson(Map<String, dynamic> json) =>
      _$PassportDataFromJson(json);

  /// Deprecated: using generated toJson
  @Deprecated('Use the generated toJson instead')
  Map<String, dynamic> toJson() => _$PassportDataToJson(this);

  static const _sentinel = _DeletedAtSentinel();

  PassportData copyWith({
    String? id,
    String? title,
    String? number,
    String? country,
    String? countryCode,
    String? issueDate,
    String? placeOfIssue,
    String? expiryDate,
    String? dateOfBirth,
    String? placeOfBirth,
    String? sex,
    String? nationality,
    String? authority,
    String? holderName,
    int? updatedAt,
    bool? isDeleted,
    Object? deletedAt = _sentinel,
  }) {
    return PassportData(
      id: id ?? this.id,
      title: title ?? this.title,
      number: number ?? this.number,
      country: country ?? this.country,
      countryCode: countryCode ?? this.countryCode,
      issueDate: issueDate ?? this.issueDate,
      placeOfIssue: placeOfIssue ?? this.placeOfIssue,
      expiryDate: expiryDate ?? this.expiryDate,
      dateOfBirth: dateOfBirth ?? this.dateOfBirth,
      placeOfBirth: placeOfBirth ?? this.placeOfBirth,
      sex: sex ?? this.sex,
      nationality: nationality ?? this.nationality,
      authority: authority ?? this.authority,
      holderName: holderName ?? this.holderName,
      updatedAt: updatedAt ?? this.updatedAt,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: identical(deletedAt, _sentinel)
          ? this.deletedAt
          : deletedAt as DateTime?,
    );
  }
}

@JsonSerializable()
class VisaData with FormattableEntry implements IdentifiableItem {
  @override
  final String id;
  final String? title;
  final String? country;
  final String? visaType;
  final String? number;
  final String? issueDate;
  final String? expiryDate;
  final int updatedAt;
  final bool isDeleted;
  final DateTime? deletedAt;

  @override
  String get entryType => 'Visa';

  @override
  Map<String, dynamic> toMap() => {
    'title': title,
    'country': country,
    'visaType': visaType,
    'number': number,
    'issueDate': issueDate,
    'expiryDate': expiryDate,
  };

  VisaData({
    required this.id,
    this.title,
    this.country,
    this.visaType,
    this.number,
    this.issueDate,
    this.expiryDate,
    int? updatedAt,
    this.isDeleted = false,
    this.deletedAt,
  }) : updatedAt = updatedAt ?? currentTimestamp();

  /// Deprecated: using generated fromJson
  @Deprecated('Use the generated fromJson instead')
  factory VisaData.fromJson(Map<String, dynamic> json) =>
      _$VisaDataFromJson(json);

  /// Deprecated: using generated toJson
  @Deprecated('Use the generated toJson instead')
  Map<String, dynamic> toJson() => _$VisaDataToJson(this);

  static const _sentinel = _DeletedAtSentinel();

  VisaData copyWith({
    String? id,
    String? title,
    String? country,
    String? visaType,
    String? number,
    String? issueDate,
    String? expiryDate,
    int? updatedAt,
    bool? isDeleted,
    Object? deletedAt = _sentinel,
  }) {
    return VisaData(
      id: id ?? this.id,
      title: title ?? this.title,
      country: country ?? this.country,
      visaType: visaType ?? this.visaType,
      number: number ?? this.number,
      issueDate: issueDate ?? this.issueDate,
      expiryDate: expiryDate ?? this.expiryDate,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: identical(deletedAt, _sentinel)
          ? this.deletedAt
          : deletedAt as DateTime?,
    );
  }
}

@JsonSerializable(explicitToJson: true)
class FinancialData {
  final List<BankAccountData> bankAccounts;
  final List<CardData> cards;
  final List<TaxIdData> taxIds;

  FinancialData({
    this.bankAccounts = const [],
    this.cards = const [],
    this.taxIds = const [],
  });

  /// Filter out soft-deleted items
  List<BankAccountData> get activeBankAccounts =>
      bankAccounts.where((b) => !b.isDeleted).toList();

  List<CardData> get activeCards => cards.where((c) => !c.isDeleted).toList();

  List<TaxIdData> get activeTaxIds =>
      taxIds.where((t) => !t.isDeleted).toList();

  /// Get soft-deleted items only
  List<BankAccountData> get deletedBankAccounts =>
      bankAccounts.where((b) => b.isDeleted).toList();

  List<CardData> get deletedCards => cards.where((c) => c.isDeleted).toList();

  List<TaxIdData> get deletedTaxIds =>
      taxIds.where((t) => t.isDeleted).toList();

  /// Deprecated: using generated fromJson
  @Deprecated('Use the generated fromJson instead')
  factory FinancialData.fromJson(Map<String, dynamic> json) =>
      _$FinancialDataFromJson(json);

  /// Deprecated: using generated toJson
  @Deprecated('Use the generated toJson instead')
  Map<String, dynamic> toJson() => _$FinancialDataToJson(this);

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

@JsonSerializable()
class BankAccountData with FormattableEntry implements IdentifiableItem {
  @override
  final String id;
  final String? title;
  final String? bankName;
  final String? accountNumber;
  final String? currency;
  final String? swiftBic;
  final String? sortCode;
  final int updatedAt;
  final bool isDeleted;
  final DateTime? deletedAt;

  @override
  String get entryType => 'BankAccount';

  @override
  Map<String, dynamic> toMap() => {
    'title': title,
    'bankName': bankName,
    'accountNumber': accountNumber,
    'currency': currency,
    'swiftBic': swiftBic,
    'sortCode': sortCode,
  };

  BankAccountData({
    required this.id,
    this.title,
    this.bankName,
    this.accountNumber,
    this.currency,
    this.swiftBic,
    this.sortCode,
    int? updatedAt,
    this.isDeleted = false,
    this.deletedAt,
  }) : updatedAt = updatedAt ?? currentTimestamp();

  /// Deprecated: using generated fromJson
  @Deprecated('Use the generated fromJson instead')
  factory BankAccountData.fromJson(Map<String, dynamic> json) =>
      _$BankAccountDataFromJson(json);

  /// Deprecated: using generated toJson
  @Deprecated('Use the generated toJson instead')
  Map<String, dynamic> toJson() => _$BankAccountDataToJson(this);

  static const _sentinel = _DeletedAtSentinel();

  BankAccountData copyWith({
    String? id,
    String? title,
    String? bankName,
    String? accountNumber,
    String? currency,
    String? swiftBic,
    String? sortCode,
    int? updatedAt,
    bool? isDeleted,
    Object? deletedAt = _sentinel,
  }) {
    return BankAccountData(
      id: id ?? this.id,
      title: title ?? this.title,
      bankName: bankName ?? this.bankName,
      accountNumber: accountNumber ?? this.accountNumber,
      currency: currency ?? this.currency,
      swiftBic: swiftBic ?? this.swiftBic,
      sortCode: sortCode ?? this.sortCode,
      updatedAt: updatedAt ?? this.updatedAt,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: identical(deletedAt, _sentinel)
          ? this.deletedAt
          : deletedAt as DateTime?,
    );
  }
}

@JsonSerializable()
class CardData with FormattableEntry implements IdentifiableItem {
  @override
  final String id;
  final String? title;
  final String? cardNumber;
  final String? cardType;
  final String? expiryDate;
  final String? holderName;
  final String? cvv;
  final int updatedAt;
  final bool isDeleted;
  final DateTime? deletedAt;

  @override
  String get entryType => 'Card';

  @override
  Map<String, dynamic> toMap() => {
    'title': title,
    'cardType': cardType,
    'cardNumber': cardNumber,
    'expiryDate': expiryDate,
    'holderName': holderName,
    'cvv': cvv,
  };

  CardData({
    required this.id,
    this.title,
    this.cardNumber,
    this.cardType,
    this.expiryDate,
    this.holderName,
    this.cvv,
    int? updatedAt,
    this.isDeleted = false,
    this.deletedAt,
  }) : updatedAt = updatedAt ?? currentTimestamp();

  /// Deprecated: using generated fromJson
  @Deprecated('Use the generated fromJson instead')
  factory CardData.fromJson(Map<String, dynamic> json) =>
      _$CardDataFromJson(json);

  /// Deprecated: using generated toJson
  @Deprecated('Use the generated toJson instead')
  Map<String, dynamic> toJson() => _$CardDataToJson(this);

  static const _sentinel = _DeletedAtSentinel();

  CardData copyWith({
    String? id,
    String? title,
    String? cardNumber,
    String? cardType,
    String? expiryDate,
    String? holderName,
    String? cvv,
    int? updatedAt,
    bool? isDeleted,
    Object? deletedAt = _sentinel,
  }) {
    return CardData(
      id: id ?? this.id,
      title: title ?? this.title,
      cardNumber: cardNumber ?? this.cardNumber,
      cardType: cardType ?? this.cardType,
      expiryDate: expiryDate ?? this.expiryDate,
      holderName: holderName ?? this.holderName,
      cvv: cvv ?? this.cvv,
      updatedAt: updatedAt ?? this.updatedAt,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: identical(deletedAt, _sentinel)
          ? this.deletedAt
          : deletedAt as DateTime?,
    );
  }
}

@JsonSerializable()
class TaxIdData with FormattableEntry implements IdentifiableItem {
  @override
  final String id;
  final String? title;
  final String? taxIdNumber;
  final String? taxIdType;
  final String? issuingAuthority;
  final String? country;
  final int updatedAt;
  final bool isDeleted;
  final DateTime? deletedAt;

  @override
  String get entryType => 'TaxId';

  @override
  Map<String, dynamic> toMap() => {
    'title': title,
    'taxIdNumber': taxIdNumber,
    'taxIdType': taxIdType,
    'issuingAuthority': issuingAuthority,
    'country': country,
  };

  TaxIdData({
    required this.id,
    this.title,
    this.taxIdNumber,
    this.taxIdType,
    this.issuingAuthority,
    this.country,
    int? updatedAt,
    this.isDeleted = false,
    this.deletedAt,
  }) : updatedAt = updatedAt ?? currentTimestamp();

  /// Deprecated: using generated fromJson
  @Deprecated('Use the generated fromJson instead')
  factory TaxIdData.fromJson(Map<String, dynamic> json) =>
      _$TaxIdDataFromJson(json);

  /// Deprecated: using generated toJson
  @Deprecated('Use the generated toJson instead')
  Map<String, dynamic> toJson() => _$TaxIdDataToJson(this);

  static const _sentinel = _DeletedAtSentinel();

  TaxIdData copyWith({
    String? id,
    String? title,
    String? taxIdNumber,
    String? taxIdType,
    String? issuingAuthority,
    String? country,
    int? updatedAt,
    bool? isDeleted,
    Object? deletedAt = _sentinel,
  }) {
    return TaxIdData(
      id: id ?? this.id,
      title: title ?? this.title,
      taxIdNumber: taxIdNumber ?? this.taxIdNumber,
      taxIdType: taxIdType ?? this.taxIdType,
      issuingAuthority: issuingAuthority ?? this.issuingAuthority,
      country: country ?? this.country,
      updatedAt: updatedAt ?? this.updatedAt,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: identical(deletedAt, _sentinel)
          ? this.deletedAt
          : deletedAt as DateTime?,
    );
  }
}

@JsonSerializable()
class SkillData with FormattableEntry implements IdentifiableItem {
  @override
  final String id;
  final String name;
  final String? level;
  final int updatedAt;
  final bool isDeleted;
  final DateTime? deletedAt;

  SkillData({
    required this.id,
    required this.name,
    this.level,
    int? updatedAt,
    this.isDeleted = false,
    this.deletedAt,
  }) : updatedAt = updatedAt ?? currentTimestamp();

  @override
  String get entryType => 'Skill';

  @override
  Map<String, dynamic> toMap() => {'name': name, 'level': level};

  /// Deprecated: using generated fromJson
  @Deprecated('Use the generated fromJson instead')
  factory SkillData.fromJson(Map<String, dynamic> json) =>
      _$SkillDataFromJson(json);

  /// Deprecated: using generated toJson
  @Deprecated('Use the generated toJson instead')
  Map<String, dynamic> toJson() => _$SkillDataToJson(this);

  SkillData copyWith({
    String? id,
    String? name,
    String? level,
    int? updatedAt,
    bool? isDeleted,
    DateTime? deletedAt,
  }) {
    return SkillData(
      id: id ?? this.id,
      name: name ?? this.name,
      level: level ?? this.level,
      updatedAt: updatedAt ?? this.updatedAt,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: deletedAt ?? this.deletedAt,
    );
  }

  @override
  String toString() =>
      level != null && level!.isNotEmpty ? '$name ($level)' : name;
}

@JsonSerializable()
class LanguageData with FormattableEntry implements IdentifiableItem {
  @override
  final String id;
  final String name;
  final String? proficiency;
  final int updatedAt;
  final bool isDeleted;
  final DateTime? deletedAt;

  LanguageData({
    required this.id,
    required this.name,
    this.proficiency,
    int? updatedAt,
    this.isDeleted = false,
    this.deletedAt,
  }) : updatedAt = updatedAt ?? currentTimestamp();

  @override
  String get entryType => 'Language';

  @override
  Map<String, dynamic> toMap() => {'name': name, 'proficiency': proficiency};

  /// Deprecated: using generated fromJson
  @Deprecated('Use the generated fromJson instead')
  factory LanguageData.fromJson(Map<String, dynamic> json) =>
      _$LanguageDataFromJson(json);

  /// Deprecated: using generated toJson
  @Deprecated('Use the generated toJson instead')
  Map<String, dynamic> toJson() => _$LanguageDataToJson(this);

  LanguageData copyWith({
    String? id,
    String? name,
    String? proficiency,
    int? updatedAt,
    bool? isDeleted,
    DateTime? deletedAt,
  }) {
    return LanguageData(
      id: id ?? this.id,
      name: name ?? this.name,
      proficiency: proficiency ?? this.proficiency,
      updatedAt: updatedAt ?? this.updatedAt,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: deletedAt ?? this.deletedAt,
    );
  }

  @override
  String toString() => name;
}

@JsonSerializable()
class AwardData with FormattableEntry implements IdentifiableItem {
  @override
  final String id;
  final String? title;
  final String? issuer;
  final String? date;
  final String? description;
  final int updatedAt;
  final bool isDeleted;
  final DateTime? deletedAt;

  AwardData({
    required this.id,
    this.title,
    this.issuer,
    this.date,
    this.description,
    int? updatedAt,
    this.isDeleted = false,
    this.deletedAt,
  }) : updatedAt = updatedAt ?? currentTimestamp();

  @override
  String get entryType => 'Award';

  @override
  Map<String, dynamic> toMap() => {
    'title': title,
    'issuer': issuer,
    'date': date,
    'description': description,
  };

  /// Deprecated: using generated fromJson
  @Deprecated('Use the generated fromJson instead')
  factory AwardData.fromJson(Map<String, dynamic> json) =>
      _$AwardDataFromJson(json);

  /// Deprecated: using generated toJson
  @Deprecated('Use the generated toJson instead')
  Map<String, dynamic> toJson() => _$AwardDataToJson(this);

  static const _sentinel = _DeletedAtSentinel();

  AwardData copyWith({
    String? id,
    String? title,
    String? issuer,
    String? date,
    String? description,
    int? updatedAt,
    bool? isDeleted,
    Object? deletedAt = _sentinel,
  }) {
    return AwardData(
      id: id ?? this.id,
      title: title ?? this.title,
      issuer: issuer ?? this.issuer,
      date: date ?? this.date,
      description: description ?? this.description,
      updatedAt: updatedAt ?? this.updatedAt,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: identical(deletedAt, _sentinel)
          ? this.deletedAt
          : deletedAt as DateTime?,
    );
  }
}

@JsonSerializable(explicitToJson: true)
class ProfessionalData {
  final List<EducationData> education;
  final List<EmploymentData> employment;
  final List<SkillData> skills;
  final List<LanguageData> languages;
  final List<AwardData> awards;

  ProfessionalData({
    this.education = const [],
    this.employment = const [],
    this.skills = const [],
    this.languages = const [],
    this.awards = const [],
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

  List<AwardData> get activeAwards =>
      awards.where((a) => !a.isDeleted).toList();

  /// Get soft-deleted items only
  List<EducationData> get deletedEducation =>
      education.where((e) => e.isDeleted).toList();

  List<EmploymentData> get deletedEmployment =>
      employment.where((e) => e.isDeleted).toList();

  List<SkillData> get deletedSkills =>
      skills.where((s) => s.isDeleted).toList();

  List<LanguageData> get deletedLanguages =>
      languages.where((l) => l.isDeleted).toList();

  List<AwardData> get deletedAwards =>
      awards.where((a) => a.isDeleted).toList();

  /// Deprecated: using generated fromJson
  @Deprecated('Use the generated fromJson instead')
  factory ProfessionalData.fromJson(Map<String, dynamic> json) =>
      _$ProfessionalDataFromJson(json);

  /// Deprecated: using generated toJson
  @Deprecated('Use the generated toJson instead')
  Map<String, dynamic> toJson() => _$ProfessionalDataToJson(this);

  ProfessionalData copyWith({
    List<EducationData>? education,
    List<EmploymentData>? employment,
    List<SkillData>? skills,
    List<LanguageData>? languages,
    List<AwardData>? awards,
  }) {
    return ProfessionalData(
      education: education ?? this.education,
      employment: employment ?? this.employment,
      skills: skills ?? this.skills,
      languages: languages ?? this.languages,
      awards: awards ?? this.awards,
    );
  }
}

@JsonSerializable()
class EducationData with FormattableEntry implements IdentifiableItem {
  @override
  final String id;
  final String? institution;
  final String? degree;
  final String? degreeCustom;
  final String? field;
  final String? startDate;
  final String? endDate;
  final int updatedAt;
  final bool isDeleted;
  final DateTime? deletedAt;

  EducationData({
    required this.id,
    this.institution,
    this.degree,
    this.degreeCustom,
    this.field,
    this.startDate,
    this.endDate,
    int? updatedAt,
    this.isDeleted = false,
    this.deletedAt,
  }) : updatedAt = updatedAt ?? currentTimestamp();

  @override
  String get entryType => 'Education';

  @override
  Map<String, dynamic> toMap() => {
    'institution': institution,
    'degree': degree,
    'degreeCustom': degreeCustom,
    'field': field,
    'startDate': startDate,
    'endDate': endDate,
  };

  /// Deprecated: using generated fromJson
  @Deprecated('Use the generated fromJson instead')
  factory EducationData.fromJson(Map<String, dynamic> json) =>
      _$EducationDataFromJson(json);

  /// Deprecated: using generated toJson
  @Deprecated('Use the generated toJson instead')
  Map<String, dynamic> toJson() => _$EducationDataToJson(this);

  static const _sentinel = _DeletedAtSentinel();

  EducationData copyWith({
    String? id,
    String? institution,
    String? degree,
    String? degreeCustom,
    String? field,
    String? startDate,
    String? endDate,
    int? updatedAt,
    bool? isDeleted,
    Object? deletedAt = _sentinel,
  }) {
    return EducationData(
      id: id ?? this.id,
      institution: institution ?? this.institution,
      degree: degree ?? this.degree,
      degreeCustom: degreeCustom ?? this.degreeCustom,
      field: field ?? this.field,
      startDate: startDate ?? this.startDate,
      endDate: endDate ?? this.endDate,
      updatedAt: updatedAt ?? this.updatedAt,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: identical(deletedAt, _sentinel)
          ? this.deletedAt
          : deletedAt as DateTime?,
    );
  }
}

@JsonSerializable()
class EmploymentData with FormattableEntry implements IdentifiableItem {
  @override
  final String id;
  final String? company;
  final String? position;
  final String? responsibilities;
  final String? startDate;
  final String? endDate;
  final int updatedAt;
  final bool isDeleted;
  final DateTime? deletedAt;

  EmploymentData({
    required this.id,
    this.company,
    this.position,
    this.responsibilities,
    this.startDate,
    this.endDate,
    int? updatedAt,
    this.isDeleted = false,
    this.deletedAt,
  }) : updatedAt = updatedAt ?? currentTimestamp();

  @override
  String get entryType => 'Employment';

  @override
  Map<String, dynamic> toMap() => {
    'company': company,
    'position': position,
    'responsibilities': responsibilities,
    'startDate': startDate,
    'endDate': endDate,
  };

  /// Deprecated: using generated fromJson
  @Deprecated('Use the generated fromJson instead')
  factory EmploymentData.fromJson(Map<String, dynamic> json) =>
      _$EmploymentDataFromJson(json);

  /// Deprecated: using generated toJson
  @Deprecated('Use the generated toJson instead')
  Map<String, dynamic> toJson() => _$EmploymentDataToJson(this);

  static const _sentinel = _DeletedAtSentinel();

  EmploymentData copyWith({
    String? id,
    String? company,
    String? position,
    String? responsibilities,
    String? startDate,
    String? endDate,
    int? updatedAt,
    bool? isDeleted,
    Object? deletedAt = _sentinel,
  }) {
    return EmploymentData(
      id: id ?? this.id,
      company: company ?? this.company,
      position: position ?? this.position,
      responsibilities: responsibilities ?? this.responsibilities,
      startDate: startDate ?? this.startDate,
      endDate: endDate ?? this.endDate,
      updatedAt: updatedAt ?? this.updatedAt,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: identical(deletedAt, _sentinel)
          ? this.deletedAt
          : deletedAt as DateTime?,
    );
  }
}

/// Info about a soft-deleted item for trash view
class DeletedItemInfo {
  final String section;
  final String itemType;
  final String id;
  final String itemLabel;
  final DateTime deletedAt;

  const DeletedItemInfo({
    required this.section,
    required this.itemType,
    required this.id,
    required this.itemLabel,
    required this.deletedAt,
  });

  /// Metadata configuration for each item type
  static const Map<String, ItemTypeMeta> _metaByType = {
    'passport': ItemTypeMeta(
      label: 'Passport',
      sectionLabel: 'Travel',
      icon: Icons.flight,
      fieldIdPrefix: 'passport',
      sensitivityFieldId: 'passport.number',
      section: 'travel',
    ),
    'visa': ItemTypeMeta(
      label: 'Visa',
      sectionLabel: 'Travel',
      icon: Icons.flight,
      fieldIdPrefix: 'visa',
      sensitivityFieldId: 'visa.number',
      section: 'travel',
    ),
    'travel_history': ItemTypeMeta(
      label: 'Travel History',
      sectionLabel: 'Travel',
      icon: Icons.history,
      fieldIdPrefix: 'travel',
      sensitivityFieldId: 'travel.date',
      section: 'travel',
    ),
    'bank_account': ItemTypeMeta(
      label: 'Bank Account',
      sectionLabel: 'Financial',
      icon: Icons.account_balance,
      fieldIdPrefix: 'bankAccount',
      sensitivityFieldId: 'bankAccount.accountNumber',
      section: 'financial',
    ),
    'card': ItemTypeMeta(
      label: 'Card',
      sectionLabel: 'Financial',
      icon: Icons.credit_card,
      fieldIdPrefix: 'card',
      sensitivityFieldId: 'card.cardNumber',
      section: 'financial',
    ),
    'education': ItemTypeMeta(
      label: 'Education',
      sectionLabel: 'Professional',
      icon: Icons.school,
      fieldIdPrefix: 'education',
      sensitivityFieldId: 'education.gpa',
      section: 'professional',
    ),
    'employment': ItemTypeMeta(
      label: 'Employment',
      sectionLabel: 'Professional',
      icon: Icons.work,
      fieldIdPrefix: 'employment',
      sensitivityFieldId: 'employment.monthlySalary',
      section: 'professional',
    ),
    'skill': ItemTypeMeta(
      label: 'Skill',
      sectionLabel: 'Professional',
      icon: Icons.star,
      fieldIdPrefix: 'skill',
      sensitivityFieldId: 'skill.name',
      section: 'professional',
    ),
    'language': ItemTypeMeta(
      label: 'Language',
      sectionLabel: 'Professional',
      icon: Icons.language,
      fieldIdPrefix: 'language',
      sensitivityFieldId: 'language.name',
      section: 'professional',
    ),
    'contact': ItemTypeMeta(
      label: 'Contact',
      sectionLabel: 'Profile',
      icon: Icons.person,
      fieldIdPrefix: 'contact',
      sensitivityFieldId: 'contact.email',
      section: 'profile',
    ),
    'idCard': ItemTypeMeta(
      label: 'ID Card',
      sectionLabel: 'Profile',
      icon: Icons.badge,
      fieldIdPrefix: 'idCard.number',
      sensitivityFieldId: 'idCard.number',
      section: 'profile',
    ),
    'address': ItemTypeMeta(
      label: 'Address',
      sectionLabel: 'Profile',
      icon: Icons.home,
      fieldIdPrefix: 'address.postalCode',
      sensitivityFieldId: 'address.postalCode',
      section: 'profile',
    ),
  };

  ItemTypeMeta? get meta => _metaByType[itemType];

  /// Static accessor for meta by item type (used by trash_page)
  static ItemTypeMeta? metaFor(String itemType) => _metaByType[itemType];

  /// All defined item types — single source of truth for item type enumeration.
  /// Use this instead of hardcoding type sets to stay DRY.
  static Iterable<String> get itemTypes => _metaByType.keys;
}

/// Metadata configuration for deleted item types
class ItemTypeMeta {
  final String label;
  final String sectionLabel;
  final IconData icon;
  final String fieldIdPrefix;
  final String sensitivityFieldId;
  final String section;

  const ItemTypeMeta({
    required this.label,
    required this.sectionLabel,
    required this.icon,
    required this.fieldIdPrefix,
    required this.sensitivityFieldId,
    required this.section,
  });
}

/// Profile storage service - stores encrypted profile data locally
/// Delegates to RustVaultService via FFI for SQLCipher-encrypted storage
// TODO: [P2] ProfileStorageService is 700+ lines - consider extracting:
// - DeletedItemInfo caching logic to a separate service
// - restoreItem/permanentDeleteItem to a TrashService class
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
    _invalidateDeletedItemsCache();
  }

  // Caching for getDeletedItems - invalidated on any profile mutation
  List<DeletedItemInfo>? _cachedDeletedItems;

  void _invalidateDeletedItemsCache() {
    _cachedDeletedItems = null;
  }

  /// Current schema version for unified object model.
  static const int kSchemaVersion = 4;

  /// Migrate profile to latest schema if needed.
  /// v3: Unified object model - everything is a UnifiedObject.
  /// v4: Default pages (profile/travel/financial/professional) migrated to UnifiedObject tree.
  ProfileData _migrateIfNeeded(ProfileData profile, Map<String, dynamic> rawJson) {
    var currentVersion = profile.schemaVersion ?? 0;
    var migrated = profile;

    // Recovery guard: if unifiedObjects is empty/missing default pages but
    // legacy fields still have data, re-run migration regardless of schemaVersion.
    // This handles cases where unifiedObjects was accidentally wiped.
    final unifiedObjects = migrated.unifiedObjects;
    final hasDefaultPages = unifiedObjects?.objects.any(
          (o) => o.id == DefaultPageIds.profile || o.id == DefaultPageIds.travel,
        ) ?? false;
    final hasLegacyData = migrated.identity != null ||
        migrated.travel != null ||
        migrated.financial != null ||
        migrated.professional != null;

    if (!hasDefaultPages && hasLegacyData) {
      if (currentVersion < 3) {
        if (unifiedObjects == null || unifiedObjects.objects.isEmpty) {
          final unifiedData = _migrateLegacyToUnified(rawJson);
          migrated = migrated.copyWith(unifiedObjects: unifiedData);
        }
        currentVersion = 3;
      }
      final existingData = migrated.unifiedObjects ?? const UnifiedObjectData();
      final migratedObjects = _migrateProfileDataToUnified(migrated, existingData);
      return migrated.copyWith(
        unifiedObjects: migratedObjects,
        schemaVersion: kSchemaVersion,
      );
    }

    if (currentVersion >= kSchemaVersion) return profile;

    // v0/v1/v2 → v3: migrate any legacy flexibleObjects/flexibleSections to UnifiedObjectData
    if (currentVersion < 3) {
      if (migrated.unifiedObjects == null || migrated.unifiedObjects!.objects.isEmpty) {
        final unifiedData = _migrateLegacyToUnified(rawJson);
        migrated = migrated.copyWith(
          unifiedObjects: unifiedData,
          schemaVersion: 3,
        );
      } else {
        migrated = migrated.copyWith(schemaVersion: 3);
      }
      currentVersion = 3;
    }

    // v3 → v4: migrate default page data (identity/travel/financial/professional)
    // into the UnifiedObject tree with predefined schemas.
    if (currentVersion < 4) {
      final existingData = migrated.unifiedObjects ?? const UnifiedObjectData();
      final hasDefaultPages = existingData.objects.any(
        (o) => o.id == DefaultPageIds.profile || o.id == DefaultPageIds.travel,
      );
      if (!hasDefaultPages) {
        final migratedObjects = _migrateProfileDataToUnified(migrated, existingData);
        migrated = migrated.copyWith(
          unifiedObjects: migratedObjects,
          schemaVersion: kSchemaVersion,
        );
      } else {
        migrated = migrated.copyWith(schemaVersion: kSchemaVersion);
      }
    }

    return migrated;
  }

  /// Migrate legacy flexibleSections / flexibleObjects to UnifiedObjectData.
  /// Operates on raw JSON maps because old type definitions have been removed.
  UnifiedObjectData _migrateLegacyToUnified(Map<String, dynamic> rawJson) {
    final objects = <UnifiedObject>[];
    final timestamp = currentTimestamp();

    String? parseString(dynamic v) => v?.toString();
    bool parseBool(dynamic v) => v == true || v == 'true';
    int? parseMillis(dynamic v) => v is int ? v : (v is num ? v.toInt() : null);
    DateTime? parseDateTime(dynamic v) {
      if (v == null) return null;
      if (v is String) return DateTime.tryParse(v);
      return null;
    }

    // -------------------------------------------------------------------------
    // Path A: old flexibleObjects (v2/v3-early FlexibleObject model)
    // -------------------------------------------------------------------------
    final legacyObjectsRaw = rawJson['flexible_objects'] as Map<String, dynamic>?;
    final legacyObjects = legacyObjectsRaw != null
        ? (legacyObjectsRaw['objects'] as List<dynamic>? ?? [])
            .map((e) => e as Map<String, dynamic>)
            .toList()
        : const <Map<String, dynamic>>[];
    if (legacyObjects.isNotEmpty) {
      final childrenByParent = <String, List<String>>{};
      for (final o in legacyObjects) {
        final parentId = parseString(o['parentId']);
        if (parentId != null) {
          childrenByParent.putIfAbsent(parentId, () => []).add(parseString(o['id']) ?? generateEntryId());
        }
      }

      for (final o in legacyObjects) {
        final objectTypeName = parseString(o['objectType']) ?? 'item';
        final typeId = switch (objectTypeName) {
          'page' => 'page',
          'section' => 'collection',
          'item' || _ => 'note',
        };
        final id = parseString(o['id']) ?? generateEntryId();
        objects.add(UnifiedObject(
          id: id,
          typeId: typeId,
          name: parseString(o['name']) ?? 'Untitled',
          iconName: parseString(o['iconName']) ?? 'folder',
          parentId: parseString(o['parentId']),
          childrenIds: childrenByParent[id] ?? const [],
          properties: const {}, // Legacy properties used old PropertyValue; safest to drop
          isDeleted: parseBool(o['isDeleted']),
          deletedAt: parseDateTime(o['deletedAt']),
          createdAt: timestamp,
          updatedAt: parseMillis(o['updatedAt']) ?? timestamp,
        ));
      }
      return UnifiedObjectData(objects: objects);
    }

    // -------------------------------------------------------------------------
    // Path B: old flexibleSections (v1 FlexibleSection model)
    // -------------------------------------------------------------------------
    final legacySectionsRaw = rawJson['flexible_sections'] as Map<String, dynamic>?;
    final legacySections = legacySectionsRaw != null
        ? (legacySectionsRaw['sections'] as List<dynamic>? ?? [])
            .map((e) => e as Map<String, dynamic>)
            .toList()
        : const <Map<String, dynamic>>[];
    if (legacySections.isNotEmpty) {
      for (final section in legacySections) {
        if (parseBool(section['isDeleted'])) continue;

        final sectionId = parseString(section['id']) ?? generateEntryId();
        final sectionTitle = parseString(section['title']) ?? 'Untitled';
        final sectionIcon = parseString(section['iconName']) ?? 'folder';
        final pageId = 'page_$sectionId';

        // Create a synthetic page for this section
        objects.add(UnifiedObject(
          id: pageId,
          typeId: 'page',
          name: sectionTitle,
          iconName: sectionIcon,
          parentId: null,
          childrenIds: [sectionId],
          createdAt: timestamp,
          updatedAt: timestamp,
        ));

        // Convert section items
        final itemIds = <String>[];
        final items = (section['items'] as List<dynamic>? ?? [])
            .map((e) => e as Map<String, dynamic>)
            .toList();
        for (final item in items) {
          if (parseBool(item['isDeleted'])) continue;
          final itemId = parseString(item['id']) ?? generateEntryId();
          itemIds.add(itemId);
          objects.add(UnifiedObject(
            id: itemId,
            typeId: 'note',
            name: parseString(item['title']) ?? 'Untitled',
            iconName: 'description',
            parentId: sectionId,
            properties: {
              'data': TextProperty(text: jsonEncode(item['data'])),
            },
            createdAt: timestamp,
            updatedAt: parseMillis(item['updatedAt']) ?? timestamp,
          ));
        }

        // Convert section to collection
        objects.add(UnifiedObject(
          id: sectionId,
          typeId: 'collection',
          name: sectionTitle,
          iconName: sectionIcon,
          parentId: pageId,
          childrenIds: itemIds,
          isDeleted: false,
          deletedAt: parseDateTime(section['deletedAt']),
          createdAt: timestamp,
          updatedAt: parseMillis(section['updatedAt']) ?? timestamp,
        ));
      }
      return UnifiedObjectData(objects: objects);
    }

    // Empty default
    return const UnifiedObjectData();
  }

  /// Migrate legacy profile data (identity/travel/financial/professional) to
  /// the UnifiedObject tree. Creates default pages, sections, and items.
  UnifiedObjectData _migrateProfileDataToUnified(
    ProfileData profile,
    UnifiedObjectData existingData,
  ) {
    final objects = List<UnifiedObject>.from(existingData.objects);
    final timestamp = currentTimestamp();

    TextProperty prop(String? value, SensitivityLevel sensitivity) {
      return TextProperty(text: value ?? '', sensitivity: sensitivity);
    }

    // Helper to look up sensitivity from FieldRegistry / FormFieldRegistry
    SensitivityLevel sens(String fieldId) {
      // 1. Try runtime-registered FormFieldRegistry first (contact.* etc.)
      final formField = FormFieldRegistry.getField(fieldId);
      if (formField != null) return formField.level;
      // 2. Fallback to static FieldRegistry defaults
      try {
        return FieldRegistry.defaultFields
            .firstWhere((f) => f.fieldId == fieldId)
            .level;
      } on Exception catch (_) {
        return SensitivityLevel.public;
      }
    }

    // -------------------------------------------------------------------------
    // Profile page
    // -------------------------------------------------------------------------
    final profileSectionChildren = <String>[];

    // Identity section (single item)
    if (profile.identity != null) {
      final identity = profile.identity!;
      final identityId = generateEntryId();
      profileSectionChildren.add(identityId);
      objects.add(UnifiedObject(
        id: identityId,
        typeId: 'profile_identity',
        name: identity.fullName ?? 'Identity',
        parentId: DefaultSectionIds.identity,
        properties: {
          'fullName': prop(identity.fullName, sens('identity.fullName')),
          'givenName': prop(identity.givenName, sens('identity.givenName')),
          'familyName': prop(identity.familyName, sens('identity.familyName')),
          'dateOfBirth': prop(identity.dateOfBirth, sens('identity.dateOfBirth')),
          'gender': prop(identity.gender, sens('identity.gender')),
          'nationality': prop(identity.nationality, sens('identity.nationality')),
        },
        createdAt: timestamp,
        updatedAt: timestamp,
      ));
    }

    // Contact items
    final contactChildren = <String>[];
    final contactEntries = profile.identity?.contact?.entries ?? [];
    for (final entry in contactEntries) {
      contactChildren.add(entry.id);
      objects.add(UnifiedObject(
        id: entry.id,
        typeId: 'profile_contact',
        name: entry.title.isNotEmpty ? entry.title : entry.value,
        parentId: DefaultSectionIds.contact,
        properties: {
          'title': prop(entry.title, sens('contact.title')),
          'type': prop(entry.type, sens('contact.type')),
          'value': prop(entry.value, sens('contact.value')),
        },
        isDeleted: entry.isDeleted,
        deletedAt: entry.deletedAt,
        createdAt: timestamp,
        updatedAt: entry.updatedAt,
      ));
    }

    // ID Card items
    final idCardChildren = <String>[];
    final idCards = profile.identity?.idCards ?? [];
    for (final card in idCards) {
      idCardChildren.add(card.id);
      objects.add(UnifiedObject(
        id: card.id,
        typeId: 'profile_id_card',
        name: card.title ?? 'ID Card',
        parentId: DefaultSectionIds.idCard,
        properties: {
          'title': prop(card.title, sens('idCard.title')),
          'number': prop(card.number, sens('idCard.number')),
          'issueDate': prop(card.issueDate, sens('idCard.issueDate')),
          'expiryDate': prop(card.expiryDate, sens('idCard.expiryDate')),
          'holderName': prop(card.holderName, sens('idCard.holderName')),
          'country': prop(card.country, sens('idCard.country')),
        },
        isDeleted: card.isDeleted,
        deletedAt: card.deletedAt,
        createdAt: timestamp,
        updatedAt: card.updatedAt,
      ));
    }

    // Address items
    final addressChildren = <String>[];
    final addresses = profile.identity?.addresses ?? [];
    for (final addr in addresses) {
      addressChildren.add(addr.id);
      objects.add(UnifiedObject(
        id: addr.id,
        typeId: 'profile_address',
        name: addr.title ?? 'Address',
        parentId: DefaultSectionIds.address,
        properties: {
          'title': prop(addr.title, sens('address.title')),
          'street': prop(addr.street, sens('address.street')),
          'city': prop(addr.city, sens('address.city')),
          'state': prop(addr.state, SensitivityLevel.public),
          'postalCode': prop(addr.postalCode, sens('address.postalCode')),
          'country': prop(addr.country, sens('address.country')),
        },
        isDeleted: addr.isDeleted,
        deletedAt: addr.deletedAt,
        createdAt: timestamp,
        updatedAt: addr.updatedAt,
      ));
    }

    // Build profile sections
    objects.add(UnifiedObject(
      id: DefaultSectionIds.identity,
      typeId: 'collection',
      name: 'Identity',
      iconName: 'person',
      parentId: DefaultPageIds.profile,
      childrenIds: profile.identity != null ? [profileSectionChildren.first] : const [],
      createdAt: timestamp,
      updatedAt: timestamp,
    ));
    objects.add(UnifiedObject(
      id: DefaultSectionIds.contact,
      typeId: 'collection',
      name: 'Contact Information',
      iconName: 'contact_mail',
      parentId: DefaultPageIds.profile,
      childrenIds: contactChildren,
      createdAt: timestamp,
      updatedAt: timestamp,
    ));
    objects.add(UnifiedObject(
      id: DefaultSectionIds.idCard,
      typeId: 'collection',
      name: 'ID Cards',
      iconName: 'badge',
      parentId: DefaultPageIds.profile,
      childrenIds: idCardChildren,
      createdAt: timestamp,
      updatedAt: timestamp,
    ));
    objects.add(UnifiedObject(
      id: DefaultSectionIds.address,
      typeId: 'collection',
      name: 'Addresses',
      iconName: 'home',
      parentId: DefaultPageIds.profile,
      childrenIds: addressChildren,
      createdAt: timestamp,
      updatedAt: timestamp,
    ));
    objects.add(UnifiedObject(
      id: DefaultPageIds.profile,
      typeId: 'page',
      name: 'Profile',
      iconName: 'person',
      parentId: null,
      childrenIds: const [
        DefaultSectionIds.identity,
        DefaultSectionIds.contact,
        DefaultSectionIds.idCard,
        DefaultSectionIds.address,
      ],
      createdAt: timestamp,
      updatedAt: timestamp,
    ));

    // -------------------------------------------------------------------------
    // Travel page
    // -------------------------------------------------------------------------
    final passports = profile.travel?.passports ?? [];
    final passportChildren = <String>[];
    for (final p in passports) {
      passportChildren.add(p.id);
      objects.add(UnifiedObject(
        id: p.id,
        typeId: 'travel_passport',
        name: p.title ?? p.country ?? 'Passport',
        parentId: DefaultSectionIds.passport,
        properties: {
          'title': prop(p.title, sens('passport.title')),
          'country': prop(p.country, sens('passport.country')),
          'countryCode': prop(p.countryCode, sens('passport.countryCode')),
          'number': prop(p.number, sens('passport.number')),
          'issueDate': prop(p.issueDate, sens('passport.issueDate')),
          'placeOfIssue': prop(p.placeOfIssue, sens('passport.placeOfIssue')),
          'expiryDate': prop(p.expiryDate, sens('passport.expiryDate')),
          'holderName': prop(p.holderName, sens('passport.holderName')),
          'dateOfBirth': prop(p.dateOfBirth, sens('passport.dateOfBirth')),
          'placeOfBirth': prop(p.placeOfBirth, sens('passport.placeOfBirth')),
          'sex': prop(p.sex, sens('passport.sex')),
          'nationality': prop(p.nationality, sens('passport.nationality')),
          'authority': prop(p.authority, sens('passport.authority')),
        },
        isDeleted: p.isDeleted,
        deletedAt: p.deletedAt,
        createdAt: timestamp,
        updatedAt: p.updatedAt,
      ));
    }

    final visas = profile.travel?.visas ?? [];
    final visaChildren = <String>[];
    for (final v in visas) {
      visaChildren.add(v.id);
      objects.add(UnifiedObject(
        id: v.id,
        typeId: 'travel_visa',
        name: v.title ?? v.country ?? 'Visa',
        parentId: DefaultSectionIds.visa,
        properties: {
          'title': prop(v.title, sens('visa.title')),
          'country': prop(v.country, sens('visa.country')),
          'visaType': prop(v.visaType, sens('visa.visaType')),
          'number': prop(v.number, sens('visa.number')),
          'issueDate': prop(v.issueDate, sens('visa.issueDate')),
          'expiryDate': prop(v.expiryDate, sens('visa.expiryDate')),
        },
        isDeleted: v.isDeleted,
        deletedAt: v.deletedAt,
        createdAt: timestamp,
        updatedAt: v.updatedAt,
      ));
    }

    final histories = profile.travel?.travelHistory ?? [];
    final historyChildren = <String>[];
    for (final h in histories) {
      historyChildren.add(h.id);
      objects.add(UnifiedObject(
        id: h.id,
        typeId: 'travel_history',
        name: h.destination,
        parentId: DefaultSectionIds.travelHistory,
        properties: {
          'destination': prop(h.destination, sens('travel.destination')),
          'travelType': prop(h.travelType, sens('travel.travelType')),
          'date': prop(h.date, sens('travel.date')),
          'departureCity': prop(h.departureCity, sens('travel.departureCity')),
          'departureTime': prop(h.departureTime, sens('travel.departureTime')),
          'arrivalTime': prop(h.arrivalTime, sens('travel.arrivalTime')),
          'flightNumber': prop(h.flightNumber, sens('travel.flightNumber')),
          'ticketPrice': prop(h.ticketPrice, sens('travel.ticketPrice')),
          'airline': prop(h.airline, sens('travel.airline')),
        },
        isDeleted: h.isDeleted,
        deletedAt: h.deletedAt,
        createdAt: timestamp,
        updatedAt: h.updatedAt,
      ));
    }

    objects.add(UnifiedObject(
      id: DefaultSectionIds.passport,
      typeId: 'collection',
      name: 'Passports',
      iconName: 'book',
      parentId: DefaultPageIds.travel,
      childrenIds: passportChildren,
      createdAt: timestamp,
      updatedAt: timestamp,
    ));
    objects.add(UnifiedObject(
      id: DefaultSectionIds.visa,
      typeId: 'collection',
      name: 'Visas',
      iconName: 'assignment_ind',
      parentId: DefaultPageIds.travel,
      childrenIds: visaChildren,
      createdAt: timestamp,
      updatedAt: timestamp,
    ));
    objects.add(UnifiedObject(
      id: DefaultSectionIds.travelHistory,
      typeId: 'collection',
      name: 'Travel History',
      iconName: 'history',
      parentId: DefaultPageIds.travel,
      childrenIds: historyChildren,
      createdAt: timestamp,
      updatedAt: timestamp,
    ));
    objects.add(UnifiedObject(
      id: DefaultPageIds.travel,
      typeId: 'page',
      name: 'Travel',
      iconName: 'flight',
      parentId: null,
      childrenIds: const [
        DefaultSectionIds.passport,
        DefaultSectionIds.visa,
        DefaultSectionIds.travelHistory,
      ],
      createdAt: timestamp,
      updatedAt: timestamp,
    ));

    // -------------------------------------------------------------------------
    // Financial page
    // -------------------------------------------------------------------------
    final bankAccounts = profile.financial?.bankAccounts ?? [];
    final bankAccountChildren = <String>[];
    for (final b in bankAccounts) {
      bankAccountChildren.add(b.id);
      objects.add(UnifiedObject(
        id: b.id,
        typeId: 'financial_bank_account',
        name: b.title ?? b.bankName ?? 'Bank Account',
        parentId: DefaultSectionIds.bankAccount,
        properties: {
          'title': prop(b.title, sens('bankAccount.title')),
          'bankName': prop(b.bankName, sens('bankAccount.bankName')),
          'accountNumber': prop(b.accountNumber, sens('bankAccount.accountNumber')),
          'currency': prop(b.currency, sens('bankAccount.currency')),
          'swiftBic': prop(b.swiftBic, sens('bankAccount.swiftBic')),
          'sortCode': prop(b.sortCode, sens('bankAccount.sortCode')),
        },
        isDeleted: b.isDeleted,
        deletedAt: b.deletedAt,
        createdAt: timestamp,
        updatedAt: b.updatedAt,
      ));
    }

    final cards = profile.financial?.cards ?? [];
    final cardChildren = <String>[];
    for (final c in cards) {
      cardChildren.add(c.id);
      objects.add(UnifiedObject(
        id: c.id,
        typeId: 'financial_card',
        name: c.title ?? c.cardType ?? 'Card',
        parentId: DefaultSectionIds.card,
        properties: {
          'title': prop(c.title, sens('card.title')),
          'cardNumber': prop(c.cardNumber, sens('card.cardNumber')),
          'cardType': prop(c.cardType, sens('card.cardType')),
          'expiryDate': prop(c.expiryDate, sens('card.expiryDate')),
          'holderName': prop(c.holderName, sens('card.holderName')),
          'cvv': prop(c.cvv, sens('card.cvv')),
        },
        isDeleted: c.isDeleted,
        deletedAt: c.deletedAt,
        createdAt: timestamp,
        updatedAt: c.updatedAt,
      ));
    }

    final taxIds = profile.financial?.taxIds ?? [];
    final taxIdChildren = <String>[];
    for (final t in taxIds) {
      taxIdChildren.add(t.id);
      objects.add(UnifiedObject(
        id: t.id,
        typeId: 'financial_tax_id',
        name: t.title ?? 'Tax ID',
        parentId: DefaultSectionIds.taxId,
        properties: {
          'title': prop(t.title, sens('taxId.title')),
          'taxIdNumber': prop(t.taxIdNumber, sens('taxId.taxIdNumber')),
          'taxIdType': prop(t.taxIdType, sens('taxId.taxIdType')),
          'issuingAuthority': prop(t.issuingAuthority, sens('taxId.issuingAuthority')),
          'country': prop(t.country, sens('taxId.country')),
        },
        isDeleted: t.isDeleted,
        deletedAt: t.deletedAt,
        createdAt: timestamp,
        updatedAt: t.updatedAt,
      ));
    }

    objects.add(UnifiedObject(
      id: DefaultSectionIds.bankAccount,
      typeId: 'collection',
      name: 'Bank Accounts',
      iconName: 'account_balance',
      parentId: DefaultPageIds.financial,
      childrenIds: bankAccountChildren,
      createdAt: timestamp,
      updatedAt: timestamp,
    ));
    objects.add(UnifiedObject(
      id: DefaultSectionIds.card,
      typeId: 'collection',
      name: 'Cards',
      iconName: 'credit_card',
      parentId: DefaultPageIds.financial,
      childrenIds: cardChildren,
      createdAt: timestamp,
      updatedAt: timestamp,
    ));
    objects.add(UnifiedObject(
      id: DefaultSectionIds.taxId,
      typeId: 'collection',
      name: 'Tax IDs',
      iconName: 'description',
      parentId: DefaultPageIds.financial,
      childrenIds: taxIdChildren,
      createdAt: timestamp,
      updatedAt: timestamp,
    ));
    objects.add(UnifiedObject(
      id: DefaultPageIds.financial,
      typeId: 'page',
      name: 'Financial',
      iconName: 'account_balance',
      parentId: null,
      childrenIds: const [
        DefaultSectionIds.bankAccount,
        DefaultSectionIds.card,
        DefaultSectionIds.taxId,
      ],
      createdAt: timestamp,
      updatedAt: timestamp,
    ));

    // -------------------------------------------------------------------------
    // Professional page
    // -------------------------------------------------------------------------
    final educationList = profile.professional?.education ?? [];
    final educationChildren = <String>[];
    for (final e in educationList) {
      educationChildren.add(e.id);
      objects.add(UnifiedObject(
        id: e.id,
        typeId: 'professional_education',
        name: e.institution ?? 'Education',
        parentId: DefaultSectionIds.education,
        properties: {
          'institution': prop(e.institution, sens('education.institution')),
          'degree': prop(e.degree, sens('education.degree')),
          'degreeCustom': prop(e.degreeCustom, sens('education.degreeCustom')),
          'field': prop(e.field, sens('education.field')),
          'startDate': prop(e.startDate, sens('education.startDate')),
          'endDate': prop(e.endDate, sens('education.endDate')),
        },
        isDeleted: e.isDeleted,
        deletedAt: e.deletedAt,
        createdAt: timestamp,
        updatedAt: e.updatedAt,
      ));
    }

    final employmentList = profile.professional?.employment ?? [];
    final employmentChildren = <String>[];
    for (final e in employmentList) {
      employmentChildren.add(e.id);
      objects.add(UnifiedObject(
        id: e.id,
        typeId: 'professional_employment',
        name: e.company ?? 'Employment',
        parentId: DefaultSectionIds.employment,
        properties: {
          'company': prop(e.company, sens('employment.company')),
          'position': prop(e.position, sens('employment.position')),
          'responsibilities': prop(e.responsibilities, sens('employment.responsibilities')),
          'startDate': prop(e.startDate, sens('employment.startDate')),
          'endDate': prop(e.endDate, sens('employment.endDate')),
        },
        isDeleted: e.isDeleted,
        deletedAt: e.deletedAt,
        createdAt: timestamp,
        updatedAt: e.updatedAt,
      ));
    }

    final skills = profile.professional?.skills ?? [];
    final skillChildren = <String>[];
    for (final s in skills) {
      skillChildren.add(s.id);
      objects.add(UnifiedObject(
        id: s.id,
        typeId: 'professional_skill',
        name: s.name,
        parentId: DefaultSectionIds.skill,
        properties: {
          'name': prop(s.name, sens('skill.name')),
          'level': prop(s.level, sens('skill.level')),
        },
        isDeleted: s.isDeleted,
        deletedAt: s.deletedAt,
        createdAt: timestamp,
        updatedAt: s.updatedAt,
      ));
    }

    final languages = profile.professional?.languages ?? [];
    final languageChildren = <String>[];
    for (final l in languages) {
      languageChildren.add(l.id);
      objects.add(UnifiedObject(
        id: l.id,
        typeId: 'professional_language',
        name: l.name,
        parentId: DefaultSectionIds.language,
        properties: {
          'name': prop(l.name, sens('language.name')),
          'proficiency': prop(l.proficiency, sens('language.proficiency')),
        },
        isDeleted: l.isDeleted,
        deletedAt: l.deletedAt,
        createdAt: timestamp,
        updatedAt: l.updatedAt,
      ));
    }

    final awards = profile.professional?.awards ?? [];
    final awardChildren = <String>[];
    for (final a in awards) {
      awardChildren.add(a.id);
      objects.add(UnifiedObject(
        id: a.id,
        typeId: 'professional_award',
        name: a.title ?? 'Award',
        parentId: DefaultSectionIds.award,
        properties: {
          'title': prop(a.title, sens('award.title')),
          'issuer': prop(a.issuer, sens('award.issuer')),
          'date': prop(a.date, sens('award.date')),
          'description': prop(a.description, sens('award.description')),
        },
        isDeleted: a.isDeleted,
        deletedAt: a.deletedAt,
        createdAt: timestamp,
        updatedAt: a.updatedAt,
      ));
    }

    objects.add(UnifiedObject(
      id: DefaultSectionIds.education,
      typeId: 'collection',
      name: 'Education',
      iconName: 'school',
      parentId: DefaultPageIds.professional,
      childrenIds: educationChildren,
      createdAt: timestamp,
      updatedAt: timestamp,
    ));
    objects.add(UnifiedObject(
      id: DefaultSectionIds.employment,
      typeId: 'collection',
      name: 'Employment',
      iconName: 'work',
      parentId: DefaultPageIds.professional,
      childrenIds: employmentChildren,
      createdAt: timestamp,
      updatedAt: timestamp,
    ));
    objects.add(UnifiedObject(
      id: DefaultSectionIds.skill,
      typeId: 'collection',
      name: 'Skills',
      iconName: 'star',
      parentId: DefaultPageIds.professional,
      childrenIds: skillChildren,
      createdAt: timestamp,
      updatedAt: timestamp,
    ));
    objects.add(UnifiedObject(
      id: DefaultSectionIds.language,
      typeId: 'collection',
      name: 'Languages',
      iconName: 'language',
      parentId: DefaultPageIds.professional,
      childrenIds: languageChildren,
      createdAt: timestamp,
      updatedAt: timestamp,
    ));
    objects.add(UnifiedObject(
      id: DefaultSectionIds.award,
      typeId: 'collection',
      name: 'Awards',
      iconName: 'emoji_events',
      parentId: DefaultPageIds.professional,
      childrenIds: awardChildren,
      createdAt: timestamp,
      updatedAt: timestamp,
    ));
    objects.add(UnifiedObject(
      id: DefaultPageIds.professional,
      typeId: 'page',
      name: 'Professional',
      iconName: 'work',
      parentId: null,
      childrenIds: const [
        DefaultSectionIds.education,
        DefaultSectionIds.employment,
        DefaultSectionIds.skill,
        DefaultSectionIds.language,
        DefaultSectionIds.award,
      ],
      createdAt: timestamp,
      updatedAt: timestamp,
    ));

    return UnifiedObjectData(
      objects: objects,
      customTypes: existingData.customTypes,
    );
  }

  /// Load profile data for an account
  /// Returns ProfileData with all fields decrypted, or null if not found
  Future<ProfileData?> loadProfile(String accountId) async {
    try {
      // Try to load from Rust vault
      final decrypted = await _rustVault.loadProfileDecrypted(accountId);
      if (decrypted == null) {
        return null;
      }

      final json = jsonDecode(decrypted) as Map<String, dynamic>;
      final profile = ProfileData.fromJson(json);
      // Apply migration if needed
      final migratedProfile = _migrateIfNeeded(profile, json);
      return migratedProfile;
    } on Exception catch (e, st) {
      DebugLogger.instance.logError('PROFILE', 'loadProfile failed: $e\n$st');
      return null;
    }
  }

  /// Save profile data for an account
  /// Encrypts and stores via RustVaultService
  Future<bool> saveProfile(String accountId, ProfileData profile) async {
    try {
      // Data protection: prevent accidental loss of unifiedObjects
      final existing = await loadProfile(accountId);
      if (existing?.unifiedObjects != null && profile.unifiedObjects == null) {
        profile = profile.copyWith(unifiedObjects: existing!.unifiedObjects);
      }

      final json = jsonEncode(profile.toJson());

      final result = await _rustVault.saveProfileEncrypted(accountId, json);

      if (result == null) {
        return false;
      }

      // Invalidate deleted items cache since profile data changed
      _invalidateDeletedItemsCache();

      return true;
    } on Exception catch (_) {
      // IOException or other Error subclasses could occur here
      return false;
    }
  }

  /// Get all soft-deleted items across all sections
  /// Results are cached to avoid rebuilding the list on every call
  /// Cache is invalidated on any profile mutation (restore, permanent delete, etc.)
  List<DeletedItemInfo> getDeletedItems(ProfileData profile) {
    if (_cachedDeletedItems != null) {
      return _cachedDeletedItems!;
    }

    final items = <DeletedItemInfo>[];

    if (profile.travel != null) {
      for (var i = 0; i < profile.travel!.passports.length; i++) {
        final p = profile.travel!.passports[i];
        if (p.isDeleted) {
          items.add(
            DeletedItemInfo(
              section: 'travel',
              itemType: 'passport',
              id: p.id,
              itemLabel: p.country ?? 'Passport',
              deletedAt: p.deletedAt ?? DateTime.now(),
            ),
          );
        }
      }
    }

    // Visa loop (separate from passport loop above)
    if (profile.travel != null) {
      for (var i = 0; i < profile.travel!.visas.length; i++) {
        final v = profile.travel!.visas[i];
        if (v.isDeleted) {
          items.add(
            DeletedItemInfo(
              section: 'travel',
              itemType: 'visa',
              id: v.id,
              itemLabel: v.country ?? 'Visa',
              deletedAt: v.deletedAt ?? DateTime.now(),
            ),
          );
        }
      }
      for (var i = 0; i < profile.travel!.travelHistory.length; i++) {
        final t = profile.travel!.travelHistory[i];
        if (t.isDeleted) {
          items.add(
            DeletedItemInfo(
              section: 'travel',
              itemType: 'travel_history',
              id: t.id,
              itemLabel: t.destination,
              deletedAt: t.deletedAt ?? DateTime.now(),
            ),
          );
        }
      }
    }

    // Financial section
    if (profile.financial != null) {
      for (var i = 0; i < profile.financial!.bankAccounts.length; i++) {
        final b = profile.financial!.bankAccounts[i];
        if (b.isDeleted) {
          items.add(
            DeletedItemInfo(
              section: 'financial',
              itemType: 'bank_account',
              id: b.id,
              itemLabel: b.bankName ?? 'Bank Account',
              deletedAt: b.deletedAt ?? DateTime.now(),
            ),
          );
        }
      }
      for (var i = 0; i < profile.financial!.cards.length; i++) {
        final c = profile.financial!.cards[i];
        if (c.isDeleted) {
          items.add(
            DeletedItemInfo(
              section: 'financial',
              itemType: 'card',
              id: c.id,
              itemLabel: c.cardType ?? 'Card',
              deletedAt: c.deletedAt ?? DateTime.now(),
            ),
          );
        }
      }
      for (var i = 0; i < profile.financial!.taxIds.length; i++) {
        final t = profile.financial!.taxIds[i];
        if (t.isDeleted) {
          items.add(
            DeletedItemInfo(
              section: 'financial',
              itemType: 'tax_id',
              id: t.id,
              itemLabel: t.taxIdType ?? 'Tax ID',
              deletedAt: t.deletedAt ?? DateTime.now(),
            ),
          );
        }
      }
    }

    // Professional section
    if (profile.professional != null) {
      for (var i = 0; i < profile.professional!.education.length; i++) {
        final e = profile.professional!.education[i];
        if (e.isDeleted) {
          items.add(
            DeletedItemInfo(
              section: 'professional',
              itemType: 'education',
              id: e.id,
              itemLabel: e.institution ?? 'Education',
              deletedAt: e.deletedAt ?? DateTime.now(),
            ),
          );
        }
      }
      for (var i = 0; i < profile.professional!.employment.length; i++) {
        final emp = profile.professional!.employment[i];
        if (emp.isDeleted) {
          items.add(
            DeletedItemInfo(
              section: 'professional',
              itemType: 'employment',
              id: emp.id,
              itemLabel: emp.company ?? 'Employment',
              deletedAt: emp.deletedAt ?? DateTime.now(),
            ),
          );
        }
      }
      for (var i = 0; i < profile.professional!.skills.length; i++) {
        final s = profile.professional!.skills[i];
        if (s.isDeleted) {
          items.add(
            DeletedItemInfo(
              section: 'professional',
              itemType: 'skill',
              id: s.id,
              itemLabel: s.toString(),
              deletedAt: s.deletedAt ?? DateTime.now(),
            ),
          );
        }
      }
      for (var i = 0; i < profile.professional!.languages.length; i++) {
        final l = profile.professional!.languages[i];
        if (l.isDeleted) {
          items.add(
            DeletedItemInfo(
              section: 'professional',
              itemType: 'language',
              id: l.id,
              itemLabel: l.toString(),
              deletedAt: l.deletedAt ?? DateTime.now(),
            ),
          );
        }
      }
    }

    // Profile/Identity section - contact entries
    if (profile.identity?.contact != null) {
      for (var i = 0; i < profile.identity!.contact!.entries.length; i++) {
        final e = profile.identity!.contact!.entries[i];
        if (e.isDeleted) {
          items.add(
            DeletedItemInfo(
              section: 'profile',
              itemType: 'contact',
              id: e.id,
              itemLabel: e.title.isNotEmpty
                  ? '${e.title} - ${e.value}'
                  : e.value,
              deletedAt: e.deletedAt ?? DateTime.now(),
            ),
          );
        }
      }
    }

    // Profile/Identity section - ID cards
    if (profile.identity?.idCards != null) {
      for (var i = 0; i < profile.identity!.idCards!.length; i++) {
        final c = profile.identity!.idCards![i];
        if (c.isDeleted) {
          items.add(
            DeletedItemInfo(
              section: 'profile',
              itemType: 'idCard',
              id: c.id,
              itemLabel: c.title ?? c.number ?? 'ID Card',
              deletedAt: c.deletedAt ?? DateTime.now(),
            ),
          );
        }
      }
    }

    // Profile/Identity section - addresses
    if (profile.identity?.addresses != null) {
      for (var i = 0; i < profile.identity!.addresses!.length; i++) {
        final a = profile.identity!.addresses![i];
        if (a.isDeleted) {
          items.add(
            DeletedItemInfo(
              section: 'profile',
              itemType: 'address',
              id: a.id,
              itemLabel: a.title ?? 'Address',
              deletedAt: a.deletedAt ?? DateTime.now(),
            ),
          );
        }
      }
    }

    // Sort by deletedAt descending (most recent first)
    items.sort((a, b) => b.deletedAt.compareTo(a.deletedAt));

    // Cache the result
    _cachedDeletedItems = items;
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
    _invalidateDeletedItemsCache();
    final updatedProfile = _calculateRestoreItem(
      profile,
      section,
      itemType,
      index,
    );
    await saveProfile(accountId, updatedProfile);
  }

  /// Pure function: calculates a new ProfileData with the restored item.
  /// Does not mutate the input profile.
  static ProfileData _calculateRestoreItem(
    ProfileData profile,
    String section,
    String itemType,
    int index,
  ) {
    switch (section) {
      case 'travel':
        if (profile.travel == null) return profile;
        if (itemType == 'passport' &&
            index < profile.travel!.passports.length) {
          final passports = List<PassportData>.from(profile.travel!.passports);
          passports[index] = passports[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
          return profile.copyWith(
            travel: profile.travel!.copyWith(passports: passports),
          );
        } else if (itemType == 'visa' && index < profile.travel!.visas.length) {
          final visas = List<VisaData>.from(profile.travel!.visas);
          visas[index] = visas[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
          return profile.copyWith(
            travel: profile.travel!.copyWith(visas: visas),
          );
        }
        return profile;
      case 'financial':
        if (profile.financial == null) return profile;
        if (itemType == 'bank_account' &&
            index < profile.financial!.bankAccounts.length) {
          final bankAccounts = List<BankAccountData>.from(
            profile.financial!.bankAccounts,
          );
          bankAccounts[index] = bankAccounts[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
          return profile.copyWith(
            financial: profile.financial!.copyWith(bankAccounts: bankAccounts),
          );
        } else if (itemType == 'card' &&
            index < profile.financial!.cards.length) {
          final cards = List<CardData>.from(profile.financial!.cards);
          cards[index] = cards[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
          return profile.copyWith(
            financial: profile.financial!.copyWith(cards: cards),
          );
        } else if (itemType == 'tax_id' &&
            index < profile.financial!.taxIds.length) {
          final taxIds = List<TaxIdData>.from(profile.financial!.taxIds);
          taxIds[index] = taxIds[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
          return profile.copyWith(
            financial: profile.financial!.copyWith(taxIds: taxIds),
          );
        }
        return profile;
      case 'professional':
        if (profile.professional == null) return profile;
        if (itemType == 'education' &&
            index < profile.professional!.education.length) {
          final education = List<EducationData>.from(
            profile.professional!.education,
          );
          education[index] = education[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
          return profile.copyWith(
            professional: profile.professional!.copyWith(education: education),
          );
        } else if (itemType == 'employment' &&
            index < profile.professional!.employment.length) {
          final employment = List<EmploymentData>.from(
            profile.professional!.employment,
          );
          employment[index] = employment[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
          return profile.copyWith(
            professional: profile.professional!.copyWith(
              employment: employment,
            ),
          );
        } else if (itemType == 'skill' &&
            index < profile.professional!.skills.length) {
          final skills = List<SkillData>.from(profile.professional!.skills);
          skills[index] = skills[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
          return profile.copyWith(
            professional: profile.professional!.copyWith(skills: skills),
          );
        } else if (itemType == 'language' &&
            index < profile.professional!.languages.length) {
          final languages = List<LanguageData>.from(
            profile.professional!.languages,
          );
          languages[index] = languages[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
          return profile.copyWith(
            professional: profile.professional!.copyWith(languages: languages),
          );
        }
        return profile;
      case 'profile':
        if (profile.identity == null) return profile;
        if (itemType == 'contact' &&
            index < (profile.identity!.contact?.entries.length ?? 0)) {
          final entries = List<ContactEntry>.from(
            profile.identity!.contact!.entries,
          );
          entries[index] = entries[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
          return profile.copyWith(
            identity: profile.identity!.copyWith(
              contact: ContactData(entries: entries),
            ),
          );
        } else if (itemType == 'idCard' &&
            index < (profile.identity!.idCards?.length ?? 0)) {
          final idCards = List<IdCardData>.from(profile.identity!.idCards!);
          idCards[index] = idCards[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
          return profile.copyWith(
            identity: profile.identity!.copyWith(idCards: idCards),
          );
        } else if (itemType == 'address' &&
            index < (profile.identity!.addresses?.length ?? 0)) {
          final addresses = List<AddressData>.from(
            profile.identity!.addresses!,
          );
          addresses[index] = addresses[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
          return profile.copyWith(
            identity: profile.identity!.copyWith(addresses: addresses),
          );
        }
        return profile;
    }
    return profile;
  }

  /// Calculate the result of permanently deleting an item (pure function).
  /// Returns a new ProfileData with the item removed, or null if the item
  /// could not be deleted (e.g., invalid index or null section).
  static ProfileData? _calculatePermanentDeleteItem(
    ProfileData profile,
    String section,
    String itemType,
    int index,
  ) {
    switch (section) {
      case 'travel':
        if (profile.travel == null) return null;
        if (itemType == 'passport' &&
            index < profile.travel!.passports.length) {
          final updated = List<PassportData>.from(profile.travel!.passports)
            ..removeAt(index);
          return profile.copyWith(
            travel: profile.travel!.copyWith(passports: updated),
          );
        } else if (itemType == 'visa' && index < profile.travel!.visas.length) {
          final updated = List<VisaData>.from(profile.travel!.visas)
            ..removeAt(index);
          return profile.copyWith(
            travel: profile.travel!.copyWith(visas: updated),
          );
        } else if (itemType == 'travel_history' &&
            index < profile.travel!.travelHistory.length) {
          final updated = List<TravelHistoryData>.from(
            profile.travel!.travelHistory,
          )..removeAt(index);
          return profile.copyWith(
            travel: profile.travel!.copyWith(travelHistory: updated),
          );
        }
        return null;
      case 'financial':
        if (profile.financial == null) return null;
        if (itemType == 'bank_account' &&
            index < profile.financial!.bankAccounts.length) {
          final updated = List<BankAccountData>.from(
            profile.financial!.bankAccounts,
          )..removeAt(index);
          return profile.copyWith(
            financial: profile.financial!.copyWith(bankAccounts: updated),
          );
        } else if (itemType == 'card' &&
            index < profile.financial!.cards.length) {
          final updated = List<CardData>.from(profile.financial!.cards)
            ..removeAt(index);
          return profile.copyWith(
            financial: profile.financial!.copyWith(cards: updated),
          );
        } else if (itemType == 'tax_id' &&
            index < profile.financial!.taxIds.length) {
          final updated = List<TaxIdData>.from(profile.financial!.taxIds)
            ..removeAt(index);
          return profile.copyWith(
            financial: profile.financial!.copyWith(taxIds: updated),
          );
        }
        return null;
      case 'professional':
        if (profile.professional == null) return null;
        if (itemType == 'education' &&
            index < profile.professional!.education.length) {
          final updated = List<EducationData>.from(
            profile.professional!.education,
          )..removeAt(index);
          return profile.copyWith(
            professional: profile.professional!.copyWith(education: updated),
          );
        } else if (itemType == 'employment' &&
            index < profile.professional!.employment.length) {
          final updated = List<EmploymentData>.from(
            profile.professional!.employment,
          )..removeAt(index);
          return profile.copyWith(
            professional: profile.professional!.copyWith(employment: updated),
          );
        } else if (itemType == 'skill' &&
            index < profile.professional!.skills.length) {
          final updated = List<SkillData>.from(profile.professional!.skills)
            ..removeAt(index);
          return profile.copyWith(
            professional: profile.professional!.copyWith(skills: updated),
          );
        } else if (itemType == 'language' &&
            index < profile.professional!.languages.length) {
          final updated = List<LanguageData>.from(
            profile.professional!.languages,
          )..removeAt(index);
          return profile.copyWith(
            professional: profile.professional!.copyWith(languages: updated),
          );
        }
        return null;
      case 'profile':
        if (profile.identity == null) return null;
        if (itemType == 'contact' &&
            index < (profile.identity!.contact?.entries.length ?? 0)) {
          final entries = List<ContactEntry>.from(
            profile.identity!.contact!.entries,
          )..removeAt(index);
          return profile.copyWith(
            identity: profile.identity!.copyWith(
              contact: ContactData(entries: entries),
            ),
          );
        } else if (itemType == 'idCard' &&
            index < (profile.identity!.idCards?.length ?? 0)) {
          final idCards = List<IdCardData>.from(profile.identity!.idCards!)
            ..removeAt(index);
          return profile.copyWith(
            identity: profile.identity!.copyWith(idCards: idCards),
          );
        } else if (itemType == 'address' &&
            index < (profile.identity!.addresses?.length ?? 0)) {
          final addresses = List<AddressData>.from(profile.identity!.addresses!)
            ..removeAt(index);
          return profile.copyWith(
            identity: profile.identity!.copyWith(addresses: addresses),
          );
        }
        return null;
      default:
        return null;
    }
  }

  /// Permanently delete a specific item (removes from list completely)
  Future<void> permanentDeleteItem(
    ProfileData profile,
    String accountId,
    String section,
    String itemType,
    int index,
  ) async {
    _invalidateDeletedItemsCache();
    final updatedProfile = _calculatePermanentDeleteItem(
      profile,
      section,
      itemType,
      index,
    );
    if (updatedProfile == null) return;
    await saveProfile(accountId, updatedProfile);
  }

  /// Permanently delete items older than 30 days
  Future<void> purgeOldDeletedItems(
    ProfileData profile,
    String accountId,
  ) async {
    final cutoff = DateTime.now().subtract(const Duration(days: 30));

    // Travel section
    if (profile.travel != null) {
      profile.travel!.passports.removeWhere(
        (p) =>
            p.isDeleted && p.deletedAt != null && p.deletedAt!.isBefore(cutoff),
      );
      profile.travel!.visas.removeWhere(
        (v) =>
            v.isDeleted && v.deletedAt != null && v.deletedAt!.isBefore(cutoff),
      );
    }

    // Financial section
    if (profile.financial != null) {
      profile.financial!.bankAccounts.removeWhere(
        (b) =>
            b.isDeleted && b.deletedAt != null && b.deletedAt!.isBefore(cutoff),
      );
      profile.financial!.cards.removeWhere(
        (c) =>
            c.isDeleted && c.deletedAt != null && c.deletedAt!.isBefore(cutoff),
      );
      profile.financial!.taxIds.removeWhere(
        (t) =>
            t.isDeleted && t.deletedAt != null && t.deletedAt!.isBefore(cutoff),
      );
    }

    // Professional section
    if (profile.professional != null) {
      profile.professional!.education.removeWhere(
        (e) =>
            e.isDeleted && e.deletedAt != null && e.deletedAt!.isBefore(cutoff),
      );
      profile.professional!.employment.removeWhere(
        (emp) =>
            emp.isDeleted &&
            emp.deletedAt != null &&
            emp.deletedAt!.isBefore(cutoff),
      );
    }

    await saveProfile(accountId, profile);
  }

  /// Check and purge old deleted items (called on app startup)
  ///
  /// If [existingProfile] is provided (already loaded), uses it instead of
  /// loading again to avoid redundant decryption.
  Future<void> purgeOldDeletedItemsIfNeeded(
    String accountId, {
    ProfileData? existingProfile,
  }) async {
    final profile = existingProfile ?? await loadProfile(accountId);
    if (profile == null) return;

    final cutoff = DateTime.now().subtract(const Duration(days: 30));
    bool hasOldItems = false;

    // Check if any deleted items are older than 30 days
    if (profile.travel != null) {
      hasOldItems =
          hasOldItems ||
          profile.travel!.passports.any(
            (p) =>
                p.isDeleted &&
                p.deletedAt != null &&
                p.deletedAt!.isBefore(cutoff),
          ) ||
          profile.travel!.visas.any(
            (v) =>
                v.isDeleted &&
                v.deletedAt != null &&
                v.deletedAt!.isBefore(cutoff),
          );
    }
    if (profile.financial != null) {
      hasOldItems =
          hasOldItems ||
          profile.financial!.bankAccounts.any(
            (b) =>
                b.isDeleted &&
                b.deletedAt != null &&
                b.deletedAt!.isBefore(cutoff),
          ) ||
          profile.financial!.cards.any(
            (c) =>
                c.isDeleted &&
                c.deletedAt != null &&
                c.deletedAt!.isBefore(cutoff),
          ) ||
          profile.financial!.taxIds.any(
            (t) =>
                t.isDeleted &&
                t.deletedAt != null &&
                t.deletedAt!.isBefore(cutoff),
          );
    }
    if (profile.professional != null) {
      hasOldItems =
          hasOldItems ||
          profile.professional!.education.any(
            (e) =>
                e.isDeleted &&
                e.deletedAt != null &&
                e.deletedAt!.isBefore(cutoff),
          ) ||
          profile.professional!.employment.any(
            (emp) =>
                emp.isDeleted &&
                emp.deletedAt != null &&
                emp.deletedAt!.isBefore(cutoff),
          );
    }

    if (hasOldItems) {
      await purgeOldDeletedItems(profile, accountId);
    }
  }

  /// Manually empty all trash (permanent delete all soft-deleted items)
  Future<void> emptyAllTrash(ProfileData profile, String accountId) async {
    final newProfile = _calculateEmptyTrash(profile);
    await saveProfile(accountId, newProfile);
  }

  /// Pure function: returns a new ProfileData with all soft-deleted items removed
  ProfileData _calculateEmptyTrash(ProfileData current) {
    // Travel section
    final newTravel = current.travel?.copyWith(
      passports: current.travel!.passports.where((p) => !p.isDeleted).toList(),
      visas: current.travel!.visas.where((v) => !v.isDeleted).toList(),
      travelHistory: current.travel!.travelHistory.where((t) => !t.isDeleted).toList(),
    );

    // Financial section
    final newFinancial = current.financial?.copyWith(
      bankAccounts: current.financial!.bankAccounts.where((b) => !b.isDeleted).toList(),
      cards: current.financial!.cards.where((c) => !c.isDeleted).toList(),
      taxIds: current.financial!.taxIds.where((t) => !t.isDeleted).toList(),
    );

    // Professional section
    final newProfessional = current.professional?.copyWith(
      education: current.professional!.education.where((e) => !e.isDeleted).toList(),
      employment: current.professional!.employment.where((emp) => !emp.isDeleted).toList(),
      skills: current.professional!.skills.where((s) => !s.isDeleted).toList(),
      languages: current.professional!.languages.where((l) => !l.isDeleted).toList(),
    );

    // Identity section
    final newIdentity = current.identity?.copyWith(
      idCards: current.identity!.idCards?.where((c) => !c.isDeleted).toList(),
      addresses: current.identity!.addresses?.where((a) => !a.isDeleted).toList(),
      contact: current.identity!.contact?.copyWith(
        entries: current.identity!.contact!.entries.where((e) => !e.isDeleted).toList(),
      ),
    );

    return current.copyWith(
      travel: newTravel,
      financial: newFinancial,
      professional: newProfessional,
      identity: newIdentity,
    );
  }
}
