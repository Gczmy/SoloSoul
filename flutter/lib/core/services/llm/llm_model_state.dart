// =============================================================================
// LLM Model State
// =============================================================================

/// Lifecycle state machine for an LLM model instance.
///
/// State transitions:
/// ```
/// UNLOADED --load()--> LOADING --success--> LOADED
///   ^                                    |
///   |                                    | --health check fail--> ERROR
///   |                                    |
///   +--------unload()-------------------+
/// ```
enum LlmModelState {
  /// Model instance created but not yet initialized.
  unloaded,

  /// Model is being loaded (e.g. HTTP session creation, model warmup).
  loading,

  /// Model is ready for inference.
  loaded,

  /// Model encountered an error and is not usable.
  /// Use [LlmModelManager.getErrorMessage] for details.
  error,
}

extension LlmModelStateExtension on LlmModelState {
  bool get isUnloaded => this == LlmModelState.unloaded;
  bool get isLoading => this == LlmModelState.loading;
  bool get isLoaded => this == LlmModelState.loaded;
  bool get isError => this == LlmModelState.error;
  bool get isReady => isLoaded;

  String get label {
    switch (this) {
      case LlmModelState.unloaded:
        return 'Unloaded';
      case LlmModelState.loading:
        return 'Loading';
      case LlmModelState.loaded:
        return 'Ready';
      case LlmModelState.error:
        return 'Error';
    }
  }
}
