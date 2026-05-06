import 'dart:convert' show jsonDecode, utf8;
import 'dart:io' show HttpClient;

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

import 'package:flutter_animate/flutter_animate.dart';
import 'package:go_router/go_router.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:solosoul_flutter/core/router/app_router.dart' show AppRoutes;
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/section_card.dart';
import 'package:solosoul_flutter/presentation/widgets/change_password_dialog.dart';
import 'package:solosoul_flutter/presentation/widgets/biometric_settings_widget.dart';
import 'package:solosoul_flutter/presentation/widgets/legal_document_sheet.dart';
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';
import 'package:solosoul_flutter/presentation/widgets/lock_vault_dialog.dart';
import 'package:solosoul_flutter/core/services/fallback_secure_storage.dart';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';
import 'package:solosoul_flutter/core/services/biometric_service.dart';
import 'package:solosoul_flutter/core/services/security_service.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart'
    show DebugLogger;
import 'package:solosoul_flutter/presentation/widgets/settings/debug_log_sheet.dart';
import 'package:solosoul_flutter/presentation/widgets/settings/version_sheet.dart';
import 'package:solosoul_flutter/presentation/widgets/settings/current_account_sheet.dart';
import 'package:solosoul_flutter/presentation/widgets/settings/all_accounts_sheet.dart';
import 'package:solosoul_flutter/presentation/widgets/settings/delete_account_button.dart';
import 'package:solosoul_flutter/presentation/widgets/settings/delete_account_dialog_content.dart';
import 'package:solosoul_flutter/presentation/widgets/settings/settings_tile.dart';
import 'package:solosoul_flutter/presentation/widgets/settings/slogan_chip.dart';
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';

part 'settings_page.g.dart';

final packageInfoProvider = FutureProvider<PackageInfo>((ref) async {
  return PackageInfo.fromPlatform();
});

/// Fetch the latest release version from GitHub API.
/// Returns the version tag (e.g. '1.4.3'), or null on failure.
final latestVersionProvider = FutureProvider<String?>((ref) async {
  final client = HttpClient();
  try {
    final request = await client.getUrl(
      Uri.parse('https://api.github.com/repos/Gczmy/SoloSoul_code/releases/latest'),
    );
    request.headers.set('Accept', 'application/vnd.github.v3+json');
    request.headers.set('User-Agent', 'SoloSoul-App');
    final response = await request.close();
    if (response.statusCode == 200) {
      final body = await response.transform(utf8.decoder).join();
      final json = jsonDecode(body) as Map<String, dynamic>;
      final tagName = json['tag_name'] as String?;
      if (tagName != null) {
        return tagName.startsWith('v') ? tagName.substring(1) : tagName;
      }
    }
    DebugLogger.instance.logWarning(
      'VERSION',
      'Failed to fetch latest version: HTTP ${response.statusCode}',
    );
  } on Exception catch (e) {
    DebugLogger.instance.logWarning('VERSION', 'Error fetching latest version: $e');
  } finally {
    client.close();
  }
  return null;
});

// Debug mode provider
@Riverpod(keepAlive: true)
class DebugMode extends _$DebugMode {
  static const _key = 'solosoul_debug_mode';

  @override
  bool build() {
    _loadDebugMode();
    return false;
  }

  Future<void> _loadDebugMode() async {
    if (kDebugMode) {
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
      } on Exception catch (e) {
        DebugLogger.instance.logError('Settings', 'Failed to enable debug mode: $e');
      }
    }
    state = true;
    DebugLogger.instance.activate();
  }

  Future<void> disableDebugMode() async {
    DebugLogger.instance.deactivate();
    try {
      final storage = FallbackSecureStorage();
      await storage.write(key: _key, value: 'false');
    } on Exception catch (e) {
      DebugLogger.instance.logError('Settings', 'Failed to disable debug mode: $e');
    }
    state = false;
  }
}

class SettingsPage extends ConsumerStatefulWidget {
  const SettingsPage({super.key});

