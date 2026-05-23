import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';

void main() {
  group('LogLevel', () {
    test('has expected values', () {
      expect(LogLevel.values, hasLength(4));
      expect(LogLevel.values, contains(LogLevel.debug));
      expect(LogLevel.values, contains(LogLevel.info));
      expect(LogLevel.values, contains(LogLevel.warning));
      expect(LogLevel.values, contains(LogLevel.error));
    });
  });

  group('SensitiveType', () {
    test('has expected values', () {
      expect(SensitiveType.values, hasLength(6));
      expect(SensitiveType.values, contains(SensitiveType.crypto));
      expect(SensitiveType.values, contains(SensitiveType.credential));
      expect(SensitiveType.values, contains(SensitiveType.token));
      expect(SensitiveType.values, contains(SensitiveType.identifier));
      expect(SensitiveType.values, contains(SensitiveType.path));
      expect(SensitiveType.values, contains(SensitiveType.generic));
    });
  });

  group('LogEntry', () {
    test('toLine formats correctly', () {
      final entry = LogEntry(
        timestamp: DateTime(2024, 6, 15, 10, 30, 0),
        tag: 'AUTH',
        level: LogLevel.info,
        message: 'Login successful',
      );
      final line = entry.toLine();
      expect(line, contains('2024-06-15'));
      expect(line, contains('[INFO]'));
      expect(line, contains('[AUTH]'));
      expect(line, contains('Login successful'));
    });

    test('toLine includes correct level labels', () {
      DateTime ts = DateTime(2024);
      expect(
        LogEntry(timestamp: ts, tag: 'T', level: LogLevel.debug, message: 'm')
            .toLine(),
        contains('[DEBUG]'),
      );
      expect(
        LogEntry(timestamp: ts, tag: 'T', level: LogLevel.info, message: 'm')
            .toLine(),
        contains('[INFO]'),
      );
      expect(
        LogEntry(timestamp: ts, tag: 'T', level: LogLevel.warning, message: 'm')
            .toLine(),
        contains('[WARN]'),
      );
      expect(
        LogEntry(timestamp: ts, tag: 'T', level: LogLevel.error, message: 'm')
            .toLine(),
        contains('[ERROR]'),
      );
    });
  });

  group('DebugLogger', () {
    late DebugLogger logger;

    setUp(() {
      logger = DebugLogger.instance;
      logger.deactivate(); // Ensure clean state
      logger.clearBuffer();
    });

    tearDown(() {
      logger.deactivate();
    });

    test('initially inactive', () {
      expect(logger.isActive, isFalse);
    });

    test('activate enables logging', () {
      logger.activate();
      expect(logger.isActive, isTrue);
    });

    test('deactivate disables logging', () {
      logger.activate();
      logger.deactivate();
      expect(logger.isActive, isFalse);
    });

    test('entries are captured even when inactive', () {
      logger.log('TEST', 'should still be buffered');
      expect(logger.entries, isNotEmpty);
      expect(logger.entries.last.tag, 'TEST');
    });

    test('entries are captured when active', () {
      logger.activate();
      logger.log('TEST', 'hello');
      expect(logger.entries, isNotEmpty);
      // activate() itself logs an entry, so our test entry is last
      expect(logger.entries.last.tag, 'TEST');
      expect(logger.entries.last.message, 'hello');
    });

    test('entries returns unmodifiable list', () {
      logger.activate();
      logger.log('TEST', 'hello');
      expect(
        () => logger.entries.add(
          LogEntry(
            timestamp: DateTime.now(),
            tag: 'X',
            level: LogLevel.debug,
            message: 'm',
          ),
        ),
        throwsUnsupportedError,
      );
    });

    test('log methods use correct levels', () {
      logger.activate();
      // activate() logs an info entry, so we need to account for it
      logger.logDebug('T', 'debug msg');
      logger.logInfo('T', 'info msg');
      logger.logWarning('T', 'warn msg');
      logger.logError('T', 'error msg');

      // Skip the activation entry (entries[0])
      final entries = logger.entries;
      expect(entries[1].level, LogLevel.debug);
      expect(entries[2].level, LogLevel.info);
      expect(entries[3].level, LogLevel.warning);
      expect(entries[4].level, LogLevel.error);
    });

    test('deactivate preserves buffer', () {
      logger.activate();
      logger.log('TEST', 'hello');
      expect(logger.entries, isNotEmpty);
      logger.deactivate();
      // Deactivate only stops console output; buffer is preserved
      expect(logger.entries, isNotEmpty);
      expect(logger.entries.last.tag, 'TEST');
    });
  });

  group('DebugLogger.redact', () {
    test('creates structured sensitive tag', () {
      final result = DebugLogger.redact('secret123', SensitiveType.credential);
      expect(result, '[[SENSITIVE:credential:secret123]]');
    });

    test('works with all SensitiveType values', () {
      for (final type in SensitiveType.values) {
        final result = DebugLogger.redact('val', type);
        expect(result, '[[SENSITIVE:${type.name}:val]]');
      }
    });
  });

  group('DebugLogger sanitization', () {
    late DebugLogger logger;

    setUp(() {
      logger = DebugLogger.instance;
      logger.deactivate();
    });

    tearDown(() {
      logger.deactivate();
    });

    test('getExportLog returns message when empty', () {
      logger.clearBuffer();
      expect(logger.getExportLog(), 'No debug logs available.');
    });

    test('getExportLog includes log entries', () {
      logger.activate();
      logger.logInfo('TEST', 'Hello world');
      final export = logger.getExportLog();
      expect(export, contains('TEST'));
      expect(export, contains('Hello world'));
    });

    test('sanitizes password patterns in log messages', () {
      logger.activate();
      logger.log('AUTH', 'password=abc123');
      final entry = logger.entries.last;
      expect(entry.message, contains('[REDACTED]'));
      expect(entry.message, isNot(contains('abc123')));
    });

    test('sanitizes key=value patterns', () {
      logger.activate();
      logger.log('AUTH', 'token: mysecrettoken');
      final entry = logger.entries.last;
      expect(entry.message, contains('[REDACTED]'));
    });

    test('sanitizes structured sensitive tags', () {
      logger.activate();
      logger.log(
        'AUTH',
        'Unlocked [[SENSITIVE:identifier:acc_abc123]]',
      );
      final entry = logger.entries.last;
      expect(entry.message, contains('[REDACTED:identifier]'));
      expect(entry.message, isNot(contains('acc_abc123')));
    });

    test('sanitizes acc_ patterns', () {
      logger.activate();
      logger.log('AUTH', 'Account acc_abcdef0123456789ab-cd loaded');
      final entry = logger.entries.last;
      expect(entry.message, contains('[REDACTED]'));
    });
  });
}
