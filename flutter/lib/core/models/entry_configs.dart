/// Configuration for which action buttons to show on an entry item.
class EntryActionsConfig {
  final bool showCopy;
  final bool showEdit;
  final bool showDelete;
  final bool showHistory;

  const EntryActionsConfig({
    this.showCopy = true,
    this.showEdit = true,
    this.showDelete = true,
    this.showHistory = true,
  });

  static const all = EntryActionsConfig();
  static const readOnly = EntryActionsConfig(showEdit: false, showDelete: false);
  static const noHistory = EntryActionsConfig(showHistory: false);
}
