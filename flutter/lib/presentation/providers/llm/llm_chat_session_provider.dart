import 'dart:async';
import 'dart:math';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:solosoul_flutter/core/models/chat_session.dart';
import 'package:solosoul_flutter/core/services/llm/chat_history_service.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_notifier.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_types.dart' show AuthState;
import 'package:solosoul_flutter/presentation/providers/llm/chat_session_list_provider.dart';
import 'package:solosoul_flutter/presentation/providers/llm/selected_chat_session_provider.dart';

// =============================================================================
// LLM Chat Session Provider
// =============================================================================

/// 单条聊天消息（纯文本，可序列化到 Provider 状态）。
///
/// **注意：** system prompt 消息不进入此模型，也不被持久化。
/// 每次发送前由 [LlmModelProvider] 动态拼接 system 消息。
class LlmChatMessage {
  final String id;
  final String text;
  final bool isUser;

  /// 是否仍在流式输出中。Widget dispose 后重新构建时，
  /// 任何遗留的 `isStreaming=true` 消息会被自动标记为完成。
  final bool isStreaming;

  /// 消息创建时间戳（毫秒级 Unix epoch）。
  /// 旧数据可能缺少此字段，默认值为 0。
  final int createdAt;

  const LlmChatMessage({
    required this.id,
    required this.text,
    required this.isUser,
    this.isStreaming = false,
    this.createdAt = 0,
  });

  LlmChatMessage copyWith({
    String? id,
    String? text,
    bool? isUser,
    bool? isStreaming,
    int? createdAt,
  }) {
    return LlmChatMessage(
      id: id ?? this.id,
      text: text ?? this.text,
      isUser: isUser ?? this.isUser,
      isStreaming: isStreaming ?? this.isStreaming,
      createdAt: createdAt ?? this.createdAt,
    );
  }

  /// 转换为可序列化的数据对象。
  ChatMessageData toData() => ChatMessageData(
        id: id,
        text: text,
        isUser: isUser,
        createdAt: createdAt,
      );

  /// 从序列化数据对象恢复。
  factory LlmChatMessage.fromData(ChatMessageData data) => LlmChatMessage(
        id: data.id,
        text: data.text,
        isUser: data.isUser,
        createdAt: data.createdAt,
      );
}

/// 管理当前选中 LLM 对话会话的消息列表。
///
/// **多会话支持：** 此 Notifier 根据 [selectedChatSessionIdProvider]
/// 的值加载对应会话的消息。切换会话时自动保存旧会话、加载新会话。
///
/// **后台接收支持：** Stream 订阅由 Notifier 持有（生命周期与 Provider 一致），
/// 切换页面后 widget 被 dispose 但 Provider 继续存活，stream 数据持续累积。
///
/// **持久化支持：** 消息变更自动 debounce 保存到加密 Vault；
/// 账号切换、Vault 锁定、或会话切换时自动保存/清空。
class LlmChatSessionNotifier extends Notifier<List<LlmChatMessage>> {
  StreamSubscription<String>? _streamSub;
  Timer? _saveTimer;
  String? _lastLoadedAccountId;
  String? _lastLoadedSessionId;

  /// 当 sendMessage 自动创建会话时，用于跳过 listener 中不必要的加载，
  /// 避免异步加载覆盖 sendMessage 刚添加的内存消息。
  bool _skipNextLoad = false;
  bool _isLoading = false;

  static const _saveDebounce = Duration(seconds: 2);

