import 'dart:async';

import 'package:characters/characters.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/llm/llm_config_service.dart';
import 'package:solosoul_flutter/core/services/llm/llm_context_service.dart';
import 'package:solosoul_flutter/core/services/llm/llm_model_manager.dart';
import 'package:solosoul_flutter/core/services/llm/llm_model_state.dart';
import 'package:solosoul_flutter/core/services/llm/llm_service.dart';
import 'package:solosoul_flutter/core/services/llm/llm_usage_stats.dart';
import 'package:solosoul_flutter/core/services/language_service.dart';
import 'package:solosoul_flutter/core/services/user_guide_service.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_notifier.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_types.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
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
  Timer? _typingTimer;
  StreamController<String>? _fallbackController;

  /// 当前 provider 实例绑定的账户 ID，用于 dispose 时正确持久化。
  String? _lastAccountId;

  /// 标记当前内存中的统计数据是否已成功从 Vault 恢复。
  /// 用于防止在数据尚未加载时就将空数据覆盖回 Vault。
  bool _hasRestoredStats = false;

  @override
  Future<LlmModelState> build() async {
    // 监听单例状态流，任何内部状态变化自动同步到 Provider
    await _stateSub?.cancel();
    _stateSub = _manager.stateStream.listen((newState) {
      if (state.hasValue && state.value != newState) {
        // ignore: unawaited_futures
        state = AsyncData(newState);
      }
    });

    ref.onDispose(() {
      _stateSub?.cancel();
      _activeStreamSub?.cancel();
      _typingTimer?.cancel();
      _fallbackController?.close();
      // 生命周期结束时尝试持久化统计（fire-and-forget）
      _persistStatsFor(_lastAccountId);
    });

    // 从加密 Vault 恢复使用统计（必须先完成，再设置账户切换监听，
    // 否则 getStats 期间 auth 变化会触发 _handleAccountSwitch 保存空数据）
    // 关键：Vault 锁定时 _load 返回空配置，getStats 不会抛异常但数据为空。
    // 若此时 restoreStats(0) 并置 _hasRestoredStats=true，Vault 解锁后不会重新加载，
    // 且 onDispose 可能将 0 覆盖回 Vault，导致真实统计永久丢失。
    final authState = ref.read(authNotifierProvider).value;
    final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
    _lastAccountId = accountId;
    if (accountId != null && authState == AuthState.unlocked) {
      try {
        final stats = await LlmConfigService.instance.getStats(accountId);
        _manager.restoreStats(stats);
        _hasRestoredStats = true;
      } on Exception catch (e) {
        SoloLog.w('LlmModelNotifier', '统计恢复失败', e);
        _hasRestoredStats = false;
      }
    } else {
      _hasRestoredStats = false;
    }

    // 账户切换监听 + Vault 解锁后重试加载
    ref.listen(authNotifierProvider, (prev, next) {
      final newId = ref.read(authNotifierProvider.notifier).selectedAccountId;
      final wasUnlocked = prev?.value == AuthState.unlocked;
      final isUnlocked = next.value == AuthState.unlocked;
      if (_lastAccountId != newId) {
        _handleAccountSwitch(_lastAccountId, newId);
      } else {
        final shouldRetryStats = newId != null && isUnlocked && !wasUnlocked && !_hasRestoredStats;
        if (shouldRetryStats) {
          _retryLoadStats(newId);
        }
      }
    });

    return _manager.state;
  }

  /// 处理账户切换：保存旧账户统计，加载新账户统计。
  ///
  /// **关键：** 保存旧账户时先同步快照内存值，再异步写入 Vault，
  /// 防止加载新账户覆盖内存后，保存操作读到的是新账户数据。
  void _handleAccountSwitch(String? oldId, String? newId) {
    // 1. 同步快照旧账户统计（必须在内存被覆盖前捕获）
    // 防御：若内存尚未从 Vault 恢复，不要保存空数据覆盖旧数据
    if (oldId != null && _hasRestoredStats) {
      _persistStatsFor(oldId, _manager.buildStatsSnapshot());
    }

    // 2. 清空内存并加载新账户统计
    _manager.resetStats();
    _hasRestoredStats = false;
    if (newId != null) {
      LlmConfigService.instance.getStats(newId).then((stats) {
        _manager.restoreStats(stats);
        _hasRestoredStats = true;
      }).catchError((Object e) {
        SoloLog.w('LlmModelNotifier', '账户切换统计加载失败', e);
        _hasRestoredStats = false;
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
        'LLM configuration not loaded',
        code: LlmErrorCode.configNotLoaded,
      );
    }

    final config = configAsync.value!;

    if (config.backendType == LlmBackendType.cloud) {
      if (!config.canUseCloud) {
        throw const LlmException(
          'Cloud configuration incomplete: please check API Key and privacy consent',
          code: LlmErrorCode.cloudConfigIncomplete,
        );
      }
      final profile = config.activeCloudProfile;
      if (profile == null) {
        throw const LlmException('No active cloud configuration', code: LlmErrorCode.noActiveProfile);
      }
      final apiKey = await LlmConfigService.instance.getApiKeyByRef(profile.apiKeyRef);
      if (apiKey == null || apiKey.isEmpty) {
        throw const LlmException('API Key is empty', code: LlmErrorCode.apiKeyMissing);
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
      throw const LlmException('Configuration not loaded', code: LlmErrorCode.configNotLoaded);
    }
    if (!config.canUseCloud) {
      throw const LlmException(
        'Cloud configuration incomplete: please check API Key, model config and privacy consent',
        code: LlmErrorCode.cloudConfigIncomplete,
      );
    }
    final profile = config.activeCloudProfile;
    if (profile == null) {
      throw const LlmException('No active cloud configuration', code: LlmErrorCode.noActiveProfile);
    }

    final apiKey = await LlmConfigService.instance.getApiKeyByRef(profile.apiKeyRef);
    if (apiKey == null || apiKey.isEmpty) {
      throw const LlmException('API Key is empty, please reconfigure', code: LlmErrorCode.apiKeyMissing);
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
      return '${profile.providerType.label} connected successfully! Model: ${profile.model}';
    } on LlmException {
      rethrow;
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

  /// 流式聊天推理。
  ///
  /// 当 [includeSystemPrompt] 为 true（默认）时，自动注入包含用户公开档案、
  /// 软件信息和 AI 使用统计的 system prompt。
  ///
  /// [history] 为历史对话记录（user/assistant 交替）。
  /// 返回的 [Stream] 在 Widget dispose 时需由调用方取消订阅，
  /// 或调用 [cancelStream] 主动中断。
  Stream<String> streamChat(
    String prompt, {
    List<LlmMessage>? history,
    bool includeSystemPrompt = true,
    int maxTokens = 512,
  }) {
    final service = _manager.service;

    // Build messages list with optional system prompt
    final messages = <LlmMessage>[];
    if (includeSystemPrompt) {
      final accountId = _lastAccountId;
      if (accountId != null) {
        // Fire-and-forget system prompt injection
        // Since streamChat is synchronous, we use a helper stream that
        // waits for context then starts the actual inference.
        return _streamChatWithContext(
          prompt: prompt,
          history: history,
          accountId: accountId,
          maxTokens: maxTokens,
        );
      }
    }

    // No system prompt: fall through to original behavior
    messages.addAll(history ?? []);
    messages.add(LlmMessage(role: 'user', content: prompt));

    if (service is! LlmLocalService) {
      // 云端服务 fallback：先完整推理再逐字 emit，模拟流式效果
      _fallbackController?.close();
      _typingTimer?.cancel();
      final controller = StreamController<String>();
      _fallbackController = controller;
      _activeStreamSub?.cancel();
      _manager.inferMessages(messages, maxTokens: maxTokens).then((result) {
        if (controller.isClosed) return;
        // 模拟打字机：每 8ms 发送一个 grapheme cluster（避免切开 surrogate pair）
        final chars = result.characters.toList();
        var index = 0;
        Timer.periodic(const Duration(milliseconds: 8), (timer) {
          _typingTimer = timer;
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
    return service.streamChatMessages(messages, maxTokens: maxTokens);
  }

  /// Helper that asynchronously builds context then starts streaming.
  Stream<String> _streamChatWithContext({
    required String prompt,
    required List<LlmMessage>? history,
    required String accountId,
    required int maxTokens,
  }) async* {
    try {
      final context = await LlmContextService.instance.buildContext(
        accountId: accountId,
        modelManager: _manager,
      );
      SoloLog.d('LlmModelNotifier',
          'System prompt injected, cached=${context.wasCached}, estTokens=${context.estimatedTokens}');

      // --- 按需检索功能指南（不作为强制提示词） ---
      final language = await LanguageService.instance.getLanguage();
      final guides = await UserGuideService.instance.findRelevantGuides(prompt, language);
      final docPrompt = _buildDocPrompt(guides);

      final messages = <LlmMessage>[
        LlmMessage(role: 'system', content: context.systemPrompt),
        if (docPrompt != null) LlmMessage(role: 'system', content: docPrompt),
        ...?history,
        LlmMessage(role: 'user', content: prompt),
      ];

      final service = _manager.service;
      if (service is! LlmLocalService) {
        // Cloud fallback with typing-machine effect
        final result = await _manager.inferMessages(messages, maxTokens: maxTokens);
        final chars = result.characters.toList();
        for (var i = 0; i < chars.length; i++) {
          yield chars[i];
          await Future.delayed(const Duration(milliseconds: 8));
        }
      } else {
        // Local Ollama native streaming
        await for (final chunk in service.streamChatMessages(messages, maxTokens: maxTokens)) {
          yield chunk;
        }
      }
    } on Exception catch (e) {
      SoloLog.w('LlmModelNotifier', 'Failed to build context, falling back to plain chat', e);
      // Fallback: chat without system prompt
      final messages = <LlmMessage>[...?history, LlmMessage(role: 'user', content: prompt)];
      final service = _manager.service;
      if (service is! LlmLocalService) {
        final result = await _manager.inferMessages(messages, maxTokens: maxTokens);
        yield result;
      } else {
        await for (final chunk in service.streamChatMessages(messages, maxTokens: maxTokens)) {
          yield chunk;
        }
      }
    }
  }

  /// 将匹配到的指南内容组装为注入用的 system message。
  /// 返回 null 表示无匹配或内容为空。
  String? _buildDocPrompt(List<GuideContent> guides) {
    if (guides.isEmpty) return null;
    final guide = guides.first;
    final buffer = StringBuffer();
    buffer.writeln('---');
    buffer.writeln('以下是与用户问题相关的功能使用文档，请参考这些信息回答用户问题。');
    buffer.writeln();
    buffer.writeln('【文档：${guide.title}】');
    buffer.writeln(guide.content);
    buffer.writeln('【文档结束】');
    buffer.writeln('---');
    final prompt = buffer.toString();
    SoloLog.d('LlmModelNotifier',
        'Injected guide doc: id=${guide.id}, chars=${prompt.length}');
    return prompt;
  }

  /// 取消正在进行的流式推理。
  void cancelStream() {
    _activeStreamSub?.cancel();
    _activeStreamSub = null;
    _typingTimer?.cancel();
    _typingTimer = null;
    _fallbackController?.close();
    _fallbackController = null;
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

  /// Session 累计推理调用次数（本应用生命周期）。
  int get sessionUsageCount => _manager.sessionUsageCount;

  /// Session 累计 Prompt Token（本应用生命周期）。
  int get sessionPromptTokens => _manager.sessionPromptTokens;

  /// Session 累计 Completion Token（本应用生命周期）。
  int get sessionCompletionTokens => _manager.sessionCompletionTokens;

  /// Session 累计总 Token（本应用生命周期）。
  int get sessionTotalTokens => _manager.sessionTotalTokens;

  /// Account 累计推理调用次数（跨会话持久化）。
  int get accountUsageCount => _manager.accountUsageCount;

  /// Account 累计总 Token（跨会话持久化）。
  int get accountTotalTokens => _manager.accountTotalTokens;

  /// Account 累计 Prompt Token（跨会话持久化）。
  int get accountPromptTokens => _manager.accountPromptTokens;

  /// Account 累计 Completion Token（跨会话持久化）。
  int get accountCompletionTokens => _manager.accountCompletionTokens;

  /// 最后加载时间。
  DateTime? get lastLoadTime => _manager.lastLoadTime;

  /// 最后使用时间。
  DateTime? get lastUsedTime => _manager.lastUsedTime;

  /// 各模型使用统计（账户累计）。
  List<LlmModelUsage> get perModelStats => _manager.perModelStats;

  /// 各模型使用统计（本次会话）。
  List<LlmModelUsage> get sessionPerModelStats => _manager.sessionPerModelStats;

  /// 每日使用统计。
  List<LlmDailyUsage> get dailyStats => _manager.dailyStats;

  /// Vault 解锁后重新尝试加载统计。
  Future<void> _retryLoadStats(String accountId) async {
    try {
      final stats = await LlmConfigService.instance.getStats(accountId);
      _manager.restoreStats(stats);
      _hasRestoredStats = true;
      SoloLog.d('LlmModelNotifier', 'Vault 解锁后统计恢复成功 '
          'usage=${stats.usageCount} tokens=${stats.totalTokensUsed}');
    } on Exception catch (e) {
      SoloLog.w('LlmModelNotifier', 'Vault 解锁后统计恢复失败', e);
    }
  }

  /// 将统计异步持久化到指定账户的 Vault。
  ///
  /// [stats] 为 null 时从当前内存读取（用于常规保存）。
  /// 账户切换时应传入同步快照后的 [stats]，避免竞态。
  Future<void> _persistStatsFor(String? accountId, [LlmUsageStats? stats]) async {
    if (accountId == null) return;
    // 防御：若内存数据尚未从 Vault 恢复，禁止保存空数据覆盖旧数据
    if (stats == null && !_hasRestoredStats) {
      SoloLog.d('LlmModelNotifier', '跳过持久化：内存数据尚未恢复');
      return;
    }
    try {
      final s = stats ?? _manager.buildStatsSnapshot();
      SoloLog.d('LlmModelNotifier', '持久化统计 account=$accountId '
          'usage=${s.usageCount} tokens=${s.totalTokensUsed} '
          'models=${s.perModelStats.length} days=${s.dailyStats.length}');
      await LlmConfigService.instance.setStats(accountId, s);
    } on Exception catch (e) {
      SoloLog.w('LlmModelNotifier', '统计持久化失败', e);
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
