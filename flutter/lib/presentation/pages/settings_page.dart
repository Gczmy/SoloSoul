import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:go_router/go_router.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:solosoul_flutter/core/router/app_router.dart' show AppRoutes;
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/section_card.dart';
import 'package:solosoul_flutter/presentation/widgets/change_password_dialog.dart';
import 'package:solosoul_flutter/presentation/widgets/biometric_settings_widget.dart';
import 'package:solosoul_flutter/presentation/widgets/legal_document_sheet.dart';
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';
import 'package:solosoul_flutter/core/services/fallback_secure_storage.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart'
    show DebugLogger, LogLevel, LogEntry;

final packageInfoProvider = FutureProvider<PackageInfo>((ref) async {
  return PackageInfo.fromPlatform();
});

// Debug mode provider
final debugModeProvider = StateNotifierProvider<DebugModeNotifier, bool>((ref) {
  return DebugModeNotifier();
});

class DebugModeNotifier extends StateNotifier<bool> {
  static const _key = 'solosoul_debug_mode';

  DebugModeNotifier() : super(false) {
    _loadDebugMode();
  }

  Future<void> _loadDebugMode() async {
    // Always false for development (DebugLogger handles its own debug mode)
    if (kDebugMode) {
      // In debug mode, DebugLogger is always active via kDebugMode check
      state = true;
      DebugLogger.instance.activate();
      return;
    }
    try {
      final storage = FallbackSecureStorage();
      final value = await storage.read(key: _key);
      state = value == 'true';
      if (state == true) {
        DebugLogger.instance.activate();
      }
    } on Exception {
      state = false;
    }
  }

  Future<void> enableDebugMode() async {
    if (!kDebugMode) {
      try {
        final storage = FallbackSecureStorage();
        await storage.write(key: _key, value: 'true');
      } on Exception {}
    }
    state = true;
    DebugLogger.instance.activate();
  }

  Future<void> disableDebugMode() async {
    DebugLogger.instance.deactivate();
    try {
      final storage = FallbackSecureStorage();
      await storage.write(key: _key, value: 'false');
    } on Exception {}
    state = false;
  }
}

class SettingsPage extends ConsumerStatefulWidget {
  const SettingsPage({super.key});

  @override
  ConsumerState<SettingsPage> createState() => _SettingsPageState();
}

