import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';
import 'package:solosoul_flutter/core/services/clipboard_monitor_service.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    show showOverlaySnackBar, SnackBarType;

/// Widget that displays a value with sensitivity-based masking.
/// - Public fields: Always shown as plaintext
/// - Private fields: Plaintext when privacy shield is OFF, masked when ON
/// - Restricted fields: Always masked, requires password verification to reveal
class SensitiveValueWidget extends ConsumerStatefulWidget {
  final String fieldId;
  final String value;
  final Widget? child; // Optional custom display widget
  /// Optional explicit sensitivity level. When provided, overrides provider lookup.
  final SensitivityLevel? sensitivityLevel;

  const SensitiveValueWidget({
    super.key,
    required this.fieldId,
    required this.value,
    this.child,
    this.sensitivityLevel,
  });

  @override
  ConsumerState<SensitiveValueWidget> createState() => _SensitiveValueWidgetState();
}

class _SensitiveValueWidgetState extends ConsumerState<SensitiveValueWidget> {
  bool _isRevealed = false;
  bool _userExplicitlyHid = false;
  Timer? _autoHideTimer;
  bool _isVerifying = false;

  @override
  void dispose() {
    _autoHideTimer?.cancel();
    super.dispose();
  }

  void _cancelAutoHide() {
    _autoHideTimer?.cancel();
    _autoHideTimer = null;
  }

  void _startAutoHideTimer() {
    _cancelAutoHide();
    // Auto-hide after 30 seconds
    _autoHideTimer = Timer(const Duration(seconds: 30), () {
      if (mounted) {
        setState(() {
          _isRevealed = false;
          _userExplicitlyHid = true;
        });
      }
    });
  }

  Future<void> _handleTap() async {
    // If already revealed, toggle to hide
    if (_isRevealed) {
      _cancelAutoHide();
      setState(() {
        _isRevealed = false;
        _userExplicitlyHid = true;
      });
      return;
    }

    // Check if this field requires verification
    final level = ref.read(effectiveSensitivityProvider(widget.fieldId));
    final isRestricted = level == SensitivityLevel.critical;

    if (isRestricted) {
      // Check if user was verified within the valid duration (1 minute)
      if (ref.read(isSensitiveAccessGrantedProvider)) {
        // Skip password dialog, just reveal
        setState(() => _isRevealed = true);
        _startAutoHideTimer();
      } else {
        // Need password verification for restricted fields
        await _verifyAndReveal();
      }
    } else {
      // Private fields in privacy mode - just reveal
      setState(() => _isRevealed = true);
      _startAutoHideTimer();
    }
  }

  Future<void> _verifyAndReveal() async {
    if (_isVerifying) return;

    setState(() => _isVerifying = true);

    // Show password dialog
    final authNotifier = ref.read(authNotifierProvider.notifier);
    final selectedAccount = authNotifier.selectedAccount;
    // Use verifyPasswordForSensitiveData instead of unlockVault to avoid auth state changes
    // unlockVault changes auth state to loading/unlocked which triggers page rebuilds
    // and causes the revealed state to be lost
    final password = await showPasswordVerificationDialog(
      context: context,
      ref: ref,
      message: 'Restricted field. Enter your master password to view.',
      passwordHint: selectedAccount?.passwordHint,
      onVerify: authNotifier.verifyPasswordForSensitiveData,
    );

    // Always reset _isVerifying when dialog closes (success, cancel, or error)
    if (!mounted) return;
    setState(() => _isVerifying = false);

    if (password == null) {
      // User cancelled - just return, do not reveal
      return;
    }

    // Mark as verified in shared sensitive page access
    ref.read(sensitivePageAccessProvider.notifier).markVerified();

    setState(() => _isRevealed = true);
    _startAutoHideTimer();
  }

