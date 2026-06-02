import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:solosoul_flutter/core/models/chat_session.dart';
import 'package:solosoul_flutter/core/router/app_router.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/llm/chat_session_list_provider.dart';
import 'package:solosoul_flutter/presentation/providers/llm/selected_chat_session_provider.dart';
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';
import 'package:solosoul_flutter/presentation/widgets/llm/llm_chat_panel.dart';
import 'package:solosoul_flutter/presentation/widgets/llm/chat_session_sidebar.dart';

// =============================================================================
// LLM Chat Page
// =============================================================================

/// 独立的 AI 对话页面，支持多会话侧边栏。
///
/// 路由: `/llm_chat`
///
/// 响应式布局：
/// - 宽屏（> 800px）：左侧固定会话侧边栏 + 右侧聊天面板
/// - 窄屏（≤ 800px）：Scaffold + Drawer 会话列表
class LlmChatPage extends ConsumerStatefulWidget {
  const LlmChatPage({super.key});

  @override
  ConsumerState<LlmChatPage> createState() => _LlmChatPageState();
}

class _LlmChatPageState extends ConsumerState<LlmChatPage> {
  final GlobalKey<ScaffoldState> _scaffoldKey = GlobalKey<ScaffoldState>();

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final isWide = constraints.maxWidth > 800;
        final selectedId = ref.watch(selectedChatSessionIdProvider);
        final sessionsAsync = ref.watch(chatSessionListProvider);
        final title = _buildTitle(sessionsAsync, selectedId, context);

        if (isWide) {
          // Wide screen: fixed sidebar + chat panel
          return Scaffold(
            appBar: SoloGlassAppBar(
              backRoute: AppRoutes.home,
              title: Text(title),
              centerTitle: true,
              actions: [
                IconButton(
                  icon: const Icon(Icons.settings),
                  tooltip: AppLocalizations.of(context).llmConfigTitle,
                  onPressed: () => context.push(AppRoutes.llmConfig),
                ),
              ],
            ),
            body: const Row(
              children: [
                ChatSessionSidebar(
                  expanded: true,
                  isDrawer: false,
                ),
                Expanded(
                  child: LlmChatPanel(showClearButton: true),
                ),
              ],
            ),
          );
        } else {
          // Narrow screen: drawer
          return Scaffold(
            key: _scaffoldKey,
            appBar: SoloGlassAppBar(
              backRoute: AppRoutes.home,
              title: Text(title),
              centerTitle: true,
              leading: IconButton(
                icon: const Icon(Icons.menu),
                onPressed: () => _scaffoldKey.currentState?.openDrawer(),
              ),
              actions: [
                IconButton(
                  icon: const Icon(Icons.settings),
                  tooltip: AppLocalizations.of(context).llmConfigTitle,
                  onPressed: () => context.push(AppRoutes.llmConfig),
                ),
              ],
            ),
            drawer: const Drawer(
              child: ChatSessionSidebar(
                expanded: true,
                isDrawer: true,
              ),
            ),
            body: const LlmChatPanel(showClearButton: true),
          );
        }
      },
    );
  }

  String _buildTitle(
    AsyncValue<List<ChatSession>> sessionsAsync,
    String? selectedId,
    BuildContext context,
  ) {
    if (selectedId == null || isNewChatSessionId(selectedId)) {
      return AppLocalizations.of(context).llmChatTitle;
    }

    return sessionsAsync.when(
      data: (sessions) {
        final session = sessions.firstWhere(
          (s) => s.id == selectedId,
          orElse: () => const ChatSession(
            id: '',
            title: '',
            createdAt: 0,
            updatedAt: 0,
          ),
        );
        return session.title.isEmpty
            ? AppLocalizations.of(context).llmChatTitle
            : session.title;
      },
      loading: () => AppLocalizations.of(context).llmChatTitle,
      error: (_, __) => AppLocalizations.of(context).llmChatTitle,
    );
  }
}
