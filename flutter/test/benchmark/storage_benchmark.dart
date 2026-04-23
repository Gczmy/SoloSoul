import 'dart:convert';

import 'package:solosoul_flutter/core/services/profile_storage_service.dart';

void main() {
  _runAllStorageBenchmarks();
}

void _runAllStorageBenchmarks() {
  print('=' * 60);
  print('STORAGE BENCHMARKS');
  print('=' * 60);

  _benchmarkProfileDataSerialization();
  _benchmarkProfileDataDeserialization();
}

void _benchmarkProfileDataSerialization() {
  print('\n--- ProfileData JSON Serialization ---');

  final profile = _createSampleProfile();
  const runs = 100;
  final results = <int>[];

  // Warm-up
  jsonEncode(profile.toJson());

  for (var i = 0; i < runs; i++) {
    final sw = Stopwatch()..start();
    jsonEncode(profile.toJson());
    sw.stop();
    results.add(sw.elapsedMicroseconds);
  }

  final avg = results.reduce((a, b) => a + b) / runs;
  final min = results.reduce((a, b) => a < b ? a : b);
  final max = results.reduce((a, b) => a > b ? a : b);

  print('Runs: $runs');
  print('Avg: ${avg.toStringAsFixed(2)} us');
  print('Min: ${min.toStringAsFixed(2)} us');
  print('Max: ${max.toStringAsFixed(2)} us');
}

void _benchmarkProfileDataDeserialization() {
  print('\n--- ProfileData JSON Deserialization ---');

  final profile = _createSampleProfile();
  final jsonString = jsonEncode(profile.toJson());
  final jsonMap = jsonDecode(jsonString) as Map<String, dynamic>;

  const runs = 100;
  final results = <int>[];

  // Warm-up
  ProfileData.fromJson(jsonMap);

  for (var i = 0; i < runs; i++) {
    final sw = Stopwatch()..start();
    ProfileData.fromJson(jsonMap);
    sw.stop();
    results.add(sw.elapsedMicroseconds);
  }

  final avg = results.reduce((a, b) => a + b) / runs;
  final min = results.reduce((a, b) => a < b ? a : b);
  final max = results.reduce((a, b) => a > b ? a : b);

  print('Runs: $runs');
  print('Avg: ${avg.toStringAsFixed(2)} us');
  print('Min: ${min.toStringAsFixed(2)} us');
  print('Max: ${max.toStringAsFixed(2)} us');
}

ProfileData _createSampleProfile() {
  return ProfileData(
    identity: IdentityData(
      fullName: 'John Doe',
      givenName: 'John',
      familyName: 'Doe',
      dateOfBirth: '1990-01-15',
      gender: 'male',
      nationality: 'US',
      idCards: [
        IdCardData(
          id: 'id1',
          title: 'Driver License',
          number: 'DL123456',
          country: 'US',
        ),
      ],
      contact: ContactData(
        entries: [
          ContactEntry(
            id: 'c1',
            title: 'Personal',
            type: 'email',
            value: 'john@example.com',
          ),
          ContactEntry(
            id: 'c2',
            title: 'Work',
            type: 'phone',
            value: '+1-555-0100',
          ),
        ],
      ),
      addresses: [
        AddressData(
          id: 'a1',
          title: 'Home',
          street: '123 Main St',
          city: 'New York',
          state: 'NY',
          postalCode: '10001',
          country: 'US',
        ),
      ],
    ),
    travel: TravelData(
      passports: [
        PassportData(
          id: 'p1',
          title: 'US Passport',
          number: '123456789',
          country: 'United States',
          countryCode: 'US',
          expiryDate: '2030-01-1',
        ),
      ],
      visas: [
        VisaData(
          id: 'v1',
          title: 'Schengen',
          country: 'EU',
          visaType: 'tourist',
          number: 'VX123456',
        ),
      ],
      travelHistory: [
        TravelHistoryData(
          id: 't1',
          destination: 'Paris, France',
          date: '2023-06-15',
          travelType: 'Airplane',
          flightNumber: 'AF123',
        ),
      ],
    ),
    financial: FinancialData(
      bankAccounts: [
        BankAccountData(
          id: 'b1',
          title: 'Primary Checking',
          bankName: 'Chase',
          accountNumber: '****1234',
          currency: 'USD',
          swiftBic: 'CHASUS33',
        ),
      ],
      cards: [
        CardData(
          id: 'cd1',
          title: 'Visa Platinum',
          cardNumber: '**** 1234',
          cardType: 'credit',
          expiryDate: '12/2026',
        ),
      ],
      taxIds: [
        TaxIdData(
          id: 't1',
          title: 'SSN',
          taxIdNumber: '***-**-1234',
          taxIdType: 'SSN',
          country: 'US',
        ),
      ],
    ),
    professional: ProfessionalData(
      education: [
        EducationData(
          id: 'e1',
          institution: 'MIT',
          degree: 'Bachelor',
          field: 'Computer Science',
          startDate: '2010-09',
          endDate: '2014-06',
        ),
      ],
      employment: [
        EmploymentData(
          id: 'emp1',
          company: 'Tech Corp',
          position: 'Senior Engineer',
          startDate: '2014-07',
        ),
      ],
      skills: [
        SkillData(id: 's1', name: 'Dart', level: 'Expert'),
        SkillData(id: 's2', name: 'Flutter', level: 'Expert'),
        SkillData(id: 's3', name: 'Rust', level: 'Advanced'),
      ],
      languages: [
        LanguageData(id: 'l1', name: 'English', proficiency: 'Native'),
        LanguageData(id: 'l2', name: 'Spanish', proficiency: 'Fluent'),
      ],
      awards: [
        AwardData(
          id: 'aw1',
          title: 'Best Paper Award',
          issuer: 'IEEE',
          date: '2020',
        ),
      ],
    ),
  );
}
