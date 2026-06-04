//! Profile data types matching Dart schema
//!
//! Contains both storage-level types (Profile, ProfileSummary) and
//! schema types (ProfileData, IdentityData, etc.) that map to Dart's schema.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Current profile schema version
pub const PROFILE_SCHEMA_VERSION: u32 = 2;

// ============================================================================
// Storage-level types (used by vault store)
// ============================================================================

/// Profile stored in vault - wraps encrypted data blob with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub data: Vec<u8>, // AES-256-GCM encrypted blob
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: u32,
}

/// Profile summary without encrypted data (for listing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSummary {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: u32,
}

impl Profile {
    /// Create a new profile with a specific ID (e.g., account ID)
    pub fn new_with_id(id: &str, name: &str, encrypted_data: Vec<u8>) -> Self {
        let now = Utc::now();
        Self {
            id: id.to_string(),
            name: name.to_string(),
            data: encrypted_data,
            created_at: now,
            updated_at: now,
            version: 1,
        }
    }

    /// Create a new profile with a random ID
    pub fn new(name: &str, encrypted_data: Vec<u8>) -> Self {
        Self::new_with_id(&Uuid::new_v4().to_string(), name, encrypted_data)
    }

    /// Update profile data
    pub fn update_data(&mut self, encrypted_data: Vec<u8>) {
        self.data = encrypted_data;
        self.updated_at = Utc::now();
        self.version += 1;
    }
}

impl ProfileSummary {
    /// Create from full profile
    pub fn from_profile(profile: &Profile) -> Self {
        Self {
            id: profile.id.clone(),
            name: profile.name.clone(),
            created_at: profile.created_at,
            updated_at: profile.updated_at,
            version: profile.version,
        }
    }
}

// ============================================================================
// VersionedProfileData - Forward-compatible envelope
// ============================================================================

/// Wrapper that captures unknown fields for forward compatibility.
/// Uses serde(flatten) to capture any fields not in ProfileData.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedProfileData {
    pub version: u32,

    #[serde(flatten)]
    pub data: ProfileData,

    /// Captured unknown fields from future schema versions.
    /// These are preserved but not re-serialized into data fields.
    #[serde(flatten)]
    #[serde(skip_deserializing)]
    pub legacy_data: HashMap<String, serde_json::Value>,
}

impl VersionedProfileData {
    /// Create a new VersionedProfileData with current schema version
    pub fn new(data: ProfileData) -> Self {
        Self {
            version: PROFILE_SCHEMA_VERSION,
            data,
            legacy_data: HashMap::new(),
        }
    }

    /// Deserialize from JSON string, capturing unknown fields
    pub fn deserialize(json_str: &str) -> Result<Self, String> {
        serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to deserialize profile data: {}", e))
    }

    /// Serialize to JSON string
    pub fn serialize(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| format!("Failed to serialize profile data: {}", e))
    }
}

