import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/biometric_service.dart';
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
    final availableBiometrics = await biometricService.getAvailableBiometrics();

    setState(() {
      _isLoading = false;
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
      // Note: _biometricEnabled state would be persisted and loaded from secure storage
      // For now, we default to false until the user enables it
    });
  }

  Future<void> _toggleBiometric(bool enable) async {
    if (enable) {
      // First verify password before enabling biometric unlock
      final authNotifier = ref.read(authNotifierProvider.notifier);
      final selectedAccount = authNotifier.selectedAccount;

      if (selectedAccount == null) {
        setState(() => _error = 'No account selected');
        return;
      }

      // Check if sensitive access is already verified (within 5 minutes)
      final accessState = ref.read(sensitivePageAccessProvider);
      String? password;

      if (accessState.isVerified) {
        // Skip password verification if already verified
        password = ''; // Non-null placeholder to indicate verified
      } else {
        // Show password verification dialog
        password = await showPasswordVerificationDialog(
          context: context,
          ref: ref,
          message: 'Verify your identity to enable biometric unlock.',
          passwordHint: selectedAccount.passwordHint,
          onVerify: authNotifier.verifyPasswordForSensitiveData,
        );
      }

      if (password == null) {
        // User cancelled
        return;
      }

      // Verify biometric is available
      final biometricService = BiometricService.instance;
      final canUse = await biometricService.isAvailable();

      if (!canUse) {
        setState(() => _error = 'Biometric authentication is not available on this device');
        return;
      }

      // Test biometric authentication works
      final success = await biometricService.authenticate(
        reason: 'Enable biometric unlock for SoloSoul',
      );

      if (!success) {
        setState(() => _error = 'Biometric authentication failed');
        return;
      }

      // Enable biometric unlock - persist this preference
      // TODO: Persist biometric enabled state to secure storage
      setState(() {
        _biometricEnabled = true;
        _error = null;
      });

      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('$_biometricType unlock enabled'),
            behavior: SnackBarBehavior.floating,
            backgroundColor: AppTheme.successColor,
          ),
        );
      }
    } else {
      // Disable biometric unlock
      setState(() => _biometricEnabled = false);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('$_biometricType unlock disabled'),
            behavior: SnackBarBehavior.floating,
          ),
        );
      }
    }
  }

  Future<void> _testBiometric() async {
    final biometricService = BiometricService.instance;
    final success = await biometricService.authenticate(
      reason: 'Test biometric unlock',
    );

    if (success && mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('Biometric authentication successful'),
          behavior: SnackBarBehavior.floating,
          backgroundColor: AppTheme.successColor,
        ),
      );
    } else if (mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('Biometric authentication failed'),
          behavior: SnackBarBehavior.floating,
          backgroundColor: AppTheme.errorColor,
        ),
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
          icon: Icons.fingerprint,
          title: 'Biometric Unlock',
          subtitle: 'Use $_biometricType to unlock',
          value: _biometricEnabled,
          onChanged: _toggleBiometric,
        ),
        if (_biometricEnabled) ...[
          const SizedBox(height: 8),
          TextButton.icon(
            onPressed: _testBiometric,
            icon: const Icon(Icons.check_circle_outline, size: 18),
            label: Text('Test $_biometricType'),
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
        Container(
          width: 40,
          height: 40,
          decoration: BoxDecoration(
            color: AppTheme.primaryColor.withValues(alpha: 0.1),
            borderRadius: BorderRadius.circular(8),
          ),
          child: Icon(icon, size: 20, color: AppTheme.primaryColor),
        ),
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
