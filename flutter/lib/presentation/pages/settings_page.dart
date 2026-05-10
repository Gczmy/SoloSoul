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
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/language_provider.dart';

part 'settings_page.g.dart';
part 'settings_page_account_section.dart';
part 'settings_page_access_section.dart';
part 'settings_page_security_section.dart';
part 'settings_page_sync_section.dart';
part 'settings_page_llm_section.dart';
part 'settings_page_app_info_section.dart';
part 'settings_page_ad_section.dart';
part 'settings_page_debug_dialog.dart';

final packageInfoProvider = FutureProvider<PackageInfo>((ref) async {
  return PackageInfo.fromPlatform();
});

/// Fetch the latest release version from GitHub API.
/// Returns the version tag (e.g. '1.4.3'), or null on failure.
final latestVersionProvider = FutureProvider<String?>((ref) async {
  final client = HttpClient();
  try {
    final request = await client.getUrl(
      Uri.parse('https://api.github.com/repos/Gczmy/SoloSoul/releases/latest'),
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
  String _vaultDataSize = '';

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
    if (stats == null) return AppLocalizations.of(context).settingsUnknown;
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
      final isBiometric = password == _DebugActivationDialog._biometricSentinel;
      final success = isBiometric ||
          await authNotifier.verifyPasswordForSensitiveData(password);
      if (success && context.mounted) {
        await ref.read(debugModeProvider.notifier).enableDebugMode();
        if (context.mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Row(
                children: [
                  const Icon(Icons.check_circle, color: Colors.white),
                  const SizedBox(width: 12),
                  Text(AppLocalizations.of(context).settingsDebugModeEnabled),
                ],
              ),
              backgroundColor: AppTheme.successColor,
            ),
          );
        }
      } else if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Row(
              children: [
                const Icon(Icons.error_outline, color: Colors.white),
                const SizedBox(width: 12),
                Text(AppLocalizations.of(context).settingsInvalidPassword),
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
      appBar: SoloGlassAppBar(
        backRoute: AppRoutes.home,
        title: Text(AppLocalizations.of(context).settingsTitle),
        actions: const [
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
