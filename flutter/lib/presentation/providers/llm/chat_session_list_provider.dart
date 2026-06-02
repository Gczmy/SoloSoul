import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:solosoul_flutter/core/models/chat_session.dart';
import 'package:solosoul_flutter/core/services/llm/chat_history_service.dart';
import 'package:solosoul_flutter/core/services/llm/chat_session_service.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_notifier.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_types.dart';
import 'package:solosoul_flutter/presentation/providers/llm/selected_chat_session_provider.dart';

// =============================================================================
// Chat Session List Provider
// =============================================================================

/// Manages the list of chat sessions for the current account.
///
/// **Lifecycle:**
/// - Vault unlocked: loads session list from encrypted storage (with auto-migration).
/// - Vault locked: clears state.
/// - Changes are debounce-saved (2s) to Vault.
///
/// **Soft Delete:**
/// - Sessions can be soft-deleted (moved to Trash) or hard-deleted (permanent).
/// - Auto-cleanup runs daily: sessions in trash > 30 days are permanently removed.
class ChatSessionListNotifier extends AsyncNotifier<List<ChatSession>> {
  Timer? _saveTimer;
  static const _saveDebounce = Duration(seconds: 2);
  static const _cleanupInterval = Duration(hours: 24);
  static const _retentionDays = 30;

  int? _lastCleanupAt;

  @override
  Future<List<ChatSession>> build() async {
    final authAsync = ref.watch(authNotifierProvider);
    final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;

    ref.onDispose(() {
      _saveTimer?.cancel();
    });

    if (authAsync.value != AuthState.unlocked || accountId == null) {
      // Vault locked or no account: clear selection and return empty
      // ignore: unawaited_futures
      Future.microtask(() {
        ref.read(selectedChatSessionIdProvider.notifier).select(null);
      });
      return [];
    }

    // Load sessions (auto-migration happens inside loadSessionList)
    final sessions = await ChatHistoryService.instance.loadSessionList(accountId);

    // Daily auto-cleanup of old trashed sessions
    final cleaned = _runCleanupIfNeeded(accountId, sessions);

    // Sort: active first (by recent), then deleted (by deletedAt desc)
    final active = ChatSessionService.instance.activeSessions(cleaned);
    final sortedActive = ChatSessionService.instance.sortSessionsByRecent(active);

    // Auto-select logic
    final selectedId = ref.read(selectedChatSessionIdProvider);
    _ensureSelection(sortedActive, selectedId);

    return cleaned;
  }

  // ---------------------------------------------------------------------------
  // Cleanup
  // ---------------------------------------------------------------------------

  List<ChatSession> _runCleanupIfNeeded(String accountId, List<ChatSession> sessions) {
    final now = DateTime.now().millisecondsSinceEpoch;
    if (_lastCleanupAt != null && (now - _lastCleanupAt!) < _cleanupInterval.inMilliseconds) {
      return sessions;
    }

    final cleaned = ChatSessionService.instance.cleanupOldDeleted(sessions, retentionDays: _retentionDays);
    final removedCount = sessions.length - cleaned.length;
    if (removedCount > 0) {
      SoloLog.d('ChatSessionList', 'auto-cleanup removed $removedCount old trashed sessions');
      // ignore: unawaited_futures
      _saveImmediately(accountId, cleaned);
    }
    _lastCleanupAt = now;
    return cleaned;
  }

  // ---------------------------------------------------------------------------
  // Selection Helpers
  // ---------------------------------------------------------------------------

  void _ensureSelection(
    List<ChatSession> activeSessions,
    String? currentSelectedId,
  ) {
    // If currently in temporary new-chat mode, keep it
    if (isNewChatSessionId(currentSelectedId)) {
      return;
    }

    if (activeSessions.isEmpty) {
      // No active sessions: enter temporary new-chat state
      // ignore: unawaited_futures
      Future.microtask(() {
        ref.read(selectedChatSessionIdProvider.notifier).select(kNewChatSessionId);
      });
      return;
    }

    // If nothing selected, select the most recent active session
    if (currentSelectedId == null) {
      // ignore: unawaited_futures
      Future.microtask(() {
        ref.read(selectedChatSessionIdProvider.notifier).select(activeSessions.first.id);
      });
      return;
    }

    // If selected session no longer exists (was deleted or doesn't exist), select first active
    final exists = activeSessions.any((s) => s.id == currentSelectedId);
    if (!exists) {
      // ignore: unawaited_futures
      Future.microtask(() {
        ref.read(selectedChatSessionIdProvider.notifier).select(activeSessions.first.id);
      });
    }
  }

  String? _findNextSelectionId(List<ChatSession> activeSessions, String removedId) {
    if (activeSessions.isEmpty) return kNewChatSessionId;
    // Try to select the session that was previously at index 0 (most recent)
    // If removed was first, select new first; otherwise keep current sort
    return activeSessions.first.id;
  }

  // ---------------------------------------------------------------------------
  // Persistence
  // ---------------------------------------------------------------------------

  void _debouncedSave() {
    _saveTimer?.cancel();
    _saveTimer = Timer(_saveDebounce, _saveCurrentList);
  }

