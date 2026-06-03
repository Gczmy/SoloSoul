import 'dart:async' show unawaited;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/biometric_credential_service.dart';
import 'package:solosoul_flutter/core/services/biometric_service.dart';
import 'package:solosoul_flutter/core/services/security_service.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';

/// Biometric settings and unlock management widget
class BiometricSettingsWidget extends ConsumerStatefulWidget {
  const BiometricSettingsWidget({super.key});

  @override
  ConsumerState<BiometricSettingsWidget> createState() => _BiometricSettingsWidgetState();
}

class _BiometricSettingsWidgetState extends ConsumerState<BiometricSettingsWidget> {
  bool _biometricEnabled = false;
  bool _faceIdEnabled = false;
  bool _isLoading = true;
  String? _error;

  @override
  void initState() {
    super.initState();
    _checkBiometricStatus();
  }

  Future<void> _checkBiometricStatus() async {
    final securityService = SecurityService.instance;
    await securityService.loadSettings();

    setState(() {
      _isLoading = false;
      _biometricEnabled = securityService.settings.biometricsEnabled;
      _faceIdEnabled = securityService.settings.faceIdEnabled;
    });
  }

  Future<String?> _showPasswordDialog(String message, {String? passwordHint}) async {
    final l10n = AppLocalizations.of(context);
    final controller = TextEditingController();
    try {
      bool obscure = true;
      String? hint = passwordHint;
      return await showDialog<String>(
      context: context,
      barrierDismissible: false,
      builder: (ctx) => StatefulBuilder(
        builder: (ctx, setDialogState) => AlertDialog(
          title: Text(l10n.settingsMasterPassword),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(message),
              const SizedBox(height: 16),
              TextField(
                controller: controller,
                obscureText: obscure,
                autofocus: true,
                decoration: InputDecoration(
                  labelText: l10n.settingsMasterPassword,
                  prefixIcon: const Icon(Icons.key),
                  suffixIcon: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      IconButton(
                        icon: const Icon(Icons.help_outline, size: 20),
                        onPressed: hint != null && hint.isNotEmpty
                            ? () {
                                ScaffoldMessenger.of(ctx).showSnackBar(
                                  SnackBar(
                                    content: Text(l10n.biometricPasswordHint(hint)),
                                    behavior: SnackBarBehavior.floating,
                                    backgroundColor: AppTheme.primaryColor,
                                    duration: const Duration(seconds: 4),
                                  ),
                                );
                              }
                            : null,
                        tooltip: hint != null && hint.isNotEmpty ? l10n.settingsShowPasswordHint : l10n.settingsNoHintAvailable,
                      ),
                      IconButton(
                        icon: Icon(
                          obscure ? Icons.visibility_outlined : Icons.visibility_off_outlined,
                          size: 20,
                        ),
                        onPressed: () => setDialogState(() => obscure = !obscure),
                      ),
                    ],
                  ),
                ),
                onSubmitted: (_) => Navigator.pop(ctx, controller.text),
              ),
            ],
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(ctx),
              child: Text(l10n.commonCancel),
            ),
            ElevatedButton(
              onPressed: () => Navigator.pop(ctx, controller.text),
              child: Text(l10n.commonConfirm),
            ),
          ],
        ),
      ),
    );
    } finally {
      // 推迟 dispose 到下一帧，避免对话框退场动画期间 TextField 仍尝试访问已 dispose 的 controller
      WidgetsBinding.instance.addPostFrameCallback((_) {
        controller.dispose();
      });
    }
  }

  Future<void> _toggleBiometric(bool enable) async {
    final l10n = AppLocalizations.of(context);
    if (enable) {
      // Verify biometric is available first
      final biometricService = BiometricService.instance;
      final canUse = await biometricService.isAvailable();
      if (!mounted) return;

      if (!canUse) {
        setState(() => _error = l10n.biometricTypeNotAvailable(l10n.loginBiometricTouchId));
        return;
      }

      // Show biometric prompt to verify device ownership
      final success = await biometricService.authenticate(
        reason: l10n.loginUnlockReason(l10n.loginBiometricTouchId),
      );
      if (!mounted) return;

      if (!success) {
        setState(() => _error = l10n.biometricTypeAuthFailed(l10n.loginBiometricTouchId));
        return;
      }

      // Get password hint for the dialog
      final authNotifier = ref.read(authNotifierProvider.notifier);
      final selectedAccount = authNotifier.selectedAccount;
      final passwordHint = selectedAccount?.passwordHint;

      // Ask for master password to store for biometric unlock
      final password = await _showPasswordDialog(
        l10n.biometricTypeEnablePasswordPrompt(l10n.loginBiometricTouchId),
        passwordHint: passwordHint,
      );
      if (!mounted) return;
      if (password == null || password.isEmpty) {
        setState(() => _error = l10n.biometricTypeEnablePasswordRequired(l10n.loginBiometricTouchId));
        return;
      }

      // Verify password is correct
      final verified = await authNotifier.verifyPasswordForSensitiveData(password);
      if (!mounted) return;
      if (!verified) {
        setState(() => _error = l10n.invalidPassword);
        return;
      }

      // Enable biometric unlock and save credential
      final securityService = SecurityService.instance;
      final accountId = authNotifier.selectedAccountId;
      await securityService.setBiometricsEnabled(true);
      if (accountId != null) {
        // Ensure deviceKey is initialized before saving credential
        await BiometricCredentialService.instance.initialize();
        final saved = await BiometricCredentialService.instance.saveBiometricCredential(accountId, password);
        if (!saved) {
          SoloLog.w('BiometricSettings', 'Failed to save biometric credential');
          if (!mounted) return;
          setState(() {
            _biometricEnabled = false;
            _error = l10n.biometricTypeSaveCredentialFailed(l10n.loginBiometricTouchId);
          });
          unawaited(securityService.setBiometricsEnabled(false));
          return;
        }
      }
      if (!mounted) return;
      setState(() {
        _biometricEnabled = true;
        _error = null;
      });

      if (mounted) {
        showOverlaySnackBar(
          context,
          content: l10n.biometricTypeUnlockEnabled(l10n.loginBiometricTouchId),
          type: SnackBarType.success,
        );
      }
    } else {
      // Disable biometric unlock - update state and snackbar immediately, persist in background
      setState(() => _biometricEnabled = false);
      if (mounted) {
        showOverlaySnackBar(
          context,
          content: l10n.biometricTypeUnlockDisabled(l10n.loginBiometricTouchId),
          type: SnackBarType.info,
        );
      }
      unawaited(SecurityService.instance.setBiometricsEnabled(false));
      final authNotifier = ref.read(authNotifierProvider.notifier);
      final accountId = authNotifier.selectedAccountId;
      if (accountId != null) {
        unawaited(BiometricCredentialService.instance.clearBiometricCredential(accountId));
      }
    }
  }

  Future<void> _toggleFaceId(bool enable) async {
    final l10n = AppLocalizations.of(context);
    if (enable) {
      // Verify biometric is available
      final biometricService = BiometricService.instance;
      final canUse = await biometricService.isAvailable();
      if (!mounted) return;

      if (!canUse) {
        setState(() => _error = l10n.biometricTypeNotAvailable(l10n.loginBiometricFaceId));
        return;
      }

      // Check if Face ID is available on this device
      final availableBiometrics = await biometricService.getAvailableBiometrics();
      final hasFaceId = availableBiometrics.any((b) => b == BiometricType.face);

      if (!hasFaceId) {
        setState(() => _error = l10n.biometricTypeNotAvailable(l10n.loginBiometricFaceId));
        return;
      }

      // Show biometric prompt to verify device ownership
      final success = await biometricService.authenticate(
        reason: l10n.loginUnlockReason(l10n.loginBiometricFaceId),
      );
      if (!mounted) return;

      if (!success) {
        setState(() => _error = l10n.biometricTypeAuthFailed(l10n.loginBiometricFaceId));
        return;
      }

      // Get password hint for the dialog
      final authNotifier = ref.read(authNotifierProvider.notifier);
      final selectedAccount = authNotifier.selectedAccount;
      final passwordHint = selectedAccount?.passwordHint;

      // Ask for master password to store for biometric unlock
      final password = await _showPasswordDialog(
        l10n.biometricTypeEnablePasswordPrompt(l10n.loginBiometricFaceId),
        passwordHint: passwordHint,
      );
      if (!mounted) return;
      if (password == null || password.isEmpty) {
        setState(() => _error = l10n.biometricTypeEnablePasswordRequired(l10n.loginBiometricFaceId));
        return;
      }

      // Verify password is correct
      final verified = await authNotifier.verifyPasswordForSensitiveData(password);
      if (!mounted) return;
      if (!verified) {
        setState(() => _error = l10n.invalidPassword);
        return;
      }

      // Enable Face ID unlock and save credential
      final securityService = SecurityService.instance;
      final accountId = authNotifier.selectedAccountId;
      await securityService.setFaceIdEnabled(true);
      if (accountId != null) {
        // Ensure deviceKey is initialized before saving credential
        await BiometricCredentialService.instance.initialize();
        final saved = await BiometricCredentialService.instance.saveBiometricCredential(accountId, password);
        if (!saved) {
          SoloLog.w('BiometricSettings', 'Failed to save Face ID credential');
          if (!mounted) return;
          setState(() {
            _faceIdEnabled = false;
            _error = l10n.biometricTypeSaveCredentialFailed(l10n.loginBiometricFaceId);
          });
          unawaited(securityService.setFaceIdEnabled(false));
          return;
        }
      }
      if (!mounted) return;
      setState(() {
        _faceIdEnabled = true;
        _error = null;
      });

      if (mounted) {
        showOverlaySnackBar(
          context,
          content: l10n.biometricTypeUnlockEnabled(l10n.loginBiometricFaceId),
          type: SnackBarType.success,
        );
      }
    } else {
      // Disable Face ID unlock - update state and snackbar immediately, persist in background
      setState(() => _faceIdEnabled = false);
      if (mounted) {
        showOverlaySnackBar(
          context,
          content: l10n.biometricTypeUnlockDisabled(l10n.loginBiometricFaceId),
          type: SnackBarType.info,
        );
      }
      unawaited(SecurityService.instance.setFaceIdEnabled(false));
      final authNotifier = ref.read(authNotifierProvider.notifier);
      final accountId = authNotifier.selectedAccountId;
      if (accountId != null) {
        unawaited(BiometricCredentialService.instance.clearBiometricCredential(accountId));
      }
    }
  }

  Future<void> _testBiometric(String biometricName) async {
    final l10n = AppLocalizations.of(context);
    final biometricService = BiometricService.instance;
    final success = await biometricService.authenticate(
      reason: l10n.biometricTypeTestReason(biometricName),
    );

    if (success && mounted) {
      showOverlaySnackBar(
        context,
        content: l10n.biometricTypeTestSuccess(biometricName),
        type: SnackBarType.success,
      );
    } else if (mounted) {
      showOverlaySnackBar(
        context,
        content: l10n.biometricTypeTestFailed(biometricName),
        type: SnackBarType.error,
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    if (_isLoading) {
      return const Center(child: CircularProgressIndicator());
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (_error != null) ...[
          Container(
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: Colors.red.shade50,
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: Colors.red.shade200),
            ),
            child: Row(
              children: [
                Icon(Icons.error_outline, color: Colors.red.shade700, size: 20),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    _error!,
                    style: TextStyle(color: Colors.red.shade700, fontSize: 13),
                  ),
                ),
                IconButton(
                  icon: const Icon(Icons.close, size: 18),
                  onPressed: () => setState(() => _error = null),
                  padding: EdgeInsets.zero,
                  constraints: const BoxConstraints(),
                ),
              ],
            ),
          ),
          const SizedBox(height: 16),
        ],
        _BiometricToggleTile(
          icon: Icons.fingerprint_outlined,
          title: l10n.loginBiometricTouchId,
          subtitle: l10n.settingsUseBiometric(l10n.loginBiometricTouchId),
          value: _biometricEnabled,
          onChanged: _toggleBiometric,
        ),
        if (_biometricEnabled) ...[
          const SizedBox(height: 4),
          Padding(
            padding: const EdgeInsets.only(left: 52),
            child: TextButton.icon(
              onPressed: () => _testBiometric(l10n.loginBiometricTouchId),
              icon: const Icon(Icons.verified_outlined, size: 16),
              label: Text(l10n.biometricTestTouchId),
            ),
          ),
        ],
        const SizedBox(height: 8),
        _BiometricToggleTile(
          icon: Icons.face_outlined,
          title: l10n.loginBiometricFaceId,
          subtitle: l10n.settingsUseBiometric(l10n.loginBiometricFaceId),
          value: _faceIdEnabled,
          onChanged: _toggleFaceId,
        ),
        if (_faceIdEnabled) ...[
          const SizedBox(height: 4),
          Padding(
            padding: const EdgeInsets.only(left: 52),
            child: TextButton.icon(
              onPressed: () => _testBiometric(l10n.loginBiometricFaceId),
              icon: const Icon(Icons.verified_outlined, size: 16),
              label: Text(l10n.biometricTestFaceId),
            ),
          ),
        ],
      ],
    );
  }
}

class _BiometricToggleTile extends StatelessWidget {
  final IconData icon;
  final String title;
  final String subtitle;
  final bool value;
  final ValueChanged<bool> onChanged;

  const _BiometricToggleTile({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.value,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        Icon(icon, size: 20, color: Theme.of(context).colorScheme.onSurfaceVariant),
        const SizedBox(width: 12),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                title,
                style: Theme.of(context).textTheme.bodyLarge,
              ),
              Text(
                subtitle,
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: Theme.of(context).colorScheme.onSurfaceVariant,
                    ),
              ),
            ],
          ),
        ),
        Switch.adaptive(
          value: value,
          onChanged: onChanged,
          activeTrackColor: AppTheme.primaryColor.withValues(alpha: 0.5),
          activeThumbColor: AppTheme.primaryColor,
        ),
      ],
    );
  }
}
