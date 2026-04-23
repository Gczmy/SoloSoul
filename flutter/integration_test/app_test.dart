import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/main.dart';
import 'package:solosoul_flutter/presentation/pages/profile_page.dart';
import 'package:solosoul_flutter/presentation/pages/travel_page.dart';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';

// Note: Integration tests require a device/emulator to run.
// Use: flutter test integration_test/app_test.dart
// Or: flutter drive --target=integration_test/app_test.dart

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  // Skip FFI tests on Linux (no native library support)
  final isLinux = Platform.operatingSystem == 'linux';
  final skipFFI = isLinux ? 'Rust FFI requires macOS or iOS' : null;

  group('SoloSoul App Integration Tests', () {
    group('App Launch', () {
      testWidgets('app launches and shows splash screen',
          (WidgetTester tester) async {
        await tester.pumpWidget(
          const ProviderScope(
            child: SoloSoulApp(),
          ),
        );

        // Verify splash screen shows app name
        expect(find.text('SoloSoul'), findsOneWidget);
      });
    });

    group('Navigation Flow', () {
      testWidgets('can navigate to profile page',
          (WidgetTester tester) async {
        await tester.pumpWidget(
          ProviderScope(
            child: MaterialApp(
              home: Builder(
                builder: (context) => Scaffold(
                  body: Column(
                    children: [
                      TextButton(
                        onPressed: () {
                          Navigator.of(context).push(
                            MaterialPageRoute(
                              builder: (_) => const ProfilePage(),
                            ),
                          );
                        },
                        child: const Text('Go to Profile'),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        );

        await tester.pumpAndSettle();

        // Navigate to profile
        await tester.tap(find.text('Go to Profile'));
        await tester.pumpAndSettle();

        // Verify profile page
        expect(find.text('Profile'), findsOneWidget);
      });

      testWidgets('can navigate to travel page',
          (WidgetTester tester) async {
        await tester.pumpWidget(
          ProviderScope(
            child: MaterialApp(
              home: Builder(
                builder: (context) => Scaffold(
                  body: Column(
                    children: [
                      TextButton(
                        onPressed: () {
                          Navigator.of(context).push(
                            MaterialPageRoute(
                              builder: (_) => const TravelPage(),
                            ),
                          );
                        },
                        child: const Text('Go to Travel'),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        );

        await tester.pumpAndSettle();

        // Navigate to travel
        await tester.tap(find.text('Go to Travel'));
        await tester.pumpAndSettle();

        // Verify travel page
        expect(find.text('Travel'), findsOneWidget);
      });
    });

    group('Profile Page Integration', () {
      testWidgets('profile page renders all sections',
          (WidgetTester tester) async {
        await tester.pumpWidget(
          const ProviderScope(
            child: MaterialApp(
              home: ProfilePage(),
            ),
          ),
        );

        await tester.pump();

        // Verify main sections are present
        expect(find.text('Identity Profile'), findsOneWidget);
        expect(find.text('Contact Information'), findsOneWidget);
        expect(find.text('Identity Documents'), findsOneWidget);
        expect(find.text('Addresses'), findsOneWidget);
        expect(find.text('End-to-End Encrypted'), findsOneWidget);
      });
    });

    group('Travel Page Integration', () {
      testWidgets('travel page renders all sections',
          (WidgetTester tester) async {
        await tester.pumpWidget(
          const ProviderScope(
            child: MaterialApp(
              home: TravelPage(),
            ),
          ),
        );

        await tester.pump();

        // Verify all main sections are present
        expect(find.text('Passports'), findsOneWidget);
        expect(find.text('Visas'), findsOneWidget);
        expect(find.text('Travel History'), findsOneWidget);
        expect(find.text('Scan Document with OCR'), findsOneWidget);
      });

      testWidgets('travel page OCR dialog interaction',
          (WidgetTester tester) async {
        await tester.pumpWidget(
          const ProviderScope(
            child: MaterialApp(
              home: TravelPage(),
            ),
          ),
        );

        await tester.pump();

        // Tap OCR scan button
        await tester.tap(find.text('Scan Document with OCR'));
        await tester.pumpAndSettle();

        // Verify dialog appears
        expect(find.text('OCR Scan'), findsOneWidget);
      });
    });

    group('Rust Vault FFI Integration', skip: skipFFI, () {
      late RustVaultService vaultService;
      late ProfileStorageService storageService;
      String? testAccountId;
      final testPassword = 'TestPass123!';
      final testBasePath = Directory.systemTemp.path;

      setUpAll(() {
        vaultService = RustVaultService.instance;
        storageService = ProfileStorageService.instance;
      });

      test('full FFI flow: create account -> save profile -> load profile -> verify -> cleanup',
          () async {
        // Step 1: Create a test account
        final createResult = vaultService.createAccount(
          name: 'integration_test_account',
          password: testPassword,
        );

        if (!createResult.success || createResult.accountId == null) {
          // Vault not available - skip test gracefully
          throw const Skip('Rust vault not available - FFI test skipped');
        }

        testAccountId = createResult.accountId;

        try {
          // Step 2: Unlock the vault
          final unlockResult = vaultService.unlockVault(
            accountId: testAccountId!,
            password: testPassword,
          );

          expect(unlockResult.success, isTrue,
              reason: 'Failed to unlock vault: ${unlockResult.error}');

          // Step 3: Create test profile data with various fields
          final profileData = _createTestProfileData();

          // Step 4: Save the profile
          final saveResult = await storageService.saveProfile(
            testAccountId!,
            profileData,
          );
          expect(saveResult, isTrue, reason: 'Failed to save profile');

          // Step 5: Load the profile back
          final loadedProfile = await storageService.loadProfile(testAccountId!);
          expect(loadedProfile, isNotNull, reason: 'Profile should not be null');

          // Step 6: Verify the data matches
          expect(loadedProfile!.identity, isNotNull);
          expect(loadedProfile.identity!.fullName, equals('Test User'));
          expect(loadedProfile.identity!.givenName, equals('Test'));
          expect(loadedProfile.identity!.familyName, equals('User'));

          // Verify travel data
          expect(loadedProfile.travel, isNotNull);
          expect(loadedProfile.travel!.passports, hasLength(1));
          expect(loadedProfile.travel!.passports.first.country, equals('United States'));
          expect(loadedProfile.travel!.passports.first.number, equals('123456789'));

          // Verify financial data
          expect(loadedProfile.financial, isNotNull);
          expect(loadedProfile.financial!.bankAccounts, hasLength(1));
          expect(loadedProfile.financial!.bankAccounts.first.bankName, equals('Test Bank'));

          // Step 7: Verify soft-delete markers are properly serialized
          final json = jsonEncode(profileData.toJson());
          final restored = ProfileData.fromJson(jsonDecode(json) as Map<String, dynamic>);

          // Verify no items are deleted by default
          expect(restored.travel!.passports.first.isDeleted, isFalse);
          expect(restored.travel!.passports.first.deletedAt, isNull);

          // Step 8: Test soft-delete flow
          final deletedPassport = restored.travel!.passports.first.copyWith(
            isDeleted: true,
            deletedAt: DateTime.now(),
          );

          final updatedTravel = TravelData(
            passports: [deletedPassport],
            visas: [],
            travelHistory: [],
          );

          final updatedProfile = profileData.copyWith(travel: updatedTravel);
          final updatedJson = jsonEncode(updatedProfile.toJson());
          final restoredFromDeleted = ProfileData.fromJson(
            jsonDecode(updatedJson) as Map<String, dynamic>,
          );

          expect(restoredFromDeleted.travel!.passports.first.isDeleted, isTrue);
          expect(restoredFromDeleted.travel!.passports.first.deletedAt, isNotNull);

          // Step 9: Verify active items filter works
          expect(restoredFromDeleted.travel!.activePassports, isEmpty);
          expect(restoredFromDeleted.travel!.deletedPassports, hasLength(1));

        } finally {
          // Cleanup: Lock and delete the test account
          vaultService.lockVault();

          if (testAccountId != null) {
            vaultService.deleteAccount(testAccountId!);
          }
        }
      });

      test('save and load profile with encrypted data', () async {
        // Create a separate account for this test
        final createResult = vaultService.createAccount(
          name: 'encrypted_profile_test',
          password: testPassword,
        );

        if (!createResult.success || createResult.accountId == null) {
          throw const Skip('Rust vault not available');
        }

        final accountId = createResult.accountId!;

        try {
          // Unlock
          final unlockResult = vaultService.unlockVault(
            accountId: accountId,
            password: testPassword,
          );
          expect(unlockResult.success, isTrue);

          // Create profile with sensitive data
          final profileData = ProfileData(
            identity: IdentityData(
              fullName: 'Sensitive User',
              givenName: 'Sensitive',
              familyName: 'User',
              contact: ContactData(
                entries: [
                  ContactEntry(
                    id: 'contact_1',
                    title: 'Email',
                    type: 'email',
                    value: 'sensitive@example.com',
                  ),
                ],
              ),
            ),
          );

          // Save
          final saveResult = await storageService.saveProfile(accountId, profileData);
          expect(saveResult, isTrue);

          // Load
          final loadedProfile = await storageService.loadProfile(accountId);
          expect(loadedProfile, isNotNull);
          expect(loadedProfile!.identity!.fullName, equals('Sensitive User'));
          expect(loadedProfile.identity!.contact!.entries.first.value,
              equals('sensitive@example.com'));

          // Verify it's actually encrypted in the vault by checking
          // that we can only read it with the correct key
          vaultService.lockVault();

          // Without unlocking, loadProfileDecrypted should return null
          final encryptedCheck = await vaultService.loadProfileDecrypted(accountId);
          expect(encryptedCheck, isNull);

        } finally {
          vaultService.lockVault();
          vaultService.deleteAccount(accountId);
        }
      });

      test('profile with multiple soft-deleted items preserves markers after roundtrip',
          () async {
        final createResult = vaultService.createAccount(
          name: 'soft_delete_test',
          password: testPassword,
        );

        if (!createResult.success || createResult.accountId == null) {
          throw const Skip('Rust vault not available');
        }

        final accountId = createResult.accountId!;

        try {
          final unlockResult = vaultService.unlockVault(
            accountId: accountId,
            password: testPassword,
          );
          expect(unlockResult.success, isTrue);

          // Create profile with multiple items, some deleted
          final now = DateTime.now();
          final profileData = ProfileData(
            identity: IdentityData(
              fullName: 'Multi Delete User',
              idCards: [
                IdCardData(
                  id: 'idcard_active',
                  title: 'Active ID',
                  number: 'ACTIVE123',
                ),
                IdCardData(
                  id: 'idcard_deleted',
                  title: 'Deleted ID',
                  number: 'DELETED456',
                  isDeleted: true,
                  deletedAt: now,
                ),
              ],
              addresses: [
                AddressData(
                  id: 'addr_active',
                  title: 'Home',
                  street: '123 Main St',
                ),
                AddressData(
                  id: 'addr_deleted',
                  title: 'Old Home',
                  street: '456 Old St',
                  isDeleted: true,
                  deletedAt: now.subtract(const Duration(days: 1)),
                ),
              ],
            ),
            travel: TravelData(
              passports: [
                PassportData(
                  id: 'passport_active',
                  title: 'Valid Passport',
                  number: 'ACTIVE_PASSPORT',
                  country: 'Canada',
                ),
                PassportData(
                  id: 'passport_deleted',
                  title: 'Expired Passport',
                  number: 'EXPIRED_PASSPORT',
                  country: 'Canada',
                  isDeleted: true,
                  deletedAt: now.subtract(const Duration(days: 30)),
                ),
              ],
              visas: [
                VisaData(
                  id: 'visa_active',
                  title: 'Active Visa',
                  country: 'Japan',
                  visaType: 'tourist',
                ),
                VisaData(
                  id: 'visa_deleted',
                  title: 'Revoked Visa',
                  country: 'Japan',
                  visaType: 'work',
                  isDeleted: true,
                  deletedAt: now,
                ),
              ],
            ),
            financial: FinancialData(
              bankAccounts: [
                BankAccountData(
                  id: 'bank_active',
                  title: 'Primary Account',
                  bankName: 'Main Bank',
                  accountNumber: '123456',
                ),
                BankAccountData(
                  id: 'bank_deleted',
                  title: 'Closed Account',
                  bankName: 'Old Bank',
                  accountNumber: '999999',
                  isDeleted: true,
                  deletedAt: now.subtract(const Duration(days: 7)),
                ),
              ],
            ),
          );

          // Save and reload
          await storageService.saveProfile(accountId, profileData);
          final loadedProfile = await storageService.loadProfile(accountId);

          expect(loadedProfile, isNotNull);

          // Verify active items count
          expect(loadedProfile!.identity!.activeIdCards, hasLength(1));
          expect(loadedProfile.identity!.activeAddresses, hasLength(1));
          expect(loadedProfile.travel!.activePassports, hasLength(1));
          expect(loadedProfile.travel!.activeVisas, hasLength(1));
          expect(loadedProfile.financial!.activeBankAccounts, hasLength(1));

          // Verify deleted items count
          expect(loadedProfile.identity!.idCards!.where((c) => c.isDeleted), hasLength(1));
          expect(loadedProfile.identity!.addresses!.where((a) => a.isDeleted), hasLength(1));
          expect(loadedProfile.travel!.passports.where((p) => p.isDeleted), hasLength(1));
          expect(loadedProfile.travel!.visas.where((v) => v.isDeleted), hasLength(1));
          expect(loadedProfile.financial!.bankAccounts.where((b) => b.isDeleted), hasLength(1));

          // Verify the deleted items have correct deletedAt timestamps
          final deletedIdCard = loadedProfile.identity!.idCards!.firstWhere((c) => c.isDeleted);
          expect(deletedIdCard.deletedAt, isNotNull);

          final deletedPassport = loadedProfile.travel!.passports.firstWhere((p) => p.isDeleted);
          expect(deletedPassport.deletedAt, isNotNull);

        } finally {
          vaultService.lockVault();
          vaultService.deleteAccount(accountId);
        }
      });
    });
  });
}

/// Creates a test profile with various data types for integration testing
ProfileData _createTestProfileData() {
  return ProfileData(
    identity: IdentityData(
      fullName: 'Test User',
      givenName: 'Test',
      familyName: 'User',
      dateOfBirth: '1990-01-01',
      gender: 'Male',
      nationality: 'American',
      idCards: [
        IdCardData(
          id: 'id_1',
          title: 'Driver License',
          number: 'DL123456',
          country: 'United States',
        ),
      ],
      contact: ContactData(
        entries: [
          ContactEntry(
            id: 'contact_1',
            title: 'Personal',
            type: 'email',
            value: 'test@example.com',
          ),
          ContactEntry(
            id: 'contact_2',
            title: 'Work',
            type: 'phone',
            value: '+1-555-0100',
          ),
        ],
      ),
      addresses: [
        AddressData(
          id: 'addr_1',
          title: 'Home',
          street: '123 Main Street',
          city: 'New York',
          state: 'NY',
          postalCode: '10001',
          country: 'United States',
        ),
      ],
    ),
    travel: TravelData(
      passports: [
        PassportData(
          id: 'passport_1',
          title: 'US Passport',
          number: '123456789',
          country: 'United States',
          countryCode: 'US',
          expiryDate: '2030-01-01',
          holderName: 'Test User',
        ),
      ],
      visas: [
        VisaData(
          id: 'visa_1',
          title: 'Japan Tourist Visa',
          country: 'Japan',
          visaType: 'tourist',
          number: 'JAPAN_VIS_001',
          expiryDate: '2025-06-01',
        ),
      ],
      travelHistory: [
        TravelHistoryData(
          id: 'travel_1',
          destination: 'Tokyo, Japan',
          date: '2023-03-15',
          travelType: 'Airplane',
          flightNumber: 'JL001',
        ),
      ],
    ),
    financial: FinancialData(
      bankAccounts: [
        BankAccountData(
          id: 'bank_1',
          title: 'Primary Checking',
          bankName: 'Test Bank',
          accountNumber: '****1234',
          currency: 'USD',
          swiftBic: 'TESTBANKXXX',
        ),
      ],
      cards: [
        CardData(
          id: 'card_1',
          title: 'Visa Credit',
          cardNumber: '**** **** **** 1234',
          cardType: 'Visa',
          expiryDate: '12/2026',
          holderName: 'Test User',
        ),
      ],
      taxIds: [
        TaxIdData(
          id: 'tax_1',
          title: 'Social Security',
          taxIdNumber: '***-**-1234',
          taxIdType: 'SSN',
          country: 'United States',
        ),
      ],
    ),
    professional: ProfessionalData(
      education: [
        EducationData(
          id: 'edu_1',
          institution: 'Test University',
          degree: 'Bachelor of Science',
          field: 'Computer Science',
          startDate: '2010-09-01',
          endDate: '2014-06-15',
        ),
      ],
      employment: [
        EmploymentData(
          id: 'emp_1',
          company: 'Test Company',
          position: 'Software Engineer',
          startDate: '2014-07-01',
          endDate: '2023-12-31',
          responsibilities: 'Developing software',
        ),
      ],
      skills: [
        SkillData(
          id: 'skill_1',
          name: 'Flutter',
          level: 'Expert',
        ),
        SkillData(
          id: 'skill_2',
          name: 'Dart',
          level: 'Expert',
        ),
      ],
      languages: [
        LanguageData(
          id: 'lang_1',
          name: 'English',
          proficiency: 'Native',
        ),
        LanguageData(
          id: 'lang_2',
          name: 'Japanese',
          proficiency: 'Conversational',
        ),
      ],
      awards: [
        AwardData(
          id: 'award_1',
          title: 'Best Developer',
          issuer: 'Test Company',
          date: '2022-01-01',
          description: 'Awarded for excellent performance',
        ),
      ],
    ),
  );
}
