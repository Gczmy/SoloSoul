import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/core/services/operation_logger.dart';
import 'package:solosoul_flutter/presentation/pages/operation_log_page.dart';

/// Service responsible for change detection and summary generation.
/// This class handles:
/// - Logging identity/travel/financial/professional changes
/// - Summarizing changes into human-readable operation strings
class OperationLogAggregator {
  OperationLogAggregator();

  // ===========================================================================
  // Change Detection and Logging
  // ===========================================================================

  /// Log changes between old and new identity data
  void logIdentityChanges(IdentityData? old, IdentityData? newData) {
    if (old == null && newData != null) {
      OperationLogService.instance.addEntry(
        OperationLogger.logIdentity(
          action: LogAction.create,
          description: 'Created identity profile',
          fieldPath: 'identity',
        ),
      );
      return;
    }
    if (old != null && newData == null) {
      OperationLogService.instance.addEntry(
        OperationLogger.logIdentity(
          action: LogAction.delete,
          description: 'Deleted identity profile',
          fieldPath: 'identity',
        ),
      );
      return;
    }
    if (old == null || newData == null) return;

    if (old.fullName != newData.fullName) {
      final action = old.fullName == null ? LogAction.create : LogAction.update;
      String description;
      if (action == LogAction.update) {
        description = 'Updated fullName: ${old.fullName} → ${newData.fullName}';
      } else {
        description = 'Added fullName: ${newData.fullName}';
      }
      OperationLogService.instance.addEntry(
        OperationLogger.logIdentity(
          action: action,
          description: description,
          fieldPath: 'fullName',
        ),
      );
    }

    final oldContacts = old.contact?.entries ?? [];
    final newContacts = newData.contact?.entries ?? [];
    logIdentityListChanges(
      oldList: oldContacts,
      newList: newContacts,
      section: LogSection.contactInformation,
      itemType: 'contact',
      compareEntry: (oldEntry, newEntry) =>
          oldEntry.title != newEntry.title ||
          oldEntry.type != newEntry.type ||
          oldEntry.value != newEntry.value,
      getLabel: (entry) => entry.title.isNotEmpty ? entry.title : entry.value,
      showDiff: true,
    );

    final oldIdCards = old.idCards ?? [];
    final newIdCards = newData.idCards ?? [];
    logIdentityListChanges(
      oldList: oldIdCards,
      newList: newIdCards,
      section: LogSection.idCard,
      itemType: 'ID card',
      compareEntry: (oldEntry, newEntry) =>
          oldEntry.title != newEntry.title ||
          oldEntry.number != newEntry.number,
      getLabel: (entry) => entry.title ?? entry.number ?? 'ID card',
      showDiff: true,
    );

    final oldAddresses = old.addresses ?? [];
    final newAddresses = newData.addresses ?? [];
    logIdentityListChanges(
      oldList: oldAddresses,
      newList: newAddresses,
      section: LogSection.address,
      itemType: 'address',
      compareEntry: (oldEntry, newEntry) =>
          oldEntry.title != newEntry.title ||
          oldEntry.country != newEntry.country ||
          oldEntry.city != newEntry.city ||
          oldEntry.street != newEntry.street ||
          oldEntry.postalCode != newEntry.postalCode,
      getLabel: (entry) => entry.title ?? 'Address',
      showDiff: true,
    );
  }

