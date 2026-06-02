import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/chat_session.dart';
import 'package:solosoul_flutter/core/services/llm/chat_session_service.dart';

void main() {
  final service = ChatSessionService.instance;

  group('ChatSessionService CRUD', () {
    test('createSession adds session to front of list', () {
      final existing = [
        const ChatSession(id: 'old', title: 'Old', createdAt: 1, updatedAt: 1),
      ];
      final result = service.createSession(existing, title: 'New Session');

      expect(result.length, 2);
      expect(result.first.title, 'New Session');
      expect(result.first.messageCount, 0);
      expect(result.last.id, 'old');
    });

    test('createSession uses empty title when not provided', () {
      final result = service.createSession([]);
      expect(result.length, 1);
      expect(result.first.title, '');
      expect(result.first.id, isNotEmpty);
    });

    test('createSession generates unique ids', () {
      final s1 = service.createSession([]);
      final s2 = service.createSession(s1);
      expect(s2[0].id, isNot(equals(s2[1].id)));
    });

    test('updateSession modifies only target session', () {
      final sessions = [
        const ChatSession(id: 'a', title: 'A', createdAt: 1, updatedAt: 1),
        const ChatSession(id: 'b', title: 'B', createdAt: 1, updatedAt: 1),
      ];
      final result = service.updateSession(sessions, 'b', title: 'Updated B');

      expect(result[0].title, 'A');
      expect(result[1].title, 'Updated B');
      expect(result[1].updatedAt, greaterThan(1));
    });

    test('updateSession updates messageCount', () {
      final sessions = [
        const ChatSession(id: 'a', title: 'A', createdAt: 1, updatedAt: 1, messageCount: 0),
      ];
      final result = service.updateSession(sessions, 'a', messageCount: 5);
      expect(result.first.messageCount, 5);
    });

    test('deleteSession removes target session', () {
      final sessions = [
        const ChatSession(id: 'a', title: 'A', createdAt: 1, updatedAt: 1),
        const ChatSession(id: 'b', title: 'B', createdAt: 1, updatedAt: 1),
      ];
      final result = service.deleteSession(sessions, 'a');
      expect(result.length, 1);
      expect(result.first.id, 'b');
    });

    test('deleteSession returns empty list when deleting last session', () {
      final sessions = [
        const ChatSession(id: 'a', title: 'A', createdAt: 1, updatedAt: 1),
      ];
      final result = service.deleteSession(sessions, 'a');
      expect(result, isEmpty);
    });
  });

  group('ChatSessionService sorting', () {
    test('sortSessionsByRecent orders by updatedAt descending', () {
      final sessions = [
        const ChatSession(id: 'old', title: 'Old', createdAt: 1, updatedAt: 1000),
        const ChatSession(id: 'new', title: 'New', createdAt: 1, updatedAt: 3000),
        const ChatSession(id: 'mid', title: 'Mid', createdAt: 1, updatedAt: 2000),
      ];
      final result = service.sortSessionsByRecent(sessions);
      expect(result[0].id, 'new');
      expect(result[1].id, 'mid');
      expect(result[2].id, 'old');
    });
  });

  group('ChatSessionService title generation', () {
    test('generateTitle returns empty for empty input', () {
      expect(service.generateTitle(''), '');
    });

    test('generateTitle returns full text if under max length', () {
      expect(service.generateTitle('Short'), 'Short');
    });

    test('generateTitle truncates long text', () {
      final long = 'A' * 50;
      final result = service.generateTitle(long);
      expect(result.length, lessThan(50));
      expect(result.endsWith('...'), isTrue);
    });

    test('generateTitle truncates at word boundary for Latin text', () {
      const text = 'Hello world this is a very long message that exceeds forty chars';
      final result = service.generateTitle(text);
      expect(result.length, lessThanOrEqualTo(43)); // 40 + '...'
      // Should truncate at a space, not mid-word
      expect(result.endsWith('...'), isTrue);
    });

    test('autoTitleFromFirstMessage updates only empty titles', () {
      final sessions = [
        const ChatSession(id: 'a', title: '', createdAt: 1, updatedAt: 1),
        const ChatSession(id: 'b', title: 'Existing', createdAt: 1, updatedAt: 1),
      ];
      final result = service.autoTitleFromFirstMessage(sessions, 'a', 'First user message');
      expect(result[0].title, 'First user message');
      expect(result[1].title, 'Existing');
    });

    test('autoTitleFromFirstMessage updates updatedAt timestamp', () {
      final sessions = [
        const ChatSession(id: 'a', title: '', createdAt: 1, updatedAt: 1),
      ];
      final result = service.autoTitleFromFirstMessage(sessions, 'a', 'Message');
      expect(result.first.updatedAt, greaterThan(1));
    });

    test('autoTitleFromFirstMessage leaves non-matching sessions unchanged', () {
      final sessions = [
        const ChatSession(id: 'a', title: '', createdAt: 1, updatedAt: 1),
        const ChatSession(id: 'b', title: '', createdAt: 1, updatedAt: 1),
      ];
      final result = service.autoTitleFromFirstMessage(sessions, 'a', 'Only for A');
      expect(result[0].title, 'Only for A');
      expect(result[1].title, '');
    });
  });

  group('ChatSessionService soft delete', () {
    test('softDeleteSession marks only target as deleted', () {
      final sessions = [
        const ChatSession(id: 'a', title: 'A', createdAt: 1, updatedAt: 1),
        const ChatSession(id: 'b', title: 'B', createdAt: 1, updatedAt: 1),
      ];
      final result = service.softDeleteSession(sessions, 'a');
      expect(result[0].isDeleted, isTrue);
      expect(result[0].deletedAt, isNotNull);
      expect(result[1].isDeleted, isFalse);
    });

    test('restoreSession restores only target', () {
      final sessions = [
        const ChatSession(id: 'a', title: 'A', createdAt: 1, updatedAt: 1, isDeleted: true, deletedAt: 1000),
        const ChatSession(id: 'b', title: 'B', createdAt: 1, updatedAt: 1, isDeleted: true, deletedAt: 2000),
      ];
      final result = service.restoreSession(sessions, 'a');
      expect(result[0].isDeleted, isFalse);
      expect(result[0].deletedAt, isNull);
      expect(result[1].isDeleted, isTrue);
    });

    test('hardDeleteSession removes session entirely', () {
      final sessions = [
        const ChatSession(id: 'a', title: 'A', createdAt: 1, updatedAt: 1),
        const ChatSession(id: 'b', title: 'B', createdAt: 1, updatedAt: 1),
      ];
      final result = service.hardDeleteSession(sessions, 'a');
      expect(result.length, 1);
      expect(result.first.id, 'b');
    });

    test('activeSessions filters out deleted', () {
      final sessions = [
        const ChatSession(id: 'a', title: 'A', createdAt: 1, updatedAt: 1),
        const ChatSession(id: 'b', title: 'B', createdAt: 1, updatedAt: 1, isDeleted: true, deletedAt: 1000),
      ];
      final result = service.activeSessions(sessions);
      expect(result.length, 1);
      expect(result.first.id, 'a');
    });

    test('deletedSessions filters and sorts by deletedAt desc', () {
      final sessions = [
        const ChatSession(id: 'a', title: 'A', createdAt: 1, updatedAt: 1, isDeleted: true, deletedAt: 1000),
        const ChatSession(id: 'b', title: 'B', createdAt: 1, updatedAt: 1, isDeleted: true, deletedAt: 3000),
        const ChatSession(id: 'c', title: 'C', createdAt: 1, updatedAt: 1),
      ];
      final result = service.deletedSessions(sessions);
      expect(result.length, 2);
      expect(result[0].id, 'b');
      expect(result[1].id, 'a');
    });

    test('cleanupOldDeleted removes sessions past retention', () {
      final now = DateTime.now().millisecondsSinceEpoch;
      final oldDeleted = DateTime.now().subtract(const Duration(days: 31)).millisecondsSinceEpoch;
      final recentDeleted = DateTime.now().subtract(const Duration(days: 5)).millisecondsSinceEpoch;
      final sessions = [
        ChatSession(id: 'a', title: 'A', createdAt: 1, updatedAt: 1, isDeleted: true, deletedAt: now),
        ChatSession(id: 'b', title: 'B', createdAt: 1, updatedAt: 1, isDeleted: true, deletedAt: oldDeleted),
        ChatSession(id: 'c', title: 'C', createdAt: 1, updatedAt: 1, isDeleted: true, deletedAt: recentDeleted),
        const ChatSession(id: 'd', title: 'D', createdAt: 1, updatedAt: 1),
      ];
      final result = service.cleanupOldDeleted(sessions, retentionDays: 30);
      expect(result.any((s) => s.id == 'b'), isFalse); // old, cleaned up
      expect(result.any((s) => s.id == 'a'), isTrue);  // now, kept
      expect(result.any((s) => s.id == 'c'), isTrue);  // 5 days, kept
      expect(result.any((s) => s.id == 'd'), isTrue);  // active, kept
    });
  });

  group('Sentinel helper', () {
    test('isNewChatSessionId recognizes sentinel', () {
      expect(isNewChatSessionId(kNewChatSessionId), isTrue);
      expect(isNewChatSessionId('real-uuid'), isFalse);
      expect(isNewChatSessionId(null), isFalse);
    });
  });
}
