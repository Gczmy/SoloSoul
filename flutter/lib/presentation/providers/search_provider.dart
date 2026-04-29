import 'dart:async';
import 'dart:isolate';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart';
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';
import 'package:solosoul_flutter/presentation/models/search_models.dart';

/// Search notifier
class SearchNotifier extends Notifier<SearchState> {
  Timer? _debounceTimer;

  @override
  SearchState build() {
    ref.onDispose(() {
      _debounceTimer?.cancel();
      _debounceTimer = null;
    });
    return const SearchState();
  }

  void setQuery(String query) {
    state = state.copyWith(query: query);
    if (query.length >= 2) {
      _debounceSearch();
    } else {
      _cancelDebounce();
      state = state.copyWith(results: []);
    }
  }

  void _debounceSearch() {
    _debounceTimer?.cancel();
    _debounceTimer = Timer(const Duration(milliseconds: 300), _performSearch);
  }

  void _cancelDebounce() {
    _debounceTimer?.cancel();
    _debounceTimer = null;
  }

  void togglePublic() {
    state = state.copyWith(searchPublic: !state.searchPublic);
    if (state.query.length >= 2) _performSearch();
  }

  void toggleInternal() {
    state = state.copyWith(searchInternal: !state.searchInternal);
    if (state.query.length >= 2) _performSearch();
  }

  void toggleSensitive() {
    state = state.copyWith(searchSensitive: !state.searchSensitive);
    if (state.query.length >= 2) _performSearch();
  }

  void toggleRestricted() {
    state = state.copyWith(searchRestricted: !state.searchRestricted);
    if (state.query.length >= 2) _performSearch();
  }

  bool isFieldRevealed(String fieldPath, SensitivityLevel level) {
    final style = ref.read(accountStyleProvider).value;
    if (style == null || !style.revealedFields.contains(fieldPath)) return false;
    if (level == SensitivityLevel.critical) {
      return ref.read(isSensitiveAccessGrantedProvider);
    }
    return true;
  }

  Future<void> revealFieldWithContext(
    BuildContext context,
    WidgetRef ref,
    SensitivityLevel level,
    String fieldPath,
  ) async {
    if (level == SensitivityLevel.critical) {
      if (!ref.read(isSensitiveAccessGrantedProvider)) {
        final password = await showPasswordVerificationDialog(
          context: context,
          ref: ref,
          message: 'Restricted field. Enter your master password to view.',
          onVerify: (password) async {
            final authNotifier = ref.read(authNotifierProvider.notifier);
            return authNotifier.verifyPasswordForSensitiveData(password);
          },
        );
        if (password == null) return;
        ref.read(sensitivePageAccessProvider.notifier).markVerified();
      }
    }

    ref.read(accountStyleProvider.notifier).revealField(fieldPath);
  }

  Future<void> unlockAllRestricted(
    BuildContext context,
    WidgetRef ref,
  ) async {
    if (!ref.read(isSensitiveAccessGrantedProvider)) {
      final password = await showPasswordVerificationDialog(
        context: context,
        ref: ref,
        message: 'Restricted field. Enter your master password to view.',
        onVerify: (password) async {
          final authNotifier = ref.read(authNotifierProvider.notifier);
          return authNotifier.verifyPasswordForSensitiveData(password);
        },
      );
      if (password == null) return;
      ref.read(sensitivePageAccessProvider.notifier).markVerified();
    }

    final sensitiveNotifier = ref.read(accountStyleProvider.notifier);
    for (final result in state.results) {
      if (result.sensitivityLevel == SensitivityLevel.critical) {
        sensitiveNotifier.revealField(result.fieldPath);
      }
    }
  }

  Future<void> _performSearch() async {
    if (state.query.isEmpty) {
      state = state.copyWith(results: []);
      return;
    }

    state = state.copyWith(isSearching: true);

    final profileAsync = ref.read(profileNotifierProvider);
    final profile = profileAsync.value;
    if (profile == null) {
      state = state.copyWith(results: [], isSearching: false);
      return;
    }

    final results = await Isolate.run(() => _executeSearch(
      profile,
      state.query,
      state.searchPublic,
      state.searchInternal,
      state.searchSensitive,
      state.searchRestricted,
    ));

    state = state.copyWith(results: results, isSearching: false);
  }

