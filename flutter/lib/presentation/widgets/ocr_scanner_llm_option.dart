/// LLM Model Option data class for OCR scanner.
class OcrScannerLlmOption {
  final String id;
  final String displayName;
  final bool isLocal;
  final bool isAvailable;

  const OcrScannerLlmOption({
    required this.id,
    required this.displayName,
    required this.isLocal,
    required this.isAvailable,
  });
}
