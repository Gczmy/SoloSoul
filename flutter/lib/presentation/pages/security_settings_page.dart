import 'dart:async' show unawaited;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/core/services/security_service.dart';
import 'package:solosoul_flutter/core/services/biometric_service.dart';
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';

class SecuritySettingsPage extends ConsumerStatefulWidget {
  const SecuritySettingsPage({super.key});

  @override
  ConsumerState<SecuritySettingsPage> createState() => _SecuritySettingsPageState();
}

class _SecuritySettingsPageState extends ConsumerState<SecuritySettingsPage> {
  late SecuritySettings _settings;
  bool _biometricsAvailable = false;
  bool _biometricsEnabled = false;
  bool _isLoading = true;

  @override
  void initState() {
    super.initState();
    _settings = SecurityService.instance.settings;
    _loadBiometrics();
  }

  Future<void> _loadBiometrics() async {
    final biometric = BiometricService.instance;
    final available = await biometric.isAvailable();
    setState(() {
      _biometricsAvailable = available;
      _biometricsEnabled = _settings.biometricsEnabled && available;
      // ignore: avoid_init_to_null
      _isLoading = false;
    });
  }

  Future<void> _updateSettings(SecuritySettings newSettings) async {
    setState(() => _settings = newSettings);
    await SecurityService.instance.saveSettings();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Security Settings'),
        actions: const [
          HeaderActionButtons(),
        ],
      ),
      body: _isLoading
          ? const Center(child: CircularProgressIndicator())
          : SingleChildScrollView(
              padding: const EdgeInsets.all(24),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  // Vault Security Section
                  const _SectionHeader(
                    title: 'Vault Security',
                    icon: Icons.lock_outlined,
                  ),
                  const SizedBox(height: 12),
                  _SettingsCard(
                    children: [
                      _DropdownSetting(
                        icon: Icons.timer_outlined,
                        title: 'Auto-Lock Delay',
                        subtitle: 'Lock vault after inactivity',
                        value: _settings.autoLockDelayMinutes,
                        options: SecuritySettings.autoLockDelayOptions,
                        labels: const ['1 min', '5 min', '15 min', '30 min', 'Never'],
                        onChanged: (value) {
                          _updateSettings(_settings.copyWith(autoLockDelayMinutes: value));
                        },
                        warningText: _settings.autoLockDelayMinutes == -1
                            ? 'Warning: Never locking disables automatic security'
                            : null,
                      ),
                      const Divider(height: 1),
                      _SwitchSetting(
                        icon: Icons.fingerprint,
                        title: 'Biometric Unlock',
                        subtitle: _biometricsAvailable
                            ? 'Use Face ID / Touch ID to unlock'
                            : 'Biometrics not available on this device',
                        value: _biometricsEnabled,
                        enabled: _biometricsAvailable,
                        onChanged: (value) async {
                          if (value) {
                            final scaffoldMessenger = ScaffoldMessenger.of(context);
                            final authenticated = await BiometricService.instance.authenticate(
                              reason: 'Verify your identity to enable biometric unlock',
                            );
                            if (authenticated) {
                              unawaited(_updateSettings(_settings.copyWith(biometricsEnabled: true)));
                              setState(() => _biometricsEnabled = true);
                              if (mounted) {
                                final hasPassword = await SecurityService.instance.getBiometricPassword();
                                if (hasPassword == null || hasPassword.isEmpty) {
                                  scaffoldMessenger.showSnackBar(
                                    const SnackBar(
                                      content: Row(
                                        children: [
                                          Icon(Icons.info_outline, color: Colors.white, size: 20),
                                          SizedBox(width: 12),
                                          Expanded(
                                            child: Text(
                                              'Biometric unlock enabled. Please go to Settings > Access to complete biometric setup with your password.',
                                            ),
                                          ),
                                        ],
                                      ),
                                      behavior: SnackBarBehavior.floating,
                                      backgroundColor: AppTheme.primaryColor,
                                      duration: Duration(seconds: 5),
                                    ),
                                  );
                                } else {
                                  scaffoldMessenger.showSnackBar(
                                    const SnackBar(
                                      content: Row(
                                        children: [
                                          Icon(Icons.check_circle, color: Colors.white, size: 20),
                                          SizedBox(width: 12),
                                          Text('Biometric unlock enabled'),
                                        ],
                                      ),
                                      behavior: SnackBarBehavior.floating,
                                      backgroundColor: AppTheme.successColor,
                                    ),
                                  );
                                }
                              }
                            } else if (mounted) {
                              scaffoldMessenger.showSnackBar(
                                const SnackBar(
                                  content: Row(
                                    children: [
                                      Icon(Icons.error_outline, color: Colors.white, size: 20),
                                      SizedBox(width: 12),
                                      Text('Biometric authentication failed or was cancelled'),
                                    ],
                                  ),
                                  behavior: SnackBarBehavior.floating,
                                  backgroundColor: AppTheme.errorColor,
                                ),
                              );
                            }
                          } else {
                            unawaited(_updateSettings(_settings.copyWith(biometricsEnabled: false)));
                            setState(() => _biometricsEnabled = false);
                            unawaited(SecurityService.instance.clearBiometricPassword());
                          }
                        },
                      ),
                    ],
                  ).animate().fadeIn(duration: 400.ms).slideX(begin: 0.05, end: 0),

                  const SizedBox(height: 24),

                  // Privacy Section
                  const _SectionHeader(
                    title: 'Privacy',
                    icon: Icons.visibility_off_outlined,
                  ),
                  const SizedBox(height: 12),
                  _SettingsCard(
                    children: [
                      _SwitchSetting(
                        icon: Icons.blur_on,
                        title: 'App Privacy Screen',
                        subtitle: 'Hide content in app switcher',
                        value: _settings.privacyScreenEnabled,
                        onTap: () {
                          _showNotImplementedSnackBar(context);
                        },
                        onChanged: (value) {
                          _updateSettings(_settings.copyWith(privacyScreenEnabled: value));
                        },
                      ),
                      const Divider(height: 1),
                      _SwitchSetting(
                        icon: Icons.window_outlined,
                        title: 'Lock on Window Blur',
                        subtitle: 'Lock when switching apps',
                        value: _settings.lockOnWindowBlur,
                        onTap: () {
                          _showNotImplementedSnackBar(context);
                        },
                        onChanged: (value) {
                          _updateSettings(_settings.copyWith(lockOnWindowBlur: value));
                        },
                      ),
                    ],
                  ).animate().fadeIn(delay: 100.ms, duration: 400.ms).slideX(begin: 0.05, end: 0),

                  const SizedBox(height: 24),

                  // Clipboard Section
                  const _SectionHeader(
                    title: 'Clipboard',
                    icon: Icons.content_paste_outlined,
                  ),
                  const SizedBox(height: 12),
                  _SettingsCard(
                    children: [
                      _DropdownSetting(
                        icon: Icons.timer_outlined,
                        title: 'Auto-Clear Delay',
                        subtitle: 'Clear clipboard after copying sensitive data',
                        value: _settings.clipboardClearDelaySeconds,
                        options: SecuritySettings.clipboardClearDelayOptions,
                        labels: const ['30 sec', '1 min', '2 min', 'Never'],
                        onChanged: (value) {
                          _updateSettings(_settings.copyWith(clipboardClearDelaySeconds: value));
                        },
                        warningText: _settings.clipboardClearDelaySeconds == -1
                            ? 'Warning: Never clearing clipboard leaves sensitive data exposed'
                            : null,
                      ),
                    ],
                  ).animate().fadeIn(delay: 200.ms, duration: 400.ms).slideX(begin: 0.05, end: 0),

                  const SizedBox(height: 32),

                  // Reset Button
                  Center(
                    child: TextButton.icon(
                      onPressed: () => _showResetConfirmation(context),
                      icon: const Icon(Icons.restore, size: 18),
                      label: const Text('Reset to Defaults'),
                      style: TextButton.styleFrom(
                        foregroundColor: theme.colorScheme.error,
                      ),
                    ),
                  ).animate().fadeIn(delay: 300.ms, duration: 400.ms),

                  const SizedBox(height: 16),

                  // Info
                  Container(
                    padding: const EdgeInsets.all(16),
                    decoration: BoxDecoration(
                      color: theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.3),
                      borderRadius: BorderRadius.circular(12),
                    ),
                    child: Row(
                      children: [
                        Icon(
                          Icons.info_outline,
                          size: 20,
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                        const SizedBox(width: 12),
                        Expanded(
                          child: Text(
                            'These settings are stored securely on your device and never leave it.',
                            style: theme.textTheme.bodySmall?.copyWith(
                              color: theme.colorScheme.onSurfaceVariant,
                            ),
                          ),
                        ),
                      ],
                    ),
                  ).animate().fadeIn(delay: 400.ms, duration: 400.ms),
                ],
              ),
            ),
    );
  }

  Future<void> _showResetConfirmation(BuildContext context) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Reset Security Settings'),
        content: const Text('This will reset all security settings to their default values. Are you sure?'),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: const Text('Reset'),
          ),
        ],
      ),
    );

    if (confirmed == true) {
      await SecurityService.instance.resetToDefaults();
      setState(() {
        _settings = SecurityService.instance.settings;
        _biometricsEnabled = _settings.biometricsEnabled && _biometricsAvailable;
      });
    }
  }

  void _showNotImplementedSnackBar(BuildContext context) {
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(
        content: Row(
          children: [
            Icon(Icons.info_outline, color: Colors.white, size: 20),
            SizedBox(width: 12),
            Text('Feature not yet implemented'),
          ],
        ),
        behavior: SnackBarBehavior.floating,
        backgroundColor: AppTheme.primaryColor,
        duration: Duration(seconds: 2),
      ),
    );
  }
}

