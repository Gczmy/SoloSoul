import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    hide SensitivityLevel;
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart';
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart' show FieldRegistry, FieldSensitivity, SensitivityLevel, formFieldRegistryProvider;
import 'package:solosoul_flutter/presentation/utils/auth_utils.dart';
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';

class SensitivitySettingsPage extends ConsumerStatefulWidget {
  const SensitivitySettingsPage({super.key});

  @override
  ConsumerState<SensitivitySettingsPage> createState() => _SensitivitySettingsPageState();
}

class _SensitivitySettingsPageState extends ConsumerState<SensitivitySettingsPage> {
  final _searchController = TextEditingController();
  String _searchQuery = '';
  bool _dialogShown = false;

  // Cached field computations to avoid re-sorting/filtering on every rebuild
  List<FieldSensitivity> _cachedEffectiveFields = const [];
  int _lastRegistryHash = 0;
  int _lastAccountStyleHash = 0;

  Map<SensitivityLevel, List<FieldSensitivity>> _cachedSections = const {};
  String _cachedSearchQuery = '';

  List<FieldSensitivity> _getEffectiveFields(
    Map<String, FieldSensitivity> registry,
    Map<String, SensitivityLevel> accountStyle,
  ) {
    final registryHash = registry.hashCode;
    final styleHash = accountStyle.hashCode;
    if (_cachedEffectiveFields.isNotEmpty &&
        _lastRegistryHash == registryHash &&
        _lastAccountStyleHash == styleHash) {
      return _cachedEffectiveFields;
    }

    final allFields = registry.values.toList();
    allFields.sort((a, b) {
      final sec = a.fieldSection.compareTo(b.fieldSection);
      return sec != 0 ? sec : a.fieldName.compareTo(b.fieldName);
    });

    _cachedEffectiveFields = allFields.map((field) {
      final overrideLevel = accountStyle[field.fieldId];
      return overrideLevel != null
          ? field.copyWith(level: overrideLevel)
          : field;
    }).toList();
    _lastRegistryHash = registryHash;
    _lastAccountStyleHash = styleHash;
    _cachedSections = const {}; // invalidate filtered cache
    return _cachedEffectiveFields;
  }

  Map<SensitivityLevel, List<FieldSensitivity>> _getFilteredSections(
    List<FieldSensitivity> effectiveFields,
    String searchQuery,
  ) {
    if (_cachedSections.isNotEmpty && _cachedSearchQuery == searchQuery) {
      return _cachedSections;
    }

    List<FieldSensitivity> filter(List<FieldSensitivity> fields) {
      if (searchQuery.isEmpty) return fields;
      final query = searchQuery.toLowerCase();
      return fields.where((f) {
        return f.fieldName.toLowerCase().contains(query) ||
            FieldRegistry.getSectionDisplayName(f.fieldSection).toLowerCase().contains(query);
      }).toList();
    }

    final public = filter(effectiveFields.where((f) => f.level == SensitivityLevel.public).toList());
    final internal = filter(effectiveFields.where((f) => f.level == SensitivityLevel.internal).toList());
    final sensitive = filter(effectiveFields.where((f) => f.level == SensitivityLevel.sensitive).toList());
    final critical = filter(effectiveFields.where((f) => f.level == SensitivityLevel.critical).toList());

    _cachedSearchQuery = searchQuery;
    _cachedSections = {
      SensitivityLevel.public: public,
      SensitivityLevel.internal: internal,
      SensitivityLevel.sensitive: sensitive,
      SensitivityLevel.critical: critical,
    };
    return _cachedSections;
  }

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  Future<void> _verifyPassword() async {
    _dialogShown = true;
    await verifyPasswordAndGrantAccess(
      context: context,
      ref: ref,
      message: 'Enter your master password to access sensitivity settings.',
    );
  }