  /// Log identity list changes with detailed field comparison
  void logIdentityListChanges<T>({
    required List<T> oldList,
    required List<T> newList,
    required LogSection section,
    required String itemType,
    required bool Function(T oldEntry, T newEntry) compareEntry,
    required String Function(T entry) getLabel,
    bool showDiff = false,
  }) {
    final oldLen = oldList.length;
    final newLen = newList.length;

    if (oldLen < newLen) {
      final count = newLen - oldLen;
      final description = count == 1
          ? 'Added $itemType${newLen > 0 ? ': ${getLabel(newList.first)}' : ''}'
          : 'Added $count $itemType items';
      addLogEntry(section: section, action: LogAction.create, description: description);
    } else if (oldLen > newLen) {
      final count = oldLen - newLen;
      final description = count == 1
          ? 'Deleted $itemType${oldLen > 0 ? ': ${getLabel(oldList.first)}' : ''}'
          : 'Deleted $count $itemType items';
      addLogEntry(section: section, action: LogAction.delete, description: description);
    } else if (oldLen > 0) {
      for (var i = 0; i < oldLen; i++) {
        if (compareEntry(oldList[i], newList[i])) {
          String description;
          if (showDiff) {
            description = 'Updated $itemType: ${getLabel(oldList[i])} → ${getLabel(newList[i])}';
          } else {
            description = 'Updated $itemType: ${getLabel(newList[i])}';
          }
          addLogEntry(section: section, action: LogAction.update, description: description);
        }
      }
    }
  }

  /// Log simple list changes based on count difference
  void logSimpleListChanges({
    required int oldList,
    required int newList,
    required LogSection section,
    required String itemType,
    String? itemLabel,
  }) {
    if (oldList < newList) {
      final count = newList - oldList;
      final description = count == 1
          ? 'Added $itemType${itemLabel != null ? ': $itemLabel' : ''}'
          : 'Added $count $itemType items';
      addLogEntry(section: section, action: LogAction.create, description: description);
    } else if (oldList > newList) {
      final count = oldList - newList;
      final description = count == 1
          ? 'Deleted $itemType${itemLabel != null ? ': $itemLabel' : ''}'
          : 'Deleted $count $itemType items';
      addLogEntry(section: section, action: LogAction.delete, description: description);
    }
  }

  /// Log changes between old and new travel data
  void logTravelChanges(TravelData? old, TravelData? newData) {
    if (old == null && newData != null) {
      addLogEntry(
        section: LogSection.travel,
        action: LogAction.create,
        description: 'Added travel data',
      );
      return;
    }
    if (old != null && newData == null) {
      addLogEntry(
        section: LogSection.travel,
        action: LogAction.delete,
        description: 'Deleted travel data',
      );
      return;
    }
    if (old == null || newData == null) return;

    logSimpleListChanges(
      oldList: old.passports.length,
      newList: newData.passports.length,
      section: LogSection.passport,
      itemType: 'passport',
      itemLabel: newData.passports.isNotEmpty ? newData.passports.first.number : null,
    );

    logSimpleListChanges(
      oldList: old.visas.length,
      newList: newData.visas.length,
      section: LogSection.visa,
      itemType: 'visa',
      itemLabel: newData.visas.isNotEmpty ? newData.visas.first.country : null,
    );

    logSimpleListChanges(
      oldList: old.travelHistory.length,
      newList: newData.travelHistory.length,
      section: LogSection.travelHistory,
      itemType: 'travel history entry',
      itemLabel: newData.travelHistory.isNotEmpty ? newData.travelHistory.first.destination : null,
    );
  }

  /// Log changes between old and new financial data
  void logFinancialChanges(FinancialData? old, FinancialData? newData) {
    if (old == null && newData != null) {
      addLogEntry(
        section: LogSection.financial,
        action: LogAction.create,
        description: 'Added financial data',
      );
      return;
    }
    if (old != null && newData == null) {
      addLogEntry(
        section: LogSection.financial,
        action: LogAction.delete,
        description: 'Deleted financial data',
      );
      return;
    }
    if (old == null || newData == null) return;

    logSimpleListChanges(
      oldList: old.bankAccounts.length,
      newList: newData.bankAccounts.length,
      section: LogSection.bankAccount,
      itemType: 'bank account',
      itemLabel: newData.bankAccounts.isNotEmpty ? newData.bankAccounts.first.bankName : null,
    );

    logSimpleListChanges(
      oldList: old.cards.length,
      newList: newData.cards.length,
      section: LogSection.card,
      itemType: 'card',
      itemLabel: newData.cards.isNotEmpty ? newData.cards.first.cardType : null,
    );
  }

