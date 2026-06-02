/// Sentinel value for the temporary "New Chat" state (not persisted).
const String kNewChatSessionId = '__new__';

/// Returns true if [id] is the temporary new-chat sentinel.
bool isNewChatSessionId(String? id) => id == kNewChatSessionId;

// =============================================================================
// Chat Session Model
// =============================================================================

/// Represents a single AI chat conversation session.
///
/// Each session has its own message history, stored separately in the Vault
/// under profile name `{accountId}_chat_{id}`.
class ChatSession {
  final String id;
  final String title;
  final int createdAt;
  final int updatedAt;
  final int messageCount;
  final bool isDeleted;
  final int? deletedAt;

  const ChatSession({
    required this.id,
    required this.title,
    required this.createdAt,
    required this.updatedAt,
    this.messageCount = 0,
    this.isDeleted = false,
    this.deletedAt,
  });

  Map<String, dynamic> toJson() => {
        'id': id,
        'title': title,
        'createdAt': createdAt,
        'updatedAt': updatedAt,
        'messageCount': messageCount,
        'isDeleted': isDeleted,
        if (deletedAt != null) 'deletedAt': deletedAt,
      };

  factory ChatSession.fromJson(Map<String, dynamic> json) => ChatSession(
        id: json['id'] as String,
        title: json['title'] as String? ?? '',
        createdAt: json['createdAt'] as int? ?? 0,
        updatedAt: json['updatedAt'] as int? ?? 0,
        messageCount: json['messageCount'] as int? ?? 0,
        isDeleted: json['isDeleted'] as bool? ?? false,
        deletedAt: json['deletedAt'] as int?,
      );

  ChatSession copyWith({
    String? id,
    String? title,
    int? createdAt,
    int? updatedAt,
    int? messageCount,
    bool? isDeleted,
    int? deletedAt,
    bool clearDeletedAt = false,
  }) {
    return ChatSession(
      id: id ?? this.id,
      title: title ?? this.title,
      createdAt: createdAt ?? this.createdAt,
      updatedAt: updatedAt ?? this.updatedAt,
      messageCount: messageCount ?? this.messageCount,
      isDeleted: isDeleted ?? this.isDeleted,
      deletedAt: clearDeletedAt ? null : (deletedAt ?? this.deletedAt),
    );
  }

  @override
  String toString() => 'ChatSession($id, $title, msgs=$messageCount${isDeleted ? ",deleted" : ""})';
}
