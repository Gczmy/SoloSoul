import 'package:flutter/material.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/models/smart_ocr_result.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/document_field_extractor.dart';
import 'package:solosoul_flutter/core/services/mrz_vault_service.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/widgets/ocr_scanner_sheet.dart';

/// 通用文档扫描按钮。
///
/// 点击后打开通用 OCR 扫描器，支持：
/// - 智能 MRZ 检测（护照/身份证自动识别并保存到对应分区）
/// - 通用文本 OCR（普通文档保存为 Note）
class ScanDocumentButton extends ConsumerWidget {
  /// 扫描后创建的对象默认放入的 Vault section ID。
  /// 例如 `DefaultSectionIds.employment` 让简历自动归入 Employment section。
  final String? parentId;

  const ScanDocumentButton({super.key, this.parentId});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Card(
      margin: const EdgeInsets.symmetric(horizontal: 16),
      child: InkWell(
        onTap: () => _showScanner(context, ref),
        borderRadius: BorderRadius.circular(12),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
          child: Row(
            children: [
              Container(
                padding: const EdgeInsets.all(10),
                decoration: BoxDecoration(
                  color: Theme.of(context).colorScheme.primaryContainer,
                  borderRadius: BorderRadius.circular(10),
                ),
                child: Icon(
                  Icons.document_scanner_outlined,
                  color: Theme.of(context).colorScheme.onPrimaryContainer,
                ),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      AppLocalizations.of(context).ocrScanDocument,
                      style: Theme.of(context).textTheme.titleMedium?.copyWith(
                            fontWeight: FontWeight.w600,
                          ),
                    ),
                    const SizedBox(height: 2),
                    Text(
                      AppLocalizations.of(context).ocrScanDescription,
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                            color: Theme.of(context).colorScheme.onSurfaceVariant,
                          ),
                    ),
                  ],
                ),
              ),
              Icon(
                Icons.chevron_right,
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
            ],
          ),
        ),
      ),
    ).animate().fadeIn(duration: 300.ms);
  }

  Future<void> _showScanner(BuildContext context, WidgetRef ref) async {
    final result = await showModalBottomSheet<SmartOcrResult?>(
      context: context,
      isScrollControlled: true,
      backgroundColor: Colors.transparent,
      builder: (context) => const OcrScannerSheet(),
    );

    if (result == null || !context.mounted) return;

    switch (result) {
      case SmartOcrMrzResult():
        final outcome = await MrzVaultService.saveMrzToVault(
          ref,
          mrzData: result.mrzData,
        );
        if (context.mounted) {
          showOverlaySnackBar(
            context,
            content: outcome.message,
            type: outcome.success ? SnackBarType.success : SnackBarType.error,
          );
        }

      case SmartOcrTextResult():
        final (saved, message) = await _saveTextResultWithMessage(
          context, ref, result,
        );
        if (context.mounted) {
          showOverlaySnackBar(
            context,
            content: message,
            type: saved ? SnackBarType.success : SnackBarType.error,
          );
        }
    }
  }

  Future<(bool, String)> _saveTextResultWithMessage(
    BuildContext context,
    WidgetRef ref,
    SmartOcrTextResult result,
  ) async {
    final l10n = AppLocalizations.of(context);
    final notifier = ref.read(unifiedObjectProvider.notifier);
    final rawText = result.ocrResult.rawText;
    final extraction = result.extraction;
    if (rawText.trim().isEmpty) return (false, l10n.ocrNoTextDetected);

    return switch (extraction.documentType) {
      'business_card' => (
          await _saveBusinessCard(notifier, extraction, rawText, l10n),
          l10n.ocrBusinessCardSaved,
        ),
      'invoice' => (
          await _saveInvoice(notifier, extraction, rawText, l10n),
          l10n.ocrInvoiceSaved,
        ),
      'resume' => await _saveResume(notifier, extraction, rawText, l10n),
      _ => (
          await _saveGenericDocument(notifier, rawText, l10n),
          l10n.ocrDocumentSavedAsNote,
        ),
    };
  }

  Future<bool> _saveBusinessCard(
    UnifiedObjectNotifier notifier,
    ExtractionResult extraction,
    String rawText,
    AppLocalizations l10n,
  ) async {
    final fields = extraction.fields;
    final name = fields['name']?.value ?? l10n.ocrBusinessCard;
    final phoneField = fields['phone'];
    final emailField = fields['email'];
    return notifier.createObject(
      name: name,
      typeId: 'contact',
      iconName: 'contact_mail',
      parentId: parentId,
      properties: {
        if (phoneField != null)
          'phone': TextProperty(
            text: phoneField.value,
            sensitivity: SensitivityLevel.internal,
          ),
        if (emailField != null)
          'email': TextProperty(
            text: emailField.value,
            sensitivity: SensitivityLevel.internal,
          ),
        'notes': TextProperty(
          text: rawText,
          sensitivity: SensitivityLevel.internal,
        ),
      },
    );
  }

  Future<bool> _saveInvoice(
    UnifiedObjectNotifier notifier,
    ExtractionResult extraction,
    String rawText,
    AppLocalizations l10n,
  ) async {
    final fields = extraction.fields;
    final invNo = fields['invoice_number']?.value;
    final total = fields['total']?.value;
    var name = l10n.ocrInvoice;
    if (invNo != null) name = '${l10n.ocrInvoice} $invNo';
    if (total != null) name = '$name — $total';

    return notifier.createObject(
      name: name,
      typeId: 'note',
      iconName: 'receipt',
      parentId: parentId,
      properties: {
        'content': TextProperty(
          text: rawText,
          sensitivity: SensitivityLevel.internal,
        ),
      },
    );
  }

  /// 将文本按第一行与剩余部分拆分，用于填充 nameExtractor 识别的字段。
  (String, String) _splitFirstLine(String text) {
    final lines = text.split('\n').where((l) => l.trim().isNotEmpty).toList();
    if (lines.isEmpty) return ('', '');
    final first = lines.first.trim();
    final rest = lines.skip(1).join('\n').trim();
    return (first, rest);
  }

  Future<(bool, String)> _saveResume(
    UnifiedObjectNotifier notifier,
    ExtractionResult extraction,
    String rawText,
    AppLocalizations l10n,
  ) async {
    final fields = extraction.fields;
    final name = fields['name']?.value ?? l10n.ocrResume;
    var successCount = 0;
    var failCount = 0;

    // ── 1. 顶层简历摘要（联系信息 + 原始文本）──
    final summaryProps = <String, PropertyValue>{
      'content': TextProperty(
        text: rawText,
        sensitivity: SensitivityLevel.internal,
      ),
    };
    final emailField = fields['email'];
    if (emailField != null) {
      summaryProps['email'] = TextProperty(
        text: emailField.value,
        sensitivity: SensitivityLevel.internal,
      );
    }
    final phoneField = fields['phone'];
    if (phoneField != null) {
      summaryProps['phone'] = TextProperty(
        text: phoneField.value,
        sensitivity: SensitivityLevel.internal,
      );
    }
    final linkedinField = fields['linkedin'];
    if (linkedinField != null) {
      summaryProps['url'] = TextProperty(
        text: linkedinField.value,
        sensitivity: SensitivityLevel.internal,
      );
    }

    final summaryId = await notifier.createObjectAndReturnId(
      name: name,
      typeId: 'note',
      iconName: 'badge',
      parentId: parentId,
      properties: summaryProps,
    );
    if (summaryId != null) {
      successCount++;
    } else {
      failCount++;
    }

    // ── 2. 核心 section 映射到对应专业类型与属性 ──
    final sectionMappings = <String, Map<String, PropertyValue> Function(String)>{
      'education': (text) {
        final (first, rest) = _splitFirstLine(text);
        return {
          'institution': TextProperty(text: first, sensitivity: SensitivityLevel.internal),
          'field': TextProperty(text: rest, sensitivity: SensitivityLevel.internal),
        };
      },
      'work_experience': (text) {
        final (first, rest) = _splitFirstLine(text);
        return {
          'company': TextProperty(text: first, sensitivity: SensitivityLevel.internal),
          'responsibilities': TextProperty(text: rest, sensitivity: SensitivityLevel.internal),
        };
      },
      'skills': (text) => {
        'name': TextProperty(text: text, sensitivity: SensitivityLevel.internal),
      },
      'languages': (text) => {
        'name': TextProperty(text: text, sensitivity: SensitivityLevel.internal),
      },
      'awards': (text) {
        final (first, rest) = _splitFirstLine(text);
        return {
          'title': TextProperty(text: first, sensitivity: SensitivityLevel.internal),
          'description': TextProperty(text: rest, sensitivity: SensitivityLevel.internal),
        };
      },
    };

    final sectionTargets = <String, ({String typeId, String sectionId, String iconName})>{
      'education': (typeId: '__preset_education', sectionId: DefaultSectionIds.education, iconName: 'school'),
      'work_experience': (typeId: '__preset_employment', sectionId: DefaultSectionIds.employment, iconName: 'work'),
      'skills': (typeId: '__preset_skill', sectionId: DefaultSectionIds.skill, iconName: 'stars'),
      'languages': (typeId: '__preset_language', sectionId: DefaultSectionIds.language, iconName: 'language'),
      'awards': (typeId: '__preset_award', sectionId: DefaultSectionIds.award, iconName: 'emoji_events'),
    };

    for (final entry in sectionMappings.entries) {
      final field = fields[entry.key];
      if (field == null) continue;
      final target = sectionTargets[entry.key]!;
      final id = await notifier.createObjectAndReturnId(
        name: entry.key[0].toUpperCase() +
            entry.key.substring(1).replaceAll('_', ' '),
        typeId: target.typeId,
        iconName: target.iconName,
        parentId: target.sectionId,
        properties: entry.value(field.value),
      );
      if (id != null) {
        successCount++;
      } else {
        failCount++;
      }
    }

    // ── 3. 其他 section 映射到最合适的类型与分区 ──
    final otherMappings = <String, ({String typeId, String sectionId, String iconName, Map<String, PropertyValue> Function(String) builder})>{
      'publications': (
        typeId: '__preset_award',
        sectionId: DefaultSectionIds.award,
        iconName: 'emoji_events',
        builder: (text) {
          final (first, rest) = _splitFirstLine(text);
          return {
            'title': TextProperty(text: first, sensitivity: SensitivityLevel.internal),
            'description': TextProperty(text: rest, sensitivity: SensitivityLevel.internal),
          };
        },
      ),
      'projects': (
        typeId: '__preset_employment',
        sectionId: DefaultSectionIds.employment,
        iconName: 'work',
        builder: (text) {
          final (first, rest) = _splitFirstLine(text);
          return {
            'company': TextProperty(text: first, sensitivity: SensitivityLevel.internal),
            'responsibilities': TextProperty(text: rest, sensitivity: SensitivityLevel.internal),
          };
        },
      ),
      'certifications': (
        typeId: '__preset_education',
        sectionId: DefaultSectionIds.education,
        iconName: 'school',
        builder: (text) {
          final (first, rest) = _splitFirstLine(text);
          return {
            'institution': TextProperty(text: first, sensitivity: SensitivityLevel.internal),
            'field': TextProperty(text: rest, sensitivity: SensitivityLevel.internal),
          };
        },
      ),
      'research': (
        typeId: '__preset_award',
        sectionId: DefaultSectionIds.award,
        iconName: 'emoji_events',
        builder: (text) {
          final (first, rest) = _splitFirstLine(text);
          return {
            'title': TextProperty(text: first, sensitivity: SensitivityLevel.internal),
            'description': TextProperty(text: rest, sensitivity: SensitivityLevel.internal),
          };
        },
      ),
      'summary': (
        typeId: 'note',
        sectionId: parentId ?? '',
        iconName: 'note',
        builder: (text) => {
          'content': TextProperty(text: text, sensitivity: SensitivityLevel.internal),
        },
      ),
      'interests': (
        typeId: 'note',
        sectionId: parentId ?? '',
        iconName: 'note',
        builder: (text) => {
          'content': TextProperty(text: text, sensitivity: SensitivityLevel.internal),
        },
      ),
    };

    for (final entry in otherMappings.entries) {
      final field = fields[entry.key];
      if (field == null) continue;
      final mapping = entry.value;
      final id = await notifier.createObjectAndReturnId(
        name: entry.key[0].toUpperCase() + entry.key.substring(1),
        typeId: mapping.typeId,
        iconName: mapping.iconName,
        parentId: mapping.sectionId.isEmpty ? parentId : mapping.sectionId,
        properties: mapping.builder(field.value),
      );
      if (id != null) {
        successCount++;
      } else {
        failCount++;
      }
    }

    final saved = successCount > 0;
    final message = switch ((successCount, failCount)) {
      (0, 0) => l10n.ocrNoResumeSections,
      (1, 0) => l10n.ocrResumeSaved,
      (_, 0) => l10n.ocrResumeSavedSections(successCount),
      (_, _) => l10n.ocrSavedSectionsFailed(successCount, failCount),
    };
    return (saved, message);
  }

  Future<bool> _saveGenericDocument(
    UnifiedObjectNotifier notifier,
    String rawText,
    AppLocalizations l10n,
  ) async {
    var name = rawText.trim().replaceAll('\n', ' ');
    if (name.length > 30) name = '${name.substring(0, 30).trimRight()}...';
    if (name.isEmpty) name = l10n.ocrScannedDocument;

    return notifier.createObject(
      name: name,
      typeId: 'note',
      iconName: 'note',
      parentId: parentId,
      properties: {
        'content': TextProperty(
          text: rawText,
          sensitivity: SensitivityLevel.internal,
        ),
      },
    );
  }
}