  /// Log changes between old and new professional data
  void logProfessionalChanges(ProfessionalData? old, ProfessionalData? newData) {
    if (old == null && newData != null) {
      addLogEntry(
        section: LogSection.professional,
        action: LogAction.create,
        description: 'Added professional data',
      );
      return;
    }
    if (old != null && newData == null) {
      addLogEntry(
        section: LogSection.professional,
        action: LogAction.delete,
        description: 'Deleted professional data',
      );
      return;
    }
    if (old == null || newData == null) return;

    logSimpleListChanges(
      oldList: old.education.length,
      newList: newData.education.length,
      section: LogSection.education,
      itemType: 'education entry',
      itemLabel: newData.education.isNotEmpty ? newData.education.first.institution : null,
    );

    logSimpleListChanges(
      oldList: old.employment.length,
      newList: newData.employment.length,
      section: LogSection.employment,
      itemType: 'employment entry',
      itemLabel: newData.employment.isNotEmpty ? newData.employment.first.company : null,
    );

    logSimpleListChanges(
      oldList: old.skills.length,
      newList: newData.skills.length,
      section: LogSection.skill,
      itemType: 'skill',
      itemLabel: newData.skills.isNotEmpty ? newData.skills.first.toString() : null,
    );

    logSimpleListChanges(
      oldList: old.languages.length,
      newList: newData.languages.length,
      section: LogSection.language,
      itemType: 'language',
      itemLabel: newData.languages.isNotEmpty ? newData.languages.first.toString() : null,
    );
  }

  /// Add a log entry based on section
  void addLogEntry({
    required LogSection section,
    required LogAction action,
    required String description,
  }) {
    switch (section) {
      case LogSection.identity:
        OperationLogService.instance.addEntry(
          OperationLogger.logIdentity(action: action, description: description),
        );
      case LogSection.contactInformation:
        OperationLogService.instance.addEntry(
          OperationLogger.logContactInformation(action: action, description: description),
        );
      case LogSection.address:
        OperationLogService.instance.addEntry(
          OperationLogger.logAddress(action: action, description: description),
        );
      case LogSection.idCard:
        OperationLogService.instance.addEntry(
          OperationLogger.logIdCard(action: action, description: description),
        );
      case LogSection.passport:
        OperationLogService.instance.addEntry(
          OperationLogger.logPassport(action: action, description: description),
        );
      case LogSection.visa:
        OperationLogService.instance.addEntry(
          OperationLogger.logVisa(action: action, description: description),
        );
      case LogSection.travelHistory:
        OperationLogService.instance.addEntry(
          OperationLogger.logTravelHistory(action: action, description: description),
        );
      case LogSection.bankAccount:
        OperationLogService.instance.addEntry(
          OperationLogger.logBankAccount(action: action, description: description),
        );
      case LogSection.card:
        OperationLogService.instance.addEntry(
          OperationLogger.logCard(action: action, description: description),
        );
      case LogSection.education:
        OperationLogService.instance.addEntry(
          OperationLogger.logEducation(action: action, description: description),
        );
      case LogSection.employment:
        OperationLogService.instance.addEntry(
          OperationLogger.logEmployment(action: action, description: description),
        );
      case LogSection.skill:
        OperationLogService.instance.addEntry(
          OperationLogger.logSkill(action: action, description: description),
        );
      case LogSection.language:
        OperationLogService.instance.addEntry(
          OperationLogger.logLanguage(action: action, description: description),
        );
      case LogSection.travel:
        OperationLogService.instance.addEntry(
          OperationLogger.logTravel(action: action, description: description),
        );
      case LogSection.financial:
        OperationLogService.instance.addEntry(
          OperationLogger.logFinancial(action: action, description: description),
        );
      case LogSection.professional:
        OperationLogService.instance.addEntry(
          OperationLogger.logProfessional(action: action, description: description),
        );
      case LogSection.sensitivitySettings:
        OperationLogService.instance.addEntry(
          OperationLogger.logSensitivitySettings(action: action, description: description),
        );
    }
  }

