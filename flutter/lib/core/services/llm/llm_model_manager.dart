import 'dart:async';

import 'package:solosoul_flutter/core/services/llm/llm_service.dart';
import 'package:solosoul_flutter/core/services/llm/llm_usage_stats.dart';

import 'llm_model_state.dart';

// =============================================================================
// LLM Model Manager
// =============================================================================

/// Manages the lifecycle of LLM service instances.
///
/// Supports hot-swapping models, health checks, and graceful error recovery.
/// All operations are async-safe.
class LlmModelManager {
  LlmModelManager._();
  static final LlmModelManager _instance = LlmModelManager._();
  static LlmModelManager get instance => _instance;

  LlmService? _service;
  LlmModelState _state = LlmModelState.unloaded;
  String? _errorMessage;
  DateTime? _lastLoadTime;
  DateTime? _lastUsedTime;
  int _usageCount = 0;
  int _totalTokensUsed = 0;

  final _stateController = StreamController<LlmModelState>.broadcast();

  // ---------------------------------------------------------------------------
  // State getters
  // ---------------------------------------------------------------------------

  LlmModelState get state => _state;
  String? get errorMessage => _errorMessage;
  DateTime? get lastLoadTime => _lastLoadTime;
  DateTime? get lastUsedTime => _lastUsedTime;
  int get usageCount => _usageCount;

  bool get isReady => _state == LlmModelState.loaded && _service != null;

  /// Total tokens consumed across all inference calls in this session.
  int get totalTokensUsed => _totalTokensUsed;

  /// Current active service, or null if not loaded.
  LlmService? get service => _service;

  /// 状态变更广播流。UI 层通过监听此流实现零延迟同步。
  Stream<LlmModelState> get stateStream => _stateController.stream;

  // ---------------------------------------------------------------------------
  // Lifecycle
  // ---------------------------------------------------------------------------

  /// Load a cloud LLM service.
  ///
  /// Validates connectivity via [LlmCloudService.testConnection] before
  /// transitioning to [LlmModelState.loaded].
  Future<void> loadCloud({
    required String apiKey,
    String endpoint = 'https://api.openai.com/v1',
    String model = 'gpt-4o-mini',
    LlmCloudProviderType provider = LlmCloudProviderType.openai,
    String anthropicVersion = '2023-06-01',
  }) async {
    await _transitionTo(LlmModelState.loading);
    try {
      final cloud = LlmCloudService(
        apiKey: apiKey,
        endpoint: endpoint,
        model: model,
        provider: provider,
        anthropicVersion: anthropicVersion,
      );
      await cloud.testConnection();
      _service = cloud;
      _errorMessage = null;
      await _transitionTo(LlmModelState.loaded);
    } catch (e) {
      _service = null;
      _errorMessage = e.toString();
      await _transitionTo(LlmModelState.error);
      rethrow;
    }
  }

  /// Load a local Ollama service.
  ///
  /// Optionally validates that the model exists before marking ready.
  Future<void> loadLocal({
    String baseUrl = 'http://localhost:11434',
    String modelName = 'qwen2.5:1.5b',
    bool validateModel = true,
  }) async {
    await _transitionTo(LlmModelState.loading);
    try {
      final local = LlmLocalService(
        baseUrl: baseUrl,
        modelName: modelName,
      );

      if (validateModel) {
        final status = await local.checkStatus();
        if (!status.serviceRunning) {
          throw const LlmException(
            'Ollama 服务未运行，请先启动 Ollama',
            code: LlmErrorCode.modelNotFound,
          );
        }
        if (!status.modelAvailable) {
          throw LlmException(
            '本地模型 $modelName 未安装，请先执行 ollama pull $modelName',
            code: LlmErrorCode.modelNotFound,
          );
        }
      }

      _service = local;
      _errorMessage = null;
      await _transitionTo(LlmModelState.loaded);
    } catch (e) {
      _service = null;
      _errorMessage = e.toString();
      await _transitionTo(LlmModelState.error);
      rethrow;
    }
  }

