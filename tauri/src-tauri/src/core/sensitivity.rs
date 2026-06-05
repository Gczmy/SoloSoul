//! SensitivityMap -- the single source of truth for field-level sensitivity.
//! Per 21_矛盾冲突与待审批事项.md: unified to 4 levels (public/internal/sensitive/critical)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;

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

/// Single source of truth for field sensitivity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivityMap {
    pub version: u8,
    pub entries: HashMap<String, SensitivityLevel>,
    pub last_modified_at: String,
}

impl SensitivityMap {
    pub fn new() -> Self {
        Self {
            version: 1,
            entries: Self::default_entries(),
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
        self.entries.get(field_id).copied().unwrap_or(SensitivityLevel::Internal)
    }

    pub fn set(&mut self, field_id: &str, level: SensitivityLevel) {
        self.entries.insert(field_id.to_string(), level);
        self.last_modified_at = Utc::now().to_rfc3339();
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
        Self { entries: Vec::new() }
    }

    pub fn push(&mut self, field_id: &str, old: SensitivityLevel, new: SensitivityLevel, reason: String) {
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
    fn default() -> Self { Self::new() }
}

impl SensitivityManager {
    pub fn new() -> Self {
        Self {
            map: Arc::new(RwLock::new(SensitivityMap::new())),
            log: Arc::new(RwLock::new(SensitivityLog::new())),
        }
    }
}

impl Default for SensitivityMap { fn default() -> Self { Self::new() } }
impl Default for SensitivityLog { fn default() -> Self { Self::new() } }
