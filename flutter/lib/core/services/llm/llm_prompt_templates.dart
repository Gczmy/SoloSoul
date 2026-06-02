// =============================================================================
// LLM Prompt Templates
// =============================================================================

/// Centralized prompt templates for all LLM-assisted business features.
///
/// All prompts follow these conventions:
/// - Structured JSON output (when applicable)
/// - Low temperature (0.3) for deterministic results
/// - Conservative max_tokens to reduce latency
/// - Chinese primary language, English technical terms preserved
class LlmPromptTemplates {
  LlmPromptTemplates._();

  // ---------------------------------------------------------------------------
  // P3: Field Mapping (Local Import AI Assist)
  // ---------------------------------------------------------------------------

  /// Prompt for AI-assisted field mapping during local file import.
  ///
  /// [fileName] — name of the file being imported.
  /// [contentPreview] — first N characters of file content.
  /// [schemaJson] — JSON description of available SoloSoul schema fields.
  static String fieldMapping({
    required String fileName,
    required String contentPreview,
    required String schemaJson,
  }) {
    return '''你是一个数据结构化专家。用户正在将本地文件导入个人知识库，请帮助将文件内容映射到标准字段。

文件名称：$fileName

文件内容预览：
```
$contentPreview
```

可用的目标字段（JSON Schema）：
```json
$schemaJson
```

请分析文件内容，建议最合适的字段映射关系。返回严格的 JSON 格式：
{
  "mappings": [
    {
      "source_field": "文件中的字段名或内容片段",
      "target_property_id": "目标字段的 propertyId",
      "confidence": 0.85,
      "reason": "简要说明映射理由"
    }
  ],
  "unmapped": ["未能映射的内容片段"],
  "suggested_object_type": "建议的 UnifiedObject 类型"
}

注意：
- 只返回 JSON，不要任何解释性文字
- confidence 范围 0.0 ~ 1.0
- 如果无法确定映射，target_property_id 留空，confidence 设为 0.0''';}

  // ---------------------------------------------------------------------------
  // P3: Text Polish
  // ---------------------------------------------------------------------------

  /// Prompt for polishing user-written text.
  static String textPolish(String originalText) {
    return '''请润色以下文本，使其表达更专业、流畅。保持原文的核心意思不变，不要添加原文没有的信息。

原文：
$originalText

润色后的文本：''';
  }

  // ---------------------------------------------------------------------------
  // P3: Translation
  // ---------------------------------------------------------------------------

  /// Prompt for translating text.
  static String translate(String originalText, String targetLanguage) {
    return '''请将以下文本翻译成$targetLanguage。保持原文的语气和风格，技术术语优先使用业界通用译法。

原文：
$originalText

翻译：''';
  }

  // ---------------------------------------------------------------------------
  // P3: Summarization
  // ---------------------------------------------------------------------------

  /// Prompt for summarizing long text.
  static String summarize(String longText, {int maxLength = 200}) {
    return '''请对以下文本生成摘要，字数控制在 $maxLength 字以内。保留关键事实和结论，去除细节和背景描述。

文本：
$longText

摘要：''';
  }

  // ---------------------------------------------------------------------------
  // P3: Application Reason Generation (for sensitive data access requests)
  // ---------------------------------------------------------------------------

  /// Prompt for generating a formal application reason text.
  static String applicationReason({
    required String requester,
    required String dataType,
    required String purpose,
  }) {
    return '''请根据以下信息，生成一段正式的申请理由文本。语气正式、简洁、客观，不超过 150 字。

- 申请人/实体：$requester
- 申请访问的数据类型：$dataType
- 使用目的：$purpose

申请理由：''';
  }

  // ---------------------------------------------------------------------------
  // P3: Schema-Aware Extraction
  // ---------------------------------------------------------------------------

  /// Prompt for extracting structured fields from unstructured text.
  static String structuredExtraction({
    required String sourceText,
    required String fieldSchemaJson,
  }) {
    return '''请从以下非结构化文本中提取指定字段。严格按照字段定义提取，如果某个字段在文本中不存在，value 设为空字符串。

待提取文本：
```
$sourceText
```

字段定义（JSON）：
```json
$fieldSchemaJson
```

返回严格的 JSON 格式：
{
  "extracted_fields": [
    {
      "property_id": "字段ID",
      "value": "提取的值",
      "confidence": 0.95,
      "sensitivity": "public"
    }
  ],
  "missing_fields": ["未找到的字段ID"],
  "overall_confidence": 0.90
}

注意：
- sensitivity 只能是 public, internal, sensitive, critical 之一
- 只返回 JSON，不要其他内容
- confidence 范围 0.0 ~ 1.0''';
  }

  // ---------------------------------------------------------------------------
  // P3: Validation / Sanity Check
  // ---------------------------------------------------------------------------

