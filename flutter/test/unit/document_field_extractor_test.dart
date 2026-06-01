import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/ocr_result.dart';
import 'package:solosoul_flutter/core/services/document_field_extractor.dart';

OcrBlock _block(String text, {double x = 0, double y = 0}) {
  return OcrBlock(
    text: text,
    confidence: 0.9,
    bbox: BoundingBox(x: x, y: y, width: 0.1, height: 0.05),
  );
}

void main() {
  group('FieldExtractorPipeline', () {
    test('extracts generic document as fallback', () {
      final blocks = [
        _block('Hello world'),
        _block('Contact: test@example.com'),
      ];
      final result = FieldExtractorPipeline.extract('Hello world Contact: test@example.com', blocks);
      expect(result.documentType, 'generic');
    });

    test('extracts business card when email and phone present', () {
      final blocks = [
        _block('John Doe'),
        _block('Engineer'),
        _block('john@example.com'),
        _block('+1 555-123-4567'),
        _block('www.example.com'),
      ];
      final result = FieldExtractorPipeline.extract(
        'John Doe Engineer john@example.com +1 555-123-4567 www.example.com',
        blocks,
      );
      expect(result.documentType, 'business_card');
      expect(result.hasFields, isTrue);
    });

    test('extracts invoice when invoice keyword present', () {
      final blocks = [
        _block('Invoice #12345'),
        _block('Date: 2024-01-01'),
        _block('Total: \$100.00'),
      ];
      final result = FieldExtractorPipeline.extract(
        'Invoice #12345 Date: 2024-01-01 Total: \$100.00',
        blocks,
      );
      expect(result.documentType, 'invoice');
    });
  });

  group('BusinessCardExtractor', () {
    final extractor = BusinessCardExtractor();

    test('canHandle returns true for email + phone + 5+ blocks', () {
      final blocks = [
        _block('John'),
        _block('john@example.com'),
        _block('+1 555-123-4567'),
        _block('Company'),
        _block('Title'),
      ];
      expect(extractor.canHandle('john@example.com +1 555-123-4567', blocks), isTrue);
    });

    test('canHandle returns false without email', () {
      final blocks = List.generate(5, (i) => _block('text \$i'));
      expect(extractor.canHandle('no email here +1 555-123-4567', blocks), isFalse);
    });

    test('canHandle returns false without phone', () {
      final blocks = List.generate(5, (i) => _block('text \$i'));
      expect(extractor.canHandle('email@example.com no phone', blocks), isFalse);
    });

    test('canHandle returns false with too few blocks', () {
      final blocks = [
        _block('john@example.com'),
        _block('+1 555-123-4567'),
      ];
      expect(extractor.canHandle('john@example.com +1 555-123-4567', blocks), isFalse);
    });

    test('extract finds email and phone', () {
      final blocks = [
        _block('John Doe'),
        _block('john@example.com'),
        _block('+1 555-123-4567'),
        _block('Company Inc'),
        _block('Engineer'),
      ];
      final fields = extractor.extract(blocks);
      expect(fields.containsKey('email'), isTrue);
      expect(fields['email']!.value, 'john@example.com');
      expect(fields.containsKey('phone'), isTrue);
    });
  });

  group('InvoiceExtractor', () {
    final extractor = InvoiceExtractor();

    test('canHandle returns true for invoice keyword', () {
      expect(extractor.canHandle('Invoice #123', []), isTrue);
      expect(extractor.canHandle('发票号码 123', []), isTrue);
    });

    test('canHandle returns true for amount + total', () {
      expect(extractor.canHandle('Total amount: \$100.00', []), isTrue);
    });

    test('canHandle returns false for generic text', () {
      expect(extractor.canHandle('Hello world', []), isFalse);
    });
  });

  group('ResumeExtractor', () {
    final extractor = ResumeExtractor();

    test('canHandle returns true for resume keywords', () {
      expect(extractor.canHandle('EDUCATION\nWORK EXPERIENCE', []), isTrue);
      expect(extractor.canHandle('Skills and Projects', []), isTrue);
    });

    test('canHandle returns true for education and experience', () {
      expect(
        extractor.canHandle('Education and Work Experience', []),
        isTrue,
      );
    });

    test('canHandle returns false for generic text', () {
      expect(extractor.canHandle('Hello world', []), isFalse);
    });
  });

  group('GenericFieldExtractor', () {
    final extractor = GenericFieldExtractor();

    test('canHandle always returns true', () {
      expect(extractor.canHandle('', []), isTrue);
      expect(extractor.canHandle('anything', []), isTrue);
    });

    test('extract finds email', () {
      final blocks = [_block('Contact: user@example.com')];
      final fields = extractor.extract(blocks);
      expect(fields.containsKey('email'), isTrue);
      expect(fields['email']!.value, 'user@example.com');
    });

    test('extract finds phone', () {
      final blocks = [_block('Call: +1 555-123-4567')];
      final fields = extractor.extract(blocks);
      expect(fields.containsKey('phone'), isTrue);
    });

    test('extract finds url', () {
      final blocks = [_block('Visit: https://example.com')];
      final fields = extractor.extract(blocks);
      expect(fields.containsKey('url'), isTrue);
      expect(fields['url']!.value, 'https://example.com');
    });

    test('extract finds date', () {
      final blocks = [_block('Date: 2024-01-15')];
      final fields = extractor.extract(blocks);
      expect(fields.containsKey('date'), isTrue);
    });
  });
}
