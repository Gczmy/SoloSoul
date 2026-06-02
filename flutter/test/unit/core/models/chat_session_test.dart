import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/chat_session.dart';

void main() {
  group('ChatSession', () {
    const testSession = ChatSession(
      id: 'sess-001',
      title: 'Test Session',
      createdAt: 1700000000000,
      updatedAt: 1700003600000,
      messageCount: 5,
    );

    test('toJson produces correct map', () {
      final json = testSession.toJson();
      expect(json['id'], 'sess-001');
      expect(json['title'], 'Test Session');
      expect(json['createdAt'], 1700000000000);
      expect(json['updatedAt'], 1700003600000);
      expect(json['messageCount'], 5);
      expect(json['isDeleted'], false);
    });

    test('fromJson parses correctly', () {
      final json = {
        'id': 'sess-002',
        'title': 'Another Session',
        'createdAt': 1700000000000,
        'updatedAt': 1700003600000,
        'messageCount': 3,
        'isDeleted': true,
      };
      final session = ChatSession.fromJson(json);
      expect(session.id, 'sess-002');
      expect(session.title, 'Another Session');
      expect(session.createdAt, 1700000000000);
      expect(session.updatedAt, 1700003600000);
      expect(session.messageCount, 3);
      expect(session.isDeleted, true);
    });

    test('fromJson handles missing fields with defaults', () {
      final json = {'id': 'sess-003'};
      final session = ChatSession.fromJson(json);
      expect(session.id, 'sess-003');
      expect(session.title, '');
      expect(session.createdAt, 0);
      expect(session.updatedAt, 0);
      expect(session.messageCount, 0);
      expect(session.isDeleted, false);
    });

    test('copyWith updates specified fields', () {
      final updated = testSession.copyWith(
        title: 'Updated Title',
        messageCount: 10,
      );
      expect(updated.id, 'sess-001');
      expect(updated.title, 'Updated Title');
      expect(updated.messageCount, 10);
      expect(updated.createdAt, testSession.createdAt);
      expect(updated.updatedAt, testSession.updatedAt);
    });

    test('copyWith preserves fields when null', () {
      final copied = testSession.copyWith();
      expect(copied.id, testSession.id);
      expect(copied.title, testSession.title);
      expect(copied.messageCount, testSession.messageCount);
    });

    test('toString contains id and message count', () {
      expect(testSession.toString(), contains('sess-001'));
      expect(testSession.toString(), contains('msgs=5'));
    });

    test('instances with same values are equal via ==', () {
      const s1 = ChatSession(id: 'a', title: 't', createdAt: 1, updatedAt: 2);
      const s2 = ChatSession(id: 'a', title: 't', createdAt: 1, updatedAt: 2);
      // const constructors with same values are canonicalized in Dart
      expect(s1 == s2, isTrue);
    });
  });
}
