import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitivity_blurred_widget.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart' hide SensitivityLevel;

/// Main widget that displays content based on sensitivity level.
///
/// Behavior by level:
/// - public: Direct display
/// - internal: Plain text, edit requires verification
/// - sensitive: Blurred/masked display, click/copy requires verification
/// - critical: Deep masked, only visible within unlock duration
class SensitivityBasedVisibilityWidget extends ConsumerStatefulWidget {
  final String fieldId;
  final String value;
  final SensitivityLevel sensitivityLevel;
  final List<String> tags;
  final Widget Function(String value)? plainTextBuilder;
  final Widget Function(String value)? maskedBuilder;
  final Widget Function(String value, VoidCallback onReveal)? blurredBuilder;
  final VoidCallback? onCopy;
  final VoidCallback? onEdit;

  const SensitivityBasedVisibilityWidget({
    super.key,
    required this.fieldId,
    required this.value,
    required this.sensitivityLevel,
    this.tags = const [],
    this.plainTextBuilder,
    this.maskedBuilder,
    this.blurredBuilder,
    this.onCopy,
    this.onEdit,
  });

  @override
  ConsumerState<SensitivityBasedVisibilityWidget> createState() =>
      _SensitivityBasedVisibilityWidgetState();
}

class _SensitivityBasedVisibilityWidgetState
    extends ConsumerState<SensitivityBasedVisibilityWidget> {
  bool _isRevealed = false;
  bool _isVerifying = false;
  Timer? _autoHideTimer;

  @override
  void dispose() {
    _autoHideTimer?.cancel();
    super.dispose();
  }

  void _cancelAutoHide() {
    _autoHideTimer?.cancel();
    _autoHideTimer = null;
  }

  void _startAutoHideTimer({Duration duration = const Duration(seconds: 30)}) {
    _cancelAutoHide();
    _autoHideTimer = Timer(duration, () {
      if (mounted) {
        setState(() => _isRevealed = false);
      }
    });
  }

  Future<void> _handleReveal() async {
    if (_isVerifying) return;

    setState(() => _isVerifying = true);

    try {
      final authNotifier = ref.read(authNotifierProvider.notifier);
      final selectedAccount = authNotifier.selectedAccount;

      final password = await showPasswordVerificationDialog(
        context: context,
        ref: ref,
        message: _getVerificationMessage(),
        passwordHint: selectedAccount?.passwordHint,
        onVerify: authNotifier.verifyPasswordForSensitiveData,
      );

      if (!mounted) return;

      if (password != null) {
        ref.read(sensitivePageAccessProvider.notifier).markVerified();
        setState(() => _isRevealed = true);
        _startAutoHideTimer();
      }
    } finally {
      if (mounted) {
        setState(() => _isVerifying = false);
      }
    }
  }

  String _getVerificationMessage() {
    switch (widget.sensitivityLevel) {
      case SensitivityLevel.public:
        return 'Enter your master password';
      case SensitivityLevel.internal:
        return 'Internal field. Enter password to edit.';
      case SensitivityLevel.sensitive:
        return 'Sensitive field. Enter password to view.';
      case SensitivityLevel.critical:
        return 'Critical field. Enter master password to view.';
    }
  }

  Future<void> _handleCopy() async {
    if (!_isRevealed && widget.sensitivityLevel != SensitivityLevel.public) {
      await _handleReveal();
      if (!_isRevealed) return;
    }

    await Clipboard.setData(ClipboardData(text: widget.value));
    if (mounted) {
      showOverlaySnackBar(
        context,
        content: 'Copied to clipboard',
        type: SnackBarType.success,
      );
    }
    widget.onCopy?.call();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final style = theme.textTheme.bodyMedium;

    // Check recent verification for auto-reveal
    final sensitiveAccess = ref.watch(sensitivePageAccessProvider);
    final oneMinuteAgo = DateTime.now().subtract(const Duration(minutes: 1));
    final hasRecentVerification = sensitiveAccess.lastVerified != null &&
        sensitiveAccess.lastVerified!.isAfter(oneMinuteAgo);

    switch (widget.sensitivityLevel) {
      case SensitivityLevel.public:
        return _buildPublicView(style);

      case SensitivityLevel.internal:
        return _buildInternalView(style, hasRecentVerification);

      case SensitivityLevel.sensitive:
        return _buildSensitiveView(style, hasRecentVerification);

      case SensitivityLevel.critical:
        return _buildCriticalView(style, hasRecentVerification);
    }
  }

  Widget _buildPublicView(TextStyle? style) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Flexible(
          child: widget.plainTextBuilder?.call(widget.value) ??
              SelectableText(widget.value, style: style),
        ),
        const SizedBox(width: 8),
        InkWell(
          onTap: _handleCopy,
          borderRadius: BorderRadius.circular(4),
          child: Padding(
            padding: const EdgeInsets.all(4),
            child: Icon(
              Icons.copy,
              size: 16,
              color: Theme.of(context).colorScheme.onSurfaceVariant,
            ),
          ),
        ),
      ],
    );
  }

  Widget _buildInternalView(TextStyle? style, bool hasRecentVerification) {
    if (!hasRecentVerification && !_isRevealed) {
      return _buildLockedView(style, 'Internal');
    }

    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Flexible(
          child: widget.plainTextBuilder?.call(widget.value) ??
              SelectableText(widget.value, style: style),
        ),
        const SizedBox(width: 8),
        InkWell(
          onTap: _handleCopy,
          borderRadius: BorderRadius.circular(4),
          child: Padding(
            padding: const EdgeInsets.all(4),
            child: Icon(
              Icons.copy,
              size: 16,
              color: Theme.of(context).colorScheme.onSurfaceVariant,
            ),
          ),
        ),
        if (widget.onEdit != null) ...[
          const SizedBox(width: 4),
          InkWell(
            onTap: widget.onEdit,
            borderRadius: BorderRadius.circular(4),
            child: Padding(
              padding: const EdgeInsets.all(4),
              child: Icon(
                Icons.edit,
                size: 16,
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
            ),
          ),
        ],
      ],
    );
  }

  Widget _buildSensitiveView(TextStyle? style, bool hasRecentVerification) {
    final effectiveRevealed = _isRevealed || hasRecentVerification;

    if (!effectiveRevealed) {
      return _buildLockedView(style, 'Sensitive');
    }

    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Flexible(
          child: widget.maskedBuilder?.call(widget.value) ??
                widget.blurredBuilder?.call(widget.value, _handleReveal) ??
                BlurredText(
                  text: widget.value,
                  isBlurred: false,
                  style: style,
                ),
        ),
        const SizedBox(width: 8),
        if (_isVerifying)
          const SizedBox(
            width: 16,
            height: 16,
            child: CircularProgressIndicator(strokeWidth: 2),
          )
        else
          InkWell(
            onTap: _handleReveal,
            borderRadius: BorderRadius.circular(4),
            child: Padding(
              padding: const EdgeInsets.all(4),
              child: Icon(
                _isRevealed ? Icons.visibility : Icons.visibility_off,
                size: 16,
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
            ),
          ),
        const SizedBox(width: 4),
        InkWell(
          onTap: _handleCopy,
          borderRadius: BorderRadius.circular(4),
          child: Padding(
            padding: const EdgeInsets.all(4),
            child: Icon(
              Icons.copy,
              size: 16,
              color: Theme.of(context).colorScheme.onSurfaceVariant,
            ),
          ),
        ),
      ],
    );
  }

  Widget _buildCriticalView(TextStyle? style, bool hasRecentVerification) {
    final effectiveRevealed = _isRevealed || hasRecentVerification;

    if (!effectiveRevealed) {
      return _buildDeepMaskedView(style);
    }

    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Flexible(
          child: MaskedText(
            value: widget.value,
            isMasked: false,
            style: style,
          ),
        ),
        const SizedBox(width: 8),
        if (_isVerifying)
          const SizedBox(
            width: 16,
            height: 16,
            child: CircularProgressIndicator(strokeWidth: 2),
          )
        else
          InkWell(
            onTap: _handleReveal,
            borderRadius: BorderRadius.circular(4),
            child: Padding(
              padding: const EdgeInsets.all(4),
              child: Icon(
                _isRevealed ? Icons.visibility : Icons.visibility_off,
                size: 16,
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
            ),
          ),
        const SizedBox(width: 4),
        InkWell(
          onTap: _handleCopy,
          borderRadius: BorderRadius.circular(4),
          child: Padding(
            padding: const EdgeInsets.all(4),
            child: Icon(
              Icons.copy,
              size: 16,
              color: Theme.of(context).colorScheme.onSurfaceVariant,
            ),
          ),
        ),
      ],
    );
  }

  Widget _buildLockedView(TextStyle? style, String label) {
    return InkWell(
      onTap: _handleReveal,
      borderRadius: BorderRadius.circular(4),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            if (_isVerifying)
              const SizedBox(
                width: 16,
                height: 16,
                child: CircularProgressIndicator(strokeWidth: 2),
              )
            else ...[
              Icon(
                Icons.lock,
                size: 16,
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
              const SizedBox(width: 4),
            ],
            Text(
              '$label - Tap to unlock',
              style: style?.copyWith(
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildDeepMaskedView(TextStyle? style) {
    return InkWell(
      onTap: _handleReveal,
      borderRadius: BorderRadius.circular(4),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            if (_isVerifying)
              const SizedBox(
                width: 16,
                height: 16,
                child: CircularProgressIndicator(strokeWidth: 2),
              )
            else ...[
              Icon(
                Icons.shield,
                size: 16,
                color: Theme.of(context).colorScheme.error,
              ),
              const SizedBox(width: 4),
            ],
            Text(
              '••••••••',
              style: style?.copyWith(
                fontFamily: 'monospace',
                letterSpacing: 2,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Provider to get effective sensitivity level for a field.
final effectiveFieldLevelProvider =
    Provider.family<SensitivityLevel, ({String fieldId, List<String> tags})>(
        (ref, params) {
  final style = ref.watch(accountStyleProvider);
  return sensitivityResolver.resolve(
    fieldId: params.fieldId,
    fieldSettings: style.fieldSettings,
    revealedFields: style.revealedFields,
    tags: params.tags,
  );
});

/// Convenience widget that automatically resolves sensitivity level
/// and uses AccountStyleProvider for configuration.
class AutoSensitivityWidget extends ConsumerWidget {
  final String fieldId;
  final String value;
  final List<String> tags;
  final Widget Function(String value)? plainTextBuilder;
  final Widget Function(String value)? maskedBuilder;
  final Widget Function(String value, VoidCallback onReveal)? blurredBuilder;

  const AutoSensitivityWidget({
    super.key,
    required this.fieldId,
    required this.value,
    this.tags = const [],
    this.plainTextBuilder,
    this.maskedBuilder,
    this.blurredBuilder,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final level = ref.watch(effectiveFieldLevelProvider(
      (fieldId: fieldId, tags: tags),
    ));

    return SensitivityBasedVisibilityWidget(
      fieldId: fieldId,
      value: value,
      sensitivityLevel: level,
      tags: tags,
      plainTextBuilder: plainTextBuilder,
      maskedBuilder: maskedBuilder,
      blurredBuilder: blurredBuilder,
    );
  }
}