class _SectionHeader extends StatelessWidget {
  final String title;
  final IconData icon;

  const _SectionHeader({required this.title, required this.icon});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Row(
      children: [
        Icon(icon, size: 20, color: AppTheme.primaryColor),
        const SizedBox(width: 8),
        Text(
          title,
          style: theme.textTheme.titleMedium?.copyWith(
            fontWeight: FontWeight.w600,
            color: AppTheme.primaryColor,
          ),
        ),
      ],
    );
  }
}

class _SettingsCard extends StatelessWidget {
  final List<Widget> children;

  const _SettingsCard({required this.children});

  @override
  Widget build(BuildContext context) {
    return Container(
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surface,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: Theme.of(context).colorScheme.outlineVariant.withValues(alpha: 0.5),
        ),
      ),
      child: Column(children: children),
    );
  }
}

class _SwitchSetting extends StatelessWidget {
  final IconData icon;
  final String title;
  final String subtitle;
  final bool value;
  final bool enabled;
  final VoidCallback? onTap;
  final ValueChanged<bool> onChanged;

  const _SwitchSetting({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.value,
    this.enabled = true,
    this.onTap,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return InkWell(
      onTap: onTap,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
        child: Row(
          children: [
            Container(
              width: 40,
              height: 40,
              decoration: BoxDecoration(
                color: AppTheme.primaryColor.withValues(alpha: 0.1),
                borderRadius: BorderRadius.circular(10),
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
                    style: theme.textTheme.bodyLarge?.copyWith(
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                  Text(
                    subtitle,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ),
            ),
            Switch(
              value: value,
              onChanged: enabled ? onChanged : null,
            ),
          ],
        ),
      ),
    );
  }
}

class _DropdownSetting extends StatelessWidget {
  final IconData icon;
  final String title;
  final String subtitle;
  final int value;
  final List<int> options;
  final List<String> labels;
  final ValueChanged<int> onChanged;
  final String? warningText;

  const _DropdownSetting({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.value,
    required this.options,
    required this.labels,
    required this.onChanged,
    this.warningText,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Container(
                width: 40,
                height: 40,
                decoration: BoxDecoration(
                  color: AppTheme.primaryColor.withValues(alpha: 0.1),
                  borderRadius: BorderRadius.circular(10),
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
                      style: theme.textTheme.bodyLarge?.copyWith(
                        fontWeight: FontWeight.w500,
                      ),
                    ),
                    Text(
                      subtitle,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                decoration: BoxDecoration(
                  color: theme.colorScheme.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: DropdownButton<int>(
                  value: value,
                  underline: const SizedBox(),
                  isDense: true,
                  items: List.generate(options.length, (i) {
                    return DropdownMenuItem(
                      value: options[i],
                      child: Text(labels[i]),
                    );
                  }),
                  onChanged: (v) {
                    if (v != null) onChanged(v);
                  },
                ),
              ),
            ],
          ),
          if (warningText != null) ...[
            const SizedBox(height: 8),
            Container(
              padding: const EdgeInsets.all(8),
              decoration: BoxDecoration(
                color: Colors.red.shade50,
                borderRadius: BorderRadius.circular(6),
                border: Border.all(color: Colors.red.shade200),
              ),
              child: Row(
                children: [
                  Icon(Icons.warning_amber, size: 16, color: Colors.red.shade700),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      warningText!,
                      style: TextStyle(
                        fontSize: 12,
                        color: Colors.red.shade900,
                        fontWeight: FontWeight.w500,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ],
      ),
    );
  }
}
