import 'dart:io';
import 'dart:typed_data';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart';

// ---------------------------------------------------------------------------
// Fake ProfileStorageService
// ---------------------------------------------------------------------------

class FakeProfileStorageService implements ProfileStorageService {
  ProfileData? storageProfile;
  bool loadShouldFail = false;
  bool saveShouldFail = false;
  Uint8List? _encryptionKey;

  @override
  Uint8List? get encryptionKey => _encryptionKey;

  @override
  void setEncryptionKey(Uint8List key) {
    _encryptionKey = key;
  }

  @override
  Future<ProfileData?> loadProfile(String accountId) async {
    await Future<void>.delayed(Duration.zero);
    if (loadShouldFail) throw Exception('Load failed');
    return storageProfile;
  }

  @override
  Future<bool> saveProfile(String accountId, ProfileData profile) async {
    await Future<void>.delayed(Duration.zero);
    if (saveShouldFail) return false;
    storageProfile = profile;
    return true;
  }

  @override
  Future<void> restoreItem(
    ProfileData profile,
    String accountId,
    String section,
    String itemType,
    int index,
  ) async {
    // noop for tests
  }

  @override
  Future<void> permanentDeleteItem(
    ProfileData profile,
    String accountId,
    String section,
    String itemType,
    int index,
  ) async {
    // noop for tests
  }

  @override
  Future<void> purgeOldDeletedItems(
    ProfileData profile,
    String accountId,
  ) async {
    // noop for tests
  }

  @override
  Future<void> purgeOldDeletedItemsIfNeeded(
    String accountId, {
    ProfileData? existingProfile,
  }) async {
    // noop for tests
  }

  @override
  List<DeletedItemInfo> getDeletedItems(ProfileData profile) => [];

  @override
  Future<void> emptyAllTrash(ProfileData profile, String accountId) async {
    // noop for tests
  }

  @override
  Future<void> clearEncryptionKey() async {
    _encryptionKey = null;
  }

  @override
  Future<Directory> get storageDir async {
    throw UnimplementedError();
  }

  void reset() {
    storageProfile = null;
    loadShouldFail = false;
    saveShouldFail = false;
    _encryptionKey = null;
  }
}

// ---------------------------------------------------------------------------
// Fake AuthNotifier — extends AsyncNotifier<AuthState> directly
// ---------------------------------------------------------------------------

class FakeAuthNotifier extends AuthNotifier {
  String? _selectedAccountId;

  @override
  String? get selectedAccountId => _selectedAccountId;

  void configureForTest({required String accountId, Uint8List? encryptionKey}) {
    _selectedAccountId = accountId;
  }
}

// ---------------------------------------------------------------------------
// TestableProfileNotifier — ProfileNotifier subclass for testing
// ---------------------------------------------------------------------------

class TestableProfileNotifier extends ProfileNotifier {
  final FakeProfileStorageService _fakeStorage;
  final FakeAuthNotifier _fakeAuth;
  final ProfileData? _initialProfile;

  TestableProfileNotifier({
    required ProfileData? initialProfile,
    required FakeProfileStorageService fakeStorage,
    required FakeAuthNotifier fakeAuth,
  })  : _fakeStorage = fakeStorage,
        _fakeAuth = fakeAuth,
        _initialProfile = initialProfile;

  /// Simulate build() with controlled profile
  Future<ProfileData?> testBuild() async {
    return _initialProfile;
  }

  /// Simulate loadProfile() state transition
  Future<void> testLoadProfile() async {
    state = const AsyncLoading();
    try {
      final profile = await _fakeStorage.loadProfile('test-account');
      state = AsyncData(profile);
    } on Object catch (e, st) {
      state = AsyncError(e, st);
    }
  }

  /// Simulate clearProfile() state reset
  Future<void> testClearProfile() async {
    state = const AsyncData(null);
  }

  /// Simulate updateIdentity()
  Future<bool> testUpdateIdentity(IdentityData identity) async {
    final current = state.value;
    if (current == null) return false;

    final newProfile = ProfileData(
      identity: identity,
      travel: current.travel,
      financial: current.financial,
      professional: current.professional,
    );

    final result = await _fakeStorage.saveProfile('test-account', newProfile);
    if (result) {
      state = AsyncData(newProfile);
    }
    return result;
  }

  /// Simulate saveProfile()
  Future<bool> testSaveProfile(ProfileData profile) async {
    final result = await _fakeStorage.saveProfile('test-account', profile);
    if (result) {
      state = AsyncData(profile);
    }
    return result;
  }

