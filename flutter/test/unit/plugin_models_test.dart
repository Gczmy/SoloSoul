import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/plugin_models.dart';

void main() {
  group('PluginRegistry', () {
    test('empty factory creates empty registry', () {
      final registry = PluginRegistry.empty();
      expect(registry.version, '1');
      expect(registry.plugins, isEmpty);
    });

    test('fromJson parses correctly', () {
      final json = {
        'version': '2',
        'updated_at': '2024-01-01T00:00:00Z',
        'plugins': {
          'test_plugin': {
            'name': 'Test Plugin',
            'publisher': 'Test Publisher',
            'latest_version': '1.0.0',
            'versions': {
              '1.0.0': {
                'sha256': 'abc123',
                'plugin_api_version': '1',
                'min_app_version': '1.0.0',
                'max_app_version': '2.0.0',
                'download_url': 'https://example.com/plugin.wasm',
                'released_at': '2024-01-01T00:00:00Z',
              },
            },
          },
        },
      };

      final registry = PluginRegistry.fromJson(json);
      expect(registry.version, '2');
      expect(registry.plugins.length, 1);
      expect(registry.plugins['test_plugin']!.name, 'Test Plugin');
    });

    test('toJson round-trip', () {
      final registry = PluginRegistry(
        version: '1',
        updatedAt: DateTime.utc(2024, 1, 1),
        plugins: {
          'plugin1': PluginRegistryEntry(
            name: 'Plugin One',
            publisher: 'Pub',
            latestVersion: '1.0.0',
            versions: {
              '1.0.0': PluginVersionInfo(
                sha256: 'hash1',
                pluginApiVersion: '1',
                minAppVersion: '1.0.0',
                maxAppVersion: '2.0.0',
                downloadUrl: 'https://example.com/plugin.wasm',
                releasedAt: DateTime.utc(2024, 1, 1),
              ),
            },
          ),
        },
      );

      final json = registry.toJson();
      expect(json['version'], '1');
      expect((json['plugins'] as Map).length, 1);
    });

    test('fromJson handles missing optional fields', () {
      final json = <String, dynamic>{
        'version': '1',
        'plugins': <String, dynamic>{},
      };

      final registry = PluginRegistry.fromJson(json);
      expect(registry.plugins, isEmpty);
    });
  });

  group('PluginRegistryEntry', () {
    test('fromJson parses i18n correctly', () {
      final json = <String, dynamic>{
        'name': 'Plugin',
        'publisher': 'Pub',
        'latest_version': '1.0.0',
        'versions': <String, dynamic>{},
        'i18n': <String, dynamic>{
          'zh': <String, String>{'name': '插件', 'description': '描述'},
          'en': <String, String>{'name': 'Plugin', 'description': 'Description'},
        },
      };

      final entry = PluginRegistryEntry.fromJson(json);
      expect(entry.i18n, isNotNull);
      expect(entry.i18n!['zh']!['name'], '插件');
      expect(entry.i18n!['en']!['description'], 'Description');
    });

    test('toJson includes all fields', () {
      final entry = PluginRegistryEntry(
        name: 'Plugin',
        publisher: 'Pub',
        latestVersion: '1.0.0',
        versions: {},
        description: 'A test plugin',
        homepage: 'https://example.com',
      );

      final json = entry.toJson();
      expect(json['name'], 'Plugin');
      expect(json['description'], 'A test plugin');
      expect(json['homepage'], 'https://example.com');
    });
  });

  group('PluginVersionInfo', () {
    test('fromJson parses correctly', () {
      final json = {
        'sha256': 'abc123',
        'plugin_api_version': '1',
        'min_app_version': '1.0.0',
        'max_app_version': '2.0.0',
        'download_url': 'https://example.com/plugin.wasm',
        'released_at': '2024-06-15T10:30:00Z',
      };

      final info = PluginVersionInfo.fromJson(json);
      expect(info.sha256, 'abc123');
      expect(info.pluginApiVersion, '1');
      expect(info.downloadUrl, 'https://example.com/plugin.wasm');
    });

    test('toJson round-trip', () {
      final info = PluginVersionInfo(
        sha256: 'hash',
        pluginApiVersion: '1',
        minAppVersion: '1.0.0',
        maxAppVersion: '2.0.0',
        downloadUrl: 'https://example.com/plugin.wasm',
        releasedAt: DateTime.utc(2024, 1, 1),
      );

      final json = info.toJson();
      expect(json['sha256'], 'hash');
      expect(json['download_url'], 'https://example.com/plugin.wasm');
    });
  });

  group('InstalledPluginInfo', () {
    test('fromJson parses correctly', () {
      final json = {
        'version': '1.0.0',
        'status': 'installed',
        'installed_at': '2024-01-01T00:00:00Z',
      };

      final info = InstalledPluginInfo.fromJson(json);
      expect(info.version, '1.0.0');
      expect(info.status, 'installed');
      expect(info.installedAt, isNotNull);
    });

    test('toJson round-trip', () {
      final info = InstalledPluginInfo(
        version: '1.0.0',
        status: 'installed',
        installedAt: DateTime.utc(2024, 1, 1),
      );

      final json = info.toJson();
      expect(json['version'], '1.0.0');
      expect(json['status'], 'installed');
    });

    test('fromJson defaults status to installed', () {
      final json = {
        'version': '1.0.0',
      };

      final info = InstalledPluginInfo.fromJson(json);
      expect(info.status, 'installed');
    });
  });
}
