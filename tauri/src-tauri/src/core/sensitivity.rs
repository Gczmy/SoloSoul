//! SensitivityMap -- the single source of truth for field-level sensitivity.
//! Per 21_矛盾冲突与待审批事项.md: unified to 4 levels (public/internal/sensitive/critical)

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensitivityLevel {
    Public,
    Internal,
    Sensitive,
    Critical,
}

impl SensitivityLevel {
    pub fn parse_level(s: &str) -> Option<Self> {
        match s {
            "public" => Some(Self::Public),
            "internal" => Some(Self::Internal),
            "sensitive" => Some(Self::Sensitive),
            "critical" => Some(Self::Critical),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Internal => "internal",
            Self::Sensitive => "sensitive",
            Self::Critical => "critical",
        }
    }
}

/// Status of a field's template source for UI display.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateSourceStatus {
    /// Template currently exists.
    Active,
    /// Template is in trash (soft-deleted).
    SoftDeleted,
    /// Template was permanently deleted.
    PermanentlyDeleted,
    /// No template source — manually configured.
    Manual,
}

/// Single source of truth for field sensitivity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivityMap {
    pub version: u8,
    pub entries: HashMap<String, SensitivityLevel>,
    /// Tracks which template each field_id originated from.
    /// `None` means manually configured (no template source).
    pub template_sources: HashMap<String, Option<String>>,
    /// UI-only: computed status of each field's template source.
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub template_source_statuses: HashMap<String, TemplateSourceStatus>,
    pub last_modified_at: String,
}

impl SensitivityMap {
    pub fn new() -> Self {
        Self {
            version: 1,
            entries: Self::default_entries(),
            template_sources: HashMap::new(),
            template_source_statuses: HashMap::new(),
            last_modified_at: Utc::now().to_rfc3339(),
        }
    }

    fn default_entries() -> HashMap<String, SensitivityLevel> {
        let mut m = HashMap::new();
        m.insert("identity.full_name".into(), SensitivityLevel::Public);
        m.insert("identity.date_of_birth".into(), SensitivityLevel::Sensitive);
        m.insert("identity.nationality".into(), SensitivityLevel::Public);
        m.insert("identity.id_number".into(), SensitivityLevel::Critical);
        m.insert("identity.email".into(), SensitivityLevel::Internal);
        m.insert("identity.phone".into(), SensitivityLevel::Internal);
        m.insert("travel.passport_number".into(), SensitivityLevel::Critical);
        m.insert("travel.passport.expiry".into(), SensitivityLevel::Sensitive);
        m.insert("financial.card_number".into(), SensitivityLevel::Critical);
        m.insert("financial.bank_account".into(), SensitivityLevel::Critical);
        m.insert("financial.cvv".into(), SensitivityLevel::Critical);
        m.insert("financial.swift".into(), SensitivityLevel::Sensitive);
        m.insert("professional.skills".into(), SensitivityLevel::Public);
        m
    }

    pub fn get(&self, field_id: &str) -> SensitivityLevel {
        self.entries
            .get(field_id)
            .copied()
            .unwrap_or(SensitivityLevel::Internal)
    }

    pub fn set(&mut self, field_id: &str, level: SensitivityLevel) {
        self.entries.insert(field_id.to_string(), level);
        self.last_modified_at = Utc::now().to_rfc3339();
    }

    pub fn set_with_source(
        &mut self,
        field_id: &str,
        level: SensitivityLevel,
        template_id: Option<String>,
    ) {
        self.entries.insert(field_id.to_string(), level);
        self.template_sources
            .insert(field_id.to_string(), template_id);
        self.last_modified_at = Utc::now().to_rfc3339();
    }