  /// Pure search function — runs in a background isolate.
  static List<SearchResultItem> _executeSearch(
    ProfileData profile,
    String query,
    bool searchPublic,
    bool searchInternal,
    bool searchSensitive,
    bool searchRestricted,
  ) {
    final results = <SearchResultItem>[];
    final lowerQuery = query.toLowerCase();

    void addResult(
      String fieldPath,
      String fieldName,
      String section,
      String value,
      SensitivityLevel level, {
      bool isDeleted = false,
    }) {
      if (!value.toLowerCase().contains(lowerQuery) &&
          !fieldName.toLowerCase().contains(lowerQuery)) {
        return;
      }

      switch (level) {
        case SensitivityLevel.public:
          if (!searchPublic) return;
          break;
        case SensitivityLevel.internal:
          if (!searchInternal) return;
          break;
        case SensitivityLevel.sensitive:
          if (!searchSensitive) return;
          break;
        case SensitivityLevel.critical:
          if (!searchRestricted) return;
          break;
      }

      results.add(
        SearchResultItem(
          fieldPath: fieldPath,
          fieldName: fieldName,
          section: section,
          sectionDisplayName: FieldRegistry.getSectionDisplayName(section),
          value: value,
          sensitivityLevel: level,
          isDeleted: isDeleted,
        ),
      );
    }

    _searchIdentity(profile, addResult);
    _searchTravel(profile, addResult);
    _searchFinancial(profile, addResult);
    _searchProfessional(profile, addResult);
    _searchUnifiedObjects(profile, addResult);

    return results;
  }

  static void _searchIdentity(
    ProfileData profile,
    void Function(
      String fieldPath,
      String fieldName,
      String section,
      String value,
      SensitivityLevel level, {
      bool isDeleted,
    }) addResult,
  ) {
    final identity = profile.identity;
    if (identity == null) return;

    if (identity.fullName != null) {
      addResult(
        'identity.fullName',
        'Full Name',
        'identity',
        identity.fullName!,
        SensitivityLevel.public,
      );
    }
    if (identity.givenName != null) {
      addResult(
        'identity.givenName',
        'Given Name',
        'identity',
        identity.givenName!,
        SensitivityLevel.public,
      );
    }
    if (identity.familyName != null) {
      addResult(
        'identity.familyName',
        'Family Name',
        'identity',
        identity.familyName!,
        SensitivityLevel.public,
      );
    }
    if (identity.dateOfBirth != null) {
      addResult(
        'identity.dateOfBirth',
        'Date of Birth',
        'identity',
        identity.dateOfBirth!,
        SensitivityLevel.internal,
      );
    }
    if (identity.gender != null) {
      addResult(
        'identity.gender',
        'Gender',
        'identity',
        identity.gender!,
        SensitivityLevel.public,
      );
    }
    if (identity.nationality != null) {
      addResult(
        'identity.nationality',
        'Nationality',
        'identity',
        identity.nationality!,
        SensitivityLevel.internal,
      );
    }

    if (identity.contact?.entries != null) {
      for (final entry in identity.contact!.entries) {
        if (!entry.isDeleted) {
          addResult(
            'contact.${entry.id}',
            entry.title,
            'contact',
            entry.value,
            SensitivityLevel.internal,
          );
        }
      }
    }

    if (identity.idCards != null) {
      for (final card in identity.idCards!) {
        if (!card.isDeleted) {
          if (card.title != null) {
            addResult(
              'idCard.title.${card.id}',
              'ID Card Label',
              'idCard',
              card.title!,
              SensitivityLevel.internal,
            );
          }
          if (card.number != null) {
            addResult(
              'idCard.number.${card.id}',
              'ID Card Number',
              'idCard',
              card.number!,
              SensitivityLevel.critical,
            );
          }
          if (card.holderName != null) {
            addResult(
              'idCard.holderName.${card.id}',
              'Holder Name',
              'idCard',
              card.holderName!,
              SensitivityLevel.internal,
            );
          }
          if (card.country != null) {
            addResult(
              'idCard.country.${card.id}',
              'Country',
              'idCard',
              card.country!,
              SensitivityLevel.public,
            );
          }
        }
      }
    }

    if (identity.addresses != null) {
      for (final addr in identity.addresses!) {
        if (!addr.isDeleted) {
          if (addr.title != null) {
            addResult(
              'address.title.${addr.id}',
              'Address Label',
              'address',
              addr.title!,
              SensitivityLevel.internal,
            );
          }
          if (addr.street != null) {
            addResult(
              'address.street.${addr.id}',
              'Street',
              'address',
              addr.street!,
              SensitivityLevel.internal,
            );
          }
          if (addr.city != null) {
            addResult(
              'address.city.${addr.id}',
              'City',
              'address',
              addr.city!,
              SensitivityLevel.public,
            );
          }
          if (addr.state != null) {
            addResult(
              'address.state.${addr.id}',
              'State/Province',
              'address',
              addr.state!,
              SensitivityLevel.public,
            );
          }
          if (addr.postalCode != null) {
            addResult(
              'address.postalCode.${addr.id}',
              'Postal Code',
              'address',
              addr.postalCode!,
              SensitivityLevel.internal,
            );
          }
          if (addr.country != null) {
            addResult(
              'address.country.${addr.id}',
              'Country',
              'address',
              addr.country!,
              SensitivityLevel.public,
            );
          }
        }
      }
    }
  }

