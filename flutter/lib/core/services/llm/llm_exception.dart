class LlmException implements Exception {
  final String message;
  final LlmErrorCode code;

  const LlmException(this.message, {this.code = LlmErrorCode.unknown});

  @override
  String toString() => 'LlmException[$code]: $message';
}

enum LlmErrorCode {
  unknown,
  timeout,
  network,
  unauthorized,
  rateLimited,
  privacyBlocked,
  modelNotFound,
}
