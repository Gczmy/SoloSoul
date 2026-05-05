import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/llm/llm_prompt_templates.dart';

void main() {
  group('LlmPromptTemplates', () {
    test('fieldMapping includes filename and schema', () {
      final prompt = LlmPromptTemplates.fieldMapping(
        fileName: 'resume.pdf',
        contentPreview: 'Name: John Doe\nEmail: john@example.com',
        schemaJson: '{"fields": [{"id": "name", "type": "string"}]}',
      );
      expect(prompt, contains('resume.pdf'));
      expect(prompt, contains('John Doe'));
      expect(prompt, contains('fields'));
      expect(prompt, contains('JSON'));
    });

    test('textPolish preserves original text in prompt', () {
      const original = 'This is bad text.';
      final prompt = LlmPromptTemplates.textPolish(original);
      expect(prompt, contains(original));
      expect(prompt, contains('润色'));
    });

    test('translate includes target language', () {
      const text = 'Hello world';
      final prompt = LlmPromptTemplates.translate(text, '中文');
      expect(prompt, contains(text));
      expect(prompt, contains('中文'));
      expect(prompt, contains('翻译'));
    });

    test('summarize respects maxLength', () {
      final longText = 'A' * 1000;
      final prompt = LlmPromptTemplates.summarize(longText, maxLength: 150);
      expect(prompt, contains(longText));
      expect(prompt, contains('150'));
      expect(prompt, contains('摘要'));
    });

    test('applicationReason includes all parameters', () {
      final prompt = LlmPromptTemplates.applicationReason(
        requester: 'Alice',
        dataType: '财务记录',
        purpose: '年度审计',
      );
      expect(prompt, contains('Alice'));
      expect(prompt, contains('财务记录'));
      expect(prompt, contains('年度审计'));
      expect(prompt, contains('申请理由'));
    });

    test('structuredExtraction includes source text and schema', () {
      final prompt = LlmPromptTemplates.structuredExtraction(
        sourceText: 'John, 30, Engineer',
        fieldSchemaJson: '{"name": "string"}',
      );
      expect(prompt, contains('John, 30, Engineer'));
      expect(prompt, contains('string'));
      expect(prompt, contains('JSON'));
    });

    test('validateExtraction includes extracted data and constraints', () {
      final prompt = LlmPromptTemplates.validateExtraction(
        extractedJson: '{"age": 30}',
        constraintsJson: '{"age": {"min": 0, "max": 120}}',
      );
      expect(prompt, contains('30'));
      expect(prompt, contains('min'));
      expect(prompt, contains('is_valid'));
    });
  });
}