    /// Load entries from vault DB. Overrides in-memory entries with persisted data.
    pub fn load_from_vault(&mut self, vault: &solosoul_vault::VaultStore) -> Result<(), String> {
        let persisted = vault.list_sensitivity_entries()?;
        for (field_id, level_str, template_id) in persisted {
            if let Some(level) = SensitivityLevel::parse_level(&level_str) {
                self.entries.insert(field_id.clone(), level);
                self.template_sources.insert(field_id, template_id);
            }
        }
        self.last_modified_at = Utc::now().to_rfc3339();
        Ok(())
    }

    /// Save all entries to vault DB.
    pub fn save_to_vault(&self, vault: &solosoul_vault::VaultStore) -> Result<(), String> {
        for (field_id, level) in &self.entries {
            let template_id = self
                .template_sources
                .get(field_id)
                .and_then(|t| t.as_deref());
            vault.save_sensitivity_entry(field_id, level.as_str(), template_id)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivityLogEntry {
    pub timestamp: String,
    pub field_id: String,
    pub old_level: SensitivityLevel,
    pub new_level: SensitivityLevel,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivityLog {
    pub entries: Vec<SensitivityLogEntry>,
}

impl SensitivityLog {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn push(
        &mut self,
        field_id: &str,
        old: SensitivityLevel,
        new: SensitivityLevel,
        reason: String,
    ) {
        self.entries.push(SensitivityLogEntry {
            timestamp: Utc::now().to_rfc3339(),
            field_id: field_id.to_string(),
            old_level: old,
            new_level: new,
            reason,
        });
        if self.entries.len() > 1000 {
            self.entries.drain(0..self.entries.len() - 1000);
        }
    }
}

/// Sensitivity manager -- registered as Tauri State
pub struct SensitivityManager {
    pub map: Arc<RwLock<SensitivityMap>>,
    pub log: Arc<RwLock<SensitivityLog>>,
}

impl Default for SensitivityManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SensitivityManager {
    pub fn new() -> Self {
        Self {
            map: Arc::new(RwLock::new(SensitivityMap::new())),
            log: Arc::new(RwLock::new(SensitivityLog::new())),
        }
    }

    /// Load sensitivity map from vault DB (called after vault unlock).
    pub fn load_from_vault(&self, vault: &solosoul_vault::VaultStore) -> Result<(), String> {
        let mut map = self.map.write().map_err(|e| e.to_string())?;
        map.load_from_vault(vault)
    }

    /// Save sensitivity map to vault DB (called before vault lock).
    pub fn save_to_vault(&self, vault: &solosoul_vault::VaultStore) -> Result<(), String> {
        let map = self.map.read().map_err(|e| e.to_string())?;
        map.save_to_vault(vault)
    }

    /// Clear all entries (called after vault lock).
    pub fn clear(&self) {
        let mut map = self.map.write().expect("SensitivityManager lock poisoned");
        map.entries.clear();
        map.template_sources.clear();
        map.template_source_statuses.clear();
        map.last_modified_at = Utc::now().to_rfc3339();
    }

    /// Compute template-source status for every field in the map.
    pub fn compute_template_source_statuses(
        &self,
        vault: &solosoul_vault::VaultStore,
    ) -> Result<HashMap<String, TemplateSourceStatus>, String> {
        let map = self.map.read().map_err(|e| e.to_string())?;
        let mut statuses = HashMap::new();

        for (field_id, template_id_opt) in &map.template_sources {
            let status = match template_id_opt {
                None => TemplateSourceStatus::Manual,
                Some(template_id) => {
                    if vault.user_template_exists(template_id).unwrap_or(false) {
                        TemplateSourceStatus::Active
                    } else if vault.is_template_in_trash(template_id).unwrap_or(false) {
                        TemplateSourceStatus::SoftDeleted
                    } else {
                        TemplateSourceStatus::PermanentlyDeleted
                    }
                }
            };
            statuses.insert(field_id.clone(), status);
        }

        Ok(statuses)
    }
}

impl Default for SensitivityMap {
    fn default() -> Self {
        Self::new()
    }
}
impl Default for SensitivityLog {
    fn default() -> Self {
        Self::new()
    }
}
