import 'dart:async' show unawaited;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/biometric_service.dart';
import 'package:solosoul_flutter/core/services/security_service.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
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

  Future<void> _toggleBiometric(bool enable) async {
    if (enable) {
      // Verify biometric is available first
      final biometricService = BiometricService.instance;
      final canUse = await biometricService.isAvailable();
      if (!mounted) return;

      if (!canUse) {
        setState(() => _error = 'Biometric authentication is not available on this device');
        return;
      }

      // Show biometric prompt directly
      final success = await biometricService.authenticate(
        reason: 'Enable $_biometricType unlock',
      );
      if (!mounted) return;

      if (!success) {
        setState(() => _error = 'Biometric authentication failed or was cancelled');
        return;
      }

      // Enable biometric unlock
      final securityService = SecurityService.instance;
      await securityService.setBiometricsEnabled(true);
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

      // Show biometric prompt directly
      final success = await biometricService.authenticate(
        reason: 'Enable Face ID unlock',
      );
      if (!mounted) return;

      if (!success) {
        setState(() => _error = 'Face ID authentication failed or was cancelled');
        return;
      }

      // Enable Face ID unlock
      final securityService = SecurityService.instance;
      await securityService.setFaceIdEnabled(true);
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
