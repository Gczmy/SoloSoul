import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    hide SensitivityLevel, showOverlaySnackBar, SnackBarType;
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    show showOverlaySnackBar, SnackBarType;
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart';
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart' show FieldRegistry, FieldSensitivity, SensitivityLevel, formFieldRegistryProvider;
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';

class SensitivitySettingsPage extends ConsumerStatefulWidget {
  const SensitivitySettingsPage({super.key});

  @override
  ConsumerState<SensitivitySettingsPage> createState() => _SensitivitySettingsPageState();
}

class _SensitivitySettingsPageState extends ConsumerState<SensitivitySettingsPage> {
  final _passwordController = TextEditingController();
  final _searchController = TextEditingController();
  bool _isLoading = false;
  bool _obscurePassword = true;
  String? _error;
  String _searchQuery = '';

  // Password field focus state
  final _passwordFocusNode = FocusNode();
  bool _isPasswordFocused = false;

  @override
  void initState() {
    super.initState();
    _passwordFocusNode.addListener(_onPasswordFocusChange);
  }

  void _onPasswordFocusChange() {
    final hasFocus = _passwordFocusNode.hasFocus;
    if (hasFocus != _isPasswordFocused) {
      setState(() => _isPasswordFocused = hasFocus);
    }
  }

  @override
  void dispose() {
    _passwordController.dispose();
    _searchController.dispose();
    _passwordFocusNode.removeListener(_onPasswordFocusChange);
    _passwordFocusNode.dispose();
    super.dispose();
  }

