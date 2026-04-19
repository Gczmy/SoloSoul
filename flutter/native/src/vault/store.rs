//! Vault store - SQLite storage with SQLCipher
//!
//! Implements双重加密: AES-256-GCM at application layer + SQLCipher at storage layer

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rusqlite::{Connection, params};
use std::sync::Mutex;
use zeroize::Zeroize;

use super::{VaultConfig, VaultStats, VaultState};
use super::migration::run_migrations;

/// Vault store with双重加密
pub struct VaultStore {
    conn: Mutex<Option<Connection>>,
    config: VaultConfig,
    state: VaultState,
}

impl VaultStore {
    /// Open or create a vault at the given path
    pub fn open(config: VaultConfig) -> Result<Self, String> {
        let path = config.path.join("vault.db");

        // Open SQLite connection
        let mut conn = Connection::open(&path)
            .map_err(|e| format!("Failed to open vault: {}", e))?;

        // Apply SQLCipher encryption BEFORE any other operations
        if let Some(ref key) = config.sqlcipher_key {
            let key_hex = hex::encode(key);
            conn.execute_batch(&format!("PRAGMA key = \"x'{key_hex}'\";"))
                .map_err(|e| format!("Failed to set SQLCipher key: {}", e))?;
        }

        // Initialize schema
        Self::init_schema(&conn)?;

        // Run migrations after schema init
        run_migrations(&mut conn).map_err(|e| format!("Migration failed: {}", e))?;

        Ok(Self {
            conn: Mutex::new(Some(conn)),
            config,
            state: VaultState::Unlocked,
        })
    }

    /// Initialize vault schema
    fn init_schema(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            r#"
            -- Profiles table (encrypted blob storage)
            CREATE TABLE IF NOT EXISTS profiles (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                data BLOB NOT NULL,          -- AES-256-GCM encrypted blob
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                version INTEGER DEFAULT 1
            );

            -- Profile index for fast lookup
            CREATE INDEX IF NOT EXISTS idx_profile_name ON profiles(name);

            -- Metadata table
            CREATE TABLE IF NOT EXISTS metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT
            );