class _SettingsPageState extends ConsumerState<SettingsPage> {
  Future<void> _showDebugActivationDialog() async {
    final passwordController = TextEditingController();
    final authNotifier = ref.read(authNotifierProvider.notifier);
    bool obscurePassword = true;
    bool hasError = false;
    String? errorMessage;
    final selectedAccount = authNotifier.selectedAccount;

    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => StatefulBuilder(
        builder: (ctx, setDialogState) => AlertDialog(
          title: const Row(
            children: [
              Icon(Icons.bug_report, color: AppTheme.primaryColor),
              SizedBox(width: 12),
              Text('Enable Debug Mode'),
            ],
          ),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Text('Enter your master password to enable Debug Log.'),
              const SizedBox(height: 16),
              TextField(
                controller: passwordController,
                obscureText: obscurePassword,
                autofocus: true,
                decoration: InputDecoration(
                  labelText: 'Master Password',
                  prefixIcon: const Icon(Icons.lock_outline),
                  border: const OutlineInputBorder(),
                  errorText: hasError ? errorMessage : null,
                  suffixIcon: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      IconButton(
                          icon: const Icon(Icons.help_outline),
                          onPressed: () {
                            ScaffoldMessenger.of(ctx).showSnackBar(
                              SnackBar(
                                content: Row(
                                  children: [
                                    const Icon(Icons.help_outline, color: Colors.white),
                                    const SizedBox(width: 12),
                                    Expanded(
                                      child: Text(
                                        selectedAccount?.passwordHint != null
                                            ? 'Password Hint: ${selectedAccount!.passwordHint}'
                                            : 'No password hint available',
                                        style: const TextStyle(color: Colors.white),
                                      ),
                                    ),
                                  ],
                                ),
                                backgroundColor: AppTheme.primaryColor,
                                duration: const Duration(seconds: 4),
                              ),
                            );
                          },
                          tooltip: 'Show password hint',
                        ),
                      IconButton(
                        icon: Icon(
                          obscurePassword
                              ? Icons.visibility_outlined
                              : Icons.visibility_off_outlined,
                        ),
                        onPressed: () {
                          setDialogState(() => obscurePassword = !obscurePassword);
                        },
                      ),
                    ],
                  ),
                ),
                onChanged: (_) {
                  if (hasError) {
                    setDialogState(() {
                      hasError = false;
                      errorMessage = null;
                    });
                  }
                },
              ),
            ],
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(ctx, false),
              child: const Text('Cancel'),
            ),
            FilledButton(
              onPressed: () => Navigator.pop(ctx, true),
              child: const Text('Enable'),
            ),
          ],
        ),
      ),
    );

    if (confirmed == true && mounted) {
      final success = await authNotifier.verifyPasswordForSensitiveData(
        passwordController.text,
      );
      if (success && mounted) {
        await ref.read(debugModeProvider.notifier).enableDebugMode();
        if (mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            const SnackBar(
              content: Row(
                children: [
                  Icon(Icons.check_circle, color: Colors.white),
                  SizedBox(width: 12),
                  Text('Debug mode enabled'),
                ],
              ),
              backgroundColor: AppTheme.successColor,
            ),
          );
        }
      } else if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Row(
              children: [
                Icon(Icons.error_outline, color: Colors.white),
                SizedBox(width: 12),
                Text('Invalid password'),
              ],
            ),
            backgroundColor: Colors.red.shade700,
          ),
        );
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final authNotifier = ref.read(authNotifierProvider.notifier);
    final accountsAsync = ref.watch(accountsProvider);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Settings'),
        actions: const [
          HeaderActionButtons(),
        ],
      ),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Account Section
            SectionCard(
              title: 'Account',
              icon: Icons.account_circle_outlined,
              children: [
                accountsAsync.when(
                  data: (accounts) {
                    final selectedId = authNotifier.selectedAccountId;
                    final currentAccount = accounts
                        .cast<AccountInfo?>()
                        .firstWhere(
                          (a) => a?.id == selectedId,
                          orElse: () => null,
                        );
                    return Column(
                      children: [
                        _SettingsTile(
                          icon: Icons.person_outline,
                          title: 'Current Account',
                          subtitle: currentAccount?.name ?? selectedId ?? 'Unknown',
                          trailing: Container(
                            padding: const EdgeInsets.symmetric(
                              horizontal: 8,
                              vertical: 4,
                            ),
                            decoration: BoxDecoration(
                              color: AppTheme.successColor.withValues(
                                alpha: 0.1,
                              ),
                              borderRadius: BorderRadius.circular(4),
                            ),
                            child: const Text(
                              'Active',
                              style: TextStyle(
                                color: AppTheme.successColor,
                                fontSize: 12,
                                fontWeight: FontWeight.w600,
                              ),
                            ),
                          ),
                          onTap: () => _showCurrentAccountSheet(
                            context,
                            ref,
                            currentAccount,
                          ),
                        ),
                        const Divider(height: 1),
                        _SettingsTile(
                          icon: Icons.people_outline,
                          title: 'All Accounts',
                          subtitle: '${accounts.length} account(s)',
                          onTap: () =>
                              _showAllAccountsSheet(context, ref, accounts),
                        ),
                      ],
                    );
                  },
                  loading: () => const Padding(
                    padding: EdgeInsets.all(16),
                    child: Center(child: CircularProgressIndicator()),
                  ),
                  error: (_, __) => const _SettingsTile(
                    icon: Icons.error_outline,
                    title: 'Error loading accounts',
                    subtitle: 'Please restart the app',
                  ),
                ),
              ],
            ).animate().fadeIn(duration: 400.ms).slideX(begin: 0.05, end: 0),

            const SizedBox(height: 16),

            // Access Section
            SectionCard(
                  title: 'Access',
                  icon: Icons.lock_outlined,
                  children: [
                    _SettingsTile(
                      icon: Icons.lock_open_outlined,
                      title: 'Lock Vault',
                      subtitle: 'Lock now and require password',
                      onTap: () {
                        // Lock vault first (synchronously sets AuthState.locked)
                        ref.read(authNotifierProvider.notifier).lockVault();
                        // Clear sensitive access after navigation to prevent
                        // watched pages from briefly showing verification screens
                        ref.read(sensitivePageAccessProvider.notifier).clear();
                        // GoRouter redirect will navigate to login
                      },
                    ),
                    const Divider(height: 1),
                    _SettingsTile(
                      icon: Icons.password_outlined,
                      title: 'Change Master Password',
                      subtitle: 'Update your vault password',
                      onTap: () async {
                        final success = await showChangePasswordDialog(
                          context: context,
                          ref: ref,
                        );
                        if (success && context.mounted) {
                          ScaffoldMessenger.of(context).showSnackBar(
                            SnackBar(
                              content: const Row(
                                children: [
                                  Icon(
                                    Icons.check_circle,
                                    color: Colors.white,
                                    size: 20,
                                  ),
                                  SizedBox(width: 12),
                                  Text('Master password changed successfully'),
                                ],
                              ),
                              backgroundColor: AppTheme.successColor,
                              behavior: SnackBarBehavior.floating,
                              shape: RoundedRectangleBorder(
                                borderRadius: BorderRadius.circular(8),
                              ),
                              margin: const EdgeInsets.all(16),
                            ),
                          );
                        }
                      },
                    ),
                    const Divider(height: 1),
                    const BiometricSettingsWidget(),
                  ],
                )
                .animate()
                .fadeIn(delay: 100.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),

            const SizedBox(height: 16),

            // Security Section
            SectionCard(
                  title: 'Security',
                  icon: Icons.shield_outlined,
                  children: [
                    _SettingsTile(
                      icon: Icons.lock_clock_outlined,
                      title: 'Auto-Lock & Privacy',
                      subtitle: 'Configure timeout and privacy settings',
                      onTap: () => context.push(AppRoutes.securitySettings),
                    ),
                    const Divider(height: 1),
                    _SettingsTile(
                      icon: Icons.security_outlined,
                      title: 'Sensitivity Level Settings',
                      subtitle: 'Configure field sensitivity',
                      onTap: () =>
                          context.push(AppRoutes.sensitivitySettings),
                    ),
                    const Divider(height: 1),
                    _SettingsTile(
                      icon: Icons.history,
                      title: 'Operation Log',
                      subtitle: 'View activity history',
                      onTap: () =>
                          context.push(AppRoutes.operationLog),
                    ),
                  ],
                )
                .animate()
                .fadeIn(delay: 100.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),

            const SizedBox(height: 16),

            // Sync Section
            SectionCard(
                  title: 'Sync',
                  icon: Icons.sync_outlined,
                  children: [
                    _SettingsTile(
                      icon: Icons.cloud_outlined,
                      title: 'Cloud Sync',
                      subtitle: 'Not configured',
                      trailing: Switch(
                        value: false,
                        onChanged: (value) =>
                            _showComingSoon(context, 'Cloud sync setup'),
                      ),
                    ),
                    const Divider(height: 1),
                    const _SettingsTile(
                      icon: Icons.wifi_off_outlined,
                      title: 'Offline Mode',
                      subtitle: 'Local data only',
                      trailing: Icon(
                        Icons.check_circle,
                        color: AppTheme.successColor,
                        size: 20,
                      ),
                    ),
                  ],
                )
                .animate()
                .fadeIn(delay: 200.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),

            const SizedBox(height: 16),

            // App Info Section
            SectionCard(
                  title: 'About',
                  icon: Icons.info_outlined,
                  children: [
                    Consumer(
                      builder: (context, ref, _) {
                        final packageInfo = ref.watch(packageInfoProvider);
                        return _SettingsTile(
                          icon: Icons.code,
                          title: 'Version',
                          subtitle: packageInfo.when(
                            data: (info) => info.version,
                            loading: () => '...',
                            error: (_, __) => '1.0.0',
                          ),
                          onTap: () => _showVersionSheet(context, ref),
                        );
                      },
                    ),
                    Consumer(
                      builder: (context, ref, _) {
                        final isDebugMode = ref.watch(debugModeProvider);
                        if (!isDebugMode) return const SizedBox.shrink();
                        return Column(
                          children: [
                            const Divider(height: 1),
                            _SettingsTile(
                              icon: Icons.bug_report_outlined,
                              title: 'Debug Log',
                              subtitle: 'View debug log',
                              onTap: () => _showDebugLogSheet(context),
                            ),
                          ],
                        );
                      },
                    ),
                    const Divider(height: 1),
                    _SettingsTile(
                      icon: Icons.description_outlined,
                      title: 'Privacy Policy',
                      subtitle: 'View our privacy policy',
                      onTap: () => showLegalDocumentSheet(
                        context: context,
                        title: 'Privacy Policy',
                        assetPath: 'assets/docs/PRIVACY_POLICY.md',
                      ),
                    ),
                    const Divider(height: 1),
                    _SettingsTile(
                      icon: Icons.article_outlined,
                      title: 'Terms of Service',
                      subtitle: 'View terms of service',
                      onTap: () => showLegalDocumentSheet(
                        context: context,
                        title: 'Terms of Service',
                        assetPath: 'assets/docs/TERMS_OF_SERVICE.md',
                      ),
                    ),
                  ],
                )
                .animate()
                .fadeIn(delay: 300.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),

            const SizedBox(height: 32),

            // Delete Account
            _DeleteAccountButton(
              onTap: () => _confirmDeleteAccount(context, ref),
            ).animate().fadeIn(delay: 400.ms).slideX(begin: 0.05, end: 0),

            const SizedBox(height: 32),

            // SoloSoul Ad
            Container(
              padding: const EdgeInsets.all(20),
              decoration: BoxDecoration(
                gradient: LinearGradient(
                  begin: Alignment.topLeft,
                  end: Alignment.bottomRight,
                  colors: [
                    AppTheme.primaryColor.withValues(alpha: 0.15),
                    AppTheme.primaryColor.withValues(alpha: 0.05),
                  ],
                ),
                borderRadius: BorderRadius.circular(16),
                border: Border.all(
                  color: AppTheme.primaryColor.withValues(alpha: 0.2),
                ),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Container(
                        width: 48,
                        height: 48,
                        decoration: BoxDecoration(
                          color: AppTheme.primaryColor.withValues(alpha: 0.2),
                          borderRadius: BorderRadius.circular(12),
                        ),
                        child: const Icon(
                          Icons.auto_awesome,
                          color: AppTheme.primaryColor,
                          size: 28,
                        ),
                      ),
                      const SizedBox(width: 16),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              'SoloSoul 独灵',
                              style: theme.textTheme.titleMedium?.copyWith(
                                fontWeight: FontWeight.bold,
                                color: AppTheme.primaryColor,
                              ),
                            ),
                            const SizedBox(height: 4),
                            Text(
                              'Your Local Digital Twin. Privacy-First Universal Identity.',
                              style: theme.textTheme.bodySmall?.copyWith(
                                color: theme.colorScheme.onSurfaceVariant,
                                height: 1.4,
                              ),
                            ),
                          ],
                        ),
                      ),
                      const SizedBox(width: 12),
                      Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          _SloganChip(
                            icon: Icons.location_on_outlined,
                            label: 'Local',
                            theme: theme,
                          ),
                          const SizedBox(width: 8),
                          _SloganChip(
                            icon: Icons.lock_outline,
                            label: 'Private',
                            theme: theme,
                          ),
                          const SizedBox(width: 8),
                          _SloganChip(
                            icon: Icons.person_outline,
                            label: 'Universal',
                            theme: theme,
                          ),
                        ],
                      ),
                    ],
                  ),
                ],
              ),
            ).animate().fadeIn(delay: 400.ms, duration: 400.ms),
          ],
        ),
      ),
    );
  }

  void _showCurrentAccountSheet(
    BuildContext context,
    WidgetRef ref,
    AccountInfo? account,
  ) {
    if (account == null) return;

    if (!context.mounted) return;

    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      backgroundColor: Colors.transparent,
      builder: (context) => _CurrentAccountSheet(account: account),
    );
  }

  void _showAllAccountsSheet(
    BuildContext context,
    WidgetRef ref,
    List<AccountInfo> accounts,
  ) {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      backgroundColor: Colors.transparent,
      builder: (context) => _AllAccountsSheet(
        accounts: accounts,
        selectedAccountId: ref
            .read(authNotifierProvider.notifier)
            .selectedAccountId,
        onSelectAccount: (accountId) async {
          final authNotifier = ref.read(authNotifierProvider.notifier);
          await authNotifier.selectAccount(accountId);
          if (context.mounted) {
            Navigator.pop(context);
            context.go(AppRoutes.login);
          }
        },
      ),
    );
  }

  void _showVersionSheet(BuildContext context, WidgetRef ref) {
    final packageInfo = ref.read(packageInfoProvider);
    showModalBottomSheet(
      context: context,
      backgroundColor: Colors.transparent,
      builder: (context) => _VersionSheet(
        packageInfo: packageInfo,
        onDebugActivationRequested: _showDebugActivationDialog,
      ),
    );
  }

  void _showDebugLogSheet(BuildContext context) {
    showModalBottomSheet(
      context: context,
      backgroundColor: Colors.transparent,
      isScrollControlled: true,
      builder: (context) => DraggableScrollableSheet(
        initialChildSize: 0.7,
        minChildSize: 0.3,
        maxChildSize: 0.95,
        builder: (context, scrollController) => _DebugLogSheet(scrollController: scrollController),
      ),
    );
  }

  void _showComingSoon(BuildContext context, String feature) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(feature),
        content: const Text(
          'This feature will be available in a future update.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('OK'),
          ),
        ],
      ),
    );
  }

  static Future<void> _confirmDeleteAccount(BuildContext context, WidgetRef ref) async {
    final result = await showDialog<bool>(
      context: context,
      barrierDismissible: false,
      builder: (dialogContext) => _DeleteAccountDialogContent(
        dialogContext: dialogContext,
        ref: ref,
      ),
    );

    if (result == true && context.mounted) {
      ref.read(authNotifierProvider.notifier).lockVault();
      ref.read(sensitivePageAccessProvider.notifier).clear();
    }
  }
}

