import 'dart:convert';

import 'package:flutter/services.dart';

import 'package:solosoul_flutter/core/utils/solo_log.dart';

// =============================================================================
// User Guide Service
// =============================================================================

/// 管理软件内部功能指南文档的索引、检索与加载。
///
/// - 启动时从 `assets/docs/guides/index.json` 加载轻量索引
/// - 用户向 AI 提问时，通过关键词匹配找到最相关的指南
/// - 用户自行阅读时，通过 `showLegalDocumentSheet()` 渲染 Markdown
///
/// **按需检索原则**：指南内容仅在用户提问匹配时才注入当前轮次对话，
/// 不作为强制提示词常驻。
class UserGuideService {
  UserGuideService._();

  static UserGuideService? _instance;
  static UserGuideService get instance => _instance ??= UserGuideService._();

  List<GuideIndexEntry>? _index;
  final Map<String, String> _contentCache = {};

  static const String _indexAssetPath = 'assets/docs/guides/index.json';
  static const int _maxDocChars = 800;
  static const int _scoreThreshold = 2;

  // ---------------------------------------------------------------------------
  // Stop words (Chinese + English)
  // ---------------------------------------------------------------------------

  static final Set<String> _stopWords = {
    '的', '了', '在', '是', '我', '有', '和', '就', '不', '人', '都', '一', '一个',
    '上', '也', '很', '到', '说', '要', '去', '你', '会', '着', '没有', '看', '好',
    '自己', '这', '那', '怎么', '如何', '什么', '吗', '呢', '吧', '啊', '可以',
    '能', '不能', '请', '谢谢', '帮忙', '一下', '请问',
    'the', 'a', 'an', 'is', 'are', 'was', 'were', 'be', 'been', 'being',
    'have', 'has', 'had', 'do', 'does', 'did', 'will', 'would', 'could',
    'should', 'may', 'might', 'must', 'can', 'shall', 'to', 'of', 'in',
    'for', 'on', 'with', 'at', 'by', 'from', 'as', 'into', 'through',
    'during', 'before', 'after', 'above', 'below', 'between', 'under',
    'and', 'but', 'or', 'yet', 'so', 'if', 'because', 'although',
    'how', 'what', 'when', 'where', 'why', 'who', 'which', 'this', 'that',
  };

  // ---------------------------------------------------------------------------
  // Index Loading
  // ---------------------------------------------------------------------------

  /// 从 asset 加载指南索引。建议在 `main()` 或 Vault 解锁后调用。
  Future<void> loadIndex() async {
    try {
      final jsonStr = await rootBundle.loadString(_indexAssetPath);
      final map = jsonDecode(jsonStr) as Map<String, dynamic>;
      final guidesJson = map['guides'] as List<dynamic>?;
      if (guidesJson == null) {
        SoloLog.w('UserGuideService', 'index.json missing "guides" field');
        return;
      }
      _index = guidesJson
          .whereType<Map<String, dynamic>>()
          .map((json) => GuideIndexEntry.fromJson(json))
          .toList();
      SoloLog.d('UserGuideService', 'loaded ${_index!.length} guide entries');
    } on Exception catch (e) {
      SoloLog.w('UserGuideService', 'Failed to load index: $e');
      _index = [];
    }
  }

  // ---------------------------------------------------------------------------
  // Retrieval
  // ---------------------------------------------------------------------------

