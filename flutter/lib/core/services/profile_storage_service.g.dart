// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: unused_element

part of 'profile_storage_service.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ProfileData _$ProfileDataFromJson(Map<String, dynamic> json) => ProfileData(
  identity: json['identity'] == null
      ? null
      : IdentityData.fromJson(json['identity'] as Map<String, dynamic>),
  travel: json['travel'] == null
      ? null
      : TravelData.fromJson(json['travel'] as Map<String, dynamic>),
  financial: json['financial'] == null
      ? null
      : FinancialData.fromJson(json['financial'] as Map<String, dynamic>),
  professional: json['professional'] == null
      ? null
      : ProfessionalData.fromJson(json['professional'] as Map<String, dynamic>),
  unifiedObjects: json['unifiedObjects'] == null
      ? null
      : UnifiedObjectData.fromJson(
          json['unifiedObjects'] as Map<String, dynamic>,
        ),
  schemaVersion: (json['schemaVersion'] as num?)?.toInt(),
);

Map<String, dynamic> _$ProfileDataToJson(ProfileData instance) =>
    <String, dynamic>{
      'identity': instance.identity?.toJson(),
      'travel': instance.travel?.toJson(),
      'financial': instance.financial?.toJson(),
      'professional': instance.professional?.toJson(),
      'unifiedObjects': instance.unifiedObjects?.toJson(),
      'schemaVersion': instance.schemaVersion,
    };

ContactEntry _$ContactEntryFromJson(Map<String, dynamic> json) => ContactEntry(
  id: json['id'] as String,
  title: json['title'] as String,
  type: json['type'] as String,
  value: json['value'] as String,
  updatedAt: (json['updatedAt'] as num?)?.toInt(),
  isDeleted: json['isDeleted'] as bool? ?? false,
  deletedAt: json['deletedAt'] == null
      ? null
      : DateTime.parse(json['deletedAt'] as String),
);

Map<String, dynamic> _$ContactEntryToJson(ContactEntry instance) =>
    <String, dynamic>{
      'id': instance.id,
      'title': instance.title,
      'type': instance.type,
      'value': instance.value,
      'updatedAt': instance.updatedAt,
      'isDeleted': instance.isDeleted,
      'deletedAt': instance.deletedAt?.toIso8601String(),
    };

ContactData _$ContactDataFromJson(Map<String, dynamic> json) => ContactData(
  entries:
      (json['entries'] as List<dynamic>?)
          ?.map((e) => ContactEntry.fromJson(e as Map<String, dynamic>))
          .toList() ??
      const [],
);

Map<String, dynamic> _$ContactDataToJson(ContactData instance) =>
    <String, dynamic>{
      'entries': instance.entries.map((e) => e.toJson()).toList(),
    };

AddressData _$AddressDataFromJson(Map<String, dynamic> json) => AddressData(
  id: json['id'] as String,
  title: json['title'] as String?,
  street: json['street'] as String?,
  city: json['city'] as String?,
  state: json['state'] as String?,
  postalCode: json['postalCode'] as String?,
  country: json['country'] as String?,
  updatedAt: (json['updatedAt'] as num?)?.toInt(),
  isDeleted: json['isDeleted'] as bool? ?? false,
  deletedAt: json['deletedAt'] == null
      ? null
      : DateTime.parse(json['deletedAt'] as String),
);

Map<String, dynamic> _$AddressDataToJson(AddressData instance) =>
    <String, dynamic>{
      'id': instance.id,
      'title': instance.title,
      'street': instance.street,
      'city': instance.city,
      'state': instance.state,
      'postalCode': instance.postalCode,
      'country': instance.country,
      'updatedAt': instance.updatedAt,
      'isDeleted': instance.isDeleted,
      'deletedAt': instance.deletedAt?.toIso8601String(),
    };

IdCardData _$IdCardDataFromJson(Map<String, dynamic> json) => IdCardData(
  id: json['id'] as String,
  title: json['title'] as String?,
  number: json['number'] as String?,
  issueDate: json['issueDate'] as String?,
  expiryDate: json['expiryDate'] as String?,
  holderName: json['holderName'] as String?,
  country: json['country'] as String?,
  updatedAt: (json['updatedAt'] as num?)?.toInt(),
  isDeleted: json['isDeleted'] as bool? ?? false,
  deletedAt: json['deletedAt'] == null
      ? null
      : DateTime.parse(json['deletedAt'] as String),
);

