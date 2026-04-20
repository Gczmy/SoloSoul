import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart';
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    hide SensitivityLevel;
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart';

/// Search result item
class SearchResultItem {
  final String fieldPath;
  final String fieldName;
  final String section;
  final String sectionDisplayName;
  final String value;
  final SensitivityLevel sensitivityLevel;
  final bool isDeleted;

  const SearchResultItem({
    required this.fieldPath,
    required this.fieldName,
    required this.section,
    required this.sectionDisplayName,
    required this.value,
    required this.sensitivityLevel,
    this.isDeleted = false,
  });
}

/// Search state
class SearchState {
  final String query;
  final bool searchPublic;
  final bool searchPrivate;
  final bool searchRestricted;
  final List<SearchResultItem> results;
  final bool isSearching;

  const SearchState({
    this.query = '',
    this.searchPublic = true,
    this.searchPrivate = false,
    this.searchRestricted = false,
    this.results = const [],
    this.isSearching = false,
  });

  SearchState copyWith({
    String? query,
    bool? searchPublic,
    bool? searchPrivate,
    bool? searchRestricted,
    List<SearchResultItem>? results,
    bool? isSearching,
  }) {
    return SearchState(
      query: query ?? this.query,
      searchPublic: searchPublic ?? this.searchPublic,
      searchPrivate: searchPrivate ?? this.searchPrivate,
      searchRestricted: searchRestricted ?? this.searchRestricted,
      results: results ?? this.results,
      isSearching: isSearching ?? this.isSearching,
    );
  }

  bool get hasActiveFilters =>
      searchPublic || searchPrivate || searchRestricted;
}

/// Search notifier
class SearchNotifier extends StateNotifier<SearchState> {
  final Ref _ref;

  SearchNotifier(this._ref) : super(const SearchState());

  void setQuery(String query) {
    state = state.copyWith(query: query);
    if (query.length >= 2) {
      _performSearch();
    } else {
      state = state.copyWith(results: []);
    }
  }

  void togglePublic() {
    state = state.copyWith(searchPublic: !state.searchPublic);
    if (state.query.length >= 2) _performSearch();
  }

  void togglePrivate() {
    state = state.copyWith(searchPrivate: !state.searchPrivate);
    if (state.query.length >= 2) _performSearch();
  }

  void toggleRestricted() {
    state = state.copyWith(searchRestricted: !state.searchRestricted);
    if (state.query.length >= 2) _performSearch();
  }

  bool isFieldRevealed(String fieldPath, SensitivityLevel level) {
    // Check if field is revealed in sensitivity settings
    final settings = _ref.read(sensitivitySettingsProvider);
    if (!settings.isFieldRevealed(fieldPath)) return false;
    // Only restricted fields require re-verification after 5-min lock
    if (level == SensitivityLevel.critical) {
      final sensitiveAccess = _ref.read(sensitivePageAccessProvider);
      if (!sensitiveAccess.isVerified) return false;
      return sensitiveAccess.lastVerified != null &&
          DateTime.now().difference(sensitiveAccess.lastVerified!).inMinutes < 5;
    }
    return true;
  }