  /// 根据用户查询返回最相关的指南内容。
  ///
  /// 匹配规则：
  /// - 对用户 query 分词（空格分割），过滤停用词
  /// - 每个指南得分 = 关键词命中次数 + 标题命中权重(+3)
  /// - 只返回得分最高的 **1 篇**（控制 token）
  /// - 得分低于 `_scoreThreshold` 时返回空（避免误触发）
  ///
  /// 返回的每篇内容已截断至 `_maxDocChars`。
  Future<List<GuideContent>> findRelevantGuides(
    String query,
    String language,
  ) async {
    if (_index == null || _index!.isEmpty) return [];

    final queryWords = _tokenize(query);
    if (queryWords.isEmpty) return [];

    GuideIndexEntry? bestMatch;
    var bestScore = 0;

    for (final guide in _index!) {
      var score = 0;

      // 关键词匹配
      for (final kw in guide.keywords) {
        final kwLower = kw.toLowerCase();
        if (queryWords.any((w) => w.contains(kwLower) || kwLower.contains(w))) {
          score++;
        }
      }

      // 标题匹配权重更高
      final titleLower = guide.title.toLowerCase();
      final titleEnLower = guide.titleEn.toLowerCase();
      if (queryWords.any((w) => titleLower.contains(w) || titleEnLower.contains(w))) {
        score += 3;
      }

      if (score > bestScore) {
        bestScore = score;
        bestMatch = guide;
      }
    }

    if (bestMatch == null || bestScore < _scoreThreshold) {
      SoloLog.d('UserGuideService',
          'No guide matched for query="$query" (bestScore=$bestScore)');
      return [];
    }

    final assetPath = _resolveAssetPath(bestMatch, language);
    final content = await loadGuideContent(assetPath);
    if (content == null || content.isEmpty) return [];

    final trimmed = _trimContent(content, _maxDocChars);
    SoloLog.d('UserGuideService',
        'Matched guide="${bestMatch.id}" score=$bestScore chars=${trimmed.length}');

    return [GuideContent(id: bestMatch.id, title: bestMatch.title, content: trimmed)];
  }

  // ---------------------------------------------------------------------------
  // Content Loading
  // ---------------------------------------------------------------------------

  /// 加载单篇指南内容，带缓存。
  ///
  /// 多语言 fallback：请求语言文件不存在时，回退到英文版。
  Future<String?> loadGuideContent(String assetPath) async {
    if (_contentCache.containsKey(assetPath)) {
      return _contentCache[assetPath];
    }

    try {
      final content = await rootBundle.loadString(assetPath);
      _contentCache[assetPath] = content;
      return content;
    } on Exception catch (e) {
      SoloLog.w('UserGuideService', 'Failed to load $assetPath: $e');
      return null;
    }
  }

  /// 获取指南列表（用于设置页展示）。
  List<GuideIndexEntry> get guideList {
    return _index != null ? List.unmodifiable(_index!) : const [];
  }

  void clearCache() {
    _contentCache.clear();
    SoloLog.d('UserGuideService', 'content cache cleared');
  }

  // ---------------------------------------------------------------------------
  // Helpers
  // ---------------------------------------------------------------------------

  String _resolveAssetPath(GuideIndexEntry guide, String language) {
    final files = guide.files;
    if (files.containsKey(language)) {
      return files[language]!;
    }
    // Fallback to English
    if (files.containsKey('en')) {
      return files['en']!;
    }
    // Last resort: first available
    return files.values.first;
  }

  List<String> _tokenize(String query) {
    return query
        .toLowerCase()
        .split(RegExp(r'[\s\p{P}]+', unicode: true))
        .where((w) => w.length >= 2 && !_stopWords.contains(w))
        .toList();
  }

  String _trimContent(String content, int maxChars) {
    if (content.length <= maxChars) return content;
    // Try to cut at a paragraph boundary
    final cutoff = content.lastIndexOf('\n\n', maxChars);
    if (cutoff > maxChars * 0.5) {
      return '${content.substring(0, cutoff)}\n\n（内容已截断）';
    }
    return '${content.substring(0, maxChars)}…（已截断）';
  }
}

// =============================================================================
// Data Models
// =============================================================================

class GuideIndexEntry {
  final String id;
  final String title;
  final String titleEn;
  final List<String> keywords;
  final Map<String, String> files;

  const GuideIndexEntry({
    required this.id,
    required this.title,
    required this.titleEn,
    required this.keywords,
    required this.files,
  });

  factory GuideIndexEntry.fromJson(Map<String, dynamic> json) {
    return GuideIndexEntry(
      id: json['id'] as String,
      title: json['title'] as String,
      titleEn: json['titleEn'] as String,
      keywords: (json['keywords'] as List<dynamic>).cast<String>(),
      files: (json['files'] as Map<String, dynamic>).cast<String, String>(),
    );
  }
}

class GuideContent {
  final String id;
  final String title;
  final String content;

  const GuideContent({
    required this.id,
    required this.title,
    required this.content,
  });
}
