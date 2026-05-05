import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/llm/llm_config_service.dart';
import 'package:solosoul_flutter/core/services/llm/llm_model_manager.dart';
import 'package:solosoul_flutter/core/services/llm/llm_model_state.dart';
import 'package:solosoul_flutter/core/services/llm/llm_service.dart';
import 'package:solosoul_flutter/core/services/llm/llm_usage_stats.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_notifier.dart';
import 'package:solosoul_flutter/presentation/providers/llm/llm_config_provider.dart';

// =============================================================================
// LLM Model Provider
// =============================================================================

/// Riverpod 封装 for [LlmModelManager]。
///
/// UI 层通过 `ref.watch(llmModelProvider)` 实时订阅模型生命周期状态，
/// 通过 `ref.read(llmModelProvider.notifier)` 调用加载/推理/流式等方法。
///
/// **状态同步机制：**
/// `build()` 中监听 `LlmModelManager.stateStream`，单例内部状态变化时
/// 自动刷新 Provider，实现零延迟 UI 同步。
class LlmModelNotifier extends AsyncNotifier<LlmModelState> {
  final LlmModelManager _manager = LlmModelManager.instance;
  StreamSubscription<LlmModelState>? _stateSub;
  StreamSubscription<String>? _activeStreamSub;

  /// 当前 provider 实例绑定的账户 ID，用于 dispose 时正确持久化。
  String? _lastAccountId;

  @override
  Future<LlmModelState> build() async {
    // 监听单例状态流，任何内部状态变化自动同步到 Provider
    _stateSub?.cancel();
    _stateSub = _manager.stateStream.listen((newState) {
      if (state.hasValue && state.value != newState) {
        // ignore: unawaited_futures
        state = AsyncData(newState);
      }
    });

    ref.onDispose(() {
      _stateSub?.cancel();
      _activeStreamSub?.cancel();
      // 生命周期结束时尝试持久化统计（fire-and-forget）
      _persistStatsFor(_lastAccountId);
    });

    // 账户切换监听：先保存旧账户，再加载新账户
    ref.listen(authNotifierProvider, (_, __) {
      final newId = ref.read(authNotifierProvider.notifier).selectedAccountId;
      if (_lastAccountId != newId) {
        _handleAccountSwitch(_lastAccountId, newId);
      }
    });

    // 从加密 Vault 恢复使用统计
    final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
    _lastAccountId = accountId;
    if (accountId != null) {
      try {
        final stats = await LlmConfigService.instance.getStats(accountId);
        _manager.restoreStats(stats);
      } on Exception catch (e) {
        // ignore: avoid_print
        print('[LlmModelNotifier] 统计恢复失败: $e');
      }
    }

    return _manager.state;
  }

  /// 处理账户切换：保存旧账户统计，加载新账户统计。
  ///
  /// **关键：** 保存旧账户时先同步快照内存值，再异步写入 Vault，
  /// 防止加载新账户覆盖内存后，保存操作读到的是新账户数据。
  void _handleAccountSwitch(String? oldId, String? newId) {
    // 1. 同步快照旧账户统计（必须在内存被覆盖前捕获）
    if (oldId != null) {
      final oldStats = LlmUsageStats(
        usageCount: _manager.usageCount,
        totalTokensUsed: _manager.totalTokensUsed,
        lastLoadTime: _manager.lastLoadTime,
        lastUsedTime: _manager.lastUsedTime,
      );
      _persistStatsFor(oldId, oldStats);
    }

    // 2. 清空内存并加载新账户统计
    _manager.resetStats();
    if (newId != null) {
      LlmConfigService.instance.getStats(newId).then((stats) {
        _manager.restoreStats(stats);
      }).catchError((Object e) {
        // ignore: avoid_print
        print('[LlmModelNotifier] 账户切换统计加载失败: $e');
      });
    }
    _lastAccountId = newId;
  }

  // ---------------------------------------------------------------------------
  // Lifecycle
  // ---------------------------------------------------------------------------

