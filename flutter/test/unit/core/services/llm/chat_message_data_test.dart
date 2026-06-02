import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/llm/chat_history_service.dart';

void main() {
  group('ChatMessageData', () {
    const msg = ChatMessageData(
      id: 'msg-001',
      text: 'Hello, AI!',
      isUser: true,
      createdAt: 1700000000000,
    );

    test('toJson produces correct map', () {
      final json = msg.toJson();
      expect(json['id'], 'msg-001');
      expect(json['text'], 'Hello, AI!');
      expect(json['isUser'], true);
      expect(json['createdAt'], 1700000000000);
    });

    test('fromJson parses correctly', () {
      final json = {
        'id': 'msg-002',
        'text': 'Response from AI',
        'isUser': false,
        'createdAt': 1700000000001,
      };
      final parsed = ChatMessageData.fromJson(json);
      expect(parsed.id, 'msg-002');
      expect(parsed.text, 'Response from AI');
      expect(parsed.isUser, false);
      expect(parsed.createdAt, 1700000000001);
    });

    test('fromJson handles missing createdAt (backward compatibility)', () {
      final json = {
        'id': 'msg-003',
        'text': 'Legacy message',
        'isUser': true,
      };
      final parsed = ChatMessageData.fromJson(json);
      expect(parsed.createdAt, 0);
    });

    test('round-trip serialization', () {
      final json = msg.toJson();
      final restored = ChatMessageData.fromJson(json);
      expect(restored.id, msg.id);
      expect(restored.text, msg.text);
      expect(restored.isUser, msg.isUser);
      expect(restored.createdAt, msg.createdAt);
    });
  });
}
