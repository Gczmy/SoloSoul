import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/ocr_result.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';

// =============================================================================
// MRZ Vault Service
// =============================================================================

/// 将解析后的 MRZ 数据保存为 Vault 中的结构化对象。
///
/// 支持自动判断证件类型：
/// - `P`（护照）→ `travel_passport`
/// - `I` / `C` / `A`（身份证/居留证/其他）→ `profile_id_card`
class MrzVaultService {
  MrzVaultService._();

  /// 根据 MRZ 数据创建对应的 Vault 对象。
  ///
  /// [ref] 用于访问 [unifiedObjectProvider]。
  /// [mrzData] 为已解析的 MRZ 结构化数据。
  ///
  /// 返回 `(success, message)` 元组。
  static Future<({bool success, String message})> saveMrzToVault(
    WidgetRef ref, {
    required MrzData mrzData,
  }) async {
    final notifier = ref.read(unifiedObjectProvider.notifier);
    final docType = mrzData.documentType;

    if (docType.startsWith('P')) {
      return _createPassport(notifier, mrzData);
    } else if (docType.startsWith('I') ||
        docType.startsWith('C') ||
        docType.startsWith('A')) {
      return _createIdCard(notifier, mrzData);
    } else {
      // 未知类型，默认当作护照处理
      return _createPassport(notifier, mrzData);
    }
  }

  // ---------------------------------------------------------------------------
  // Passport
  // ---------------------------------------------------------------------------

  static Future<({bool success, String message})> _createPassport(
    UnifiedObjectNotifier notifier,
    MrzData mrz,
  ) async {
    final properties = <String, PropertyValue>{
      'title': TextProperty(
        text: mrz.documentType,
        sensitivity: SensitivityLevel.public,
      ),
      'country': TextProperty(
        text: mrz.country,
        sensitivity: SensitivityLevel.public,
      ),
      'countryCode': TextProperty(
        text: mrz.country,
        sensitivity: SensitivityLevel.public,
      ),
      'number': TextProperty(
        text: mrz.documentNumber,
        sensitivity: SensitivityLevel.critical,
      ),
      'holderName': TextProperty(
        text: '${mrz.surname} ${mrz.givenNames}'.trim(),
        sensitivity: SensitivityLevel.public,
      ),
      'nationality': TextProperty(
        text: mrz.nationality,
        sensitivity: SensitivityLevel.public,
      ),
      'dateOfBirth': TextProperty(
        text: mrz.dateOfBirth,
        sensitivity: SensitivityLevel.sensitive,
      ),
      'sex': TextProperty(
        text: mrz.sex,
        sensitivity: SensitivityLevel.public,
      ),
      'expiryDate': TextProperty(
        text: mrz.expiryDate,
        sensitivity: SensitivityLevel.sensitive,
      ),
    };

    final success = await notifier.createObject(
      name: '${mrz.surname} ${mrz.givenNames}'.trim(),
      typeId: 'travel_passport',
      iconName: 'book',
      parentId: DefaultSectionIds.passport,
      properties: properties,
    );

    if (success) {
      return (success: true, message: 'Passport saved: ${mrz.documentNumber}');
    }
    return (success: false, message: 'Failed to save passport');
  }

  // ---------------------------------------------------------------------------
  // ID Card
  // ---------------------------------------------------------------------------

  static Future<({bool success, String message})> _createIdCard(
    UnifiedObjectNotifier notifier,
    MrzData mrz,
  ) async {
    final properties = <String, PropertyValue>{
      'title': TextProperty(
        text: mrz.documentType,
        sensitivity: SensitivityLevel.public,
      ),
      'number': TextProperty(
        text: mrz.documentNumber,
        sensitivity: SensitivityLevel.critical,
      ),
      'holderName': TextProperty(
        text: '${mrz.surname} ${mrz.givenNames}'.trim(),
        sensitivity: SensitivityLevel.public,
      ),
      'country': TextProperty(
        text: mrz.country,
        sensitivity: SensitivityLevel.public,
      ),
      'dateOfBirth': TextProperty(
        text: mrz.dateOfBirth,
        sensitivity: SensitivityLevel.sensitive,
      ),
      'sex': TextProperty(
        text: mrz.sex,
        sensitivity: SensitivityLevel.public,
      ),
      'expiryDate': TextProperty(
        text: mrz.expiryDate,
        sensitivity: SensitivityLevel.sensitive,
      ),
    };

    final success = await notifier.createObject(
      name: '${mrz.surname} ${mrz.givenNames}'.trim(),
      typeId: 'profile_id_card',
      iconName: 'badge',
      parentId: DefaultSectionIds.idCard,
      properties: properties,
    );

    if (success) {
      return (success: true, message: 'ID Card saved: ${mrz.documentNumber}');
    }
    return (success: false, message: 'Failed to save ID card');
  }
}