  // ===========================================================================
  // Change Summaries
  // ===========================================================================

  /// Summarize identity changes into a human-readable operation string.
  String summarizeIdentityChanges(IdentityData? old, IdentityData? newData, bool isCreate) {
    if (isCreate) return 'Created Profile';

    final changes = <String>[];

    if (old?.fullName != newData?.fullName) changes.add('Full Name');
    if (old?.givenName != newData?.givenName) changes.add('Given Name');
    if (old?.familyName != newData?.familyName) changes.add('Family Name');
    if (old?.dateOfBirth != newData?.dateOfBirth) changes.add('Date of Birth');
    if (old?.gender != newData?.gender) changes.add('Gender');
    if (old?.nationality != newData?.nationality) changes.add('Nationality');

    final oldContacts = old?.contact?.entries ?? [];
    final newContacts = newData?.contact?.entries ?? [];
    if (newContacts.length > oldContacts.length) {
      changes.add('Contact Information (${newContacts.length - oldContacts.length} added)');
    } else if (newContacts.length < oldContacts.length) {
      changes.add('Contact Information (${oldContacts.length - newContacts.length} removed)');
    } else if (oldContacts.length != newContacts.length ||
        (oldContacts.isNotEmpty && newContacts.isNotEmpty &&
         (oldContacts.first.title != newContacts.first.title ||
          oldContacts.first.value != newContacts.first.value))) {
      changes.add('Contact Information');
    }

    final oldIdCards = old?.idCards ?? [];
    final newIdCards = newData?.idCards ?? [];
    if (newIdCards.length > oldIdCards.length) {
      changes.add('ID Card (${newIdCards.length - oldIdCards.length} added)');
    } else if (newIdCards.length < oldIdCards.length) {
      changes.add('ID Card (${oldIdCards.length - newIdCards.length} removed)');
    } else if (newIdCards.isNotEmpty && oldIdCards.isNotEmpty &&
        (oldIdCards.first.title != newIdCards.first.title ||
         oldIdCards.first.number != newIdCards.first.number)) {
      changes.add('ID Card');
    }

    final oldAddresses = old?.addresses ?? [];
    final newAddresses = newData?.addresses ?? [];
    if (newAddresses.length > oldAddresses.length) {
      changes.add('Address (${newAddresses.length - oldAddresses.length} added)');
    } else if (newAddresses.length < oldAddresses.length) {
      changes.add('Address (${oldAddresses.length - newAddresses.length} removed)');
    } else if (newAddresses.isNotEmpty && oldAddresses.isNotEmpty &&
        (oldAddresses.first.title != newAddresses.first.title ||
         oldAddresses.first.city != newAddresses.first.city)) {
      changes.add('Address');
    }

    if (changes.isEmpty) return 'Updated Profile';

    if (changes.length <= 3) {
      return 'Updated Profile — ${changes.join(', ')}';
    }
    return 'Updated Profile — ${changes.take(3).join(', ')} (+${changes.length - 3} more)';
  }

