import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/biometric_service.dart';
import 'package:solosoul_flutter/core/services/security_service.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';

// ignore_for_file: use_build_context_synchronously

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
  String message = 'Restricted field. Enter your master password to proceed.',
  String? passwordHint,
  required Future<bool> Function(String password) onVerify,
}) async {
  // Capture context before async operations to avoid lint warning
  final dialogContext = context;

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
      message: message,
      passwordHint: passwordHint,
      onVerify: onVerify,
      biometricService: biometricService,
      securityService: securityService,
    );
    return showDialog<String>(
      context: dialogContext,
      barrierDismissible: false,
      builder: (dialogContext) => dialogBuilder,
    );
  }

  // Fall back to password-only dialog
  final passwordDialogContent = PasswordVerificationDialogContent(
    message: message,
    passwordHint: passwordHint,
    onVerify: onVerify,
  );
  return showDialog<String>(
    context: dialogContext,
    barrierDismissible: false,
    builder: (dialogContext) => passwordDialogContent,
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
    extends State<PasswordVerificationDialogContent> {
  final _controller = TextEditingController();
  final _focusNode = FocusNode();

  String? _errorMessage;
  bool _isVerifying = false;
  bool _hasError = false;
  bool _userHasTypedAfterError = false;
  bool _obscurePassword = true;

  // Hint overlay tracking
  OverlayEntry? _hintOverlayEntry;
  Timer? _hintOverlayTimer;

  @override
  void initState() {
    super.initState();
    _controller.addListener(_onTextChanged);
  }

  @override
  void dispose() {
    _hintOverlayTimer?.cancel();
    _hintOverlayEntry?.remove();
    _hintOverlayEntry = null;
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

  void _showHintOverlay(String hint) {
    // Cancel any existing timer and remove existing overlay
    _hintOverlayTimer?.cancel();
    _hintOverlayEntry?.remove();

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
                      'Password Hint: $hint',
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
                    onPressed: () {
                      _hintOverlayTimer?.cancel();
                      _hintOverlayEntry?.remove();
                      _hintOverlayEntry = null;
                    },
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );

    overlay.insert(_hintOverlayEntry!);
    _hintOverlayTimer = Timer(const Duration(seconds: 4), () {
      _hintOverlayEntry?.remove();
      _hintOverlayEntry = null;
    });
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
        _errorMessage = 'Invalid password';
        _hasError = true;
        _userHasTypedAfterError = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final isPasswordEmpty = _controller.text.isEmpty;

    return AlertDialog(
      title: Row(
        children: [
          Icon(Icons.lock_outline, color: Colors.orange.shade700),
          const SizedBox(width: 8),
          const Text('Verify Identity'),
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
              labelText: 'Master Password',
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
                      icon: const Icon(Icons.help_outline, size: 20),
                      onPressed: () => _showHintOverlay(widget.passwordHint ?? 'No password hint available'),
                      tooltip: 'Show password hint',
                    ),
                  IconButton(
                    icon: Icon(
                      _obscurePassword
                          ? Icons.visibility_outlined
                          : Icons.visibility_off_outlined,
                      size: 20,
                    ),
                    onPressed: () {
                      setState(() {
                        _obscurePassword = !_obscurePassword;
                      });
                    },
                    tooltip: _obscurePassword ? 'Show password' : 'Hide password',
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
          child: const Text('Cancel'),
        ),
        ElevatedButton(
          onPressed: isPasswordEmpty ? null : _verify,
          child: _isVerifying
              ? const SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Text('Verify'),
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
    extends State<BiometricPasswordDialogContent> {
  final _controller = TextEditingController();
  final _focusNode = FocusNode();

  String? _errorMessage;
  bool _isVerifying = false;
  bool _hasError = false;
  bool _userHasTypedAfterError = false;
  bool _isBiometricVerified = false;
  bool _obscurePassword = true;

  // Hint overlay tracking
  OverlayEntry? _hintOverlayEntry;
  Timer? _hintOverlayTimer;

  @override
  void initState() {
    super.initState();
    _controller.addListener(_onTextChanged);
  }

  @override
  void dispose() {
    _hintOverlayTimer?.cancel();
    _hintOverlayEntry?.remove();
    _hintOverlayEntry = null;
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
      reason: 'Verify your identity',
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
        _errorMessage = 'Invalid password';
        _hasError = true;
        _userHasTypedAfterError = false;
      });
    }
  }

  void _showHintOverlay(String hint) {
    // Cancel any existing timer and remove existing overlay
    _hintOverlayTimer?.cancel();
    _hintOverlayEntry?.remove();

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
                      'Password Hint: $hint',
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
                    onPressed: () {
                      _hintOverlayTimer?.cancel();
                      _hintOverlayEntry?.remove();
                      _hintOverlayEntry = null;
                    },
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );

    overlay.insert(_hintOverlayEntry!);
    _hintOverlayTimer = Timer(const Duration(seconds: 4), () {
      _hintOverlayEntry?.remove();
      _hintOverlayEntry = null;
    });
  }

  @override
  Widget build(BuildContext context) {
    final isPasswordEmpty = _controller.text.isEmpty;
    final biometricType = widget.securityService.settings.faceIdEnabled
        ? 'Face ID'
        : 'Touch ID';

    return AlertDialog(
      title: Row(
        children: [
          Icon(Icons.lock_outline, color: Colors.orange.shade700),
          const SizedBox(width: 8),
          const Text('Verify Identity'),
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
            controller: _controller,
            focusNode: _focusNode,
            obscureText: _obscurePassword,
            autofocus: _isBiometricVerified,
            decoration: InputDecoration(
              labelText: 'Master Password',
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
                      icon: const Icon(Icons.help_outline, size: 20),
                      onPressed: () => _showHintOverlay(widget.passwordHint ?? 'No password hint available'),
                      tooltip: 'Show password hint',
                    ),
                  IconButton(
                    icon: Icon(
                      _obscurePassword
                          ? Icons.visibility_outlined
                          : Icons.visibility_off_outlined,
                      size: 20,
                    ),
                    onPressed: () {
                      setState(() {
                        _obscurePassword = !_obscurePassword;
                      });
                    },
                    tooltip: _obscurePassword ? 'Show password' : 'Hide password',
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
          child: const Text('Cancel'),
        ),
        ElevatedButton(
          onPressed: isPasswordEmpty ? null : _verify,
          child: _isVerifying
              ? const SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Text('Verify'),
        ),
      ],
    );
  }
}