  /// 根据当前 [llmConfigProvider] 的配置自动加载对应模型。
  ///
  /// - cloud 后端：需要 apiKey + consent，否则抛出 [LlmException]
  /// - local 后端：自动检测 Ollama 服务和模型
  Future<void> loadFromConfig() async {
    final configAsync = ref.read(llmConfigProvider);
    if (!configAsync.hasValue) {
      throw const LlmException(
        'LLM 配置尚未加载',
        code: LlmErrorCode.unknown,
      );
    }

    final config = configAsync.value!;

    if (config.backendType == LlmBackendType.cloud) {
      if (!config.canUseCloud) {
        throw const LlmException(
          '云端配置不完整：请检查 API Key 和隐私同意',
          code: LlmErrorCode.unauthorized,
        );
      }
      final profile = config.activeCloudProfile;
      if (profile == null) {
        throw const LlmException('没有激活的云端配置', code: LlmErrorCode.unknown);
      }
      final apiKey = await LlmConfigService.instance.getApiKeyByRef(profile.apiKeyRef);
      if (apiKey == null || apiKey.isEmpty) {
        throw const LlmException('API Key 为空', code: LlmErrorCode.unauthorized);
      }
      await _manager.loadCloud(
        apiKey: apiKey,
        endpoint: profile.endpoint,
        model: profile.model,
        provider: profile.providerType,
        anthropicVersion: profile.anthropicVersion ?? '2023-06-01',
      );
    } else {
      await _manager.loadLocal(
        modelName: config.localModelPath ?? 'qwen2.5:1.5b',
      );
    }
  }

  /// 卸载当前模型并释放资源。
  Future<void> unload() async {
    await _activeStreamSub?.cancel();
    _activeStreamSub = null;
    await _manager.unload();
  }

  /// 健康检查：测试当前服务连通性。
  ///
  /// 返回 `true` 表示健康；失败时自动将状态置为 [LlmModelState.error]。
  Future<bool> healthCheck() async {
    return _manager.healthCheck();
  }

  /// 测试当前激活的云端配置连接。
  ///
  /// 从 [llmConfigProvider] 读取 active profile，通过 [LlmConfigService]
  /// 取出 apiKey 明文，创建临时 [LlmCloudService] 测试连接，
  /// 用后立即丢弃 apiKey 局部变量。
  Future<String> testActiveCloudConnection() async {
    final config = ref.read(llmConfigProvider).value;
    if (config == null) {
      throw const LlmException('配置未加载', code: LlmErrorCode.unknown);
    }
    if (!config.canUseCloud) {
      throw const LlmException(
        '云端配置不完整：请检查 API Key、模型配置和隐私同意',
        code: LlmErrorCode.unauthorized,
      );
    }
    final profile = config.activeCloudProfile;
    if (profile == null) {
      throw const LlmException('没有激活的云端配置', code: LlmErrorCode.unknown);
    }

    final apiKey = await LlmConfigService.instance.getApiKeyByRef(profile.apiKeyRef);
    if (apiKey == null || apiKey.isEmpty) {
      throw const LlmException('API Key 为空，请重新配置', code: LlmErrorCode.unauthorized);
    }

    final service = LlmCloudService(
      apiKey: apiKey,
      endpoint: profile.endpoint,
      model: profile.model,
      provider: profile.providerType,
      anthropicVersion: profile.anthropicVersion ?? '2023-06-01',
    );

    try {
      await service.testConnection();
      return '${profile.providerType.label} 连接成功！模型: ${profile.model}';
    } on LlmException catch (e) {
      throw LlmException('${profile.providerType.label} 连接失败: ${e.message}', code: e.code);
    }
  }

  // ---------------------------------------------------------------------------
  // Inference
  // ---------------------------------------------------------------------------

  /// 单轮推理。
  ///
  /// 模型必须处于 [LlmModelState.loaded] 状态，否则抛出 [LlmException]。
  Future<String> infer(String prompt, {int maxTokens = 512}) async {
    return _manager.infer(prompt, maxTokens: maxTokens);
  }