  /// Unload the current model and release resources.
  Future<void> unload() async {
    _service = null;
    _errorMessage = null;
    await _transitionTo(LlmModelState.unloaded);
  }

  /// Hot-reload: unload current, then load a new cloud model.
  Future<void> reloadCloud({
    required String apiKey,
    String endpoint = 'https://api.openai.com/v1',
    String model = 'gpt-4o-mini',
    LlmCloudProviderType provider = LlmCloudProviderType.openai,
    String anthropicVersion = '2023-06-01',
  }) async {
    await unload();
    await loadCloud(
      apiKey: apiKey,
      endpoint: endpoint,
      model: model,
      provider: provider,
      anthropicVersion: anthropicVersion,
    );
  }

  /// Hot-reload: unload current, then load a new local model.
  Future<void> reloadLocal({
    String baseUrl = 'http://localhost:11434',
    String modelName = 'qwen2.5:1.5b',
    bool validateModel = true,
  }) async {
    await unload();
    await loadLocal(baseUrl: baseUrl, modelName: modelName, validateModel: validateModel);
  }

  // ---------------------------------------------------------------------------
  // Health & Usage
  // ---------------------------------------------------------------------------

  /// Perform a health check on the active service.
  ///
  /// Returns `true` if healthy. On failure, transitions to [LlmModelState.error].
  Future<bool> healthCheck() async {
    if (_service == null || _state != LlmModelState.loaded) return false;
    try {
      await _service!.testConnection();
      return true;
    } on Exception catch (e) {
      _errorMessage = e.toString();
      await _transitionTo(LlmModelState.error);
      return false;
    }
  }

  /// Record a usage event.
  void recordUsage() {
    _usageCount++;
    _lastUsedTime = DateTime.now();
  }

  /// Record token consumption from the most recent inference.
  void recordTokens(int tokens) {
    _totalTokensUsed += tokens;
  }

  /// Restore statistics from persisted data (e.g. after login).
  void restoreStats(LlmUsageStats stats) {
    _usageCount = stats.usageCount;
    _totalTokensUsed = stats.totalTokensUsed;
    if (stats.lastLoadTime != null) {
      _lastLoadTime = stats.lastLoadTime;
    }
    if (stats.lastUsedTime != null) {
      _lastUsedTime = stats.lastUsedTime;
    }
  }

  /// Reset usage statistics.
  void resetStats() {
    _usageCount = 0;
    _totalTokensUsed = 0;
    _lastUsedTime = null;
  }

  // ---------------------------------------------------------------------------
  // Inference helpers
  // ---------------------------------------------------------------------------

  /// Convenience method: infer via the active service.
  ///
  /// Throws [LlmException] if no service is loaded.
  Future<String> infer(String prompt, {int maxTokens = 512}) async {
    if (_service == null || _state != LlmModelState.loaded) {
      throw const LlmException(
        '模型未加载',
        code: LlmErrorCode.modelNotFound,
      );
    }
    final result = await _service!.infer(prompt, maxTokens: maxTokens);
    recordUsage();
    recordTokens(_service!.lastTokenUsage.totalTokens);
    return result;
  }

  /// Convenience method: multi-turn inference via the active service.
  Future<String> inferMessages(List<LlmMessage> messages, {int maxTokens = 512}) async {
    if (_service == null || _state != LlmModelState.loaded) {
      throw const LlmException(
        '模型未加载',
        code: LlmErrorCode.modelNotFound,
      );
    }
    final result = await _service!.inferMessages(messages, maxTokens: maxTokens);
    recordUsage();
    recordTokens(_service!.lastTokenUsage.totalTokens);
    return result;
  }

  // ---------------------------------------------------------------------------
  // Internal
  // ---------------------------------------------------------------------------

  /// 释放资源。应用退出或账户切换时调用。
  void dispose() {
    if (!_stateController.isClosed) {
      _stateController.close();
    }
  }

  Future<void> _transitionTo(LlmModelState newState) async {
    _state = newState;
    if (newState == LlmModelState.loaded) {
      _lastLoadTime = DateTime.now();
    }
    if (!_stateController.isClosed) {
      _stateController.add(newState);
    }
  }
}
