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

  // Current model info for stats tracking
  String _currentModelName = '';
  String _currentProvider = '';

  // Session-level stats (memory only, reset on app restart / account switch)
  int _sessionUsageCount = 0;
  int _sessionPromptTokens = 0;
  int _sessionCompletionTokens = 0;
  final Map<String, LlmModelUsage> _sessionPerModelStats = {};

  // Account-level accumulated stats (loaded from / saved to Vault)
  int _accountUsageCount = 0;
  int _accountPromptTokens = 0;
  int _accountCompletionTokens = 0;
  final Map<String, LlmModelUsage> _perModelStats = {};
  final List<LlmDailyUsage> _dailyStats = [];

  final _stateController = StreamController<LlmModelState>.broadcast();

  // ---------------------------------------------------------------------------
  // State getters
  // ---------------------------------------------------------------------------

  LlmModelState get state => _state;
  String? get errorMessage => _errorMessage;
  DateTime? get lastLoadTime => _lastLoadTime;
  DateTime? get lastUsedTime => _lastUsedTime;

  bool get isReady => _state == LlmModelState.loaded && _service != null;

  /// Current active service, or null if not loaded.
  LlmService? get service => _service;

  /// 状态变更广播流。UI 层通过监听此流实现零延迟同步。
  Stream<LlmModelState> get stateStream => _stateController.stream;

  // ---------------------------------------------------------------------------
  // Current model info
  // ---------------------------------------------------------------------------

  String get currentModelName => _currentModelName;
  String get currentProvider => _currentProvider;

  // ---------------------------------------------------------------------------
  // Session-level stats getters
  // ---------------------------------------------------------------------------

  int get sessionUsageCount => _sessionUsageCount;
  int get sessionPromptTokens => _sessionPromptTokens;
  int get sessionCompletionTokens => _sessionCompletionTokens;
  int get sessionTotalTokens => _sessionPromptTokens + _sessionCompletionTokens;
  List<LlmModelUsage> get sessionPerModelStats => List.unmodifiable(_sessionPerModelStats.values);

  // ---------------------------------------------------------------------------
  // Account-level stats getters
  // ---------------------------------------------------------------------------

  int get accountUsageCount => _accountUsageCount;
  int get accountPromptTokens => _accountPromptTokens;
  int get accountCompletionTokens => _accountCompletionTokens;
  int get accountTotalTokens => _accountPromptTokens + _accountCompletionTokens;

  List<LlmModelUsage> get perModelStats => List.unmodifiable(_perModelStats.values);
  List<LlmDailyUsage> get dailyStats => List.unmodifiable(_dailyStats);

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
      _currentModelName = model;
      _currentProvider = provider.name;
      _errorMessage = null;
      await _transitionTo(LlmModelState.loaded);
      _recordModelLoad('${provider.name}/$model', model, provider.name);
    } on Exception catch (e) {
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
            'Ollama service is not running. Please start Ollama first.',
            code: LlmErrorCode.modelNotFound,
          );
        }
        if (!status.modelAvailable) {
          throw LlmException(
            'Local model $modelName is not installed. Please run ollama pull $modelName first.',
            code: LlmErrorCode.modelNotFound,
          );
        }
      }

      _service = local;
      _currentModelName = modelName;
      _currentProvider = 'ollama';
      _errorMessage = null;
      await _transitionTo(LlmModelState.loaded);
      _recordModelLoad('ollama/$modelName', modelName, 'ollama');
    } on Exception catch (e) {
      _service = null;
      _errorMessage = e.toString();
      await _transitionTo(LlmModelState.error);
      rethrow;
    }
  }

  /// Unload the current model and release resources.
  Future<void> unload() async {
    _service?.dispose();
    _service = null;
    _errorMessage = null;
    _currentModelName = '';
    _currentProvider = '';
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

  /// Record a single inference event with model-specific and daily tracking.
  void recordInference({
    required String modelName,
    required String provider,
    required LlmTokenUsage tokenUsage,
  }) {
    // Session-level
    _sessionUsageCount++;
    _sessionPromptTokens += tokenUsage.promptTokens;
    _sessionCompletionTokens += tokenUsage.completionTokens;
    _lastUsedTime = DateTime.now();

    // Session-level per-model stats
    final sessionKey = '$provider/$modelName';
    final now = DateTime.now();
    final existingSession = _sessionPerModelStats[sessionKey];
    if (existingSession != null) {
      _sessionPerModelStats[sessionKey] = existingSession.copyWith(
        usageCount: existingSession.usageCount + 1,
        promptTokens: existingSession.promptTokens + tokenUsage.promptTokens,
        completionTokens: existingSession.completionTokens + tokenUsage.completionTokens,
        lastUsedTime: now,
      );
    } else {
      _sessionPerModelStats[sessionKey] = LlmModelUsage(
        modelName: modelName,
        provider: provider,
        usageCount: 1,
        promptTokens: tokenUsage.promptTokens,
        completionTokens: tokenUsage.completionTokens,
        lastUsedTime: now,
      );
    }

    // Account-level totals
    _accountUsageCount++;
    _accountPromptTokens += tokenUsage.promptTokens;
    _accountCompletionTokens += tokenUsage.completionTokens;

    // Per-model stats
    final key = '$provider/$modelName';
    final existing = _perModelStats[key];
    if (existing != null) {
      _perModelStats[key] = existing.copyWith(
        usageCount: existing.usageCount + 1,
        promptTokens: existing.promptTokens + tokenUsage.promptTokens,
        completionTokens: existing.completionTokens + tokenUsage.completionTokens,
        lastUsedTime: DateTime.now(),
      );
    } else {
      _perModelStats[key] = LlmModelUsage(
        modelName: modelName,
        provider: provider,
        usageCount: 1,
        promptTokens: tokenUsage.promptTokens,
        completionTokens: tokenUsage.completionTokens,
        lastUsedTime: DateTime.now(),
      );
    }

    // Daily stats
    final today = DateTime.now();
    final todayKey = DateTime(today.year, today.month, today.day);
    final modelKey = '$provider/$modelName';
    final existingDayIndex = _dailyStats.indexWhere(
      (d) => DateTime(d.date.year, d.date.month, d.date.day) == todayKey,
    );
    if (existingDayIndex >= 0) {
      final existingDay = _dailyStats[existingDayIndex];
      final updatedPerModel = Map<String, int>.from(existingDay.perModelTokens);
      updatedPerModel[modelKey] = (updatedPerModel[modelKey] ?? 0) + tokenUsage.totalTokens;
      _dailyStats[existingDayIndex] = existingDay.copyWith(
        totalTokens: existingDay.totalTokens + tokenUsage.totalTokens,
        usageCount: existingDay.usageCount + 1,
        perModelTokens: updatedPerModel,
      );
    } else {
      _dailyStats.add(LlmDailyUsage(
        date: todayKey,
        totalTokens: tokenUsage.totalTokens,
        usageCount: 1,
        perModelTokens: {modelKey: tokenUsage.totalTokens},
      ));
    }
  }

  /// Restore statistics from persisted data (e.g. after login).
  void restoreStats(LlmUsageStats stats) {
    _accountUsageCount = stats.usageCount;
    _accountPromptTokens = stats.totalPromptTokens;
    _accountCompletionTokens = stats.totalCompletionTokens;
    if (stats.lastLoadTime != null) {
      _lastLoadTime = stats.lastLoadTime;
    }
    if (stats.lastUsedTime != null) {
      _lastUsedTime = stats.lastUsedTime;
    }
    _perModelStats.clear();
    for (final m in stats.perModelStats) {
      _perModelStats['${m.provider}/${m.modelName}'] = m;
    }
    _dailyStats.clear();
    _dailyStats.addAll(stats.dailyStats);
  }

  /// Build an [LlmUsageStats] snapshot for persistence.
  LlmUsageStats buildStatsSnapshot() {
    return LlmUsageStats(
      usageCount: _accountUsageCount,
      totalPromptTokens: _accountPromptTokens,
      totalCompletionTokens: _accountCompletionTokens,
      lastLoadTime: _lastLoadTime,
      lastUsedTime: _lastUsedTime,
      perModelStats: _perModelStats.values.toList(),
      dailyStats: List.from(_dailyStats),
      sessionUsageCount: _sessionUsageCount,
      sessionPromptTokens: _sessionPromptTokens,
      sessionCompletionTokens: _sessionCompletionTokens,
    );
  }

  /// Reset usage statistics.
  void resetStats() {
    _sessionUsageCount = 0;
    _sessionPromptTokens = 0;
    _sessionCompletionTokens = 0;
    _sessionPerModelStats.clear();
    _accountUsageCount = 0;
    _accountPromptTokens = 0;
    _accountCompletionTokens = 0;
    _perModelStats.clear();
    _dailyStats.clear();
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
        'Model not loaded',
        code: LlmErrorCode.modelNotFound,
      );
    }
    final result = await _service!.infer(prompt, maxTokens: maxTokens);
    recordInference(
      modelName: _currentModelName,
      provider: _currentProvider,
      tokenUsage: _service!.lastTokenUsage,
    );
    return result;
  }

  /// Convenience method: multi-turn inference via the active service.
  Future<String> inferMessages(List<LlmMessage> messages, {int maxTokens = 512}) async {
    if (_service == null || _state != LlmModelState.loaded) {
      throw const LlmException(
        'Model not loaded',
        code: LlmErrorCode.modelNotFound,
      );
    }
    final result = await _service!.inferMessages(messages, maxTokens: maxTokens);
    recordInference(
      modelName: _currentModelName,
      provider: _currentProvider,
      tokenUsage: _service!.lastTokenUsage,
    );
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

  void _recordModelLoad(String loadKey, String modelName, String provider) {
    final now = DateTime.now();
    final existing = _perModelStats[loadKey];
    _perModelStats[loadKey] = existing != null
        ? existing.copyWith(lastLoadTime: now)
        : LlmModelUsage(modelName: modelName, provider: provider, lastLoadTime: now);
    final sessionExisting = _sessionPerModelStats[loadKey];
    _sessionPerModelStats[loadKey] = sessionExisting != null
        ? sessionExisting.copyWith(lastLoadTime: now)
        : LlmModelUsage(modelName: modelName, provider: provider, lastLoadTime: now);
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
