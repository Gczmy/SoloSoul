import 'package:flutter_riverpod/flutter_riverpod.dart';

// =============================================================================
// Selected Chat Session ID Provider
// =============================================================================

/// Holds the ID of the currently active chat session.
///
/// Special values:
/// - `kNewChatSessionId` ('__new__'): temporary new-chat state (not persisted).
///   Entered when user clicks "New Chat". No session exists in storage yet.
/// - `null`: no session selected (e.g. on vault lock).
/// - Real UUID: an actual persisted session.
///
/// The [chatSessionListProvider] auto-selects the most recent session on load
/// if this value is null and active sessions exist.
class _SelectedChatSessionIdNotifier extends Notifier<String?> {
  @override
  String? build() => null;
  void select(String? id) => state = id;
}

final selectedChatSessionIdProvider =
    NotifierProvider<_SelectedChatSessionIdNotifier, String?>(
  () => _SelectedChatSessionIdNotifier(),
);
