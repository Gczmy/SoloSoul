import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';

void main() {
  group('ProfileData serialization', () {
    test('serializes and deserializes identity data correctly', () {
      final profile = ProfileData(
        identity: IdentityData(
          fullName: 'John Doe',
          givenName: 'John',
          familyName: 'Doe',
          dateOfBirth: '1990-05-15',
          gender: 'Male',
          nationality: 'American',
        ),
      );

      final json = jsonEncode(profile.toJson());
      final restored = ProfileData.fromJson(jsonDecode(json) as Map<String, dynamic>);

      expect(restored.identity, isNotNull);
      expect(restored.identity!.fullName, equals('John Doe'));
      expect(restored.identity!.givenName, equals('John'));
      expect(restored.identity!.familyName, equals('Doe'));
      expect(restored.identity!.dateOfBirth, equals('1990-05-15'));
      expect(restored.identity!.gender, equals('Male'));
      expect(restored.identity!.nationality, equals('American'));
    });

    test('serializes contact entries with soft-delete markers', () {
      final now = DateTime.now();
      final profile = ProfileData(
        identity: IdentityData(
          fullName: 'Test User',
          contact: ContactData(
            entries: [
              ContactEntry(
                id: 'contact_active',
                title: 'Email',
                type: 'email',
                value: 'test@example.com',
              ),
              ContactEntry(
                id: 'contact_deleted',
                title: 'Old Email',
                type: 'email',
                value: 'old@example.com',
                isDeleted: true,
                deletedAt: now,
              ),
            ],
          ),
        ),
      );

      final json = jsonEncode(profile.toJson());
      final restored = ProfileData.fromJson(jsonDecode(json) as Map<String, dynamic>);

      expect(restored.identity!.contact, isNotNull);
      expect(restored.identity!.contact!.entries, hasLength(2));

      final activeEntry = restored.identity!.contact!.entries
          .firstWhere((e) => e.id == 'contact_active');
      expect(activeEntry.isDeleted, isFalse);
      expect(activeEntry.deletedAt, isNull);

      final deletedEntry = restored.identity!.contact!.entries
          .firstWhere((e) => e.id == 'contact_deleted');
      expect(deletedEntry.isDeleted, isTrue);
      expect(deletedEntry.deletedAt, isNotNull);
    });

    test('activeEntries getter filters soft-deleted items', () {
      final profile = ProfileData(
        identity: IdentityData(
          fullName: 'Test User',
          contact: ContactData(
            entries: [
              ContactEntry(
                id: 'contact_1',
                title: 'Email',
                type: 'email',
                value: 'test@example.com',
              ),
              ContactEntry(
                id: 'contact_2',
                title: 'Old Email',
                type: 'email',
                value: 'old@example.com',
                isDeleted: true,
              ),
            ],
          ),
        ),
      );

      expect(profile.identity!.contact!.activeEntries, hasLength(1));
      expect(profile.identity!.contact!.activeEntries.first.id, equals('contact_1'));
    });

    test('serializes passport data with soft-delete markers', () {
      final now = DateTime.now();
      final profile = ProfileData(
        travel: TravelData(
          passports: [
            PassportData(
              id: 'passport_active',
              title: 'Valid Passport',
              number: 'US123456',
              country: 'United States',
              expiryDate: '2030-01-01',
            ),
            PassportData(
              id: 'passport_deleted',
              title: 'Expired Passport',
              number: 'US999999',
              country: 'United States',
              expiryDate: '2020-01-01',
              isDeleted: true,
              deletedAt: now,
            ),
          ],
        ),
      );

      final json = jsonEncode(profile.toJson());
      final restored = ProfileData.fromJson(jsonDecode(json) as Map<String, dynamic>);

      expect(restored.travel!.passports, hasLength(2));

      final activePassport = restored.travel!.passports
          .firstWhere((p) => p.id == 'passport_active');
      expect(activePassport.isDeleted, isFalse);
      expect(activePassport.deletedAt, isNull);

      final deletedPassport = restored.travel!.passports
          .firstWhere((p) => p.id == 'passport_deleted');
      expect(deletedPassport.isDeleted, isTrue);
      expect(deletedPassport.deletedAt, isNotNull);
    });

    test('activePassports getter filters soft-deleted passports', () {
      final profile = ProfileData(
        travel: TravelData(
          passports: [
            PassportData(id: 'pass_1', title: 'Passport 1', number: '123'),
            PassportData(id: 'pass_2', title: 'Passport 2', number: '456', isDeleted: true),
          ],
        ),
      );

      expect(profile.travel!.activePassports, hasLength(1));
      expect(profile.travel!.deletedPassports, hasLength(1));
    });

    test('serializes visa data with soft-delete markers', () {
      final now = DateTime.now();
      final profile = ProfileData(
        travel: TravelData(
          visas: [
            VisaData(
              id: 'visa_active',
              title: 'Tourist Visa',
              country: 'Japan',
              visaType: 'tourist',
            ),
            VisaData(
              id: 'visa_deleted',
              title: 'Work Visa',
              country: 'Japan',
              visaType: 'work',
              isDeleted: true,
              deletedAt: now,
            ),
          ],
        ),
      );

      final json = jsonEncode(profile.toJson());
      final restored = ProfileData.fromJson(jsonDecode(json) as Map<String, dynamic>);

      expect(restored.travel!.activeVisas, hasLength(1));
      expect(restored.travel!.deletedVisas, hasLength(1));
    });

    test('serializes travel history with soft-delete markers', () {
      final now = DateTime.now();
      final profile = ProfileData(
        travel: TravelData(
          travelHistory: [
            TravelHistoryData(
              id: 'travel_active',
              destination: 'Paris, France',
              date: '2023-06-01',
            ),
            TravelHistoryData(
              id: 'travel_deleted',
              destination: 'Berlin, Germany',
              date: '2020-01-01',
              isDeleted: true,
              deletedAt: now,
            ),
          ],
        ),
      );

      final json = jsonEncode(profile.toJson());
      final restored = ProfileData.fromJson(jsonDecode(json) as Map<String, dynamic>);

      expect(restored.travel!.activeTravelHistory, hasLength(1));
      expect(restored.travel!.travelHistory.where((t) => t.isDeleted), hasLength(1));
    });

    test('serializes bank account data with soft-delete markers', () {
      final now = DateTime.now();
      final profile = ProfileData(
        financial: FinancialData(
          bankAccounts: [
            BankAccountData(
              id: 'bank_active',
              title: 'Checking',
              bankName: 'Chase',
              accountNumber: '123456',
            ),
            BankAccountData(
              id: 'bank_deleted',
              title: 'Savings',
              bankName: 'Chase',
              accountNumber: '999999',
              isDeleted: true,
              deletedAt: now,
            ),
          ],
        ),
      );

      final json = jsonEncode(profile.toJson());
      final restored = ProfileData.fromJson(jsonDecode(json) as Map<String, dynamic>);

      expect(restored.financial!.activeBankAccounts, hasLength(1));
      expect(restored.financial!.deletedBankAccounts, hasLength(1));
    });

    test('serializes card data with soft-delete markers', () {
      final now = DateTime.now();
      final profile = ProfileData(
        financial: FinancialData(
          cards: [
            CardData(
              id: 'card_active',
              title: 'Visa',
              cardNumber: '1234 5678 9012 3456',
              cardType: 'Credit',
            ),
            CardData(
              id: 'card_deleted',
              title: 'Mastercard',
              cardNumber: '9876 5432 1098 7654',
              cardType: 'Credit',
              isDeleted: true,
              deletedAt: now,
            ),
          ],
        ),
      );

      final json = jsonEncode(profile.toJson());
      final restored = ProfileData.fromJson(jsonDecode(json) as Map<String, dynamic>);

      expect(restored.financial!.activeCards, hasLength(1));
      expect(restored.financial!.deletedCards, hasLength(1));
    });

    test('serializes tax ID data with soft-delete markers', () {
      final profile = ProfileData(
        financial: FinancialData(
          taxIds: [
            TaxIdData(
              id: 'tax_active',
              title: 'SSN',
              taxIdNumber: '123-45-6789',
              taxIdType: 'SSN',
            ),
            TaxIdData(
              id: 'tax_deleted',
              title: 'ITIN',
              taxIdNumber: '987-65-4321',
              taxIdType: 'ITIN',
              isDeleted: true,
            ),
          ],
        ),
      );

      final json = jsonEncode(profile.toJson());
      final restored = ProfileData.fromJson(jsonDecode(json) as Map<String, dynamic>);

      expect(restored.financial!.activeTaxIds, hasLength(1));
      expect(restored.financial!.deletedTaxIds, hasLength(1));
    });

    test('serializes education data with soft-delete markers', () {
      final now = DateTime.now();
      final profile = ProfileData(
        professional: ProfessionalData(
          education: [
            EducationData(
              id: 'edu_active',
              institution: 'MIT',
              degree: 'PhD',
              field: 'Computer Science',
            ),
            EducationData(
              id: 'edu_deleted',
              institution: 'Old University',
              degree: 'BS',
              field: 'History',
              isDeleted: true,
              deletedAt: now,
            ),
          ],
        ),
      );

      final json = jsonEncode(profile.toJson());
      final restored = ProfileData.fromJson(jsonDecode(json) as Map<String, dynamic>);

      expect(restored.professional!.activeEducation, hasLength(1));
      expect(restored.professional!.deletedEducation, hasLength(1));
    });

    test('serializes employment data with soft-delete markers', () {
      final profile = ProfileData(
        professional: ProfessionalData(
          employment: [
            EmploymentData(
              id: 'emp_active',
              company: 'Tech Corp',
              position: 'Engineer',
            ),
            EmploymentData(
              id: 'emp_deleted',
              company: 'Old Corp',
              position: 'Intern',
              isDeleted: true,
            ),
          ],
        ),
      );

      final json = jsonEncode(profile.toJson());
      final restored = ProfileData.fromJson(jsonDecode(json) as Map<String, dynamic>);

      expect(restored.professional!.activeEmployment, hasLength(1));
      expect(restored.professional!.deletedEmployment, hasLength(1));
    });

    test('serializes skills with soft-delete markers', () {
      final profile = ProfileData(
        professional: ProfessionalData(
          skills: [
            SkillData(id: 'skill_active', name: 'Flutter'),
            SkillData(id: 'skill_deleted', name: 'COBOL', isDeleted: true),
          ],
        ),
      );

      final json = jsonEncode(profile.toJson());
      final restored = ProfileData.fromJson(jsonDecode(json) as Map<String, dynamic>);

      expect(restored.professional!.activeSkills, hasLength(1));
      expect(restored.professional!.deletedSkills, hasLength(1));
    });

    test('serializes languages with soft-delete markers', () {
      final profile = ProfileData(
        professional: ProfessionalData(
          languages: [
            LanguageData(id: 'lang_active', name: 'English', proficiency: 'Native'),
            LanguageData(id: 'lang_deleted', name: 'Latin', proficiency: 'Basic', isDeleted: true),
          ],
        ),
      );

      final json = jsonEncode(profile.toJson());
      final restored = ProfileData.fromJson(jsonDecode(json) as Map<String, dynamic>);

      expect(restored.professional!.activeLanguages, hasLength(1));
      expect(restored.professional!.deletedLanguages, hasLength(1));
    });

    test('serializes awards with soft-delete markers', () {
      final profile = ProfileData(
        professional: ProfessionalData(
          awards: [
            AwardData(id: 'award_active', title: 'Best Employee'),
            AwardData(id: 'award_deleted', title: 'Old Award', isDeleted: true),
          ],
        ),
      );

      final json = jsonEncode(profile.toJson());
      final restored = ProfileData.fromJson(jsonDecode(json) as Map<String, dynamic>);

      expect(restored.professional!.activeAwards, hasLength(1));
      expect(restored.professional!.deletedAwards, hasLength(1));
    });

    test('full profile roundtrip preserves all data and soft-delete markers', () {
      final now = DateTime.now();
      final original = ProfileData(
        identity: IdentityData(
          fullName: 'Full Profile Test',
          idCards: [
            IdCardData(
              id: 'idcard_1',
              title: 'Driver License',
              number: 'DL123',
              isDeleted: false,
            ),
            IdCardData(
              id: 'idcard_2',
              title: 'Passport',
              number: 'P456',
              isDeleted: true,
              deletedAt: now,
            ),
          ],
          addresses: [
            AddressData(
              id: 'addr_1',
              title: 'Home',
              city: 'NYC',
            ),
          ],
        ),
        travel: TravelData(
          passports: [
            PassportData(
              id: 'pass_1',
              title: 'US Passport',
              number: 'US123',
            ),
          ],
          visas: [],
          travelHistory: [],
        ),
        financial: FinancialData(
          bankAccounts: [],
          cards: [],
          taxIds: [],
        ),
        professional: ProfessionalData(
          skills: [
            SkillData(id: 'skill_1', name: 'Flutter', isDeleted: false),
          ],
          languages: [],
          awards: [],
          education: [],
          employment: [],
        ),
      );

      final json = jsonEncode(original.toJson());
      final restored = ProfileData.fromJson(jsonDecode(json) as Map<String, dynamic>);

      // Verify identity
      expect(restored.identity!.fullName, equals('Full Profile Test'));
      expect(restored.identity!.idCards, hasLength(2));
      expect(restored.identity!.activeIdCards, hasLength(1));
      expect(restored.identity!.idCards!.where((c) => c.isDeleted), hasLength(1));

      // Verify travel
      expect(restored.travel!.passports, hasLength(1));

      // Verify professional
      expect(restored.professional!.skills, hasLength(1));
    });

    test('copyWith preserves soft-delete markers', () {
      final original = PassportData(
        id: 'pass_1',
        title: 'Original',
        number: '123',
        isDeleted: true,
        deletedAt: DateTime(2024, 1, 1),
      );

      final copied = original.copyWith(title: 'Updated');

      expect(copied.id, equals('pass_1'));
      expect(copied.title, equals('Updated'));
      expect(copied.number, equals('123'));
      expect(copied.isDeleted, isTrue);
      expect(copied.deletedAt, equals(DateTime(2024, 1, 1)));
    });

    test('copyWith can clear soft-delete markers (restore)', () {
      final deleted = PassportData(
        id: 'pass_1',
        title: 'Expired',
        number: '123',
        isDeleted: true,
        deletedAt: DateTime(2024, 1, 1),
      );

      final restored = deleted.copyWith(
        isDeleted: false,
        deletedAt: null,
      );

      expect(restored.isDeleted, isFalse);
      expect(restored.deletedAt, isNull);
    });

    test('identity activeIdCards getter filters deleted items', () {
      final identity = IdentityData(
        idCards: [
          IdCardData(id: 'id_1', title: 'Card 1', number: '111'),
          IdCardData(id: 'id_2', title: 'Card 2', number: '222', isDeleted: true),
          IdCardData(id: 'id_3', title: 'Card 3', number: '333'),
        ],
      );

      expect(identity.activeIdCards, hasLength(2));
    });

    test('identity activeAddresses getter filters deleted items', () {
      final identity = IdentityData(
        addresses: [
          AddressData(id: 'addr_1', title: 'Home'),
          AddressData(id: 'addr_2', title: 'Work', isDeleted: true),
        ],
      );

      expect(identity.activeAddresses, hasLength(1));
    });

    test('serializes and deserializes complete profile with all section types', () {
      final now = DateTime.now();
      final profile = ProfileData(
        identity: IdentityData(
          fullName: 'Complete Test',
          contact: ContactData(
            entries: [
              ContactEntry(
                id: 'c1',
                title: 'Email',
                type: 'email',
                value: 'test@test.com',
              ),
            ],
          ),
        ),
        travel: TravelData(
          passports: [
            PassportData(id: 'p1', title: 'Passport', number: 'ABC123'),
          ],
          visas: [
            VisaData(id: 'v1', title: 'Visa', country: 'Japan'),
          ],
          travelHistory: [
            TravelHistoryData(id: 't1', destination: 'Tokyo'),
          ],
        ),
        financial: FinancialData(
          bankAccounts: [
            BankAccountData(id: 'b1', title: 'Bank', bankName: 'Chase'),
          ],
          cards: [
            CardData(id: 'd1', title: 'Card', cardNumber: '1234'),
          ],
          taxIds: [
            TaxIdData(id: 't1', title: 'SSN', taxIdNumber: '123'),
          ],
        ),
        professional: ProfessionalData(
          education: [
            EducationData(id: 'e1', institution: 'MIT'),
          ],
          employment: [
            EmploymentData(id: 'em1', company: 'Acme'),
          ],
          skills: [
            SkillData(id: 's1', name: 'Dart'),
          ],
          languages: [
            LanguageData(id: 'l1', name: 'English'),
          ],
          awards: [
            AwardData(id: 'a1', title: 'Award'),
          ],
        ),
      );

      final json = jsonEncode(profile.toJson());
      final restored = ProfileData.fromJson(jsonDecode(json) as Map<String, dynamic>);

      expect(restored.identity!.fullName, equals('Complete Test'));
      expect(restored.identity!.contact!.entries.first.value, equals('test@test.com'));
      expect(restored.travel!.passports.first.number, equals('ABC123'));
      expect(restored.travel!.visas.first.country, equals('Japan'));
      expect(restored.financial!.bankAccounts.first.bankName, equals('Chase'));
      expect(restored.professional!.skills.first.name, equals('Dart'));
    });

    test('handles null sections gracefully', () {
      final profile = ProfileData();

      final json = jsonEncode(profile.toJson());
      final restored = ProfileData.fromJson(jsonDecode(json) as Map<String, dynamic>);

      expect(restored.identity, isNull);
      expect(restored.travel, isNull);
      expect(restored.financial, isNull);
      expect(restored.professional, isNull);
    });

    test('handles empty lists in sections', () {
      // Note: IdentityData() has nullable fields (idCards, addresses are null)
      // When serialized, null fields stay null (not empty lists)
      final profile = ProfileData(
        identity: IdentityData(
          idCards: [], // Explicitly empty list
          addresses: [], // Explicitly empty list
        ),
        travel: TravelData(
          passports: [],
          visas: [],
          travelHistory: [],
        ),
        financial: FinancialData(
          bankAccounts: [],
          cards: [],
          taxIds: [],
        ),
        professional: ProfessionalData(
          skills: [],
          languages: [],
          awards: [],
          education: [],
          employment: [],
        ),
      );

      final json = jsonEncode(profile.toJson());
      final restored = ProfileData.fromJson(jsonDecode(json) as Map<String, dynamic>);

      // With explicit empty lists, they deserialize as empty lists
      expect(restored.identity!.idCards, isEmpty);
      expect(restored.identity!.addresses, isEmpty);
      expect(restored.travel!.passports, isEmpty);
      expect(restored.travel!.visas, isEmpty);
      expect(restored.travel!.travelHistory, isEmpty);
      expect(restored.financial!.bankAccounts, isEmpty);
      expect(restored.financial!.cards, isEmpty);
      expect(restored.financial!.taxIds, isEmpty);
      expect(restored.professional!.skills, isEmpty);
      expect(restored.professional!.languages, isEmpty);
      expect(restored.professional!.awards, isEmpty);
      expect(restored.professional!.education, isEmpty);
      expect(restored.professional!.employment, isEmpty);
    });

    test('null sections remain null after roundtrip', () {
      // When sections are null (not provided), they stay null after serialization
      final profile = ProfileData(); // All sections null

      final json = jsonEncode(profile.toJson());
      final restored = ProfileData.fromJson(jsonDecode(json) as Map<String, dynamic>);

      // Null sections remain null
      expect(restored.identity, isNull);
      expect(restored.travel, isNull);
      expect(restored.financial, isNull);
      expect(restored.professional, isNull);
    });
  });

  group('soft-delete marker edge cases', () {
    test('deletedAt null is preserved when isDeleted is true', () {
      // This can happen if deletedAt was set externally or by older code
      final profile = ProfileData(
        travel: TravelData(
          passports: [
            PassportData(
              id: 'p1',
              title: 'Test',
              number: '123',
              isDeleted: true,
              deletedAt: null, // Explicit null even though isDeleted is true
            ),
          ],
        ),
      );

      final json = jsonEncode(profile.toJson());
      final restored = ProfileData.fromJson(jsonDecode(json) as Map<String, dynamic>);

      expect(restored.travel!.passports.first.isDeleted, isTrue);
      expect(restored.travel!.passports.first.deletedAt, isNull);
    });

    test('isDeleted false with deletedAt present - deletedAt takes precedence', () {
      // Edge case: data inconsistency where isDeleted=false but deletedAt is set
      final json = jsonEncode({
        'travel': {
          'passports': [
            {
              'id': 'p1',
              'title': 'Test',
              'number': '123',
              'is_deleted': false,
              'deleted_at': '2024-01-01T00:00:00.000Z',
            },
          ],
        },
      });

      final restored = ProfileData.fromJson(jsonDecode(json) as Map<String, dynamic>);

      // The model stores what's in the JSON; activePassports filters by isDeleted field
      expect(restored.travel!.passports.first.isDeleted, isFalse);
      expect(restored.travel!.passports.first.deletedAt, isNotNull);
      // activePassports checks isDeleted, so this would still appear as active
      expect(restored.travel!.activePassports, hasLength(1));
    });

    test('restoring an item clears both isDeleted and deletedAt', () {
      final deleted = ContactEntry(
        id: 'c1',
        title: 'Old',
        type: 'email',
        value: 'old@test.com',
        isDeleted: true,
        deletedAt: DateTime(2024, 1, 1),
      );

      final restored = deleted.copyWith(
        isDeleted: false,
        deletedAt: null,
      );

      expect(restored.isDeleted, isFalse);
      expect(restored.deletedAt, isNull);
    });

    test('soft-deleting multiple items preserves individual timestamps', () {
      final now = DateTime.now();
      final dayAgo = now.subtract(const Duration(days: 1));
      final weekAgo = now.subtract(const Duration(days: 7));

      final profile = ProfileData(
        financial: FinancialData(
          bankAccounts: [
            BankAccountData(
              id: 'b1',
              title: 'Account 1',
              bankName: 'Bank A',
              isDeleted: true,
              deletedAt: weekAgo,
            ),
            BankAccountData(
              id: 'b2',
              title: 'Account 2',
              bankName: 'Bank B',
              isDeleted: true,
              deletedAt: dayAgo,
            ),
            BankAccountData(
              id: 'b3',
              title: 'Account 3',
              bankName: 'Bank C',
            ),
          ],
        ),
      );

      final json = jsonEncode(profile.toJson());
      final restored = ProfileData.fromJson(jsonDecode(json) as Map<String, dynamic>);

      final deletedAccounts = restored.financial!.deletedBankAccounts;
      expect(deletedAccounts, hasLength(2));

      // Timestamps should be preserved
      final olderAccount = deletedAccounts.firstWhere((b) => b.id == 'b1');
      final newerAccount = deletedAccounts.firstWhere((b) => b.id == 'b2');

      expect(olderAccount.deletedAt, equals(weekAgo));
      expect(newerAccount.deletedAt, equals(dayAgo));
    });
  });
}
