import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/models/operation_log_models.dart';
import 'package:solosoul_flutter/presentation/providers/operation_log_provider.dart';

void main() {
  group('OperationLogServiceNotifier', () {
    setUp(() {
      OperationLogService.instance.clearMemoryCache();
    });

    test('build returns 0', () {
      final container = ProviderContainer();
      final version = container.read(operationLogProvider);
      expect(version, 0);
      container.dispose();
    });

    test('increments when service notifies', () async {
      final container = ProviderContainer();
      expect(container.read(operationLogProvider), 0);

      final entry = OperationEntry(
        timestamp: DateTime.now(),
        action: 'create',
        section: 'profile',
        description: 'test',
      );
      await OperationLogService.instance.addEntry(entry);

      expect(container.read(operationLogProvider), 1);
      container.dispose();
    });

    test('increments multiple times', () async {
      final container = ProviderContainer();
      expect(container.read(operationLogProvider), 0); // trigger build first
      for (var i = 0; i < 3; i++) {
        await OperationLogService.instance.addEntry(OperationEntry(
          timestamp: DateTime.now(),
          action: 'create',
          section: 'profile',
          description: 'entry $i',
        ));
      }
      expect(container.read(operationLogProvider), 3);
      container.dispose();
    });
  });

  group('LogActionFilter', () {
    test('build returns empty set', () {
      final container = ProviderContainer();
      expect(container.read(logActionFilterProvider), isEmpty);
      container.dispose();
    });

    test('setFilters updates state', () {
      final container = ProviderContainer();
      container.read(logActionFilterProvider.notifier).setFilters({'create', 'update'});
      expect(container.read(logActionFilterProvider), {'create', 'update'});
      container.dispose();
    });

    test('clear resets to empty', () {
      final container = ProviderContainer();
      final notifier = container.read(logActionFilterProvider.notifier);
      notifier.setFilters({'create'});
      notifier.clear();
      expect(container.read(logActionFilterProvider), isEmpty);
      container.dispose();
    });
  });

  group('LogDeviceFilter', () {
    test('build returns empty set', () {
      final container = ProviderContainer();
      expect(container.read(logDeviceFilterProvider), isEmpty);
      container.dispose();
    });

    test('setFilters updates state', () {
      final container = ProviderContainer();
      container.read(logDeviceFilterProvider.notifier).setFilters({'macos', 'ios'});
      expect(container.read(logDeviceFilterProvider), {'macos', 'ios'});
      container.dispose();
    });

    test('clear resets to empty', () {
      final container = ProviderContainer();
      final notifier = container.read(logDeviceFilterProvider.notifier);
      notifier.setFilters({'macos'});
      notifier.clear();
      expect(container.read(logDeviceFilterProvider), isEmpty);
      container.dispose();
    });
  });

  group('OperationLogEntries', () {
    setUp(() {
      OperationLogService.instance.clearMemoryCache();
    });

    test('build returns empty when no entries', () {
      final container = ProviderContainer();
      final entries = container.read(operationLogEntriesProvider);
      expect(entries, isEmpty);
      container.dispose();
    });

    test('build returns entries from service', () async {
      final container = ProviderContainer();
      final entry = OperationEntry(
        timestamp: DateTime.now(),
        action: 'create',
        section: 'profile',
        description: 'test entry',
      );
      await OperationLogService.instance.addEntry(entry);
      final entries = container.read(operationLogEntriesProvider);
      expect(entries.length, 1);
      expect(entries.first.description, 'test entry');
      container.dispose();
    });
  });

  group('OperationLogFilteredEntries', () {
    setUp(() {
      OperationLogService.instance.clearMemoryCache();
    });

    test('build returns all when no filters', () async {
      final container = ProviderContainer();
      await OperationLogService.instance.addEntry(OperationEntry(
        timestamp: DateTime.now(),
        action: 'create',
        section: 'profile',
        description: 'd1',
      ));
      await OperationLogService.instance.addEntry(OperationEntry(
        timestamp: DateTime.now().add(const Duration(seconds: 1)),
        action: 'delete',
        section: 'profile',
        description: 'd2',
      ));

      final entries = container.read(operationLogFilteredEntriesProvider);
      expect(entries.length, 2);
      container.dispose();
    });

    test('build filters by action', () async {
      final container = ProviderContainer();
      await OperationLogService.instance.addEntry(OperationEntry(
        timestamp: DateTime.now(),
        action: 'create',
        section: 'profile',
        description: 'd1',
      ));
      await OperationLogService.instance.addEntry(OperationEntry(
        timestamp: DateTime.now().add(const Duration(seconds: 1)),
        action: 'delete',
        section: 'profile',
        description: 'd2',
      ));

      container.read(logActionFilterProvider.notifier).setFilters({'create'});
      final entries = container.read(operationLogFilteredEntriesProvider);
      expect(entries.length, 1);
      expect(entries.first.action, 'create');
      container.dispose();
    });

    test('build returns empty when filters match nothing', () async {
      final container = ProviderContainer();
      await OperationLogService.instance.addEntry(OperationEntry(
        timestamp: DateTime.now(),
        action: 'create',
        section: 'profile',
        description: 'd1',
      ));

      container.read(logActionFilterProvider.notifier).setFilters({'delete'});
      final entries = container.read(operationLogFilteredEntriesProvider);
      expect(entries, isEmpty);
      container.dispose();
    });
  });

  group('OperationLogService', () {
    late OperationLogService service;

    setUp(() {
      service = OperationLogService.instance;
      service.clearMemoryCache();
    });

    test('getEntries returns empty list initially', () {
      expect(service.getEntries(), isEmpty);
    });

    test('addEntry adds an entry', () async {
      final entry = OperationEntry(
        timestamp: DateTime.now(),
        action: 'create',
        section: 'profile',
        description: 'Created profile',
      );
      await service.addEntry(entry);
      final entries = service.getEntries();
      expect(entries.length, 1);
      expect(entries.first.action, 'create');
    });

    test('getEntries returns unmodifiable list', () async {
      final entry = OperationEntry(
        timestamp: DateTime.now(),
        action: 'create',
        section: 'profile',
        description: 'test',
      );
      await service.addEntry(entry);
      final entries = service.getEntries();
      expect(() => entries.add(entry), throwsUnsupportedError);
    });

    test('entries are sorted newest first', () async {
      final older = OperationEntry(
        timestamp: DateTime.now().subtract(const Duration(days: 1)),
        action: 'create',
        section: 'profile',
        description: 'older',
      );
      final newer = OperationEntry(
        timestamp: DateTime.now(),
        action: 'create',
        section: 'profile',
        description: 'newer',
      );
      await service.addEntry(older);
      await service.addEntry(newer);
      final entries = service.getEntries();
      expect(entries.first.description, 'newer');
      expect(entries.last.description, 'older');
    });

    test('getFilteredEntries filters by action', () async {
      final e1 = OperationEntry(
        timestamp: DateTime.now(),
        action: 'create',
        section: 'profile',
        description: 'd1',
      );
      final e2 = OperationEntry(
        timestamp: DateTime.now().add(const Duration(seconds: 1)),
        action: 'delete',
        section: 'profile',
        description: 'd2',
      );
      await service.addEntry(e1);
      await service.addEntry(e2);
      final filtered = service.getFilteredEntries(actionFilters: {'create'});
      expect(filtered.length, 1);
      expect(filtered.first.action, 'create');
    });

    test('getFilteredEntries filters by device', () async {
      final e1 = OperationEntry(
        timestamp: DateTime.now(),
        action: 'create',
        section: 'profile',
        description: 'd1',
        device: 'macos',
      );
      final e2 = OperationEntry(
        timestamp: DateTime.now().add(const Duration(seconds: 1)),
        action: 'create',
        section: 'profile',
        description: 'd2',
        device: 'macos',
      );
      await service.addEntry(e1);
      await service.addEntry(e2);
      // addEntry overwrites device to current platform, so both are macos
      final filtered = service.getFilteredEntries(deviceFilters: {'ios'});
      expect(filtered, isEmpty);
      final filteredMacos = service.getFilteredEntries(deviceFilters: {'macos'});
      expect(filteredMacos.length, 2);
    });

    test('getFilteredEntries returns all when no filters', () async {
      final e1 = OperationEntry(
        timestamp: DateTime.now(),
        action: 'create',
        section: 'profile',
        description: 'd1',
      );
      final e2 = OperationEntry(
        timestamp: DateTime.now().add(const Duration(seconds: 1)),
        action: 'delete',
        section: 'profile',
        description: 'd2',
      );
      await service.addEntry(e1);
      await service.addEntry(e2);
      final filtered = service.getFilteredEntries();
      expect(filtered.length, 2);
    });

    test('clearMemoryCache clears entries', () async {
      final entry = OperationEntry(
        timestamp: DateTime.now(),
        action: 'create',
        section: 'profile',
        description: 'test',
      );
      await service.addEntry(entry);
      expect(service.getEntries().length, 1);
      service.clearMemoryCache();
      expect(service.getEntries(), isEmpty);
    });

    test('clearForCurrentAccount clears everything', () async {
      final entry = OperationEntry(
        timestamp: DateTime.now(),
        action: 'create',
        section: 'profile',
        description: 'test',
      );
      await service.addEntry(entry);
      expect(service.getEntries().length, 1);
      await service.clearForCurrentAccount();
      expect(service.getEntries(), isEmpty);
    });

    test('TTL removes entries older than 90 days', () async {
      final oldEntry = OperationEntry(
        timestamp: DateTime.now().subtract(const Duration(days: 91)),
        action: 'create',
        section: 'profile',
        description: 'old',
      );
      final newEntry = OperationEntry(
        timestamp: DateTime.now(),
        action: 'create',
        section: 'profile',
        description: 'new',
      );
      await service.addEntry(oldEntry);
      await service.addEntry(newEntry);
      final entries = service.getEntries();
      expect(entries.any((e) => e.description == 'old'), isFalse);
      expect(entries.any((e) => e.description == 'new'), isTrue);
    });

    test('TTL enforces max 500 entries', () async {
      for (var i = 0; i < 505; i++) {
        final entry = OperationEntry(
          timestamp: DateTime.now().subtract(Duration(seconds: i)),
          action: 'create',
          section: 'profile',
          description: 'entry_$i',
        );
        await service.addEntry(entry);
      }
      final entries = service.getEntries();
      expect(entries.length, lessThanOrEqualTo(500));
    });

    test('addEntry captures device platform', () async {
      final entry = OperationEntry(
        timestamp: DateTime.now(),
        action: 'create',
        section: 'profile',
        description: 'device test',
      );
      await service.addEntry(entry);
      final entries = service.getEntries();
      expect(entries.first.device, isNot('unknown'));
      expect(entries.first.device, isA<String>());
    });

    test('addEntry preserves existing device when matches', () async {
      final entry = OperationEntry(
        timestamp: DateTime.now(),
        action: 'create',
        section: 'profile',
        description: 'device match',
        device: 'macos',
      );
      await service.addEntry(entry);
      final entries = service.getEntries();
      expect(entries.first.device, 'macos');
    });
  });
}