            -- Audit log table (append-only)
            CREATE TABLE IF NOT EXISTS audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                action TEXT NOT NULL,
                details TEXT,
                session_id TEXT
            );

            -- Sync state table
            CREATE TABLE IF NOT EXISTS sync_state (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            -- System configuration table (key/value store for schema versioning)
            CREATE TABLE IF NOT EXISTS sys_config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            -- Schema migrations tracking table
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at INTEGER NOT NULL,
                description TEXT
            );
            "#,
        )
        .map_err(|e| format!("Failed to initialize schema: {}", e))?;

        // Initialize data_version to '1' if not already set
        let version_exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sys_config WHERE key = 'data_version')",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !version_exists {
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO sys_config (key, value, updated_at) VALUES ('data_version', '1', ?1)",
                params![now],
            )
            .map_err(|e| format!("Failed to initialize data_version: {}", e))?;
        }

        Ok(())
    }

    /// Get vault state
    pub fn state(&self) -> VaultState {
        self.state
    }

    /// Get vault statistics
    pub fn stats(&self) -> Result<VaultStats, String> {
        let conn_guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = conn_guard.as_ref().ok_or("Vault is locked")?;

        let profile_count: usize = conn
            .query_row("SELECT COUNT(*) FROM profiles", [], |row| row.get(0))
            .map_err(|e| e.to_string())?;

        let total_size_bytes: u64 = conn
            .query_row("SELECT COALESCE(SUM(LENGTH(data)), 0) FROM profiles", [], |row| row.get(0))
            .map_err(|e| e.to_string())?;

        let last_modified: Option<String> = conn
            .query_row(
                "SELECT MAX(updated_at) FROM profiles",
                [],
                |row| row.get(0),
            )
            .ok();

        Ok(VaultStats {
            profile_count,
            total_size_bytes,
            last_modified,
        })
    }

    /// Lock the vault
    pub fn lock(&mut self) {
        // Properly close the connection to release file locks
        if let Ok(mut conn_guard) = self.conn.lock() {
            if let Some(conn) = conn_guard.take() {
                // Execute WAL checkpoint to flush data to main db
                let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)");
                // Connection will be dropped here when it goes out of scope
                // This properly closes the connection and releases file locks
            }
        }

        self.state = VaultState::Locked;
        // Clear SQLCipher key from memory
        if let Some(ref mut key) = self.config.sqlcipher_key {
            key.zeroize();
        }
    }

    /// Save a profile (INSERT or UPDATE)
    pub fn save_profile(&self, profile: &crate::vault::Profile) -> Result<(), String> {
        let mut conn_guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = conn_guard.as_mut().ok_or("Vault is locked")?;
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO profiles (id, name, data, created_at, updated_at, version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                data = excluded.data,
                updated_at = excluded.updated_at,
                version = excluded.version",
            rusqlite::params![
                profile.id,
                profile.name,
                profile.data,
                profile.created_at.to_rfc3339(),
                now,
                profile.version,
            ],
        )
        .map_err(|e| format!("Failed to save profile: {}", e))?;

        Ok(())
    }

    /// Load a profile by ID
    pub fn load_profile(&self, id: &str) -> Result<Option<crate::vault::Profile>, String> {
        let mut conn_guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = conn_guard.as_mut().ok_or("Vault is locked")?;

        let mut stmt = conn
            .prepare("SELECT id, name, data, created_at, updated_at, version FROM profiles WHERE id = ?1")
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let result = stmt
            .query_row(rusqlite::params![id], |row| {
                let created_str: String = row.get(3)?;
                let updated_str: String = row.get(4)?;
                Ok(crate::vault::Profile {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    data: row.get(2)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&updated_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    version: row.get(5)?,
                })
            })
            .ok();

        Ok(result)
    }

    /// Delete a profile by ID
    pub fn delete_profile(&self, id: &str) -> Result<(), String> {
        let mut conn_guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = conn_guard.as_mut().ok_or("Vault is locked")?;

        let affected = conn
            .execute("DELETE FROM profiles WHERE id = ?1", rusqlite::params![id])
            .map_err(|e| format!("Failed to delete profile: {}", e))?;

        if affected == 0 {
            return Err("Profile not found".to_string());
        }

        Ok(())
    }

    /// List all profile summaries
    pub fn list_profiles(&self) -> Result<Vec<crate::vault::ProfileSummary>, String> {
        let mut conn_guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = conn_guard.as_mut().ok_or("Vault is locked")?;

        let mut stmt = conn
            .prepare("SELECT id, name, created_at, updated_at, version FROM profiles ORDER BY updated_at DESC")
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let profiles = stmt
            .query_map([], |row| {
                let created_str: String = row.get(2)?;
                let updated_str: String = row.get(3)?;
                Ok(crate::vault::ProfileSummary {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&updated_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    version: row.get(4)?,
                })
            })
            .map_err(|e| format!("Failed to query profiles: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect profiles: {}", e))?;

        Ok(profiles)
    }

    /// Search profiles by name (case-insensitive LIKE)
    pub fn search_profiles(&self, query: &str) -> Result<Vec<crate::vault::ProfileSummary>, String> {
        let mut conn_guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = conn_guard.as_mut().ok_or("Vault is locked")?;

        let pattern = format!("%{}%", query.to_lowercase());
        let mut stmt = conn
            .prepare("SELECT id, name, created_at, updated_at, version FROM profiles WHERE LOWER(name) LIKE ?1 ORDER BY updated_at DESC")
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let profiles = stmt
            .query_map([&pattern], |row| {
                let created_str: String = row.get(2)?;
                let updated_str: String = row.get(3)?;
                Ok(crate::vault::ProfileSummary {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&updated_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    version: row.get(4)?,
                })
            })
            .map_err(|e| format!("Failed to query profiles: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect profiles: {}", e))?;

        Ok(profiles)
    }

    /// Save field histories (encrypted blob)
    pub fn save_field_histories(&self, account_id: &str, data: &[u8]) -> Result<(), String> {
        let mut conn_guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = conn_guard.as_mut().ok_or("Vault is locked")?;
        let now = chrono::Utc::now().to_rfc3339();
        let key = format!("HIST_{}", account_id);

        conn.execute(
            "INSERT INTO metadata (key, value, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET
                value = excluded.value,
                updated_at = excluded.updated_at",
            rusqlite::params![key, base64_encode(data), now],
        )
        .map_err(|e| format!("Failed to save field histories: {}", e))?;

        Ok(())
    }

    /// Load field histories (encrypted blob)
    pub fn load_field_histories(&self, account_id: &str) -> Result<Option<Vec<u8>>, String> {
        let mut conn_guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = conn_guard.as_mut().ok_or("Vault is locked")?;
        let key = format!("HIST_{}", account_id);

        let result: Option<String> = conn
            .query_row(
                "SELECT value FROM metadata WHERE key = ?1",
                rusqlite::params![key],
                |row| row.get(0),
            )
            .ok();

        match result {
            Some(encoded) => {
                let decoded = base64_decode(&encoded)
                    .map_err(|e| format!("Failed to decode field histories: {}", e))?;
                Ok(Some(decoded))
            }
            None => Ok(None),
        }
    }

    /// Delete field histories
    pub fn delete_field_histories(&self, account_id: &str) -> Result<(), String> {
        let mut conn_guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = conn_guard.as_mut().ok_or("Vault is locked")?;
        let key = format!("HIST_{}", account_id);

        conn.execute("DELETE FROM metadata WHERE key = ?1", rusqlite::params![key])
            .map_err(|e| format!("Failed to delete field histories: {}", e))?;

        Ok(())
    }
}

/// Helper for hex encoding
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Helper for base64 decoding
fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    BASE64.decode(input).map_err(|e| format!("Base64 decode error: {}", e))
}

