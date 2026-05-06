import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/llm/llm_backend_type.dart';
import 'package:solosoul_flutter/core/services/llm/llm_session.dart';

void main() {
  group('LlmSession', () {
    test('create generates a session with cloud backend', () {
      final session = LlmSession.create(LlmBackendType.cloud);
      expect(session.backend, LlmBackendType.cloud);
      expect(session.sessionId, isNotEmpty);
      expect(session.sessionId.length, 22);
      expect(session.createdAt.isBefore(DateTime.now().add(const Duration(seconds: 1))), isTrue);
    });

    test('create generates a session with local backend', () {
      final session = LlmSession.create(LlmBackendType.local);
      expect(session.backend, LlmBackendType.local);
      expect(session.sessionId, isNotEmpty);
    });

    test('sessionId is unique across creations', () {
      final s1 = LlmSession.create(LlmBackendType.cloud);
      final s2 = LlmSession.create(LlmBackendType.cloud);
      expect(s1.sessionId, isNot(equals(s2.sessionId)));
    });

    test('sessionId uses URL-safe base64 characters only', () {
      final session = LlmSession.create(LlmBackendType.local);
      expect(
        session.sessionId,
        matches(r'^[A-Za-z0-9_-]{22}$'),
      );
    });
  });
}
