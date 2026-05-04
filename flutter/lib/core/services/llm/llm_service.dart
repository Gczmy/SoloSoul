import 'dart:convert';
import 'dart:math';

// =============================================================================
// LLM Backend Type
// =============================================================================

/// Supported LLM inference backends.
enum LlmBackendType {
  /// Local model inference via Rust native layer (e.g. llama.cpp / candle).
  local,

  /// Cloud API (OpenAI, Anthropic, self-hosted endpoint).
  cloud,
}

// =============================================================================
// LLM Service Interface
// =============================================================================

/// Abstract interface for LLM inference — concrete implementations provided
/// per backend ([LlmLocalService], [LlmCloudService]).
///
/// **Thread safety:** All implementations must guarantee that [infer] is safe
/// to call concurrently. Heavy local inference runs on a Rust background
/// thread pool; cloud calls are isolated per [LlmSession].
abstract class LlmService {
  /// Perform a single-turn inference.
  ///
  /// [prompt]   – sanitized prompt string (caller must strip PII before cloud).
  /// [maxTokens] – hard cap on output length.
  ///
  /// Returns the generated text, or throws [LlmException] on failure.
  Future<String> infer(String prompt, {int maxTokens = 512});
}

// =============================================================================
// Local LLM Stub
// =============================================================================

/// Placeholder for local model inference.
///
/// TODO(P1): Wire to Rust `frb_llm_infer()` once native layer exposes it.
class LlmLocalService implements LlmService {
  static final LlmLocalService _instance = LlmLocalService._();
  static LlmLocalService get instance => _instance;
  LlmLocalService._();

  @override
  Future<String> infer(String prompt, {int maxTokens = 512}) async {
    // Stub: returns deterministic placeholder text for UI layout testing.
    await Future<void>.delayed(const Duration(milliseconds: 200));
    return '[Local LLM 占位响应] 提示词长度 ${prompt.length} 字符，'
        '最大输出 $maxTokens tokens。';
  }
}

// =============================================================================
// Cloud LLM Stub
// =============================================================================

/// Placeholder for cloud LLM inference (OpenAI-compatible API).
///
/// TODO(P1): Replace with real HTTP client (Dio + retries + timeout).
class LlmCloudService implements LlmService {
  final String apiKey;
  final String endpoint;
  final String model;

  LlmCloudService({
    required this.apiKey,
    this.endpoint = 'https://api.openai.com/v1',
    this.model = 'gpt-4o-mini',
  });

  @override
  Future<String> infer(String prompt, {int maxTokens = 512}) async {
    // Stub: simulates network latency.
    await Future<void>.delayed(const Duration(milliseconds: 800));
    return '[Cloud LLM 占位响应] 模型 $model，提示词 ${prompt.length} 字符。';
  }
}

// =============================================================================
// Session Manager
// =============================================================================

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

// =============================================================================
// Exceptions
// =============================================================================

class LlmException implements Exception {
  final String message;
  final LlmErrorCode code;

  const LlmException(this.message, {this.code = LlmErrorCode.unknown});

  @override
  String toString() => 'LlmException[$code]: $message';
}

enum LlmErrorCode {
  unknown,
  timeout,
  network,
  unauthorized,
  rateLimited,
  privacyBlocked,
  modelNotFound,
}
