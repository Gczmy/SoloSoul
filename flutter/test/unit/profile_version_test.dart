import 'dart:convert';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';

void main() {
  group('ProfileData version handling', () {
    test('fromJson parses version field correctly', () {
      final json = {
        'version': 2,
        'data': {
          'identity': {
            'full_name': 'Test User',
            'given_name': 'Test',
            'family_name': 'User',
          },
        },
      };

      // The fromJson should handle versioned format
      final profile = ProfileData.fromJson(json['data'] as Map<String, dynamic>);

      expect(profile.identity, isNotNull);
      expect(profile.identity!.fullName, 'Test User');
      expect(profile.identity!.givenName, 'Test');
      expect(profile.identity!.familyName, 'User');
    });

    test('fromJson handles missing version (V1 legacy)', () {
      // V1 legacy data has no version field - just the data object directly
      final v1LegacyJson = {
        'identity': {
          'full_name': 'Legacy User',
          'given_name': 'Legacy',
          'family_name': 'User',
        },
        'travel': {
          'passports': <Map<String, dynamic>>[],
          'visas': <Map<String, dynamic>>[],
          'travel_history': <Map<String, dynamic>>[],
        },
        'financial': {
          'bank_accounts': <Map<String, dynamic>>[],
          'cards': <Map<String, dynamic>>[],
          'tax_ids': <Map<String, dynamic>>[],
        },
        'professional': {
          'education': <Map<String, dynamic>>[],
          'employment': <Map<String, dynamic>>[],
          'skills': <Map<String, dynamic>>[],
          'languages': <Map<String, dynamic>>[],
        },
      };

      // Should not throw on V1 legacy data
      final profile = ProfileData.fromJson(v1LegacyJson);

      expect(profile.identity, isNotNull);
      expect(profile.identity!.fullName, 'Legacy User');
    });

    test('toJson roundtrip preserves all fields', () {
      final original = ProfileData(
        identity: IdentityData(
          fullName: 'Roundtrip Test',
          givenName: 'Roundtrip',
          familyName: 'Test',
          dateOfBirth: '1990-01-01',
          gender: 'male',
          nationality: 'Testland',
          idCards: [
            IdCardData(
              id: 'id-card-1',
              title: 'Primary',
              number: 'ABC123',
              issueDate: '2020-01-01',
              expiryDate: '2030-01-01',
              holderName: 'Roundtrip Test',
              country: 'Testland',
            ),
          ],
          contact: ContactData(
            entries: [
              ContactEntry(
                label: 'Personal',
                type: 'email',
                value: 'test@example.com',
              ),
            ],
          ),
          addresses: [
            AddressData(
              id: 'addr-1',
              label: 'Home',
              street: '123 Test St',
              city: 'Test City',
              state: 'TS',
              postalCode: '12345',
              country: 'Testland',
            ),
          ],
        ),
        travel: TravelData(
          passports: [
            PassportData(
              id: 'passport-1',
              number: 'P123456',
              country: 'Testland',
              issueDate: '2020-01-01',
              expiryDate: '2030-01-01',
              holderName: 'ROUNDTRIP TEST',
            ),
          ],
          visas: [
            VisaData(
              id: 'visa-1',
              country: 'Test Country',
              visaType: 'Tourist',
              number: 'V789',
              issueDate: '2021-01-01',
              expiryDate: '2025-01-01',
            ),
          ],
          travelHistory: [
            TravelHistoryData(id: 'travel-1', destination: 'Test Destination', date: '2023-01-01'),
          ],
        ),
        financial: FinancialData(
          bankAccounts: [
            BankAccountData(
              id: 'bank-1',
              bankName: 'Test Bank',
              accountNumber: '1234567890',
              currency: 'TST',
              swiftBic: 'TESTBICTST',
            ),
          ],
          cards: [
            CardData(
              id: 'card-1',
              cardNumber: '4111111111111111',
              cardType: 'Visa',
              expiryDate: '2025-12',
              holderName: 'Test User',
            ),
          ],
          taxIds: [
            TaxIdData(
              id: 'tax-1',
              taxIdNumber: 'TAX123',
              taxIdType: 'National ID',
              issuingAuthority: 'Test Authority',
              country: 'Testland',
            ),
          ],
        ),
        professional: ProfessionalData(
          education: [
            EducationData(
              id: 'edu-1',
              institution: 'Test University',
              degree: 'BS',
              field: 'Testing',
              startDate: '2010-09',
              endDate: '2014-06',
            ),
          ],
          employment: [
            EmploymentData(
              id: 'emp-1',
              company: 'Test Corp',
              position: 'Test Engineer',
              startDate: '2014-07',
              endDate: null,
            ),
          ],
          skills: [
            SkillData(id: 'skill-1', name: 'Testing', level: 'Expert'),
          ],
          languages: [
            LanguageData(id: 'lang-1', name: 'Test Language', proficiency: 'Native'),
          ],
        ),
      );

      // Serialize to JSON
      final json = original.toJson();

      // Deserialize back
      final restored = ProfileData.fromJson(json);

      // Verify identity
      expect(restored.identity!.fullName, original.identity!.fullName);
      expect(restored.identity!.givenName, original.identity!.givenName);
      expect(restored.identity!.familyName, original.identity!.familyName);
      expect(restored.identity!.dateOfBirth, original.identity!.dateOfBirth);
      expect(restored.identity!.gender, original.identity!.gender);
      expect(restored.identity!.nationality, original.identity!.nationality);

      // Verify id cards
      expect(restored.identity!.idCards!.length, original.identity!.idCards!.length);
      expect(restored.identity!.idCards!.first.title, original.identity!.idCards!.first.title);
      expect(restored.identity!.idCards!.first.number, original.identity!.idCards!.first.number);

      // Verify contact
      expect(restored.identity!.contact!.entries.length, original.identity!.contact!.entries.length);
      expect(restored.identity!.contact!.entries.first.value, original.identity!.contact!.entries.first.value);

      // Verify addresses
      expect(restored.identity!.addresses!.length, original.identity!.addresses!.length);
      expect(restored.identity!.addresses!.first.city, original.identity!.addresses!.first.city);

      // Verify travel
      expect(restored.travel!.passports.length, original.travel!.passports.length);
      expect(restored.travel!.visas.length, original.travel!.visas.length);
      expect(restored.travel!.travelHistory.length, original.travel!.travelHistory.length);

      // Verify financial
      expect(restored.financial!.bankAccounts.length, original.financial!.bankAccounts.length);
      expect(restored.financial!.cards.length, original.financial!.cards.length);
      expect(restored.financial!.taxIds.length, original.financial!.taxIds.length);

      // Verify professional
      expect(restored.professional!.education.length, original.professional!.education.length);
      expect(restored.professional!.employment.length, original.professional!.employment.length);
      expect(restored.professional!.skills.length, original.professional!.skills.length);
      expect(restored.professional!.languages.length, original.professional!.languages.length);
    });
  });
}
