import 'dart:convert';

// =============================================================================
// LLM Usage Stats Model
// =============================================================================

/// Persisted LLM usage statistics per account.
///
/// Stored as an encrypted JSON blob via [RustVaultService].
class LlmUsageStats {
  final int usageCount;
  final int totalTokensUsed;
  final DateTime? lastLoadTime;
  final DateTime? lastUsedTime;

  const LlmUsageStats({
    this.usageCount = 0,
    this.totalTokensUsed = 0,
    this.lastLoadTime,
    this.lastUsedTime,
  });

  LlmUsageStats copyWith({
    int? usageCount,
    int? totalTokensUsed,
    DateTime? lastLoadTime,
    DateTime? lastUsedTime,
  }) {
    return LlmUsageStats(
      usageCount: usageCount ?? this.usageCount,
      totalTokensUsed: totalTokensUsed ?? this.totalTokensUsed,
      lastLoadTime: lastLoadTime ?? this.lastLoadTime,
      lastUsedTime: lastUsedTime ?? this.lastUsedTime,
    );
  }

  Map<String, dynamic> toJson() => {
        'usageCount': usageCount,
        'totalTokensUsed': totalTokensUsed,
        'lastLoadTime': lastLoadTime?.toIso8601String(),
        'lastUsedTime': lastUsedTime?.toIso8601String(),
      };

  factory LlmUsageStats.fromJson(Map<String, dynamic> json) {
    DateTime? parseDt(String? raw) {
      if (raw == null || raw.isEmpty) return null;
      return DateTime.tryParse(raw);
    }

    return LlmUsageStats(
      usageCount: json['usageCount'] as int? ?? 0,
      totalTokensUsed: json['totalTokensUsed'] as int? ?? 0,
      lastLoadTime: parseDt(json['lastLoadTime'] as String?),
      lastUsedTime: parseDt(json['lastUsedTime'] as String?),
    );
  }

  factory LlmUsageStats.fromJsonString(String jsonStr) {
    final map = jsonDecode(jsonStr) as Map<String, dynamic>;
    return LlmUsageStats.fromJson(map);
  }

  String toJsonString() => jsonEncode(toJson());
}