  /// Prompt for validating extracted data against schema constraints.
  static String validateExtraction({
    required String extractedJson,
    required String constraintsJson,
  }) {
    return '''请检查以下提取结果是否符合字段约束条件。返回 JSON 格式：

提取结果：
```json
$extractedJson
```

约束条件：
```json
$constraintsJson
```

返回格式：
{
  "is_valid": true,
  "warnings": [
    {
      "field_id": "字段ID",
      "message": "警告描述"
    }
  ]
}''';
  }

  // ---------------------------------------------------------------------------
  // P2: AI Chat System Prompt (Context-Aware Assistant)
  // ---------------------------------------------------------------------------

  /// System prompt for the AI chat assistant.
  ///
  /// Injects software info, user public profile, preferences, and usage stats
  /// so the AI can provide contextually aware responses.
  static String chatSystemPrompt({
    required String appVersion,
    required String platform,
    required String language,
    required Map<String, List<Map<String, String>>> userPublicInfo,
    required Map<String, dynamic> preferences,
    required List<String> installedPlugins,
    required Map<String, dynamic> usageStats,
  }) {
    final buffer = StringBuffer();

    // AI Identity
    buffer.writeln('你是 SoloSoul（独灵）的智能助手。你正在帮助用户操作一款本地优先、隐私优先的个人数字孪生与通用身份管理应用。');
    buffer.writeln();

    // Software Info
    buffer.writeln('## 软件信息');
    buffer.writeln('- 应用名称：SoloSoul（独灵）');
    buffer.writeln('- 当前版本：$appVersion');
    buffer.writeln('- 运行平台：$platform');
    buffer.writeln('- 界面语言：$language');
    buffer.writeln();

    // User Public Profile
    if (userPublicInfo.isNotEmpty) {
      buffer.writeln('## 用户公开档案（仅包含用户标记为"公开"的非敏感信息）');
      for (final entry in userPublicInfo.entries) {
        final typeName = entry.key;
        final objects = entry.value;
        if (objects.isEmpty) continue;
        buffer.writeln('- **$typeName**：');
        for (final obj in objects) {
          final props = obj.entries
              .where((e) => e.value.isNotEmpty)
              .map((e) => '${e.key}: ${e.value}')
              .join(', ');
          if (props.isNotEmpty) {
            buffer.writeln('  - $props');
          }
        }
      }
      buffer.writeln();
    }

    // Preferences
    if (preferences.isNotEmpty) {
      buffer.writeln('## 用户偏好设置');
      for (final entry in preferences.entries) {
        buffer.writeln('- ${entry.key}：${entry.value}');
      }
      buffer.writeln();
    }

    // Installed Plugins
    if (installedPlugins.isNotEmpty) {
      buffer.writeln('## 已安装插件');
      buffer.writeln('- ${installedPlugins.join('、')}');
      buffer.writeln();
    } else {
      buffer.writeln('## 已安装插件');
      buffer.writeln('- 暂无');
      buffer.writeln();
    }

    // Usage Stats
    buffer.writeln('## AI 使用统计');
    final currentModel = usageStats['currentModel'] as String? ?? '未知';
    final currentProvider = usageStats['currentProvider'] as String? ?? '未知';
    final sessionCalls = usageStats['sessionCalls'] as int? ?? 0;
    final sessionTokens = usageStats['sessionTokens'] as int? ?? 0;
    final accountCalls = usageStats['accountCalls'] as int? ?? 0;
    final accountTokens = usageStats['accountTokens'] as int? ?? 0;

    buffer.writeln('- 当前模型：$currentModel（$currentProvider）');
    buffer.writeln('- 本次会话：$sessionCalls 次调用，$sessionTokens tokens');
    buffer.writeln('- 账户累计：$accountCalls 次调用，$accountTokens tokens');
    buffer.writeln();

    // Behavior Guidelines
    buffer.writeln('## 行为指引');
    buffer.writeln('1. 你可以根据用户的公开档案和偏好，提供个性化的回答和建议。');
    buffer.writeln('2. 当用户询问软件功能、使用方法或遇到问题时，你可以基于上述软件信息给出准确的指导。');
    buffer.writeln('3. 如果用户询问的信息不在上述公开档案中（如身份证号、银行账号等敏感信息），请明确告知用户你无法访问此类敏感数据，并建议用户自行查看对应页面。');
    buffer.writeln('4. 以上信息可能被截断，如需更多详情请明确询问。');
    buffer.writeln('5. **请使用用户提问的语言来回答。如果用户用中文提问，用中文回答；如果用户用英文提问，用英文回答。**');
    buffer.writeln('6. **插件（Plugin）与对象（Object）是两个不同的概念**：插件是 Wasm 扩展模块（如 OCR 扫描器），通过插件市场安装；对象是保险库中的个人数据记录（如护照、银行账户）。请勿将二者混淆。');
    buffer.writeln('7. 当用户询问已安装插件相关问题时，你可以参考"已安装插件"区域的信息。如果用户未安装任何插件，可以引导其前往插件市场浏览。');

    return buffer.toString();
  }
}