  Future<void> revealFieldWithContext(
    BuildContext context,
    WidgetRef ref,
    SensitivityLevel level,
    String fieldPath,
  ) async {
    // Only restricted fields require password verification
    if (level == SensitivityLevel.critical) {
      final sensitiveAccess = ref.read(sensitivePageAccessProvider);
      if (!sensitiveAccess.isVerified) {
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

    // Reveal this specific field via shared settings
    ref.read(sensitivitySettingsProvider.notifier).revealField(fieldPath);
  }

  Future<void> unlockAllRestricted(
    BuildContext context,
    WidgetRef ref,
  ) async {
    final sensitiveAccess = ref.read(sensitivePageAccessProvider);
    if (!sensitiveAccess.isVerified) {
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

    // Reveal all restricted fields in results
    final sensitiveNotifier = ref.read(sensitivitySettingsProvider.notifier);
    for (final result in state.results) {
      if (result.sensitivityLevel == SensitivityLevel.critical) {
        sensitiveNotifier.revealField(result.fieldPath);
      }
    }
  }

  void _performSearch() {
    if (state.query.isEmpty) {
      state = state.copyWith(results: []);
      return;
    }

    state = state.copyWith(isSearching: true);

    final profile = _ref.read(profileNotifierProvider);
    if (profile == null) {
      state = state.copyWith(results: [], isSearching: false);
      return;
    }

    final results = <SearchResultItem>[];
    final query = state.query.toLowerCase();

    void addResult(
      String fieldPath,
      String fieldName,
      String section,
      String value,
      SensitivityLevel level, {
      bool isDeleted = false,
    }) {
      // Check if field matches query
      if (!value.toLowerCase().contains(query) &&
          !fieldName.toLowerCase().contains(query)) {
        return;
      }

      // Check sensitivity filter
      switch (level) {
        case SensitivityLevel.public:
          if (!state.searchPublic) return;
          break;
        case SensitivityLevel.internal:
        case SensitivityLevel.sensitive:
          if (!state.searchPrivate) return;
          break;
        case SensitivityLevel.critical:
          if (!state.searchRestricted) return;
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

    // Search Identity
    final identity = profile.identity;
    if (identity != null) {
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

      // Contact entries
      if (identity.contact?.entries != null) {
        for (final entry in identity.contact!.entries) {
          if (!entry.isDeleted) {
            addResult(
              'contact.${entry.id}',
              entry.label,
              'contact',
              entry.value,
              SensitivityLevel.internal,
            );
          }
        }
      }

      // ID Cards
      if (identity.idCards != null) {
        for (final card in identity.idCards!) {
          if (!card.isDeleted) {
            if (card.label != null) {
              addResult(
                'idCard.label.${card.id}',
                'ID Card Label',
                'idCard',
                card.label!,
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

      // Addresses
      if (identity.addresses != null) {
        for (final addr in identity.addresses!) {
          if (!addr.isDeleted) {
            if (addr.label != null) {
              addResult(
                'address.label.${addr.id}',
                'Address Label',
                'address',
                addr.label!,
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

    // Search Travel
    final travel = profile.travel;
    if (travel != null) {
      // Passports
      if (travel.passports != null) {
        for (final passport in travel.passports!) {
          if (!passport.isDeleted) {
            if (passport.number != null) {
              addResult(
                'passport.number.${passport.id}',
                'Passport Number',
                'passport',
                passport.number!,
                SensitivityLevel.critical,
              );
            }
            if (passport.country != null) {
              addResult(
                'passport.country.${passport.id}',
                'Country',
                'passport',
                passport.country!,
                SensitivityLevel.public,
              );
            }
            if (passport.holderName != null) {
              addResult(
                'passport.holderName.${passport.id}',
                'Holder Name',
                'passport',
                passport.holderName!,
                SensitivityLevel.internal,
              );
            }
          }
        }
      }

      // Visas
      if (travel.visas != null) {
        for (final visa in travel.visas!) {
          if (!visa.isDeleted) {
            if (visa.number != null) {
              addResult(
                'visa.number.${visa.id}',
                'Visa Number',
                'visa',
                visa.number!,
                SensitivityLevel.critical,
              );
            }
            if (visa.country != null) {
              addResult(
                'visa.country.${visa.id}',
                'Country',
                'visa',
                visa.country!,
                SensitivityLevel.public,
              );
            }
            if (visa.visaType != null) {
              addResult(
                'visa.visaType.${visa.id}',
                'Visa Type',
                'visa',
                visa.visaType!,
                SensitivityLevel.internal,
              );
            }
          }
        }
      }

      // Travel History
      if (travel.travelHistory != null) {
        for (final history in travel.travelHistory!) {
          if (!history.isDeleted) {
            if (history.destination != null) {
              addResult(
                'travelHistory.destination.${history.id}',
                'Destination',
                'travelHistory',
                history.destination!,
                SensitivityLevel.public,
              );
            }
          }
        }
      }
    }

    // Search Financial
    final financial = profile.financial;
    if (financial != null) {
      // Bank Accounts
      if (financial.bankAccounts != null) {
        for (final account in financial.bankAccounts!) {
          if (!account.isDeleted) {
            if (account.bankName != null) {
              addResult(
                'bankAccount.bankName.${account.id}',
                'Bank Name',
                'bankAccount',
                account.bankName!,
                SensitivityLevel.public,
              );
            }
            if (account.accountNumber != null) {
              addResult(
                'bankAccount.accountNumber.${account.id}',
                'Account Number',
                'bankAccount',
                account.accountNumber!,
                SensitivityLevel.critical,
              );
            }
            if (account.swiftBic != null) {
              addResult(
                'bankAccount.swiftBic.${account.id}',
                'SWIFT/BIC',
                'bankAccount',
                account.swiftBic!,
                SensitivityLevel.critical,
              );
            }
            if (account.currency != null) {
              addResult(
                'bankAccount.currency.${account.id}',
                'Currency',
                'bankAccount',
                account.currency!,
                SensitivityLevel.public,
              );
            }
          }
        }
      }

      // Cards
      if (financial.cards != null) {
        for (final card in financial.cards!) {
          if (!card.isDeleted) {
            if (card.cardType != null) {
              addResult(
                'card.cardType.${card.id}',
                'Card Type',
                'card',
                card.cardType!,
                SensitivityLevel.public,
              );
            }
            if (card.cardNumber != null) {
              addResult(
                'card.cardNumber.${card.id}',
                'Card Number',
                'card',
                card.cardNumber!,
                SensitivityLevel.critical,
              );
            }
            if (card.holderName != null) {
              addResult(
                'card.holderName.${card.id}',
                'Holder Name',
                'card',
                card.holderName!,
                SensitivityLevel.internal,
              );
            }
          }
        }
      }

      // Tax IDs
      if (financial.taxIds != null) {
        for (final taxId in financial.taxIds!) {
          if (!taxId.isDeleted) {
            if (taxId.taxIdType != null) {
              addResult(
                'taxId.taxIdType.${taxId.id}',
                'Tax ID Type',
                'taxId',
                taxId.taxIdType!,
                SensitivityLevel.internal,
              );
            }
            if (taxId.taxIdNumber != null) {
              addResult(
                'taxId.taxIdNumber.${taxId.id}',
                'Tax ID Number',
                'taxId',
                taxId.taxIdNumber!,
                SensitivityLevel.critical,
              );
            }
          }
        }
      }
    }

    // Search Professional
    final professional = profile.professional;
    if (professional != null) {
      // Education
      if (professional.education != null) {
        for (final edu in professional.education!) {
          if (!edu.isDeleted) {
            if (edu.institution != null) {
              addResult(
                'education.institution.${edu.id}',
                'Institution',
                'education',
                edu.institution!,
                SensitivityLevel.public,
              );
            }
            if (edu.degree != null) {
              addResult(
                'education.degree.${edu.id}',
                'Degree',
                'education',
                edu.degree!,
                SensitivityLevel.public,
              );
            }
            if (edu.field != null) {
              addResult(
                'education.field.${edu.id}',
                'Field of Study',
                'education',
                edu.field!,
                SensitivityLevel.public,
              );
            }
          }
        }
      }

      // Employment
      if (professional.employment != null) {
        for (final emp in professional.employment!) {
          if (!emp.isDeleted) {
            if (emp.company != null) {
              addResult(
                'employment.company.${emp.id}',
                'Company',
                'employment',
                emp.company!,
                SensitivityLevel.public,
              );
            }
            if (emp.position != null) {
              addResult(
                'employment.position.${emp.id}',
                'Position',
                'employment',
                emp.position!,
                SensitivityLevel.public,
              );
            }
          }
        }
      }

      // Skills
      if (professional.skills != null) {
        for (final skill in professional.skills!) {
          if (!skill.isDeleted) {
            if (skill.name != null) {
              addResult(
                'skills.name.${skill.id}',
                'Skill Name',
                'skills',
                skill.name!,
                SensitivityLevel.public,
              );
            }
          }
        }
      }

      // Languages
      if (professional.languages != null) {
        for (final lang in professional.languages!) {
          if (!lang.isDeleted) {
            if (lang.name != null) {
              addResult(
                'languages.name.${lang.id}',
                'Language',
                'languages',
                lang.name!,
                SensitivityLevel.public,
              );
            }
          }
        }
      }
    }

    state = state.copyWith(results: results, isSearching: false);
  }
}

/// Search provider
final searchProvider = StateNotifierProvider<SearchNotifier, SearchState>((
  ref,
) {
  return SearchNotifier(ref);
});

/// Search Page
class SearchPage extends ConsumerStatefulWidget {
  const SearchPage({super.key});

  @override
  ConsumerState<SearchPage> createState() => _SearchPageState();
}

class _SearchPageState extends ConsumerState<SearchPage> {
  final _searchController = TextEditingController();
  final _focusNode = FocusNode();

  @override
  void initState() {
    super.initState();
    // Auto-focus the search field
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _focusNode.requestFocus();
    });
  }

  @override
  void dispose() {
    _searchController.dispose();
    _focusNode.dispose();
    super.dispose();
  }

  void _showHistorySheet(BuildContext context, WidgetRef ref) {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      builder: (context) => DraggableScrollableSheet(
        initialChildSize: 0.7,
        minChildSize: 0.5,
        maxChildSize: 0.95,
        expand: false,
        builder: (context, scrollController) {
          return _HistorySheet(scrollController: scrollController, ref: ref);
        },
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final searchState = ref.watch(searchProvider);
    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Search'),
        actions: [
          IconButton(
            icon: const Icon(Icons.history),
            tooltip: 'Field History',
            onPressed: () => _showHistorySheet(context, ref),
          ),
        ],
      ),
      body: Column(
        children: [
          // Search input
          Padding(
            padding: const EdgeInsets.all(16),
            child: TextField(
              controller: _searchController,
              focusNode: _focusNode,
              decoration: InputDecoration(
                hintText: 'Search fields...',
                prefixIcon: const Icon(Icons.search),
                suffixIcon: _searchController.text.isNotEmpty
                    ? IconButton(
                        icon: const Icon(Icons.clear),
                        onPressed: () {
                          _searchController.clear();
                          ref.read(searchProvider.notifier).setQuery('');
                        },
                      )
                    : null,
                border: OutlineInputBorder(
                  borderRadius: BorderRadius.circular(12),
                ),
              ),
              onChanged: (value) {
                ref.read(searchProvider.notifier).setQuery(value);
              },
            ),
          ),

          // Sensitivity filter checkboxes
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: Row(
              children: [
                FilterChip(
                  label: const Text('Public'),
                  selected: searchState.searchPublic,
                  onSelected: (_) {
                    ref.read(searchProvider.notifier).togglePublic();
                  },
                  avatar: searchState.searchPublic
                      ? const Icon(Icons.check, size: 18)
                      : null,
                ),
                const SizedBox(width: 8),
                FilterChip(
                  label: const Text('Private'),
                  selected: searchState.searchPrivate,
                  onSelected: (_) {
                    ref.read(searchProvider.notifier).togglePrivate();
                  },
                  avatar: searchState.searchPrivate
                      ? const Icon(Icons.check, size: 18)
                      : null,
                ),
                const SizedBox(width: 8),
                FilterChip(
                  label: const Text('Restricted'),
                  selected: searchState.searchRestricted,
                  onSelected: (_) {
                    ref.read(searchProvider.notifier).toggleRestricted();
                  },
                  avatar: searchState.searchRestricted
                      ? const Icon(Icons.check, size: 18)
                      : null,
                ),
                const Spacer(),
                if (searchState.searchRestricted)
                  TextButton.icon(
                    icon: const Icon(Icons.lock_open, size: 18),
                    label: const Text('Unlock'),
                    onPressed: () {
                      ref
                          .read(searchProvider.notifier)
                          .unlockAllRestricted(context, ref);
                    },
                  ),
              ],
            ),
          ),

          const Divider(height: 24),

          // Results
          Expanded(child: _buildResults(searchState, theme)),
        ],
      ),
    );
  }

  Widget _buildResults(SearchState searchState, ThemeData theme) {
    if (searchState.query.isEmpty) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(Icons.search, size: 64, color: theme.colorScheme.outline),
            const SizedBox(height: 16),
            Text(
              'Enter at least 2 characters to search',
              style: theme.textTheme.bodyLarge?.copyWith(
                color: theme.colorScheme.outline,
              ),
            ),
          ],
        ),
      );
    }

    if (searchState.isSearching) {
      return const Center(child: CircularProgressIndicator());
    }

    if (searchState.results.isEmpty) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(Icons.search_off, size: 64, color: theme.colorScheme.outline),
            const SizedBox(height: 16),
            Text(
              'No results found',
              style: theme.textTheme.bodyLarge?.copyWith(
                color: theme.colorScheme.outline,
              ),
            ),
            const SizedBox(height: 8),
            Text(
              'Try adjusting your filters or search terms',
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.outline,
              ),
            ),
          ],
        ),
      );
    }

    // Group results by section
    final groupedResults = <String, List<SearchResultItem>>{};
    for (final result in searchState.results) {
      groupedResults.putIfAbsent(result.sectionDisplayName, () => []);
      groupedResults[result.sectionDisplayName]!.add(result);
    }

    return ListView.builder(
      padding: const EdgeInsets.symmetric(horizontal: 16),
      itemCount: groupedResults.length,
      itemBuilder: (context, index) {
        final sectionName = groupedResults.keys.elementAt(index);
        final sectionResults = groupedResults[sectionName]!;

        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Padding(
              padding: const EdgeInsets.symmetric(vertical: 8),
              child: Text(
                sectionName,
                style: theme.textTheme.titleSmall?.copyWith(
                  color: AppTheme.primaryColor,
                  fontWeight: FontWeight.bold,
                ),
              ),
            ),
            ...sectionResults.map(
              (result) => _SearchResultTile(
                result: result,
                onReveal: () {
                  ref
                      .read(searchProvider.notifier)
                      .revealFieldWithContext(context, ref, result.sensitivityLevel, result.fieldPath);
                },
              ),
            ),
            const SizedBox(height: 8),
          ],
        );
      },
    );
  }
}

class _SearchResultTile extends ConsumerWidget {
  final SearchResultItem result;
  final VoidCallback onReveal;

