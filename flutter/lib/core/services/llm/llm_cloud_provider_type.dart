/// 云端 LLM API 提供商类型。
enum LlmCloudProviderType {
  /// OpenAI-compatible API（含 DeepSeek、Moonshot、DashScope 等）。
  openai,

  /// Anthropic Messages API。
  anthropic,
}

extension LlmCloudProviderTypeExtension on LlmCloudProviderType {
  String get label => switch (this) {
        LlmCloudProviderType.openai => 'OpenAI',
        LlmCloudProviderType.anthropic => 'Anthropic',
      };

  String toJson() => name;

  static LlmCloudProviderType fromJson(String? raw) {
    if (raw == null) return LlmCloudProviderType.openai;
    return switch (raw) {
      'openai' => LlmCloudProviderType.openai,
      'anthropic' => LlmCloudProviderType.anthropic,
      _ => LlmCloudProviderType.openai,
    };
  }
}
