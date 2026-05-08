import 'dart:typed_data';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/ocr_result.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/attachment_storage_service.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/core/utils/mrz_date_utils.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
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
    Uint8List? imageBytes,
    bool saveImage = false,
  }) async {
    final notifier = ref.read(unifiedObjectProvider.notifier);
    final docType = mrzData.documentType;

    if (docType.startsWith('V')) {
      return _createVisa(
        ref,
        notifier,
        mrzData,
        imageBytes: imageBytes,
        saveImage: saveImage,
      );
    } else if (docType.startsWith('P')) {
      return _createPassport(
        ref,
        notifier,
        mrzData,
        imageBytes: imageBytes,
        saveImage: saveImage,
      );
    } else if (docType.startsWith('I') ||
        docType.startsWith('C') ||
        docType.startsWith('A')) {
      return _createIdCard(
        ref,
        notifier,
        mrzData,
        imageBytes: imageBytes,
        saveImage: saveImage,
      );
    } else {
      // 未知类型，默认当作护照处理
      return _createPassport(
        ref,
        notifier,
        mrzData,
        imageBytes: imageBytes,
        saveImage: saveImage,
      );
    }
  }

  // ---------------------------------------------------------------------------
  // Passport
  // ---------------------------------------------------------------------------

  static Future<({bool success, String message})> _createPassport(
    WidgetRef ref,
    UnifiedObjectNotifier notifier,
    MrzData mrz, {
    Uint8List? imageBytes,
    bool saveImage = false,
  }) async {
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
      'dateOfBirth': DateProperty(
        isoDate: parseMrzDate(mrz.dateOfBirth),
        sensitivity: SensitivityLevel.sensitive,
      ),
      'sex': TextProperty(
        text: mrz.sex,
        sensitivity: SensitivityLevel.public,
      ),
      'expiryDate': DateProperty(
        isoDate: parseMrzDate(mrz.expiryDate),
        sensitivity: SensitivityLevel.sensitive,
      ),
    };

    final objectId = await notifier.createObjectAndReturnId(
      name: '${mrz.surname} ${mrz.givenNames}'.trim(),
      typeId: 'travel_passport',
      iconName: 'book',
      parentId: DefaultSectionIds.passport,
      properties: properties,
    );

    if (objectId == null) {
      return (success: false, message: 'Failed to save passport');
    }

    // Save attachment if requested
    if (saveImage && imageBytes != null) {
      await _saveAttachment(
        ref: ref,
        notifier: notifier,
        objectId: objectId,
        fileName: 'passport_scan_${DateTime.now().millisecondsSinceEpoch}.jpg',
        bytes: imageBytes,
      );
    }

    return (success: true, message: 'Passport saved: ${mrz.documentNumber}');
  }

  // ---------------------------------------------------------------------------
  // Visa
  // ---------------------------------------------------------------------------

  static Future<({bool success, String message})> _createVisa(
    WidgetRef ref,
    UnifiedObjectNotifier notifier,
    MrzData mrz, {
    Uint8List? imageBytes,
    bool saveImage = false,
  }) async {
    final properties = <String, PropertyValue>{
      'title': TextProperty(
        text: mrz.documentType,
        sensitivity: SensitivityLevel.public,
      ),
      'country': TextProperty(
        text: mrz.country,
        sensitivity: SensitivityLevel.public,
      ),
      'visaType': TextProperty(
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
      'issueDate': DateProperty(
        isoDate: parseMrzDate(mrz.dateOfBirth),
        sensitivity: SensitivityLevel.internal,
      ),
      'expiryDate': DateProperty(
        isoDate: parseMrzDate(mrz.expiryDate),
        sensitivity: SensitivityLevel.internal,
      ),
    };

    final objectId = await notifier.createObjectAndReturnId(
      name: 'Visa ${mrz.country}',
      typeId: 'travel_visa',
      iconName: 'assignment_ind',
      parentId: DefaultSectionIds.visa,
      properties: properties,
    );

    if (objectId == null) {
      return (success: false, message: 'Failed to save visa');
    }

    if (saveImage && imageBytes != null) {
      await _saveAttachment(
        ref: ref,
        notifier: notifier,
        objectId: objectId,
        fileName: 'visa_scan_${DateTime.now().millisecondsSinceEpoch}.jpg',
        bytes: imageBytes,
      );
    }

    return (success: true, message: 'Visa saved: ${mrz.documentNumber}');
  }

  // ---------------------------------------------------------------------------
  // ID Card
  // ---------------------------------------------------------------------------

  static Future<({bool success, String message})> _createIdCard(
    WidgetRef ref,
    UnifiedObjectNotifier notifier,
    MrzData mrz, {
    Uint8List? imageBytes,
    bool saveImage = false,
  }) async {
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
      'dateOfBirth': DateProperty(
        isoDate: parseMrzDate(mrz.dateOfBirth),
        sensitivity: SensitivityLevel.sensitive,
      ),
      'sex': TextProperty(
        text: mrz.sex,
        sensitivity: SensitivityLevel.public,
      ),
      'expiryDate': DateProperty(
        isoDate: parseMrzDate(mrz.expiryDate),
        sensitivity: SensitivityLevel.sensitive,
      ),
    };

    final objectId = await notifier.createObjectAndReturnId(
      name: '${mrz.surname} ${mrz.givenNames}'.trim(),
      typeId: 'profile_id_card',
      iconName: 'badge',
      parentId: DefaultSectionIds.idCard,
      properties: properties,
    );

    if (objectId == null) {
      return (success: false, message: 'Failed to save ID card');
    }

    // Save attachment if requested
    if (saveImage && imageBytes != null) {
      await _saveAttachment(
        ref: ref,
        notifier: notifier,
        objectId: objectId,
        fileName: 'id_card_scan_${DateTime.now().millisecondsSinceEpoch}.jpg',
        bytes: imageBytes,
      );
    }

    return (success: true, message: 'ID Card saved: ${mrz.documentNumber}');
  }

  static Future<void> _saveAttachment({
    required WidgetRef ref,
    required UnifiedObjectNotifier notifier,
    required String objectId,
    required String fileName,
    required Uint8List bytes,
  }) async {
    try {
      final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
      if (accountId == null) {
        SoloLog.w('MrzVaultService', 'No account selected, skipping attachment save');
        return;
      }

      final attachment = await AttachmentStorageService().saveAttachment(
        accountId: accountId,
        fileName: fileName,
        bytes: bytes,
      );

      final objects = ref.read(unifiedObjectProvider).objects;
      final object = objects.firstWhere((o) => o.id == objectId);
      final updated = object.copyWith(
        attachments: [...object.attachments, attachment],
      );

      await notifier.updateObject(objectId, attachments: updated.attachments);
      SoloLog.d('MrzVaultService', 'Attachment saved: ${attachment.fileName}');
    } on Exception catch (e, st) {
      SoloLog.e('MrzVaultService', 'Failed to save attachment', e, st);
    }
  }
}
