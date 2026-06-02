import 'dart:convert';

import 'package:solosoul_flutter/core/models/chat_session.dart';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';

// =============================================================================
// Chat Message Data (Serializable DTO)
// =============================================================================

/// 可序列化的聊天消息数据（与 [LlmChatMessage] 对应）。
class ChatMessageData {
  final String id;
  final String text;
  final bool isUser;

  /// 消息创建时间戳（毫秒级 Unix epoch）。
  /// 旧数据可能缺少此字段，默认值为 0。
  final int createdAt;

  const ChatMessageData({
    required this.id,
    required this.text,
    required this.isUser,
    this.createdAt = 0,
  });

  Map<String, dynamic> toJson() => {
        'id': id,
        'text': text,
        'isUser': isUser,
        'createdAt': createdAt,
      };

  factory ChatMessageData.fromJson(Map<String, dynamic> json) => ChatMessageData(
        id: json['id'] as String,
        text: json['text'] as String,
        isUser: json['isUser'] as bool,
        createdAt: (json['createdAt'] as num?)?.toInt() ?? 0,
      );
}

// =============================================================================
// Chat History Service
// =============================================================================

/// 管理 AI 对话历史的加密持久化，支持多会话存储。
///
/// 存储结构：
/// - `${accountId}_chat_sessions`    → 会话元数据列表（version 2）
/// - `${accountId}_chat_${sessionId}` → 单个会话的消息列表（version 1）
/// - `${accountId}_chat_backup`      → 迁移后的旧格式备份
///
/// 使用 [RustVaultService] 的 profile 存储机制，复用 SQLCipher 加密保护。
class ChatHistoryService {
  static ChatHistoryService? _instance;
  static ChatHistoryService get instance => _instance ??= ChatHistoryService._();
  ChatHistoryService._();

  final RustVaultService _vault = RustVaultService.instance;

  static const String _legacySessionId = 'default';

  static String _profileName(String accountId) => '${accountId}_chat';
  static String _sessionListName(String accountId) => '${accountId}_chat_sessions';
  static String _sessionMessagesName(String accountId, String sessionId) => '${accountId}_chat_$sessionId';
  static String _legacyBackupName(String accountId) => '${accountId}_chat_backup';

  // ---------------------------------------------------------------------------
  // Legacy Compatibility (maps to default session)
  // ---------------------------------------------------------------------------

  /// 保存聊天历史（兼容旧接口，映射到 default 会话）。
  Future<void> save(String accountId, List<ChatMessageData> messages) async {
    return saveSessionMessages(accountId, _legacySessionId, messages);
  }

  /// 加载聊天历史（兼容旧接口，从 default 会话读取）。
  Future<List<ChatMessageData>> load(String accountId) async {
    return loadSessionMessages(accountId, _legacySessionId);
  }

  /// 删除聊天历史（兼容旧接口，删除 default 会话）。
  Future<void> delete(String accountId) async {
    return deleteSessionMessages(accountId, _legacySessionId);
  }

  // ---------------------------------------------------------------------------
  // Session List (Metadata)
  // ---------------------------------------------------------------------------

  /// 保存会话元数据列表。
  Future<void> saveSessionList(String accountId, List<ChatSession> sessions) async {
    if (!_vault.isVaultUnlocked()) {
      SoloLog.w('ChatHistoryService', 'saveSessionList skipped: vault locked');
      return;
    }

    final payload = {
      'version': 2,
      'savedAt': DateTime.now().toIso8601String(),
      'sessions': sessions.map((s) => s.toJson()).toList(),
    };

    try {
      await _vault.saveProfileEncrypted(
        _sessionListName(accountId),
        jsonEncode(payload),
      );
      SoloLog.d('ChatHistoryService',
          'saved ${sessions.length} sessions for $accountId');
    } on Exception catch (e) {
      SoloLog.w('ChatHistoryService', 'saveSessionList failed: $e');
    }
  }

  /// 加载会话元数据列表（含自动迁移）。
  ///
  /// 若检测到旧格式 `${accountId}_chat`，自动迁移为新的多会话结构，
  /// 并将旧文件重命名为 `${accountId}_chat_backup` 保留备份。
  Future<List<ChatSession>> loadSessionList(String accountId) async {
    if (!_vault.isVaultUnlocked()) {
      SoloLog.w('ChatHistoryService', 'loadSessionList skipped: vault locked');
      return [];
    }

    // 1. Try new format first
    try {
      final jsonStr = await _vault.loadProfileDecrypted(_sessionListName(accountId));
      if (jsonStr != null && jsonStr.isNotEmpty) {
        final map = jsonDecode(jsonStr) as Map<String, dynamic>;
        final sessionsJson = map['sessions'] as List<dynamic>?;
        if (sessionsJson != null) {
          final sessions = sessionsJson
              .whereType<Map<String, dynamic>>()
              .map((json) => ChatSession.fromJson(json))
              .toList();
          SoloLog.d('ChatHistoryService',
              'loaded ${sessions.length} sessions for $accountId');
          return sessions;
        }
      }
    } on Exception catch (e) {
      SoloLog.w('ChatHistoryService', 'loadSessionList new format failed: $e');
    }

    // 2. No new format found — check for legacy data to migrate
    if (await hasLegacyChat(accountId)) {
      SoloLog.d('ChatHistoryService', 'migrating legacy chat for $accountId');
      return _migrateLegacyChat(accountId);
    }

    // 3. New user — empty list
    return [];
  }