  @override
  Widget build(BuildContext context) {
    if (!ref.watch(isSensitiveAccessGrantedProvider)) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (!_dialogShown) _verifyPassword();
      });
      return Scaffold(
        appBar: SoloGlassAppBar(title: Text(AppLocalizations.of(context).sensitivitySettingsTitle)),
        body: Center(
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Icon(
                Icons.lock_outline,
                size: 64,
                color: Theme.of(context).colorScheme.primary,
              ),
              const SizedBox(height: 24),
              Text(
                'Password Required',
                style: Theme.of(context).textTheme.headlineSmall,
              ),
              const SizedBox(height: 16),
              ElevatedButton(
                onPressed: _verifyPassword,
                child: Text(AppLocalizations.of(context).sensitivitySettingsVerify),
              ),
            ],
          ),
        ),
      );
    }
    final registry = ref.watch(formFieldRegistryProvider);
    final accountStyle = ref.watch(accountStyleProvider).value?.fieldSettings ?? {};
    final notifier = ref.read(accountStyleProvider.notifier);

    final effectiveFields = _getEffectiveFields(registry, accountStyle);
    final sections = _getFilteredSections(effectiveFields, _searchQuery);

    return _SensitivitySettingsView(
      searchController: _searchController,
      searchQuery: _searchQuery,
      onSearchChanged: (value) => setState(() => _searchQuery = value),
      onClearSearch: () {
        _searchController.clear();
        setState(() => _searchQuery = '');
      },
      effectiveFields: effectiveFields,
      sections: sections,
      onUpgrade: (fieldId) => notifier.upgradeField(fieldId),
      onDowngrade: (fieldId) => notifier.downgradeField(fieldId),
      onDowngradeCritical: (fieldId) => _showDowngradeConfirmation(context, ref, fieldId),
    );
  }


  void _showDowngradeConfirmation(
    BuildContext context,
    WidgetRef ref,
    String fieldId,
  ) {
    final accountStyle = ref.read(accountStyleProvider).value ?? const AccountStyle();
    final registry = ref.read(formFieldRegistryProvider);
    final field = registry[fieldId];
    if (field == null) return; // Field not found, shouldn't happen
    final effectiveLevel = accountStyle.fieldSettings[fieldId] ?? field.level;

    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: Row(
          children: [
            Icon(Icons.warning_amber, color: Colors.orange.shade700),
            const SizedBox(width: 8),
            Text(AppLocalizations.of(context).sensitivitySettingsConfirmDowngrade),
          ],
        ),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'You are about to downgrade "${field.fieldName}" to a lower sensitivity level.',
              style: Theme.of(context).textTheme.bodyMedium,
            ),
            const SizedBox(height: 12),
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: Colors.orange.shade50,
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: Colors.orange.shade200),
              ),
              child: Row(
                children: [
                  Icon(Icons.info_outline, color: Colors.orange.shade700, size: 20),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      'This field will be visible with fewer protections. Continue?',
                      style: TextStyle(
                        color: Colors.orange.shade900,
                        fontSize: 13,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: Text(AppLocalizations.of(context).commonCancel),
          ),
          FilledButton(
            onPressed: () {
              final newLevel = SensitivityLevel.values[effectiveLevel.index - 1];
              ref.read(accountStyleProvider.notifier).setFieldLevel(fieldId, newLevel);
              Navigator.pop(context);
              showOverlaySnackBar(
                context,
                content: '"${field.fieldName}" moved to Private',
                type: SnackBarType.info,
              );
            },
            style: FilledButton.styleFrom(
              backgroundColor: Colors.orange,
            ),
            child: Text(AppLocalizations.of(context).commonConfirm),
          ),
        ],
      ),
    );
  }

}

class _SensitivitySection extends StatelessWidget {
  final String title;
  final String subtitle;
  final IconData icon;
  final Color color;
  final List<FieldSensitivity> fields;
  final void Function(String fieldId)? onUpgrade;
  final void Function(String fieldId)? onDowngrade;
  final bool isHighest;
  final bool isLowest;

  const _SensitivitySection({
    required this.title,
    required this.subtitle,
    required this.icon,
    required this.color,
    required this.fields,
    this.onUpgrade,
    this.onDowngrade,
    this.isHighest = false,
    this.isLowest = false,
  });

