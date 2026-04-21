import 'package:flutter/material.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';

/// Operation type for notification messages
enum OperationType {
  create,
  update,
  delete,
  restore,
  purge,
}

/// Structured message model for operation notifications
/// Messages are privacy-aware and don't expose sensitive details when privacy mode is on
class OperationMessage {
  final OperationType type;
  final String section; // identity, travel, financial, professional
  final String? itemName; // The name of the item (hidden in privacy mode)
  final String? fieldName; // The specific field that was modified
  final bool isPrivacyModeActive;

  const OperationMessage({
    required this.type,
    required this.section,
    this.itemName,
    this.fieldName,
    this.isPrivacyModeActive = false,
  });

  /// Get the message text in English based on operation type
  /// Note: Privacy mode affects data display, not operation descriptions
  String get message {
    final sectionLabel = _getSectionLabel();

    switch (type) {
      case OperationType.create:
        // Use fieldName if available (the specific item name like "email - work")
        if (fieldName != null && fieldName!.isNotEmpty) {
          return 'Added $fieldName to $sectionLabel';
        }
        if (itemName != null && itemName!.isNotEmpty) {
          return 'Added $itemName to $sectionLabel';
        }
        // Fallback: provide more descriptive message based on section
        return 'Added new item to $sectionLabel';

      case OperationType.update:
        // Use fieldName if available (the specific item name like "email - work")
        if (fieldName != null && fieldName!.isNotEmpty) {
          return 'Updated $fieldName in $sectionLabel';
        }
        if (itemName != null && itemName!.isNotEmpty) {
          return 'Updated $itemName in $sectionLabel';
        }
        return 'Updated $sectionLabel';

      case OperationType.delete:
        // Use fieldName if available (the specific item name like "email - work")
        if (fieldName != null && fieldName!.isNotEmpty) {
          return 'Deleted $fieldName from $sectionLabel';
        }
        if (itemName != null && itemName!.isNotEmpty) {
          return 'Deleted $itemName from $sectionLabel';
        }
        return 'Deleted from $sectionLabel';

      case OperationType.restore:
        // Use fieldName if available (the specific item name like "email - work")
        if (fieldName != null && fieldName!.isNotEmpty) {
          return 'Restored $fieldName to $sectionLabel';
        }
        if (itemName != null && itemName!.isNotEmpty) {
          return 'Restored $itemName to $sectionLabel';
        }
        return 'Restored $sectionLabel';

      case OperationType.purge:
        // Use fieldName if available (the specific item name like "email - work")
        if (fieldName != null && fieldName!.isNotEmpty) {
          return 'Permanently deleted $fieldName';
        }
        if (itemName != null && itemName!.isNotEmpty) {
          return 'Permanently deleted $itemName';
        }
        return 'Permanently deleted from $sectionLabel';
    }
  }

  String _getSectionLabel() {
    // Section name is not sensitive info, always show it
    return _sectionDisplayName;
  }

  String get _sectionDisplayName {
    switch (section) {
      case 'identity':
        return 'Identity';
      case 'contact information':
        return 'Contact Information';
      case 'address':
        return 'Address';
      case 'id card':
        return 'ID Card';
      case 'passport':
        return 'Passport';
      case 'visa':
        return 'Visa';
      case 'travel history':
        return 'Travel History';
      case 'bank account':
        return 'Bank Account';
      case 'card':
        return 'Card';
      case 'education':
        return 'Education';
      case 'employment':
        return 'Employment';
      case 'skill':
        return 'Skill';
      case 'language':
        return 'Language';
      case 'travel':
        return 'Travel';
      case 'financial':
        return 'Financial';
      case 'professional':
        return 'Professional';
      default:
        return section; // Use the raw section value if not matched
    }
  }

  /// Get icon for the operation type
  IconData get icon {
    switch (type) {
      case OperationType.create:
        return Icons.add_circle_outline;
      case OperationType.update:
        return Icons.edit_outlined;
      case OperationType.delete:
        return Icons.delete_outline;
      case OperationType.restore:
        return Icons.restore;
      case OperationType.purge:
        return Icons.delete_forever;
    }
  }

  /// Get snackbar type for the operation
  SnackBarType get snackBarType {
    switch (type) {
      case OperationType.create:
        return SnackBarType.success;
      case OperationType.update:
        return SnackBarType.info;
      case OperationType.delete:
        return SnackBarType.warning;
      case OperationType.restore:
        return SnackBarType.info;
      case OperationType.purge:
        return SnackBarType.error;
    }
  }
}

/// Operation notification service for showing top-floating feedback
/// Avoids bottom tab bar and provides privacy-aware messages
class OperationNotification {
  static OverlayEntry? _currentEntry;

  /// Show a top-floating notification for operation feedback
  static void show(
    BuildContext context, {
    required OperationMessage message,
    VoidCallback? onUndo,
    Duration duration = const Duration(seconds: 3),
  }) {
    // Remove any existing notification
    dismiss();

    final overlay = Overlay.of(context);

    _currentEntry = OverlayEntry(
      builder: (context) => _NotificationWidget(
        message: message,
        onDismiss: dismiss,
        onUndo: onUndo,
        duration: duration,
      ),
    );

    overlay.insert(_currentEntry!);
  }

  /// Dismiss the current notification
  static void dismiss() {
    _currentEntry?.remove();
    _currentEntry = null;
  }
}

class _NotificationWidget extends StatefulWidget {
  final OperationMessage message;
  final VoidCallback onDismiss;
  final VoidCallback? onUndo;
  final Duration duration;

  const _NotificationWidget({
    required this.message,
    required this.onDismiss,
    this.onUndo,
    required this.duration,
  });

  @override
  State<_NotificationWidget> createState() => _NotificationWidgetState();
}

class _NotificationWidgetState extends State<_NotificationWidget>
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

    _controller.forward();

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
      widget.onDismiss();
    });
  }

  @override
  Widget build(BuildContext context) {
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
                        message.message,
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
                        child: const Text(
                          'Undo',
                          style: TextStyle(
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                      ),
                    IconButton(
                      onPressed: _dismiss,
                      icon: const Icon(Icons.close, color: Colors.white70, size: 18),
                      padding: EdgeInsets.zero,
                      constraints: const BoxConstraints(),
                      tooltip: 'Dismiss',
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
