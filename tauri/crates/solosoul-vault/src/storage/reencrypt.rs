//! P008: `VaultStore::reencrypt_all`（整库换钥重加密）独立子模块。
//! 本模块实现换钥全量重写：每表「旧钥解密 → 新钥加密」，单事务原子提交，
//! 任一步失败整体回滚（N-2，避免「部分行已换新钥、失败行仍为旧钥」的混态）。

use crate::encryption::{
    decrypt_field, decrypt_text_field, encrypt_field, encrypt_text_field, DataEncryptionKey,
};
use crate::VaultStore;

use super::rewrite_table;

impl VaultStore {
    /// 整库换钥重加密：全部表用 `new_key` 重写（profiles / objects / trash_items /
    /// object_snapshots / user_templates / audit_log）。
    ///
    /// 调用方（改密/KDF 升级）负责在调用前已验证 `old_key` 可解密全部数据。
    /// 持连接锁 + 单事务；任一行失败整体回滚。
    pub fn reencrypt_all(
        &self,
        old_key: &DataEncryptionKey,
        new_key: &DataEncryptionKey,
    ) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;

        let result: Result<(), String> = (|| {
            // 每表「旧钥解密 → 新钥加密」（换钥，全量重写）
            rewrite_table(
                &tx,
                "SELECT id, data FROM profiles",
                "UPDATE profiles SET data = ?1 WHERE id = ?2",
                "profiles",
                true,
                |row| {
                    let data: Vec<u8> = row.get(1).map_err(|e| e.to_string())?;
                    let plain = decrypt_field(old_key, &data)?;
                    let encrypted = encrypt_field(new_key, &plain)?;
                    Ok(Some(vec![rusqlite::types::Value::Blob(encrypted)]))
                },
            )?;

            rewrite_table(
                &tx,
                "SELECT id, properties, property_labels FROM objects",
                "UPDATE objects SET properties = ?1, property_labels = ?2 WHERE id = ?3",
                "objects",
                true,
                |row| {
                    let properties: String = row.get(1).map_err(|e| e.to_string())?;
                    let labels: Option<String> = row.get(2).map_err(|e| e.to_string())?;
                    let plain_props = decrypt_text_field(old_key, &properties)?;
                    let encrypted_props = encrypt_text_field(new_key, &plain_props)?;
                    let plain_labels = labels
                        .as_deref()
                        .map(|l| decrypt_text_field(old_key, l))
                        .transpose()?;
                    let encrypted_labels = plain_labels
                        .map(|l| encrypt_text_field(new_key, &l))
                        .transpose()?
                        .unwrap_or_default();
                    Ok(Some(vec![
                        rusqlite::types::Value::Text(encrypted_props),
                        rusqlite::types::Value::Text(encrypted_labels),
                    ]))
                },
            )?;

            rewrite_table(
                &tx,
                "SELECT id, data FROM trash_items",
                "UPDATE trash_items SET data = ?1 WHERE id = ?2",
                "trash_items",
                true,
                |row| {
                    let data: Vec<u8> = row.get(1).map_err(|e| e.to_string())?;
                    let plain = decrypt_field(old_key, &data)?;
                    let encrypted = encrypt_field(new_key, &plain)?;
                    Ok(Some(vec![rusqlite::types::Value::Blob(encrypted)]))
                },
            )?;

            rewrite_table(
                &tx,
                "SELECT id, data FROM object_snapshots",
                "UPDATE object_snapshots SET data = ?1 WHERE id = ?2",
                "object_snapshots",
                true,
                |row| {
                    let data: Vec<u8> = row.get(1).map_err(|e| e.to_string())?;
                    let plain = decrypt_field(old_key, &data)?;
                    let encrypted = encrypt_field(new_key, &plain)?;
                    Ok(Some(vec![rusqlite::types::Value::Blob(encrypted)]))
                },
            )?;

            rewrite_table(
                &tx,
                "SELECT id, properties_json FROM user_templates",
                "UPDATE user_templates SET properties_json = ?1 WHERE id = ?2",
                "user_templates",
                true,
                |row| {
                    let props_json: String = row.get(1).map_err(|e| e.to_string())?;
                    let plain = decrypt_text_field(old_key, &props_json)?;
                    let encrypted = encrypt_text_field(new_key, &plain)?;
                    Ok(Some(vec![rusqlite::types::Value::Text(encrypted)]))
                },
            )?;

            rewrite_table(
                &tx,
                "SELECT id, details, entity_name FROM audit_log",
                "UPDATE audit_log SET details = ?1, entity_name = ?2 WHERE id = ?3",
                "audit_log",
                true,
                |row| {
                    let details: Option<String> = row.get(1).map_err(|e| e.to_string())?;
                    let entity_name: Option<String> = row.get(2).map_err(|e| e.to_string())?;
                    let plain_details = details
                        .as_deref()
                        .map(|d| decrypt_text_field(old_key, d))
                        .transpose()?;
                    let encrypted_details = plain_details
                        .map(|d| encrypt_text_field(new_key, &d))
                        .transpose()?
                        .unwrap_or_default();
                    let plain_name = entity_name
                        .as_deref()
                        .map(|n| decrypt_text_field(old_key, n))
                        .transpose()?;
                    let encrypted_name = plain_name
                        .map(|n| encrypt_text_field(new_key, &n))
                        .transpose()?
                        .unwrap_or_default();
                    Ok(Some(vec![
                        rusqlite::types::Value::Text(encrypted_details),
                        rusqlite::types::Value::Text(encrypted_name),
                    ]))
                },
            )?;

            Ok(())
        })();

        // N-2: 仅在全部解密+重加密成功时提交；任一行失败则整体回滚（丢弃 tx 触发），
        // 避免"部分行已换新钥、失败行仍为旧钥"的混态——混态会令改密/KDF 升级后
        // 账户部分数据永久不可解密。
        match result {
            Ok(()) => {
                tx.commit().map_err(|e| e.to_string())?;
                tracing::info!("Vault re-encryption completed successfully");
                Ok(())
            }
            Err(e) => {
                tracing::error!("Vault re-encryption failed, transaction rolled back: {}", e);
                Err(e)
            }
        }
    }
}
