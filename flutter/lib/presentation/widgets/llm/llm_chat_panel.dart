import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:solosoul_flutter/core/router/app_router.dart';
import 'package:solosoul_flutter/core/services/llm/llm_model_state.dart';
import 'package:solosoul_flutter/core/services/llm/llm_config_models.dart';
import 'package:solosoul_flutter/core/services/llm/llm_service.dart';
import 'package:solosoul_flutter/presentation/providers/llm/llm_chat_session_provider.dart';
import 'package:solosoul_flutter/presentation/providers/llm/llm_config_provider.dart';
import 'package:solosoul_flutter/presentation/providers/llm/llm_model_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/llm/llm_chat_bubble.dart';

// =============================================================================
// LLM Chat Panel
// =============================================================================

/// 可复用的 LLM 聊天面板。
///
/// 消息状态由 [llmChatSessionProvider] 持久化管理，切换页面不会丢失。
/// 流式输出期间逐字更新 Provider，返回页面后可看到已累积的部分文本。
class LlmChatPanel extends ConsumerStatefulWidget {
  /// 面板顶部标题。为 `null` 时不显示标题栏。
  final String? title;

  /// 是否显示清除会话按钮。
  final bool showClearButton;

  const LlmChatPanel({
    super.key,
    this.title,
    this.showClearButton = true,
  });

  @override
  ConsumerState<LlmChatPanel> createState() => _LlmChatPanelState();
}

class _LlmChatPanelState extends ConsumerState<LlmChatPanel> {
  final _inputController = TextEditingController();
  final _scrollController = ScrollController();
  bool _isLoadingConfig = false;
  String? _loadError;

  Future<void> _loadModel() async {
    if (_isLoadingConfig) return;
    setState(() {
      _isLoadingConfig = true;
      _loadError = null;
    });
    try {
      await ref.read(llmModelProvider.notifier).loadFromConfig();
    } on LlmException catch (e) {
      if (mounted) {
        setState(() => _loadError = e.message);
      }
    } on Exception catch (e) {
      if (mounted) {
        setState(() => _loadError = e.toString());
      }
    } finally {
      if (mounted) {
        setState(() => _isLoadingConfig = false);
      }
    }
  }

  @override
  void initState() {
    super.initState();
    // 恢复遗留的流式消息（页面切换后）
    WidgetsBinding.instance.addPostFrameCallback((_) {
      ref.read(llmChatSessionProvider.notifier).recover();
    });
    // 面板初始化时自动加载已配置的模型（带防抖，避免重复调用）
    WidgetsBinding.instance.addPostFrameCallback((_) async {
      final modelAsync = ref.read(llmModelProvider);
      final shouldLoad = !modelAsync.hasValue ||
          modelAsync.value == LlmModelState.unloaded ||
          modelAsync.value == LlmModelState.error;
      if (shouldLoad) {
        await _loadModel();
      }
    });
  }

  @override
  void dispose() {
    _inputController.dispose();
    _scrollController.dispose();
    super.dispose();
  }

  ({IconData icon, String label})? _backendStatus(LlmConfigState? config, bool isReady) {
    if (config == null) return null;
    final icon = config.backendType == LlmBackendType.cloud
        ? Icons.cloud_outlined
        : Icons.computer_outlined;
    final source = config.backendType == LlmBackendType.cloud ? '云端' : '本地';
    final model = config.backendType == LlmBackendType.cloud
        ? (config.activeCloudProfile?.model ?? config.cloudModel)
        : (config.localModelPath ?? 'qwen2.5:1.5b');
    final status = isReady ? '就绪' : '未就绪';
    return (icon: icon, label: '$source · ${model.isNotEmpty ? model : '未配置'} · $status');
  }

  void _scrollToBottom() {
    Future.delayed(const Duration(milliseconds: 50), () {
      if (_scrollController.hasClients) {
        _scrollController.animateTo(
          _scrollController.position.maxScrollExtent,
          duration: const Duration(milliseconds: 200),
          curve: Curves.easeOut,
        );
      }
    });
  }

