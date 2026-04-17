import 'package:solosoul_flutter/core/repositories/base_vault_repository.dart';

/// Identity section repository
/// Handles: fullName, givenName, familyName, dateOfBirth, gender, nationality,
///          idCards, contact, addresses
class IdentityRepository extends BaseVaultRepository {
  static const String sectionName = 'identity';

  // Identity-specific methods can be added here as needed
}

/// Financial section repository
/// Handles: bankAccounts, cards, taxIds
class FinancialRepository extends BaseVaultRepository {
  static const String sectionName = 'financial';

  // Financial-specific methods can be added here as needed
}

/// Travel section repository
/// Handles: passports, visas, travelHistory
class TravelRepository extends BaseVaultRepository {
  static const String sectionName = 'travel';

  // Travel-specific methods can be added here as needed
}

/// Professional section repository
/// Handles: education, employment, skills, languages
class ProfessionalRepository extends BaseVaultRepository {
  static const String sectionName = 'professional';

  // Professional-specific methods can be added here as needed
}

/// Health section repository
/// Handles: medicalRecords, allergies, medications, emergencyContacts
class HealthRepository extends BaseVaultRepository {
  static const String sectionName = 'health';

  // Health-specific methods can be added here as needed
}

/// Education section repository
/// Handles: degrees, certificates, trainings, skillsAcquired
class EducationRepository extends BaseVaultRepository {
  static const String sectionName = 'education';

  // Education-specific methods can be added here as needed
}

/// Family section repository
/// Handles: familyMembers, relationships, nextOfKin
class FamilyRepository extends BaseVaultRepository {
  static const String sectionName = 'family';

  // Family-specific methods can be added here as needed
}

/// Insurance section repository
/// Handles: policies, claims, coverage
class InsuranceRepository extends BaseVaultRepository {
  static const String sectionName = 'insurance';

  // Insurance-specific methods can be added here as needed
}

/// Legal section repository
/// Handles: documents, contracts, agreements, idNumbers
class LegalRepository extends BaseVaultRepository {
  static const String sectionName = 'legal';

  // Legal-specific methods can be added here as needed
}

/// Subscription section repository
/// Handles: subscriptions, memberships, services
class SubscriptionRepository extends BaseVaultRepository {
  static const String sectionName = 'subscription';

  // Subscription-specific methods can be added here as needed
}

/// Other section repository
/// Handles: misc categories
class OtherRepository extends BaseVaultRepository {
  static const String sectionName = 'other';

  // Other-specific methods can be added here as needed
}
