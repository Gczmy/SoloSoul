#![cfg_attr(not(test), allow(dead_code))]
#![cfg_attr(not(test), allow(unused_imports))]
//! SoloSoul Vault 存储库
//!
//! 提供：
//! - SQLite 存储 (profiles / metadata / audit_log)
//! - Vault 生命周期管理（open / lock）
//! - 原子文件写入（safe_storage）

pub mod migration;
pub mod profile;
pub mod safe_storage;
pub mod storage;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Vault configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    pub path: PathBuf,
    pub account_id: String,
    pub sqlcipher_key: Option<Vec<u8>>,
}

impl VaultConfig {
    pub fn new(account_id: &str, path: PathBuf) -> Self {
        Self {
            account_id: account_id.to_string(),
            path,
            sqlcipher_key: None,
        }
    }
}

/// Vault state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultState {
    Locked,
    Unlocked,
}

/// Vault statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultStats {
    pub profile_count: usize,
    pub total_size_bytes: u64,
    pub last_modified: Option<String>,
}

pub use profile::{Profile, ProfileData, ProfileSummary, VersionedProfileData};
pub use storage::VaultStore;

// =============================================================================
// Object storage layer — unified object model (P0-1)
// =============================================================================

/// A single unified object stored in the objects table.
/// This is the canonical representation of a user-visible "thing" in SoloSoul.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ObjectRecord {
    pub id: String,
    pub account_id: String,
    #[serde(rename = "typeId")]
    pub type_id: String,
    pub name: String,
    #[serde(rename = "iconName")]
    pub icon_name: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    #[serde(rename = "childrenIds")]
    pub children_ids: Vec<String>,
    pub properties: serde_json::Value,
    #[serde(rename = "propertyLabels")]
    pub property_labels: Option<serde_json::Value>,
    #[serde(rename = "sensitivityLevel")]
    pub sensitivity_level: String,
    #[serde(rename = "isDeleted")]
    pub is_deleted: bool,
    #[serde(rename = "deletedAt")]
    pub deleted_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub version: u32,
}

/// Lightweight summary of an object for listing (no full properties).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ObjectSummary {
    pub id: String,
    pub name: String,
    #[serde(rename = "collectionType")]
    pub collection_type: String,
    #[serde(rename = "sensitivityLevel")]
    pub sensitivity_level: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "isDeleted")]
    pub is_deleted: bool,
    /// First few property key-value pairs for card previews
    pub properties: serde_json::Value,
}

impl ObjectSummary {
    pub fn from_record(r: &ObjectRecord) -> Self {
        Self {
            id: r.id.clone(),
            name: r.name.clone(),
            collection_type: r.type_id.clone(),
            sensitivity_level: r.sensitivity_level.clone(),
            created_at: r.created_at.clone(),
            updated_at: r.updated_at.clone(),
            is_deleted: r.is_deleted,
            properties: r.properties.clone(),
        }
    }
}