class _DeleteAccountButton extends StatefulWidget {
  const _DeleteAccountButton({required this.onTap});

  final VoidCallback onTap;

  @override
  State<_DeleteAccountButton> createState() => _DeleteAccountButtonState();
}

class _DeleteAccountButtonState extends State<_DeleteAccountButton> {
  bool _isHovered = false;

  @override
  Widget build(BuildContext context) {
    return MouseRegion(
      onEnter: (_) => setState(() => _isHovered = true),
      onExit: (_) => setState(() => _isHovered = false),
      child: GestureDetector(
        onTap: widget.onTap,
        child: Container(
          width: double.infinity,
          padding: const EdgeInsets.symmetric(vertical: 16),
          decoration: BoxDecoration(
            border: Border.all(color: AppTheme.errorColor),
            borderRadius: BorderRadius.circular(12),
            color: _isHovered
                ? AppTheme.errorColor.withValues(alpha: 0.1)
                : Colors.transparent,
            boxShadow: _isHovered
                ? [
                    BoxShadow(
                      color: AppTheme.errorColor.withValues(alpha: 0.3),
                      blurRadius: 0,
                      spreadRadius: 0,
                    ),
                  ]
                : null,
          ),
          child: const Text(
            'Delete Account',
            textAlign: TextAlign.center,
            style: TextStyle(
              color: AppTheme.errorColor,
              fontSize: 16,
              fontWeight: FontWeight.w600,
            ),
          ),
        ),
      ),
    );
  }
}

