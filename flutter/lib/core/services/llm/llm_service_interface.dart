import 'llm_message.dart';
import 'llm_response.dart';

/// Abstract interface for LLM inference.
///
/// **Thread safety:** All implementations must guarantee that [infer] is safe
/// to call concurrently. Heavy local inference runs on a background isolate;
/// cloud calls are isolated per [LlmSession].
abstract class LlmService {
  /// Perform a single-turn inference.
  ///
  /// [prompt] – sanitized prompt string (caller must strip PII before cloud).
  /// [maxTokens] – hard cap on output length.
  ///
  /// Returns the generated text, or throws [LlmException] on failure.
  Future<String> infer(String prompt, {int maxTokens = 512});

  /// Perform multi-turn inference with message history.
  ///
  /// [messages] – conversation history including system prompt.
  /// [maxTokens] – hard cap on output length.
  ///
  /// Returns the generated text, or throws [LlmException] on failure.
  Future<String> inferMessages(List<LlmMessage> messages, {int maxTokens = 512});

  /// Test whether the service is reachable and functional.
  ///
  /// Sends a minimal inference request (e.g. max_tokens=10).
  /// Returns normally if healthy, throws [LlmException] otherwise.
  Future<void> testConnection();

  /// Token usage from the most recent inference call.
  ///
  /// Returns [LlmTokenUsage.empty] if no inference has been performed yet,
  /// or if the underlying provider does not report token counts (e.g. stream).
  LlmTokenUsage get lastTokenUsage;

  /// Release resources held by this service (e.g. HTTP clients).
  void dispose();
}
