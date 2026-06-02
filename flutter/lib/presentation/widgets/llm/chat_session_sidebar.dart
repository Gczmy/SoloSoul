import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart' show showOverlaySnackBar, SnackBarType;

import 'package:solosoul_flutter/core/models/chat_session.dart';
import 'package:solosoul_flutter/core/services/llm/chat_history_service.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_notifier.dart';
import 'package:solosoul_flutter/presentation/providers/llm/chat_session_list_provider.dart';
import 'package:solosoul_flutter/presentation/providers/llm/llm_chat_session_provider.dart';
import 'package:solosoul_flutter/presentation/providers/llm/selected_chat_session_provider.dart';

// =============================================================================
// Chat Session Sidebar
// =============================================================================

/// 会话列表侧边栏，支持宽屏固定模式和窄屏 Drawer 模式。
///
/// - 显示会话列表，按最近更新时间降序排列
/// - 支持新建、重命名、删除会话
/// - 支持折叠/展开，状态持久化到 UserPreferences
class ChatSessionSidebar extends ConsumerStatefulWidget {
  final bool expanded;
  final bool isDrawer;

  const ChatSessionSidebar({
    super.key,
    this.expanded = true,
    this.isDrawer = false,
  });

  @override
  ConsumerState<ChatSessionSidebar> createState() => _ChatSessionSidebarState();
}

class _ChatSessionSidebarState extends ConsumerState<ChatSessionSidebar> {
  final _renameController = TextEditingController();
  String? _renamingSessionId;
  bool _isTrashExpanded = false;

  static const double _expandedWidth = 240;
  static const double _collapsedWidth = 72;

  @override
  void dispose() {
    _renameController.dispose();
    super.dispose();
  }

  void _startRename(ChatSession session) {
    setState(() {
      _renamingSessionId = session.id;
      _renameController.text = session.title;
    });
  }

  void _confirmRename(String sessionId) {
    final newTitle = _renameController.text.trim();
    if (newTitle.isNotEmpty) {
      ref.read(chatSessionListProvider.notifier).updateSessionTitle(sessionId, newTitle);
    }
    setState(() => _renamingSessionId = null);
  }

  void _onNewSession() {
    final currentId = ref.read(selectedChatSessionIdProvider);
    if (isNewChatSessionId(currentId)) return; // Already temporary, no-op
    ref.read(selectedChatSessionIdProvider.notifier).select(kNewChatSessionId);
  }

  /// 软删除：无对话框，直接移入回收站并显示 Overlay Toast。
  void _deleteSession(ChatSession session, AppLocalizations l10n) {
    final notifier = ref.read(chatSessionListProvider.notifier);
    notifier.softDeleteSession(session.id);
    showOverlaySnackBar(
      context,
      content: l10n.llmChatSessionMovedToTrash,
      duration: const Duration(seconds: 4),
      type: SnackBarType.info,
      actionLabel: l10n.llmChatUndoDelete,
      onAction: () => notifier.restoreSession(session.id),
    );
  }

