import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';
import 'package:path_provider/path_provider.dart';
import 'package:uuid/uuid.dart';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';

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
  String id;
  String label; // e.g., "Personal", "Work", "Emergency"
  String type; // "email", "phone", "mobile"
  String value;
  int updatedAt;
  bool isDeleted;
  DateTime? deletedAt;

  ContactEntry({
    required this.id,
    required this.label,
    required this.type,
    required this.value,
    int? updatedAt,
    this.isDeleted = false,
    this.deletedAt,
  }) : updatedAt = updatedAt ?? currentTimestamp();

  factory ContactEntry.fromJson(Map<String, dynamic> json) {
    final id = json['id'] as String?;
    return ContactEntry(
      id: id ?? generateEntryId(),
      label: json['label'] ?? '',
      type: json['type'] ?? 'email',
      value: json['value'] ?? '',
      updatedAt: json['updated_at'] ?? currentTimestamp(),
      isDeleted: json['is_deleted'] ?? false,
      deletedAt: json['deleted_at'] != null
          ? DateTime.tryParse(json['deleted_at'])
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'id': id,
    'label': label,
    'type': type,
    'value': value,
    'updated_at': updatedAt,
    'is_deleted': isDeleted,
    'deleted_at': deletedAt?.toIso8601String(),
  };

  static const _sentinel = _DeletedAtSentinel();

  ContactEntry copyWith({
    String? id,
    String? label,
    String? type,
    String? value,
    int? updatedAt,
    bool? isDeleted,
    Object? deletedAt = _sentinel,
  }) {
    return ContactEntry(
      id: id ?? this.id,
      label: label ?? this.label,
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
  String id;
  String? label;
  String? street;
  String? city;
  String? state;
  String? postalCode;
  String? country;
  int updatedAt;
  bool isDeleted;
  DateTime? deletedAt;

  AddressData({
    required this.id,
    this.label,
    this.street,
    this.city,
    this.state,
    this.postalCode,
    this.country,
    int? updatedAt,
    this.isDeleted = false,
    this.deletedAt,
  }) : updatedAt = updatedAt ?? currentTimestamp();

  factory AddressData.fromJson(Map<String, dynamic> json) {
    final id = json['id'] as String?;
    return AddressData(
      id: id ?? generateEntryId(),
      label: json['label'],
      street: json['street'],
      city: json['city'],
      state: json['state'],
      postalCode: json['postal_code'],
      country: json['country'],
      updatedAt: json['updated_at'] ?? currentTimestamp(),
      isDeleted: json['is_deleted'] ?? false,
      deletedAt: json['deleted_at'] != null
          ? DateTime.tryParse(json['deleted_at'])
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'id': id,
    'label': label,
    'street': street,
    'city': city,
    'state': state,
    'postal_code': postalCode,
    'country': country,
    'updated_at': updatedAt,
    'is_deleted': isDeleted,
    'deleted_at': deletedAt?.toIso8601String(),
  };

  static const _sentinel = _DeletedAtSentinel();

  AddressData copyWith({
    String? id,
    String? label,
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
      label: label ?? this.label,
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

class IdCardData {
  String id;
  String? label;
  String? number;
  String? issueDate;
  String? expiryDate;
  String? holderName;
  String? country;
  int updatedAt;
  bool isDeleted;
  DateTime? deletedAt;

  IdCardData({
    required this.id,
    this.label,
    this.number,
    this.issueDate,
    this.expiryDate,
    this.holderName,
    this.country,
    int? updatedAt,
    this.isDeleted = false,
    this.deletedAt,
  }) : updatedAt = updatedAt ?? currentTimestamp();

  factory IdCardData.fromJson(Map<String, dynamic> json) {
    final id = json['id'] as String?;
    return IdCardData(
      id: id ?? generateEntryId(), // Generate ID if missing (for legacy data)
      label: json['label'],
      number: json['number'],
      issueDate: json['issue_date'],
      expiryDate: json['expiry_date'],
      holderName: json['holder_name'],
      country: json['country'],
      updatedAt: json['updated_at'] ?? currentTimestamp(),
      isDeleted: json['is_deleted'] ?? false,
      deletedAt: json['deleted_at'] != null
          ? DateTime.tryParse(json['deleted_at'])
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'id': id,
    'label': label,
    'number': number,
    'issue_date': issueDate,
    'expiry_date': expiryDate,
    'holder_name': holderName,
    'country': country,
    'updated_at': updatedAt,
    'is_deleted': isDeleted,
    'deleted_at': deletedAt?.toIso8601String(),
  };

  static const _sentinel = _DeletedAtSentinel();

  IdCardData copyWith({
    String? id,
    String? label,
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
      label: label ?? this.label,
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

class TravelHistoryData {
  String id;
  String destination;
  String? date;
  String? departureCity;
  String? departureTime;
  String? arrivalTime;
  String? flightNumber;
  String? ticketPrice;
  String? airline;
  int updatedAt;
  bool isDeleted;
  DateTime? deletedAt;

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
    int? updatedAt,
    this.isDeleted = false,
    this.deletedAt,
  }) : updatedAt = updatedAt ?? currentTimestamp();

  factory TravelHistoryData.fromJson(Map<String, dynamic> json) {
    final id = json['id'] as String?;
    return TravelHistoryData(
      id: id ?? generateEntryId(),
      destination: json['destination'] ?? '',
      date: json['date'],
      departureCity: json['departure_city'],
      departureTime: json['departure_time'],
      arrivalTime: json['arrival_time'],
      flightNumber: json['flight_number'],
      ticketPrice: json['ticket_price'],
      airline: json['airline'],
      updatedAt: json['updated_at'] ?? currentTimestamp(),
      isDeleted: json['is_deleted'] ?? false,
      deletedAt: json['deleted_at'] != null
          ? DateTime.tryParse(json['deleted_at'])
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'id': id,
    'destination': destination,
    'date': date,
    'departure_city': departureCity,
    'departure_time': departureTime,
    'arrival_time': arrivalTime,
    'flight_number': flightNumber,
    'ticket_price': ticketPrice,
    'airline': airline,
    'updated_at': updatedAt,
    'is_deleted': isDeleted,
    'deleted_at': deletedAt?.toIso8601String(),
  };

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

  List<VisaData> get activeVisas => visas.where((v) => !v.isDeleted).toList();

  List<TravelHistoryData> get activeTravelHistory =>
      travelHistory.where((t) => !t.isDeleted).toList();

  /// Get soft-deleted items only
  List<PassportData> get deletedPassports =>
      passports.where((p) => p.isDeleted).toList();

  List<VisaData> get deletedVisas => visas.where((v) => v.isDeleted).toList();

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
  String id;
  String? number;
  String? country;
  String? issueDate;
  String? expiryDate;
  String? holderName;
  int updatedAt;
  bool isDeleted;
  DateTime? deletedAt;

  PassportData({
    required this.id,
    this.number,
    this.country,
    this.issueDate,
    this.expiryDate,
    this.holderName,
    int? updatedAt,
    this.isDeleted = false,
    this.deletedAt,
  }) : updatedAt = updatedAt ?? currentTimestamp();

  factory PassportData.fromJson(Map<String, dynamic> json) {
    final id = json['id'] as String?;
    return PassportData(
      id: id ?? generateEntryId(),
      number: json['number'],
      country: json['country'],
      issueDate: json['issue_date'],
      expiryDate: json['expiry_date'],
      holderName: json['holder_name'],
      updatedAt: json['updated_at'] ?? currentTimestamp(),
      isDeleted: json['is_deleted'] ?? false,
      deletedAt: json['deleted_at'] != null
          ? DateTime.tryParse(json['deleted_at'])
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'id': id,
    'number': number,
    'country': country,
    'issue_date': issueDate,
    'expiry_date': expiryDate,
    'holder_name': holderName,
    'updated_at': updatedAt,
    'is_deleted': isDeleted,
    'deleted_at': deletedAt?.toIso8601String(),
  };

  static const _sentinel = _DeletedAtSentinel();

  PassportData copyWith({
    String? id,
    String? number,
    String? country,
    String? issueDate,
    String? expiryDate,
    String? holderName,
    int? updatedAt,
    bool? isDeleted,
    Object? deletedAt = _sentinel,
  }) {
    return PassportData(
      id: id ?? this.id,
      number: number ?? this.number,
      country: country ?? this.country,
      issueDate: issueDate ?? this.issueDate,
      expiryDate: expiryDate ?? this.expiryDate,
      holderName: holderName ?? this.holderName,
      updatedAt: updatedAt ?? this.updatedAt,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: identical(deletedAt, _sentinel)
          ? this.deletedAt
          : deletedAt as DateTime?,
    );
  }
}

class VisaData {
  String id;
  String? country;
  String? visaType;
  String? number;
  String? issueDate;
  String? expiryDate;
  int updatedAt;
  bool isDeleted;
  DateTime? deletedAt;

  VisaData({
    required this.id,
    this.country,
    this.visaType,
    this.number,
    this.issueDate,
    this.expiryDate,
    int? updatedAt,
    this.isDeleted = false,
    this.deletedAt,
  }) : updatedAt = updatedAt ?? currentTimestamp();

  factory VisaData.fromJson(Map<String, dynamic> json) {
    final id = json['id'] as String?;
    return VisaData(
      id: id ?? generateEntryId(),
      country: json['country'],
      visaType: json['visa_type'],
      number: json['number'],
      issueDate: json['issue_date'],
      expiryDate: json['expiry_date'],
      updatedAt: json['updated_at'] ?? currentTimestamp(),
      isDeleted: json['is_deleted'] ?? false,
      deletedAt: json['deleted_at'] != null
          ? DateTime.tryParse(json['deleted_at'])
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'id': id,
    'country': country,
    'visa_type': visaType,
    'number': number,
    'issue_date': issueDate,
    'expiry_date': expiryDate,
    'updated_at': updatedAt,
    'is_deleted': isDeleted,
    'deleted_at': deletedAt?.toIso8601String(),
  };

  static const _sentinel = _DeletedAtSentinel();

  VisaData copyWith({
    String? id,
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

class FinancialData {
  List<BankAccountData> bankAccounts;
  List<CardData> cards;
  List<TaxIdData> taxIds;

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
  String id;
  String? bankName;
  String? accountNumber;
  String? currency;
  String? swiftBic;
  int updatedAt;
  bool isDeleted;
  DateTime? deletedAt;

  BankAccountData({
    required this.id,
    this.bankName,
    this.accountNumber,
    this.currency,
    this.swiftBic,
    int? updatedAt,
    this.isDeleted = false,
    this.deletedAt,
  }) : updatedAt = updatedAt ?? currentTimestamp();

  factory BankAccountData.fromJson(Map<String, dynamic> json) {
    final id = json['id'] as String?;
    return BankAccountData(
      id: id ?? generateEntryId(),
      bankName: json['bank_name'],
      accountNumber: json['account_number'],
      currency: json['currency'],
      swiftBic: json['swift_bic'],
      updatedAt: json['updated_at'] ?? currentTimestamp(),
      isDeleted: json['is_deleted'] ?? false,
      deletedAt: json['deleted_at'] != null
          ? DateTime.tryParse(json['deleted_at'])
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'id': id,
    'bank_name': bankName,
    'account_number': accountNumber,
    'currency': currency,
    'swift_bic': swiftBic,
    'updated_at': updatedAt,
    'is_deleted': isDeleted,
    'deleted_at': deletedAt?.toIso8601String(),
  };

  static const _sentinel = _DeletedAtSentinel();

  BankAccountData copyWith({
    String? id,
    String? bankName,
    String? accountNumber,
    String? currency,
    String? swiftBic,
    int? updatedAt,
    bool? isDeleted,
    Object? deletedAt = _sentinel,
  }) {
    return BankAccountData(
      id: id ?? this.id,
      bankName: bankName ?? this.bankName,
      accountNumber: accountNumber ?? this.accountNumber,
      currency: currency ?? this.currency,
      swiftBic: swiftBic ?? this.swiftBic,
      updatedAt: updatedAt ?? this.updatedAt,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: identical(deletedAt, _sentinel)
          ? this.deletedAt
          : deletedAt as DateTime?,
    );
  }
}

class CardData {
  String id;
  String? cardNumber;
  String? cardType;
  String? expiryDate;
  String? holderName;
  int updatedAt;
  bool isDeleted;
  DateTime? deletedAt;

  CardData({
    required this.id,
    this.cardNumber,
    this.cardType,
    this.expiryDate,
    this.holderName,
    int? updatedAt,
    this.isDeleted = false,
    this.deletedAt,
  }) : updatedAt = updatedAt ?? currentTimestamp();

  factory CardData.fromJson(Map<String, dynamic> json) {
    final id = json['id'] as String?;
    return CardData(
      id: id ?? generateEntryId(),
      cardNumber: json['card_number'],
      cardType: json['card_type'],
      expiryDate: json['expiry_date'],
      holderName: json['holder_name'],
      updatedAt: json['updated_at'] ?? currentTimestamp(),
      isDeleted: json['is_deleted'] ?? false,
      deletedAt: json['deleted_at'] != null
          ? DateTime.tryParse(json['deleted_at'])
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'id': id,
    'card_number': cardNumber,
    'card_type': cardType,
    'expiry_date': expiryDate,
    'holder_name': holderName,
    'updated_at': updatedAt,
    'is_deleted': isDeleted,
    'deleted_at': deletedAt?.toIso8601String(),
  };

  static const _sentinel = _DeletedAtSentinel();

  CardData copyWith({
    String? id,
    String? cardNumber,
    String? cardType,
    String? expiryDate,
    String? holderName,
    int? updatedAt,
    bool? isDeleted,
    Object? deletedAt = _sentinel,
  }) {
    return CardData(
      id: id ?? this.id,
      cardNumber: cardNumber ?? this.cardNumber,
      cardType: cardType ?? this.cardType,
      expiryDate: expiryDate ?? this.expiryDate,
      holderName: holderName ?? this.holderName,
      updatedAt: updatedAt ?? this.updatedAt,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: identical(deletedAt, _sentinel)
          ? this.deletedAt
          : deletedAt as DateTime?,
    );
  }
}

class TaxIdData {
  String id;
  String? taxIdNumber;
  String? taxIdType;
  String? issuingAuthority;
  String? country;
  int updatedAt;
  bool isDeleted;
  DateTime? deletedAt;

  TaxIdData({
    required this.id,
    this.taxIdNumber,
    this.taxIdType,
    this.issuingAuthority,
    this.country,
    int? updatedAt,
    this.isDeleted = false,
    this.deletedAt,
  }) : updatedAt = updatedAt ?? currentTimestamp();

  factory TaxIdData.fromJson(Map<String, dynamic> json) {
    final id = json['id'] as String?;
    return TaxIdData(
      id: id ?? generateEntryId(),
      taxIdNumber: json['tax_id_number'],
      taxIdType: json['tax_id_type'],
      issuingAuthority: json['issuing_authority'],
      country: json['country'],
      updatedAt: json['updated_at'] ?? currentTimestamp(),
      isDeleted: json['is_deleted'] ?? false,
      deletedAt: json['deleted_at'] != null
          ? DateTime.tryParse(json['deleted_at'])
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'id': id,
    'tax_id_number': taxIdNumber,
    'tax_id_type': taxIdType,
    'issuing_authority': issuingAuthority,
    'country': country,
    'updated_at': updatedAt,
    'is_deleted': isDeleted,
    'deleted_at': deletedAt?.toIso8601String(),
  };

  static const _sentinel = _DeletedAtSentinel();

  TaxIdData copyWith({
    String? id,
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

class SkillData {
  String id;
  String name;
  String? level;
  int updatedAt;
  bool isDeleted;
  DateTime? deletedAt;

  SkillData({
    required this.id,
    required this.name,
    this.level,
    int? updatedAt,
    this.isDeleted = false,
    this.deletedAt,
  }) : updatedAt = updatedAt ?? currentTimestamp();

  factory SkillData.fromJson(Map<String, dynamic> json) {
    final id = json['id'] as String?;
    return SkillData(
      id: id ?? generateEntryId(),
      name: json['name'] ?? '',
      level: json['level'],
      updatedAt: json['updated_at'] ?? currentTimestamp(),
      isDeleted: json['is_deleted'] ?? false,
      deletedAt: json['deleted_at'] != null
          ? DateTime.tryParse(json['deleted_at'])
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'id': id,
    'name': name,
    'level': level,
    'updated_at': updatedAt,
    'is_deleted': isDeleted,
    'deleted_at': deletedAt?.toIso8601String(),
  };

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

class LanguageData {
  String id;
  String name;
  String? proficiency;
  int updatedAt;
  bool isDeleted;
  DateTime? deletedAt;

  LanguageData({
    required this.id,
    required this.name,
    this.proficiency,
    int? updatedAt,
    this.isDeleted = false,
    this.deletedAt,
  }) : updatedAt = updatedAt ?? currentTimestamp();

  factory LanguageData.fromJson(Map<String, dynamic> json) {
    final id = json['id'] as String?;
    return LanguageData(
      id: id ?? generateEntryId(),
      name: json['name'] ?? '',
      proficiency: json['proficiency'],
      updatedAt: json['updated_at'] ?? currentTimestamp(),
      isDeleted: json['is_deleted'] ?? false,
      deletedAt: json['deleted_at'] != null
          ? DateTime.tryParse(json['deleted_at'])
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'id': id,
    'name': name,
    'proficiency': proficiency,
    'updated_at': updatedAt,
    'is_deleted': isDeleted,
    'deleted_at': deletedAt?.toIso8601String(),
  };

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

class AwardData {
  String id;
  String? title;
  String? issuer;
  String? date;
  String? description;
  int updatedAt;
  bool isDeleted;
  DateTime? deletedAt;

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

  factory AwardData.fromJson(Map<String, dynamic> json) {
    final id = json['id'] as String?;
    return AwardData(
      id: id ?? generateEntryId(),
      title: json['title'],
      issuer: json['issuer'],
      date: json['date'],
      description: json['description'],
      updatedAt: json['updated_at'] ?? currentTimestamp(),
      isDeleted: json['is_deleted'] ?? false,
      deletedAt: json['deleted_at'] != null
          ? DateTime.tryParse(json['deleted_at'])
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'id': id,
    'title': title,
    'issuer': issuer,
    'date': date,
    'description': description,
    'updated_at': updatedAt,
    'is_deleted': isDeleted,
    'deleted_at': deletedAt?.toIso8601String(),
  };

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

class ProfessionalData {
  List<EducationData> education;
  List<EmploymentData> employment;
  List<SkillData> skills;
  List<LanguageData> languages;
  List<AwardData> awards;

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
      awards:
          (json['awards'] as List<dynamic>?)
              ?.map((a) => AwardData.fromJson(a))
              .toList() ??
          [],
    );
  }

  Map<String, dynamic> toJson() => {
    'education': education.map((e) => e.toJson()).toList(),
    'employment': employment.map((e) => e.toJson()).toList(),
    'skills': skills.map((s) => s.toJson()).toList(),
    'languages': languages.map((l) => l.toJson()).toList(),
    'awards': awards.map((a) => a.toJson()).toList(),
  };

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

class EducationData {
  String id;
  String? institution;
  String? degree;
  String? degreeCustom;
  String? field;
  String? startDate;
  String? endDate;
  int updatedAt;
  bool isDeleted;
  DateTime? deletedAt;

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

  factory EducationData.fromJson(Map<String, dynamic> json) {
    final id = json['id'] as String?;
    return EducationData(
      id: id ?? generateEntryId(),
      institution: json['institution'],
      degree: json['degree'],
      degreeCustom: json['degree_custom'],
      field: json['field'],
      startDate: json['start_date'],
      endDate: json['end_date'],
      updatedAt: json['updated_at'] ?? currentTimestamp(),
      isDeleted: json['is_deleted'] ?? false,
      deletedAt: json['deleted_at'] != null
          ? DateTime.tryParse(json['deleted_at'])
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'id': id,
    'institution': institution,
    'degree': degree,
    'degree_custom': degreeCustom,
    'field': field,
    'start_date': startDate,
    'end_date': endDate,
    'updated_at': updatedAt,
    'is_deleted': isDeleted,
    'deleted_at': deletedAt?.toIso8601String(),
  };

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

class EmploymentData {
  String id;
  String? company;
  String? position;
  String? responsibilities;
  String? startDate;
  String? endDate;
  int updatedAt;
  bool isDeleted;
  DateTime? deletedAt;

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

  factory EmploymentData.fromJson(Map<String, dynamic> json) {
    final id = json['id'] as String?;
    return EmploymentData(
      id: id ?? generateEntryId(),
      company: json['company'],
      position: json['position'],
      responsibilities: json['responsibilities'],
      startDate: json['start_date'],
      endDate: json['end_date'],
      updatedAt: json['updated_at'] ?? currentTimestamp(),
      isDeleted: json['is_deleted'] ?? false,
      deletedAt: json['deleted_at'] != null
          ? DateTime.tryParse(json['deleted_at'])
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'id': id,
    'company': company,
    'position': position,
    'responsibilities': responsibilities,
    'start_date': startDate,
    'end_date': endDate,
    'updated_at': updatedAt,
    'is_deleted': isDeleted,
    'deleted_at': deletedAt?.toIso8601String(),
  };

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
  final String section; // 'travel', 'financial', 'professional'
  final String
  itemType; // 'passport', 'visa', 'bank_account', 'card', 'education', 'employment'
  final String id; // unique ID of the item
  final String itemLabel; // display name for UI
  final DateTime deletedAt;

  const DeletedItemInfo({
    required this.section,
    required this.itemType,
    required this.id,
    required this.itemLabel,
    required this.deletedAt,
  });
}

/// Single historical value entry for a field
class FieldHistoryEntry {
  final String value;
  final DateTime timestamp;

  const FieldHistoryEntry({required this.value, required this.timestamp});

  factory FieldHistoryEntry.fromJson(Map<String, dynamic> json) {
    return FieldHistoryEntry(
      value: json['value'] as String? ?? '',
      timestamp: json['timestamp'] != null
          ? DateTime.parse(json['timestamp'] as String)
          : DateTime.now(),
    );
  }

  Map<String, dynamic> toJson() => {
    'value': value,
    'timestamp': timestamp.toIso8601String(),
  };
}

/// Complete history for a specific field on an item
class FieldHistory {
  final String fieldId;
  final String itemId;
  final List<FieldHistoryEntry> entries;

  const FieldHistory({
    required this.fieldId,
    required this.itemId,
    required this.entries,
  });

  factory FieldHistory.fromJson(Map<String, dynamic> json) {
    return FieldHistory(
      fieldId: json['field_id'] as String? ?? '',
      itemId: json['item_id'] as String? ?? '',
      entries:
          (json['entries'] as List<dynamic>?)
              ?.map(
                (e) => FieldHistoryEntry.fromJson(e as Map<String, dynamic>),
              )
              .toList() ??
          [],
    );
  }

  Map<String, dynamic> toJson() => {
    'field_id': fieldId,
    'item_id': itemId,
    'entries': entries.map((e) => e.toJson()).toList(),
  };

  FieldHistory copyWith({
    String? fieldId,
    String? itemId,
    List<FieldHistoryEntry>? entries,
  }) {
    return FieldHistory(
      fieldId: fieldId ?? this.fieldId,
      itemId: itemId ?? this.itemId,
      entries: entries ?? this.entries,
    );
  }
}

/// All field histories for a profile, keyed by item id and field id
class ProfileFieldHistories {
  final Map<String, Map<String, FieldHistory>>
  histories; // itemId -> fieldId -> FieldHistory

  ProfileFieldHistories({Map<String, Map<String, FieldHistory>>? histories})
    : histories = histories ?? {};

  factory ProfileFieldHistories.fromJson(Map<String, dynamic> json) {
    final result = <String, Map<String, FieldHistory>>{};
    final data = json['histories'] as Map<String, dynamic>? ?? {};
    for (final itemEntry in data.entries) {
      final itemId = itemEntry.key;
      final fields = itemEntry.value as Map<String, dynamic>? ?? {};
      result[itemId] = {};
      for (final fieldEntry in fields.entries) {
        final fieldId = fieldEntry.key;
        final historyData = fieldEntry.value as Map<String, dynamic>? ?? {};
        result[itemId]![fieldId] = FieldHistory.fromJson(historyData);
      }
    }
    return ProfileFieldHistories(histories: result);
  }

  Map<String, dynamic> toJson() => {
    'histories': histories.map(
      (itemId, fields) => MapEntry(
        itemId,
        fields.map((fieldId, h) => MapEntry(fieldId, h.toJson())),
      ),
    ),
  };

  /// Get history for a specific field
  FieldHistory? getHistory(String itemId, String fieldId) {
    return histories[itemId]?[fieldId];
  }

  /// Get all histories for an item
  Map<String, FieldHistory> getItemHistories(String itemId) {
    return histories[itemId] ?? {};
  }

  /// Add a new history entry for a field
  ProfileFieldHistories addEntry(String itemId, String fieldId, String value) {
    final entry = FieldHistoryEntry(value: value, timestamp: DateTime.now());
    final newHistories = Map<String, Map<String, FieldHistory>>.from(
      histories.map((k, v) => MapEntry(k, Map<String, FieldHistory>.from(v))),
    );

    newHistories[itemId] ??= {};
    final existing = newHistories[itemId]![fieldId];
    if (existing != null) {
      newHistories[itemId]![fieldId] = existing.copyWith(
        entries: [...existing.entries, entry],
      );
    } else {
      newHistories[itemId]![fieldId] = FieldHistory(
        fieldId: fieldId,
        itemId: itemId,
        entries: [entry],
      );
    }

    return ProfileFieldHistories(histories: newHistories);
  }
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
    if (decrypted == null) {
      return null;
    }

    try {
      final json = jsonDecode(decrypted) as Map<String, dynamic>;
      final profile = ProfileData.fromJson(json);
      // DEBUG: Log loaded bank accounts
      if (profile.financial?.bankAccounts != null) {
        final deleted = profile.financial!.bankAccounts
            .where((b) => b.isDeleted)
            .toList();
        for (var b in deleted) {}
      }
      return profile;
    } catch (e) {
      return null;
    }
  }

  /// Save profile data for an account
  /// Encrypts and stores via RustVaultService
  Future<bool> saveProfile(String accountId, ProfileData profile) async {
    try {
      final json = jsonEncode(profile.toJson());

      final result = await _rustVault.saveProfileEncrypted(accountId, json);

      if (result == null) {
        return false;
      }

      // DEBUG: Log save success/failure and check bank accounts
      if (profile.financial?.bankAccounts != null) {
        final deleted = profile.financial!.bankAccounts
            .where((b) => b.isDeleted)
            .toList();
        for (var b in deleted) {}
      }
      return true;
    } catch (e) {
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
              itemLabel: e.label.isNotEmpty
                  ? '${e.label} - ${e.value}'
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
              itemLabel: c.label ?? c.number ?? 'ID Card',
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
              itemLabel: a.label ?? 'Address',
              deletedAt: a.deletedAt ?? DateTime.now(),
            ),
          );
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
        if (itemType == 'passport' &&
            index < profile.travel!.passports.length) {
          profile.travel!.passports[index] = profile.travel!.passports[index]
              .copyWith(isDeleted: false, deletedAt: null);
        } else if (itemType == 'visa' && index < profile.travel!.visas.length) {
          profile.travel!.visas[index] = profile.travel!.visas[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
        }
        break;
      case 'financial':
        if (profile.financial == null) return;
        if (itemType == 'bank_account' &&
            index < profile.financial!.bankAccounts.length) {
          profile.financial!.bankAccounts[index] = profile
              .financial!
              .bankAccounts[index]
              .copyWith(isDeleted: false, deletedAt: null);
        } else if (itemType == 'card' &&
            index < profile.financial!.cards.length) {
          profile.financial!.cards[index] = profile.financial!.cards[index]
              .copyWith(isDeleted: false, deletedAt: null);
        } else if (itemType == 'tax_id' &&
            index < profile.financial!.taxIds.length) {
          profile.financial!.taxIds[index] = profile.financial!.taxIds[index]
              .copyWith(isDeleted: false, deletedAt: null);
        }
        break;
      case 'professional':
        if (profile.professional == null) return;
        if (itemType == 'education' &&
            index < profile.professional!.education.length) {
          profile.professional!.education[index] = profile
              .professional!
              .education[index]
              .copyWith(isDeleted: false, deletedAt: null);
        } else if (itemType == 'employment' &&
            index < profile.professional!.employment.length) {
          profile.professional!.employment[index] = profile
              .professional!
              .employment[index]
              .copyWith(isDeleted: false, deletedAt: null);
        } else if (itemType == 'skill' &&
            index < profile.professional!.skills.length) {
          profile.professional!.skills[index] = profile
              .professional!
              .skills[index]
              .copyWith(isDeleted: false, deletedAt: null);
        } else if (itemType == 'language' &&
            index < profile.professional!.languages.length) {
          profile.professional!.languages[index] = profile
              .professional!
              .languages[index]
              .copyWith(isDeleted: false, deletedAt: null);
        }
        break;
      case 'profile':
        if (profile.identity == null) return;
        if (itemType == 'contact' &&
            index < (profile.identity!.contact?.entries.length ?? 0)) {
          final entries = List<ContactEntry>.from(
            profile.identity!.contact!.entries,
          );
          entries[index] = entries[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
          profile.identity = profile.identity!.copyWith(
            contact: ContactData(entries: entries),
          );
        } else if (itemType == 'idCard' &&
            index < (profile.identity!.idCards?.length ?? 0)) {
          final idCards = List<IdCardData>.from(profile.identity!.idCards!);
          idCards[index] = idCards[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
          profile.identity = profile.identity!.copyWith(idCards: idCards);
        } else if (itemType == 'address' &&
            index < (profile.identity!.addresses?.length ?? 0)) {
          final addresses = List<AddressData>.from(
            profile.identity!.addresses!,
          );
          addresses[index] = addresses[index].copyWith(
            isDeleted: false,
            deletedAt: null,
          );
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
        if (itemType == 'passport' &&
            index < profile.travel!.passports.length) {
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
        if (itemType == 'bank_account' &&
            index < profile.financial!.bankAccounts.length) {
          final updated = List<BankAccountData>.from(
            profile.financial!.bankAccounts,
          );
          updated.removeAt(index);
          profile.financial = profile.financial!.copyWith(
            bankAccounts: updated,
          );
        } else if (itemType == 'card' &&
            index < profile.financial!.cards.length) {
          final updated = List<CardData>.from(profile.financial!.cards);
          updated.removeAt(index);
          profile.financial = profile.financial!.copyWith(cards: updated);
        } else if (itemType == 'tax_id' &&
            index < profile.financial!.taxIds.length) {
          final updated = List<TaxIdData>.from(profile.financial!.taxIds);
          updated.removeAt(index);
          profile.financial = profile.financial!.copyWith(taxIds: updated);
        }
        break;
      case 'professional':
        if (profile.professional == null) return;
        if (itemType == 'education' &&
            index < profile.professional!.education.length) {
          final updated = List<EducationData>.from(
            profile.professional!.education,
          );
          updated.removeAt(index);
          profile.professional = profile.professional!.copyWith(
            education: updated,
          );
        } else if (itemType == 'employment' &&
            index < profile.professional!.employment.length) {
          final updated = List<EmploymentData>.from(
            profile.professional!.employment,
          );
          updated.removeAt(index);
          profile.professional = profile.professional!.copyWith(
            employment: updated,
          );
        } else if (itemType == 'skill' &&
            index < profile.professional!.skills.length) {
          final updated = List<SkillData>.from(profile.professional!.skills);
          updated.removeAt(index);
          profile.professional = profile.professional!.copyWith(
            skills: updated,
          );
        } else if (itemType == 'language' &&
            index < profile.professional!.languages.length) {
          final updated = List<LanguageData>.from(
            profile.professional!.languages,
          );
          updated.removeAt(index);
          profile.professional = profile.professional!.copyWith(
            languages: updated,
          );
        }
        break;
      case 'profile':
        if (profile.identity == null) return;
        if (itemType == 'contact' &&
            index < (profile.identity!.contact?.entries.length ?? 0)) {
          final entries = List<ContactEntry>.from(
            profile.identity!.contact!.entries,
          );
          entries.removeAt(index);
          profile.identity = profile.identity!.copyWith(
            contact: ContactData(entries: entries),
          );
        } else if (itemType == 'idCard' &&
            index < (profile.identity!.idCards?.length ?? 0)) {
          final idCards = List<IdCardData>.from(profile.identity!.idCards!);
          idCards.removeAt(index);
          profile.identity = profile.identity!.copyWith(idCards: idCards);
        } else if (itemType == 'address' &&
            index < (profile.identity!.addresses?.length ?? 0)) {
          final addresses = List<AddressData>.from(
            profile.identity!.addresses!,
          );
          addresses.removeAt(index);
          profile.identity = profile.identity!.copyWith(addresses: addresses);
        }
        break;
    }
    await saveProfile(accountId, profile);
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
      final entries = profile.identity!.contact!.entries
          .where((e) => !e.isDeleted)
          .toList();
      profile.identity = profile.identity!.copyWith(
        contact: ContactData(entries: entries),
      );
    }
    if (profile.identity?.idCards != null) {
      final idCards = profile.identity!.idCards!
          .where((c) => !c.isDeleted)
          .toList();
      profile.identity = profile.identity!.copyWith(idCards: idCards);
    }
    if (profile.identity?.addresses != null) {
      final addresses = profile.identity!.addresses!
          .where((a) => !a.isDeleted)
          .toList();
      profile.identity = profile.identity!.copyWith(addresses: addresses);
    }

    await saveProfile(accountId, profile);
  }

  /// Load field histories for an account
  Future<ProfileFieldHistories> loadFieldHistories(String accountId) async {
    try {
      final dir = await storageDir;
      final file = File('${dir.path}/${accountId}_field_histories.json');
      if (!await file.exists()) {
        return ProfileFieldHistories();
      }
      final contents = await file.readAsString();
      final json = jsonDecode(contents) as Map<String, dynamic>;
      return ProfileFieldHistories.fromJson(json);
    } catch (e) {
      return ProfileFieldHistories();
    }
  }

  /// Save field histories for an account
  Future<bool> saveFieldHistories(
    String accountId,
    ProfileFieldHistories histories,
  ) async {
    try {
      final dir = await storageDir;
      if (!await dir.exists()) {
        await dir.create(recursive: true);
      }
      final file = File('${dir.path}/${accountId}_field_histories.json');
      final json = jsonEncode(histories.toJson());
      await file.writeAsString(json);
      return true;
    } catch (e) {
      return false;
    }
  }

  /// Add a field history entry when a field is updated
  Future<ProfileFieldHistories> addFieldHistory({
    required String accountId,
    required String itemId,
    required String fieldId,
    required String value,
    ProfileFieldHistories? existingHistories,
  }) async {
    final histories = existingHistories ?? await loadFieldHistories(accountId);
    final updated = histories.addEntry(itemId, fieldId, value);
    await saveFieldHistories(accountId, updated);
    return updated;
  }
}
