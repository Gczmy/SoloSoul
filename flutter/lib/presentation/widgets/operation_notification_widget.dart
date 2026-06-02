import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/services/operation_notification.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart' show AppTheme, SnackBarType;

/// A top-floating notification widget for operation feedback.
/// Displayed via [OperationNotification.show] using the Overlay API.
class OperationNotificationWidget extends StatefulWidget {
  final OperationMessage message;
  final VoidCallback onDismiss;
  final VoidCallback? onUndo;
  final Duration duration;

  const OperationNotificationWidget({
    super.key,
    required this.message,
    required this.onDismiss,
    this.onUndo,
    required this.duration,
  });

  @override
  State<OperationNotificationWidget> createState() =>
      _OperationNotificationWidgetState();
}

class _OperationNotificationWidgetState
    extends State<OperationNotificationWidget>
    with SingleTickerProviderStateMixin {
  late AnimationController _controller;
  late Animation<double> _fadeAnimation;
  late Animation<Offset> _slideAnimation;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      duration: const Duration(milliseconds: 300),
      vsync: this,
    );

    _fadeAnimation = Tween<double>(begin: 0.0, end: 1.0).animate(
      CurvedAnimation(parent: _controller, curve: Curves.easeOut),
    );

    _slideAnimation = Tween<Offset>(
      begin: const Offset(0, -1),
      end: Offset.zero,
    ).animate(CurvedAnimation(parent: _controller, curve: Curves.easeOut));

    // Defer animation start until after the first layout to avoid
    // hitTest accessing RenderFractionalTranslation before it's laid out.
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) {
        _controller.forward();
      }
    });

    // Auto dismiss after duration
    Future.delayed(widget.duration, () {
      if (mounted) {
        _dismiss();
      }
    });
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _dismiss() {
    _controller.reverse().then((_) {
      if (mounted) {
        widget.onDismiss();
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final message = widget.message;

    // Get colors based on snackbar type
    final (bgColor, icon, iconColor) = switch (message.snackBarType) {
      SnackBarType.info => (
          Theme.of(context).colorScheme.inverseSurface,
          message.icon,
          Theme.of(context).colorScheme.primary,
        ),
      SnackBarType.success => (
          AppTheme.successColor.withValues(alpha: 0.95),
          message.icon,
          Colors.white,
        ),
      SnackBarType.warning => (
          Colors.orange.shade700,
          message.icon,
          Colors.white,
        ),
      SnackBarType.error => (
          AppTheme.errorColor.withValues(alpha: 0.95),
          message.icon,
          Colors.white,
        ),
    };

    // Position below status bar and app bar (kToolbarHeight + top padding)
    final topOffset = MediaQuery.of(context).padding.top +
        kToolbarHeight +
        8;

    return Positioned(
      top: topOffset,
      left: 16.0,
      right: 16.0,
      child: SafeArea(
        child: FadeTransition(
          opacity: _fadeAnimation,
          child: SlideTransition(
            position: _slideAnimation,
            child: Material(
              color: Colors.transparent,
              child: Container(
                padding: const EdgeInsets.symmetric(
                  horizontal: 16.0,
                  vertical: 14.0,
                ),
                decoration: BoxDecoration(
                  color: bgColor,
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
                    Icon(icon, color: iconColor, size: 22),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Text(
                        message.getMessage(l10n),
                        style: const TextStyle(
                          color: Colors.white,
                          fontSize: 14.0,
                          fontWeight: FontWeight.w500,
                        ),
                      ),
                    ),
                    if (widget.onUndo != null)
                      TextButton(
                        onPressed: () {
                          widget.onUndo!();
                          _dismiss();
                        },
                        style: TextButton.styleFrom(
                          foregroundColor: Colors.white,
                          padding: const EdgeInsets.symmetric(horizontal: 12),
                          minimumSize: Size.zero,
                          tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                        ),
                        child: Text(
                          l10n.operationNotifUndo,
                          style: const TextStyle(
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                      ),
                    IconButton(
                      onPressed: _dismiss,
                      icon: const Icon(Icons.close, color: Colors.white70, size: 18),
                      padding: EdgeInsets.zero,
                      constraints: const BoxConstraints(),
                      tooltip: l10n.operationNotifDismiss,
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
