//! Delta generation and application for peer-to-peer sync.
//!
//! Converts between the vault's `VaultSyncRecord` representation and the wire
//! `SyncRecord` format, applying LWW conflict resolution per record.

use crate::hlc::Hlc;
use crate::protocol::SyncRecord;
use solosoul_vault::{RecordHlc, SyncWatermark, VaultStore, VaultSyncRecord};
use std::collections::HashMap;

/// Tables synchronized in the first milestone (attachments excluded).
pub const SYNC_TABLES: &[&str] = &["profiles", "objects", "user_templates", "trash_items"];

/// Statistics returned after applying a sync batch.
#[derive(Debug, Clone, Default)]
pub struct ApplyStats {
    pub examined: u64,
    pub applied: u64,
    pub skipped: u64,
    pub errors: Vec<String>,
}

fn hlc_to_record_hlc(hlc: &Hlc) -> RecordHlc {
    RecordHlc {
        wall_time_ms: hlc.wall_time_ms,
        counter: hlc.counter,
        node_id: hlc.node_id_string(),
    }
}

fn record_hlc_to_hlc(hlc: &RecordHlc) -> Hlc {
    Hlc::new(hlc.wall_time_ms, hlc.counter, &hlc.node_id)
}

fn watermark_to_vault(wm: &crate::hlc::SyncWatermark) -> SyncWatermark {
    SyncWatermark {
        wall_time_ms: wm.wall_time_ms,
        counter: wm.counter,
        node_id: hex::encode(wm.node_id),
    }
}

/// Generate the set of records for a table that are newer than the peer watermark.
pub fn generate_delta(
    store: &VaultStore,
    table: &str,
    watermark: &crate::hlc::SyncWatermark,
    account_id: &str,
    local_node_id: &str,
) -> Result<Vec<SyncRecord>, String> {
    let vault_watermark = watermark_to_vault(watermark);
    let changes =
        store.list_sync_changes_since(table, &vault_watermark, account_id, local_node_id)?;
    Ok(changes
        .into_iter()
        .map(|rec| SyncRecord {
            id: rec.id,
            table: rec.table,
            data: rec.data,
            hlc: record_hlc_to_hlc(&rec.hlc),
            deleted: rec.deleted,
        })
        .collect())
}

/// Apply a batch of records for a single table.
pub fn apply_sync_records(
    store: &VaultStore,
    _table: &str,
    records: &[SyncRecord],
    local_node_id: &str,
) -> Result<ApplyStats, String> {
    let mut stats = ApplyStats::default();
    for rec in records {
        stats.examined += 1;
        let vault_rec = VaultSyncRecord {
            id: rec.id.clone(),
            table: rec.table.clone(),
            data: rec.data.clone(),
            hlc: hlc_to_record_hlc(&rec.hlc),
            deleted: rec.deleted,
        };
        match store.apply_sync_record(&vault_rec, local_node_id) {
            Ok(true) => stats.applied += 1,
            Ok(false) => stats.skipped += 1,
            Err(e) => {
                stats.skipped += 1;
                stats.errors.push(format!("{}: {}", rec.id, e));
            }
        }
    }
    Ok(stats)
}

/// Apply records grouped by table in the canonical order.
pub fn apply_sync_batch(
    store: &VaultStore,
    records_by_table: HashMap<String, Vec<SyncRecord>>,
    local_node_id: &str,
) -> Result<ApplyStats, String> {
    let mut total = ApplyStats::default();
    for table in SYNC_TABLES {
        if let Some(records) = records_by_table.get(*table) {
            let stats = apply_sync_records(store, table, records, local_node_id)?;
            total.examined += stats.examined;
            total.applied += stats.applied;
            total.skipped += stats.skipped;
            total.errors.extend(stats.errors);
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hlc::Hlc;
    use solosoul_vault::{Profile, VaultConfig, VaultStore};
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn open_test_vault(account_id: &str, path: std::path::PathBuf) -> VaultStore {
        let config = VaultConfig::new(account_id, path).with_data_key([1u8; 32]);
        VaultStore::open(config).unwrap()
    }

    #[test]
    fn test_watermark_roundtrip() {
        let hlc = Hlc::new(100, 5, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let wm = crate::hlc::SyncWatermark::from_hlc(&hlc);
        let vwm = watermark_to_vault(&wm);
        assert_eq!(vwm.wall_time_ms, 100);
        assert_eq!(vwm.counter, 5);
    }

    #[test]
    fn test_delta_profile_roundtrip() {
        let dir_a = TempDir::new().unwrap();
        let dir_b = TempDir::new().unwrap();
        let vault_a = open_test_vault("acc_delta", dir_a.path().to_path_buf());
        let vault_b = open_test_vault("acc_delta", dir_b.path().to_path_buf());

        let profile = Profile {
            id: "p1".to_string(),
            name: "Travel".to_string(),
            data: b"encrypted payload".to_vec(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            version: 1,
        };
        vault_a.save_profile(&profile).unwrap();

        let watermark = crate::hlc::SyncWatermark::zero();
        let records =
            generate_delta(&vault_a, "profiles", &watermark, "acc_delta", "node_a").unwrap();
        assert_eq!(records.len(), 1);

        let mut batch = HashMap::new();
        batch.insert("profiles".to_string(), records);
        let stats = apply_sync_batch(&vault_b, batch, "node_b").unwrap();
        assert_eq!(stats.applied, 1);

        let synced = vault_b.load_profile("p1").unwrap().unwrap();
        assert_eq!(synced.name, "Travel");
        assert_eq!(synced.data, b"encrypted payload");
    }
}