  Future<void> _sendMessage() async {
    final text = _inputController.text.trim();
    if (text.isEmpty) return;

    final modelState = ref.read(llmModelProvider);
    if (!modelState.hasValue || modelState.value != LlmModelState.loaded) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('模型尚未加载，请先配置 LLM')),
      );
      return;
    }

    final session = ref.read(llmChatSessionProvider.notifier);
    final notifier = ref.read(llmModelProvider.notifier);

    // 获取流式响应
    late final Stream<String> stream;
    try {
      stream = notifier.streamChat(text);
    } on Exception catch (e) {
      await session.sendMessage(text, Stream.error(e));
      return;
    }

    // 获取 stream 成功后立即清空输入框，避免重复发送
    _inputController.clear();
    // 交给 Provider 在后台消费，widget dispose 不影响
    await session.sendMessage(text, stream);
  }

  void _clearSession() {
    ref.read(llmChatSessionProvider.notifier).clear();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final modelAsync = ref.watch(llmModelProvider);
    final configAsync = ref.watch(llmConfigProvider);
    final messages = ref.watch(llmChatSessionProvider);
    final isReady = modelAsync.value == LlmModelState.loaded;
    final isSending = messages.any((m) => m.isStreaming);
    final backendStatus = _backendStatus(configAsync.value, isReady);

    // AI 输出期间自动滚动到底部
    if (isSending) {
      WidgetsBinding.instance.addPostFrameCallback((_) => _scrollToBottom());
    }

    return Column(
      children: [
        // Title bar
        if (widget.title != null)
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
            decoration: BoxDecoration(
              border: Border(
                bottom: BorderSide(color: theme.colorScheme.outlineVariant),
              ),
            ),
            child: Row(
              children: [
                Text(
                  widget.title!,
                  style: theme.textTheme.titleMedium?.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const Spacer(),
                _ModelStatusChip(isReady: isReady),
                if (widget.showClearButton) ...[
                  const SizedBox(width: 8),
                  IconButton(
                    icon: const Icon(Icons.delete_outline, size: 20),
                    tooltip: '清除会话',
                    onPressed: messages.isEmpty ? null : _clearSession,
                  ),
                ],
              ],
            ),
          ),

        // Message list
        Expanded(
          child: messages.isEmpty
              ? _EmptyState(
                  theme: theme,
                  configAsync: configAsync,
                  loadError: _loadError,
                  isLoading: _isLoadingConfig,
                  onLoadModel: _loadModel,
                )
              : ListView.builder(
                  controller: _scrollController,
                  padding: const EdgeInsets.all(16),
                  itemCount: messages.length,
                  itemBuilder: (context, index) {
                    final msg = messages[index];
                    return Padding(
                      padding: const EdgeInsets.only(bottom: 12),
                      child: _buildMessageBubble(theme, msg),
                    );
                  },
                ),
        ),

        // Model status + Input area
        SafeArea(
          child: Container(
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 12),
            decoration: BoxDecoration(
              border: Border(
                top: BorderSide(color: theme.colorScheme.outlineVariant),
              ),
            ),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                // 状态行：[圆点][图标][模型名称]
                Row(
                  children: [
                    Container(
                      width: 8,
                      height: 8,
                      decoration: BoxDecoration(
                        shape: BoxShape.circle,
                        color: isReady ? Colors.green : Colors.red,
                      ),
                    ),
                    const SizedBox(width: 8),
                    if (backendStatus != null) ...[
                      Icon(
                        backendStatus.icon,
                        size: 14,
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                      const SizedBox(width: 6),
                      Expanded(
                        child: Text(
                          backendStatus.label,
                          style: theme.textTheme.labelSmall?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                    ],
                  ],
                ),
                const SizedBox(height: 8),
                // 输入框 + 发送按钮
                Row(
                  children: [
                    Expanded(
                      child: TextField(
                        controller: _inputController,
                        enabled: isReady && !isSending,
                        maxLines: 4,
                        minLines: 1,
                        textInputAction: TextInputAction.send,
                        onSubmitted: (_) => _sendMessage(),
                        decoration: InputDecoration(
                          hintText: isReady ? '输入消息...' : '模型未就绪',
                          filled: true,
                          fillColor: theme.colorScheme.surfaceContainerHighest,
                          contentPadding: const EdgeInsets.symmetric(
                            horizontal: 16,
                            vertical: 12,
                          ),
                          border: OutlineInputBorder(
                            borderRadius: BorderRadius.circular(24),
                            borderSide: BorderSide.none,
                          ),
                        ),
                      ),
                    ),
                    const SizedBox(width: 8),
                    IconButton.filled(
                      onPressed: (isReady && !isSending) ? _sendMessage : null,
                      icon: isSending
                          ? const SizedBox(
                              width: 20,
                              height: 20,
                              child: CircularProgressIndicator(
                                strokeWidth: 2,
                                color: Colors.white,
                              ),
                            )
                          : const Icon(Icons.send),
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
      ],
    );
  }

  Widget _buildMessageBubble(ThemeData theme, LlmChatMessage msg) {
    if (msg.isUser) {
      return LlmChatBubble(
        message: msg.text,
        isUser: true,
        isStreaming: false,
      );
    }

    // AI 消息
    if (msg.isStreaming && msg.text.isEmpty) {
      // 刚开始流式输出，尚无文本
      return const Row(
        children: [
          SizedBox(
            width: 16,
            height: 16,
            child: CircularProgressIndicator(strokeWidth: 2),
          ),
          SizedBox(width: 8),
          Text('正在思考…'),
        ],
      );
    }

    if (!msg.isStreaming && msg.text.isEmpty) {
      // 流式结束但未收到任何内容
      return Text(
        '（未收到回复）',
        style: theme.textTheme.bodySmall?.copyWith(
          color: theme.colorScheme.error,
          fontStyle: FontStyle.italic,
        ),
      );
    }

    // 已完成 或 流式中已有文本（LlmChatBubble 自带 typing dots）
    return LlmChatBubble(
      message: msg.text,
      isUser: false,
      isStreaming: msg.isStreaming,
    );
  }
}

// =============================================================================
// Sub-widgets
// =============================================================================

class _ModelStatusChip extends StatelessWidget {
  final bool isReady;

  const _ModelStatusChip({required this.isReady});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Chip(
      visualDensity: VisualDensity.compact,
      avatar: Icon(
        isReady ? Icons.check_circle : Icons.error_outline,
        size: 16,
        color: isReady ? theme.colorScheme.primary : theme.colorScheme.error,
      ),
      label: Text(
        isReady ? '就绪' : '未就绪',
        style: theme.textTheme.labelSmall,
      ),
    );
  }
}

class _EmptyState extends StatelessWidget {
  final ThemeData theme;
  final AsyncValue<LlmConfigState> configAsync;
  final String? loadError;
  final bool isLoading;
  final VoidCallback onLoadModel;

  const _EmptyState({
    required this.theme,
    required this.configAsync,
    this.loadError,
    this.isLoading = false,
    required this.onLoadModel,
  });

  @override
  Widget build(BuildContext context) {
    final hasError = loadError != null && loadError!.isNotEmpty;
    final config = configAsync.hasValue ? configAsync.value : null;
    final isCloud = config?.backendType == LlmBackendType.cloud;
    final loadLabel = isCloud ? '连接云端模型' : '启动本地模型';

    return Center(
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 32),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            if (isLoading)
              SizedBox(
                width: 48,
                height: 48,
                child: CircularProgressIndicator(
                  strokeWidth: 2,
                  color: theme.colorScheme.primary,
                ),
              )
            else
              Icon(
                hasError ? Icons.error_outline : Icons.chat_bubble_outline,
                size: 48,
                color: hasError
                    ? theme.colorScheme.error
                    : theme.colorScheme.onSurfaceVariant,
              ),
            const SizedBox(height: 16),
            Text(
              isLoading
                  ? '正在加载模型配置…'
                  : (loadError ?? '开始与 AI 对话'),
              textAlign: TextAlign.center,
              style: theme.textTheme.bodyMedium?.copyWith(
                color: hasError
                    ? theme.colorScheme.error
                    : theme.colorScheme.onSurfaceVariant,
              ),
            ),
            if (!isLoading) ...[
              const SizedBox(height: 24),
              // 手动启动模型按钮
              FilledButton.icon(
                onPressed: onLoadModel,
                icon: Icon(isCloud ? Icons.cloud : Icons.computer),
                label: Text(loadLabel),
              ),
              const SizedBox(height: 12),
              // 跳转到配置页面
              TextButton.icon(
                onPressed: () => context.push(AppRoutes.llmConfig),
                icon: const Icon(Icons.settings, size: 16),
                label: const Text('前往 LLM 配置'),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
