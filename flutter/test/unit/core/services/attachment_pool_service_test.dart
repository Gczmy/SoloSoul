import 'dart:io';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:path_provider/path_provider.dart';
import 'package:solosoul_flutter/core/services/attachment_pool_service.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  late Directory testAppSupportDir;

  setUpAll(() async {
    testAppSupportDir = Directory.systemTemp.createTempSync('solosoul_pool_test_');

    // Mock path_provider method channel
    const channel = MethodChannel('plugins.flutter.io/path_provider');
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
      if (call.method == 'getApplicationSupportDirectory') {
        return testAppSupportDir.path;
      }
      return null;
    });
  });

  tearDownAll(() {
    const channel = MethodChannel('plugins.flutter.io/path_provider');
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, null);
    if (testAppSupportDir.existsSync()) {
      testAppSupportDir.deleteSync(recursive: true);
    }
  });

  final accountId = 'test_account_123';
  final pool = AttachmentPoolService.instance;

  tearDown(() async {
    // 清理测试 account 的池目录
    try {
      final appDir = await getApplicationSupportDirectory();
      final poolDir = Directory(
        '${appDir.path}/solosoul_backups/$accountId/attachments_pool',
      );
      if (poolDir.existsSync()) {
        poolDir.deleteSync(recursive: true);
      }
    } on Exception {
      // 忽略清理错误
    }
  });

  group('AttachmentPoolService', () {
    test('poolFileExists returns false for non-existent file', () async {
      final exists = await pool.poolFileExists(accountId, 'nonexistent');
      expect(exists, false);
    });

    test('ensureInPool copies file to pool', () async {
      // 创建临时源文件
      final tempDir = Directory.systemTemp.createTempSync('pool_src_');
      final srcFile = File('${tempDir.path}/test.solo');
      srcFile.writeAsStringSync('encrypted content');

      final ok = await pool.ensureInPool(accountId, 'file001', srcFile.path);
      expect(ok, true);

      final exists = await pool.poolFileExists(accountId, 'file001');
      expect(exists, true);

      // 清理
      tempDir.deleteSync(recursive: true);
    });

    test('ensureInPool skips if already in pool', () async {
      final tempDir = Directory.systemTemp.createTempSync('pool_src_');
      final srcFile = File('${tempDir.path}/test.solo');
      srcFile.writeAsStringSync('encrypted content');

      // 第一次放入
      final ok1 = await pool.ensureInPool(accountId, 'file002', srcFile.path);
      expect(ok1, true);

      // 第二次应直接跳过
      final ok2 = await pool.ensureInPool(accountId, 'file002', srcFile.path);
      expect(ok2, true);

      // 确认文件内容未被覆盖（实际上不会复制）
      final exists = await pool.poolFileExists(accountId, 'file002');
      expect(exists, true);

      tempDir.deleteSync(recursive: true);
    });

    test('getFromPool copies file to destination', () async {
      final tempDir = Directory.systemTemp.createTempSync('pool_src_');
      final srcFile = File('${tempDir.path}/test.solo');
      srcFile.writeAsStringSync('encrypted content');

      await pool.ensureInPool(accountId, 'file003', srcFile.path);

      final dstPath = '${tempDir.path}/restored.solo';
      final ok = await pool.getFromPool(accountId, 'file003', dstPath);
      expect(ok, true);
      expect(File(dstPath).existsSync(), true);
      expect(File(dstPath).readAsStringSync(), 'encrypted content');

      tempDir.deleteSync(recursive: true);
    });

    test('getFromPool returns false for missing file', () async {
      final tempDir = Directory.systemTemp.createTempSync('pool_dst_');
      final dstPath = '${tempDir.path}/missing.solo';

      final ok = await pool.getFromPool(accountId, 'missing_id', dstPath);
      expect(ok, false);
      expect(File(dstPath).existsSync(), false);

      tempDir.deleteSync(recursive: true);
    });

    test('removeFromPool deletes file', () async {
      final tempDir = Directory.systemTemp.createTempSync('pool_src_');
      final srcFile = File('${tempDir.path}/test.solo');
      srcFile.writeAsStringSync('encrypted content');

      await pool.ensureInPool(accountId, 'file004', srcFile.path);
      expect(await pool.poolFileExists(accountId, 'file004'), true);

      await pool.removeFromPool(accountId, 'file004');
      expect(await pool.poolFileExists(accountId, 'file004'), false);

      tempDir.deleteSync(recursive: true);
    });

    test('getPoolSize returns total size of all pooled files', () async {
      final tempDir = Directory.systemTemp.createTempSync('pool_src_');

      for (var i = 0; i < 3; i++) {
        final srcFile = File('${tempDir.path}/file$i.solo');
        srcFile.writeAsStringSync('content $i'); // 8 + i bytes roughly
        await pool.ensureInPool(accountId, 'sizefile$i', srcFile.path);
      }

      final size = await pool.getPoolSize(accountId);
      expect(size, greaterThan(0));

      tempDir.deleteSync(recursive: true);
    });

    test('getPoolFileIds returns all file IDs', () async {
      final tempDir = Directory.systemTemp.createTempSync('pool_src_');

      final ids = <String>{'id_a', 'id_b', 'id_c'};
      for (final id in ids) {
        final srcFile = File('${tempDir.path}/$id.solo');
        srcFile.writeAsStringSync('data');
        await pool.ensureInPool(accountId, id, srcFile.path);
      }

      final poolIds = await pool.getPoolFileIds(accountId);
      expect(poolIds, equals(ids));

      tempDir.deleteSync(recursive: true);
    });

    test('getPoolFileSize returns correct size', () async {
      final tempDir = Directory.systemTemp.createTempSync('pool_src_');
      final srcFile = File('${tempDir.path}/sized.solo');
      srcFile.writeAsStringSync('12345'); // 5 bytes

      await pool.ensureInPool(accountId, 'sized_file', srcFile.path);
      final size = await pool.getPoolFileSize(accountId, 'sized_file');
      expect(size, 5);

      final missingSize = await pool.getPoolFileSize(accountId, 'missing');
      expect(missingSize, 0);

      tempDir.deleteSync(recursive: true);
    });
  });
}