  static void _searchTravel(
    ProfileData profile,
    void Function(
      String fieldPath,
      String fieldName,
      String section,
      String value,
      SensitivityLevel level, {
      bool isDeleted,
    }) addResult,
  ) {
    final travel = profile.travel;
    if (travel == null) return;

    for (final passport in travel.passports) {
      if (!passport.isDeleted) {
        final number = passport.number;
        if (number != null) {
          addResult(
            'passport.number.${passport.id}',
            'Passport Number',
            'passport',
            number,
            SensitivityLevel.critical,
          );
        }
        final country = passport.country;
        if (country != null) {
          addResult(
            'passport.country.${passport.id}',
            'Country',
            'passport',
            country,
            SensitivityLevel.public,
          );
        }
        final holderName = passport.holderName;
        if (holderName != null) {
          addResult(
            'passport.holderName.${passport.id}',
            'Holder Name',
            'passport',
            holderName,
            SensitivityLevel.internal,
          );
        }
      }
    }

    for (final visa in travel.visas) {
      if (!visa.isDeleted) {
        final number = visa.number;
        if (number != null) {
          addResult(
            'visa.number.${visa.id}',
            'Visa Number',
            'visa',
            number,
            SensitivityLevel.critical,
          );
        }
        final country = visa.country;
        if (country != null) {
          addResult(
            'visa.country.${visa.id}',
            'Country',
            'visa',
            country,
            SensitivityLevel.public,
          );
        }
        final visaType = visa.visaType;
        if (visaType != null) {
          addResult(
            'visa.visaType.${visa.id}',
            'Visa Type',
            'visa',
            visaType,
            SensitivityLevel.internal,
          );
        }
      }
    }

    for (final history in travel.travelHistory) {
      if (!history.isDeleted) {
        addResult(
          'travelHistory.destination.${history.id}',
          'Destination',
          'travelHistory',
          history.destination,
          SensitivityLevel.public,
        );
      }
    }
  }

  static void _searchFinancial(
    ProfileData profile,
    void Function(
      String fieldPath,
      String fieldName,
      String section,
      String value,
      SensitivityLevel level, {
      bool isDeleted,
    }) addResult,
  ) {
    final financial = profile.financial;
    if (financial == null) return;

    for (final account in financial.bankAccounts) {
      if (!account.isDeleted) {
        final bankName = account.bankName;
        if (bankName != null) {
          addResult(
            'bankAccount.bankName.${account.id}',
            'Bank Name',
            'bankAccount',
            bankName,
            SensitivityLevel.public,
          );
        }
        final accountNumber = account.accountNumber;
        if (accountNumber != null) {
          addResult(
            'bankAccount.accountNumber.${account.id}',
            'Account Number',
            'bankAccount',
            accountNumber,
            SensitivityLevel.critical,
          );
        }
        final swiftBic = account.swiftBic;
        if (swiftBic != null) {
          addResult(
            'bankAccount.swiftBic.${account.id}',
            'SWIFT/BIC',
            'bankAccount',
            swiftBic,
            SensitivityLevel.critical,
          );
        }
        final currency = account.currency;
        if (currency != null) {
          addResult(
            'bankAccount.currency.${account.id}',
            'Currency',
            'bankAccount',
            currency,
            SensitivityLevel.public,
          );
        }
      }
    }

    for (final card in financial.cards) {
      if (!card.isDeleted) {
        final cardType = card.cardType;
        if (cardType != null) {
          addResult(
            'card.cardType.${card.id}',
            'Card Type',
            'card',
            cardType,
            SensitivityLevel.public,
          );
        }
        final cardNumber = card.cardNumber;
        if (cardNumber != null) {
          addResult(
            'card.cardNumber.${card.id}',
            'Card Number',
            'card',
            cardNumber,
            SensitivityLevel.critical,
          );
        }
        final holderName = card.holderName;
        if (holderName != null) {
          addResult(
            'card.holderName.${card.id}',
            'Holder Name',
            'card',
            holderName,
            SensitivityLevel.internal,
          );
        }
      }
    }

    for (final taxId in financial.taxIds) {
      if (!taxId.isDeleted) {
        final taxIdType = taxId.taxIdType;
        if (taxIdType != null) {
          addResult(
            'taxId.taxIdType.${taxId.id}',
            'Tax ID Type',
            'taxId',
            taxIdType,
            SensitivityLevel.internal,
          );
        }
        final taxIdNumber = taxId.taxIdNumber;
        if (taxIdNumber != null) {
          addResult(
            'taxId.taxIdNumber.${taxId.id}',
            'Tax ID Number',
            'taxId',
            taxIdNumber,
            SensitivityLevel.critical,
          );
        }
      }
    }
  }

