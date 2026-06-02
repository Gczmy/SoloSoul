import 'package:uuid/uuid.dart';

import 'package:solosoul_flutter/core/models/chat_session.dart';

const _uuid = Uuid();

// =============================================================================
// Chat Session Service
// =============================================================================

/// Pure-function service for managing chat session lists.
///
/// All methods return new list instances (immutable operations),
/// following the same pattern as [UnifiedObjectService].
class ChatSessionService {
  ChatSessionService._();
  static final ChatSessionService instance = ChatSessionService._();

  // ---------------------------------------------------------------------------
  // CRUD Operations
  // ---------------------------------------------------------------------------

  /// Create a new session and append it to the list.
  ///
  /// [title] — optional initial title. If null/empty, the UI layer
  /// should display a default label (e.g. "New Chat").
  /// The new session is inserted at the beginning of the list
  /// (most recent first).
  List<ChatSession> createSession(
    List<ChatSession> sessions, {
    String? title,
  }) {
    final now = DateTime.now().millisecondsSinceEpoch;
    final newSession = ChatSession(
      id: _uuid.v4(),
      title: title ?? '',
      createdAt: now,
      updatedAt: now,
      messageCount: 0,
    );
    return [newSession, ...sessions];
  }

  /// Update a session's properties.
  List<ChatSession> updateSession(
    List<ChatSession> sessions,
    String sessionId, {
    String? title,
    int? messageCount,
  }) {
    return sessions.map((s) {
      if (s.id != sessionId) return s;
      return s.copyWith(
        title: title,
        messageCount: messageCount,
        updatedAt: DateTime.now().millisecondsSinceEpoch,
      );
    }).toList();
  }

  /// Delete a session from the list (hard delete).
  List<ChatSession> deleteSession(List<ChatSession> sessions, String sessionId) {
    return sessions.where((s) => s.id != sessionId).toList();
  }

  // ---------------------------------------------------------------------------
  // Soft Delete / Restore / Hard Delete
  // ---------------------------------------------------------------------------

  /// Soft-delete a session (move to trash).
  List<ChatSession> softDeleteSession(List<ChatSession> sessions, String sessionId) {
    final now = DateTime.now().millisecondsSinceEpoch;
    return sessions.map((s) {
      if (s.id != sessionId) return s;
      return s.copyWith(
        isDeleted: true,
        deletedAt: now,
        updatedAt: now,
      );
    }).toList();
  }

  /// Restore a soft-deleted session back to active.
  List<ChatSession> restoreSession(List<ChatSession> sessions, String sessionId) {
    return sessions.map((s) {
      if (s.id != sessionId) return s;
      return s.copyWith(
        isDeleted: false,
        clearDeletedAt: true,
        // 恢复时不修改 updatedAt，保留对话最后更新时间
      );
    }).toList();
  }

  /// Hard-delete a session permanently (remove from list entirely).
  List<ChatSession> hardDeleteSession(List<ChatSession> sessions, String sessionId) {
    return sessions.where((s) => s.id != sessionId).toList();
  }

  /// Remove sessions that have been soft-deleted for longer than [retentionDays].
  List<ChatSession> cleanupOldDeleted(
    List<ChatSession> sessions, {
    int retentionDays = 30,
  }) {
    final cutoff = DateTime.now().subtract(Duration(days: retentionDays)).millisecondsSinceEpoch;
    return sessions.where((s) {
      if (!s.isDeleted || s.deletedAt == null) return true;
      return s.deletedAt! > cutoff;
    }).toList();
  }

  /// Filter active (non-deleted) sessions.
  List<ChatSession> activeSessions(List<ChatSession> sessions) {
    return sessions.where((s) => !s.isDeleted).toList();
  }

  /// Filter deleted sessions, sorted by deletedAt desc.
  List<ChatSession> deletedSessions(List<ChatSession> sessions) {
    final filtered = sessions.where((s) => s.isDeleted).toList();
    filtered.sort((a, b) => (b.deletedAt ?? 0).compareTo(a.deletedAt ?? 0));
    return filtered;
  }

  /// Sort sessions by most recently updated (descending).
  List<ChatSession> sortSessionsByRecent(List<ChatSession> sessions) {
    final sorted = List<ChatSession>.from(sessions);
    sorted.sort((a, b) => b.updatedAt.compareTo(a.updatedAt));
    return sorted;
  }

  // ---------------------------------------------------------------------------
  // Title Generation
  // ---------------------------------------------------------------------------

  /// Generate a display title from the first user message.
  ///
  /// Smart truncation rules:
  /// - Max 40 characters
  /// - For CJK text: truncates at character boundary
  /// - For Latin text: prefers word boundary (space)
  /// - Falls back to mid-word truncation if no good boundary found
  /// - Appends "..." if truncated
  String generateTitle(String firstUserMessage) {
    if (firstUserMessage.isEmpty) return '';

    const maxLen = 40;
    if (firstUserMessage.length <= maxLen) return firstUserMessage;

    // Try to find a good truncation point
    var cutPoint = maxLen;

    // For Latin text: try to find a space within the last 10 chars before maxLen
    const searchStart = maxLen - 10;
    for (var i = maxLen; i >= searchStart && i > 0; i--) {
      if (firstUserMessage[i - 1] == ' ') {
        cutPoint = i - 1;
        break;
      }
    }

    return '${firstUserMessage.substring(0, cutPoint)}...';
  }

  /// Update a session's title based on the first user message.
  /// Only updates if the current title is empty (default / unnamed).
  List<ChatSession> autoTitleFromFirstMessage(
    List<ChatSession> sessions,
    String sessionId,
    String firstUserMessage,
  ) {
    return sessions.map((s) {
      if (s.id != sessionId) return s;
      if (s.title.isNotEmpty) return s;
      return s.copyWith(
        title: generateTitle(firstUserMessage),
        updatedAt: DateTime.now().millisecondsSinceEpoch,
      );
    }).toList();
  }
}
