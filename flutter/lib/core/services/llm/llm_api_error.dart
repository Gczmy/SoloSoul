/// 统一的云端 API 错误结构，屏蔽 OpenAI / Anthropic 的格式差异。
class LlmApiError {
  final String message;
  final String type;
  final int statusCode;

  const LlmApiError({
    required this.message,
    required this.type,
    required this.statusCode,
  });
}