  const _SearchResultTile({
    required this.result,
    required this.onReveal,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    // Watch sensitivitySettingsProvider so rebuild happens when fields are revealed
    ref.watch(sensitivitySettingsProvider);
    final isRevealed = ref
        .read(searchProvider.notifier)
        .isFieldRevealed(result.fieldPath, result.sensitivityLevel);
    final showMasked =
        result.sensitivityLevel != SensitivityLevel.public && !isRevealed;

    return Card(
      margin: const EdgeInsets.only(bottom: 8),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Expanded(
                  child: Text(
                    result.fieldName,
                    style: theme.textTheme.bodyMedium?.copyWith(
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                ),
                SensitivityTag(level: result.sensitivityLevel),
                if (result.isDeleted) ...[
                  const SizedBox(width: 8),
                  Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 6,
                      vertical: 2,
                    ),
                    decoration: BoxDecoration(
                      color: Colors.grey.withValues(alpha: 0.1),
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Text(
                      'Deleted',
                      style: theme.textTheme.labelSmall?.copyWith(
                        color: Colors.grey,
                      ),
                    ),
                  ),
                ],
              ],
            ),
            const SizedBox(height: 8),
            Row(
              children: [
                Expanded(
                  child: Text(
                    showMasked ? '••••••••' : result.value,
                    style: theme.textTheme.bodyMedium?.copyWith(
                      color: showMasked
                          ? theme.colorScheme.outline
                          : theme.colorScheme.onSurface,
                      fontFamily: showMasked ? null : 'monospace',
                    ),
                  ),
                ),
                if (showMasked)
                  TextButton.icon(
                    icon: const Icon(Icons.visibility_off, size: 16),
                    label: const Text('Reveal'),
                    onPressed: onReveal,
                    style: TextButton.styleFrom(
                      padding: const EdgeInsets.symmetric(horizontal: 8),
                      minimumSize: Size.zero,
                      tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                    ),
                  ),
              ],
            ),
            if (showMasked)
              Padding(
                padding: const EdgeInsets.only(top: 4),
                child: Row(
                  children: [
                    Icon(
                      Icons.info_outline,
                      size: 14,
                      color: theme.colorScheme.outline,
                    ),
                    const SizedBox(width: 4),
                    Text(
                      result.sensitivityLevel == SensitivityLevel.critical
                          ? 'Restricted - password required to view'
                          : 'Private - reveal to view',
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.outline,
                      ),
                    ),
                  ],
                ),
              ),
          ],
        ),
      ),
    );
  }
}