  @override
  List<LlmChatMessage> build() {
    // Schedule initial load AFTER build() returns.
    // Never call async code directly in build() — if the Future completes
    // synchronously (e.g. vault locked), any state read inside it will crash
    // with "uninitialized provider" in Riverpod 3.x.
    Future.microtask(() {
      final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
      final selectedSessionId = ref.read(selectedChatSessionIdProvider);
      if (selectedSessionId != null &&
          !isNewChatSessionId(selectedSessionId) &&
          accountId != null &&
          _lastLoadedSessionId == null) {
        _loadSessionMessagesAsync(accountId, selectedSessionId);
      }
    });

    // Listen for session switches. Listener callbacks always run after
    // build() completes, so state is safe to access here.
    ref.listen(selectedChatSessionIdProvider, (previous, next) {
      final currentAccountId =
          ref.read(authNotifierProvider.notifier).selectedAccountId;

      // Save previous session
      if (previous != null &&
          !isNewChatSessionId(previous) &&
          _lastLoadedAccountId != null &&
          _lastLoadedSessionId == previous) {
        _saveSessionMessages(_lastLoadedAccountId!, previous);
      }

      // Enter temporary new-chat state
      if (next != null && isNewChatSessionId(next)) {
        _lastLoadedSessionId = null;
        _lastLoadedAccountId = null;
        if (state.isNotEmpty) state = [];
        return;
      }

      // Load new session (skip if sendMessage already set _lastLoadedSessionId)
      if (next != null &&
          !isNewChatSessionId(next) &&
          currentAccountId != null &&
          _lastLoadedSessionId != next) {
        _loadSessionMessagesAsync(currentAccountId, next);
      }

      // Clear when deselected
      if (next == null && _lastLoadedSessionId != null) {
        _lastLoadedSessionId = null;
        _lastLoadedAccountId = null;
        if (state.isNotEmpty) state = [];
      }
    });

    // Listen to auth state
    ref.listen(authNotifierProvider, (previous, next) {
      final nextState = next.value;
      final currentAccountId =
          ref.read(authNotifierProvider.notifier).selectedAccountId;
      final currentSessionId = ref.read(selectedChatSessionIdProvider);

      if (nextState == AuthState.unlocked) {
        if (currentAccountId != null &&
            currentSessionId != null &&
            !isNewChatSessionId(currentSessionId) &&
            _lastLoadedSessionId == null) {
          _loadSessionMessagesAsync(currentAccountId, currentSessionId);
        }
      } else if (nextState == AuthState.locked) {
        // Save current session before clearing
        if (_lastLoadedAccountId != null && _lastLoadedSessionId != null) {
          _saveSessionMessages(_lastLoadedAccountId!, _lastLoadedSessionId!);
        }
        _saveTimer?.cancel();
        _streamSub?.cancel();
        _streamSub = null;
        if (state.isNotEmpty) {
          state = [];
        }
        _lastLoadedAccountId = null;
        _lastLoadedSessionId = null;
      }
    });

    ref.onDispose(() {
      _saveTimer?.cancel();
      _streamSub?.cancel();
    });

    return [];
  }

  bool get hasStreamingMessage => state.any((m) => m.isStreaming);

  // ---------------------------------------------------------------------------
  // Persistence
  // ---------------------------------------------------------------------------

  /// 异步加载指定会话的消息。
  Future<void> _loadSessionMessagesAsync(
    String accountId,
    String sessionId,
  ) async {
    if (accountId == _lastLoadedAccountId && sessionId == _lastLoadedSessionId) {
      return;
    }
    if (_isLoading) return;

    // Guard: sendMessage created a session and will add messages itself.
    // Skip loading to avoid overwriting in-memory messages.
    if (_skipNextLoad) {
      _skipNextLoad = false;
      _lastLoadedAccountId = accountId;
      _lastLoadedSessionId = sessionId;
      return;
    }

    _isLoading = true;
    try {
      final history = await ChatHistoryService.instance.loadSessionMessages(
        accountId,
        sessionId,
      );

      _lastLoadedAccountId = accountId;
      _lastLoadedSessionId = sessionId;

      if (history.isNotEmpty) {
        state = history.map((data) => LlmChatMessage.fromData(data)).toList();
        SoloLog.d('LlmChatSession', 'loaded ${history.length} messages for session $sessionId');
      } else {
        if (state.isNotEmpty) {
          state = [];
        }
      }
    } finally {
      _isLoading = false;
    }
  }

  /// 保存当前消息列表到指定会话。
  Future<void> _saveSessionMessages(String accountId, String sessionId) async {
    final persistable = state
        .where((m) => !m.isStreaming)
        .map((m) => m.toData())
        .toList();

    await ChatHistoryService.instance.saveSessionMessages(
      accountId,
      sessionId,
      persistable,
    );
  }

  /// 立即保存当前会话消息。
  Future<void> _saveCurrentSession() async {
    final accountId = _lastLoadedAccountId;
    final sessionId = _lastLoadedSessionId;
    if (accountId == null || sessionId == null) return;
    await _saveSessionMessages(accountId, sessionId);
  }

  /// Debounced 保存：避免每次 state 变更都触发 Vault IO。
  void _debouncedSave() {
    _saveTimer?.cancel();
    _saveTimer = Timer(_saveDebounce, () {
      _saveCurrentSession();
    });
  }

  // ---------------------------------------------------------------------------
  // Send Message
  // ---------------------------------------------------------------------------