class _DeleteAccountDialogContent extends StatefulWidget {
  final BuildContext dialogContext;
  final WidgetRef ref;

  const _DeleteAccountDialogContent({
    required this.dialogContext,
    required this.ref,
  });

  @override
  State<_DeleteAccountDialogContent> createState() => _DeleteAccountDialogContentState();
}

class _DeleteAccountDialogContentState extends State<_DeleteAccountDialogContent> {
  final _passwordController = TextEditingController();
  final _formKey = GlobalKey<FormState>();
  bool _isDeleting = false;
  bool _obscurePassword = true;
  String? _errorMessage;

  @override
  void dispose() {
    _passwordController.dispose();
    super.dispose();
  }

  Future<void> _handleDelete() async {
    if (!_formKey.currentState!.validate()) return;

    setState(() => _isDeleting = true);

    final authNotifier = widget.ref.read(authNotifierProvider.notifier);
    final navigator = Navigator.of(widget.dialogContext);
    final success = await authNotifier.deleteAccount(_passwordController.text);

    if (!success) {
      setState(() {
        _isDeleting = false;
        _errorMessage = 'Invalid password';
      });
      return;
    }

    widget.ref.invalidate(accountsProvider);

    if (mounted) {
      navigator.pop(true);
    }
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Center(child: Text('Delete Account')),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Container(
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: Colors.red.shade50,
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: Colors.red.shade200),
            ),
            child: Row(
              children: [
                Icon(Icons.info_outline, color: Colors.red.shade700, size: 20),
                const SizedBox(width: 8),
                const Expanded(
                  child: Text(
                    '删除账户后，该账号的所有数据都会被清空，确定要删除吗？',
                    style: TextStyle(color: Colors.red, fontSize: 13),
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 16),
          Form(
            key: _formKey,
            child: TextFormField(
              controller: _passwordController,
              obscureText: _obscurePassword,
              autofocus: true,
              enabled: !_isDeleting,
              onChanged: (_) => setState(() => _errorMessage = null),
              decoration: InputDecoration(
                labelText: 'Enter password to confirm',
                errorText: _errorMessage,
                errorStyle: TextStyle(color: Colors.red.shade700, fontWeight: FontWeight.w500),
                prefixIcon: const Icon(Icons.lock_outline),
                suffixIcon: IconButton(
                  icon: Icon(_obscurePassword ? Icons.visibility_outlined : Icons.visibility_off_outlined, size: 20),
                  onPressed: () => setState(() => _obscurePassword = !_obscurePassword),
                ),
                enabledBorder: OutlineInputBorder(borderRadius: BorderRadius.circular(8), borderSide: BorderSide(color: Colors.grey.shade400)),
                errorBorder: OutlineInputBorder(borderRadius: BorderRadius.circular(8), borderSide: BorderSide(color: Colors.red.shade300)),
                focusedErrorBorder: OutlineInputBorder(borderRadius: BorderRadius.circular(8), borderSide: BorderSide(color: Colors.red.shade500, width: 2)),
              ),
              validator: (v) => v == null || v.isEmpty ? 'Password is required' : null,
            ),
          ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: _isDeleting ? null : () => Navigator.pop(widget.dialogContext, false),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: _isDeleting ? null : _handleDelete,
          style: FilledButton.styleFrom(backgroundColor: AppTheme.errorColor),
          child: _isDeleting
              ? const SizedBox(height: 20, child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white))
              : const Text('Delete Account'),
        ),
      ],
    );
  }
}

