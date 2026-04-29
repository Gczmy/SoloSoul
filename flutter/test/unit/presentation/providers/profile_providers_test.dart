import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart';

void main() {
  group('Section Item Providers', () {
    group('EducationItems', () {
      test('returns empty list when profile is null', () {
        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(() => _FakeProfileNotifier(null)),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(educationItemsProvider);
        expect(items, isEmpty);
      });

      test('returns empty list when professional is null', () {
        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(() => _FakeProfileNotifier(const ProfileData())),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(educationItemsProvider);
        expect(items, isEmpty);
      });

      test('returns sorted education items', () {
        final professional = ProfessionalData(
          education: [
            EducationData(id: '1', institution: 'MIT', degree: 'Bachelor'),
            EducationData(id: '2', institution: 'Harvard', degree: 'PhD'),
            EducationData(id: '3', institution: 'Stanford', degree: 'Master'),
          ],
        );

        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(
              () => _FakeProfileNotifier(ProfileData(professional: professional)),
            ),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(educationItemsProvider);
        expect(items.length, 3);
        // Should be sorted: PhD first, then Master, then Bachelor
        expect(items[0].institution, 'Harvard'); // PhD
        expect(items[1].institution, 'Stanford'); // Master
        expect(items[2].institution, 'MIT'); // Bachelor
      });

      test('filters out deleted items', () {
        final professional = ProfessionalData(
          education: [
            EducationData(id: '1', institution: 'MIT', degree: 'Bachelor', isDeleted: false),
            EducationData(id: '2', institution: 'Deleted School', degree: 'Master', isDeleted: true),
          ],
        );

        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(
              () => _FakeProfileNotifier(ProfileData(professional: professional)),
            ),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(educationItemsProvider);
        expect(items.length, 1);
        expect(items[0].institution, 'MIT');
      });
    });

    group('BankAccountItems', () {
      test('returns empty list when profile is null', () {
        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(() => _FakeProfileNotifier(null)),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(bankAccountItemsProvider);
        expect(items, isEmpty);
      });

      test('returns empty list when financial is null', () {
        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(() => _FakeProfileNotifier(const ProfileData())),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(bankAccountItemsProvider);
        expect(items, isEmpty);
      });

      test('returns active bank accounts', () {
        final financial = FinancialData(
          bankAccounts: [
            BankAccountData(id: '1', bankName: 'Chase', isDeleted: false),
            BankAccountData(id: '2', bankName: 'BoA', isDeleted: true),
          ],
        );

        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(
              () => _FakeProfileNotifier(ProfileData(financial: financial)),
            ),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(bankAccountItemsProvider);
        expect(items.length, 1);
        expect(items[0].bankName, 'Chase');
      });
    });

    group('EmploymentItems', () {
      test('returns empty list when profile is null', () {
        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(() => _FakeProfileNotifier(null)),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(employmentItemsProvider);
        expect(items, isEmpty);
      });

      test('returns active employment items', () {
        final professional = ProfessionalData(
          employment: [
            EmploymentData(id: '1', company: 'Google', isDeleted: false),
            EmploymentData(id: '2', company: 'Meta', isDeleted: true),
          ],
        );

        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(
              () => _FakeProfileNotifier(ProfileData(professional: professional)),
            ),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(employmentItemsProvider);
        expect(items.length, 1);
        expect(items[0].company, 'Google');
      });
    });

    group('SkillItems', () {
      test('returns empty list when profile is null', () {
        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(() => _FakeProfileNotifier(null)),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(skillItemsProvider);
        expect(items, isEmpty);
      });

      test('returns active skills', () {
        final professional = ProfessionalData(
          skills: [
            SkillData(id: '1', name: 'Dart', isDeleted: false),
            SkillData(id: '2', name: 'Java', isDeleted: true),
          ],
        );

        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(
              () => _FakeProfileNotifier(ProfileData(professional: professional)),
            ),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(skillItemsProvider);
        expect(items.length, 1);
        expect(items[0].name, 'Dart');
      });
    });

    group('TaxIdItems', () {
      test('returns empty list when profile is null', () {
        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(() => _FakeProfileNotifier(null)),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(taxIdItemsProvider);
        expect(items, isEmpty);
      });

      test('returns active tax IDs', () {
        final financial = FinancialData(
          taxIds: [
            TaxIdData(id: '1', taxIdNumber: '123', isDeleted: false),
            TaxIdData(id: '2', taxIdNumber: '456', isDeleted: true),
          ],
        );

        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(
              () => _FakeProfileNotifier(ProfileData(financial: financial)),
            ),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(taxIdItemsProvider);
        expect(items.length, 1);
        expect(items[0].taxIdNumber, '123');
      });
    });

    group('PassportItems', () {
      test('returns empty list when profile is null', () {
        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(() => _FakeProfileNotifier(null)),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(passportItemsProvider);
        expect(items, isEmpty);
      });

      test('returns active passports', () {
        final travel = TravelData(
          passports: [
            PassportData(id: '1', number: 'P123', isDeleted: false),
            PassportData(id: '2', number: 'P456', isDeleted: true),
          ],
        );

        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(
              () => _FakeProfileNotifier(ProfileData(travel: travel)),
            ),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(passportItemsProvider);
        expect(items.length, 1);
        expect(items[0].number, 'P123');
      });
    });

    group('VisaItems', () {
      test('returns empty list when profile is null', () {
        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(() => _FakeProfileNotifier(null)),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(visaItemsProvider);
        expect(items, isEmpty);
      });

      test('returns active visas', () {
        final travel = TravelData(
          visas: [
            VisaData(id: '1', number: 'V123', isDeleted: false),
            VisaData(id: '2', number: 'V456', isDeleted: true),
          ],
        );

        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(
              () => _FakeProfileNotifier(ProfileData(travel: travel)),
            ),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(visaItemsProvider);
        expect(items.length, 1);
        expect(items[0].number, 'V123');
      });
    });

    group('TravelHistoryItems', () {
      test('returns empty list when profile is null', () {
        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(() => _FakeProfileNotifier(null)),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(travelHistoryItemsProvider);
        expect(items, isEmpty);
      });

      test('returns active travel history', () {
        final travel = TravelData(
          travelHistory: [
            TravelHistoryData(id: '1', destination: 'NYC', isDeleted: false),
            TravelHistoryData(id: '2', destination: 'LA', isDeleted: true),
          ],
        );

        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(
              () => _FakeProfileNotifier(ProfileData(travel: travel)),
            ),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(travelHistoryItemsProvider);
        expect(items.length, 1);
        expect(items[0].destination, 'NYC');
      });
    });

    group('CardItems', () {
      test('returns empty list when profile is null', () {
        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(() => _FakeProfileNotifier(null)),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(cardItemsProvider);
        expect(items, isEmpty);
      });

      test('returns active cards', () {
        final financial = FinancialData(
          cards: [
            CardData(id: '1', cardNumber: '1234', isDeleted: false),
            CardData(id: '2', cardNumber: '5678', isDeleted: true),
          ],
        );

        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(
              () => _FakeProfileNotifier(ProfileData(financial: financial)),
            ),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(cardItemsProvider);
        expect(items.length, 1);
        expect(items[0].cardNumber, '1234');
      });
    });

    group('ContactItems', () {
      test('returns empty list when profile is null', () {
        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(() => _FakeProfileNotifier(null)),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(contactItemsProvider);
        expect(items, isEmpty);
      });

      test('returns empty list when identity is null', () {
        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(() => _FakeProfileNotifier(const ProfileData())),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(contactItemsProvider);
        expect(items, isEmpty);
      });

      test('returns active contact entries', () {
        final identity = IdentityData(
          contact: ContactData(
            entries: [
              ContactEntry(id: '1', title: 'Email', type: 'email', value: 'test@example.com', isDeleted: false),
              ContactEntry(id: '2', title: 'Phone', type: 'phone', value: '123', isDeleted: true),
            ],
          ),
        );

        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(
              () => _FakeProfileNotifier(ProfileData(identity: identity)),
            ),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(contactItemsProvider);
        expect(items.length, 1);
        expect(items[0].title, 'Email');
      });
    });

    group('LanguageItems', () {
      test('returns empty list when profile is null', () {
        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(() => _FakeProfileNotifier(null)),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(languageItemsProvider);
        expect(items, isEmpty);
      });

      test('returns active languages', () {
        final professional = ProfessionalData(
          languages: [
            LanguageData(id: '1', name: 'English', isDeleted: false),
            LanguageData(id: '2', name: 'Spanish', isDeleted: true),
          ],
        );

        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(
              () => _FakeProfileNotifier(ProfileData(professional: professional)),
            ),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(languageItemsProvider);
        expect(items.length, 1);
        expect(items[0].name, 'English');
      });
    });

    group('AwardItems', () {
      test('returns empty list when profile is null', () {
        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(() => _FakeProfileNotifier(null)),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(awardItemsProvider);
        expect(items, isEmpty);
      });

      test('returns active awards', () {
        final professional = ProfessionalData(
          awards: [
            AwardData(id: '1', title: 'Best Dev', isDeleted: false),
            AwardData(id: '2', title: 'MVP', isDeleted: true),
          ],
        );

        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(
              () => _FakeProfileNotifier(ProfileData(professional: professional)),
            ),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(awardItemsProvider);
        expect(items.length, 1);
        expect(items[0].title, 'Best Dev');
      });
    });

    group('IdCardItems', () {
      test('returns empty list when profile is null', () {
        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(() => _FakeProfileNotifier(null)),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(idCardItemsProvider);
        expect(items, isEmpty);
      });

      test('returns empty list when identity is null', () {
        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(() => _FakeProfileNotifier(const ProfileData())),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(idCardItemsProvider);
        expect(items, isEmpty);
      });

      test('returns active ID cards', () {
        final identity = IdentityData(
          idCards: [
            IdCardData(id: '1', title: 'Driver License', isDeleted: false),
            IdCardData(id: '2', title: 'Old ID', isDeleted: true),
          ],
        );

        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(
              () => _FakeProfileNotifier(ProfileData(identity: identity)),
            ),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(idCardItemsProvider);
        expect(items.length, 1);
        expect(items[0].title, 'Driver License');
      });
    });

    group('AddressItems', () {
      test('returns empty list when profile is null', () {
        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(() => _FakeProfileNotifier(null)),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(addressItemsProvider);
        expect(items, isEmpty);
      });

      test('returns empty list when identity is null', () {
        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(() => _FakeProfileNotifier(const ProfileData())),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(addressItemsProvider);
        expect(items, isEmpty);
      });

      test('returns active addresses', () {
        final identity = IdentityData(
          addresses: [
            AddressData(id: '1', title: 'Home', city: 'NYC', isDeleted: false),
            AddressData(id: '2', title: 'Old', city: 'LA', isDeleted: true),
          ],
        );

        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(
              () => _FakeProfileNotifier(ProfileData(identity: identity)),
            ),
          ],
        );
        addTearDown(container.dispose);

        final items = container.read(addressItemsProvider);
        expect(items.length, 1);
        expect(items[0].city, 'NYC');
      });
    });
  });

  group('Section Providers', () {
    group('ProfileIdentity', () {
      test('returns null when profile is null', () {
        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(() => _FakeProfileNotifier(null)),
          ],
        );
        addTearDown(container.dispose);

        final identity = container.read(profileIdentityProvider);
        expect(identity, isNull);
      });

      test('returns identity data from profile', () {
        final identity = IdentityData(fullName: 'Test User');
        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(
              () => _FakeProfileNotifier(ProfileData(identity: identity)),
            ),
          ],
        );
        addTearDown(container.dispose);

        final result = container.read(profileIdentityProvider);
        expect(result, isNotNull);
        expect(result!.fullName, 'Test User');
      });
    });

    group('ProfileTravel', () {
      test('returns null when profile is null', () {
        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(() => _FakeProfileNotifier(null)),
          ],
        );
        addTearDown(container.dispose);

        final travel = container.read(profileTravelProvider);
        expect(travel, isNull);
      });

      test('returns travel data from profile', () {
        final travel = TravelData();
        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(
              () => _FakeProfileNotifier(ProfileData(travel: travel)),
            ),
          ],
        );
        addTearDown(container.dispose);

        final result = container.read(profileTravelProvider);
        expect(result, isNotNull);
      });
    });

    group('ProfileFinancial', () {
      test('returns null when profile is null', () {
        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(() => _FakeProfileNotifier(null)),
          ],
        );
        addTearDown(container.dispose);

        final financial = container.read(profileFinancialProvider);
        expect(financial, isNull);
      });

      test('returns financial data from profile', () {
        final financial = FinancialData();
        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(
              () => _FakeProfileNotifier(ProfileData(financial: financial)),
            ),
          ],
        );
        addTearDown(container.dispose);

        final result = container.read(profileFinancialProvider);
        expect(result, isNotNull);
      });
    });

    group('ProfileProfessional', () {
      test('returns null when profile is null', () {
        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(() => _FakeProfileNotifier(null)),
          ],
        );
        addTearDown(container.dispose);

        final professional = container.read(profileProfessionalProvider);
        expect(professional, isNull);
      });

      test('returns professional data from profile', () {
        final professional = ProfessionalData();
        final container = ProviderContainer(
          overrides: [
            profileNotifierProvider.overrideWith(
              () => _FakeProfileNotifier(ProfileData(professional: professional)),
            ),
          ],
        );
        addTearDown(container.dispose);

        final result = container.read(profileProfessionalProvider);
        expect(result, isNotNull);
      });
    });
  });
}

/// Fake profile notifier for testing
class _FakeProfileNotifier extends ProfileNotifier {
  _FakeProfileNotifier(this._profileData);

  final ProfileData? _profileData;

  @override
  Future<ProfileData?> build() async {
    state = AsyncData(_profileData);
    return _profileData;
  }
}