  @override
  ConsumerState<SettingsPage> createState() => _SettingsPageState();
}

class _SettingsPageState extends ConsumerState<SettingsPage> {
  String _vaultDataSize = 'Loading...';

  @override
  void initState() {
    super.initState();
    _loadVaultDataSize();
  }

  Future<void> _loadVaultDataSize() async {
    final size = await _getVaultDataSize();
    if (mounted) setState(() => _vaultDataSize = size);
  }

  Future<String> _getVaultDataSize() async {
    final stats = await RustVaultService.instance.getVaultStats();
    if (stats == null) return 'Unknown';
    final bytes = stats.totalSizeBytes.toInt();
    if (bytes < 1024) return '$bytes B';
    if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
    if (bytes < 1024 * 1024 * 1024) {
      return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
    }
    return '${(bytes / (1024 * 1024 * 1024)).toStringAsFixed(2)} GB';
  }

  static Future<void> _showDebugActivationDialog(BuildContext context, WidgetRef ref) async {
    final authNotifier = ref.read(authNotifierProvider.notifier);
    final biometricService = BiometricService.instance;
    final securityService = SecurityService.instance;
    await securityService.loadSettings();

    final selectedAccount = authNotifier.selectedAccount;
    final isBiometricAvailable = await biometricService.isAvailable();
    final isBiometricEnabled = securityService.settings.biometricsEnabled ||
        securityService.settings.faceIdEnabled;
    final canUseBiometric = isBiometricAvailable && isBiometricEnabled;

    if (!context.mounted) return;
    final password = await showDialog<String?>(
      context: context,
      barrierDismissible: false,
      builder: (ctx) => _DebugActivationDialog(
        selectedAccount: selectedAccount,
        canUseBiometric: canUseBiometric,
        faceIdEnabled: securityService.settings.faceIdEnabled,
      ),
    );

    if (password != null && context.mounted) {
      final success = await authNotifier.verifyPasswordForSensitiveData(password);
      if (success && context.mounted) {
        await ref.read(debugModeProvider.notifier).enableDebugMode();
        if (context.mounted) {
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
      } else if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(
            content: Row(
              children: [
                Icon(Icons.error_outline, color: Colors.white),
                SizedBox(width: 12),
                Text('Invalid password'),
              ],
            ),
            backgroundColor: Colors.red,
          ),
        );
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: const SoloGlassAppBar(
        title: Text('Settings'),
        actions: [
          HeaderActionButtons(),
        ],
      ),
      body: SingleChildScrollView(
        padding: AppTheme.kPagePadding,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            _AccountSettingsSection(vaultDataSize: _vaultDataSize),
            const SizedBox(height: 16),
            const _AccessSettingsSection(),
            const SizedBox(height: 16),
            const _SecuritySettingsSection(),
            const SizedBox(height: 16),
            const _SyncSettingsSection(),
            const SizedBox(height: 16),
            const _LLMSettingsSection(),
            const SizedBox(height: 16),
            const _AppInfoSection(),
            const SizedBox(height: 32),
            DeleteAccountButton(
              onTap: () => _confirmDeleteAccount(context, ref),
            ).animate().fadeIn(delay: 400.ms).slideX(begin: 0.05, end: 0),
            const SizedBox(height: 32),
            const _SoloSoulAdSection(),
          ],
        ),
      ),
    );
  }

  static Future<void> _confirmDeleteAccount(BuildContext context, WidgetRef ref) async {
    final result = await showDialog<bool>(
      context: context,
      barrierDismissible: false,
      builder: (dialogContext) => DeleteAccountDialogContent(
        dialogContext: dialogContext,
        ref: ref,
      ),
    );

    if (result == true && context.mounted) {
      await ref.read(authNotifierProvider.notifier).lockVault();
      ref.read(sensitivePageAccessProvider.notifier).clear();
    }
  }
}

class _AccountSettingsSection extends ConsumerWidget {
  final String vaultDataSize;

