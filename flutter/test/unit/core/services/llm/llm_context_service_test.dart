import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/profile_data.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/llm/llm_context_service.dart';
import 'package:solosoul_flutter/core/services/llm/llm_prompt_templates.dart';

void main() {
  group('LlmPromptTemplates.chatSystemPrompt', () {
    test('includes all required sections', () {
      final prompt = LlmPromptTemplates.chatSystemPrompt(
        appVersion: '1.2.3',
        platform: 'macOS',
        language: 'zh',
        userPublicInfo: {
          'Identity': [
            {'Full Name': '张三', 'Gender': '男'}
          ]
        },
        preferences: {
          '应用语言': '中文',
          '自动锁定': '5 分钟',
        },
        usageStats: {
          'currentModel': 'gpt-4o-mini',
          'currentProvider': 'openai',
          'sessionCalls': 3,
          'sessionTokens': 1500,
          'accountCalls': 42,
          'accountTokens': 25000,
        },
      );

      expect(prompt, contains('SoloSoul'));
      expect(prompt, contains('1.2.3'));
      expect(prompt, contains('macOS'));
      expect(prompt, contains('张三'));
      expect(prompt, contains('男'));
      expect(prompt, contains('gpt-4o-mini'));
      expect(prompt, contains('1500'));
      expect(prompt, contains('25000'));
      expect(prompt, contains('请使用用户提问的语言来回答'));
    });

    test('handles empty user info gracefully', () {
      final prompt = LlmPromptTemplates.chatSystemPrompt(
        appVersion: '1.0.0',
        platform: 'iOS',
        language: 'en',
        userPublicInfo: {},
        preferences: {},
        usageStats: {
          'currentModel': 'qwen2.5:1.5b',
          'currentProvider': 'ollama',
          'sessionCalls': 0,
          'sessionTokens': 0,
          'accountCalls': 0,
          'accountTokens': 0,
        },
      );

      expect(prompt, contains('SoloSoul'));
      expect(prompt, isNot(contains('用户公开档案')));
      expect(prompt, contains('qwen2.5:1.5b'));
    });

    test('includes language adaptivity instruction', () {
      final prompt = LlmPromptTemplates.chatSystemPrompt(
        appVersion: '1.0.0',
        platform: 'Android',
        language: 'zh',
        userPublicInfo: {},
        preferences: {},
        usageStats: {},
      );

      expect(prompt, contains('请使用用户提问的语言来回答'));
    });
  });

  group('LlmContextService cache key', () {
    test('same profile produces same cache key', () {
      final profile = _makeProfile([
        _makeObject(
          typeId: '__preset_identity',
          properties: {
            'fullName': const TextProperty(text: 'Alice', sensitivity: SensitivityLevel.public),
          },
        ),
      ]);

      final key1 = LlmContextServiceTestHelper.buildCacheKey('acc1', profile);
      final key2 = LlmContextServiceTestHelper.buildCacheKey('acc1', profile);
      expect(key1, equals(key2));
    });

    test('different accounts produce different cache keys', () {
      final profile = _makeProfile([
        _makeObject(
          typeId: '__preset_identity',
          properties: {'fullName': const TextProperty(text: 'Alice', sensitivity: SensitivityLevel.public)},
        ),
      ]);

      final key1 = LlmContextServiceTestHelper.buildCacheKey('acc1', profile);
      final key2 = LlmContextServiceTestHelper.buildCacheKey('acc2', profile);
      expect(key1, isNot(equals(key2)));
    });

    test('different profile contents produce different cache keys', () {
      final profile1 = _makeProfile([
        _makeObject(
          typeId: '__preset_identity',
          properties: {'fullName': const TextProperty(text: 'Alice', sensitivity: SensitivityLevel.public)},
          updatedAt: 1000,
        ),
      ]);
      final profile2 = _makeProfile([
        _makeObject(
          typeId: '__preset_identity',
          properties: {'fullName': const TextProperty(text: 'Bob', sensitivity: SensitivityLevel.public)},
          updatedAt: 2000,
        ),
      ]);

      final key1 = LlmContextServiceTestHelper.buildCacheKey('acc1', profile1);
      final key2 = LlmContextServiceTestHelper.buildCacheKey('acc1', profile2);
      expect(key1, isNot(equals(key2)));
    });
  });

  group('LlmContextService public info extraction', () {
    test('only public properties are included', () {
      final profile = _makeProfile([
        _makeObject(
          typeId: '__preset_identity',
          properties: {
            'fullName': const TextProperty(text: 'Alice', sensitivity: SensitivityLevel.public),
            'dateOfBirth': const TextProperty(text: '1990-01-01', sensitivity: SensitivityLevel.sensitive),
            'nationality': const TextProperty(text: 'CN', sensitivity: SensitivityLevel.sensitive),
          },
        ),
      ]);

      final info = LlmContextServiceTestHelper.extractPublicInfo(profile);

      expect(info.keys, contains('Identity'));
      final identityObjects = info['Identity']!;
      expect(identityObjects.length, equals(1));
      final props = identityObjects.first;
      expect(props.keys, contains('Full Name'));
      expect(props['Full Name'], equals('Alice'));
      expect(props.containsKey('Date Of Birth'), isFalse);
      expect(props.containsKey('Nationality'), isFalse);
    });

    test('critical properties are excluded', () {
      final profile = _makeProfile([
        _makeObject(
          typeId: '__preset_bank_account',
          properties: {
            'title': const TextProperty(text: 'Main Account', sensitivity: SensitivityLevel.public),
            'accountNumber': const TextProperty(text: '1234567890', sensitivity: SensitivityLevel.critical),
          },
        ),
      ]);

      final info = LlmContextServiceTestHelper.extractPublicInfo(profile);
      final bankObjects = info['Bank Account']!;
      final props = bankObjects.first;
      expect(props.keys, contains('Title'));
      expect(props.containsKey('Account Number'), isFalse);
    });

    test('deleted objects are skipped', () {
      final profile = _makeProfile([
        _makeObject(
          typeId: '__preset_identity',
          properties: {'fullName': const TextProperty(text: 'Deleted', sensitivity: SensitivityLevel.public)},
          isDeleted: true,
        ),
      ]);

      final info = LlmContextServiceTestHelper.extractPublicInfo(profile);
      expect(info.isEmpty, isTrue);
    });

    test('limits objects per type', () {
      final objects = <UnifiedObject>[];
      for (var i = 0; i < 5; i++) {
        objects.add(_makeObject(
          typeId: '__preset_education',
          properties: {
            'institution': TextProperty(text: 'University $i', sensitivity: SensitivityLevel.public),
          },
        ));
      }
      final profile = _makeProfile(objects);

      final info = LlmContextServiceTestHelper.extractPublicInfo(profile);
      expect(info['Education']!.length, lessThanOrEqualTo(3));
    });

    test('limits properties per object', () {
      final properties = <String, PropertyValue>{};
      for (var i = 0; i < 12; i++) {
        properties['field$i'] = TextProperty(text: 'value$i', sensitivity: SensitivityLevel.public);
      }
      final profile = _makeProfile([
        _makeObject(typeId: '__preset_skill', properties: properties),
      ]);

      final info = LlmContextServiceTestHelper.extractPublicInfo(profile);
      final skillObjects = info['Skill']!;
      expect(skillObjects.first.length, lessThanOrEqualTo(8));
    });

    test('truncates long values', () {
      final longText = 'A' * 200;
      final profile = _makeProfile([
        _makeObject(
          typeId: '__preset_identity',
          properties: {
            'fullName': TextProperty(text: longText, sensitivity: SensitivityLevel.public),
          },
        ),
      ]);

      final info = LlmContextServiceTestHelper.extractPublicInfo(profile);
      final value = info['Identity']!.first['Full Name']!;
      expect(value.length, lessThanOrEqualTo(103)); // 100 + '...'
      expect(value.endsWith('...'), isTrue);
    });

    test('groups by typeId', () {
      final profile = _makeProfile([
        _makeObject(
          typeId: '__preset_identity',
          properties: {'fullName': const TextProperty(text: 'Alice', sensitivity: SensitivityLevel.public)},
        ),
        _makeObject(
          typeId: '__preset_education',
          properties: {'institution': const TextProperty(text: 'MIT', sensitivity: SensitivityLevel.public)},
        ),
      ]);

      final info = LlmContextServiceTestHelper.extractPublicInfo(profile);
      expect(info.keys, contains('Identity'));
      expect(info.keys, contains('Education'));
    });
  });

  group('LlmContextService helpers', () {
    test('typeDisplayName strips __preset_ prefix', () {
      expect(LlmContextServiceTestHelper.typeDisplayName('__preset_identity'), equals('Identity'));
      expect(LlmContextServiceTestHelper.typeDisplayName('__preset_bank_account'), equals('Bank Account'));
      expect(LlmContextServiceTestHelper.typeDisplayName('custom_type'), equals('Custom Type'));
    });

    test('propertyValueToString handles all types', () {
      expect(LlmContextServiceTestHelper.propertyValueToString(const TextProperty(text: 'hello')), equals('hello'));
      expect(LlmContextServiceTestHelper.propertyValueToString(const NumberProperty(value: 42.5)), equals('42.5'));
      expect(LlmContextServiceTestHelper.propertyValueToString(const DateProperty(isoDate: '2024-01-01')), equals('2024-01-01'));
      expect(LlmContextServiceTestHelper.propertyValueToString(const CheckboxProperty(checked: true)), equals('是'));
      expect(LlmContextServiceTestHelper.propertyValueToString(const SelectProperty(options: [], selectedId: 'opt1')), equals('opt1'));
      expect(LlmContextServiceTestHelper.propertyValueToString(const MultiSelectProperty(options: [], selectedIds: ['a', 'b'])), equals('a, b'));
      expect(LlmContextServiceTestHelper.propertyValueToString(const RelationProperty(targetObjectId: 'obj1')), equals('obj1'));
      expect(LlmContextServiceTestHelper.propertyValueToString(const UrlProperty(url: 'https://example.com')), equals('https://example.com'));
    });

    test('trimToLimit enforces max chars', () {
      final longText = 'A' * 2500;
      final trimmed = LlmContextServiceTestHelper.trimToLimit(longText, 2000);
      expect(trimmed.length, lessThanOrEqualTo(2020)); // account for truncation message
    });
  });
}