class _CurrentAccountSheet extends StatelessWidget {
  final AccountInfo account;

  const _CurrentAccountSheet({required this.account});

  String _formatDateTime(DateTime? dt) {
    if (dt == null) return 'N/A';
    return '${dt.year}-${dt.month.toString().padLeft(2, '0')}-${dt.day.toString().padLeft(2, '0')} '
        '${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}';
  }

  IconData _getDeviceIcon(String deviceName) {
    final lower = deviceName.toLowerCase();
    if (lower.contains('iphone') || lower.contains('ios')) {
      return Icons.phone_iphone;
    }
    if (lower.contains('android')) return Icons.phone_android;
    if (lower.contains('mac') || lower.contains('darwin')) {
      return Icons.laptop_mac;
    }
    if (lower.contains('windows')) return Icons.desktop_windows;
    if (lower.contains('linux')) return Icons.computer;
    if (lower.contains('web') || lower.contains('browser')) return Icons.web;
    return Icons.devices;
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Container(
      decoration: BoxDecoration(
        color: theme.colorScheme.surface,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(20)),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          const SizedBox(height: 12),
          Container(
            width: 40,
            height: 4,
            decoration: BoxDecoration(
              color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.3),
              borderRadius: BorderRadius.circular(2),
            ),
          ),
          const SizedBox(height: 20),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 24),
            child: Column(
              children: [
                // Account name header
                Row(
                  children: [
                    Container(
                      width: 56,
                      height: 56,
                      decoration: BoxDecoration(
                        color: AppTheme.primaryColor.withValues(alpha: 0.1),
                        borderRadius: BorderRadius.circular(16),
                      ),
                      child: const Icon(
                        Icons.account_circle,
                        size: 32,
                        color: AppTheme.primaryColor,
                      ),
                    ),
                    const SizedBox(width: 16),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            account.name,
                            style: theme.textTheme.titleLarge?.copyWith(
                              fontWeight: FontWeight.w600,
                            ),
                          ),
                          Text(
                            'Account ID: ${account.id}',
                            style: theme.textTheme.bodySmall?.copyWith(
                              color: theme.colorScheme.onSurfaceVariant,
                            ),
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 24),

                // Account info items with icons
                _InfoTile(
                  icon: Icons.calendar_today_outlined,
                  title: 'Created',
                  value: _formatDateTime(account.createdAt),
                ),
                const Divider(height: 1),
                _InfoTile(
                  icon: Icons.login_outlined,
                  title: 'Last Login',
                  value: _formatDateTime(account.lastLoginAt),
                ),
                const Divider(height: 1),
                _InfoTile(
                  icon: Icons.update_outlined,
                  title: 'Last Operation',
                  value: account.lastOperationDesc ?? 'No recent operations',
                  subtitle: account.lastOperationAt != null
                      ? _formatDateTime(account.lastOperationAt)
                      : null,
                ),
                const Divider(height: 1),
                _InfoTile(
                  icon: Icons.devices_outlined,
                  title: 'Login Devices',
                  value: account.recentDevices.isEmpty
                      ? 'No devices recorded'
                      : '${account.recentDevices.length} device(s)',
                ),
              ],
            ),
          ),

