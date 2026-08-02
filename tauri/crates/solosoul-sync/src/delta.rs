//! Delta generation and application for peer-to-peer sync.
//!
//! Converts between the vault's `VaultSyncRecord` representation and the wire
//! `SyncRecord` format, applying LWW conflict resolution per record.

use crate::hlc::Hlc;
use crate::protocol::SyncRecord;
use crate::types::{ApplyStats, ConflictRecord};
use solosoul_vault::{BorrowedSyncRecord, RecordHlc, SyncWatermark, VaultStore};

/// Tables synchronized in the first milestone (attachments excluded).
pub const SYNC_TABLES: &[&str] = &["profiles", "objects", "user_templates", "trash_items"];

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

pub fn hlc_to_sync_watermark(hlc: &Hlc) -> crate::hlc::SyncWatermark {
    crate::hlc::SyncWatermark {
        wall_time_ms: hlc.wall_time_ms,
        counter: hlc.counter,
        node_id: hlc.node_id,
    }
}

pub fn watermark_to_vault(wm: &crate::hlc::SyncWatermark) -> SyncWatermark {
    SyncWatermark {
        wall_time_ms: wm.wall_time_ms,
        counter: wm.counter,
        node_id: hex::encode(wm.node_id),
    }
}

/// A single page of delta records returned by `generate_delta_paginated`.
#[derive(Debug, Clone)]
pub struct DeltaPage {
    pub records: Vec<SyncRecord>,
    pub finished: bool,
}

/// Generate a paginated page of records for a table that are newer than the peer watermark.
pub fn generate_delta_paginated(
    store: &VaultStore,
    table: &str,
    watermark: &crate::hlc::SyncWatermark,
    account_id: &str,
    local_node_id: &str,
    limit: usize,
    offset: usize,
) -> Result<DeltaPage, String> {
    let vault_watermark = watermark_to_vault(watermark);
    let changes = store.list_sync_changes_since_paginated(
        table,
        &vault_watermark,
        account_id,
        local_node_id,
        limit,
        offset,
    )?;
    let finished = changes.len() < limit;
    let records = changes
        .into_iter()
        .map(|rec| SyncRecord {
            id: rec.id,
            table: rec.table,
            data: rec.data,
            hlc: record_hlc_to_hlc(&rec.hlc),
            deleted: rec.deleted,
        })
        .collect();
    Ok(DeltaPage { records, finished })
}

/// Compute the maximum HLC in a non-empty list of records.
pub fn max_record_hlc(records: &[SyncRecord]) -> Option<Hlc> {
    records.iter().max_by_key(|r| r.hlc).map(|r| r.hlc)
}

/// Apply a batch of records for a single table.
///
/// P115: 整批单事务应用——借用视图零克隆传给 `apply_sync_records_batch`，
/// 每条记录只查一次 HLC（结果带写前本地 HLC 供冲突报告复用），不再逐条克隆 JSON。
pub fn apply_sync_records(
    store: &VaultStore,
    table: &str,
    records: &[SyncRecord],
    local_node_id: &str,
) -> Result<ApplyStats, String> {
    let mut stats = ApplyStats::default();
    let table_stats = stats.per_table.entry(table.to_string()).or_default();

    // 借用视图需要持有被引用的 `RecordHlc`，先批量转换一次。
    let hlcs: Vec<RecordHlc> = records.iter().map(|r| hlc_to_record_hlc(&r.hlc)).collect();
    let borrowed: Vec<BorrowedSyncRecord> = records
        .iter()
        .zip(hlcs.iter())
        .map(|(rec, hlc)| BorrowedSyncRecord {
            id: &rec.id,
            table: &rec.table,
            data: &rec.data,
            hlc,
            deleted: rec.deleted,
        })
        .collect();

    let outcomes = store.apply_sync_records_batch(&borrowed, local_node_id)?;

    for (rec, outcome) in records.iter().zip(outcomes.iter()) {
        stats.examined += 1;
        table_stats.examined += 1;
        if let Some(err) = &outcome.error {
            stats.skipped += 1;
            table_stats.skipped += 1;
            stats.errors.push(format!("{}: {}", rec.id, err));
            continue;
        }
        if outcome.applied {
            stats.applied += 1;
            table_stats.applied += 1;
            continue;
        }
        stats.skipped += 1;
        table_stats.skipped += 1;
        // A skip is a conflict when the local record is strictly newer than the remote one.
        if let Some(local) = &outcome.local_hlc {
            let local = record_hlc_to_hlc(local);
            if local > rec.hlc {
                stats.conflicts.push(ConflictRecord {
                    table: rec.table.clone(),
                    id: rec.id.clone(),
                    local_hlc: local,
                    remote_hlc: rec.hlc,
                    winner: "local".to_string(),
                });
                // 持久化冲突记录，供用户在冲突 UI 中查看并解决。
                let local_record_hlc = hlc_to_record_hlc(&local);
                let remote_record_hlc = hlc_to_record_hlc(&rec.hlc);
                let local_data = store
                    .get_sync_conflict_local_data(&rec.table, &rec.id)
                    .unwrap_or_default()
                    .unwrap_or_else(|| serde_json::Value::Object(Default::default()));
                if let Err(e) = store.save_sync_conflict(
                    &rec.table,
                    &rec.id,
                    &local_record_hlc,
                    &remote_record_hlc,
                    &local_data,
                    &rec.data,
                    rec.deleted,
                ) {
                    tracing::warn!("save_sync_conflict failed: {}", e);
                }
            }
        }
    }
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hlc::Hlc;
    use solosoul_vault::{Profile, VaultConfig, VaultStore};
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
        let page = generate_delta_paginated(
            &vault_a,
            "profiles",
            &watermark,
            "acc_delta",
            "node_a",
            usize::MAX,
            0,
        )
        .unwrap();
        assert_eq!(page.records.len(), 1);

        let stats = apply_sync_records(&vault_b, "profiles", &page.records, "node_b").unwrap();
        assert_eq!(stats.applied, 1);

        let synced = vault_b.load_profile("p1").unwrap().unwrap();
        assert_eq!(synced.name, "Travel");
        assert_eq!(synced.data, b"encrypted payload");
    }
}
