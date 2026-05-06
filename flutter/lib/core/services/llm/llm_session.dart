import 'dart:convert';
import 'dart:math';

import 'llm_backend_type.dart';

/// Lightweight session handle for tracking LLM usage and applying
/// per-account rate limits / privacy gates.
class LlmSession {
  final String sessionId;
  final DateTime createdAt;
  final LlmBackendType backend;

  LlmSession._(this.sessionId, this.backend) : createdAt = DateTime.now();

  factory LlmSession.create(LlmBackendType backend) {
    final id = _generateSessionId();
    return LlmSession._(id, backend);
  }

  static String _generateSessionId() {
    final bytes = List<int>.generate(16, (_) => Random.secure().nextInt(256));
    return base64Url.encode(bytes).substring(0, 22);
  }
}
