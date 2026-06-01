import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';

void main() {
  group('DebugLogger', () {
    late DebugLogger logger;

    setUp(() {
      logger = DebugLogger.instance;
      logger.clearBuffer();
      logger.deactivate();
    });

    tearDown(() {
      logger.deactivate();
    });

    group('redact', () {
      test('wraps value with structured tag', () {
        expect(
          DebugLogger.redact('secret123', SensitiveType.credential),
          '[[SENSITIVE:credential:secret123]]',
        );
      });

      test('supports all sensitive types', () {
        expect(
          DebugLogger.redact('key', SensitiveType.crypto),
          '[[SENSITIVE:crypto:key]]',
        );
        expect(
          DebugLogger.redact('token', SensitiveType.token),
          '[[SENSITIVE:token:token]]',
        );
        expect(
          DebugLogger.redact('id', SensitiveType.identifier),
          '[[SENSITIVE:identifier:id]]',
        );
        expect(
          DebugLogger.redact('path', SensitiveType.path),
          '[[SENSITIVE:path:path]]',
        );
        expect(
          DebugLogger.redact('data', SensitiveType.generic),
          '[[SENSITIVE:generic:data]]',
        );
      });
    });

    group('activate/deactivate', () {
      test('isActive returns false by default', () {
        expect(logger.isActive, isFalse);
      });

      test('activate sets isActive to true', () {
        logger.activate();
        expect(logger.isActive, isTrue);
      });

      test('deactivate sets isActive to false', () {
        logger.activate();
        logger.deactivate();
        expect(logger.isActive, isFalse);
      });
    });

    group('logging', () {
      test('log adds entry to buffer', () {
        logger.log('TEST', 'message', LogLevel.info);
        expect(logger.entries.length, 1);
        expect(logger.entries.first.tag, 'TEST');
        expect(logger.entries.first.message, 'message');
        expect(logger.entries.first.level, LogLevel.info);
      });

      test('logError sets error level', () {
        logger.logError('AUTH', 'login failed');
        expect(logger.entries.first.level, LogLevel.error);
      });

      test('logInfo sets info level', () {
        logger.logInfo('BACKUP', 'started');
        expect(logger.entries.first.level, LogLevel.info);
      });

      test('logDebug sets debug level', () {
        logger.logDebug('TRACE', 'detail');
        expect(logger.entries.first.level, LogLevel.debug);
      });

      test('logWarning sets warning level', () {
        logger.logWarning('WARN', 'deprecated');
        expect(logger.entries.first.level, LogLevel.warning);
      });

      test('default log level is debug', () {
        logger.log('TAG', 'msg');
        expect(logger.entries.first.level, LogLevel.debug);
      });
    });

    group('sanitization', () {
      test('sanitizes structured sensitive tags', () {
        logger.logInfo('AUTH', 'Token: ${DebugLogger.redact('abc', SensitiveType.token)}');
        final export = logger.getExportLog();
        expect(export, contains('[REDACTED:token]'));
        expect(export, isNot(contains('abc')));
      });

      test('sanitizes password patterns', () {
        logger.logInfo('AUTH', 'password: secret123');
        final export = logger.getExportLog();
        expect(export, contains('password: [REDACTED]'));
        expect(export, isNot(contains('secret123')));
      });

      test('sanitizes key=value patterns', () {
        logger.logInfo('CRYPTO', 'salt=abcd1234');
        final export = logger.getExportLog();
        expect(export, contains('[REDACTED]'));
        expect(export, isNot(contains('abcd1234')));
      });

      test('sanitizes JSON field patterns', () {
        logger.logInfo('CONFIG', '"password": "myPass"');
        final export = logger.getExportLog();
        expect(export, contains('[REDACTED]'));
        expect(export, isNot(contains('myPass')));
      });

      test('sanitizes long hex strings', () {
        logger.logInfo('KEY', 'Key: aabbccdd11223344');
        final export = logger.getExportLog();
        expect(export, contains('[REDACTED]'));
        expect(export, isNot(contains('aabbccdd11223344')));
      });

      test('sanitizes file paths', () {
        logger.logInfo('PATH', 'Vault at /Users/test/solosoul/vault/data');
        final export = logger.getExportLog();
        expect(export, contains('[REDACTED]'));
      });

      test('does not over-sanitize short safe strings', () {
        logger.logInfo('MSG', 'Hello world 1234');
        final export = logger.getExportLog();
        expect(export, contains('Hello world 1234'));
      });

      test('double-sanitizes on export', () {
        logger.logInfo('AUTH', 'token: abcdef1234567890');
        final export = logger.getExportLog();
        expect(export, isNot(contains('abcdef1234567890')));
      });
    });

    group('buffer limits', () {
      test('truncates old entries when exceeding max buffer size', () {
        for (var i = 0; i < 1005; i++) {
          logger.log('TAG', 'msg $i');
        }
        expect(logger.entries.length, 1000);
        // Oldest entries should be removed
        expect(logger.entries.first.message, isNot('msg 0'));
        expect(logger.entries.last.message, 'msg 1004');
      });
    });

    group('getExportLog', () {
      test('returns message when buffer is empty', () {
        expect(logger.getExportLog(), 'No debug logs available.');
      });

      test('exports entries with timestamp and level', () {
        logger.logInfo('TEST', 'hello');
        final export = logger.getExportLog();
        expect(export, contains('[TEST]'));
        expect(export, contains('hello'));
        expect(export, contains('INFO'));
      });

      test('exports multiple entries separated by newline', () {
        logger.logInfo('A', 'first');
        logger.logError('B', 'second');
        final export = logger.getExportLog();
        expect(export.split('\n').length, 2);
      });
    });

    group('LogEntry', () {
      test('toLine formats correctly', () {
        final entry = LogEntry(
          timestamp: DateTime(2025, 1, 1, 12, 0, 0),
          tag: 'TAG',
          level: LogLevel.error,
          message: 'error msg',
        );
        expect(entry.toLine(), contains('[TAG]'));
        expect(entry.toLine(), contains('error msg'));
        expect(entry.toLine(), contains('ERROR'));
      });

      test('level label for warning is WARN', () {
        final entry = LogEntry(
          timestamp: DateTime.now(),
          tag: 'T',
          level: LogLevel.warning,
          message: 'm',
        );
        expect(entry.toLine(), contains('[WARN]'));
      });
    });
  });
}
