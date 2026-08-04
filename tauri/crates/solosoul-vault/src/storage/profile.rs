//! Profile 域 —— 自 `storage.rs` 拆分（P223-② 表域拆分第八域，收尾）。
//!
//! 本模块承载 `VaultStore` 的 Profile CRUD 方法（原 storage.rs 985-1068 + 1094-1120 行，
//! 逐行搬运零行为变更）：`save_profile`（直锁 conn + `save_profile_tx` 私有助手）/
//! `load_profile`（PROFILE_LOAD_SQL + `OptionalExtension`）/ `delete_profile`
//! （硬删 + `record_tombstone`）/ `list_profiles`（ProfileSummary 列表）。
//!
//! **非连续提取说明**：sync 节点状态方法 `get|set_sync_node_id` / `get|set_sync_secret_key`
//! 原位于 delete_profile 与 list_profiles 之间，属 sync 元数据，有意留在根模块
//! （不属 Profile 域）。
//!
//! 共享设施：`data_key()` 经根模块隐私向下可见；`PROFILE_LOAD_SQL` / `PROFILE_SAVE_SQL`
//! 为根模块 SQL 常量（`super::`）；`encrypt_field` / `decrypt_field` +
//! `DataEncryptionKey` 从 `crate::encryption` 导入；`record_tombstone` 属 sync_meta 域
//! pub(crate)。`save_profile_tx` 提 `pub(crate)`（sync_apply.rs 兄弟域
//! `apply_profile_sync_record_tx` 调用），其余 4 个 pub API 可见性不变
//! （src-tauri commands/backup.rs + CLI + 根测试模块）。

use rusqlite::{params, Connection, OptionalExtension};

use super::{with_tx, VaultStore, PROFILE_LOAD_SQL, PROFILE_SAVE_SQL};
use crate::encryption::{decrypt_field, encrypt_field, DataEncryptionKey};
use crate::{Profile, ProfileSummary};

impl VaultStore {
    pub fn save_profile(&self, profile: &Profile) -> Result<(), String> {
        let key = self.data_key()?;
        // 方案 B（R-3 根治）：本地写统一生成并落库 HLC。生成需读 sync_hlc 最大值，
        // 必须在持锁前调用（new_local_hlc 内部自行锁 conn）。save_profile_tx 被
        // sync_apply 远端应用路径复用（自写 HLC），故本地写 HLC 在入口事务内落。
        let hlc = self.new_local_hlc()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        with_tx(
            conn,
            "Failed to begin transaction",
            "Failed to commit transaction",
            |c| {
                Self::save_profile_tx(c, &key, profile)?;
                Self::set_record_hlc_tx(c, "profiles", &profile.id, &hlc)?;
                Ok(())
            },
        )
    }

    /// P115: 事务内保存 Profile（连接由调用方持有，批量应用单事务内复用）。
    /// P213: 事务内保存 Profile（连接由调用方持有，批量应用单事务内复用）。
    pub(crate) fn save_profile_tx(
        conn: &mut Connection,
        key: &DataEncryptionKey,
        profile: &Profile,
    ) -> Result<(), String> {
        let now = chrono::Utc::now().to_rfc3339();
        let encrypted_data = encrypt_field(key, &profile.data)?;
        let mut stmt = conn
            .prepare_cached(PROFILE_SAVE_SQL)
            .map_err(|e| format!("Failed to prepare save_profile: {}", e))?;
        stmt.execute(params![
            profile.id,
            profile.name,
            encrypted_data,
            profile.created_at.to_rfc3339(),
            now,
            profile.version
        ])
        .map_err(|e| format!("Failed to save profile: {}", e))?;
        Ok(())
    }

    pub fn load_profile(&self, id: &str) -> Result<Option<Profile>, String> {
        let key = self.data_key()?;
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn
            .prepare_cached(PROFILE_LOAD_SQL)
            .map_err(|e| format!("Failed to prepare: {}", e))?;
        let result = stmt
            .query_row(params![id], |row| {
                let created_str: String = row.get(3)?;
                let updated_str: String = row.get(4)?;
                let raw_data: Vec<u8> = row.get(2)?;
                let data = decrypt_field(&key, &raw_data).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        2,
                        rusqlite::types::Type::Blob,
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Profile decryption failed: {}", e),
                        )),
                    )
                })?;
                Ok(Profile {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    data,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_str)
                        .map(|d| d.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&updated_str)
                        .map(|d| d.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    version: row.get(5)?,
                })
            })
            .optional()
            .map_err(|e| format!("Failed to load profile: {}", e))?;
        Ok(result)
    }

    pub fn delete_profile(&self, id: &str) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let affected = conn
            .execute("DELETE FROM profiles WHERE id = ?1", params![id])
            .map_err(|e| format!("Failed to delete: {}", e))?;
        if affected == 0 {
            return Err("Profile not found".to_string());
        }
        drop(guard);
        self.record_tombstone("profiles", id)?;
        Ok(())
    }

    pub fn list_profiles(&self) -> Result<Vec<ProfileSummary>, String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let mut stmt = conn.prepare(
        "SELECT id, name, created_at, updated_at, version FROM profiles ORDER BY updated_at DESC"
    ).map_err(|e| format!("Failed to prepare: {}", e))?;
        let profiles = stmt
            .query_map([], |row| {
                let created_str: String = row.get(2)?;
                let updated_str: String = row.get(3)?;
                Ok(ProfileSummary {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_str)
                        .map(|d| d.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&updated_str)
                        .map(|d| d.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    version: row.get(4)?,
                })
            })
            .map_err(|e| format!("Failed to query: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect: {}", e))?;
        Ok(profiles)
    }
}
