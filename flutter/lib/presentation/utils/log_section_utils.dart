import 'package:solosoul_flutter/presentation/models/operation_log_models.dart'
    show LogSection;

/// Map typeId to LogSection for operation logging.
LogSection? logSectionForTypeId(String typeId) {
  return switch (typeId) {
    'profile_identity' => LogSection.identity,
    'profile_contact' => LogSection.contactInformation,
    'profile_id_card' => LogSection.idCard,
    'profile_address' => LogSection.address,
    'travel_passport' => LogSection.passport,
    'travel_visa' => LogSection.visa,
    'travel_history' => LogSection.travelHistory,
    'financial_bank_account' => LogSection.bankAccount,
    'financial_card' => LogSection.card,
    'financial_tax_id' => LogSection.financial,
    'professional_education' => LogSection.education,
    'professional_employment' => LogSection.employment,
    'professional_skill' => LogSection.skill,
    'professional_language' => LogSection.language,
    'professional_award' => LogSection.professional,
    _ => null,
  };
}