          // Login devices list
          if (account.recentDevices.isNotEmpty) ...[
            const SizedBox(height: 16),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 24),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'Recent Devices',
                    style: theme.textTheme.titleSmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                  const SizedBox(height: 8),
                  ...account.recentDevices.map(
                    (device) => Padding(
                      padding: const EdgeInsets.only(bottom: 8),
                      child: Row(
                        children: [
                          Icon(
                            _getDeviceIcon(device.deviceName),
                            size: 18,
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                          const SizedBox(width: 12),
                          Expanded(
                            child: Text(
                              device.deviceName,
                              style: theme.textTheme.bodyMedium,
                            ),
                          ),
                          Text(
                            _formatDateTime(device.lastUsed),
                            style: theme.textTheme.bodySmall?.copyWith(
                              color: theme.colorScheme.onSurfaceVariant,
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ],

          const SizedBox(height: 32),
        ],
      ),
    );
  }
}

class _InfoTile extends StatelessWidget {
  final IconData icon;
  final String title;
  final String value;
  final String? subtitle;

  const _InfoTile({
    required this.icon,
    required this.title,
    required this.value,
    this.subtitle,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 12),
      child: Row(
        children: [
          Container(
            width: 36,
            height: 36,
            decoration: BoxDecoration(
              color: AppTheme.primaryColor.withValues(alpha: 0.1),
              borderRadius: BorderRadius.circular(8),
            ),
            child: Icon(icon, size: 18, color: AppTheme.primaryColor),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  title,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
                const SizedBox(height: 2),
                Text(value, style: theme.textTheme.bodyMedium),
                if (subtitle != null) ...[
                  const SizedBox(height: 2),
                  Text(
                    subtitle!,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _AllAccountsSheet extends StatelessWidget {
  final List<AccountInfo> accounts;
  final String? selectedAccountId;
  final Future<void> Function(String accountId) onSelectAccount;

  const _AllAccountsSheet({
    required this.accounts,
    required this.selectedAccountId,
    required this.onSelectAccount,
  });

  String _formatDateTime(DateTime? dt) {
    if (dt == null) return 'N/A';
    return '${dt.year}-${dt.month.toString().padLeft(2, '0')}-${dt.day.toString().padLeft(2, '0')}';
  }

  IconData _getDeviceIcon(String deviceName) {
    final lower = deviceName.toLowerCase();
    if (lower.contains('iphone') || lower.contains('ios')) {
      return Icons.phone_iphone;
    }
    if (lower.contains('android')) return Icons.phone_android;
    if (lower.contains('mac') || lower.contains('darwin')) {
      return Icons.laptop_mac;
    }
    if (lower.contains('windows')) return Icons.desktop_windows;
    if (lower.contains('linux')) return Icons.computer;
    if (lower.contains('web') || lower.contains('browser')) return Icons.web;
    return Icons.devices;
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Container(
      constraints: BoxConstraints(
        maxHeight: MediaQuery.of(context).size.height * 0.7,
      ),
      decoration: BoxDecoration(
        color: theme.colorScheme.surface,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(20)),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          const SizedBox(height: 12),
          Container(
            width: 40,
            height: 4,
            decoration: BoxDecoration(
              color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.3),
              borderRadius: BorderRadius.circular(2),
            ),
          ),
          const SizedBox(height: 20),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 24),
            child: Row(
              children: [
                const Icon(
                  Icons.manage_accounts_outlined,
                  color: AppTheme.primaryColor,
                ),
                const SizedBox(width: 12),
                Text(
                  'All Accounts',
                  style: theme.textTheme.titleLarge?.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const Spacer(),
                Text(
                  '${accounts.length} account(s)',
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 16),
          Flexible(
            child: ListView.separated(
              shrinkWrap: true,
              padding: const EdgeInsets.symmetric(horizontal: 24),
              itemCount: accounts.length,
              separatorBuilder: (_, __) => const SizedBox(height: 8),
              itemBuilder: (context, index) {
                final account = accounts[index];
                final isSelected = account.id == selectedAccountId;

                return Material(
                  color: isSelected
                      ? AppTheme.primaryColor.withValues(alpha: 0.1)
                      : theme.colorScheme.surfaceContainerHighest.withValues(
                          alpha: 0.3,
                        ),
                  borderRadius: BorderRadius.circular(12),
                  child: InkWell(
                    borderRadius: BorderRadius.circular(12),
                    onTap: isSelected
                        ? null
                        : () => onSelectAccount(account.id),
                    child: Padding(
                      padding: const EdgeInsets.all(16),
                      child: Row(
                        children: [
                          Container(
                            width: 44,
                            height: 44,
                            decoration: BoxDecoration(
                              color: isSelected
                                  ? AppTheme.primaryColor.withValues(alpha: 0.2)
                                  : theme.colorScheme.surfaceContainerHighest,
                              borderRadius: BorderRadius.circular(12),
                            ),
                            child: Icon(
                              Icons.account_circle,
                              color: isSelected
                                  ? AppTheme.primaryColor
                                  : theme.colorScheme.onSurfaceVariant,
                            ),
                          ),
                          const SizedBox(width: 12),
                          Expanded(
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                Row(
                                  children: [
                                    Text(
                                      account.name,
                                      style: theme.textTheme.titleMedium
                                          ?.copyWith(
                                            fontWeight: FontWeight.w600,
                                          ),
                                    ),
                                    if (isSelected) ...[
                                      const SizedBox(width: 8),
                                      Container(
                                        padding: const EdgeInsets.symmetric(
                                          horizontal: 6,
                                          vertical: 2,
                                        ),
                                        decoration: BoxDecoration(
                                          color: AppTheme.successColor
                                              .withValues(alpha: 0.1),
                                          borderRadius: BorderRadius.circular(
                                            4,
                                          ),
                                        ),
                                        child: const Text(
                                          'Active',
                                          style: TextStyle(
                                            color: AppTheme.successColor,
                                            fontSize: 10,
                                            fontWeight: FontWeight.w600,
                                          ),
                                        ),
                                      ),
                                    ],
                                  ],
                                ),
                                const SizedBox(height: 4),
                                Text(
                                  'Last login: ${_formatDateTime(account.lastLoginAt)}',
                                  style: theme.textTheme.bodySmall?.copyWith(
                                    color: theme.colorScheme.onSurfaceVariant,
                                  ),
                                ),
                              ],
                            ),
                          ),
                          if (account.recentDevices.isNotEmpty) ...[
                            const SizedBox(width: 8),
                            Icon(
                              _getDeviceIcon(
                                account.recentDevices.first.deviceName,
                              ),
                              size: 18,
                              color: theme.colorScheme.onSurfaceVariant,
                            ),
                          ],
                          if (!isSelected)
                            Icon(
                              Icons.chevron_right,
                              color: theme.colorScheme.onSurfaceVariant,
                            ),
                        ],
                      ),
                    ),
                  ),
                );
              },
            ),
          ),
          const SizedBox(height: 32),
        ],
      ),
    );
  }
}

class _DebugLogSheet extends StatefulWidget {
  final ScrollController scrollController;

  const _DebugLogSheet({required this.scrollController});

  @override
  State<_DebugLogSheet> createState() => _DebugLogSheetState();
}

class _DebugLogSheetState extends State<_DebugLogSheet> {
  List<LogEntry> _entries = [];

  @override
  void initState() {
    super.initState();
    _loadLog();
  }

  void _loadLog() {
    setState(() {
      _entries = DebugLogger.instance.entries;
    });
  }

  Future<void> _copyToClipboard() async {
    final text = DebugLogger.instance.getExportLog();
    await Clipboard.setData(ClipboardData(text: text));
    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: const Row(
          children: [
            Icon(Icons.check_circle, color: Colors.white, size: 20),
            SizedBox(width: 12),
            Text('Logs copied to clipboard'),
          ],
        ),
        backgroundColor: AppTheme.successColor,
        behavior: SnackBarBehavior.floating,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
        margin: const EdgeInsets.all(16),
      ),
    );
  }

  Color _levelColor(LogLevel level) {
    switch (level) {
      case LogLevel.error:
        return Colors.red.shade700;
      case LogLevel.warning:
        return Colors.orange.shade700;
      case LogLevel.info:
        return Colors.blue.shade700;
      case LogLevel.debug:
        return Colors.grey.shade600;
    }
  }

  TextStyle _levelStyle(LogLevel level, Color baseColor) {
    return TextStyle(
      fontFamily: 'monospace',
      fontSize: 11,
      color: baseColor,
      fontWeight: FontWeight.w600,
    );
  }

  TextStyle _normalStyle(Color baseColor) {
    return TextStyle(
      fontFamily: 'monospace',
      fontSize: 11,
      color: baseColor,
    );
  }

  Widget _buildLogText(BuildContext context) {
    if (_entries.isEmpty) {
      return Text(
        'No debug logs available.',
        style: TextStyle(
          fontFamily: 'monospace',
          fontSize: 11,
          color: Theme.of(context).colorScheme.onSurfaceVariant,
        ),
      );
    }

    final baseColor = Theme.of(context).colorScheme.onSurface;
    final spans = <TextSpan>[];

    for (final entry in _entries) {
      final color = _levelColor(entry.level);
      spans.add(TextSpan(
        text: '[${entry.timestamp.toIso8601String()}] ',
        style: _normalStyle(baseColor),
      ));
      spans.add(TextSpan(
        text: '[${entry.level.name.toUpperCase()}] ',
        style: _levelStyle(entry.level, color),
      ));
      spans.add(TextSpan(
        text: '[${entry.tag}] ',
        style: _normalStyle(baseColor),
      ));
      spans.add(TextSpan(
        text: '${entry.message}\n',
        style: _normalStyle(baseColor),
      ));
    }

    return SelectableText.rich(
      TextSpan(children: spans),
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Container(
      decoration: BoxDecoration(
        color: theme.colorScheme.surface,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(20)),
      ),
      child: Column(
        children: [
          const SizedBox(height: 12),
          Container(
            width: 40,
            height: 4,
            decoration: BoxDecoration(
              color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.3),
              borderRadius: BorderRadius.circular(2),
            ),
          ),
          const SizedBox(height: 16),

          // Warning banner
          Container(
            margin: const EdgeInsets.symmetric(horizontal: 24),
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: Colors.red.withValues(alpha: 0.1),
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: Colors.red.withValues(alpha: 0.3)),
            ),
            child: Row(
              children: [
                Icon(Icons.warning_amber, color: Colors.red.shade700, size: 20),
                const SizedBox(width: 12),
                Expanded(
                  child: Text(
                    'Debug mode is active. Logs are being recorded.',
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: Colors.red.shade700,
                    ),
                  ),
                ),
              ],
            ),
          ),

          const SizedBox(height: 16),

          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 24),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text('Debug Log', style: theme.textTheme.titleLarge),
                Row(
                  children: [
                    IconButton(
                      icon: const Icon(Icons.refresh),
                      onPressed: _loadLog,
                      tooltip: 'Refresh',
                    ),
                    IconButton(
                      icon: const Icon(Icons.copy),
                      onPressed: () async {
                        await _copyToClipboard();
                        if (mounted) Navigator.pop(context);
                      },
                      tooltip: 'Copy to clipboard',
                    ),
                  ],
                ),
              ],
            ),
          ),
          const Divider(),
          Expanded(
            child: SingleChildScrollView(
              controller: widget.scrollController,
              padding: const EdgeInsets.all(16),
              child: _buildLogText(context),
            ),
          ),
        ],
      ),
    );
  }
}

