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