  const _AccountSettingsSection({required this.vaultDataSize});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final accountsAsync = ref.watch(accountsProvider);
    final authNotifier = ref.read(authNotifierProvider.notifier);

    return SectionCard(
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
                SettingsTile(
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
                  onTap: () {
                    if (currentAccount == null) return;
                    if (!context.mounted) return;
                    showModalBottomSheet(
                      context: context,
                      isScrollControlled: true,
                      backgroundColor: Colors.transparent,
                      builder: (context) => CurrentAccountSheet(account: currentAccount),
                    );
                  },
                ),
                const Divider(height: 1),
                SettingsTile(
                  icon: Icons.people_outline,
                  title: 'All Accounts',
                  subtitle: '${accounts.length} account(s)',
                  onTap: () {
                    showModalBottomSheet(
                      context: context,
                      isScrollControlled: true,
                      backgroundColor: Colors.transparent,
                      builder: (sheetContext) => AllAccountsSheet(
                        accounts: accounts,
                        selectedAccountId: ref
                            .read(authNotifierProvider.notifier)
                            .selectedAccountId,
                        onSelectAccount: (accountId) async {
                          final authNotifier = ref.read(authNotifierProvider.notifier);
                          await authNotifier.lockVault();
                          await authNotifier.selectAccount(accountId);
                          if (context.mounted) {
                            context.go(AppRoutes.login);
                          }
                        },
                      ),
                    );
                  },
                ),
                const Divider(height: 1),
                SettingsTile(
                  icon: Icons.storage_outlined,
                  title: 'Data Management',
                  subtitle: vaultDataSize,
                  onTap: () => context.push(AppRoutes.dataManagement),
                ),
              ],
            );
          },
          loading: () => const Padding(
            padding: EdgeInsets.all(16),
            child: Center(child: CircularProgressIndicator()),
          ),
          error: (_, __) => const SettingsTile(
            icon: Icons.error_outline,
            title: 'Error loading accounts',
            subtitle: 'Please restart the app',
          ),
        ),
      ],
    ).animate().fadeIn(duration: 400.ms).slideX(begin: 0.05, end: 0);
  }
}

class _AccessSettingsSection extends ConsumerWidget {
  const _AccessSettingsSection();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return SectionCard(
      title: 'Access',
      icon: Icons.lock_outlined,
      children: [
        SettingsTile(
          icon: Icons.lock_open_outlined,
          title: 'Lock Vault',
          subtitle: 'Lock now and require password',
          onTap: () async {
            final confirmed = await showLockVaultDialog(context);
            if (confirmed == true && context.mounted) {
              await ref.read(authNotifierProvider.notifier).lockVault();
              ref.read(sensitivePageAccessProvider.notifier).clear();
            }
          },
        ),
        const Divider(height: 1),
        SettingsTile(
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
    ).animate().fadeIn(delay: 100.ms, duration: 400.ms).slideX(begin: 0.05, end: 0);
  }
}

class _SecuritySettingsSection extends ConsumerWidget {
  const _SecuritySettingsSection();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return SectionCard(
      title: 'Security',
      icon: Icons.shield_outlined,
      children: [
        SettingsTile(
          icon: Icons.lock_clock_outlined,
          title: 'Auto-Lock & Privacy',
          subtitle: 'Configure timeout and privacy settings',
          onTap: () async {
            final authNotifier = ref.read(authNotifierProvider.notifier);
            final selectedAccount = authNotifier.selectedAccount;
            final result = await showPasswordVerificationDialog(
              context: context,
              ref: ref,
              message: 'Enter your master password to access security settings.',
              passwordHint: selectedAccount?.passwordHint,
              onVerify: authNotifier.verifyPasswordForSensitiveData,
            );
            if (!context.mounted) return;
            if (result != null) {
              ref.read(sensitivePageAccessProvider.notifier).markVerified();
              if (context.mounted) {
                await context.push(AppRoutes.securitySettings);
              }
            }
          },
        ),
        const Divider(height: 1),
        SettingsTile(
          icon: Icons.security_outlined,
          title: 'Sensitivity Level Settings',
          subtitle: 'Configure field sensitivity',
          onTap: () => context.push(AppRoutes.sensitivitySettings),
        ),
        const Divider(height: 1),
        SettingsTile(
          icon: Icons.history,
          title: 'Operation Log',
          subtitle: 'View activity history',
          onTap: () => context.push(AppRoutes.operationLog),
        ),
      ],
    ).animate().fadeIn(delay: 100.ms, duration: 400.ms).slideX(begin: 0.05, end: 0);
  }
}

