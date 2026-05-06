import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

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
}

/// 管理 LLM 对话会话的消息列表。
///
/// **后台接收支持：** Stream 订阅由 Notifier 持有（生命周期与 Provider 一致），
/// 切换页面后 widget 被 dispose 但 Provider 继续存活，stream 数据持续累积。
/// 切回页面时 widget 从最新状态重建，直接显示后台已接收的内容。
class LlmChatSessionNotifier extends Notifier<List<LlmChatMessage>> {
  StreamSubscription<String>? _streamSub;

  @override
  List<LlmChatMessage> build() => [];

  bool get hasStreamingMessage => state.any((m) => m.isStreaming);

  /// 发送用户消息并开始消费 AI 流式响应。
  ///
  /// Stream 订阅由 Notifier 本地持有，widget dispose 不影响后台接收。
  Future<void> sendMessage(String text, Stream<String> stream) async {
    if (hasStreamingMessage) return;

    // 1. 添加用户消息
    final userId = 'user_${DateTime.now().millisecondsSinceEpoch}';
    state = [...state, LlmChatMessage(id: userId, text: text, isUser: true)];

    // 2. 添加空 AI 消息（等待流式填充）
    final aiId = 'ai_${DateTime.now().millisecondsSinceEpoch}';
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
            text: '（模型未返回任何内容，请检查配置或重试）',
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
      },
      onError: (Object err) {
        debounceTimer?.cancel();
        state = state.map((m) {
          if (m.id != aiId) return m;
          return m.copyWith(
            text: '推理失败: $err',
            isStreaming: false,
          );
        }).toList();
        _streamSub = null;
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
    state = [];
  }
}

final llmChatSessionProvider =
    NotifierProvider<LlmChatSessionNotifier, List<LlmChatMessage>>(
  () => LlmChatSessionNotifier(),
);
