import 'package:flutter/material.dart';

/// Inherited widget that provides action callbacks to child widgets
/// used inside form sections or object cards.
class EntryActionsContext extends InheritedWidget {
  final VoidCallback? onEdit;
  final VoidCallback? onDelete;
  final Future<void> Function(String)? onCopy;
  final VoidCallback? onToggleHistory;

  const EntryActionsContext({
    super.key,
    required super.child,
    this.onEdit,
    this.onDelete,
    this.onCopy,
    this.onToggleHistory,
  });

  static EntryActionsContext? of(BuildContext context) {
    return context.dependOnInheritedWidgetOfExactType<EntryActionsContext>();
  }

  @override
  bool updateShouldNotify(EntryActionsContext old) {
    return onEdit != old.onEdit ||
        onDelete != old.onDelete ||
        onCopy != old.onCopy ||
        onToggleHistory != old.onToggleHistory;
  }
}
