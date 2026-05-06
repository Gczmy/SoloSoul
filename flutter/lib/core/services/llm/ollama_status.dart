/// Ollama service health status.
class OllamaStatus {
  final bool serviceRunning;
  final bool modelAvailable;
  final List<String> installedModels;

  const OllamaStatus({
    required this.serviceRunning,
    required this.modelAvailable,
    this.installedModels = const [],
  });

  bool get isReady => serviceRunning && modelAvailable;
}
