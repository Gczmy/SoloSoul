//! Profile data types for vault storage

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

pub const PROFILE_SCHEMA_VERSION: u32 = 2;

use crate::{CloudSyncConfig, WebDavConfig};

/// Profile stored in vault - wraps encrypted data blob with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub data: Vec<u8>,
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

    pub fn new(name: &str, encrypted_data: Vec<u8>) -> Self {
        Self::new_with_id(&Uuid::new_v4().to_string(), name, encrypted_data)
    }

    pub fn update_data(&mut self, encrypted_data: Vec<u8>) {
        self.data = encrypted_data;
        self.updated_at = Utc::now();
        self.version += 1;
    }
}

/// Profile data sections
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileData {
    pub identity: Option<IdentityData>,
    pub travel: Option<TravelData>,
    pub financial: Option<FinancialData>,
    pub professional: Option<ProfessionalData>,
    pub preferences: Option<PreferencesData>,
    #[serde(default, rename = "unified_objects")]
    pub unified_objects: Option<serde_json::Value>,
    /// Phase 2 云同步：云盘同步配置（WebDAV/OneDrive 等），随 profile 加密存储。
    #[serde(default)]
    pub cloud_sync_config: Option<CloudSyncConfig>,
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
    #[serde(alias = "entryType", rename = "type")]
    #[serde(deserialize_with = "deserialize_contact_type")]
    pub entry_type: String,
    pub value: String,
    #[serde(alias = "isDeleted", rename = "isDeleted")]
    pub is_deleted: bool,
    #[serde(alias = "deletedAt", rename = "deletedAt")]
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Deserialize `contact_type` accepting either a plain JSON string or a legacy
/// `{"entry_type": "..."}` object (backward compat, replaced by `type` field name).
fn deserialize_contact_type<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, IgnoredAny, MapAccess, Visitor};

    struct ContactTypeVisitor;

    impl<'de> Visitor<'de> for ContactTypeVisitor {
        type Value = String;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a string or an object with an `entry_type` field")
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<String, E> {
            Ok(v.to_string())
        }

        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<String, A::Error> {
            let mut entry_type = None;
            while let Some(key) = map.next_key::<String>()? {
                if key == "entry_type" || key == "entryType" {
                    entry_type = Some(map.next_value::<String>()?);
                } else {
                    // Silently skip unknown fields for backward compatibility.
                    let _ = map.next_value::<IgnoredAny>();
                }
            }
            entry_type.ok_or_else(|| de::Error::missing_field("entry_type"))
        }
    }

    deserializer.deserialize_any(ContactTypeVisitor)
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
    pub is_deleted: bool,
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
pub struct PassportData {
    pub number: Option<String>,
    pub country: Option<String>,
    pub issue_date: Option<String>,
    pub expiry_date: Option<String>,
    pub holder_name: Option<String>,
    pub is_deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisaData {
    pub country: Option<String>,
    pub visa_type: Option<String>,
    pub number: Option<String>,
    pub issue_date: Option<String>,
    pub expiry_date: Option<String>,
    pub is_deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TravelHistoryData {
    pub destination: String,
    pub date: Option<String>,
    pub is_deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialData {
    pub bank_accounts: Vec<BankAccountData>,
    pub cards: Vec<CardData>,
    pub tax_ids: Vec<TaxIdData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankAccountData {
    pub bank_name: Option<String>,
    pub account_number: Option<String>,
    pub currency: Option<String>,
    pub swift_bic: Option<String>,
    pub sort_code: Option<String>,
    pub is_deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardData {
    pub card_number: Option<String>,
    pub card_type: Option<String>,
    pub expiry_date: Option<String>,
    pub holder_name: Option<String>,
    pub cvv: Option<String>,
    pub is_deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxIdData {
    pub tax_id_number: Option<String>,
    pub tax_id_type: Option<String>,
    pub issuing_authority: Option<String>,
    pub country: Option<String>,
    pub is_deleted: bool,
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
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub is_deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmploymentData {
    pub company: Option<String>,
    pub position: Option<String>,
    pub responsibilities: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub is_deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillData {
    pub name: String,
    pub level: Option<String>,
    pub is_deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageData {
    pub name: String,
    pub proficiency: Option<String>,
    pub is_deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PreferencesData {
    pub meal_preference: Option<String>,
    pub seat_preference: Option<String>,
    pub travel_companions: Option<String>,
}
