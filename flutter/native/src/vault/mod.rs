//! Vault module - Secure local storage with双重加密
//!
//! Provides:
//! - rusqlite + SQLCipher双重加密存储
//! - Profile management
//! - Secure memory handling with mlock/zeroize

pub mod migration;
#[cfg(test)]
mod migration_tests;
pub mod processor;
mod profile;
mod store;

pub use profile::*;
pub use store::*;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Vault configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    /// Vault path
    pub path: PathBuf,
    /// Account ID
    pub account_id: String,
    /// SQLCipher key (derived from master password)
    pub sqlcipher_key: Option<Vec<u8>>,
}

impl VaultConfig {
    /// Create new vault configuration
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
    /// Vault is locked
    Locked,
    /// Vault is unlocked
    Unlocked,
    /// Vault is corrupted
    Corrupted,
}

/// Vault statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultStats {
    pub profile_count: usize,
    pub total_size_bytes: u64,
    pub last_modified: Option<String>,
}
