import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
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
  final String? customMessage; // Optional override for auto-generated message

  const OperationMessage({
    required this.type,
    required this.section,
    this.itemName,
    this.fieldName,
    this.isPrivacyModeActive = false,
    this.customMessage,
  });

  /// Get the localized message text based on operation type
  /// Note: Privacy mode affects data display, not operation descriptions
  String getMessage(AppLocalizations l10n) {
    if (customMessage != null) return customMessage!;

    final name = fieldName?.isNotEmpty == true
        ? fieldName!
        : (itemName?.isNotEmpty == true ? itemName! : null);

    if (name != null) {
      return switch (type) {
        OperationType.create => l10n.operationNotifCreated(name),
        OperationType.update => l10n.operationNotifUpdated(name),
        OperationType.delete => l10n.operationNotifDeleted(name),
        OperationType.restore => l10n.operationNotifRestored(name),
        OperationType.purge => l10n.operationNotifPurged(name),
      };
    }

    // Fallback without name
    final sectionLabel = _getSectionLabel(l10n);
    return switch (type) {
      OperationType.create => 'Added new item to $sectionLabel',
      OperationType.update => 'Updated $sectionLabel',
      OperationType.delete => 'Deleted from $sectionLabel',
      OperationType.restore => 'Restored $sectionLabel',
      OperationType.purge => 'Permanently deleted from $sectionLabel',
    };
  }

  String _getSectionLabel(AppLocalizations l10n) {
    return _sectionDisplayName(l10n);
  }

  String _sectionDisplayName(AppLocalizations l10n) {
    // Match both stored names (e.g. 'Passports') and normalized keys.
    final s = section.toLowerCase();
    switch (s) {
      case 'identity':
      case 'identities':
        return l10n.logSectionIdentity;
      case 'contact':
      case 'contact information':
      case 'contacts':
        return l10n.logSectionContactInfo;
      case 'address':
      case 'addresses':
        return l10n.logSectionAddress;
      case 'id card':
      case 'id cards':
        return l10n.logSectionIdCard;
      case 'passport':
      case 'passports':
        return l10n.logSectionPassport;
      case 'visa':
      case 'visas':
        return l10n.logSectionVisa;
      case 'travel history':
      case 'travel histories':
        return l10n.logSectionTravelHistory;
      case 'bank account':
      case 'bank accounts':
        return l10n.logSectionBankAccount;
      case 'card':
      case 'cards':
        return l10n.logSectionCard;
      case 'education':
        return l10n.logSectionEducation;
      case 'employment':
      case 'employments':
        return l10n.logSectionEmployment;
      case 'skill':
      case 'skills':
        return l10n.logSectionSkill;
      case 'language':
      case 'languages':
        return l10n.logSectionLanguage;
      case 'travel':
        return l10n.logSectionTravel;
      case 'financial':
        return l10n.logSectionFinancial;
      case 'professional':
        return l10n.logSectionProfessional;
      default:
        return section;
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
  static bool _isInserted = false;

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
    _isInserted = false;

    final entry = OverlayEntry(
      builder: (context) => _NotificationWidget(
        message: message,
        onDismiss: dismiss,
        onUndo: onUndo,
        duration: duration,
      ),
    );
    _currentEntry = entry;

    // Defer insertion to after the current frame to avoid layout/hitTest issues
    // with FractionalTranslation (SlideTransition) during active gesture handling.
    WidgetsBinding.instance.addPostFrameCallback((_) {
      // Guard against multiple insert attempts when show() is called twice
      // in the same frame (the second call overwrites _currentEntry).
      if (_currentEntry != entry) return;
      if (_isInserted) return;
      overlay.insert(entry);
      _isInserted = true;
    });
  }

  /// Dismiss the current notification
  static void dismiss() {
    if (_currentEntry != null && _isInserted) {
      _currentEntry?.remove();
    }
    _currentEntry = null;
    _isInserted = false;
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
