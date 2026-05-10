part of 'settings_page.dart';

/// Debug mode activation dialog with password + biometric options.
/// Extracted from SettingsPage to reduce file length.
class _DebugActivationDialog extends StatefulWidget {
  static const _biometricSentinel = '__BIOMETRIC__';

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
      reason: AppLocalizations.of(context).settingsVerifyIdentityDebug,
    );
    if (success && mounted) {
      Navigator.pop(context, _DebugActivationDialog._biometricSentinel);
    }
  }

  @override
  Widget build(BuildContext context) {
    final biometricType = widget.faceIdEnabled
        ? AppLocalizations.of(context).loginBiometricFaceId
        : AppLocalizations.of(context).loginBiometricTouchId;
    return AlertDialog(
      title: Row(
        children: [
          const Icon(Icons.bug_report, color: AppTheme.primaryColor),
          const SizedBox(width: 12),
          Text(AppLocalizations.of(context).settingsEnableDebugMode),
        ],
      ),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(AppLocalizations.of(context).settingsEnableDebugModeDesc),
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
                label: Text(
                    AppLocalizations.of(context).settingsUseBiometric(biometricType)),
              ),
            ),
            const SizedBox(height: 12),
            Row(
              children: [
                const Expanded(child: Divider()),
                Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 8),
                  child: Text(AppLocalizations.of(context).settingsOr,
                      style: const TextStyle(color: Colors.grey)),
                ),
                const Expanded(child: Divider()),
              ],
            ),
            const SizedBox(height: 12),
          ],
          TextField(
            controller: _passwordController,
            obscureText: _obscurePassword,
            autofocus: true,
            decoration: InputDecoration(
              labelText: AppLocalizations.of(context).settingsMasterPassword,
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
                                      ? AppLocalizations.of(context).biometricPasswordHint(
                                          widget.selectedAccount!.passwordHint!)
                                      : AppLocalizations.of(context).loginNoPasswordHint,
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
                    tooltip: AppLocalizations.of(context).settingsShowPasswordHint,
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
          child: Text(AppLocalizations.of(context).commonCancel),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(context, _passwordController.text),
          child: Text(AppLocalizations.of(context).settingsEnable),
        ),
      ],
    );
  }
}
