/// Supported LLM inference backends.
enum LlmBackendType {
  /// Local model inference via HTTP API (Ollama) or Rust native layer.
  local,

  /// Cloud API (OpenAI-compatible, Anthropic, self-hosted endpoint).
  cloud,
}
