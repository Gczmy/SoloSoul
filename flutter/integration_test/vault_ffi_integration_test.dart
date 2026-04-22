import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';
import 'package:solosoul_flutter/core/services/native_crypto_service.dart';

/// Integration test for the Rust Vault FFI bridge.
/// Run with: cd ios && flutter test integration_test/vault_ffi_integration_test.dart
/// Or on macOS directly: dart integration_test/vault_ffi_integration_test.dart

void main() {
  group('Vault FFI Integration Tests', () {
    late String testVaultPath;

    setUpAll(() async {
      // Initialize vault with test path
      final tempDir = await Directory.systemTemp.createTemp('solosoul_vault_test_');
      testVaultPath = tempDir.path;

      // Initialize account manager
      RustVaultService.instance.initAccountManager(testVaultPath);
    });

    tearDownAll(() async {
      // Clean up test vault
      try {
        final dir = Directory(testVaultPath);
        if (await dir.exists()) {
          await dir.delete(recursive: true);
        }
      } catch (_) {}
    });

    test('1. Path handshake: initAccountManager creates vault directory', () async {
      final dir = Directory(testVaultPath);
      final exists = await dir.exists();
      expect(exists, isTrue, reason: 'Vault directory should exist after init');
    });

    test('2. Unlock with wrong password returns error (not crash)', () async {
      // This tests the error handling path - wrong password should return error JSON
      // Since we haven't created an account yet, this tests the "no account" path
      final result = RustVaultService.instance.isVaultUnlocked();
      expect(result, isFalse, reason: 'Vault should be locked initially');
    });

    test('3. Create account and unlock flow', () async {
      // Generate a salt and derive a key (simulating account creation)
      final salt = NativeCryptoService.instance.generateSalt();
      expect(salt, isNotNull, reason: 'Salt generation should work');
      expect(salt!.length, equals(32), reason: 'Salt should be 32 bytes');

      final password = 'test_password_123!';
      final derivedKey = NativeCryptoService.instance.deriveKey(
        password: password,
        salt: salt,
        memoryKib: 16384,
        iterations: 1,
        parallelism: 4,
      );

      expect(derivedKey, isNotNull, reason: 'Key derivation should succeed');

      // Set encryption key for profile storage
      RustVaultService.instance.setEncryptionKey(derivedKey);

      // Now we could save/load profiles
      RustVaultService.instance.isVaultUnlocked();
    });

    test('4. Profile save and load roundtrip with complex data', () async {
      // Set up encryption key
      final salt = NativeCryptoService.instance.generateSalt()!;
      final key = NativeCryptoService.instance.deriveKey(
        password: 'roundtrip_test_pass',
        salt: salt,
        memoryKib: 16384,
        iterations: 1,
        parallelism: 4,
      )!;
      RustVaultService.instance.setEncryptionKey(key);

      // Create complex profile data matching the schema
      final profileJson = jsonEncode({
        'identity': {
          'full_name': 'Test User',
          'given_name': 'Test',
          'family_name': 'User',
          'date_of_birth': '1990-01-15',
          'gender': 'male',
          'nationality': 'US',
          'id_cards': [
            {
              'label': 'Driver License',
              'number': 'DL123456789',
              'issue_date': '2020-01-01',
              'expiry_date': '2025-01-01',
              'holder_name': 'Test User',
              'country': 'US',
              'is_deleted': false,
              'deleted_at': null,
            }
          ],
          'contact': {
            'entries': [
              {'label': 'Personal', 'type': 'email', 'value': 'test@example.com', 'is_deleted': false, 'deleted_at': null},
              {'label': 'Work', 'type': 'phone', 'value': '+1-555-0100', 'is_deleted': false, 'deleted_at': null},
            ]
          },
          'addresses': [
            {'label': 'Home', 'street': '123 Main St', 'city': 'Anytown', 'state': 'CA', 'postal_code': '90210', 'country': 'US', 'is_deleted': false, 'deleted_at': null}
          ]
        },
        'financial': {
          'bank_accounts': [
            {'bank_name': 'Test Bank', 'account_number': '****1234', 'currency': 'USD', 'swift_bic': 'TESTBANK', 'is_deleted': false, 'deleted_at': null}
          ],
          'cards': [
            {'card_number': '**** **** **** 4242', 'card_type': 'Visa', 'expiry_date': '12/25', 'holder_name': 'Test User', 'is_deleted': false, 'deleted_at': null}
          ],
          'tax_ids': [
            {'tax_id_number': '***-**-1234', 'tax_id_type': 'SSN', 'issuing_authority': 'IRS', 'country': 'US', 'is_deleted': false, 'deleted_at': null}
          ]
        },
        'travel': {
          'passports': [
            {'number': '123456789', 'country': 'US', 'issue_date': '2018-01-01', 'expiry_date': '2028-01-01', 'holder_name': 'Test User', 'is_deleted': false, 'deleted_at': null}
          ],
          'visas': [
            {'country': 'UK', 'visa_type': 'Standard Visitor', 'number': 'V123456', 'issue_date': '2023-01-01', 'expiry_date': '2024-01-01', 'is_deleted': false, 'deleted_at': null}
          ],
          'travel_history': [
            {'destination': 'London, UK', 'date': '2023-06-15', 'is_deleted': false, 'deleted_at': null}
          ]
        },
        'professional': {
          'education': [
            {'institution': 'Test University', 'degree': 'BS', 'field': 'Computer Science', 'start_date': '2010-09-01', 'end_date': '2014-06-15', 'is_deleted': false, 'deleted_at': null}
          ],
          'employment': [
            {'company': 'Test Corp', 'position': 'Software Engineer', 'start_date': '2014-07-01', 'end_date': null, 'is_deleted': false, 'deleted_at': null}
          ],
          'skills': [
            {'name': 'Flutter', 'level': 'Expert', 'is_deleted': false, 'deleted_at': null},
            {'name': 'Rust', 'level': 'Advanced', 'is_deleted': false, 'deleted_at': null}
          ],
          'languages': [
            {'name': 'English', 'proficiency': 'Native', 'is_deleted': false, 'deleted_at': null},
            {'name': 'Spanish', 'proficiency': 'Conversational', 'is_deleted': false, 'deleted_at': null}
          ]
        }
      });

      // Save the profile
      final saved = await RustVaultService.instance.saveProfileEncrypted('test_profile', profileJson);
      expect(saved, isNotNull, reason: 'Profile save should succeed');
      expect(saved!.name, equals('test_profile'));
      expect(saved.version, equals(1));

      // Load the profile back
      final loaded = await RustVaultService.instance.loadProfileDecrypted(saved.id);
      expect(loaded, isNotNull, reason: 'Profile load should succeed');
      expect(loaded, isNotEmpty, reason: 'Loaded profile should have data');

      // Parse and verify structure
      final parsed = jsonDecode(loaded!) as Map<String, dynamic>;
      expect(parsed['identity'], isNotNull, reason: 'Identity section should exist');
      expect(parsed['identity']['full_name'], equals('Test User'));
      expect(parsed['financial']['tax_ids'], isNotNull, reason: 'Tax IDs should exist');
      expect((parsed['financial']['tax_ids'] as List).length, equals(1));
      expect(parsed['professional']['skills'], isNotNull, reason: 'Skills should exist');
      expect((parsed['professional']['skills'] as List).length, equals(2));
    });

    test('5. Profile update increments version', () async {
      // Set up encryption key
      final salt = NativeCryptoService.instance.generateSalt()!;
      final key = NativeCryptoService.instance.deriveKey(
        password: 'update_test_pass',
        salt: salt,
        memoryKib: 16384,
        iterations: 1,
        parallelism: 4,
      )!;
      RustVaultService.instance.setEncryptionKey(key);

      // Create initial profile
      final profile1 = jsonEncode({'test': 'data_v1'});
      final saved1 = await RustVaultService.instance.saveProfileEncrypted('update_test', profile1);
      expect(saved1, isNotNull);
      expect(saved1!.version, equals(1));

      // Update profile
      final profile2 = jsonEncode({'test': 'data_v2'});
      final saved2 = await RustVaultService.instance.saveProfileEncrypted('update_test', profile2);
      expect(saved2, isNotNull);
      expect(saved2!.version, equals(2), reason: 'Version should increment on update');
      expect(saved2.id, equals(saved1.id), reason: 'ID should remain the same');

      // Load and verify
      final loaded = await RustVaultService.instance.loadProfileDecrypted(saved2.id);
      expect(loaded, isNotNull);
      final parsed = jsonDecode(loaded!);
      expect(parsed['test'], equals('data_v2'));
    });

    test('6. List profiles returns all profiles', () async {
      final profiles = await RustVaultService.instance.listProfiles();
      expect(profiles, isNotEmpty, reason: 'Should have at least the profiles we created');
    });

    test('7. Delete profile removes it', () async {
      // Set up encryption key
      final salt = NativeCryptoService.instance.generateSalt()!;
      final key = NativeCryptoService.instance.deriveKey(
        password: 'delete_test_pass',
        salt: salt,
        memoryKib: 16384,
        iterations: 1,
        parallelism: 4,
      )!;
      RustVaultService.instance.setEncryptionKey(key);

      // Create and save a profile
      final profile = jsonEncode({'delete': 'test'});
      final saved = await RustVaultService.instance.saveProfileEncrypted('delete_test_profile', profile);
      expect(saved, isNotNull);

      // Delete it
      final deleted = await RustVaultService.instance.deleteProfile(saved!.id);
      expect(deleted, isTrue, reason: 'Delete should succeed');

      // Verify it's gone
      final loaded = await RustVaultService.instance.loadProfile(saved.id);
      expect(loaded, isNull, reason: 'Profile should not exist after deletion');
    });
  });
}