// =============================================================================
// Test Helpers
// =============================================================================

ProfileData _makeProfile(List<UnifiedObject> objects) {
  return ProfileData(
    unifiedObjects: UnifiedObjectData(objects: objects),
    schemaVersion: 6,
  );
}

UnifiedObject _makeObject({
  required String typeId,
  required Map<String, PropertyValue> properties,
  bool isDeleted = false,
  int updatedAt = 1000,
}) {
  return UnifiedObject(
    id: 'obj_$updatedAt',
    typeId: typeId,
    name: 'Test',
    properties: properties,
    isDeleted: isDeleted,
    createdAt: updatedAt,
    updatedAt: updatedAt,
  );
}

/// Test helper that exposes private methods of [LlmContextService] for unit testing.
/// These delegates are only used in tests and do not affect production behavior.
class LlmContextServiceTestHelper {
  static String buildCacheKey(String accountId, ProfileData? profile) {
    if (profile?.unifiedObjects == null) return '${accountId}_null';
    final objects = profile!.unifiedObjects!.objects;
    var signature = 0;
    for (final obj in objects) {
      signature += obj.updatedAt;
    }
    return '${accountId}_${objects.length}_$signature';
  }

  static Map<String, List<Map<String, String>>> extractPublicInfo(ProfileData? profile) {
    final result = <String, List<Map<String, String>>>{};
    if (profile == null) return result;

    final objects = profile.unifiedObjects?.objects;
    if (objects == null || objects.isEmpty) return result;

    final byType = <String, List<UnifiedObject>>{};
    for (final obj in objects) {
      if (obj.isDeleted) continue;
      final typeId = obj.typeId ?? 'other';
      byType.putIfAbsent(typeId, () => []).add(obj);
    }

    for (final typeEntry in byType.entries) {
      final typeId = typeEntry.key;
      final typeObjects = typeEntry.value;
      final typeName = _typeDisplayName(typeId);
      final objectList = <Map<String, String>>[];
      final objectsToProcess = typeObjects.take(3);

      for (final obj in objectsToProcess) {
        final props = <String, String>{};
        var propCount = 0;

        for (final propEntry in obj.properties.entries) {
          if (propCount >= 8) break;
          final propValue = propEntry.value;
          if (propValue.sensitivity != SensitivityLevel.public) continue;

          final displayValue = _propertyValueToString(propValue);
          if (displayValue.isEmpty) continue;

          final label = _propertyKeyToLabel(propEntry.key);
          props[label] = _truncate(displayValue, 100);
          propCount++;
        }

        if (props.isNotEmpty) {
          objectList.add(props);
        }
      }

      if (objectList.isNotEmpty) {
        result[typeName] = objectList;
      }
    }

    return result;
  }

