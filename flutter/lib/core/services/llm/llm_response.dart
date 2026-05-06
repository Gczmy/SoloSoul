/// Standardized response from any LLM provider (cloud or local).
class LlmInferenceResponse {
  final String content;
  final String model;
  final String provider;
  final String? finishReason;
  final LlmTokenUsage usage;

  const LlmInferenceResponse({
    required this.content,
    required this.model,
    required this.provider,
    this.finishReason,
    this.usage = const LlmTokenUsage(),
  });

  static const empty = LlmInferenceResponse(
    content: '',
    model: '',
    provider: '',
  );
}

/// Token usage statistics from an inference call.
class LlmTokenUsage {
  final int promptTokens;
  final int completionTokens;
  final int totalTokens;

  const LlmTokenUsage({
    this.promptTokens = 0,
    this.completionTokens = 0,
    this.totalTokens = 0,
  });

  factory LlmTokenUsage.fromJson(Map<String, dynamic> json) => LlmTokenUsage(
        promptTokens: json['prompt_tokens'] as int? ?? 0,
        completionTokens: json['completion_tokens'] as int? ?? 0,
        totalTokens: json['total_tokens'] as int? ?? 0,
      );
}
