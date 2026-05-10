import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';
import 'package:go_router/go_router.dart';
import 'package:solosoul_flutter/core/router/app_router.dart';
import 'package:solosoul_flutter/presentation/widgets/llm/llm_chat_panel.dart';

// =============================================================================
// LLM Chat Page
// =============================================================================

/// 独立的 AI 对话页面。
///
/// 路由: `/llm_chat`
class LlmChatPage extends StatelessWidget {
  const LlmChatPage({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: SoloGlassAppBar(
        backRoute: AppRoutes.home,
        title: Text(AppLocalizations.of(context).llmChatTitle),
        centerTitle: true,
        actions: [
          IconButton(
            icon: const Icon(Icons.settings),
            tooltip: AppLocalizations.of(context).llmConfigTitle,
            onPressed: () => context.push(AppRoutes.llmConfig),
          ),
        ],
      ),
      body: const LlmChatPanel(showClearButton: true),
    );
  }
}
