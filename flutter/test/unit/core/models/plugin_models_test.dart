import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/plugin_models.dart';

void main() {
  group('PluginRegistry', () {
    test('empty factory creates empty registry', () {
      final registry = PluginRegistry.empty();
      expect(registry.version, '1');
      expect(registry.plugins, isEmpty);
      expect(registry.updatedAt.isUtc, isTrue);
    });

    test('fromJson parses full registry', () {
      final json = {
        'version': '1',
        'updated_at': '2024-01-01T00:00:00Z',
        'plugins': {
          'com.example.plugin': {
            'name': 'Test Plugin',
            'publisher': 'Test Publisher',
            'latest_version': '1.0.0',
            'versions': {
              '1.0.0': {
                'sha256': 'abc123',
                'plugin_api_version': '1.0',
                'min_app_version': '1.0.0',
                'max_app_version': '2.0.0',
                'download_url': 'https://example.com/plugin.wasm',
                'released_at': '2024-01-01T00:00:00Z',
              }
            }
          }
        }
      };

      final registry = PluginRegistry.fromJson(json);
      expect(registry.version, '1');
      expect(registry.plugins.length, 1);
      expect(registry.plugins['com.example.plugin']?.name, 'Test Plugin');
    });

    test('fromJson handles null plugins', () {
      final json = {
        'version': '1',
        'updated_at': '2024-01-01T00:00:00Z',
      };

      final registry = PluginRegistry.fromJson(json);
      expect(registry.plugins, isEmpty);
    });

    test('toJson roundtrip preserves data', () {
      final original = PluginRegistry(
        version: '1',
        updatedAt: DateTime.utc(2024, 1, 1),
        plugins: {
          'com.test.plugin': PluginRegistryEntry(
            name: 'Test',
            publisher: 'Publisher',
            latestVersion: '1.0.0',
            versions: {},
          )
        },
      );

      final json = original.toJson();
      final restored = PluginRegistry.fromJson(json);

      expect(restored.version, original.version);
      expect(restored.plugins.length, original.plugins.length);
      expect(restored.plugins['com.test.plugin']?.name, 'Test');
    });
  });

  group('PluginRegistryEntry', () {
    test('fromJson parses correctly', () {
      final json = {
        'name': 'My Plugin',
        'publisher': 'My Publisher',
        'latest_version': '2.0.0',
        'versions': {
          '1.0.0': {
            'sha256': 'hash1',
            'plugin_api_version': '1.0',
            'min_app_version': '1.0.0',
            'max_app_version': '2.0.0',
            'download_url': 'https://example.com/v1.wasm',
            'released_at': '2024-01-01T00:00:00Z',
          },
          '2.0.0': {
            'sha256': 'hash2',
            'plugin_api_version': '1.0',
            'min_app_version': '1.0.0',
            'max_app_version': '3.0.0',
            'download_url': 'https://example.com/v2.wasm',
            'released_at': '2024-06-01T00:00:00Z',
          }
        }
      };

      final entry = PluginRegistryEntry.fromJson(json);
      expect(entry.name, 'My Plugin');
      expect(entry.publisher, 'My Publisher');
      expect(entry.latestVersion, '2.0.0');
      expect(entry.versions.length, 2);
      expect(entry.versions['1.0.0']?.sha256, 'hash1');
      expect(entry.versions['2.0.0']?.sha256, 'hash2');
    });

    test('toJson roundtrip preserves data', () {
      final original = PluginRegistryEntry(
        name: 'Test',
        publisher: 'Pub',
        latestVersion: '1.0.0',
        versions: {
          '1.0.0': PluginVersionInfo(
            sha256: 'abc',
            pluginApiVersion: '1.0',
            minAppVersion: '1.0.0',
            maxAppVersion: '2.0.0',
            downloadUrl: 'https://example.com',
            releasedAt: DateTime.utc(2024, 1, 1),
          )
        },
      );

      final json = original.toJson();
      final restored = PluginRegistryEntry.fromJson(json);

      expect(restored.name, original.name);
      expect(restored.versions.length, 1);
      expect(restored.versions['1.0.0']?.sha256, 'abc');
    });
  });

  group('PluginVersionInfo', () {
    test('fromJson parses correctly', () {
      final json = {
        'sha256': 'deadbeef',
        'plugin_api_version': '1.0',
        'min_app_version': '1.0.0',
        'max_app_version': '999.999.999',
        'download_url': 'https://plugins.solosoul.dev/slotgo/plugin.wasm',
        'released_at': '2026-05-23T00:00:00Z',
      };

      final info = PluginVersionInfo.fromJson(json);
      expect(info.sha256, 'deadbeef');
      expect(info.pluginApiVersion, '1.0');
      expect(info.minAppVersion, '1.0.0');
      expect(info.maxAppVersion, '999.999.999');
      expect(info.downloadUrl, 'https://plugins.solosoul.dev/slotgo/plugin.wasm');
      expect(info.releasedAt.year, 2026);
    });

    test('fromJson handles missing released_at', () {
      final json = {
        'sha256': 'abc',
        'plugin_api_version': '1.0',
        'min_app_version': '1.0.0',
        'max_app_version': '2.0.0',
        'download_url': 'https://example.com',
      };

      final info = PluginVersionInfo.fromJson(json);
      expect(info.sha256, 'abc');
      expect(info.releasedAt.isAfter(DateTime(2000)), isTrue);
    });

    test('toJson roundtrip preserves data', () {
      final original = PluginVersionInfo(
        sha256: 'hash',
        pluginApiVersion: '1.0',
        minAppVersion: '1.0.0',
        maxAppVersion: '2.0.0',
        downloadUrl: 'https://example.com',
        releasedAt: DateTime.utc(2024, 6, 15),
      );

      final json = original.toJson();
      final restored = PluginVersionInfo.fromJson(json);

      expect(restored.sha256, original.sha256);
      expect(restored.pluginApiVersion, original.pluginApiVersion);
      expect(restored.downloadUrl, original.downloadUrl);
      expect(restored.releasedAt, original.releasedAt);
    });
  });

  group('InstalledPluginInfo', () {
    test('fromJson parses installed plugin', () {
      final json = {
        'version': '1.0.0',
        'status': 'installed',
        'installed_at': '2024-01-01T00:00:00Z',
      };

      final info = InstalledPluginInfo.fromJson(json);
      expect(info.version, '1.0.0');
      expect(info.status, 'installed');
      expect(info.installedAt, isNotNull);
      expect(info.uninstalledAt, isNull);
    });

    test('fromJson parses uninstalled plugin', () {
      final json = {
        'version': '1.0.0',
        'status': 'uninstalled',
        'installed_at': '2024-01-01T00:00:00Z',
        'uninstalled_at': '2024-02-01T00:00:00Z',
      };

      final info = InstalledPluginInfo.fromJson(json);
      expect(info.status, 'uninstalled');
      expect(info.uninstalledAt, isNotNull);
    });

    test('fromJson uses defaults for missing fields', () {
      final json = <String, dynamic>{};

      final info = InstalledPluginInfo.fromJson(json);
      expect(info.version, '');
      expect(info.status, 'installed');
      expect(info.installedAt, isNull);
    });

    test('toJson omits null dates', () {
      final info = InstalledPluginInfo(
        version: '1.0.0',
        status: 'installed',
      );

      final json = info.toJson();
      expect(json.containsKey('installed_at'), isFalse);
      expect(json.containsKey('uninstalled_at'), isFalse);
      expect(json['version'], '1.0.0');
    });

    test('toJson includes non-null dates', () {
      final info = InstalledPluginInfo(
        version: '1.0.0',
        status: 'installed',
        installedAt: DateTime.utc(2024, 1, 1),
      );

      final json = info.toJson();
      expect(json.containsKey('installed_at'), isTrue);
      expect(json['installed_at'], '2024-01-01T00:00:00.000Z');
    });
  });

  group('Plugin Exceptions', () {
    test('PluginNotFoundException toString', () {
      final e = PluginNotFoundException('my-plugin');
      expect(e.toString(), contains('my-plugin'));
    });

    test('PluginIncompatibleException toString', () {
      final e = PluginIncompatibleException('my-plugin');
      expect(e.toString(), contains('my-plugin'));
    });

    test('PluginSecurityException toString', () {
      final e = PluginSecurityException('hash mismatch');
      expect(e.toString(), contains('hash mismatch'));
    });

    test('PluginExecutionException toString', () {
      final e = PluginExecutionException('wasm trap');
      expect(e.toString(), contains('wasm trap'));
    });
  });

  group('PluginUpdateInfo', () {
    test('creates with required fields', () {
      final info = PluginUpdateInfo(
        pluginId: 'com.test.plugin',
        currentVersion: '1.0.0',
        latestVersion: '2.0.0',
      );
      expect(info.pluginId, 'com.test.plugin');
      expect(info.currentVersion, '1.0.0');
      expect(info.latestVersion, '2.0.0');
    });
  });

  group('PluginRunResult', () {
    test('creates with exit code', () {
      final result = PluginRunResult(exitCode: 0);
      expect(result.exitCode, 0);
    });
  });
}
