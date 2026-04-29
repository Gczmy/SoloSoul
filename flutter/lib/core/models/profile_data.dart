import 'package:flutter/material.dart';
import 'package:json_annotation/json_annotation.dart';
import 'package:solosoul_flutter/core/models/base_models.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';

part 'profile_data.g.dart';

// Re-export for backward compatibility
/// Maximum character limits for form fields
const int kMaxFieldLength = 32;
const int kMaxNameLength = 16;

/// Sentinel value for copyWith to distinguish "not provided" from "explicitly null"
class _DeletedAtSentinel {
  const _DeletedAtSentinel();
}

/// Returns current timestamp in milliseconds since epoch
int currentTimestamp() => DateTime.now().millisecondsSinceEpoch;

@JsonSerializable(explicitToJson: true)
class ProfileData {
  final IdentityData? identity;
  final TravelData? travel;
  final FinancialData? financial;
  final ProfessionalData? professional;
  @JsonKey(name: 'unified_objects')
  final UnifiedObjectData? unifiedObjects;
  @JsonKey(name: 'schema_version')
  final int? schemaVersion;

  const ProfileData({
    this.identity,
    this.travel,
    this.financial,
    this.professional,
    this.unifiedObjects,
    this.schemaVersion,
  });

  factory ProfileData.fromJson(Map<String, dynamic> json) =>
      _$ProfileDataFromJson(json);

  Map<String, dynamic> toJson() => _$ProfileDataToJson(this);

  /// Collect all item IDs across legacy sections and unified objects.
  /// Used for orphan history cleanup and integrity validation.
  Set<String> collectAllItemIds() {
    final ids = <String>{};

    // UnifiedObject IDs
    if (unifiedObjects != null) {
      for (final obj in unifiedObjects!.objects) {
        ids.add(obj.id);
      }
    }

    // Identity section
    if (identity != null) {
      for (final card in identity!.idCards ?? []) {
        ids.add(card.id);
      }
      for (final addr in identity!.addresses ?? []) {
        ids.add(addr.id);
      }
      for (final entry in identity!.contact?.entries ?? []) {
        ids.add(entry.id);
      }
    }

    // Travel section
    if (travel != null) {
      for (final p in travel!.passports) {
        ids.add(p.id);
      }
      for (final v in travel!.visas) {
        ids.add(v.id);
      }
      for (final t in travel!.travelHistory) {
        ids.add(t.id);
      }
    }

    // Financial section
    if (financial != null) {
      for (final b in financial!.bankAccounts) {
        ids.add(b.id);
      }
      for (final c in financial!.cards) {
        ids.add(c.id);
      }
      for (final t in financial!.taxIds) {
        ids.add(t.id);
      }
    }

    // Professional section
    if (professional != null) {
      for (final e in professional!.education) {
        ids.add(e.id);
      }
      for (final emp in professional!.employment) {
        ids.add(emp.id);
      }
      for (final s in professional!.skills) {
        ids.add(s.id);
      }
      for (final l in professional!.languages) {
        ids.add(l.id);
      }
      for (final a in professional!.awards) {
        ids.add(a.id);
      }
    }

    return ids;
  }

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

@JsonSerializable(explicitToJson: true)
class IdentityData {
  @JsonKey(name: 'full_name')
  final String? fullName;
  @JsonKey(name: 'given_name')
  final String? givenName;
  @JsonKey(name: 'family_name')
  final String? familyName;
  @JsonKey(name: 'date_of_birth')
  final String? dateOfBirth;
  final String? gender;
  final String? nationality;
  @JsonKey(name: 'id_cards')
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

  factory IdentityData.fromJson(Map<String, dynamic> json) =>
      _$IdentityDataFromJson(json);

  Map<String, dynamic> toJson() => _$IdentityDataToJson(this);

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