class _SyncSettingsSection extends ConsumerWidget {
  const _SyncSettingsSection();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return SectionCard(
      title: 'Sync',
      icon: Icons.sync_outlined,
      children: [
        SettingsTile(
          icon: Icons.cloud_outlined,
          title: 'Cloud Sync',
          subtitle: 'Not configured',
          trailing: Switch(
            value: false,
            onChanged: (value) => _showComingSoon(context, 'Cloud sync setup'),
          ),
        ),
        const Divider(height: 1),
        const SettingsTile(
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
    ).animate().fadeIn(delay: 200.ms, duration: 400.ms).slideX(begin: 0.05, end: 0);
  }
}

class _LLMSettingsSection extends ConsumerWidget {
  const _LLMSettingsSection();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return SectionCard(
      title: 'AI Assistant',
      icon: Icons.psychology_outlined,
      children: [
        SettingsTile(
          icon: Icons.smart_toy_outlined,
          title: 'LLM Configuration',
          subtitle: 'Local model or cloud API',
          onTap: () => context.push(AppRoutes.llmConfig),
        ),
        SettingsTile(
          icon: Icons.chat_bubble_outline,
          title: 'AI 对话',
          subtitle: '与本地或云端模型聊天',
          onTap: () => context.push(AppRoutes.llmChat),
        ),
      ],
    ).animate().fadeIn(delay: 250.ms, duration: 400.ms).slideX(begin: 0.05, end: 0);
  }
}

class _AppInfoSection extends ConsumerWidget {
  const _AppInfoSection();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return SectionCard(
      title: 'About',
      icon: Icons.info_outlined,
      children: [
        Consumer(
          builder: (context, ref, _) {
            final packageInfo = ref.watch(packageInfoProvider);
            return SettingsTile(
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
                SettingsTile(
                  icon: Icons.bug_report_outlined,
                  title: 'Debug Log',
                  subtitle: 'View debug log',
                  onTap: () => _showDebugLogSheet(context, ref),
                ),
              ],
            );
          },
        ),
        const Divider(height: 1),
        SettingsTile(
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
        SettingsTile(
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
    ).animate().fadeIn(delay: 300.ms, duration: 400.ms).slideX(begin: 0.05, end: 0);
  }
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

void _showVersionSheet(BuildContext context, WidgetRef ref) {
  final packageInfo = ref.read(packageInfoProvider);
  showModalBottomSheet(
    context: context,
    backgroundColor: Colors.transparent,
    builder: (context) => VersionSheet(
      packageInfo: packageInfo,
      onDebugActivationRequested: () => _SettingsPageState._showDebugActivationDialog(context, ref),
    ),
  );
}

void _showDebugLogSheet(BuildContext context, WidgetRef ref) {
  showModalBottomSheet(
    context: context,
    backgroundColor: Colors.transparent,
    isScrollControlled: true,
    builder: (context) => DraggableScrollableSheet(
      initialChildSize: 0.7,
      minChildSize: 0.3,
      maxChildSize: 0.95,
      builder: (context, scrollController) => DebugLogSheet(
        scrollController: scrollController,
        onDisableDebugMode: () async {
          await ref.read(debugModeProvider.notifier).disableDebugMode();
        },
      ),
    ),
  );
}

class _SoloSoulAdSection extends StatelessWidget {
  const _SoloSoulAdSection();

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
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
              const Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  SloganChip(
                    icon: Icons.location_on_outlined,
                    label: 'Local',
                  ),
                  SizedBox(width: 8),
                  SloganChip(
                    icon: Icons.lock_outline,
                    label: 'Private',
                  ),
                  SizedBox(width: 8),
                  SloganChip(
                    icon: Icons.person_outline,
                    label: 'Universal',
                  ),
                ],
              ),
            ],
          ),
        ],
      ),
    ).animate().fadeIn(delay: 400.ms, duration: 400.ms);
  }
}

