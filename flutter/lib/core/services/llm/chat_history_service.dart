import 'dart:convert';

import 'package:solosoul_flutter/core/services/rust_vault_service.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';

// =============================================================================
// Chat History Service
// =============================================================================

/// 可序列化的聊天消息数据（与 [LlmChatMessage] 对应）。
class ChatMessageData {
  final String id;
  final String text;
  final bool isUser;

  const ChatMessageData({
    required this.id,
    required this.text,
    required this.isUser,
  });

  Map<String, dynamic> toJson() => {
        'id': id,
        'text': text,
        'isUser': isUser,
      };

  factory ChatMessageData.fromJson(Map<String, dynamic> json) => ChatMessageData(
        id: json['id'] as String,
        text: json['text'] as String,
        isUser: json['isUser'] as bool,
      );
}

/// 管理 AI 对话历史的加密持久化。
///
/// 使用 [RustVaultService] 的 profile 存储机制（独立于主 ProfileData），
/// profile name 格式为 `{accountId}_chat`，确保：
/// - 完全独立，不影响现有 ProfileData 结构
/// - 复用 SQLCipher 加密保护
/// - 可按需加载，不拖累主 profile 加载性能
class ChatHistoryService {
  static ChatHistoryService? _instance;
  static ChatHistoryService get instance => _instance ??= ChatHistoryService._();
  ChatHistoryService._();

  final RustVaultService _vault = RustVaultService.instance;

  static String _profileName(String accountId) => '${accountId}_chat';

  // ---------------------------------------------------------------------------
  // Save / Load
  // ---------------------------------------------------------------------------

  /// 保存聊天历史（加密存储到 Vault）。
  ///
  /// [accountId] 当前登录账号 ID。
  /// [messages] 消息列表。流式中（isStreaming=true）的消息会被过滤掉不保存。
  ///
  /// 若 [messages] 为空，保存空列表（覆盖删除旧数据）。
  Future<void> save(String accountId, List<ChatMessageData> messages) async {
    if (!_vault.isVaultUnlocked()) {
      SoloLog.w('ChatHistoryService', 'save skipped: vault locked');
      return;
    }

    // 过滤掉流式中的消息（不保存未完成的回复）
    final persistable = messages.where((m) => m.text.isNotEmpty).toList();

    final payload = {
      'version': 1,
      'savedAt': DateTime.now().toIso8601String(),
      'messages': persistable.map((m) => m.toJson()).toList(),
    };

    try {
      await _vault.saveProfileEncrypted(
        _profileName(accountId),
        jsonEncode(payload),
      );
      SoloLog.d('ChatHistoryService',
          'saved ${persistable.length} messages for $accountId');
    } on Exception catch (e) {
      SoloLog.w('ChatHistoryService', 'save failed: $e');
    }
  }

  /// 加载聊天历史（从 Vault 解密）。
  ///
  /// 若 Vault 未解锁、记录不存在或解析失败，返回空列表。
  Future<List<ChatMessageData>> load(String accountId) async {
    if (!_vault.isVaultUnlocked()) {
      SoloLog.w('ChatHistoryService', 'load skipped: vault locked');
      return [];
    }

    try {
      final jsonStr = await _vault.loadProfileDecrypted(_profileName(accountId));
      if (jsonStr == null || jsonStr.isEmpty) {
        SoloLog.d('ChatHistoryService', 'no history found for $accountId');
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
          'loaded ${messages.length} messages for $accountId');
      return messages;
    } on Exception catch (e) {
      SoloLog.w('ChatHistoryService', 'load failed: $e');
      return [];
    }
  }

  /// 删除指定账号的聊天历史。
  Future<void> delete(String accountId) async {
    if (!_vault.isVaultUnlocked()) return;
    try {
      await _vault.deleteProfile(_profileName(accountId));
      SoloLog.d('ChatHistoryService', 'deleted history for $accountId');
    } on Exception catch (e) {
      SoloLog.w('ChatHistoryService', 'delete failed: $e');
    }
  }
}
