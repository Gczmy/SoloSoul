import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/core/models/ocr_result.dart';

/// Section picker options for MRZ import.
const ocrScannerSectionOptions = [
  ('passport', 'Passport'),
  ('visa', 'Visa'),
  ('id_card', 'ID Card'),
];

/// Get section label from section ID.
String ocrScannerSectionLabel(String sectionId) {
  return ocrScannerSectionOptions
      .firstWhere((o) => o.$1 == sectionId, orElse: () => ('passport', 'Passport'))
      .$2;
}

/// Detect section ID from MRZ document type.
String ocrScannerDetectedSectionId(MrzData mrz) {
  final dt = mrz.documentType;
  if (dt.startsWith('V')) return 'visa';
  if (dt.startsWith('I') || dt.startsWith('C') || dt.startsWith('A')) return 'id_card';
  return 'passport';
}

/// Show section picker dialog.
void showOcrScannerSectionPicker(
  BuildContext context,
  AppLocalizations l10n,
  MrzData mrz,
  String? currentTargetSectionId,
  ValueChanged<String?> onSectionChanged,
) {
  showDialog(
    context: context,
    builder: (ctx) => SimpleDialog(
      title: Text(l10n.workspaceAddSectionButton),
      children: [
        RadioGroup<String>(
          groupValue: currentTargetSectionId ?? ocrScannerDetectedSectionId(mrz),
          onChanged: (v) {
            onSectionChanged(v);
            Navigator.pop(ctx);
          },
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: ocrScannerSectionOptions.map((opt) {
              return RadioListTile<String>(
                value: opt.$1,
                title: Text(opt.$2),
              );
            }).toList(),
          ),
        ),
      ],
    ),
  );
}
