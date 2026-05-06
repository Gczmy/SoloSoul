import 'dart:convert';

// =============================================================================
// LLM Per-Model Usage
// =============================================================================

/// Usage statistics for a single LLM model.
class LlmModelUsage {
  final String modelName;
  final String provider;
  final int usageCount;
  final int promptTokens;
  final int completionTokens;
  final DateTime? lastLoadTime;
  final DateTime? lastUsedTime;

  const LlmModelUsage({
    required this.modelName,
    required this.provider,
    this.usageCount = 0,
    this.promptTokens = 0,
    this.completionTokens = 0,
    this.lastLoadTime,
    this.lastUsedTime,
  });

  int get totalTokens => promptTokens + completionTokens;

  LlmModelUsage copyWith({
    String? modelName,
    String? provider,
    int? usageCount,
    int? promptTokens,
    int? completionTokens,
    DateTime? lastLoadTime,
    DateTime? lastUsedTime,
  }) {
    return LlmModelUsage(
      modelName: modelName ?? this.modelName,
      provider: provider ?? this.provider,
      usageCount: usageCount ?? this.usageCount,
      promptTokens: promptTokens ?? this.promptTokens,
      completionTokens: completionTokens ?? this.completionTokens,
      lastLoadTime: lastLoadTime ?? this.lastLoadTime,
      lastUsedTime: lastUsedTime ?? this.lastUsedTime,
    );
  }

  Map<String, dynamic> toJson() => {
        'modelName': modelName,
        'provider': provider,
        'usageCount': usageCount,
        'promptTokens': promptTokens,
        'completionTokens': completionTokens,
        'lastLoadTime': lastLoadTime?.toIso8601String(),
        'lastUsedTime': lastUsedTime?.toIso8601String(),
      };

  factory LlmModelUsage.fromJson(Map<String, dynamic> json) {
    DateTime? parseDt(String? raw) {
      if (raw == null || raw.isEmpty) return null;
      return DateTime.tryParse(raw);
    }

    return LlmModelUsage(
      modelName: json['modelName'] as String? ?? '',
      provider: json['provider'] as String? ?? '',
      usageCount: json['usageCount'] as int? ?? 0,
      promptTokens: json['promptTokens'] as int? ?? 0,
      completionTokens: json['completionTokens'] as int? ?? 0,
      lastLoadTime: parseDt(json['lastLoadTime'] as String?),
      lastUsedTime: parseDt(json['lastUsedTime'] as String?),
    );
  }
}

// =============================================================================
// LLM Daily Usage
// =============================================================================

/// Usage statistics for a single day.
class LlmDailyUsage {
  final DateTime date;
  final int totalTokens;
  final int usageCount;

  /// 各模型当日 Token 消耗。Key: "provider/modelName"
  final Map<String, int> perModelTokens;

  const LlmDailyUsage({
    required this.date,
    this.totalTokens = 0,
    this.usageCount = 0,
    this.perModelTokens = const {},
  });

  LlmDailyUsage copyWith({
    DateTime? date,
    int? totalTokens,
    int? usageCount,
    Map<String, int>? perModelTokens,
  }) {
    return LlmDailyUsage(
      date: date ?? this.date,
      totalTokens: totalTokens ?? this.totalTokens,
      usageCount: usageCount ?? this.usageCount,
      perModelTokens: perModelTokens ?? this.perModelTokens,
    );
  }

  Map<String, dynamic> toJson() => {
        'date': date.toIso8601String(),
        'totalTokens': totalTokens,
        'usageCount': usageCount,
        'perModelTokens': perModelTokens,
      };

  factory LlmDailyUsage.fromJson(Map<String, dynamic> json) {
    DateTime? parseDt(String? raw) {
      if (raw == null || raw.isEmpty) return null;
      return DateTime.tryParse(raw);
    }

    final perModelRaw = json['perModelTokens'] as Map<String, dynamic>?;
    final perModel = perModelRaw != null
        ? perModelRaw.map((k, v) => MapEntry(k, v as int))
        : const <String, int>{};

    return LlmDailyUsage(
      date: parseDt(json['date'] as String?) ?? DateTime.now(),
      totalTokens: json['totalTokens'] as int? ?? 0,
      usageCount: json['usageCount'] as int? ?? 0,
      perModelTokens: perModel,
    );
  }
}

// =============================================================================
// LLM Usage Stats (persisted per account)
// =============================================================================