  /// Summarize travel changes into a human-readable operation string.
  String summarizeTravelChanges(TravelData? old, TravelData? newData, bool isCreate) {
    if (isCreate) return 'Created Travel Data';

    final changes = <String>[];
    final oldPassports = old?.passports ?? [];
    final newPassports = newData?.passports ?? [];
    if (newPassports.length > oldPassports.length) {
      changes.add('Passport (${newPassports.length - oldPassports.length} added)');
    } else if (newPassports.length < oldPassports.length) {
      changes.add('Passport (${oldPassports.length - newPassports.length} removed)');
    }

    final oldVisas = old?.visas ?? [];
    final newVisas = newData?.visas ?? [];
    if (newVisas.length > oldVisas.length) {
      changes.add('Visa (${newVisas.length - oldVisas.length} added)');
    } else if (newVisas.length < oldVisas.length) {
      changes.add('Visa (${oldVisas.length - newVisas.length} removed)');
    }

    final oldHistory = old?.travelHistory ?? [];
    final newHistory = newData?.travelHistory ?? [];
    if (newHistory.length > oldHistory.length) {
      changes.add('Travel History (${newHistory.length - oldHistory.length} added)');
    } else if (newHistory.length < oldHistory.length) {
      changes.add('Travel History (${oldHistory.length - newHistory.length} removed)');
    }

    if (changes.isEmpty) return 'Updated Travel Data';
    return 'Updated Travel Data — ${changes.join(', ')}';
  }

  /// Summarize financial changes into a human-readable operation string.
  String summarizeFinancialChanges(FinancialData? old, FinancialData? newData, bool isCreate) {
    if (isCreate) return 'Created Financial Data';

    final changes = <String>[];
    final oldAccounts = old?.bankAccounts ?? [];
    final newAccounts = newData?.bankAccounts ?? [];
    if (newAccounts.length > oldAccounts.length) {
      changes.add('Bank Account (${newAccounts.length - oldAccounts.length} added)');
    } else if (newAccounts.length < oldAccounts.length) {
      changes.add('Bank Account (${oldAccounts.length - newAccounts.length} removed)');
    }

    final oldCards = old?.cards ?? [];
    final newCards = newData?.cards ?? [];
    if (newCards.length > oldCards.length) {
      changes.add('Card (${newCards.length - oldCards.length} added)');
    } else if (newCards.length < oldCards.length) {
      changes.add('Card (${oldCards.length - newCards.length} removed)');
    }

    if (changes.isEmpty) return 'Updated Financial Data';
    return 'Updated Financial Data — ${changes.join(', ')}';
  }

  /// Summarize professional changes into a human-readable operation string.
  String summarizeProfessionalChanges(ProfessionalData? old, ProfessionalData? newData, bool isCreate) {
    if (isCreate) return 'Created Professional Data';

    final changes = <String>[];
    final oldEdu = old?.education ?? [];
    final newEdu = newData?.education ?? [];
    if (newEdu.length > oldEdu.length) {
      changes.add('Education (${newEdu.length - oldEdu.length} added)');
    } else if (newEdu.length < oldEdu.length) {
      changes.add('Education (${oldEdu.length - newEdu.length} removed)');
    }

    final oldEmp = old?.employment ?? [];
    final newEmp = newData?.employment ?? [];
    if (newEmp.length > oldEmp.length) {
      changes.add('Employment (${newEmp.length - oldEmp.length} added)');
    } else if (newEmp.length < oldEmp.length) {
      changes.add('Employment (${oldEmp.length - newEmp.length} removed)');
    }

    final oldSkills = old?.skills ?? [];
    final newSkills = newData?.skills ?? [];
    if (newSkills.length > oldSkills.length) {
      changes.add('Skill (${newSkills.length - oldSkills.length} added)');
    } else if (newSkills.length < oldSkills.length) {
      changes.add('Skill (${oldSkills.length - newSkills.length} removed)');
    }

    final oldLang = old?.languages ?? [];
    final newLang = newData?.languages ?? [];
    if (newLang.length > oldLang.length) {
      changes.add('Language (${newLang.length - oldLang.length} added)');
    } else if (newLang.length < oldLang.length) {
      changes.add('Language (${oldLang.length - newLang.length} removed)');
    }

    if (changes.isEmpty) return 'Updated Professional Data';
    return 'Updated Professional Data — ${changes.join(', ')}';
  }
}