/// Debug mode activation dialog with password + biometric options.
/// Debug mode activation dialog with password + biometric options.
/// Extracted from SettingsPage._showDebugActivationDialog to reduce method length.
class _DebugActivationDialog extends StatefulWidget {
  final AccountInfo? selectedAccount;
  final bool canUseBiometric;
  final bool faceIdEnabled;

  const _DebugActivationDialog({
    this.selectedAccount,
    required this.canUseBiometric,
    required this.faceIdEnabled,
  });

  @override
  State<_DebugActivationDialog> createState() => _DebugActivationDialogState();
}

class _DebugActivationDialogState extends State<_DebugActivationDialog> {
  final _passwordController = TextEditingController();
  bool _obscurePassword = true;
  bool _hasError = false;
  String? _errorMessage;

  @override
  void dispose() {
    _passwordController.dispose();
    super.dispose();
  }

  Future<void> _tryBiometric() async {
    final success = await BiometricService.instance.authenticate(
      reason: 'Verify your identity to enable debug mode',
    );
    if (success && mounted) {
      Navigator.pop(context, '');
    }
  }

  @override
  Widget build(BuildContext context) {
    final biometricType = widget.faceIdEnabled ? 'Face ID' : 'Touch ID';
    return AlertDialog(
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
          if (widget.canUseBiometric) ...[
            const SizedBox(height: 16),
            SizedBox(
              width: double.infinity,
              child: OutlinedButton.icon(
                onPressed: _tryBiometric,
                icon: Icon(
                  widget.faceIdEnabled
                      ? Icons.face_outlined
                      : Icons.fingerprint_outlined,
                ),
                label: Text('Use $biometricType'),
              ),
            ),
            const SizedBox(height: 12),
            const Row(
              children: [
                Expanded(child: Divider()),
                Padding(
                  padding: EdgeInsets.symmetric(horizontal: 8),
                  child: Text('or', style: TextStyle(color: Colors.grey)),
                ),
                Expanded(child: Divider()),
              ],
            ),
            const SizedBox(height: 12),
          ],
          TextField(
            controller: _passwordController,
            obscureText: _obscurePassword,
            autofocus: true,
            decoration: InputDecoration(
              labelText: 'Master Password',
              prefixIcon: const Icon(Icons.lock_outline),
              border: const OutlineInputBorder(),
              errorText: _hasError ? _errorMessage : null,
              suffixIcon: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  IconButton(
                    icon: const Icon(Icons.help_outline),
                    onPressed: () {
                      ScaffoldMessenger.of(context).showSnackBar(
                        SnackBar(
                          content: Row(
                            children: [
                              const Icon(Icons.help_outline, color: Colors.white),
                              const SizedBox(width: 12),
                              Expanded(
                                child: Text(
                                  widget.selectedAccount?.passwordHint != null
                                      ? 'Password Hint: ${widget.selectedAccount!.passwordHint}'
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
                      _obscurePassword
                          ? Icons.visibility_outlined
                          : Icons.visibility_off_outlined,
                    ),
                    onPressed: () {
                      setState(() => _obscurePassword = !_obscurePassword);
                    },
                  ),
                ],
              ),
            ),
            onChanged: (_) {
              if (_hasError) {
                setState(() {
                  _hasError = false;
                  _errorMessage = null;
                });
              }
            },
          ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context, null),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(context, _passwordController.text),
          child: const Text('Enable'),
        ),
      ],
    );
  }
}