/// Helper for base64 encoding
fn base64_encode(data: &[u8]) -> String {
    BASE64.encode(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::Profile;
    use std::path::PathBuf;
    use uuid::Uuid;

    fn create_unique_temp_dir() -> PathBuf {
        let temp_dir = std::env::temp_dir().join(format!("solosoul_test_vault_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        temp_dir
    }

    fn cleanup_vault(temp_dir: PathBuf) {
        std::fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn test_vault_open() {
        let temp_dir = create_unique_temp_dir();

        let config = VaultConfig::new("test_account", temp_dir.clone());
        let vault = VaultStore::open(config);

        assert!(vault.is_ok());

        cleanup_vault(temp_dir);
    }

    #[test]
    fn test_vault_save_and_load_profile() {
        let temp_dir = create_unique_temp_dir();
        let config = VaultConfig::new("test_account", temp_dir.clone());
        let vault = VaultStore::open(config).expect("Failed to open vault");

        let profile = Profile::new("test_profile", vec![1, 2, 3, 4, 5]);
        let result = vault.save_profile(&profile);
        assert!(result.is_ok(), "Failed to save profile: {:?}", result.err());

        let loaded = vault.load_profile(&profile.id);
        assert!(loaded.is_ok());
        let loaded = loaded.unwrap().unwrap();
        assert_eq!(loaded.id, profile.id);
        assert_eq!(loaded.name, profile.name);
        assert_eq!(loaded.data, profile.data);

        cleanup_vault(temp_dir);
    }

    #[test]
    fn test_vault_update_profile() {
        let temp_dir = create_unique_temp_dir();
        let config = VaultConfig::new("test_account", temp_dir.clone());
        let vault = VaultStore::open(config).expect("Failed to open vault");

        let mut profile = Profile::new("test_profile", vec![1, 2, 3]);
        let result = vault.save_profile(&profile);
        assert!(result.is_ok());

        profile.update_data(vec![10, 20, 30, 40]);
        let result = vault.save_profile(&profile);
        assert!(result.is_ok());

        let loaded = vault.load_profile(&profile.id).unwrap().unwrap();
        assert_eq!(loaded.data, vec![10, 20, 30, 40]);
        assert_eq!(loaded.version, 2);

        cleanup_vault(temp_dir);
    }

    #[test]
    fn test_vault_delete_profile() {
        let temp_dir = create_unique_temp_dir();
        let config = VaultConfig::new("test_account", temp_dir.clone());
        let vault = VaultStore::open(config).expect("Failed to open vault");

        let profile = Profile::new("test_profile", vec![1, 2, 3]);
        vault.save_profile(&profile).unwrap();

        let result = vault.delete_profile(&profile.id);
        assert!(result.is_ok());

        let loaded = vault.load_profile(&profile.id).unwrap();
        assert!(loaded.is_none());

        cleanup_vault(temp_dir);
    }

    #[test]
    fn test_vault_delete_nonexistent() {
        let temp_dir = create_unique_temp_dir();
        let config = VaultConfig::new("test_account", temp_dir.clone());
        let vault = VaultStore::open(config).expect("Failed to open vault");

        let result = vault.delete_profile("nonexistent_id");
        assert!(result.is_err());

        cleanup_vault(temp_dir);
    }

    #[test]
    fn test_vault_list_profiles() {
        let temp_dir = create_unique_temp_dir();
        let config = VaultConfig::new("test_account", temp_dir.clone());
        let vault = VaultStore::open(config).expect("Failed to open vault");

        let profile1 = Profile::new("profile_1", vec![1]);
        let profile2 = Profile::new("profile_2", vec![2]);
        let profile3 = Profile::new("profile_3", vec![3]);

        vault.save_profile(&profile1).unwrap();
        vault.save_profile(&profile2).unwrap();
        vault.save_profile(&profile3).unwrap();

        let profiles = vault.list_profiles().unwrap();
        assert_eq!(profiles.len(), 3);

        cleanup_vault(temp_dir);
    }

    #[test]
    fn test_vault_with_sqlcipher_key() {
        // Note: SQLCipher bundled with rusqlite requires proper initialization
        // This test verifies vault can be opened with SQLCipher key
        let temp_dir = create_unique_temp_dir();

        let sqlcipher_key = vec![0u8; 32];
        let config = VaultConfig {
            path: temp_dir.clone(),
            account_id: "test_account".to_string(),
            sqlcipher_key: Some(sqlcipher_key),
        };

        let vault = VaultStore::open(config);
        // Vault opens successfully with SQLCipher key set
        assert!(vault.is_ok(), "Failed to open vault with SQLCipher key");

        // Verify vault state is Unlocked
        assert_eq!(vault.unwrap().state(), VaultState::Unlocked);

        cleanup_vault(temp_dir);
    }

    #[test]
    fn test_vault_lock() {
        let temp_dir = create_unique_temp_dir();
        let config = VaultConfig::new("test_account", temp_dir.clone());
        let mut vault = VaultStore::open(config).expect("Failed to open vault");

        let profile = Profile::new("test_profile", vec![1, 2, 3]);
        vault.save_profile(&profile).expect("Failed to save profile");

        vault.lock();

        assert_eq!(vault.state(), VaultState::Locked);

        cleanup_vault(temp_dir);
    }
}