class _VersionSheet extends ConsumerStatefulWidget {
  final AsyncValue<PackageInfo> packageInfo;
  final Future<void> Function() onDebugActivationRequested;

  const _VersionSheet({
    required this.packageInfo,
    required this.onDebugActivationRequested,
  });

  @override
  ConsumerState<_VersionSheet> createState() => _VersionSheetState();
}

class _VersionSheetState extends ConsumerState<_VersionSheet> {
  int _tapCount = 0;
  DateTime? _lastTapTime;

  void _handleCurrentVersionTap() {
    final now = DateTime.now();
    // Reset if more than 2 seconds between taps
    if (_lastTapTime != null && now.difference(_lastTapTime!).inSeconds > 2) {
      _tapCount = 0;
    }
    _lastTapTime = now;
    _tapCount++;

    if (_tapCount >= 5) {
      _tapCount = 0;
      widget.onDebugActivationRequested();
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    const latestVersion = '1.0.0';

    return Container(
      decoration: BoxDecoration(
        color: theme.colorScheme.surface,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(20)),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          const SizedBox(height: 12),
          Container(
            width: 40,
            height: 4,
            decoration: BoxDecoration(
              color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.3),
              borderRadius: BorderRadius.circular(2),
            ),
          ),
          const SizedBox(height: 24),
          Flexible(
            child: SingleChildScrollView(
              padding: const EdgeInsets.symmetric(horizontal: 24),
              child: Column(
                children: [
                  // App icon
                  Container(
                    width: 72,
                    height: 72,
                    decoration: BoxDecoration(
                      color: AppTheme.primaryColor.withValues(alpha: 0.1),
                      borderRadius: BorderRadius.circular(20),
                    ),
                    child: const Icon(
                      Icons.shield_outlined,
                      size: 40,
                      color: AppTheme.primaryColor,
                    ),
                  ),
                  const SizedBox(height: 16),
                  Text(
                    'SoloSoul',
                    style: theme.textTheme.titleLarge?.copyWith(
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const SizedBox(height: 24),

                  // Version info items
                  GestureDetector(
                    onTap: _handleCurrentVersionTap,
                    child: _VersionInfoTile(
                      icon: Icons.info_outline,
                      title: 'Current Version',
                      value: widget.packageInfo.when(
                        data: (info) => '${info.version}${_tapCount > 0 ? ' ($_tapCount)' : ''}',
                        loading: () => '...',
                        error: (_, __) => '1.0.0',
                      ),
                      trailing: _tapCount > 0
                          ? Container(
                              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                              decoration: BoxDecoration(
                                color: AppTheme.primaryColor.withValues(alpha: 0.1),
                                borderRadius: BorderRadius.circular(10),
                              ),
                              child: Text(
                                '/5',
                                style: const TextStyle(
                                  color: AppTheme.primaryColor,
                                  fontSize: 11,
                                  fontWeight: FontWeight.w600,
                                ),
                              ),
                            )
                          : null,
                    ),
                  ),
                  const Divider(height: 1),
                  const _VersionInfoTile(
                    icon: Icons.cloud_download_outlined,
                    title: 'Latest Version',
                    value: latestVersion,
                  ),
                  const Divider(height: 1),
                  const _VersionInfoTile(
                    icon: Icons.check_circle_outline,
                    title: 'Update Status',
                    value: 'Up to date',
                  ),
                  const Divider(height: 1),
                  _VersionInfoTile(
                    icon: Icons.phone_android,
                    title: 'Platform',
                    value:
                        Platform.isMacOS ? 'macOS' : Platform.operatingSystem[0].toUpperCase() + Platform.operatingSystem.substring(1),
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 32),
        ],
      ),
    );
  }
}

class _VersionInfoTile extends StatelessWidget {
  final IconData icon;
  final String title;
  final String value;
  final Widget? trailing;

  const _VersionInfoTile({
    required this.icon,
    required this.title,
    required this.value,
    this.trailing,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 12),
      child: Row(
        children: [
          Container(
            width: 36,
            height: 36,
            decoration: BoxDecoration(
              color: AppTheme.primaryColor.withValues(alpha: 0.1),
              borderRadius: BorderRadius.circular(8),
            ),
            child: Icon(icon, size: 18, color: AppTheme.primaryColor),
          ),
          const SizedBox(width: 12),
          Expanded(child: Text(title, style: theme.textTheme.bodyMedium)),
          if (trailing != null) ...[
            trailing!,
            const SizedBox(width: 8),
          ],
          Text(
            value,
            style: theme.textTheme.bodyMedium?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
              fontWeight: FontWeight.w500,
            ),
          ),
        ],
      ),
    );
  }
}

class _SettingsTile extends StatelessWidget {
  final IconData icon;
  final String title;
  final String subtitle;
  final Widget? trailing;
  final VoidCallback? onTap;

  const _SettingsTile({
    required this.icon,
    required this.title,
    required this.subtitle,
    this.trailing,
    this.onTap,
  });

  static const _verticalPadding = 12.0;
  static const _iconSize = 20.0;
  static const _spacing = 12.0;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return InkWell(
      onTap: onTap,
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: _verticalPadding),
        child: Row(
          children: [
            Icon(icon, size: _iconSize, color: theme.colorScheme.onSurfaceVariant),
            const SizedBox(width: _spacing),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(title, style: theme.textTheme.bodyLarge),
                  Text(
                    subtitle,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ),
            ),
            if (trailing != null) trailing!,
            if (trailing == null && onTap != null)
              Icon(
                Icons.chevron_right,
                color: theme.colorScheme.onSurfaceVariant,
              ),
          ],
        ),
      ),
    );
  }
}

class _SloganChip extends StatelessWidget {
  final IconData icon;
  final String label;
  final ThemeData theme;

  const _SloganChip({
    required this.icon,
    required this.label,
    required this.theme,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
      decoration: BoxDecoration(
        color: AppTheme.primaryColor.withValues(alpha: 0.1),
        borderRadius: BorderRadius.circular(12),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 12, color: AppTheme.primaryColor),
          const SizedBox(width: 4),
          Text(
            label,
            style: theme.textTheme.labelSmall?.copyWith(
              color: AppTheme.primaryColor,
              fontWeight: FontWeight.w600,
            ),
          ),
        ],
      ),
    );
  }
}
