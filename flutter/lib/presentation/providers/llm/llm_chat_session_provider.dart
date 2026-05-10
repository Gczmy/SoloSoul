import 'dart:async';
import 'dart:math';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:solosoul_flutter/core/services/llm/chat_history_service.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_notifier.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_types.dart' show AuthState;

// =============================================================================
// LLM Chat Session Provider
// =============================================================================

/// 单条聊天消息（纯文本，可序列化到 Provider 状态）。
class LlmChatMessage {
  final String id;
  final String text;
  final bool isUser;

  /// 是否仍在流式输出中。Widget dispose 后重新构建时，
  /// 任何遗留的 `isStreaming=true` 消息会被自动标记为完成。
  final bool isStreaming;

  const LlmChatMessage({
    required this.id,
    required this.text,
    required this.isUser,
    this.isStreaming = false,
  });

  LlmChatMessage copyWith({
    String? id,
    String? text,
    bool? isUser,
    bool? isStreaming,
  }) {
    return LlmChatMessage(
      id: id ?? this.id,
      text: text ?? this.text,
      isUser: isUser ?? this.isUser,
      isStreaming: isStreaming ?? this.isStreaming,
    );
  }

  /// 转换为可序列化的数据对象。
  ChatMessageData toData() => ChatMessageData(
        id: id,
        text: text,
        isUser: isUser,
      );

  /// 从序列化数据对象恢复。
  factory LlmChatMessage.fromData(ChatMessageData data) => LlmChatMessage(
        id: data.id,
        text: data.text,
        isUser: data.isUser,
      );
}

/// 管理 LLM 对话会话的消息列表。
///
/// **后台接收支持：** Stream 订阅由 Notifier 持有（生命周期与 Provider 一致），
/// 切换页面后 widget 被 dispose 但 Provider 继续存活，stream 数据持续累积。
/// 切回页面时 widget 从最新状态重建，直接显示后台已接收的内容。
///
/// **持久化支持：** 消息变更自动 debounce 保存到加密 Vault；
/// 账号切换或 Vault 解锁时自动加载/清空。
class LlmChatSessionNotifier extends Notifier<List<LlmChatMessage>> {
  StreamSubscription<String>? _streamSub;
  Timer? _saveTimer;
  String? _lastLoadedAccountId;

  static const _saveDebounce = Duration(seconds: 2);

  @override
  List<LlmChatMessage> build() {
    // 监听认证状态：解锁时加载历史，锁定时清空
    ref.listen(authNotifierProvider, (previous, next) {
      final nextState = next.value;

      if (nextState == AuthState.unlocked) {
        // Vault 解锁后加载当前账号的历史记录
        _loadHistoryAsync();
      } else if (nextState == AuthState.locked) {
        // Vault 锁定时清空内存中的消息（敏感数据不留在内存）
        _saveTimer?.cancel();
        _streamSub?.cancel();
        _streamSub = null;
        if (state.isNotEmpty) {
          state = [];
        }
        _lastLoadedAccountId = null;
      }
    });

    // 如果当前已经是解锁状态，立即异步加载
    final currentState = ref.read(authNotifierProvider).value;
    if (currentState == AuthState.unlocked) {
      _loadHistoryAsync();
    }

    return [];
  }

  bool get hasStreamingMessage => state.any((m) => m.isStreaming);

  // ---------------------------------------------------------------------------
  // Persistence
  // ---------------------------------------------------------------------------

  /// 异步加载当前账号的聊天历史。
  Future<void> _loadHistoryAsync() async {
    final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
    if (accountId == null || accountId == _lastLoadedAccountId) return;

    final history = await ChatHistoryService.instance.load(accountId);
    if (history.isNotEmpty) {
      _lastLoadedAccountId = accountId;
      state = history
          .map((data) => LlmChatMessage.fromData(data))
          .toList();
      SoloLog.d('LlmChatSession', 'loaded ${history.length} messages');
    } else {
      _lastLoadedAccountId = accountId;
      if (state.isNotEmpty) {
        state = [];
      }
    }
  }

  /// 立即保存当前消息列表。
  Future<void> _saveHistory() async {
    final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
    if (accountId == null) return;

    final persistable = state
        .where((m) => !m.isStreaming)
        .map((m) => m.toData())
        .toList();

    await ChatHistoryService.instance.save(accountId, persistable);
  }

  /// Debounced 保存：避免每次 state 变更都触发 Vault IO。
  void _debouncedSave() {
    _saveTimer?.cancel();
    _saveTimer = Timer(_saveDebounce, _saveHistory);
  }

  // ---------------------------------------------------------------------------
  // Send Message
  // ---------------------------------------------------------------------------

  /// 发送用户消息并开始消费 AI 流式响应。
  ///
  /// Stream 订阅由 Notifier 本地持有，widget dispose 不影响后台接收。
  Future<void> sendMessage(String text, Stream<String> stream, {required AppLocalizations l10n}) async {
    if (hasStreamingMessage) return;

    // 1. 添加用户消息
    final userId = 'user_${DateTime.now().millisecondsSinceEpoch}_${Random.secure().nextInt(999999)}';
    state = [...state, LlmChatMessage(id: userId, text: text, isUser: true)];
    _debouncedSave();

    // 2. 添加空 AI 消息（等待流式填充）
    final aiId = 'ai_${DateTime.now().millisecondsSinceEpoch}_${Random.secure().nextInt(999999)}';
    state = [
      ...state,
      LlmChatMessage(id: aiId, text: '', isUser: false, isStreaming: true),
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
      },
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

  void clear() {
    _streamSub?.cancel();
    _streamSub = null;
    _saveTimer?.cancel();
    state = [];
    // 保存空列表（覆盖删除旧数据）
    _saveHistory();
  }
}

final llmChatSessionProvider =
    NotifierProvider<LlmChatSessionNotifier, List<LlmChatMessage>>(
  () => LlmChatSessionNotifier(),
);
