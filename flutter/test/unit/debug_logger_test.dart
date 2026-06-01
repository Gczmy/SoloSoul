import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';

void main() {
  group('LogEntry', () {
    test('toLine formats correctly', () {
      final entry = LogEntry(
        timestamp: DateTime.utc(2024, 1, 1, 12, 0, 0),
        tag: 'TEST',
        level: LogLevel.info,
        message: 'Hello',
      );
      expect(entry.toLine(), contains('TEST'));
      expect(entry.toLine(), contains('Hello'));
      expect(entry.toLine(), contains('INFO'));
    });

    test('toLine for error level', () {
      final entry = LogEntry(
        timestamp: DateTime.utc(2024, 1, 1),
        tag: 'ERR',
        level: LogLevel.error,
        message: 'Fail',
      );
      expect(entry.toLine(), contains('ERROR'));
    });
  });

  group('DebugLogger', () {
    setUp(() {
      DebugLogger.instance.clearBuffer();
      DebugLogger.instance.deactivate();
    });

    test('isActive is false by default', () {
      expect(DebugLogger.instance.isActive, isFalse);
    });

    test('activate sets isActive to true', () {
      DebugLogger.instance.activate();
      expect(DebugLogger.instance.isActive, isTrue);
    });

    test('logInfo adds entry to buffer', () {
      DebugLogger.instance.logInfo('TAG', 'message');
      final logs = DebugLogger.instance.entries;
      expect(logs.length, 1);
      expect(logs.first.tag, 'TAG');
      expect(logs.first.level, LogLevel.info);
    });

    test('logWarning adds entry to buffer', () {
      DebugLogger.instance.logWarning('TAG', 'warn');
      final logs = DebugLogger.instance.entries;
      expect(logs.first.level, LogLevel.warning);
    });

    test('logError adds entry to buffer', () {
      DebugLogger.instance.logError('TAG', 'error');
      final logs = DebugLogger.instance.entries;
      expect(logs.first.level, LogLevel.error);
    });

    test('logDebug adds entry to buffer', () {
      DebugLogger.instance.logDebug('TAG', 'debug');
      final logs = DebugLogger.instance.entries;
      expect(logs.first.level, LogLevel.debug);
    });

    test('clearBuffer removes all entries', () {
      DebugLogger.instance.logInfo('TAG', 'msg1');
      DebugLogger.instance.logInfo('TAG', 'msg2');
      expect(DebugLogger.instance.entries.length, 2);

      DebugLogger.instance.clearBuffer();
      expect(DebugLogger.instance.entries, isEmpty);
    });

    test('buffer has max size limit', () {
      for (var i = 0; i < 1010; i++) {
        DebugLogger.instance.logInfo('TAG', 'msg \$i');
      }
      expect(DebugLogger.instance.entries.length, lessThanOrEqualTo(1000));
    });

    test('sanitizes sensitive tags', () {
      DebugLogger.instance.logInfo('TAG', 'token: [[SENSITIVE:token:abc123]]');
      final log = DebugLogger.instance.entries.first.message;
      expect(log, isNot(contains('abc123')));
      expect(log, contains('[REDACTED:token]'));
    });
  });
}