  @override
  Widget build(BuildContext context) {
    return Card(
      margin: EdgeInsets.zero,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Section Header
            Row(
              children: [
                Container(
                  padding: const EdgeInsets.all(8),
                  decoration: BoxDecoration(
                    color: color.withValues(alpha: 0.1),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Icon(icon, color: color, size: 20),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          Text(
                            title,
                            style: Theme.of(context).textTheme.titleMedium?.copyWith(
                                  fontWeight: FontWeight.w600,
                                  color: color,
                                ),
                          ),
                          const SizedBox(width: 8),
                          Container(
                            padding: const EdgeInsets.symmetric(
                              horizontal: 8,
                              vertical: 2,
                            ),
                            decoration: BoxDecoration(
                              color: color.withValues(alpha: 0.1),
                              borderRadius: BorderRadius.circular(12),
                            ),
                            child: Text(
                              '${fields.length}',
                              style: TextStyle(
                                color: color,
                                fontSize: 12,
                                fontWeight: FontWeight.w600,
                              ),
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 2),
                      Text(
                        subtitle,
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                              color: Theme.of(context).colorScheme.onSurfaceVariant,
                            ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
            const SizedBox(height: 12),
            const Divider(height: 1),
            const SizedBox(height: 8),

            // Fields List
            if (fields.isEmpty)
              Padding(
                padding: const EdgeInsets.symmetric(vertical: 16),
                child: Center(
                  child: Text(
                    'No fields in this section',
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: Theme.of(context).colorScheme.onSurfaceVariant,
                        ),
                  ),
                ),
              )
            else
              ...fields.map((field) => _FieldListTile(
                    field: field,
                    onUpgrade: onUpgrade,
                    onDowngrade: onDowngrade,
                    isHighest: isHighest,
                    isLowest: isLowest,
                  )),
          ],
        ),
      ),
    );
  }
}

class _FieldListTile extends StatelessWidget {
  final FieldSensitivity field;
  final void Function(String fieldId)? onUpgrade;
  final void Function(String fieldId)? onDowngrade;
  final bool isHighest;
  final bool isLowest;

  const _FieldListTile({
    required this.field,
    this.onUpgrade,
    this.onDowngrade,
    this.isHighest = false,
    this.isLowest = false,
  });

  @override
  Widget build(BuildContext context) {
    final sectionName = FieldRegistry.getSectionDisplayName(field.fieldSection);

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Row(
        children: [
          // Field info
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  field.fieldName,
                  style: Theme.of(context).textTheme.bodyMedium,
                ),
                Text(
                  sectionName,
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: Theme.of(context).colorScheme.onSurfaceVariant,
                      ),
                ),
              ],
            ),
          ),

          // Level change buttons
          PopupMenuButton<String>(
            icon: Icon(
              Icons.more_vert,
              size: 20,
              color: Theme.of(context).colorScheme.onSurfaceVariant,
            ),
            tooltip: AppLocalizations.of(context).sensitivitySettingsChangeLevel,
            onSelected: (value) {
              if (value == 'upgrade' && onUpgrade != null) {
                onUpgrade!(field.fieldId);
                showOverlaySnackBar(
                  context,
                  content: '"${field.fieldName}" moved to higher sensitivity',
                  type: SnackBarType.info,
                );
              } else if (value == 'downgrade' && onDowngrade != null) {
                onDowngrade!(field.fieldId);
              }
            },
            itemBuilder: (context) => [
              if (onUpgrade != null)
                PopupMenuItem(
                  value: 'upgrade',
                  child: Row(
                    children: [
                      Icon(
                        Icons.arrow_upward,
                        color: Colors.red.shade700,
                        size: 18,
                      ),
                      const SizedBox(width: 8),
                      Text(
                        isHighest ? 'Keep at Highest' : 'Move to Higher',
                        style: TextStyle(color: Colors.red.shade700),
                      ),
                    ],
                  ),
                ),
              if (onDowngrade != null)
                PopupMenuItem(
                  value: 'downgrade',
                  child: Row(
                    children: [
                      Icon(
                        Icons.arrow_downward,
                        color: Colors.orange.shade700,
                        size: 18,
                      ),
                      const SizedBox(width: 8),
                      Text(
                        isLowest ? 'Keep at Lowest' : 'Move to Lower',
                        style: TextStyle(color: Colors.orange.shade700),
                      ),
                    ],
                  ),
                ),
            ],
          ),
        ],
      ),
    );
  }
}