  /// 硬删除：弹出对话框二次确认。
  void _confirmHardDelete(ChatSession session, AppLocalizations l10n) {
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(l10n.llmChatDeleteForever),
        content: Text(l10n.llmChatHardDeleteSessionConfirm(session.title)),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(),
            child: Text(MaterialLocalizations.of(ctx).cancelButtonLabel),
          ),
          TextButton(
            onPressed: () {
              Navigator.of(ctx).pop();
              ref.read(chatSessionListProvider.notifier).hardDeleteSession(session.id);
            },
            style: TextButton.styleFrom(foregroundColor: Theme.of(ctx).colorScheme.error),
            child: Text(l10n.llmChatDeleteForever),
          ),
        ],
      ),
    );
  }

  void _emptyTrash(AppLocalizations l10n) {
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(l10n.llmChatEmptyTrash),
        content: Text(l10n.llmChatEmptyTrashConfirm),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(),
            child: Text(MaterialLocalizations.of(ctx).cancelButtonLabel),
          ),
          TextButton(
            onPressed: () {
              Navigator.of(ctx).pop();
              ref.read(chatSessionListProvider.notifier).emptyTrash();
            },
            style: TextButton.styleFrom(foregroundColor: Theme.of(ctx).colorScheme.error),
            child: Text(l10n.llmChatDeleteForever),
          ),
        ],
      ),
    );
  }

  void _selectSession(String sessionId, AppLocalizations l10n) {
    final isStreaming = ref.read(llmChatSessionProvider.notifier).hasStreamingMessage;
    if (isStreaming) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(l10n.llmChatSwitchBlocked),
          duration: const Duration(seconds: 2),
        ),
      );
      return;
    }
    ref.read(selectedChatSessionIdProvider.notifier).select(sessionId);
    if (widget.isDrawer) {
      Navigator.of(context).pop(); // Close drawer
    }
  }

  String _formatTime(int timestamp, AppLocalizations l10n) {
    final dt = DateTime.fromMillisecondsSinceEpoch(timestamp);
    final now = DateTime.now();
    final diff = now.difference(dt);

    if (diff.inMinutes < 1) return l10n.timeJustNow;
    if (diff.inHours < 1) return l10n.timeMinutesAgo(diff.inMinutes);
    if (diff.inDays < 1) return l10n.timeHoursAgo(diff.inHours);
    if (diff.inDays < 7) return l10n.timeDaysAgo(diff.inDays);
    return l10n.timeDateShort(dt.month, dt.day);
  }

  String _formatMessageTime(int timestamp, AppLocalizations l10n) {
    final dt = DateTime.fromMillisecondsSinceEpoch(timestamp);
    final hour = dt.hour.toString().padLeft(2, '0');
    final minute = dt.minute.toString().padLeft(2, '0');
    final timePart = '$hour:$minute';

    final now = DateTime.now();
    final today = DateTime(now.year, now.month, now.day);
    final msgDay = DateTime(dt.year, dt.month, dt.day);
    final diffDays = today.difference(msgDay).inDays;

    if (diffDays == 0) return timePart;
    if (diffDays == 1) return '${l10n.timeYesterday} $timePart';
    if (dt.year == now.year) return '${dt.month}/${dt.day} $timePart';
    return '${dt.year}/${dt.month}/${dt.day} $timePart';
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context);
    final sessionsAsync = ref.watch(chatSessionListProvider);
    final selectedId = ref.watch(selectedChatSessionIdProvider);

    return Container(
      width: widget.expanded ? _expandedWidth : _collapsedWidth,
      color: theme.colorScheme.surfaceContainerLowest,
      child: Column(
        children: [
          // Header: New Session button
          _buildHeader(theme, l10n),
          const Divider(height: 1),
          // Session list + Trash
          Expanded(
            child: sessionsAsync.when(
              data: (sessions) => Column(
                children: [
                  Expanded(
                    child: _buildSessionList(sessions, selectedId, theme, l10n),
                  ),
                  if (widget.expanded) _buildTrashSection(sessions, theme, l10n),
                ],
              ),
              loading: () => const Center(child: CircularProgressIndicator()),
              error: (err, _) => Center(child: Text('Error: $err')),
            ),
          ),
          // Footer: collapse toggle (only in wide mode)
          if (!widget.isDrawer)
            _buildFooter(theme, l10n),
        ],
      ),
    );
  }

  Widget _buildHeader(ThemeData theme, AppLocalizations l10n) {
    return Padding(
      padding: const EdgeInsets.all(12),
      child: widget.expanded
          ? FilledButton.icon(
              onPressed: _onNewSession,
              icon: const Icon(Icons.add, size: 18),
              label: Text(l10n.llmChatNewSession),
            )
          : IconButton(
              icon: const Icon(Icons.add),
              tooltip: l10n.llmChatNewSession,
              onPressed: _onNewSession,
            ),
    );
  }

  Widget _buildSessionList(List<ChatSession> sessions, String? selectedId, ThemeData theme, AppLocalizations l10n) {
    // Only show active (non-deleted) sessions in the main list
    final activeSessions = sessions.where((s) => !s.isDeleted).toList();
    final isTemp = isNewChatSessionId(selectedId);

    if (activeSessions.isEmpty) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Text(
            widget.expanded ? l10n.llmChatStartConversation : '',
            textAlign: TextAlign.center,
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
        ),
      );
    }

    return ListView.builder(
      itemCount: activeSessions.length,
      padding: const EdgeInsets.symmetric(vertical: 4),
      itemBuilder: (context, index) {
        final session = activeSessions[index];
        final isSelected = !isTemp && session.id == selectedId;
        final isRenaming = _renamingSessionId == session.id;

        return _buildSessionTile(session, isSelected, isRenaming, theme, l10n);
      },
    );
  }

  Widget _buildSessionTile(
    ChatSession session,
    bool isSelected,
    bool isRenaming,
    ThemeData theme,
    AppLocalizations l10n,
  ) {
    final bgColor = isSelected
        ? theme.colorScheme.primary.withValues(alpha: 0.1)
        : Colors.transparent;
    final fgColor = isSelected
        ? theme.colorScheme.primary
        : theme.colorScheme.onSurface;

    Widget content;

    if (!widget.expanded) {
      // Collapsed: just icon
      content = InkWell(
        onTap: () => _selectSession(session.id, l10n),
        child: Container(
          height: 44,
          alignment: Alignment.center,
          child: Icon(
            Icons.chat_bubble_outline,
            size: 20,
            color: fgColor,
          ),
        ),
      );
    } else if (isRenaming) {
      // Inline rename
      content = Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
        child: TextField(
          controller: _renameController,
          autofocus: true,
          decoration: const InputDecoration(
            isDense: true,
            contentPadding: EdgeInsets.symmetric(horizontal: 8, vertical: 6),
          ),
          onSubmitted: (_) => _confirmRename(session.id),
          onTapOutside: (_) => _confirmRename(session.id),
        ),
      );
    } else {
      // Normal expanded tile
      content = InkWell(
        onTap: () => _selectSession(session.id, l10n),
        onLongPress: () => _showSessionMenu(session, l10n),
        child: Container(
          height: 56,
          padding: const EdgeInsets.only(left: 12, right: 4, top: 8, bottom: 8),
          child: Row(
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    Text(
                      session.title.isEmpty ? l10n.llmChatDefaultSessionTitle : session.title,
                      style: theme.textTheme.bodyMedium?.copyWith(
                        color: fgColor,
                        fontWeight: isSelected ? FontWeight.w600 : null,
                      ),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                    const SizedBox(height: 2),
                    Text(
                      l10n.llmChatMessageCount(session.messageCount, _formatTime(session.updatedAt, l10n)),
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                        fontSize: 11,
                      ),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ],
                ),
              ),
              // PopupMenuButton — Flutter automatically anchors the menu
              // directly below the icon, no manual position calculation needed.
              PopupMenuButton<String>(
                icon: Icon(
                  Icons.more_vert,
                  size: 18,
                  color: isSelected
                      ? theme.colorScheme.primary
                      : theme.colorScheme.onSurfaceVariant,
                ),
                padding: EdgeInsets.zero,
                constraints: const BoxConstraints(minWidth: 28, minHeight: 28),
                splashRadius: 20,
                itemBuilder: (_) => [
                  PopupMenuItem(
                    value: 'rename',
                    child: Row(
                      children: [
                        const Icon(Icons.edit, size: 18),
                        const SizedBox(width: 8),
                        Text(l10n.llmChatRenameSession),
                      ],
                    ),
                  ),
                  PopupMenuItem(
                    value: 'delete',
                    child: Row(
                      children: [
                        Icon(Icons.delete_outline, size: 18, color: theme.colorScheme.error),
                        const SizedBox(width: 8),
                        Text(l10n.llmChatDeleteSession, style: TextStyle(color: theme.colorScheme.error)),
                      ],
                    ),
                  ),
                ],
                onSelected: (value) {
                  if (value == 'rename') {
                    _startRename(session);
                  } else if (value == 'delete') {
                    _deleteSession(session, l10n);
                  }
                },
              ),
            ],
          ),
        ),
      );
    }

    return Padding(
      padding: const EdgeInsets.symmetric(
        horizontal: 8,
        vertical: 2,
      ),
      child: Material(
        color: bgColor,
        borderRadius: BorderRadius.circular(8),
        child: content,
      ),
    );
  }

  void _showSessionMenu(ChatSession session, AppLocalizations l10n, {BuildContext? anchorContext}) {
    final anchor = anchorContext ?? context;
    final RenderBox? overlay =
        Overlay.of(context).context.findRenderObject() as RenderBox?;
    final RenderBox? tile = anchor.findRenderObject() as RenderBox?;

    showMenu(
      context: context,
      position: RelativeRect.fromRect(
        Rect.fromPoints(
          tile?.localToGlobal(Offset.zero) ?? Offset.zero,
          tile?.localToGlobal(tile.size.bottomRight(Offset.zero)) ?? Offset.zero,
        ),
        Offset.zero & (overlay?.size ?? Size.zero),
      ),
      items: [
        PopupMenuItem(
          onTap: () => Future.microtask(() => _startRename(session)),
          child: Row(
            children: [
              const Icon(Icons.edit, size: 18),
              const SizedBox(width: 8),
              Text(l10n.llmChatRenameSession),
            ],
          ),
        ),
        PopupMenuItem(
          onTap: () => Future.microtask(() => _deleteSession(session, l10n)),
          child: Row(
            children: [
              Icon(Icons.delete_outline, size: 18, color: Theme.of(context).colorScheme.error),
              const SizedBox(width: 8),
              Text(l10n.llmChatDeleteSession, style: TextStyle(color: Theme.of(context).colorScheme.error)),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildFooter(ThemeData theme, AppLocalizations l10n) {
    return Column(
      children: [
        const Divider(height: 1),
        InkWell(
          onTap: () {
            // Notify parent to toggle expanded state
            // For now, this is handled by the parent widget
          },
          child: Container(
            height: 40,
            alignment: Alignment.center,
            child: widget.expanded
                ? Row(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      Icon(
                        Icons.chevron_left,
                        size: 18,
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                      const SizedBox(width: 4),
                      Text(
                        l10n.timeCollapse,
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ],
                  )
                : Icon(
                    Icons.chevron_right,
                    size: 18,
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
          ),
        ),
      ],
    );
  }

  // ---------------------------------------------------------------------------
  // Trash Section
  // ---------------------------------------------------------------------------

  // ---------------------------------------------------------------------------
  // Trash Session Details
  // ---------------------------------------------------------------------------

  void _showTrashSessionDetails(ChatSession session, AppLocalizations l10n) async {
    final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
    if (accountId == null) return;

    final messages = await ChatHistoryService.instance.loadSessionMessages(
      accountId,
      session.id,
    );

    if (!mounted) return;

    await showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(session.title.isEmpty ? l10n.llmChatDefaultSessionTitle : session.title),
        content: SizedBox(
          width: double.maxFinite,
          height: 400,
          child: messages.isEmpty
              ? Center(child: Text(l10n.llmChatStartConversation))
              : ListView.builder(
                  itemCount: messages.length,
                  itemBuilder: (_, index) {
                    final msg = messages[index];
                    return Padding(
                      padding: const EdgeInsets.symmetric(vertical: 6),
                      child: Column(
                        crossAxisAlignment: msg.isUser
                            ? CrossAxisAlignment.end
                            : CrossAxisAlignment.start,
                        children: [
                          Container(
                            padding: const EdgeInsets.symmetric(
                              horizontal: 12,
                              vertical: 8,
                            ),
                            decoration: BoxDecoration(
                              color: msg.isUser
                                  ? Theme.of(ctx).colorScheme.primaryContainer
                                  : Theme.of(ctx).colorScheme.surfaceContainerHighest,
                              borderRadius: BorderRadius.circular(12),
                            ),
                            constraints: const BoxConstraints(maxWidth: 320),
                            child: Text(
                              msg.text,
                              style: Theme.of(ctx).textTheme.bodyMedium,
                            ),
                          ),
                          if (msg.createdAt > 0) ...[
                            const SizedBox(height: 2),
                            Text(
                              _formatMessageTime(msg.createdAt, l10n),
                              style: Theme.of(ctx).textTheme.labelSmall?.copyWith(
                                color: Theme.of(ctx).colorScheme.onSurfaceVariant,
                                fontSize: 11,
                              ),
                            ),
                          ],
                        ],
                      ),
                    );
                  },
                ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(),
            child: Text(l10n.llmChatClose),
          ),
        ],
      ),
    );
  }

  Widget _buildTrashSection(List<ChatSession> sessions, ThemeData theme, AppLocalizations l10n) {
    final deleted = sessions.where((s) => s.isDeleted).toList()
      ..sort((a, b) => (b.deletedAt ?? 0).compareTo(a.deletedAt ?? 0));

    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        const Divider(height: 1),
        InkWell(
          onTap: () => setState(() => _isTrashExpanded = !_isTrashExpanded),
          child: Container(
            height: 40,
            padding: const EdgeInsets.symmetric(horizontal: 12),
            child: Row(
              children: [
                Icon(
                  _isTrashExpanded ? Icons.expand_more : Icons.chevron_right,
                  size: 18,
                  color: theme.colorScheme.onSurfaceVariant,
                ),
                const SizedBox(width: 4),
                Text(
                  l10n.llmChatTrash,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
                const SizedBox(width: 4),
                Text(
                  '(${deleted.length})',
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
                const Spacer(),
                if (deleted.isNotEmpty)
                  IconButton(
                    icon: const Icon(Icons.delete_forever, size: 18),
                    tooltip: l10n.llmChatEmptyTrash,
                    color: theme.colorScheme.error,
                    onPressed: () => _emptyTrash(l10n),
                    padding: EdgeInsets.zero,
                    constraints: const BoxConstraints(minWidth: 32, minHeight: 32),
                  ),
              ],
            ),
          ),
        ),
        if (_isTrashExpanded && deleted.isNotEmpty)
          ...deleted.map((s) => _buildTrashTile(s, theme, l10n)),
      ],
    );
  }

  Widget _buildTrashTile(ChatSession session, ThemeData theme, AppLocalizations l10n) {
    return InkWell(
      onTap: () {}, // Trash items are not selectable
      child: Container(
        height: 40,
        padding: const EdgeInsets.symmetric(horizontal: 12),
        child: Row(
          children: [
            const SizedBox(width: 24), // indent under expand icon
            Expanded(
              child: Text(
                session.title.isEmpty ? l10n.llmChatDefaultSessionTitle : session.title,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
            ),
            IconButton(
              icon: const Icon(Icons.info_outline, size: 16),
              tooltip: l10n.llmChatViewDetails,
              color: theme.colorScheme.onSurfaceVariant,
              onPressed: () => _showTrashSessionDetails(session, l10n),
              padding: EdgeInsets.zero,
              constraints: const BoxConstraints(minWidth: 28, minHeight: 28),
            ),
            IconButton(
              icon: const Icon(Icons.restore, size: 16),
              tooltip: l10n.llmChatRestore,
              color: theme.colorScheme.primary,
              onPressed: () {
                ref.read(chatSessionListProvider.notifier).restoreSession(session.id);
              },
              padding: EdgeInsets.zero,
              constraints: const BoxConstraints(minWidth: 28, minHeight: 28),
            ),
            IconButton(
              icon: const Icon(Icons.delete_forever, size: 16),
              tooltip: l10n.llmChatDeleteForever,
              color: theme.colorScheme.error,
              onPressed: () => _confirmHardDelete(session, l10n),
              padding: EdgeInsets.zero,
              constraints: const BoxConstraints(minWidth: 28, minHeight: 28),
            ),
          ],
        ),
      ),
    );
  }
}