// ============================================================================
// Profile Data Structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileData {
    pub identity: Option<IdentityData>,
    pub travel: Option<TravelData>,
    pub financial: Option<FinancialData>,
    pub professional: Option<ProfessionalData>,
    pub preferences: Option<PreferencesData>,
    #[serde(default, rename = "unified_objects")]
    pub unified_objects: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityData {
    #[serde(alias = "fullName", rename = "fullName")]
    pub full_name: Option<String>,
    #[serde(alias = "givenName", rename = "givenName")]
    pub given_name: Option<String>,
    #[serde(alias = "familyName", rename = "familyName")]
    pub family_name: Option<String>,
    #[serde(alias = "dateOfBirth", rename = "dateOfBirth")]
    pub date_of_birth: Option<String>,
    pub gender: Option<String>,
    pub nationality: Option<String>,
    #[serde(alias = "idCards", rename = "idCards")]
    pub id_cards: Vec<IdCardData>,
    pub contact: Option<ContactData>,
    pub addresses: Vec<AddressData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactEntry {
    pub label: String,
    #[serde(alias = "type", rename = "type")]
    #[serde(deserialize_with = "deserialize_contact_type")]
    pub entry_type: String,
    pub value: String,
    #[serde(alias = "isDeleted", rename = "isDeleted")]
    pub is_deleted: bool,
    #[serde(alias = "deletedAt", rename = "deletedAt")]
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Custom deserializer for contact entry type field to handle both "type" and "entry_type"
fn deserialize_contact_type<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum TypeOrEntryType {
        Type(String),
        EntryType {
            #[serde(alias = "entry_type")]
            entry_type: String,
        },
    }

    let raw = TypeOrEntryType::deserialize(deserializer)?;
    match raw {
        TypeOrEntryType::Type(s) => Ok(s),
        TypeOrEntryType::EntryType { entry_type } => Ok(entry_type),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactData {
    pub entries: Vec<ContactEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressData {
    pub label: Option<String>,
    pub street: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub district: Option<String>,
    #[serde(alias = "postalCode", rename = "postalCode")]
    pub postal_code: Option<String>,
    pub country: Option<String>,
    #[serde(alias = "isDeleted", rename = "isDeleted")]
    pub is_deleted: bool,
    #[serde(alias = "deletedAt", rename = "deletedAt")]
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdCardData {
    pub label: Option<String>,
    pub number: Option<String>,
    #[serde(alias = "issueDate", rename = "issueDate")]
    pub issue_date: Option<String>,
    #[serde(alias = "expiryDate", rename = "expiryDate")]
    pub expiry_date: Option<String>,
    #[serde(alias = "holderName", rename = "holderName")]
    pub holder_name: Option<String>,
    pub country: Option<String>,
    #[serde(alias = "isDeleted", rename = "isDeleted")]
    pub is_deleted: bool,
    #[serde(alias = "deletedAt", rename = "deletedAt")]
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TravelData {
    pub passports: Vec<PassportData>,
    pub visas: Vec<VisaData>,
    #[serde(alias = "travelHistory", rename = "travelHistory")]
    pub travel_history: Vec<TravelHistoryData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TravelHistoryData {
    pub destination: String,
    pub date: Option<String>,
    #[serde(alias = "isDeleted", rename = "isDeleted")]
    pub is_deleted: bool,
    #[serde(alias = "deletedAt", rename = "deletedAt")]
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassportData {
    pub number: Option<String>,
    pub country: Option<String>,
    #[serde(alias = "issueDate", rename = "issueDate")]
    pub issue_date: Option<String>,
    #[serde(alias = "expiryDate", rename = "expiryDate")]
    pub expiry_date: Option<String>,
    #[serde(alias = "holderName", rename = "holderName")]
    pub holder_name: Option<String>,
    #[serde(alias = "isDeleted", rename = "isDeleted")]
    pub is_deleted: bool,
    #[serde(alias = "deletedAt", rename = "deletedAt")]
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisaData {
    pub country: Option<String>,
    #[serde(alias = "visaType", rename = "visaType")]
    pub visa_type: Option<String>,
    pub number: Option<String>,
    #[serde(alias = "issueDate", rename = "issueDate")]
    pub issue_date: Option<String>,
    #[serde(alias = "expiryDate", rename = "expiryDate")]
    pub expiry_date: Option<String>,
    #[serde(alias = "isDeleted", rename = "isDeleted")]
    pub is_deleted: bool,
    #[serde(alias = "deletedAt", rename = "deletedAt")]
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialData {
    #[serde(alias = "bankAccounts", rename = "bankAccounts")]
    pub bank_accounts: Vec<BankAccountData>,
    pub cards: Vec<CardData>,
    #[serde(alias = "taxIds", rename = "taxIds")]
    pub tax_ids: Vec<TaxIdData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankAccountData {
    #[serde(alias = "bankName", rename = "bankName")]
    pub bank_name: Option<String>,
    #[serde(alias = "accountNumber", rename = "accountNumber")]
    pub account_number: Option<String>,
    pub currency: Option<String>,
    #[serde(alias = "swiftBic", rename = "swiftBic")]
    pub swift_bic: Option<String>,
    #[serde(alias = "sortCode", rename = "sortCode")]
    pub sort_code: Option<String>,
    #[serde(alias = "isDeleted", rename = "isDeleted")]
    pub is_deleted: bool,
    #[serde(alias = "deletedAt", rename = "deletedAt")]
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardData {
    #[serde(alias = "cardNumber", rename = "cardNumber")]
    pub card_number: Option<String>,
    #[serde(alias = "cardType", rename = "cardType")]
    pub card_type: Option<String>,
    #[serde(alias = "expiryDate", rename = "expiryDate")]
    pub expiry_date: Option<String>,
    #[serde(alias = "holderName", rename = "holderName")]
    pub holder_name: Option<String>,
    #[serde(alias = "cvv", rename = "cvv")]
    pub cvv: Option<String>,
    #[serde(alias = "isDeleted", rename = "isDeleted")]
    pub is_deleted: bool,
    #[serde(alias = "deletedAt", rename = "deletedAt")]
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxIdData {
    #[serde(alias = "taxIdNumber", rename = "taxIdNumber")]
    pub tax_id_number: Option<String>,
    #[serde(alias = "taxIdType", rename = "taxIdType")]
    pub tax_id_type: Option<String>,
    #[serde(alias = "issuingAuthority", rename = "issuingAuthority")]
    pub issuing_authority: Option<String>,
    pub country: Option<String>,
    #[serde(alias = "isDeleted", rename = "isDeleted")]
    pub is_deleted: bool,
    #[serde(alias = "deletedAt", rename = "deletedAt")]
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfessionalData {
    pub education: Vec<EducationData>,
    pub employment: Vec<EmploymentData>,
    pub skills: Vec<SkillData>,
    pub languages: Vec<LanguageData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EducationData {
    pub institution: Option<String>,
    pub degree: Option<String>,
    pub field: Option<String>,
    #[serde(alias = "startDate", rename = "startDate")]
    pub start_date: Option<String>,
    #[serde(alias = "endDate", rename = "endDate")]
    pub end_date: Option<String>,
    #[serde(alias = "isDeleted", rename = "isDeleted")]
    pub is_deleted: bool,
    #[serde(alias = "deletedAt", rename = "deletedAt")]
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmploymentData {
    pub company: Option<String>,
    pub position: Option<String>,
    pub responsibilities: Option<String>,
    #[serde(alias = "startDate", rename = "startDate")]
    pub start_date: Option<String>,
    #[serde(alias = "endDate", rename = "endDate")]
    pub end_date: Option<String>,
    #[serde(alias = "isDeleted", rename = "isDeleted")]
    pub is_deleted: bool,
    #[serde(alias = "deletedAt", rename = "deletedAt")]
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillData {
    pub name: String,
    pub level: Option<String>,
    #[serde(alias = "isDeleted", rename = "isDeleted")]
    pub is_deleted: bool,
    #[serde(alias = "deletedAt", rename = "deletedAt")]
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageData {
    pub name: String,
    pub proficiency: Option<String>,
    #[serde(alias = "isDeleted", rename = "isDeleted")]
    pub is_deleted: bool,
    #[serde(alias = "deletedAt", rename = "deletedAt")]
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferencesData {
    #[serde(alias = "mealPreference", rename = "mealPreference")]
    pub meal_preference: Option<String>,
    #[serde(alias = "seatPreference", rename = "seatPreference")]
    pub seat_preference: Option<String>,
    #[serde(alias = "travelCompanions", rename = "travelCompanions")]
    pub travel_companions: Option<String>,
    #[serde(alias = "notificationPreferences", rename = "notificationPreferences")]
    pub notification_preferences: Option<NotificationPreferencesData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreferencesData {
    pub email: bool,
    pub sms: bool,
    pub push: bool,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_versioned_profile_data_new() {
        let profile_data = ProfileData {
            identity: None,
            travel: None,
            financial: None,
            professional: None,
            preferences: None,
        };

        let vpd = VersionedProfileData::new(profile_data);
        assert_eq!(vpd.version, PROFILE_SCHEMA_VERSION);
        assert!(vpd.legacy_data.is_empty());
    }

    #[test]
    fn test_versioned_profile_data_serialize_deserialize_roundtrip() {
        let profile_data = ProfileData {
            identity: Some(IdentityData {
                full_name: Some("John Doe".to_string()),
                given_name: Some("John".to_string()),
                family_name: Some("Doe".to_string()),
                date_of_birth: Some("1990-01-15".to_string()),
                gender: Some("Male".to_string()),
                nationality: Some("US".to_string()),
                id_cards: vec![IdCardData {
                    label: Some("Primary".to_string()),
                    number: Some("AB123456".to_string()),
                    issue_date: Some("2020-01-01".to_string()),
                    expiry_date: Some("2030-01-01".to_string()),
                    holder_name: Some("JOHN DOE".to_string()),
                    country: Some("US".to_string()),
                    is_deleted: false,
                    deleted_at: None,
                }],
                contact: Some(ContactData {
                    entries: vec![ContactEntry {
                        label: "Personal".to_string(),
                        entry_type: "email".to_string(),
                        value: "john@example.com".to_string(),
                        is_deleted: false,
                        deleted_at: None,
                    }],
                }),
                addresses: vec![AddressData {
                    label: Some("Home".to_string()),
                    street: Some("123 Main St".to_string()),
                    city: Some("NYC".to_string()),
                    state: Some("NY".to_string()),
                    district: None,
                    postal_code: Some("10001".to_string()),
                    country: Some("US".to_string()),
                    is_deleted: false,
                    deleted_at: None,
                }],
            }),
            travel: Some(TravelData {
                passports: vec![PassportData {
                    number: Some("AB123456".to_string()),
                    country: Some("US".to_string()),
                    issue_date: Some("2020-01-01".to_string()),
                    expiry_date: Some("2030-01-01".to_string()),
                    holder_name: Some("JOHN DOE".to_string()),
                    is_deleted: false,
                    deleted_at: None,
                }],
                visas: vec![],
                travel_history: vec![],
            }),
            financial: Some(FinancialData {
                bank_accounts: vec![],
                cards: vec![],
                tax_ids: vec![TaxIdData {
                    tax_id_number: Some("123-45-6789".to_string()),
                    tax_id_type: Some("SSN".to_string()),
                    issuing_authority: Some("IRS".to_string()),
                    country: Some("US".to_string()),
                    is_deleted: false,
                    deleted_at: None,
                }],
            }),
            professional: Some(ProfessionalData {
                education: vec![],
                employment: vec![],
                skills: vec![SkillData {
                    name: "Rust".to_string(),
                    level: Some("Advanced".to_string()),
                    is_deleted: false,
                    deleted_at: None,
                }],
                languages: vec![LanguageData {
                    name: "English".to_string(),
                    proficiency: Some("Native".to_string()),
                    is_deleted: false,
                    deleted_at: None,
                }],
            }),
            preferences: None,
        };

        let vpd = VersionedProfileData::new(profile_data);
        let json = vpd.serialize().unwrap();

        // Deserialize and check
        let deserialized = VersionedProfileData::deserialize(&json).unwrap();
        assert_eq!(deserialized.version, PROFILE_SCHEMA_VERSION);
        assert!(deserialized.legacy_data.is_empty());

        // Check nested data
        let identity = deserialized.data.identity.unwrap();
        assert_eq!(identity.full_name, Some("John Doe".to_string()));
        assert_eq!(identity.id_cards.len(), 1);
        assert_eq!(identity.contact.as_ref().unwrap().entries.len(), 1);
        assert_eq!(identity.addresses.len(), 1);

        let financial = deserialized.data.financial.unwrap();
        assert_eq!(financial.tax_ids.len(), 1);

        let professional = deserialized.data.professional.unwrap();
        assert_eq!(professional.skills.len(), 1);
        assert_eq!(professional.languages.len(), 1);
    }

    #[test]
    fn test_contact_entry_deserialize_with_type_field() {
        // Test deserialization with "type" field (as sent by Dart)
        let json = r#"{
            "label": "Work",
            "type": "phone",
            "value": "+1234567890",
            "isDeleted": false,
            "deletedAt": null
        }"#;

        let entry: ContactEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.label, "Work");
        assert_eq!(entry.entry_type, "phone");
        assert_eq!(entry.value, "+1234567890");
        assert!(!entry.is_deleted);
    }

    #[test]
    fn test_soft_delete_fields_preserved() {
        let now = Utc::now();
        let profile_data = ProfileData {
            identity: Some(IdentityData {
                full_name: None,
                given_name: None,
                family_name: None,
                date_of_birth: None,
                gender: None,
                nationality: None,
                id_cards: vec![],
                contact: None,
                addresses: vec![],
            }),
            travel: Some(TravelData {
                passports: vec![PassportData {
                    number: None,
                    country: None,
                    issue_date: None,
                    expiry_date: None,
                    holder_name: None,
                    is_deleted: true,
                    deleted_at: Some(now),
                }],
                visas: vec![],
                travel_history: vec![],
            }),
            financial: None,
            professional: None,
            preferences: None,
        };

        let vpd = VersionedProfileData::new(profile_data);
        let json = vpd.serialize().unwrap();

        // Verify deleted_at is preserved
        let deserialized = VersionedProfileData::deserialize(&json).unwrap();
        let passport = &deserialized.data.travel.as_ref().unwrap().passports[0];
        assert!(passport.is_deleted);
        assert!(passport.deleted_at.is_some());
    }
}