Map<String, dynamic> _$IdCardDataToJson(IdCardData instance) =>
    <String, dynamic>{
      'id': instance.id,
      'title': instance.title,
      'number': instance.number,
      'issueDate': instance.issueDate,
      'expiryDate': instance.expiryDate,
      'holderName': instance.holderName,
      'country': instance.country,
      'updatedAt': instance.updatedAt,
      'isDeleted': instance.isDeleted,
      'deletedAt': instance.deletedAt?.toIso8601String(),
    };

TravelHistoryData _$TravelHistoryDataFromJson(Map<String, dynamic> json) =>
    TravelHistoryData(
      id: json['id'] as String,
      destination: json['destination'] as String,
      date: json['date'] as String?,
      departureCity: json['departureCity'] as String?,
      departureTime: json['departureTime'] as String?,
      arrivalTime: json['arrivalTime'] as String?,
      flightNumber: json['flightNumber'] as String?,
      ticketPrice: json['ticketPrice'] as String?,
      airline: json['airline'] as String?,
      travelType: json['travelType'] as String?,
      updatedAt: (json['updatedAt'] as num?)?.toInt(),
      isDeleted: json['isDeleted'] as bool? ?? false,
      deletedAt: json['deletedAt'] == null
          ? null
          : DateTime.parse(json['deletedAt'] as String),
    );

Map<String, dynamic> _$TravelHistoryDataToJson(TravelHistoryData instance) =>
    <String, dynamic>{
      'id': instance.id,
      'destination': instance.destination,
      'date': instance.date,
      'departureCity': instance.departureCity,
      'departureTime': instance.departureTime,
      'arrivalTime': instance.arrivalTime,
      'flightNumber': instance.flightNumber,
      'ticketPrice': instance.ticketPrice,
      'airline': instance.airline,
      'travelType': instance.travelType,
      'updatedAt': instance.updatedAt,
      'isDeleted': instance.isDeleted,
      'deletedAt': instance.deletedAt?.toIso8601String(),
    };

TravelData _$TravelDataFromJson(Map<String, dynamic> json) => TravelData(
  passports:
      (json['passports'] as List<dynamic>?)
          ?.map((e) => PassportData.fromJson(e as Map<String, dynamic>))
          .toList() ??
      const [],
  visas:
      (json['visas'] as List<dynamic>?)
          ?.map((e) => VisaData.fromJson(e as Map<String, dynamic>))
          .toList() ??
      const [],
  travelHistory:
      (json['travelHistory'] as List<dynamic>?)
          ?.map((e) => TravelHistoryData.fromJson(e as Map<String, dynamic>))
          .toList() ??
      const [],
);

Map<String, dynamic> _$TravelDataToJson(TravelData instance) =>
    <String, dynamic>{
      'passports': instance.passports.map((e) => e.toJson()).toList(),
      'visas': instance.visas.map((e) => e.toJson()).toList(),
      'travelHistory': instance.travelHistory.map((e) => e.toJson()).toList(),
    };

PassportData _$PassportDataFromJson(Map<String, dynamic> json) => PassportData(
  id: json['id'] as String,
  title: json['title'] as String?,
  number: json['number'] as String?,
  country: json['country'] as String?,
  countryCode: json['countryCode'] as String?,
  issueDate: json['issueDate'] as String?,
  placeOfIssue: json['placeOfIssue'] as String?,
  expiryDate: json['expiryDate'] as String?,
  dateOfBirth: json['dateOfBirth'] as String?,
  placeOfBirth: json['placeOfBirth'] as String?,
  sex: json['sex'] as String?,
  nationality: json['nationality'] as String?,
  authority: json['authority'] as String?,
  holderName: json['holderName'] as String?,
  updatedAt: (json['updatedAt'] as num?)?.toInt(),
  isDeleted: json['isDeleted'] as bool? ?? false,
  deletedAt: json['deletedAt'] == null
      ? null
      : DateTime.parse(json['deletedAt'] as String),
);