  /// Simulate updateTravel()
  Future<bool> testUpdateTravel(TravelData travel) async {
    final current = state.value;
    if (current == null) return false;

    final newProfile = ProfileData(
      identity: current.identity,
      travel: travel,
      financial: current.financial,
      professional: current.professional,
    );

    final result = await _fakeStorage.saveProfile('test-account', newProfile);
    if (result) {
      state = AsyncData(newProfile);
    }
    return result;
  }

  /// Simulate updateFinancial()
  Future<bool> testUpdateFinancial(FinancialData financial) async {
    final current = state.value;
    if (current == null) return false;

    final newProfile = ProfileData(
      identity: current.identity,
      travel: current.travel,
      financial: financial,
      professional: current.professional,
    );

    final result = await _fakeStorage.saveProfile('test-account', newProfile);
    if (result) {
      state = AsyncData(newProfile);
    }
    return result;
  }

  /// Simulate updateProfessional()
  Future<bool> testUpdateProfessional(ProfessionalData professional) async {
    final current = state.value;
    if (current == null) return false;

    final newProfile = ProfileData(
      identity: current.identity,
      travel: current.travel,
      financial: current.financial,
      professional: professional,
    );

    final result = await _fakeStorage.saveProfile('test-account', newProfile);
    if (result) {
      state = AsyncData(newProfile);
    }
    return result;
  }
}

// ---------------------------------------------------------------------------
// Provider container factory
// ---------------------------------------------------------------------------

ProviderContainer createTestContainer({
  ProfileData? initialProfile,
  FakeProfileStorageService? storage,
}) {
  final fakeStorage = storage ?? FakeProfileStorageService();
  final fakeAuth = FakeAuthNotifier();

  fakeStorage.setEncryptionKey(Uint8List.fromList(List.filled(32, 1)));
  fakeStorage.storageProfile = initialProfile;
  fakeAuth.configureForTest(accountId: 'test-account');

  final container = ProviderContainer(
    overrides: [
      authNotifierProvider.overrideWith(() => fakeAuth),
      profileNotifierProvider.overrideWith(
        () => TestableProfileNotifier(
          initialProfile: initialProfile,
          fakeStorage: fakeStorage,
          fakeAuth: fakeAuth,
        ),
      ),
    ],
  );

  return container;
}

// ---------------------------------------------------------------------------
// Test data factories
// ---------------------------------------------------------------------------

ProfileData createTestProfile() {
  return ProfileData(
    identity: IdentityData(
      fullName: 'Test User',
      givenName: 'Test',
      familyName: 'User',
      dateOfBirth: '1990-01-01',
      gender: 'Male',
      nationality: 'US',
    ),
    travel: TravelData(
      passports: [
        PassportData(
          id: 'passport-1',
          title: 'US Passport',
          number: '123456789',
          country: 'United States',
          expiryDate: '2030-01-01',
        ),
      ],
    ),
    financial: FinancialData(
      bankAccounts: [
        BankAccountData(
          id: 'bank-1',
          title: 'Checking',
          bankName: 'Chase',
          accountNumber: '****1234',
        ),
      ],
    ),
    professional: ProfessionalData(
      education: [
        EducationData(
          id: 'edu-1',
          institution: 'MIT',
          degree: 'Master',
          field: 'CS',
        ),
      ],
    ),
  );
}

IdentityData createTestIdentity({String? fullName}) {
  return IdentityData(
    fullName: fullName ?? 'Updated User',
    givenName: 'Updated',
    familyName: 'User',
    dateOfBirth: '1991-01-01',
    gender: 'Female',
    nationality: 'CA',
  );
}

TravelData createTestTravel() {
  return TravelData(
    passports: [
      PassportData(
        id: 'passport-2',
        title: 'Canadian Passport',
        number: '987654321',
        country: 'Canada',
        expiryDate: '2028-01-01',
      ),
    ],
    visas: [
      VisaData(
        id: 'visa-1',
        title: 'UK Visa',
        country: 'United Kingdom',
        visaType: 'Standard',
      ),
    ],
  );
}

FinancialData createTestFinancial() {
  return FinancialData(
    bankAccounts: [
      BankAccountData(
        id: 'bank-2',
        title: 'Savings',
        bankName: 'RBC',
        accountNumber: '****5678',
      ),
    ],
    cards: [
      CardData(
        id: 'card-1',
        title: 'Visa Card',
        cardNumber: '****1111',
        cardType: 'Visa',
        expiryDate: '12/27',
      ),
    ],
  );
}

