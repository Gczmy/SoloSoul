import 'dart:async' show unawaited;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/biometric_service.dart';
import 'package:solosoul_flutter/core/services/security_service.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';

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
  String _biometricType = 'Biometric';
  String? _error;

  @override
  void initState() {
    super.initState();
    _checkBiometricStatus();
  }

  Future<void> _checkBiometricStatus() async {
    final biometricService = BiometricService.instance;
    final securityService = SecurityService.instance;
    await securityService.loadSettings();
    final availableBiometrics = await biometricService.getAvailableBiometrics();

    setState(() {
      _isLoading = false;
      _biometricEnabled = securityService.settings.biometricsEnabled;
      _faceIdEnabled = securityService.settings.faceIdEnabled;

      if (availableBiometrics.isNotEmpty) {
        if (availableBiometrics.any((b) => b == BiometricType.face)) {
          _biometricType = 'Face ID';
        } else if (availableBiometrics.any((b) => b == BiometricType.fingerprint)) {
          _biometricType = 'Touch ID';
        } else if (availableBiometrics.any((b) => b == BiometricType.iris)) {
          _biometricType = 'Iris';
        } else {
          _biometricType = 'Biometric';
        }
      }
    });
  }

  Future<String?> _showPasswordInputDialog(BuildContext context, String message, String? passwordHint) async {
    final controller = TextEditingController();
    bool obscure = true;
    return showDialog<String>(
      context: context,
      barrierDismissible: false,
      builder: (ctx) => StatefulBuilder(
        builder: (ctx, setDialogState) => AlertDialog(
          title: const Text('Verify Identity'),
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
                  labelText: 'Master Password',
                  prefixIcon: const Icon(Icons.key),
                  suffixIcon: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      if (passwordHint != null && passwordHint.isNotEmpty)
                        IconButton(
                          icon: const Icon(Icons.help_outline, size: 20),
                          onPressed: () {
                            ScaffoldMessenger.of(ctx).showSnackBar(
                              SnackBar(
                                content: Text('Password Hint: $passwordHint'),
                                behavior: SnackBarBehavior.floating,
                                backgroundColor: AppTheme.primaryColor,
                                duration: const Duration(seconds: 4),
                              ),
                            );
                          },
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
              child: const Text('Cancel'),
            ),
            ElevatedButton(
              onPressed: () => Navigator.pop(ctx, controller.text),
              child: const Text('Confirm'),
            ),
          ],
        ),
      ),
    );
  }

  Future<void> _toggleBiometric(bool enable) async {
    if (enable) {
      // First verify password before enabling biometric unlock
      final authNotifier = ref.read(authNotifierProvider.notifier);
      AccountInfo? selectedAccount = authNotifier.selectedAccount;

      // 如果当前没有选中账户，尝试使用第一个可用账户
      if (selectedAccount == null) {
        final accounts = await authNotifier.getAccountsSortedByRecent();
        if (!mounted) return;
        if (accounts.isNotEmpty) {
          selectedAccount = accounts.first;
        }
      }

      if (selectedAccount == null) {
        setState(() => _error = 'No account available. Please create or select an account first.');
        return;
      }

      String? password;

      if (ref.read(isSensitiveAccessGrantedProvider)) {
        // Skip password verification dialog if already verified, but we still need the actual password
        password = '';
      } else {
        // Show password verification dialog
        password = await showPasswordVerificationDialog(
          context: context,
          ref: ref,
          message: 'Verify your identity to enable biometric unlock.',
          passwordHint: selectedAccount.passwordHint,
          onVerify: authNotifier.verifyPasswordForSensitiveData,
        );
        if (!mounted) return;
      }

      if (password == null) {
        // User cancelled
        return;
      }

      // If verified via biometric or already granted, we need the actual password for secure storage
      if (password == 'biometric' || password.isEmpty) {
        final actualPassword = await _showPasswordInputDialog(
          context,
          'Enter your master password to enable biometric unlock.',
          selectedAccount.passwordHint,
        );
        if (!mounted) return;
        if (actualPassword == null || actualPassword.isEmpty) return;

        // Verify the actual password
        final verified = await authNotifier.verifyPasswordForSensitiveData(actualPassword);
        if (!mounted) return;
        if (!verified) {
          if (mounted) {
            showOverlaySnackBar(
              context,
              content: 'Invalid password',
              type: SnackBarType.error,
            );
          }
          return;
        }
        password = actualPassword;
      }

      // Mark as verified so Lock Sensitivity Access button appears in header
      ref.read(sensitivePageAccessProvider.notifier).markVerified();

      // Verify biometric is available
      final biometricService = BiometricService.instance;
      final canUse = await biometricService.isAvailable();
      if (!mounted) return;

      if (!canUse) {
        setState(() => _error = 'Biometric authentication is not available on this device');
        return;
      }

      // Authenticate with biometric to confirm device ownership
      final biometricSuccess = await biometricService.authenticate(
        reason: 'Verify your identity to enable $_biometricType unlock',
      );
      if (!mounted) return;

      if (!biometricSuccess) {
        setState(() => _error = 'Biometric authentication failed or was cancelled');
        return;
      }

      // Enable biometric unlock (Touch ID / fingerprint)
      final securityService = SecurityService.instance;
      await securityService.setBiometricsEnabled(true);
      await securityService.saveBiometricPassword(password);
      if (!mounted) return;
      setState(() {
        _biometricEnabled = true;
        _error = null;
      });

      if (mounted) {
        showOverlaySnackBar(
          context,
          content: '$_biometricType unlock enabled',
          type: SnackBarType.success,
        );
      }
    } else {
      // Disable biometric unlock - update state and snackbar immediately, persist in background
      setState(() => _biometricEnabled = false);
      if (mounted) {
        showOverlaySnackBar(
          context,
          content: '$_biometricType unlock disabled',
          type: SnackBarType.info,
        );
      }
      unawaited(SecurityService.instance.setBiometricsEnabled(false));
      unawaited(SecurityService.instance.clearBiometricPassword());
    }
  }

  Future<void> _toggleFaceId(bool enable) async {
    if (enable) {
      // First verify password before enabling Face ID unlock
      final authNotifier = ref.read(authNotifierProvider.notifier);
      AccountInfo? selectedAccount = authNotifier.selectedAccount;

      // 如果当前没有选中账户，尝试使用第一个可用账户
      if (selectedAccount == null) {
        final accounts = await authNotifier.getAccountsSortedByRecent();
        if (!mounted) return;
        if (accounts.isNotEmpty) {
          selectedAccount = accounts.first;
        }
      }

      if (selectedAccount == null) {
        setState(() => _error = 'No account available. Please create or select an account first.');
        return;
      }

      String? password;

      if (ref.read(isSensitiveAccessGrantedProvider)) {
        // Skip password verification dialog if already verified, but we still need the actual password
        password = '';
      } else {
        // Show password verification dialog
        password = await showPasswordVerificationDialog(
          context: context,
          ref: ref,
          message: 'Verify your identity to enable Face ID unlock.',
          passwordHint: selectedAccount.passwordHint,
          onVerify: authNotifier.verifyPasswordForSensitiveData,
        );
        if (!mounted) return;
      }

      if (password == null) {
        // User cancelled
        return;
      }

      // If verified via biometric or already granted, we need the actual password for secure storage
      if (password == 'biometric' || password.isEmpty) {
        final actualPassword = await _showPasswordInputDialog(
          context,
          'Enter your master password to enable Face ID unlock.',
          selectedAccount.passwordHint,
        );
        if (!mounted) return;
        if (actualPassword == null || actualPassword.isEmpty) return;

        // Verify the actual password
        final verified = await authNotifier.verifyPasswordForSensitiveData(actualPassword);
        if (!mounted) return;
        if (!verified) {
          if (mounted) {
            showOverlaySnackBar(
              context,
              content: 'Invalid password',
              type: SnackBarType.error,
            );
          }
          return;
        }
        password = actualPassword;
      }

      // Mark as verified so Lock Sensitivity Access button appears in header
      ref.read(sensitivePageAccessProvider.notifier).markVerified();

      // Verify biometric is available
      final biometricService = BiometricService.instance;
      final canUse = await biometricService.isAvailable();
      if (!mounted) return;

      if (!canUse) {
        setState(() => _error = 'Face ID is not available on this device');
        return;
      }

      // Check if Face ID is available on this device
      final availableBiometrics = await biometricService.getAvailableBiometrics();
      final hasFaceId = availableBiometrics.any((b) => b == BiometricType.face);

      if (!hasFaceId) {
        setState(() => _error = 'Face ID is not available on this device');
        return;
      }

      // Authenticate with Face ID to confirm device ownership
      final biometricSuccess = await biometricService.authenticate(
        reason: 'Verify your identity to enable Face ID unlock',
      );
      if (!mounted) return;

      if (!biometricSuccess) {
        setState(() => _error = 'Face ID authentication failed or was cancelled');
        return;
      }

      // Enable Face ID unlock
      final securityService = SecurityService.instance;
      await securityService.setFaceIdEnabled(true);
      await securityService.saveBiometricPassword(password);
      if (!mounted) return;
      setState(() {
        _faceIdEnabled = true;
        _error = null;
      });

      if (mounted) {
        showOverlaySnackBar(
          context,
          content: 'Face ID unlock enabled',
          type: SnackBarType.success,
        );
      }
    } else {
      // Disable Face ID unlock - update state and snackbar immediately, persist in background
      setState(() => _faceIdEnabled = false);
      if (mounted) {
        showOverlaySnackBar(
          context,
          content: 'Face ID unlock disabled',
          type: SnackBarType.info,
        );
      }
      unawaited(SecurityService.instance.setFaceIdEnabled(false));
      unawaited(SecurityService.instance.clearBiometricPassword());
    }
  }

  Future<void> _testBiometric() async {
    final biometricService = BiometricService.instance;
    final success = await biometricService.authenticate(
      reason: 'Test biometric unlock',
    );

    if (success && mounted) {
      showOverlaySnackBar(
        context,
        content: 'Biometric authentication successful',
        type: SnackBarType.success,
      );
    } else if (mounted) {
      showOverlaySnackBar(
        context,
        content: 'Biometric authentication failed',
        type: SnackBarType.error,
      );
    }
  }

  @override
  Widget build(BuildContext context) {
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
          title: 'Touch ID',
          subtitle: 'Use Touch ID to unlock',
          value: _biometricEnabled,
          onChanged: _toggleBiometric,
        ),
        if (_biometricEnabled) ...[
          const SizedBox(height: 4),
          Padding(
            padding: const EdgeInsets.only(left: 52),
            child: TextButton.icon(
              onPressed: _testBiometric,
              icon: const Icon(Icons.verified_outlined, size: 16),
              label: const Text('Test Touch ID'),
            ),
          ),
        ],
        const SizedBox(height: 8),
        _BiometricToggleTile(
          icon: Icons.face_outlined,
          title: 'Face ID',
          subtitle: 'Use Face ID to unlock',
          value: _faceIdEnabled,
          onChanged: _toggleFaceId,
        ),
        if (_faceIdEnabled) ...[
          const SizedBox(height: 4),
          Padding(
            padding: const EdgeInsets.only(left: 52),
            child: TextButton.icon(
              onPressed: _testBiometric,
              icon: const Icon(Icons.verified_outlined, size: 16),
              label: const Text('Test Face ID'),
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
        Switch(
          value: value,
          onChanged: onChanged,
        ),
      ],
    );
  }
}