  /// 发送用户消息并开始消费 AI 流式响应。
  ///
  /// Stream 订阅由 Notifier 本地持有，widget dispose 不影响后台接收。
  Future<void> sendMessage(String text, Stream<String> stream, {required AppLocalizations l10n}) async {
    if (hasStreamingMessage) return;

    var sessionId = _lastLoadedSessionId;
    var accountId = _lastLoadedAccountId;

    // Ensure we have an active session before sending.
    // If the user entered AI chat before session list finished loading,
    // or no session was ever selected, create one on-the-fly.
    if (sessionId == null) {
      accountId ??= ref.read(authNotifierProvider.notifier).selectedAccountId;
      if (accountId != null) {
        // Prevent listener from loading empty messages and overwriting
        // the message we're about to add.
        _skipNextLoad = true;
        ref.read(chatSessionListProvider.notifier).createSession();
        sessionId = ref.read(selectedChatSessionIdProvider);
        _lastLoadedSessionId = sessionId;
        _lastLoadedAccountId = accountId;
      }
    }

    // 1. 添加用户消息
    final now = DateTime.now().millisecondsSinceEpoch;
    final userId = 'user_${now}_${Random.secure().nextInt(999999)}';
    state = [...state, LlmChatMessage(id: userId, text: text, isUser: true, createdAt: now)];
    _debouncedSave();

    // Auto-title from first user message if still default
    if (state.length == 1 && sessionId != null) {
      ref.read(chatSessionListProvider.notifier).autoTitleFromMessage(sessionId, text);
    }

    // 2. 添加空 AI 消息（等待流式填充）
    final aiId = 'ai_${now}_${Random.secure().nextInt(999999)}';
    state = [
      ...state,
      LlmChatMessage(id: aiId, text: '', isUser: false, isStreaming: true, createdAt: now),
    ];

    // 3. 取消旧订阅（理论上不会发生，因为 hasStreamingMessage 已拦截）
    await _streamSub?.cancel();

    // 4. 订阅 stream：在后台逐字累积，widget 重建时直接读取
    // 使用 100ms debounce 批量刷新 state，避免每个 chunk 都触发 rebuild
    final buffer = StringBuffer();
    Timer? debounceTimer;

    void flushState({bool finish = false}) {
      state = state.map((m) {
        if (m.id != aiId) return m;
        if (finish && buffer.isEmpty) {
          return m.copyWith(
            text: l10n.llmChatEmptyResponse,
            isStreaming: false,
          );
        }
        return m.copyWith(
          text: buffer.toString(),
          isStreaming: !finish,
        );
      }).toList();
    }

    _streamSub = stream.listen(
      (chunk) {
        buffer.write(chunk);
        debounceTimer?.cancel();
        debounceTimer = Timer(const Duration(milliseconds: 100), () => flushState());
      },
      onDone: () {
        debounceTimer?.cancel();
        flushState(finish: true);
        _streamSub = null;
        // 流式输出完成，保存最终消息
        _debouncedSave();
        // 更新会话统计
        _updateSessionStats();
      },
      onError: (Object err) {
        debounceTimer?.cancel();
        state = state.map((m) {
          if (m.id != aiId) return m;
          return m.copyWith(
            text: l10n.llmChatInferenceFailed(err.toString()),
            isStreaming: false,
          );
        }).toList();
        _streamSub = null;
        // 错误也保存，保留上下文
        _debouncedSave();
        _updateSessionStats();
      },
    );
  }

  /// 更新当前会话的 messageCount 统计。
  void _updateSessionStats() {
    final sessionId = _lastLoadedSessionId;
    if (sessionId == null) return;
    final messageCount = state.where((m) => !m.isStreaming).length;
    ref.read(chatSessionListProvider.notifier).updateSessionStats(
      sessionId,
      messageCount,
    );
  }

  /// 恢复会话：仅在 stream 已结束但消息仍遗留 isStreaming=true 时清理状态。
  /// 若 stream 仍在活跃运行中（_streamSub != null），保持 isStreaming 不变，
  /// 让 UI 继续显示加载状态，避免页面切换回来后错误显示"（未收到回复）"。
  void recover() {
    if (!hasStreamingMessage) return;
    if (_streamSub != null) return; // stream 仍在运行，不中断
    state = state.map((m) {
      if (!m.isStreaming) return m;
      return m.copyWith(isStreaming: false);
    }).toList();
  }

  /// 清空当前会话的消息（不删除会话本身）。
  /// 在临时状态下（_lastLoadedSessionId == null）跳过保存。
  void clear() {
    _streamSub?.cancel();
    _streamSub = null;
    _saveTimer?.cancel();
    state = [];
    // 保存空列表（覆盖删除旧消息数据），但临时状态下跳过
    if (_lastLoadedSessionId != null) {
      _saveCurrentSession();
      _updateSessionStats();
    }
  }
}

final llmChatSessionProvider =
    NotifierProvider<LlmChatSessionNotifier, List<LlmChatMessage>>(
  () => LlmChatSessionNotifier(),
);