class _HistorySheet extends StatelessWidget {
  final ScrollController scrollController;
  final WidgetRef ref;

  const _HistorySheet({required this.scrollController, required this.ref});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Container(
      decoration: BoxDecoration(
        color: theme.colorScheme.surface,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(16)),
      ),
      child: Column(
        children: [
          // Handle bar
          Container(
            margin: const EdgeInsets.only(top: 12),
            width: 40,
            height: 4,
            decoration: BoxDecoration(
              color: theme.colorScheme.outline.withValues(alpha: 0.4),
              borderRadius: BorderRadius.circular(2),
            ),
          ),
          // Title
          Padding(
            padding: const EdgeInsets.all(16),
            child: Row(
              children: [
                Icon(Icons.history, color: AppTheme.primaryColor),
                const SizedBox(width: 8),
                Text(
                  'Field History',
                  style: theme.textTheme.titleLarge?.copyWith(
                    fontWeight: FontWeight.bold,
                  ),
                ),
              ],
            ),
          ),
          const Divider(height: 1),
          // History list
          Expanded(
            child: FutureBuilder(
              future: ref.read(fieldHistoriesProvider.notifier).loadHistories(),
              builder: (context, snapshot) {
                if (snapshot.connectionState == ConnectionState.waiting) {
                  return const Center(child: CircularProgressIndicator());
                }

                final histories = ref.watch(fieldHistoriesProvider);
                final allChanges = <_HistoryChangeItem>[];

                // Collect all changes from all histories
                for (final itemEntry in histories.histories.entries) {
                  final itemId = itemEntry.key;
                  for (final fieldEntry in itemEntry.value.entries) {
                    final fieldId = fieldEntry.key;
                    final history = fieldEntry.value;
                    for (final entry in history.entries) {
                      allChanges.add(
                        _HistoryChangeItem(
                          itemId: itemId,
                          fieldId: fieldId,
                          values: entry.values,
                          timestamp: entry.timestamp,
                        ),
                      );
                    }
                  }
                }

                // Sort by timestamp (newest first)
                allChanges.sort((a, b) => b.timestamp.compareTo(a.timestamp));

                if (allChanges.isEmpty) {
                  return Center(
                    child: Column(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        Icon(
                          Icons.history,
                          size: 64,
                          color: theme.colorScheme.outline,
                        ),
                        const SizedBox(height: 16),
                        Text(
                          'No history yet',
                          style: theme.textTheme.bodyLarge?.copyWith(
                            color: theme.colorScheme.outline,
                          ),
                        ),
                      ],
                    ),
                  );
                }

                return ListView.builder(
                  controller: scrollController,
                  padding: const EdgeInsets.symmetric(horizontal: 16),
                  itemCount: allChanges.length,
                  itemBuilder: (context, index) {
                    final change = allChanges[index];
                    return _HistoryChangeTile(change: change, theme: theme);
                  },
                );
              },
            ),
          ),
        ],
      ),
    );
  }
}