Map<String, dynamic> _$PassportDataToJson(PassportData instance) =>
    <String, dynamic>{
      'id': instance.id,
      'title': instance.title,
      'number': instance.number,
      'country': instance.country,
      'countryCode': instance.countryCode,
      'issueDate': instance.issueDate,
      'placeOfIssue': instance.placeOfIssue,
      'expiryDate': instance.expiryDate,
      'dateOfBirth': instance.dateOfBirth,
      'placeOfBirth': instance.placeOfBirth,
      'sex': instance.sex,
      'nationality': instance.nationality,
      'authority': instance.authority,
      'holderName': instance.holderName,
      'updatedAt': instance.updatedAt,
      'isDeleted': instance.isDeleted,
      'deletedAt': instance.deletedAt?.toIso8601String(),
    };

VisaData _$VisaDataFromJson(Map<String, dynamic> json) => VisaData(
  id: json['id'] as String,
  title: json['title'] as String?,
  country: json['country'] as String?,
  visaType: json['visaType'] as String?,
  number: json['number'] as String?,
  issueDate: json['issueDate'] as String?,
  expiryDate: json['expiryDate'] as String?,
  updatedAt: (json['updatedAt'] as num?)?.toInt(),
  isDeleted: json['isDeleted'] as bool? ?? false,
  deletedAt: json['deletedAt'] == null
      ? null
      : DateTime.parse(json['deletedAt'] as String),
);

Map<String, dynamic> _$VisaDataToJson(VisaData instance) => <String, dynamic>{
  'id': instance.id,
  'title': instance.title,
  'country': instance.country,
  'visaType': instance.visaType,
  'number': instance.number,
  'issueDate': instance.issueDate,
  'expiryDate': instance.expiryDate,
  'updatedAt': instance.updatedAt,
  'isDeleted': instance.isDeleted,
  'deletedAt': instance.deletedAt?.toIso8601String(),
};

FinancialData _$FinancialDataFromJson(Map<String, dynamic> json) =>
    FinancialData(
      bankAccounts:
          (json['bankAccounts'] as List<dynamic>?)
              ?.map((e) => BankAccountData.fromJson(e as Map<String, dynamic>))
              .toList() ??
          const [],
      cards:
          (json['cards'] as List<dynamic>?)
              ?.map((e) => CardData.fromJson(e as Map<String, dynamic>))
              .toList() ??
          const [],
      taxIds:
          (json['taxIds'] as List<dynamic>?)
              ?.map((e) => TaxIdData.fromJson(e as Map<String, dynamic>))
              .toList() ??
          const [],
    );

Map<String, dynamic> _$FinancialDataToJson(FinancialData instance) =>
    <String, dynamic>{
      'bankAccounts': instance.bankAccounts.map((e) => e.toJson()).toList(),
      'cards': instance.cards.map((e) => e.toJson()).toList(),
      'taxIds': instance.taxIds.map((e) => e.toJson()).toList(),
    };

BankAccountData _$BankAccountDataFromJson(Map<String, dynamic> json) =>
    BankAccountData(
      id: json['id'] as String,
      title: json['title'] as String?,
      bankName: json['bankName'] as String?,
      accountNumber: json['accountNumber'] as String?,
      currency: json['currency'] as String?,
      swiftBic: json['swiftBic'] as String?,
      sortCode: json['sortCode'] as String?,
      updatedAt: (json['updatedAt'] as num?)?.toInt(),
      isDeleted: json['isDeleted'] as bool? ?? false,
      deletedAt: json['deletedAt'] == null
          ? null
          : DateTime.parse(json['deletedAt'] as String),
    );

Map<String, dynamic> _$BankAccountDataToJson(BankAccountData instance) =>
    <String, dynamic>{
      'id': instance.id,
      'title': instance.title,
      'bankName': instance.bankName,
      'accountNumber': instance.accountNumber,
      'currency': instance.currency,
      'swiftBic': instance.swiftBic,
      'sortCode': instance.sortCode,
      'updatedAt': instance.updatedAt,
      'isDeleted': instance.isDeleted,
      'deletedAt': instance.deletedAt?.toIso8601String(),
    };

