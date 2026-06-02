import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/ocr_result.dart';
import 'package:solosoul_flutter/core/services/document_field_extractor.dart';

void main() {
  group('ExtractionResult', () {
    test('hasFields returns true when fields not empty', () {
      const result = ExtractionResult(
        documentType: 'test',
        fields: {
          'name': ExtractedField(
            value: 'Test',
            bbox: BoundingBox(x: 0, y: 0, width: 1, height: 1),
          ),
        },
        rawText: 'test',
      );
      expect(result.hasFields, isTrue);
    });

    test('hasFields returns false when fields empty', () {
      const result = ExtractionResult(
        documentType: 'generic',
        fields: {},
        rawText: 'test',
      );
      expect(result.hasFields, isFalse);
    });
  });

  group('BusinessCardExtractor', () {
    final extractor = BusinessCardExtractor();

    test('canHandle returns true for text with email and phone', () {
      final blocks = [
        _block('John Doe', 0, 0),
        _block('john@example.com', 0, 0.1),
        _block('+1 234 567 8900', 0, 0.2),
        _block('Company', 0, 0.3),
        _block('Address', 0, 0.4),
      ];
      const text = 'John Doe john@example.com +1 234 567 8900 Company Address';
      expect(extractor.canHandle(text, blocks), isTrue);
    });

    test('canHandle returns false without email', () {
      final blocks = List.generate(5, (i) => _block('text', 0, i * 0.1));
      expect(extractor.canHandle('no email here +1234567890', blocks), isFalse);
    });

    test('canHandle returns false without phone', () {
      final blocks = List.generate(5, (i) => _block('text', 0, i * 0.1));
      expect(extractor.canHandle('test@example.com no phone', blocks), isFalse);
    });

    test('canHandle returns false for too few blocks', () {
      final blocks = [
        _block('john@example.com', 0, 0),
        _block('+1234567890', 0, 0.1),
      ];
      expect(extractor.canHandle('john@example.com +1234567890', blocks), isFalse);
    });

    test('canHandle returns false for too many blocks', () {
      final blocks = List.generate(30, (i) => _block('text', 0, i * 0.01));
      final text = 'john@example.com +1234567890 ${'word ' * 30}';
      expect(extractor.canHandle(text, blocks), isFalse);
    });

    test('extract finds name and email', () {
      final blocks = [
        _block('John', 0.1, 0.0, height: 0.05),
        _block('Doe', 0.3, 0.0, height: 0.05),
        _block('Manager', 0.1, 0.08, height: 0.03),
        _block('john@example.com', 0.1, 0.15, height: 0.02),
        _block('+1234567890', 0.1, 0.22, height: 0.02),
      ];
      final fields = extractor.extract(blocks);
      expect(fields.containsKey('name'), isTrue);
      expect(fields.containsKey('email'), isTrue);
      expect(fields['email']!.value, 'john@example.com');
    });
  });

  group('FieldExtractorPipeline', () {
    test('extract returns generic for empty input', () {
      final result = FieldExtractorPipeline.extract('', []);
      expect(result.documentType, 'generic');
      expect(result.fields, isEmpty);
    });

    test('extract detects business card', () {
      final blocks = [
        _block('Alice', 0.1, 0.0, height: 0.05),
        _block('Smith', 0.3, 0.0, height: 0.05),
        _block('alice@test.com', 0.1, 0.1, height: 0.02),
        _block('+86 138 0013 8000', 0.1, 0.2, height: 0.02),
        _block('Engineer', 0.1, 0.08, height: 0.03),
      ];
      const text = 'Alice Smith Engineer alice@test.com +86 138 0013 8000';
      final result = FieldExtractorPipeline.extract(text, blocks);
      expect(result.documentType, 'business_card');
      expect(result.hasFields, isTrue);
      expect(result.fields.containsKey('name'), isTrue);
      expect(result.fields.containsKey('email'), isTrue);
    });
  });
}

OcrBlock _block(String text, double x, double y, {double height = 0.02}) {
  return OcrBlock(
    text: text,
    confidence: 0.9,
    bbox: BoundingBox(x: x, y: y, width: 0.1, height: height),
  );
}