  static String typeDisplayName(String typeId) => _typeDisplayName(typeId);
  static String propertyValueToString(PropertyValue value) => _propertyValueToString(value);
  static String trimToLimit(String text, int maxChars) => _trimToLimit(text, maxChars);

  static String _typeDisplayName(String typeId) {
    final clean = typeId.startsWith('__preset_') ? typeId.substring(9) : typeId;
    return clean
        .replaceAll('_', ' ')
        .replaceAllMapped(RegExp(r'([a-z])([A-Z])'), (m) => '${m[1]} ${m[2]}')
        .split(' ')
        .map((w) => w.isEmpty ? w : '${w[0].toUpperCase()}${w.substring(1)}')
        .join(' ');
  }

  static String _propertyKeyToLabel(String key) {
    return key
        .replaceAll('_', ' ')
        .replaceAllMapped(RegExp(r'([a-z])([A-Z])'), (m) => '${m[1]} ${m[2]}')
        .split(' ')
        .map((w) => w.isEmpty ? w : '${w[0].toUpperCase()}${w.substring(1)}')
        .join(' ');
  }

  static String _propertyValueToString(PropertyValue value) {
    return switch (value) {
      TextProperty(:final text) => text,
      NumberProperty(:final value) => value?.toString() ?? '',
      DateProperty(:final isoDate) => isoDate ?? '',
      CheckboxProperty(:final checked) => checked ? '是' : '否',
      SelectProperty(:final selectedId) => selectedId ?? '',
      MultiSelectProperty(:final selectedIds) => selectedIds.join(', '),
      RelationProperty(:final targetObjectId) => targetObjectId ?? '',
      UrlProperty(:final url) => url ?? '',
    };
  }

  static String _truncate(String value, int maxLength) {
    if (value.length <= maxLength) return value;
    return '${value.substring(0, maxLength)}...';
  }

  static String _trimToLimit(String text, int maxChars) {
    if (text.length <= maxChars) return text;
    final cutoff = text.lastIndexOf('\n', maxChars);
    if (cutoff > maxChars * 0.8) {
      return '${text.substring(0, cutoff)}\n\n（部分信息已被截断以控制长度）';
    }
    return '${text.substring(0, maxChars)}...（部分信息已被截断）';
  }
}