CardData _$CardDataFromJson(Map<String, dynamic> json) => CardData(
  id: json['id'] as String,
  title: json['title'] as String?,
  cardNumber: json['cardNumber'] as String?,
  cardType: json['cardType'] as String?,
  expiryDate: json['expiryDate'] as String?,
  holderName: json['holderName'] as String?,
  cvv: json['cvv'] as String?,
  updatedAt: (json['updatedAt'] as num?)?.toInt(),
  isDeleted: json['isDeleted'] as bool? ?? false,
  deletedAt: json['deletedAt'] == null
      ? null
      : DateTime.parse(json['deletedAt'] as String),
);

Map<String, dynamic> _$CardDataToJson(CardData instance) => <String, dynamic>{
  'id': instance.id,
  'title': instance.title,
  'cardNumber': instance.cardNumber,
  'cardType': instance.cardType,
  'expiryDate': instance.expiryDate,
  'holderName': instance.holderName,
  'cvv': instance.cvv,
  'updatedAt': instance.updatedAt,
  'isDeleted': instance.isDeleted,
  'deletedAt': instance.deletedAt?.toIso8601String(),
};

TaxIdData _$TaxIdDataFromJson(Map<String, dynamic> json) => TaxIdData(
  id: json['id'] as String,
  title: json['title'] as String?,
  taxIdNumber: json['taxIdNumber'] as String?,
  taxIdType: json['taxIdType'] as String?,
  issuingAuthority: json['issuingAuthority'] as String?,
  country: json['country'] as String?,
  updatedAt: (json['updatedAt'] as num?)?.toInt(),
  isDeleted: json['isDeleted'] as bool? ?? false,
  deletedAt: json['deletedAt'] == null
      ? null
      : DateTime.parse(json['deletedAt'] as String),
);

Map<String, dynamic> _$TaxIdDataToJson(TaxIdData instance) => <String, dynamic>{
  'id': instance.id,
  'title': instance.title,
  'taxIdNumber': instance.taxIdNumber,
  'taxIdType': instance.taxIdType,
  'issuingAuthority': instance.issuingAuthority,
  'country': instance.country,
  'updatedAt': instance.updatedAt,
  'isDeleted': instance.isDeleted,
  'deletedAt': instance.deletedAt?.toIso8601String(),
};

SkillData _$SkillDataFromJson(Map<String, dynamic> json) => SkillData(
  id: json['id'] as String,
  name: json['name'] as String,
  level: json['level'] as String?,
  updatedAt: (json['updatedAt'] as num?)?.toInt(),
  isDeleted: json['isDeleted'] as bool? ?? false,
  deletedAt: json['deletedAt'] == null
      ? null
      : DateTime.parse(json['deletedAt'] as String),
);

Map<String, dynamic> _$SkillDataToJson(SkillData instance) => <String, dynamic>{
  'id': instance.id,
  'name': instance.name,
  'level': instance.level,
  'updatedAt': instance.updatedAt,
  'isDeleted': instance.isDeleted,
  'deletedAt': instance.deletedAt?.toIso8601String(),
};

LanguageData _$LanguageDataFromJson(Map<String, dynamic> json) => LanguageData(
  id: json['id'] as String,
  name: json['name'] as String,
  proficiency: json['proficiency'] as String?,
  updatedAt: (json['updatedAt'] as num?)?.toInt(),
  isDeleted: json['isDeleted'] as bool? ?? false,
  deletedAt: json['deletedAt'] == null
      ? null
      : DateTime.parse(json['deletedAt'] as String),
);

Map<String, dynamic> _$LanguageDataToJson(LanguageData instance) =>
    <String, dynamic>{
      'id': instance.id,
      'name': instance.name,
      'proficiency': instance.proficiency,
      'updatedAt': instance.updatedAt,
      'isDeleted': instance.isDeleted,
      'deletedAt': instance.deletedAt?.toIso8601String(),
    };

AwardData _$AwardDataFromJson(Map<String, dynamic> json) => AwardData(
  id: json['id'] as String,
  title: json['title'] as String?,
  issuer: json['issuer'] as String?,
  date: json['date'] as String?,
  description: json['description'] as String?,
  updatedAt: (json['updatedAt'] as num?)?.toInt(),
  isDeleted: json['isDeleted'] as bool? ?? false,
  deletedAt: json['deletedAt'] == null
      ? null
      : DateTime.parse(json['deletedAt'] as String),
);

