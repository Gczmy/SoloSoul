import 'dart:convert';

// =============================================================================
// LLM Field Mapping Parser
// =============================================================================

/// AI 字段映射建议。
class LlmFieldSuggestion {
  /// 文件中的源字段名或内容片段。
  final String sourceField;

  /// 目标 propertyId。
  final String? targetPropertyId;

  /// 置信度 0.0 ~ 1.0。
  final double confidence;

  /// 映射理由。
  final String reason;

  /// 来源：'local' 或 'cloud'。
  final String source;

  const LlmFieldSuggestion({
    required this.sourceField,
    this.targetPropertyId,
    required this.confidence,
    required this.reason,
    required this.source,
  });
}

/// LLM 字段映射解析结果。
class LlmFieldMappingResult {
  final List<LlmFieldSuggestion> mappings;
  final List<String> unmapped;
  final String? suggestedObjectType;

  const LlmFieldMappingResult({
    required this.mappings,
    required this.unmapped,
    this.suggestedObjectType,
  });
}

// =============================================================================
// Parser
// =============================================================================

class LlmFieldMappingParser {
  LlmFieldMappingParser._();

  /// 解析 LLM 返回的 JSON 字符串。
  ///
  /// 期望格式：
  /// ```json
  /// {
  ///   "mappings": [
  ///     {"source_field": "...", "target_property_id": "...", "confidence": 0.85, "reason": "..."}
  ///   ],
  ///   "unmapped": ["..."],
  ///   "suggested_object_type": "..."
  /// }
  /// ```
  static LlmFieldMappingResult parse(String jsonText, {String source = 'local'}) {
    final cleaned = _extractJson(jsonText);
    final json = jsonDecode(cleaned) as Map<String, dynamic>;

    final mappingsList = (json['mappings'] as List<dynamic>? ?? []);
    final mappings = <LlmFieldSuggestion>[];
    for (final m in mappingsList) {
      if (m is! Map<String, dynamic>) continue;
      mappings.add(LlmFieldSuggestion(
        sourceField: (m['source_field'] ?? '').toString(),
        targetPropertyId: m['target_property_id']?.toString(),
        confidence: (m['confidence'] as num?)?.toDouble() ?? 0.0,
        reason: (m['reason'] ?? '').toString(),
        source: source,
      ));
    }

    final unmappedList = (json['unmapped'] as List<dynamic>? ?? [])
        .map((e) => e.toString())
        .toList();

    return LlmFieldMappingResult(
      mappings: mappings,
      unmapped: unmappedList,
      suggestedObjectType: json['suggested_object_type']?.toString(),
    );
  }

  /// 从可能包含 markdown 代码块的文本中提取 JSON。
  static String _extractJson(String text) {
    final codeBlockRe = RegExp(r'```(?:json)?\s*([\s\S]*?)\s*```');
    final match = codeBlockRe.firstMatch(text);
    if (match != null) {
      return match.group(1)!.trim();
    }
    return text.trim();
  }
}
