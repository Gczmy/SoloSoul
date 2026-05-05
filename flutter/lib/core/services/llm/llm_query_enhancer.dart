import 'dart:convert';

import 'package:solosoul_flutter/core/services/llm/llm_service.dart';

// =============================================================================
// LLM Query Enhancer
// =============================================================================

/// Enhances user queries using LLM for better retrieval quality.
///
/// Implements a three-tier fallback strategy:
/// 1. LLM-based semantic expansion (JSON output)
/// 2. Rule-based synonym expansion
/// 3. Return original query unchanged
///
/// Inspired by xiaoyaosearch's query enhancement pipeline.
class LlmQueryEnhancer {
  final LlmService llm;

  const LlmQueryEnhancer({required this.llm});

  /// Enhance a search query.
  ///
  /// Returns [EnhancementResult] containing expanded and rewritten variants.
  /// Never throws — falls back to original query on any error.
  Future<EnhancementResult> enhance(String query) async {
    // Tier 0: Skip enhancement for unsuitable queries
    if (!shouldEnhance(query)) {
      return EnhancementResult.original(query);
    }

    // Tier 1: LLM-based enhancement
    try {
      final prompt = buildPrompt(query);
      final response = await llm.infer(prompt, maxTokens: 150);
      final parsed = parseResponse(response, query);
      if (parsed.enhanced) return parsed;
    } on Exception catch (_) {
      // Log and fall through to rule-based
    }

    // Tier 2: Rule-based synonym expansion
    return ruleBasedEnhance(query);
  }

  // ---------------------------------------------------------------------------
  // Should enhance?
  // ---------------------------------------------------------------------------

  static bool shouldEnhance(String query) {
    query = query.trim();

    // Too short
    if (query.length <= 2) return false;

    // File path or name
    if (query.contains('.') || query.contains('/') || query.contains('\\') || query.contains(':')) {
      return false;
    }

    // Quoted exact query
    if (query.startsWith('"') && query.endsWith('"')) return false;

    // Boolean operators
    final lower = query.toLowerCase();
    if (lower.contains(' and ') || lower.contains(' or ') || lower.contains(' not ')) {
      return false;
    }
    if (lower.contains('+') || lower.contains(' -')) return false;

    return true;
  }

  // ---------------------------------------------------------------------------
  // Prompt engineering
  // ---------------------------------------------------------------------------

  static String buildPrompt(String query) {
    return '''你是一个搜索查询优化专家。请对以下中文查询进行优化：

原始查询：$query

请提供JSON格式响应：
{
    "expanded_query": "扩展查询（添加3-5个同义词，用空格分隔）",
    "rewritten_query": "重写查询（更准确的表达）"
}

示例：
输入：怎么用Python
输出：{"expanded_query": "怎么用Python Python使用方法 Python教程 Python入门 Python操作指南", "rewritten_query": "Python使用方法教程"}

请只返回JSON，不要其他内容。''';
  }

  // ---------------------------------------------------------------------------
  // Response parsing
  // ---------------------------------------------------------------------------

  static EnhancementResult parseResponse(String content, String original) {
    try {
      // Extract JSON from potentially markdown-wrapped response
      final jsonStr = _extractJson(content);
      if (jsonStr == null) return EnhancementResult.original(original);

      final json = jsonDecode(jsonStr) as Map<String, dynamic>;
      final expanded = json['expanded_query'] as String?;
      final rewritten = json['rewritten_query'] as String?;

      if (expanded == null || rewritten == null) {
        return EnhancementResult.original(original);
      }

      return EnhancementResult(
        original: original,
        expanded: expanded.trim(),
        rewritten: rewritten.trim(),
        enhanced: true,
      );
    } on FormatException catch (_) {
      return EnhancementResult.original(original);
    }
  }

  static String? _extractJson(String content) {
    // Try to find JSON block between curly braces
    final start = content.indexOf('{');
    final end = content.lastIndexOf('}');
    if (start == -1 || end == -1 || end <= start) return null;
    return content.substring(start, end + 1);
  }

  // ---------------------------------------------------------------------------
  // Rule-based fallback
  // ---------------------------------------------------------------------------

  static final Map<String, List<String>> _synonymMap = {
    '怎么用': ['使用方法', '操作指南', '教程', '使用步骤'],
    '如何': ['怎么', '怎样', '如何操作', '方法'],
    '什么是': ['定义', '概念', '含义', '解释'],
    '为什么': ['原因', '理由', '原理', '成因'],
    '下载': ['下载地址', '获取', '安装包'],
    '安装': ['安装教程', '配置', '部署', '搭建'],
    '打不开': ['无法启动', '启动失败', '运行错误'],
    '错误': ['问题', '异常', 'bug', '故障'],
    '教程': ['指南', '手册', '文档', '学习资料'],
    '配置': ['设置', '参数', '选项', '自定义'],
    '优化': ['改进', '提升', '调优', '增强'],
    '删除': ['移除', '清除', '清理', '卸载'],
  };

  static EnhancementResult ruleBasedEnhance(String query) {
    final lower = query.toLowerCase();
    final expandedTerms = <String>[];

    for (final entry in _synonymMap.entries) {
      if (lower.contains(entry.key)) {
        expandedTerms.addAll(entry.value);
        break; // Only expand the first match
      }
    }

    if (expandedTerms.isEmpty) {
      return EnhancementResult.original(query);
    }

    final expanded = '$query ${expandedTerms.take(3).join(' ')}';
    return EnhancementResult(
      original: query,
      expanded: expanded,
      rewritten: query,
      enhanced: true,
    );
  }
}

// =============================================================================
// Enhancement Result
// =============================================================================

class EnhancementResult {
  final String original;
  final String expanded;
  final String rewritten;
  final bool enhanced;

  const EnhancementResult({
    required this.original,
    required this.expanded,
    required this.rewritten,
    required this.enhanced,
  });

  factory EnhancementResult.original(String query) => EnhancementResult(
        original: query,
        expanded: query,
        rewritten: query,
        enhanced: false,
      );
}
