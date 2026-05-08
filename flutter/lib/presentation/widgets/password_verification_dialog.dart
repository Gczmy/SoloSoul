import 'dart:async';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/biometric_service.dart';
import 'package:solosoul_flutter/core/services/security_service.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';

// ignore_for_file: use_build_context_synchronously

/// Mixin for password dialog overlay management.
/// Extracts the duplicate showHintOverlay logic shared by both dialog states.
mixin PasswordDialogOverlayMixin<T extends StatefulWidget> on State<T> {
  OverlayEntry? _hintOverlayEntry;
  Timer? _hintOverlayTimer;

  /// Cancel timer and remove overlay. Safe to call multiple times.
  void disposeOverlay() {
    _hintOverlayTimer?.cancel();
    _hintOverlayEntry?.remove();
    _hintOverlayEntry = null;
  }

  /// Show a floating password hint overlay at the top of the screen.
  /// Auto-dismisses after 4 seconds.
  void showHintOverlay(String hint) {
    disposeOverlay();
    final l10n = AppLocalizations.of(context);

    final overlay = Overlay.of(context);
    _hintOverlayEntry = OverlayEntry(
      builder: (overlayCtx) => Positioned(
        top: MediaQuery.of(context).padding.top + kToolbarHeight + 8,
        left: 16,
        right: 16,
        child: SafeArea(
          child: Material(
            color: Colors.transparent,
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
              decoration: BoxDecoration(
                color: Colors.orange.shade700,
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
                  const Icon(Icons.lightbulb_outline, color: Colors.white, size: 22),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Text(
                      AppLocalizations.of(context).biometricPasswordHint(hint),
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
                    onPressed: disposeOverlay,
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );

    overlay.insert(_hintOverlayEntry!);
    _hintOverlayTimer = Timer(const Duration(seconds: 4), disposeOverlay);
  }
}

/// Shared password verification dialog for sensitive operations.
/// Returns the password if verified, null if cancelled.
/// Biometric authentication is offered if the user has enabled it in settings.
///
/// Usage:
/// ```dart
/// final password = await showPasswordVerificationDialog(
///   context: context,
///   ref: ref,
///   onVerify: (password) => authNotifier.verifyPasswordForSensitiveData(password),
/// );
/// ```
Future<String?> showPasswordVerificationDialog({
  required BuildContext context,
  required WidgetRef ref,
  String? message,
  String? passwordHint,
  required Future<bool> Function(String password) onVerify,
}) async {
  // Capture context before async operations to avoid lint warning
  final dialogContext = context;
  final l10n = AppLocalizations.of(context);
  final effectiveMessage = message ?? AppLocalizations.of(context).passwordVerificationRestricted;

  // Check if biometric auth is available and enabled
  final biometricService = BiometricService.instance;
  final securityService = SecurityService.instance;
  await securityService.loadSettings();
  final isBiometricAvailable = await biometricService.isAvailable();
  final isBiometricEnabled = securityService.settings.biometricsEnabled ||
      securityService.settings.faceIdEnabled;

  // If biometric is available and enabled, offer it as an option
  if (isBiometricAvailable && isBiometricEnabled) {
    final dialogBuilder = BiometricPasswordDialogContent(
      message: effectiveMessage,
      passwordHint: passwordHint,
      onVerify: onVerify,
      biometricService: biometricService,
      securityService: securityService,
    );
    return showGeneralDialog<String>(
      context: dialogContext,
      barrierDismissible: false,
      barrierLabel: 'Dialog',
      pageBuilder: (context, anim1, anim2) => Center(
        child: SizedBox(width: 720, child: dialogBuilder),
      ),
    );
  }

  // Fall back to password-only dialog
  final passwordDialogContent = PasswordVerificationDialogContent(
    message: effectiveMessage,
    passwordHint: passwordHint,
    onVerify: onVerify,
  );
  return showGeneralDialog<String>(
    context: dialogContext,
    barrierDismissible: false,
    barrierLabel: 'Dialog',
    pageBuilder: (context, anim1, anim2) => Center(
      child: SizedBox(width: 720, child: passwordDialogContent),
    ),
  );
}

/// Password verification dialog content (public for testing).
class PasswordVerificationDialogContent extends StatefulWidget {
  const PasswordVerificationDialogContent({
    super.key,
    required this.message,
    this.passwordHint,
    required this.onVerify,
  });

  final String message;
  final String? passwordHint;
  final Future<bool> Function(String password) onVerify;

  @override
  State<PasswordVerificationDialogContent> createState() =>
      PasswordVerificationDialogContentState();
}

class PasswordVerificationDialogContentState
    extends State<PasswordVerificationDialogContent>
    with PasswordDialogOverlayMixin {
  final _controller = TextEditingController();
  final _focusNode = FocusNode();

  String? _errorMessage;
  bool _isVerifying = false;
  bool _hasError = false;
  bool _userHasTypedAfterError = false;
  bool _obscurePassword = true;

  @override
  void initState() {
    super.initState();
    _controller.addListener(_onTextChanged);
  }

  @override
  void dispose() {
    disposeOverlay();
    _controller.removeListener(_onTextChanged);
    // Securely clear password text before disposing
    _controller.text = '';
    _controller.dispose();
    _focusNode.dispose();
    super.dispose();
  }

  void _onTextChanged() {
    // Clear error only when user types after seeing an error
    if (_hasError && _controller.text.isNotEmpty && !_userHasTypedAfterError) {
      _userHasTypedAfterError = true;
      _hasError = false;
    }
    if (_hasError && _controller.text.isEmpty) {
      _userHasTypedAfterError = false;
    }
    setState(() {});
  }

  Future<void> _verify() async {
    if (_controller.text.isEmpty) return;
    setState(() => _isVerifying = true);
    final success = await widget.onVerify(_controller.text);
    if (!mounted) return;
    if (success) {
      Navigator.of(context).pop(_controller.text);
    } else {
      setState(() {
        _isVerifying = false;
        _errorMessage = AppLocalizations.of(context).passwordVerificationInvalid;
        _hasError = true;
        _userHasTypedAfterError = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final isPasswordEmpty = _controller.text.isEmpty;

    return AlertDialog(
      title: Row(
        children: [
          Icon(Icons.lock_outline, color: Colors.orange.shade700),
          const SizedBox(width: 8),
          Text(AppLocalizations.of(context).dialogVerifyIdentity),
        ],
      ),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
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
                    widget.message,
                    style: const TextStyle(fontSize: 13),
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 16),
          TextField(
            controller: _controller,
            focusNode: _focusNode,
            obscureText: _obscurePassword,
            autofocus: true,
            decoration: InputDecoration(
              labelText: AppLocalizations.of(context).loginMasterPassword,
              prefixIcon: const Icon(Icons.key),
              errorText: _hasError ? _errorMessage : null,
              errorStyle: TextStyle(
                color: Colors.red.shade700,
                fontWeight: FontWeight.w500,
              ),
              enabledBorder: AppTheme.passwordFieldEnabledBorder,
              errorBorder: AppTheme.passwordFieldErrorBorder,
              focusedErrorBorder: AppTheme.passwordFieldFocusedErrorBorder,
              suffixIcon: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  IconButton(
                      icon: Icon(
                        Icons.help_outline,
                        size: 20,
                        color: _hasError ? Colors.red.shade700 : null,
                      ),
                      onPressed: () => showHintOverlay(widget.passwordHint ?? AppLocalizations.of(context).loginNoPasswordHint),
                      tooltip: AppLocalizations.of(context).settingsShowPasswordHint,
                    ),
                  IconButton(
                    icon: Icon(
                      _obscurePassword
                          ? Icons.visibility_outlined
                          : Icons.visibility_off_outlined,
                      size: 20,
                      color: _hasError ? Colors.red.shade700 : null,
                    ),
                    onPressed: () {
                      setState(() {
                        _obscurePassword = !_obscurePassword;
                      });
                    },
                    tooltip: _obscurePassword ? AppLocalizations.of(context).commonShowPassword : AppLocalizations.of(context).commonHidePassword,
                  ),
                ],
              ),
            ),
            onSubmitted: (_) => _verify(),
          ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context, null),
          child: Text(AppLocalizations.of(context).commonCancel),
        ),
        ElevatedButton(
          onPressed: isPasswordEmpty ? null : _verify,
          child: _isVerifying
              ? const SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : Text(AppLocalizations.of(context).commonConfirm),
        ),
      ],
    );
  }
}

/// Biometric-enhanced password verification dialog with Touch ID/Face ID option
/// Biometric-enhanced password verification dialog content (public for testing).
class BiometricPasswordDialogContent extends StatefulWidget {
  const BiometricPasswordDialogContent({
    super.key,
    required this.message,
    this.passwordHint,
    required this.onVerify,
    required this.biometricService,
    required this.securityService,
  });

  final String message;
  final String? passwordHint;
  final Future<bool> Function(String password) onVerify;
  final BiometricService biometricService;
  final SecurityService securityService;

  @override
  State<BiometricPasswordDialogContent> createState() =>
      BiometricPasswordDialogContentState();
}

class BiometricPasswordDialogContentState
    extends State<BiometricPasswordDialogContent>
    with PasswordDialogOverlayMixin {
  final _controller = TextEditingController();
  final _focusNode = FocusNode();

  String? _errorMessage;
  bool _isVerifying = false;
  bool _hasError = false;
  bool _userHasTypedAfterError = false;
  bool _isBiometricVerified = false;
  bool _obscurePassword = true;

  @override
  void initState() {
    super.initState();
    _controller.addListener(_onTextChanged);
  }

  @override
  void dispose() {
    disposeOverlay();
    _controller.removeListener(_onTextChanged);
    // Securely clear password text before disposing
    _controller.text = '';
    _controller.dispose();
    _focusNode.dispose();
    super.dispose();
  }

  void _onTextChanged() {
    if (_hasError && _controller.text.isNotEmpty && !_userHasTypedAfterError) {
      _userHasTypedAfterError = true;
      _hasError = false;
    }
    if (_hasError && _controller.text.isEmpty) {
      _userHasTypedAfterError = false;
    }
    setState(() {});
  }

  Future<void> _tryBiometric() async {
    if (_isBiometricVerified || _isVerifying) return;

    final success = await widget.biometricService.authenticate(
      reason: AppLocalizations.of(context).dialogVerifyIdentity,
    );

    if (!mounted) return;

    if (success) {
      setState(() => _isBiometricVerified = true);
      Navigator.of(context).pop(_controller.text.isEmpty ? 'biometric' : _controller.text);
    }
  }

  Future<void> _verify() async {
    if (_controller.text.isEmpty) return;
    setState(() => _isVerifying = true);
    final success = await widget.onVerify(_controller.text);
    if (!mounted) return;
    if (success) {
      Navigator.of(context).pop(_controller.text);
    } else {
      setState(() {
        _isVerifying = false;
        _errorMessage = AppLocalizations.of(context).passwordVerificationInvalid;
        _hasError = true;
        _userHasTypedAfterError = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final isPasswordEmpty = _controller.text.isEmpty;
    final biometricType = widget.securityService.settings.faceIdEnabled
        ? AppLocalizations.of(context).loginBiometricFaceId
        : AppLocalizations.of(context).loginBiometricTouchId;

    return AlertDialog(
      title: Row(
        children: [
          Icon(Icons.lock_outline, color: Colors.orange.shade700),
          const SizedBox(width: 8),
          Text(AppLocalizations.of(context).dialogVerifyIdentity),
        ],
      ),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
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
                    widget.message,
                    style: const TextStyle(fontSize: 13),
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 16),
          if (!_isBiometricVerified) ...[
            SizedBox(
              width: double.infinity,
              child: OutlinedButton.icon(
                onPressed: _isVerifying ? null : _tryBiometric,
                icon: Icon(
                  widget.securityService.settings.faceIdEnabled
                      ? Icons.face_outlined
                      : Icons.fingerprint_outlined,
                ),
                label: Text(AppLocalizations.of(context).dialogUseBiometric(biometricType)),
              ),
            ),
            const SizedBox(height: 12),
            Row(
              children: [
                const Expanded(child: Divider()),
                Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 8),
                  child: Text(AppLocalizations.of(context).settingsOr, style: const TextStyle(color: Colors.grey)),
                ),
                const Expanded(child: Divider()),
              ],
            ),
            const SizedBox(height: 12),
          ],
          TextField(
            controller: _controller,
            focusNode: _focusNode,
            obscureText: _obscurePassword,
            autofocus: _isBiometricVerified,
            decoration: InputDecoration(
              labelText: AppLocalizations.of(context).loginMasterPassword,
              prefixIcon: const Icon(Icons.key),
              errorText: _hasError ? _errorMessage : null,
              errorStyle: TextStyle(
                color: Colors.red.shade700,
                fontWeight: FontWeight.w500,
              ),
              enabledBorder: AppTheme.passwordFieldEnabledBorder,
              errorBorder: AppTheme.passwordFieldErrorBorder,
              focusedErrorBorder: AppTheme.passwordFieldFocusedErrorBorder,
              suffixIcon: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  IconButton(
                      icon: Icon(
                        Icons.help_outline,
                        size: 20,
                        color: _hasError ? Colors.red.shade700 : null,
                      ),
                      onPressed: () => showHintOverlay(widget.passwordHint ?? AppLocalizations.of(context).loginNoPasswordHint),
                      tooltip: AppLocalizations.of(context).settingsShowPasswordHint,
                    ),
                  IconButton(
                    icon: Icon(
                      _obscurePassword
                          ? Icons.visibility_outlined
                          : Icons.visibility_off_outlined,
                      size: 20,
                      color: _hasError ? Colors.red.shade700 : null,
                    ),
                    onPressed: () {
                      setState(() {
                        _obscurePassword = !_obscurePassword;
                      });
                    },
                    tooltip: _obscurePassword ? AppLocalizations.of(context).commonShowPassword : AppLocalizations.of(context).commonHidePassword,
                  ),
                ],
              ),
            ),
            onSubmitted: (_) => _verify(),
          ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context, null),
          child: Text(AppLocalizations.of(context).commonCancel),
        ),
        ElevatedButton(
          onPressed: isPasswordEmpty ? null : _verify,
          child: _isVerifying
              ? const SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : Text(AppLocalizations.of(context).commonConfirm),
        ),
      ],
    );
  }
}