  static void _searchProfessional(
    ProfileData profile,
    void Function(
      String fieldPath,
      String fieldName,
      String section,
      String value,
      SensitivityLevel level, {
      bool isDeleted,
    }) addResult,
  ) {
    final professional = profile.professional;
    if (professional == null) return;

    for (final edu in professional.education) {
      if (!edu.isDeleted) {
        final institution = edu.institution;
        if (institution != null) {
          addResult(
            'education.institution.${edu.id}',
            'Institution',
            'education',
            institution,
            SensitivityLevel.public,
          );
        }
        final degree = edu.degree;
        if (degree != null) {
          addResult(
            'education.degree.${edu.id}',
            'Degree',
            'education',
            degree,
            SensitivityLevel.public,
          );
        }
        final field = edu.field;
        if (field != null) {
          addResult(
            'education.field.${edu.id}',
            'Field of Study',
            'education',
            field,
            SensitivityLevel.public,
          );
        }
      }
    }

    for (final emp in professional.employment) {
      if (!emp.isDeleted) {
        final company = emp.company;
        if (company != null) {
          addResult(
            'employment.company.${emp.id}',
            'Company',
            'employment',
            company,
            SensitivityLevel.public,
          );
        }
        final position = emp.position;
        if (position != null) {
          addResult(
            'employment.position.${emp.id}',
            'Position',
            'employment',
            position,
            SensitivityLevel.public,
          );
        }
      }
    }

    for (final skill in professional.skills) {
      if (!skill.isDeleted) {
        addResult(
          'skills.name.${skill.id}',
          'Skill Name',
          'skills',
          skill.name,
          SensitivityLevel.public,
        );
      }
    }

    for (final lang in professional.languages) {
      if (!lang.isDeleted) {
        addResult(
          'languages.name.${lang.id}',
          'Language',
          'languages',
          lang.name,
          SensitivityLevel.public,
        );
      }
    }
  }

  static void _searchUnifiedObjects(
    ProfileData profile,
    void Function(
      String fieldPath,
      String fieldName,
      String section,
      String value,
      SensitivityLevel level, {
      bool isDeleted,
    }) addResult,
  ) {
    final data = profile.unifiedObjects;
    if (data == null) return;

    // Build a lookup map for parent objects
    final objectMap = {for (final o in data.objects) o.id: o};

    // Search page names
    for (final obj in data.objects) {
      if (obj.isDeleted) continue;
      if (obj.typeId != 'page') continue;

      addResult(
        'page.${obj.id}.name',
        'Page Name',
        'page',
        obj.name,
        SensitivityLevel.public,
      );
    }

    // Search item-level object properties
    for (final obj in data.objects) {
      if (obj.isDeleted) continue;
      if (obj.typeId != 'item') continue;

      final parent = obj.parentId != null ? objectMap[obj.parentId] : null;
      final sectionName = parent?.name ?? 'Custom';

      for (final entry in obj.properties.entries) {
        final prop = entry.value;
        final valueStr = switch (prop) {
          TextProperty(:final text) => text,
          NumberProperty(:final value) => value?.toString() ?? '',
          DateProperty(:final isoDate) => isoDate ?? '',
          CheckboxProperty(:final checked) => checked ? 'Yes' : 'No',
          SelectProperty(:final selectedId) => selectedId ?? '',
          MultiSelectProperty(:final selectedIds) => selectedIds.join(', '),
          RelationProperty(:final targetObjectId) => targetObjectId ?? '',
          UrlProperty(:final url) => url ?? '',
        };
        if (valueStr.isEmpty) continue;

        addResult(
          'unifiedObject.${obj.id}.${entry.key}',
          entry.key,
          sectionName,
          valueStr,
          prop.sensitivity,
        );
      }
    }
  }
}

/// Search provider
final searchProvider = NotifierProvider<SearchNotifier, SearchState>(() {
  return SearchNotifier();
});