class _SensitivitySettingsView extends StatelessWidget {
  final TextEditingController searchController;
  final String searchQuery;
  final ValueChanged<String> onSearchChanged;
  final VoidCallback onClearSearch;
  final List<FieldSensitivity> effectiveFields;
  final Map<SensitivityLevel, List<FieldSensitivity>> sections;
  final void Function(String) onUpgrade;
  final void Function(String) onDowngrade;
  final void Function(String) onDowngradeCritical;

  const _SensitivitySettingsView({
    required this.searchController,
    required this.searchQuery,
    required this.onSearchChanged,
    required this.onClearSearch,
    required this.effectiveFields,
    required this.sections,
    required this.onUpgrade,
    required this.onDowngrade,
    required this.onDowngradeCritical,
  });

  @override
  Widget build(BuildContext context) {
    final publicFields = sections[SensitivityLevel.public]!;
    final internalFields = sections[SensitivityLevel.internal]!;
    final sensitiveFields = sections[SensitivityLevel.sensitive]!;
    final criticalFields = sections[SensitivityLevel.critical]!;

    final hasResults = publicFields.isNotEmpty || internalFields.isNotEmpty || sensitiveFields.isNotEmpty || criticalFields.isNotEmpty;
    final totalFields = effectiveFields.length;

    return Scaffold(
      appBar: SoloGlassAppBar(
        title: Text(AppLocalizations.of(context).sensitivitySettingsTitle),
        actions: const [
          HeaderActionButtons(),
        ],
      ),
      body: effectiveFields.isEmpty
          ? const Center(child: CircularProgressIndicator())
          : Column(
              children: [
                // Search bar
                Padding(
                  padding: const EdgeInsets.all(16),
                  child: TextField(
                    controller: searchController,
                    onChanged: onSearchChanged,
                    decoration: InputDecoration(
                      hintText: AppLocalizations.of(context).sensitivitySettingsSearchHint,
                      prefixIcon: const Icon(Icons.search),
                      suffixIcon: searchQuery.isNotEmpty
                          ? IconButton(
                              icon: const Icon(Icons.clear),
                              onPressed: onClearSearch,
                            )
                          : null,
                      border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(12),
                      ),
                      contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
                    ),
                  ),
                ),

                // Results count
                if (searchQuery.isNotEmpty)
                  Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 16),
                    child: Row(
                      children: [
                        Text(
                          hasResults
                              ? 'Found ${publicFields.length + internalFields.length + sensitiveFields.length + criticalFields.length} result(s)'
                              : 'No results found',
                          style: Theme.of(context).textTheme.bodySmall?.copyWith(
                                color: hasResults
                                    ? Theme.of(context).colorScheme.onSurfaceVariant
                                    : Colors.orange,
                              ),
                        ),
                        const Spacer(),
                        Text(
                          '$totalFields total fields',
                          style: Theme.of(context).textTheme.bodySmall?.copyWith(
                                color: Theme.of(context).colorScheme.onSurfaceVariant,
                              ),
                        ),
                      ],
                    ),
                  ),

                const SizedBox(height: 8),

                // Sections list
                Expanded(
                  child: ListView(
                    padding: const EdgeInsets.symmetric(horizontal: 16),
                    children: [
                      // Header info
                      Container(
                        padding: const EdgeInsets.all(16),
                        decoration: BoxDecoration(
                          color: AppTheme.primaryColor.withValues(alpha: 0.1),
                          borderRadius: BorderRadius.circular(12),
                        ),
                        child: Row(
                          children: [
                            const Icon(
                              Icons.security,
                              color: AppTheme.primaryColor,
                              size: 24,
                            ),
                            const SizedBox(width: 12),
                            Expanded(
                              child: Text(
                                'Adjust the sensitivity level for each field. Restricted fields require additional verification to view.',
                                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                                      color: AppTheme.primaryColor,
                                    ),
                              ),
                            ),
                          ],
                        ),
                      ).animate().fadeIn(duration: 400.ms),

                      const SizedBox(height: 24),

                      // Critical Section (Highest)
                      if (criticalFields.isNotEmpty)
                        _SensitivitySection(
                          title: 'Critical',
                          subtitle: 'Maximum sensitivity - always masked, requires verification',
                          icon: Icons.shield,
                          color: Colors.red.shade900,
                          fields: criticalFields,
                          onUpgrade: null, // Can't upgrade further
                          onDowngrade: onDowngradeCritical,
                          isHighest: true,
                        ).animate().fadeIn(delay: 100.ms, duration: 400.ms),

                      if (criticalFields.isNotEmpty) const SizedBox(height: 16),

                      // Sensitive Section
                      if (sensitiveFields.isNotEmpty)
                        _SensitivitySection(
                          title: 'Sensitive',
                          subtitle: 'Personal information requiring protection',
                          icon: Icons.visibility_off,
                          color: Colors.orange,
                          fields: sensitiveFields,
                          onUpgrade: onUpgrade,
                          onDowngrade: onDowngrade,
                          isHighest: false,
                          isLowest: false,
                        ).animate().fadeIn(delay: 150.ms, duration: 400.ms),

                      if (sensitiveFields.isNotEmpty) const SizedBox(height: 16),

                      // Internal Section
                      if (internalFields.isNotEmpty)
                        _SensitivitySection(
                          title: 'Internal',
                          subtitle: 'Internal use only - can be hidden by display settings',
                          icon: Icons.visibility,
                          color: Colors.blue,
                          fields: internalFields,
                          onUpgrade: onUpgrade,
                          onDowngrade: onDowngrade,
                          isHighest: false,
                          isLowest: false,
                        ).animate().fadeIn(delay: 200.ms, duration: 400.ms),

                      if (internalFields.isNotEmpty) const SizedBox(height: 16),

                      // Public Section (Lowest)
                      if (publicFields.isNotEmpty)
                        _SensitivitySection(
                          title: 'Public',
                          subtitle: 'Lowest sensitivity - always visible',
                          icon: Icons.public,
                          color: Colors.green,
                          fields: publicFields,
                          onUpgrade: onUpgrade,
                          onDowngrade: null, // Can't downgrade further
                          isHighest: false,
                          isLowest: true,
                        ).animate().fadeIn(delay: 300.ms, duration: 400.ms),

                      // No results message
                      if (!hasResults && searchQuery.isNotEmpty)
                        Container(
                          padding: const EdgeInsets.all(32),
                          child: Column(
                            children: [
                              Icon(
                                Icons.search_off,
                                size: 48,
                                color: Theme.of(context).colorScheme.onSurfaceVariant.withValues(alpha: 0.5),
                              ),
                              const SizedBox(height: 16),
                              Text(
                                'No fields match "$searchQuery"',
                                style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                                      color: Theme.of(context).colorScheme.onSurfaceVariant,
                                    ),
                              ),
                              const SizedBox(height: 8),
                              TextButton(
                                onPressed: onClearSearch,
                                child: Text(AppLocalizations.of(context).sensitivitySettingsClearSearch),
                              ),
                            ],
                          ),
                        ),

                      const SizedBox(height: 32),

                      // Field count summary
                      Center(
                        child: Text(
                          '$totalFields fields configured',
                          style: Theme.of(context).textTheme.bodySmall?.copyWith(
                                color: Theme.of(context).colorScheme.onSurfaceVariant,
                              ),
                        ),
                      ),

                      const SizedBox(height: 16),
                    ],
                  ),
                ),
              ],
            ),
    );
  }
}