  Future<void> _saveCurrentList() async {
    final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
    if (accountId == null) return;
    final sessions = state.value ?? [];
    await ChatHistoryService.instance.saveSessionList(accountId, sessions);
  }

  Future<void> _saveImmediately(
    String accountId,
    List<ChatSession> sessions,
  ) async {
    await ChatHistoryService.instance.saveSessionList(accountId, sessions);
  }

  // ---------------------------------------------------------------------------
  // Public API
  // ---------------------------------------------------------------------------

  /// Create a new session and auto-select it.
  /// Called from sendMessage when the user sends the first message in temp mode.
  void createSession({String? title}) {
    final current = state.value ?? [];
    final updated = ChatSessionService.instance.createSession(current, title: title);
    state = AsyncData(updated);
    _debouncedSave();

    // Auto-select the new session (inserted at beginning)
    final newSession = updated.first;
    ref.read(selectedChatSessionIdProvider.notifier).select(newSession.id);
    SoloLog.d('ChatSessionList', 'created session ${newSession.id}');
  }

  /// Update a session's title.
  void updateSessionTitle(String sessionId, String newTitle) {
    final current = state.value ?? [];
    final updated = ChatSessionService.instance.updateSession(
      current,
      sessionId,
      title: newTitle,
    );
    state = AsyncData(updated);
    _debouncedSave();
  }

  /// Update message count and timestamp after new messages arrive.
  void updateSessionStats(String sessionId, int messageCount) {
    final current = state.value ?? [];
    final updated = ChatSessionService.instance.updateSession(
      current,
      sessionId,
      messageCount: messageCount,
    );
    state = AsyncData(updated);
    _debouncedSave();
  }

  /// Auto-generate title from first user message if still default.
  void autoTitleFromMessage(String sessionId, String firstUserMessage) {
    final current = state.value ?? [];
    final updated = ChatSessionService.instance.autoTitleFromFirstMessage(
      current,
      sessionId,
      firstUserMessage,
    );
    state = AsyncData(updated);
    _debouncedSave();
  }

  // ---------------------------------------------------------------------------
  // Soft Delete / Restore / Hard Delete
  // ---------------------------------------------------------------------------

  /// Soft-delete a session (move to Trash).
  void softDeleteSession(String sessionId) {
    final current = state.value ?? [];
    final updated = ChatSessionService.instance.softDeleteSession(current, sessionId);
    state = AsyncData(updated);
    _debouncedSave();

    // Handle selection change
    final selectedId = ref.read(selectedChatSessionIdProvider);
    if (selectedId == sessionId) {
      final active = ChatSessionService.instance.activeSessions(updated);
      final nextId = _findNextSelectionId(active, sessionId);
      ref.read(selectedChatSessionIdProvider.notifier).select(nextId);
    }
    SoloLog.d('ChatSessionList', 'soft-deleted session $sessionId');
  }

  /// Restore a soft-deleted session from Trash back to active.
  void restoreSession(String sessionId) {
    final current = state.value ?? [];
    final updated = ChatSessionService.instance.restoreSession(current, sessionId);
    state = AsyncData(updated);
    _debouncedSave();
    SoloLog.d('ChatSessionList', 'restored session $sessionId');
  }

  /// Hard-delete a session permanently (from Trash).
  void hardDeleteSession(String sessionId) {
    final current = state.value ?? [];
    final updated = ChatSessionService.instance.hardDeleteSession(current, sessionId);
    state = AsyncData(updated);
    _debouncedSave();

    // Delete message file permanently
    final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
    if (accountId != null) {
      ChatHistoryService.instance.deleteSessionMessages(accountId, sessionId);
    }
    SoloLog.d('ChatSessionList', 'hard-deleted session $sessionId');
  }

  /// Empty Trash: permanently delete all soft-deleted sessions.
  void emptyTrash() {
    final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
    if (accountId == null) return;

    final current = state.value ?? [];
    final toDelete = ChatSessionService.instance.deletedSessions(current);
    final updated = ChatSessionService.instance.activeSessions(current);

    // Delete all message files
    for (final s in toDelete) {
      ChatHistoryService.instance.deleteSessionMessages(accountId, s.id);
    }

    state = AsyncData(updated);
    _saveImmediately(accountId, updated);
    SoloLog.d('ChatSessionList', 'emptied trash (${toDelete.length} sessions)');
  }

  /// Clear all sessions and enter temporary new-chat state.
  void resetSessions() {
    final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
    if (accountId == null) return;

    // Delete all message files
    final sessions = state.value ?? [];
    for (final s in sessions) {
      ChatHistoryService.instance.deleteSessionMessages(accountId, s.id);
    }

    state = const AsyncData([]);
    _saveImmediately(accountId, []);
    ref.read(selectedChatSessionIdProvider.notifier).select(kNewChatSessionId);
  }
}

final chatSessionListProvider =
    AsyncNotifierProvider<ChatSessionListNotifier, List<ChatSession>>(
  () => ChatSessionListNotifier(),
);