  // ---------------------------------------------------------------------------
  // Session Messages
  // ---------------------------------------------------------------------------

  /// 保存指定会话的消息列表。
  Future<void> saveSessionMessages(
    String accountId,
    String sessionId,
    List<ChatMessageData> messages,
  ) async {
    if (!_vault.isVaultUnlocked()) {
      SoloLog.w('ChatHistoryService', 'saveSessionMessages skipped: vault locked');
      return;
    }

    // 过滤掉空消息
    final persistable = messages.where((m) => m.text.isNotEmpty).toList();

    final payload = {
      'version': 1,
      'savedAt': DateTime.now().toIso8601String(),
      'messages': persistable.map((m) => m.toJson()).toList(),
    };

    try {
      await _vault.saveProfileEncrypted(
        _sessionMessagesName(accountId, sessionId),
        jsonEncode(payload),
      );
      SoloLog.d('ChatHistoryService',
          'saved ${persistable.length} messages for session $sessionId');
    } on Exception catch (e) {
      SoloLog.w('ChatHistoryService', 'saveSessionMessages failed: $e');
    }
  }

  /// 加载指定会话的消息列表。
  Future<List<ChatMessageData>> loadSessionMessages(
    String accountId,
    String sessionId,
  ) async {
    if (!_vault.isVaultUnlocked()) {
      SoloLog.w('ChatHistoryService', 'loadSessionMessages skipped: vault locked');
      return [];
    }

    try {
      final jsonStr = await _vault.loadProfileDecrypted(
        _sessionMessagesName(accountId, sessionId),
      );
      if (jsonStr == null || jsonStr.isEmpty) {
        return [];
      }

      final map = jsonDecode(jsonStr) as Map<String, dynamic>;
      final messagesJson = map['messages'] as List<dynamic>?;
      if (messagesJson == null) return [];

      final messages = messagesJson
          .whereType<Map<String, dynamic>>()
          .map((json) => ChatMessageData.fromJson(json))
          .toList();

      SoloLog.d('ChatHistoryService',
          'loaded ${messages.length} messages for session $sessionId');
      return messages;
    } on Exception catch (e) {
      SoloLog.w('ChatHistoryService', 'loadSessionMessages failed: $e');
      return [];
    }
  }

  /// 删除指定会话的消息文件。
  Future<void> deleteSessionMessages(String accountId, String sessionId) async {
    if (!_vault.isVaultUnlocked()) return;
    try {
      await _vault.deleteProfile(_sessionMessagesName(accountId, sessionId));
      SoloLog.d('ChatHistoryService', 'deleted messages for session $sessionId');
    } on Exception catch (e) {
      SoloLog.w('ChatHistoryService', 'deleteSessionMessages failed: $e');
    }
  }

  // ---------------------------------------------------------------------------
  // Legacy Migration
  // ---------------------------------------------------------------------------

  /// 检测是否存在未迁移的旧格式聊天数据。
  Future<bool> hasLegacyChat(String accountId) async {
    if (!_vault.isVaultUnlocked()) return false;
    try {
      final jsonStr = await _vault.loadProfileDecrypted(_profileName(accountId));
      return jsonStr != null && jsonStr.isNotEmpty;
    } on Exception catch (_) {
      return false;
    }
  }

  /// 将旧格式数据迁移到新的多会话结构。
  ///
  /// 1. 读取旧消息
  /// 2. 创建 default 会话
  /// 3. 保存会话列表 + 消息
  /// 4. 将旧文件重命名为 backup，不删除
  Future<List<ChatSession>> _migrateLegacyChat(String accountId) async {
    try {
      // Read legacy messages
      final legacyMessages = await load(accountId);

      // Create default session
      final now = DateTime.now().millisecondsSinceEpoch;
      final defaultSession = ChatSession(
        id: _legacySessionId,
        title: '默认对话',
        createdAt: now,
        updatedAt: now,
        messageCount: legacyMessages.length,
      );

      // Save new structure
      await saveSessionList(accountId, [defaultSession]);
      await saveSessionMessages(accountId, _legacySessionId, legacyMessages);

      // Rename old file to backup (don't delete)
      await _renameLegacyToBackup(accountId);

      SoloLog.d('ChatHistoryService',
          'migrated ${legacyMessages.length} messages to default session for $accountId');
      return [defaultSession];
    } on Exception catch (e) {
      SoloLog.e('ChatHistoryService', 'legacy migration failed: $e');
      return [];
    }
  }

  Future<void> _renameLegacyToBackup(String accountId) async {
    try {
      // Note: RustVaultService may not support rename directly.
      // Workaround: read old data, write to backup, delete old.
      final oldData = await _vault.loadProfileDecrypted(_profileName(accountId));
      if (oldData != null) {
        await _vault.saveProfileEncrypted(_legacyBackupName(accountId), oldData);
        await _vault.deleteProfile(_profileName(accountId));
        SoloLog.d('ChatHistoryService',
            'renamed legacy chat to backup for $accountId');
      }
    } on Exception catch (e) {
      SoloLog.w('ChatHistoryService', 'rename legacy to backup failed: $e');
    }
  }
}
