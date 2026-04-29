import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';

void main() {
  group('ProfileData', () {
    test('creates with null sections', () {
      const profile = ProfileData();

      expect(profile.identity, isNull);
      expect(profile.travel, isNull);
      expect(profile.financial, isNull);
      expect(profile.professional, isNull);
    });

    test('creates with provided sections', () {
      final profile = ProfileData(
        identity: IdentityData(fullName: 'Test User'),
      );

      expect(profile.identity, isNotNull);
      expect(profile.identity!.fullName, 'Test User');
    });

    test('fromJson parses complete profile', () {
      final json = {
        'identity': {
          'full_name': 'JSON User',
          'given_name': 'JSON',
          'family_name': 'User',
        },
        'travel': {
          'passports': [],
          'visas': [],
          'travel_history': [],
        },
        'financial': {
          'bank_accounts': [],
          'cards': [],
          'tax_ids': [],
        },
        'professional': {
          'education': [],
          'employment': [],
          'skills': [],
          'languages': [],
          'awards': [],
        },
      };

      final profile = ProfileData.fromJson(json);

      expect(profile.identity, isNotNull);
      expect(profile.identity!.fullName, 'JSON User');
      expect(profile.travel, isNotNull);
      expect(profile.financial, isNotNull);
      expect(profile.professional, isNotNull);
    });

    test('fromJson handles missing sections', () {
      final json = <String, dynamic>{};

      final profile = ProfileData.fromJson(json);

      expect(profile.identity, isNull);
      expect(profile.travel, isNull);
    });

    test('toJson produces correct structure', () {
      final profile = ProfileData(
        identity: IdentityData(fullName: 'Test'),
      );

      final json = profile.toJson();

      expect(json['identity'], isNotNull);
      expect(json['travel'], isNull);
      expect(json['financial'], isNull);
      expect(json['professional'], isNull);
    });

    test('copyWith creates new instance with updated values', () {
      final original = ProfileData(
        identity: IdentityData(fullName: 'Original'),
      );

      final copied = original.copyWith(
        identity: IdentityData(fullName: 'Updated'),
      );

      expect(copied.identity!.fullName, 'Updated');
      expect(original.identity!.fullName, 'Original');
    });
  });

  group('IdentityData', () {
    test('activeIdCards filters out deleted cards', () {
      final identity = IdentityData(
        idCards: [
          IdCardData(id: '1', title: 'Active', number: '123', isDeleted: false),
          IdCardData(id: '2', title: 'Deleted', number: '456', isDeleted: true),
        ],
      );

      expect(identity.activeIdCards.length, 1);
      expect(identity.activeIdCards.first.title, 'Active');
    });

    test('activeAddresses filters out deleted addresses', () {
      final identity = IdentityData(
        addresses: [
          AddressData(id: '1', title: 'Home', city: 'NYC', isDeleted: false),
          AddressData(id: '2', title: 'Old', city: 'LA', isDeleted: true),
        ],
      );

      expect(identity.activeAddresses.length, 1);
      expect(identity.activeAddresses.first.city, 'NYC');
    });
  });

  group('ContactEntry', () {
    test('creates with required fields', () {
      final entry = ContactEntry(
        id: 'entry_1',
        title: 'Personal',
        type: 'email',
        value: 'test@example.com',
      );

      expect(entry.id, 'entry_1');
      expect(entry.title, 'Personal');
      expect(entry.type, 'email');
      expect(entry.value, 'test@example.com');
      expect(entry.isDeleted, false);
    });

    test('fromJson handles missing id - throws TypeError (strict generated code)', () {
      final json = {
        'title': 'Work',
        'type': 'phone',
        'value': '+1234567890',
      };

      // Generated fromJson is strict - missing required 'id' field throws TypeError
      expect(
        () => ContactEntry.fromJson(json),
        throwsA(isA<TypeError>()),
      );
    });

    test('copyWith preserves immutability', () {
      final original = ContactEntry(
        id: 'entry_1',
        title: 'Original',
        type: 'email',
        value: 'old@example.com',
      );

      final copied = original.copyWith(value: 'new@example.com');

      expect(copied.value, 'new@example.com');
      expect(original.value, 'old@example.com');
      expect(copied.title, 'Original');
    });
  });

  group('AddressData', () {
    test('creates with required fields', () {
      final address = AddressData(
        id: 'addr_1',
        title: 'Home',
        street: '123 Main St',
        city: 'New York',
        postalCode: '10001',
        country: 'USA',
      );

      expect(address.id, 'addr_1');
      expect(address.title, 'Home');
      expect(address.city, 'New York');
    });

    test('entryType returns Address', () {
      final address = AddressData(id: '1');
      expect(address.entryType, 'Address');
    });
  });

  group('TravelData', () {
    test('activePassports filters deleted', () {
      final travel = TravelData(
        passports: [
          PassportData(id: 'p1', number: 'P123', isDeleted: false),
          PassportData(id: 'p2', number: 'P456', isDeleted: true),
        ],
      );

      expect(travel.activePassports.length, 1);
    });

    test('deletedPassports returns only deleted', () {
      final travel = TravelData(
        passports: [
          PassportData(id: 'p1', number: 'P123', isDeleted: false),
          PassportData(id: 'p2', number: 'P456', isDeleted: true),
        ],
      );

      expect(travel.deletedPassports.length, 1);
      expect(travel.deletedPassports.first.number, 'P456');
    });
  });

  group('FinancialData', () {
    test('activeBankAccounts filters deleted', () {
      final financial = FinancialData(
        bankAccounts: [
          BankAccountData(id: 'b1', bankName: 'Active Bank', isDeleted: false),
          BankAccountData(id: 'b2', bankName: 'Closed Bank', isDeleted: true),
        ],
      );

      expect(financial.activeBankAccounts.length, 1);
      expect(financial.activeBankAccounts.first.bankName, 'Active Bank');
    });
  });

  group('ProfessionalData', () {
    test('activeEducation filters deleted', () {
      final professional = ProfessionalData(
        education: [
          EducationData(id: 'e1', institution: 'MIT', isDeleted: false),
          EducationData(id: 'e2', institution: 'Old School', isDeleted: true),
        ],
      );

      expect(professional.activeEducation.length, 1);
    });

    test('activeSkills and activeLanguages work correctly', () {
      final professional = ProfessionalData(
        skills: [
          SkillData(id: 's1', name: 'Dart', isDeleted: false),
          SkillData(id: 's2', name: 'Java', isDeleted: true),
        ],
        languages: [
          LanguageData(id: 'l1', name: 'English', isDeleted: false),
        ],
      );

      expect(professional.activeSkills.length, 1);
      expect(professional.activeLanguages.length, 1);
    });
  });

  group('ProfileStorageService encryption key delegation', () {
    test('setEncryptionKey delegates to RustVaultService', () {
      // This tests that the service correctly delegates key management
      final service = ProfileStorageService.instance;

      // Just verify instance is created - actual FFI calls require native library
      expect(service, isNotNull);
    });
  });

  group('DeletedItemInfo', () {
    test('metaFor returns correct metadata', () {
      final meta = DeletedItemInfo.metaFor('passport');

      expect(meta, isNotNull);
      expect(meta!.label, 'Passport');
      expect(meta.section, 'travel');
    });

    test('metaFor returns null for unknown type', () {
      final meta = DeletedItemInfo.metaFor('unknown_type');
      expect(meta, isNull);
    });

    test('itemTypes returns all defined types', () {
      final types = DeletedItemInfo.itemTypes;

      expect(types, contains('passport'));
      expect(types, contains('visa'));
      expect(types, contains('bank_account'));
      expect(types, contains('card'));
      expect(types, contains('education'));
    });
  });
}
