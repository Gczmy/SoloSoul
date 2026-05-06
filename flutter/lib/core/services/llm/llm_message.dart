/// A single message in a multi-turn conversation.
///
/// Follows the OpenAI chat completions message format.
class LlmMessage {
  final String role; // 'system' | 'user' | 'assistant'
  final String content;

  const LlmMessage({required this.role, required this.content});

  Map<String, String> toJson() => {'role': role, 'content': content};

  factory LlmMessage.fromJson(Map<String, dynamic> json) => LlmMessage(
        role: (json['role'] as String?) ?? 'user',
        content: (json['content'] as String?) ?? '',
      );

  @override
  String toString() => 'LlmMessage($role: ${content.substring(0, content.length > 30 ? 30 : content.length)}...)';
}