  void _showPasswordHint(String hint) {
    // Use Overlay so the timer persists across navigation
    final overlay = Overlay.of(context);
    late OverlayEntry entry;

    entry = OverlayEntry(
      builder: (ctx) => Positioned(
        top: MediaQuery.of(context).padding.top + kToolbarHeight + 8,
        left: 16,
        right: 16,
        child: SafeArea(
          child: Material(
            color: Colors.transparent,
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
              decoration: BoxDecoration(
                color: AppTheme.primaryColor,
                borderRadius: BorderRadius.circular(12),
                boxShadow: [
                  BoxShadow(
                    color: Colors.black.withValues(alpha: 0.15),
                    blurRadius: 10,
                    offset: const Offset(0, 4),
                  ),
                ],
              ),
              child: Row(
                children: [
                  const Icon(Icons.help_outline, color: Colors.white, size: 22),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Text(
                      'Password Hint: $hint',
                      style: const TextStyle(
                        color: Colors.white,
                        fontSize: 14,
                        fontWeight: FontWeight.w500,
                      ),
                    ),
                  ),
                  IconButton(
                    icon: const Icon(Icons.close, color: Colors.white70, size: 18),
                    padding: EdgeInsets.zero,
                    constraints: const BoxConstraints(),
                    onPressed: () => entry.remove(),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );

    overlay.insert(entry);
    Timer(const Duration(seconds: 4), () {
      if (entry.mounted) {
        entry.remove();
      }
    });
  }

  Future<void> _verifyPassword() async {
    setState(() {
      _isLoading = true;
      _error = null;
    });

    final authNotifier = ref.read(authNotifierProvider.notifier);
    final success = await authNotifier.verifyPasswordForSensitiveData(_passwordController.text);

    if (success) {
      // Mark as verified in shared sensitive page access
      ref.read(sensitivePageAccessProvider.notifier).markVerified();
      if (mounted) {
        _passwordController.clear();
        setState(() => _isLoading = false);
      }
    } else {
      if (mounted) {
        setState(() {
          _error = 'Invalid password';
          _isLoading = false;
        });
      }
      _passwordController.clear();
    }
  }

  @override
  Widget build(BuildContext context) {
    // If already verified recently, show settings directly
    if (ref.watch(isSensitiveAccessGrantedProvider)) {
      return _buildSettingsView();
    }

    // Otherwise show password verification
    return _buildPasswordVerification();
  }

  Widget _buildPasswordVerification() {
    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Sensitivity Settings'),
      ),
      body: Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Icon(
                Icons.lock_outline,
                size: 64,
                color: theme.colorScheme.primary,
              ),
              const SizedBox(height: 24),
              Text(
                'Password Required',
                style: theme.textTheme.headlineSmall,
              ),
              const SizedBox(height: 8),
              Text(
                'Enter your master password to access sensitivity settings',
                style: theme.textTheme.bodyMedium?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 32),
              SizedBox(
                width: 300,
                child: Builder(
                  builder: (ctx) {
                    final authNotifier = ref.read(authNotifierProvider.notifier);
                    final hint = authNotifier.selectedAccount?.passwordHint;
                    final hasError = _error != null;
                    final errorColor = Colors.red.shade700;
                    final normalColor = Theme.of(ctx).colorScheme.onSurfaceVariant;
                    return TextField(
                      controller: _passwordController,
                      obscureText: _obscurePassword,
                      focusNode: _passwordFocusNode,
                      autofocus: true,
                      onSubmitted: (_) => _verifyPassword(),
                      decoration: InputDecoration(
                        labelText: 'Master Password',
                        labelStyle: TextStyle(
                          color: hasError
                              ? errorColor
                              : _isPasswordFocused
                              ? AppTheme.primaryColor
                              : Theme.of(ctx).colorScheme.onSurface,
                        ),
                        floatingLabelStyle: TextStyle(
                          color: hasError
                              ? errorColor
                              : _isPasswordFocused
                              ? AppTheme.primaryColor
                              : Theme.of(ctx).colorScheme.onSurface,
                        ),
                        errorText: _error,
                        errorStyle: TextStyle(
                          color: errorColor,
                          fontWeight: FontWeight.w500,
                        ),
                        prefixIcon: Icon(
                          Icons.key,
                          color: hasError
                              ? errorColor
                              : _isPasswordFocused
                              ? AppTheme.primaryColor
                              : normalColor,
                        ),
                        suffixIcon: Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            IconButton(
                              icon: Icon(
                                Icons.help_outline,
                                size: 20,
                                color: hasError
                                    ? errorColor
                                    : _isPasswordFocused
                                    ? AppTheme.primaryColor
                                    : normalColor,
                              ),
                              onPressed: () => _showPasswordHint(hint ?? 'No password hint available'),
                              tooltip: 'Show password hint',
                            ),
                            IconButton(
                              icon: Icon(
                                _obscurePassword
                                    ? Icons.visibility_outlined
                                    : Icons.visibility_off_outlined,
                                size: 20,
                                color: hasError
                                    ? errorColor
                                    : _isPasswordFocused
                                    ? AppTheme.primaryColor
                                    : normalColor,
                              ),
                              onPressed: () {
                                setState(() => _obscurePassword = !_obscurePassword);
                              },
                            ),
                          ],
                        ),
                        enabledBorder: OutlineInputBorder(
                          borderRadius: BorderRadius.circular(8),
                          borderSide: BorderSide(color: Colors.grey.shade400),
                        ),
                        errorBorder: OutlineInputBorder(
                          borderRadius: BorderRadius.circular(8),
                          borderSide: BorderSide(color: Colors.red.shade300),
                        ),
                        focusedErrorBorder: OutlineInputBorder(
                          borderRadius: BorderRadius.circular(8),
                          borderSide: BorderSide(color: Colors.red.shade500, width: 2),
                        ),
                      ),
                    );
                  },
                ),
              ),
              const SizedBox(height: 16),
              ElevatedButton(
                onPressed: _isLoading ? null : _verifyPassword,
                child: _isLoading
                    ? const SizedBox(
                        width: 24,
                        height: 24,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Text('Verify'),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildSettingsView() {
    // Watch reactive formFieldRegistryProvider for field list changes
    final registry = ref.watch(formFieldRegistryProvider);
    // Watch the full accountStyle - select on AsyncNotifier gives AsyncValue, access via .value
    final accountStyle = ref.watch(accountStyleProvider).value?.fieldSettings ?? {};
    final notifier = ref.read(accountStyleProvider.notifier);

    // Build effective field list by combining formFieldRegistryProvider with account style overrides
    List<FieldSensitivity> buildEffectiveFields() {
      // Get all registered fields from the reactive provider
      final allFields = registry.values.toSet().toList();
      allFields.sort((a, b) {
        final sec = a.fieldSection.compareTo(b.fieldSection);
        return sec != 0 ? sec : a.fieldName.compareTo(b.fieldName);
      });

      return allFields.map((field) {
        final overrideLevel = accountStyle[field.fieldId];
        return overrideLevel != null
            ? field.copyWith(level: overrideLevel)
            : field;
      }).toList();
    }

    // Filter fields based on search query
    List<FieldSensitivity> filterFields(List<FieldSensitivity> fields) {
      if (_searchQuery.isEmpty) return fields;
      final query = _searchQuery.toLowerCase();
      return fields.where((f) {
        return f.fieldName.toLowerCase().contains(query) ||
            FieldRegistry.getSectionDisplayName(f.fieldSection).toLowerCase().contains(query);
      }).toList();
    }

    final effectiveFields = buildEffectiveFields();
    final publicFields = filterFields(effectiveFields.where((f) => f.level == SensitivityLevel.public).toList());
    final internalFields = filterFields(effectiveFields.where((f) => f.level == SensitivityLevel.internal).toList());
    final sensitiveFields = filterFields(effectiveFields.where((f) => f.level == SensitivityLevel.sensitive).toList());
    final criticalFields = filterFields(effectiveFields.where((f) => f.level == SensitivityLevel.critical).toList());

    final hasResults = publicFields.isNotEmpty || internalFields.isNotEmpty || sensitiveFields.isNotEmpty || criticalFields.isNotEmpty;
    final totalFields = effectiveFields.length;

    return Scaffold(
      appBar: AppBar(
        title: const Text('Sensitivity Settings'),
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
                    controller: _searchController,
                    onChanged: (value) => setState(() => _searchQuery = value),
                    decoration: InputDecoration(
                      hintText: 'Search fields...',
                      prefixIcon: const Icon(Icons.search),
                      suffixIcon: _searchQuery.isNotEmpty
                          ? IconButton(
                              icon: const Icon(Icons.clear),
                              onPressed: () {
                                _searchController.clear();
                                setState(() => _searchQuery = '');
                              },
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
                if (_searchQuery.isNotEmpty)
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
                          onDowngrade: (fieldId) => _showDowngradeConfirmation(
                            context,
                            ref,
                            fieldId,
                          ),
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
                          onUpgrade: (fieldId) => notifier.upgradeField(fieldId),
                          onDowngrade: (fieldId) => notifier.downgradeField(fieldId),
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
                          onUpgrade: (fieldId) => notifier.upgradeField(fieldId),
                          onDowngrade: (fieldId) => notifier.downgradeField(fieldId),
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
                          onUpgrade: (fieldId) => notifier.upgradeField(fieldId),
                          onDowngrade: null, // Can't downgrade further
                          isHighest: false,
                          isLowest: true,
                        ).animate().fadeIn(delay: 300.ms, duration: 400.ms),

                      // No results message
                      if (!hasResults && _searchQuery.isNotEmpty)
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
                                'No fields match "$_searchQuery"',
                                style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                                      color: Theme.of(context).colorScheme.onSurfaceVariant,
                                    ),
                              ),
                              const SizedBox(height: 8),
                              TextButton(
                                onPressed: () {
                                  _searchController.clear();
                                  setState(() => _searchQuery = '');
                                },
                                child: const Text('Clear search'),
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

  void _showDowngradeConfirmation(
    BuildContext context,
    WidgetRef ref,
    String fieldId,
  ) {
    final accountStyle = ref.read(accountStyleProvider).valueOrNull ?? const AccountStyle();
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
            const Text('Confirm Downgrade'),
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
            child: const Text('Cancel'),
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
            child: const Text('Confirm'),
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
            tooltip: 'Change sensitivity level',
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
