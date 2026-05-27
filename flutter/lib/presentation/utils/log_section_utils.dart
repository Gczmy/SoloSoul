import 'package:solosoul_flutter/presentation/models/operation_log_models.dart'
    show LogSection;

/// Map typeId to LogSection for operation logging.
LogSection? logSectionForTypeId(String typeId) {
  return switch (typeId) {
    '__preset_identity' => LogSection.identity,
    '__preset_contact' => LogSection.contactInformation,
    '__preset_identity_document' => LogSection.idCard,
    '__preset_address' => LogSection.address,
    '__preset_passport' => LogSection.passport,
    '__preset_visa' => LogSection.visa,
    '__preset_travel_history' => LogSection.travelHistory,
    '__preset_bank_account' => LogSection.bankAccount,
    '__preset_payment_card' => LogSection.card,
    '__preset_tax_id' => LogSection.financial,
    '__preset_education' => LogSection.education,
    '__preset_employment' => LogSection.employment,
    '__preset_skill' => LogSection.skill,
    '__preset_language' => LogSection.language,
    '__preset_award' => LogSection.professional,
    _ => null,
  };
}