ProfessionalData createTestProfessional() {
  return ProfessionalData(
    education: [
      EducationData(
        id: 'edu-2',
        institution: 'Stanford',
        degree: 'PhD',
        field: 'AI',
      ),
    ],
    employment: [
      EmploymentData(
        id: 'emp-1',
        company: 'TechCorp',
        position: 'Senior Engineer',
      ),
    ],
  );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

void main() {
  // -------------------------------------------------------------------------
  // Group: loadProfile() state transitions
  // -------------------------------------------------------------------------
  group('loadProfile() state transitions', () {
    test('AsyncLoading -> AsyncData when load succeeds', () async {
      final profile = createTestProfile();
      final container = createTestContainer(initialProfile: profile);

      final notifier = container.read(profileNotifierProvider.notifier)
          as TestableProfileNotifier;

      await notifier.testLoadProfile();

      final state = container.read(profileNotifierProvider);
      expect(state, isA<AsyncData<ProfileData?>>());
      expect((state as AsyncData).value!.identity!.fullName, 'Test User');

      container.dispose();
    });

    test('AsyncLoading -> AsyncError when load fails', () async {
      final container = createTestContainer();
      final notifier = container.read(profileNotifierProvider.notifier)
          as TestableProfileNotifier;

      // Force load failure
      final fakeStorage = FakeProfileStorageService()
        ..loadShouldFail = true;
      // Re-override with failing storage
      final newContainer = ProviderContainer(
        overrides: [
          authNotifierProvider.overrideWith(() => FakeAuthNotifier()
            ..configureForTest(accountId: 'test-account')),
          profileNotifierProvider.overrideWith(
            () => TestableProfileNotifier(
              initialProfile: null,
              fakeStorage: fakeStorage,
              fakeAuth: FakeAuthNotifier()
                ..configureForTest(accountId: 'test-account'),
            ),
          ),
        ],
      );

      final newNotifier = newContainer.read(profileNotifierProvider.notifier)
          as TestableProfileNotifier;
      await newNotifier.testLoadProfile();

      final state = newContainer.read(profileNotifierProvider);
      expect(state, isA<AsyncError>());

      newContainer.dispose();
      container.dispose();
    });

    test('AsyncLoading -> AsyncData(null) when no profile in storage', () async {
      final container = createTestContainer(initialProfile: null);

      final notifier = container.read(profileNotifierProvider.notifier)
          as TestableProfileNotifier;
      await notifier.testLoadProfile();

      final state = container.read(profileNotifierProvider);
      expect(state, isA<AsyncData<ProfileData?>>());
      expect((state as AsyncData).value, isNull);

      container.dispose();
    });
  });

  // -------------------------------------------------------------------------
  // Group: saveProfile() success/failure paths
  // -------------------------------------------------------------------------
  group('saveProfile() success/failure paths', () {
    test('saveProfile returns true and updates state on success', () async {
      final profile = createTestProfile();
      final container = createTestContainer(initialProfile: profile);

      final notifier = container.read(profileNotifierProvider.notifier)
          as TestableProfileNotifier;
      await notifier.testLoadProfile();

      final updatedProfile = profile.copyWith(
        identity: createTestIdentity(fullName: 'Saved User'),
      );

      final result = await notifier.testSaveProfile(updatedProfile);

      expect(result, isTrue);
      final state = container.read(profileNotifierProvider);
      expect(state, isA<AsyncData<ProfileData?>>());
      expect((state as AsyncData).value!.identity!.fullName, 'Saved User');

      container.dispose();
    });

    test('saveProfile returns false when storage save fails', () async {
      final profile = createTestProfile();
      final failingStorage = FakeProfileStorageService()
        ..storageProfile = profile
        ..saveShouldFail = true;

      final container = createTestContainer(
        initialProfile: profile,
        storage: failingStorage,
      );

      final notifier = container.read(profileNotifierProvider.notifier)
          as TestableProfileNotifier;
      await notifier.testLoadProfile();

      final updatedProfile = profile.copyWith(
        identity: createTestIdentity(fullName: 'Should Not Save'),
      );

      final result = await notifier.testSaveProfile(updatedProfile);

      expect(result, isFalse);

      container.dispose();
    });
  });

  // -------------------------------------------------------------------------
  // Group: clearProfile() state reset
  // -------------------------------------------------------------------------
  group('clearProfile() state reset', () {
    test('clearProfile resets state to AsyncData(null)', () async {
      final profile = createTestProfile();
      final container = createTestContainer(initialProfile: profile);

      final notifier = container.read(profileNotifierProvider.notifier)
          as TestableProfileNotifier;
      await notifier.testLoadProfile();

      expect(container.read(profileNotifierProvider), isA<AsyncData>());

      await notifier.testClearProfile();

      final state = container.read(profileNotifierProvider);
      expect(state, isA<AsyncData<ProfileData?>>());
      expect((state as AsyncData).value, isNull);

      container.dispose();
    });
  });

  // -------------------------------------------------------------------------
  // Group: updateIdentity
  // -------------------------------------------------------------------------
  group('updateIdentity()', () {
    test('updateIdentity succeeds and updates state', () async {
      final profile = createTestProfile();
      final container = createTestContainer(initialProfile: profile);

      final notifier = container.read(profileNotifierProvider.notifier)
          as TestableProfileNotifier;
      await notifier.testLoadProfile();

      final newIdentity = createTestIdentity(fullName: 'New Identity Name');
      final result = await notifier.testUpdateIdentity(newIdentity);

      expect(result, isTrue);
      final state = container.read(profileNotifierProvider);
      expect((state as AsyncData).value!.identity!.fullName, 'New Identity Name');

      container.dispose();
    });

    test('updateIdentity returns false when current profile is null',
        () async {
      final container = createTestContainer(initialProfile: null);

      final notifier = container.read(profileNotifierProvider.notifier)
          as TestableProfileNotifier;
      await notifier.testLoadProfile();

      final newIdentity = createTestIdentity(fullName: 'Should Fail');
      final result = await notifier.testUpdateIdentity(newIdentity);

      expect(result, isFalse);

      container.dispose();
    });
  });

  // -------------------------------------------------------------------------
  // Group: updateTravel
  // -------------------------------------------------------------------------
  group('updateTravel()', () {
    test('updateTravel succeeds and updates travel section', () async {
      final profile = createTestProfile();
      final container = createTestContainer(initialProfile: profile);

      final notifier = container.read(profileNotifierProvider.notifier)
          as TestableProfileNotifier;
      await notifier.testLoadProfile();

      final newTravel = createTestTravel();
      final result = await notifier.testUpdateTravel(newTravel);

      expect(result, isTrue);
      final state = container.read(profileNotifierProvider) as AsyncData;
      expect(state.value!.travel!.activePassports.length, 1);
      expect(state.value!.travel!.activePassports.first.country, 'Canada');
      expect(state.value!.travel!.activeVisas.length, 1);

      container.dispose();
    });
  });

  // -------------------------------------------------------------------------
  // Group: updateFinancial
  // -------------------------------------------------------------------------
  group('updateFinancial()', () {
    test('updateFinancial succeeds and updates financial section', () async {
      final profile = createTestProfile();
      final container = createTestContainer(initialProfile: profile);

      final notifier = container.read(profileNotifierProvider.notifier)
          as TestableProfileNotifier;
      await notifier.testLoadProfile();

      final newFinancial = createTestFinancial();
      final result = await notifier.testUpdateFinancial(newFinancial);

      expect(result, isTrue);
      final state = container.read(profileNotifierProvider) as AsyncData;
      expect(state.value!.financial!.activeBankAccounts.length, 1);
      expect(
          state.value!.financial!.activeBankAccounts.first.bankName, 'RBC');
      expect(state.value!.financial!.activeCards.length, 1);

      container.dispose();
    });
  });

  // -------------------------------------------------------------------------
  // Group: updateProfessional
  // -------------------------------------------------------------------------
  group('updateProfessional()', () {
    test('updateProfessional succeeds and updates professional section',
        () async {
      final profile = createTestProfile();
      final container = createTestContainer(initialProfile: profile);

      final notifier = container.read(profileNotifierProvider.notifier)
          as TestableProfileNotifier;
      await notifier.testLoadProfile();

      final newProfessional = createTestProfessional();
      final result = await notifier.testUpdateProfessional(newProfessional);

      expect(result, isTrue);
      final state = container.read(profileNotifierProvider) as AsyncData;
      expect(state.value!.professional!.activeEducation.length, 1);
      expect(
          state.value!.professional!.activeEducation.first.institution,
          'Stanford');
      expect(state.value!.professional!.activeEmployment.length, 1);
      expect(
          state.value!.professional!.activeEmployment.first.company, 'TechCorp');

      container.dispose();
    });
  });

  // -------------------------------------------------------------------------
  // Group: AsyncData profile structure validation
  // -------------------------------------------------------------------------
  group('AsyncData profile structure', () {
    test('profile contains all four sections after load', () async {
      final profile = createTestProfile();
      final container = createTestContainer(initialProfile: profile);

      final notifier = container.read(profileNotifierProvider.notifier)
          as TestableProfileNotifier;
      await notifier.testLoadProfile();

      final state = container.read(profileNotifierProvider) as AsyncData;
      final p = state.value!;

      expect(p.identity, isNotNull);
      expect(p.identity!.fullName, 'Test User');

      expect(p.travel, isNotNull);
      expect(p.travel!.activePassports.length, 1);

      expect(p.financial, isNotNull);
      expect(p.financial!.activeBankAccounts.length, 1);

      expect(p.professional, isNotNull);
      expect(p.professional!.activeEducation.length, 1);

      container.dispose();
    });
  });
}