  String _maskedValue(String value) {
    // Short values (dates, short IDs, phone numbers ≤ 12 chars) are fully masked
    if (value.length <= 12) {
      return '••••••••';
    }
    // Partial masking: show first 4 and last 4 characters
    // e.g., "6222••••••••1234"
    return '${value.substring(0, 4)}••••••••${value.substring(value.length - 4)}';
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    // Watch sensitivity settings to rebuild when they change
    final displayMode = ref.watch(
      accountStyleProvider.select((s) => s.value?.displayMode),
    );
    final isPrivacyShieldEnabled = displayMode == SensitivityDisplayMode.hidePrivate;
    final SensitivityLevel fieldLevel = widget.sensitivityLevel ??
        ref.watch(effectiveSensitivityProvider(widget.fieldId));

    // Watch sensitive page access to detect recent verification
    final hasRecentVerification = ref.watch(isSensitiveAccessGrantedProvider);

    // Privacy mode is ON when hidePrivate or hideAll is enabled
    final isPrivacyModeOn = isPrivacyShieldEnabled ||
        displayMode == SensitivityDisplayMode.hideAll;

    bool shouldMask = switch (fieldLevel) {
      SensitivityLevel.public => false,
      SensitivityLevel.internal => isPrivacyModeOn,
      SensitivityLevel.sensitive => true,
      // Critical fields: mask when privacy mode is ON, otherwise require verification
      SensitivityLevel.critical => isPrivacyModeOn || !hasRecentVerification,
    };

    // Public fields: show plaintext without button
    if (!shouldMask) {
      return widget.child ?? SelectableText(widget.value);
    }

    // For restricted fields, check if recently verified - if so, auto-reveal
    // But if user explicitly hid it, respect their choice
    // Auto-mask: when privacy mode is ON, critical fields are never auto-revealed
    final bool revealed = (fieldLevel == SensitivityLevel.critical &&
        hasRecentVerification &&
        !_userExplicitlyHid &&
        !isPrivacyModeOn)
        ? true
        : _isRevealed;
    final String displayText = revealed ? widget.value : _maskedValue(widget.value);
    final bool isMasked = !revealed;
    final IconData icon = _isVerifying
        ? Icons.hourglass_empty
        : (revealed ? Icons.visibility : Icons.visibility_off);

    // Separate text (selectable, no toggle) from icon button (toggles visibility)
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        // Text is selectable and does NOT toggle visibility
        Flexible(
          child: SelectableText(
            displayText,
            style: theme.textTheme.bodyMedium?.copyWith(
              fontFamily: isMasked ? 'monospace' : null,
              letterSpacing: isMasked ? 2 : null,
            ),
          ),
        ),
        const SizedBox(width: 4),
        // Eye button toggles visibility (for all masked fields)
        if (_isVerifying)
          const SizedBox(
            width: 16,
            height: 16,
            child: CircularProgressIndicator(strokeWidth: 2),
          )
        else
          InkWell(
            onTap: _handleTap,
            borderRadius: BorderRadius.circular(4),
            child: Padding(
              padding: const EdgeInsets.all(2),
              child: Icon(
                icon,
                size: 16,
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ),
        const SizedBox(width: 2),
        // Copy button only visible when revealed
        if (revealed)
          InkWell(
            onTap: () {
              Clipboard.setData(ClipboardData(text: widget.value));
              ClipboardMonitorService.instance.notifySensitiveCopied();
              showOverlaySnackBar(
                context,
                content: 'Copied to clipboard',
                type: SnackBarType.success,
                duration: const Duration(seconds: 1),
              );
            },
            borderRadius: BorderRadius.circular(4),
            child: Padding(
              padding: const EdgeInsets.all(2),
              child: Icon(
                Icons.copy,
                size: 16,
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ),
      ],
    );
  }
}

/// Convenience widget for displaying a masked field value with label
class SensitiveFieldTile extends StatelessWidget {
  final String label;
  final String fieldId;
  final String value;
  final bool isEmpty;

  const SensitiveFieldTile({
    super.key,
    required this.label,
    required this.fieldId,
    required this.value,
    this.isEmpty = false,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  label,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
                const SizedBox(height: 2),
                if (isEmpty)
                  Text(
                    'Tap to add',
                    style: theme.textTheme.bodyLarge?.copyWith(
                      color: theme.colorScheme.primary,
                    ),
                  )
                else
                  SensitiveValueWidget(
                    fieldId: fieldId,
                    value: value,
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