class _HistoryChangeItem {
  final String itemId;
  final String fieldId;
  final Map<String, String> values;
  final DateTime timestamp;

  const _HistoryChangeItem({
    required this.itemId,
    required this.fieldId,
    required this.values,
    required this.timestamp,
  });
}

class _HistoryChangeTile extends StatelessWidget {
  final _HistoryChangeItem change;
  final ThemeData theme;

  const _HistoryChangeTile({required this.change, required this.theme});

  String _formatTimestamp(DateTime timestamp) {
    final now = DateTime.now();
    final diff = now.difference(timestamp);

    if (diff.inDays > 365) {
      return '${(diff.inDays / 365).floor()} year(s) ago';
    } else if (diff.inDays > 30) {
      return '${(diff.inDays / 30).floor()} month(s) ago';
    } else if (diff.inDays > 0) {
      return '${diff.inDays} day(s) ago';
    } else if (diff.inHours > 0) {
      return '${diff.inHours} hour(s) ago';
    } else if (diff.inMinutes > 0) {
      return '${diff.inMinutes} minute(s) ago';
    } else {
      return 'Just now';
    }
  }

  String _formatFullTimestamp(DateTime timestamp) {
    return '${timestamp.year}-${timestamp.month.toString().padLeft(2, '0')}-${timestamp.day.toString().padLeft(2, '0')} '
        '${timestamp.hour.toString().padLeft(2, '0')}:${timestamp.minute.toString().padLeft(2, '0')}';
  }