/// Persisted LLM usage statistics per account.
///
/// Stored as an encrypted JSON blob via [RustVaultService].
class LlmUsageStats {
  // -------------------------------------------------------------------------
  // Account-level accumulated stats (persisted)
  // -------------------------------------------------------------------------

  final int usageCount;
  final int totalPromptTokens;
  final int totalCompletionTokens;
  final DateTime? lastLoadTime;
  final DateTime? lastUsedTime;
  final List<LlmModelUsage> perModelStats;
  final List<LlmDailyUsage> dailyStats;

  // -------------------------------------------------------------------------
  // Session-level stats (memory only, not persisted)
  // -------------------------------------------------------------------------

  final int sessionUsageCount;
  final int sessionPromptTokens;
  final int sessionCompletionTokens;

  const LlmUsageStats({
    this.usageCount = 0,
    this.totalPromptTokens = 0,
    this.totalCompletionTokens = 0,
    this.lastLoadTime,
    this.lastUsedTime,
    this.perModelStats = const [],
    this.dailyStats = const [],
    this.sessionUsageCount = 0,
    this.sessionPromptTokens = 0,
    this.sessionCompletionTokens = 0,
  });

  int get totalTokensUsed => totalPromptTokens + totalCompletionTokens;
  int get sessionTotalTokens => sessionPromptTokens + sessionCompletionTokens;

  LlmUsageStats copyWith({
    int? usageCount,
    int? totalPromptTokens,
    int? totalCompletionTokens,
    DateTime? lastLoadTime,
    DateTime? lastUsedTime,
    List<LlmModelUsage>? perModelStats,
    List<LlmDailyUsage>? dailyStats,
    int? sessionUsageCount,
    int? sessionPromptTokens,
    int? sessionCompletionTokens,
  }) {
    return LlmUsageStats(
      usageCount: usageCount ?? this.usageCount,
      totalPromptTokens: totalPromptTokens ?? this.totalPromptTokens,
      totalCompletionTokens: totalCompletionTokens ?? this.totalCompletionTokens,
      lastLoadTime: lastLoadTime ?? this.lastLoadTime,
      lastUsedTime: lastUsedTime ?? this.lastUsedTime,
      perModelStats: perModelStats ?? this.perModelStats,
      dailyStats: dailyStats ?? this.dailyStats,
      sessionUsageCount: sessionUsageCount ?? this.sessionUsageCount,
      sessionPromptTokens: sessionPromptTokens ?? this.sessionPromptTokens,
      sessionCompletionTokens: sessionCompletionTokens ?? this.sessionCompletionTokens,
    );
  }

  Map<String, dynamic> toJson() => {
        'usageCount': usageCount,
        'totalPromptTokens': totalPromptTokens,
        'totalCompletionTokens': totalCompletionTokens,
        'lastLoadTime': lastLoadTime?.toIso8601String(),
        'lastUsedTime': lastUsedTime?.toIso8601String(),
        'perModelStats': perModelStats.map((m) => m.toJson()).toList(),
        'dailyStats': dailyStats.map((d) => d.toJson()).toList(),
      };

  factory LlmUsageStats.fromJson(Map<String, dynamic> json) {
    DateTime? parseDt(String? raw) {
      if (raw == null || raw.isEmpty) return null;
      return DateTime.tryParse(raw);
    }

    final perModelList = (json['perModelStats'] as List<dynamic>? ?? [])
        .map((e) => LlmModelUsage.fromJson(e as Map<String, dynamic>))
        .toList();

    final dailyList = (json['dailyStats'] as List<dynamic>? ?? [])
        .map((e) => LlmDailyUsage.fromJson(e as Map<String, dynamic>))
        .toList();

    return LlmUsageStats(
      usageCount: json['usageCount'] as int? ?? 0,
      totalPromptTokens: json['totalPromptTokens'] as int? ?? 0,
      totalCompletionTokens: json['totalCompletionTokens'] as int? ?? 0,
      lastLoadTime: parseDt(json['lastLoadTime'] as String?),
      lastUsedTime: parseDt(json['lastUsedTime'] as String?),
      perModelStats: perModelList,
      dailyStats: dailyList,
    );
  }

  factory LlmUsageStats.fromJsonString(String jsonStr) {
    final map = jsonDecode(jsonStr) as Map<String, dynamic>;
    return LlmUsageStats.fromJson(map);
  }

  String toJsonString() => jsonEncode(toJson());
}
