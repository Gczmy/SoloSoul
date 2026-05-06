import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/llm/llm_field_mapping_parser.dart';

void main() {
  group('LlmFieldMappingParser', () {
    test('parses valid JSON with all fields', () {
      const jsonText = '''
      {
        "mappings": [
          {"source_field": "姓名", "target_property_id": "fullName", "confidence": 0.95, "reason": "直接对应"},
          {"source_field": "电话", "target_property_id": "phone", "confidence": 0.88, "reason": "常见字段"}
        ],
        "unmapped": ["备注"],
        "suggested_object_type": "profile_identity"
      }
      ''';

      final result = LlmFieldMappingParser.parse(jsonText);

      expect(result.mappings.length, 2);
      expect(result.mappings[0].sourceField, '姓名');
      expect(result.mappings[0].targetPropertyId, 'fullName');
      expect(result.mappings[0].confidence, 0.95);
      expect(result.mappings[0].reason, '直接对应');
      expect(result.mappings[0].source, 'local');

      expect(result.unmapped, ['备注']);
      expect(result.suggestedObjectType, 'profile_identity');
    });

    test('parses JSON inside markdown code block', () {
      const jsonText = '''
      ```json
      {
        "mappings": [
          {"source_field": "email", "target_property_id": "email", "confidence": 0.9, "reason": "匹配"}
        ],
        "unmapped": []
      }
      ```
      ''';

      final result = LlmFieldMappingParser.parse(jsonText);
      expect(result.mappings.length, 1);
      expect(result.mappings[0].sourceField, 'email');
      expect(result.unmapped, isEmpty);
    });

    test('parses plain JSON without code block', () {
      const jsonText = '{"mappings": [], "unmapped": ["x"]}';
      final result = LlmFieldMappingParser.parse(jsonText);
      expect(result.mappings, isEmpty);
      expect(result.unmapped, ['x']);
    });

    test('handles empty mappings and unmapped', () {
      const jsonText = '{}';
      final result = LlmFieldMappingParser.parse(jsonText);
      expect(result.mappings, isEmpty);
      expect(result.unmapped, isEmpty);
      expect(result.suggestedObjectType, isNull);
    });

    test('uses custom source parameter', () {
      const jsonText = '''
      {"mappings": [{"source_field": "a", "confidence": 0.5, "reason": "r"}], "unmapped": []}
      ''';
      final result = LlmFieldMappingParser.parse(jsonText, source: 'cloud');
      expect(result.mappings.first.source, 'cloud');
    });

    test('handles missing optional fields with defaults', () {
      const jsonText = '''
      {"mappings": [{"source_field": "a"}], "unmapped": []}
      ''';
      final result = LlmFieldMappingParser.parse(jsonText);
      expect(result.mappings.first.targetPropertyId, isNull);
      expect(result.mappings.first.confidence, 0.0);
      expect(result.mappings.first.reason, '');
    });

    test('skips non-map entries in mappings array', () {
      const jsonText = '''
      {"mappings": ["bad", {"source_field": "good", "confidence": 1.0, "reason": "ok"}], "unmapped": []}
      ''';
      final result = LlmFieldMappingParser.parse(jsonText);
      expect(result.mappings.length, 1);
      expect(result.mappings.first.sourceField, 'good');
    });

    test('converts confidence to double', () {
      const jsonText = '''
      {"mappings": [{"source_field": "a", "confidence": 1, "reason": "r"}], "unmapped": []}
      ''';
      final result = LlmFieldMappingParser.parse(jsonText);
      expect(result.mappings.first.confidence, isA<double>());
      expect(result.mappings.first.confidence, 1.0);
    });

    test('throws FormatException on invalid JSON', () {
      const jsonText = 'not json at all';
      expect(
        () => LlmFieldMappingParser.parse(jsonText),
        throwsA(isA<FormatException>()),
      );
    });
  });
}