Map<String, dynamic> _$AwardDataToJson(AwardData instance) => <String, dynamic>{
  'id': instance.id,
  'title': instance.title,
  'issuer': instance.issuer,
  'date': instance.date,
  'description': instance.description,
  'updatedAt': instance.updatedAt,
  'isDeleted': instance.isDeleted,
  'deletedAt': instance.deletedAt?.toIso8601String(),
};

ProfessionalData _$ProfessionalDataFromJson(Map<String, dynamic> json) =>
    ProfessionalData(
      education:
          (json['education'] as List<dynamic>?)
              ?.map((e) => EducationData.fromJson(e as Map<String, dynamic>))
              .toList() ??
          const [],
      employment:
          (json['employment'] as List<dynamic>?)
              ?.map((e) => EmploymentData.fromJson(e as Map<String, dynamic>))
              .toList() ??
          const [],
      skills:
          (json['skills'] as List<dynamic>?)
              ?.map((e) => SkillData.fromJson(e as Map<String, dynamic>))
              .toList() ??
          const [],
      languages:
          (json['languages'] as List<dynamic>?)
              ?.map((e) => LanguageData.fromJson(e as Map<String, dynamic>))
              .toList() ??
          const [],
      awards:
          (json['awards'] as List<dynamic>?)
              ?.map((e) => AwardData.fromJson(e as Map<String, dynamic>))
              .toList() ??
          const [],
    );

Map<String, dynamic> _$ProfessionalDataToJson(ProfessionalData instance) =>
    <String, dynamic>{
      'education': instance.education.map((e) => e.toJson()).toList(),
      'employment': instance.employment.map((e) => e.toJson()).toList(),
      'skills': instance.skills.map((e) => e.toJson()).toList(),
      'languages': instance.languages.map((e) => e.toJson()).toList(),
      'awards': instance.awards.map((e) => e.toJson()).toList(),
    };

EducationData _$EducationDataFromJson(Map<String, dynamic> json) =>
    EducationData(
      id: json['id'] as String,
      institution: json['institution'] as String?,
      degree: json['degree'] as String?,
      degreeCustom: json['degreeCustom'] as String?,
      field: json['field'] as String?,
      startDate: json['startDate'] as String?,
      endDate: json['endDate'] as String?,
      updatedAt: (json['updatedAt'] as num?)?.toInt(),
      isDeleted: json['isDeleted'] as bool? ?? false,
      deletedAt: json['deletedAt'] == null
          ? null
          : DateTime.parse(json['deletedAt'] as String),
    );

Map<String, dynamic> _$EducationDataToJson(EducationData instance) =>
    <String, dynamic>{
      'id': instance.id,
      'institution': instance.institution,
      'degree': instance.degree,
      'degreeCustom': instance.degreeCustom,
      'field': instance.field,
      'startDate': instance.startDate,
      'endDate': instance.endDate,
      'updatedAt': instance.updatedAt,
      'isDeleted': instance.isDeleted,
      'deletedAt': instance.deletedAt?.toIso8601String(),
    };

EmploymentData _$EmploymentDataFromJson(Map<String, dynamic> json) =>
    EmploymentData(
      id: json['id'] as String,
      company: json['company'] as String?,
      position: json['position'] as String?,
      responsibilities: json['responsibilities'] as String?,
      startDate: json['startDate'] as String?,
      endDate: json['endDate'] as String?,
      updatedAt: (json['updatedAt'] as num?)?.toInt(),
      isDeleted: json['isDeleted'] as bool? ?? false,
      deletedAt: json['deletedAt'] == null
          ? null
          : DateTime.parse(json['deletedAt'] as String),
    );

Map<String, dynamic> _$EmploymentDataToJson(EmploymentData instance) =>
    <String, dynamic>{
      'id': instance.id,
      'company': instance.company,
      'position': instance.position,
      'responsibilities': instance.responsibilities,
      'startDate': instance.startDate,
      'endDate': instance.endDate,
      'updatedAt': instance.updatedAt,
      'isDeleted': instance.isDeleted,
      'deletedAt': instance.deletedAt?.toIso8601String(),
    };