  @override
  Widget build(BuildContext context) {
    return Card(
      margin: const EdgeInsets.only(bottom: 8),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    change.fieldId,
                    style: theme.textTheme.bodyMedium?.copyWith(
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                  const SizedBox(height: 4),
                  Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: change.values.entries.map((e) {
                      return Padding(
                        padding: const EdgeInsets.only(bottom: 2),
                        child: Row(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            SizedBox(
                              width: 80,
                              child: Text(
                                e.key,
                                style: theme.textTheme.bodySmall?.copyWith(
                                  color: theme.colorScheme.onSurfaceVariant,
                                  fontWeight: FontWeight.w500,
                                ),
                              ),
                            ),
                            Expanded(
                              child: Text(
                                e.value.isNotEmpty ? e.value : '(empty)',
                                style: theme.textTheme.bodyMedium?.copyWith(
                                  fontStyle: e.value.isEmpty ? FontStyle.italic : null,
                                  color: e.value.isEmpty
                                      ? theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.6)
                                      : null,
                                ),
                              ),
                            ),
                          ],
                        ),
                      );
                    }).toList(),
                  ),
                ],
              ),
            ),
            const SizedBox(width: 8),
            Tooltip(
              message: _formatFullTimestamp(change.timestamp),
              child: Text(
                _formatTimestamp(change.timestamp),
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
