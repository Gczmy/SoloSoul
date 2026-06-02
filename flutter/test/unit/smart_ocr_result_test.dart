import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/ocr_result.dart';
import 'package:solosoul_flutter/core/models/smart_ocr_result.dart';
import 'package:solosoul_flutter/core/services/document_field_extractor.dart';

void main() {
  group('SmartOcrTextResult', () {
    test('constructs with ocr result and extraction', () {
      const ocr = OcrResult(rawText: 'Hello', blocks: [], confidence: 0.9);
      const extraction = ExtractionResult(
        documentType: 'generic',
        fields: {},
        rawText: 'Hello',
      );
      const result = SmartOcrTextResult(ocr, extraction);

      expect(result.ocrResult, ocr);
      expect(result.extraction, extraction);
    });
  });

  group('SmartOcrMrzResult', () {
    test('constructs with mrz data and raw ocr', () {
      const ocr = OcrResult(rawText: 'P<CHN...', blocks: [], confidence: 0.95);
      const mrz = MrzData(
        documentType: 'P',
        country: 'CHN',
        surname: 'Li',
        givenNames: 'Wei',
        documentNumber: 'E12345678',
        nationality: 'CHN',
        dateOfBirth: '900101',
        sex: 'M',
        expiryDate: '250101',
        confidence: 1.0,
        rawLines: ['line1', 'line2'],
      );
      const result = SmartOcrMrzResult(mrzData: mrz, rawOcrResult: ocr);

      expect(result.mrzData, mrz);
      expect(result.rawOcrResult, ocr);
    });
  });
}