  /// 多轮消息推理。
  Future<String> inferMessages(
    List<LlmMessage> messages, {
    int maxTokens = 512,
  }) async {
    return _manager.inferMessages(messages, maxTokens: maxTokens);
  }

  // ---------------------------------------------------------------------------
  // Streaming
  // ---------------------------------------------------------------------------

  /// 流式聊天推理（本地 Ollama 专属）。
  ///
  /// 返回的 [Stream] 在 Widget dispose 时需由调用方取消订阅，
  /// 或调用 [cancelStream] 主动中断。
  Stream<String> streamChat(
    String prompt, {
    List<LlmMessage>? history,
    int maxTokens = 512,
  }) {
    final service = _manager.service;
    if (service is! LlmLocalService) {
      // 云端服务 fallback：先完整推理再逐字 emit，模拟流式效果
      final controller = StreamController<String>();
      _activeStreamSub?.cancel();
      _manager.infer(prompt, maxTokens: maxTokens).then((result) {
        if (controller.isClosed) return;
        // 模拟打字机：每 30ms 发送一个字符
        final chars = result.split('');
        var index = 0;
        Timer.periodic(const Duration(milliseconds: 30), (timer) {
          if (controller.isClosed) {
            timer.cancel();
            return;
          }
          if (index < chars.length) {
            controller.add(chars[index]);
            index++;
          } else {
            timer.cancel();
            controller.close();
          }
        });
      }).catchError((Object err) {
        if (!controller.isClosed) {
          controller.addError(err);
          controller.close();
        }
      });
      return controller.stream;
    }

    // 本地 Ollama 原生流式
    final stream = service.streamChat(
      prompt,
      history: history,
      maxTokens: maxTokens,
    );
    return stream;
  }

  /// 取消正在进行的流式推理。
  void cancelStream() {
    _activeStreamSub?.cancel();
    _activeStreamSub = null;
  }

  // ---------------------------------------------------------------------------
  // Getters
  // ---------------------------------------------------------------------------

  /// 当前活跃的服务实例（可能为 null）。
  LlmService? get service => _manager.service;

  /// 最近的错误信息。
  String? get errorMessage => _manager.errorMessage;

  /// 模型是否就绪。
  bool get isReady => _manager.isReady;

  // ---------------------------------------------------------------------------
  // Usage Statistics
  // ---------------------------------------------------------------------------

  /// 累计推理调用次数（当前会话）。
  int get usageCount => _manager.usageCount;

  /// 累计消耗 Token 数（当前会话）。
  int get totalTokensUsed => _manager.totalTokensUsed;

  /// 最后加载时间。
  DateTime? get lastLoadTime => _manager.lastLoadTime;

  /// 最后使用时间。
  DateTime? get lastUsedTime => _manager.lastUsedTime;

  /// 将统计异步持久化到指定账户的 Vault。
  ///
  /// [stats] 为 null 时从当前内存读取（用于常规保存）。
  /// 账户切换时应传入同步快照后的 [stats]，避免竞态。
  Future<void> _persistStatsFor(String? accountId, [LlmUsageStats? stats]) async {
    if (accountId == null) return;
    try {
      final s = stats ?? LlmUsageStats(
        usageCount: _manager.usageCount,
        totalTokensUsed: _manager.totalTokensUsed,
        lastLoadTime: _manager.lastLoadTime,
        lastUsedTime: _manager.lastUsedTime,
      );
      await LlmConfigService.instance.setStats(accountId, s);
    } on Exception catch (e) {
      // ignore: avoid_print
      print('[LlmModelNotifier] 统计持久化失败: $e');
    }
  }

  /// 重置使用统计并持久化。
  Future<void> resetStats() async {
    _manager.resetStats();
    await _persistStatsFor(_lastAccountId);
  }
}

// =============================================================================
// Provider
// =============================================================================

final llmModelProvider =
    AsyncNotifierProvider<LlmModelNotifier, LlmModelState>(
  () => LlmModelNotifier(),
);
