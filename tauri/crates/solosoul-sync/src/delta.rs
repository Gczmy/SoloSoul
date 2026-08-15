//! Delta generation and application for peer-to-peer sync.
//!
//! Converts between the vault's `VaultSyncRecord` representation and the wire
//! `SyncRecord` format, applying LWW conflict resolution per record.

use crate::hlc::Hlc;
use crate::protocol::SyncRecord;
use crate::types::{ApplyStats, ConflictRecord};
use solosoul_vault::{BorrowedSyncRecord, RecordHlc, SyncWatermark, VaultStore};

/// Tables synchronized in the first milestone (attachments excluded).
/// P004: 加入 llm_conversations（会话行存储，随设备同步）。
pub const SYNC_TABLES: &[&str] = &[
    "profiles",
    "objects",
    "user_templates",
    "trash_items",
    "llm_conversations",
];

/// 簿记字段：随每次编辑/同步应用变化、与内容差异无关。
/// 冲突自动消解比较时剥除，避免「内容一致、仅版本/时间不同」的假冲突
/// （两台设备修改同一对象的不同字段时，version/updated_at 必然不同，
/// 但它们是 HLC 时间裁决的副产物，不是用户可决策的内容差异）。
/// 覆盖 ObjectRecord 的 snake_case（updated_at）与 Profile/UserTemplate
/// 线格式的 camelCase（updatedAt）两种命名。
/// 已知限制（保守安全）：Profile 的 local 快照为 snake_case 序列化且含 id 键，
/// 线格式为 camelCase 子集，键形差异使两侧比较通常不相等——profile 冲突照旧
/// 记录（不自动消解，行为与修复前一致）；主场景（objects）已闭环。
const BOOKKEEPING_KEYS: &[&str] = &["version", "updated_at", "updatedAt"];

/// 剥除对象快照中的簿记字段后返回（用于判断两侧内容是否已收敛）。
/// 非对象值（数组/标量/null）原样返回。
fn strip_bookkeeping(value: &serde_json::Value) -> serde_json::Value {
    let Some(obj) = value.as_object() else {
        return value.clone();
    };
    let mut out = serde_json::Map::new();
    for (k, v) in obj {
        if !BOOKKEEPING_KEYS.contains(&k.as_str()) {
            out.insert(k.clone(), v.clone());
        }
    }
    serde_json::Value::Object(out)
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
    last_row_id: Option<&str>,
) -> Result<DeltaPage, String> {
    let vault_watermark = watermark_to_vault(watermark);
    let changes = store.list_sync_changes_since_paginated(
        table,
        &vault_watermark,
        account_id,
        local_node_id,
        limit,
        last_row_id,
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

    // P016: 冲突候选收集（本地 HLC 严格新于远端）——本地数据抓取与持久化延后到
    // 批量阶段：本地快照按表单查询批量取（objects 走 load_objects_batch），
    // 冲突写入单事务批量 commit，避免大量冲突时 N 次逐条解密/写事务。
    let mut conflict_candidates: Vec<(&SyncRecord, RecordHlc)> = Vec::new();
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
                conflict_candidates.push((rec, hlc_to_record_hlc(&local)));
            }
        }
    }

    // P016: 批量阶段——本地数据单查询批量取 + 自动消解判定 + 单事务批量持久化。
    if !conflict_candidates.is_empty() {
        let candidate_ids: Vec<String> = conflict_candidates
            .iter()
            .map(|(rec, _)| rec.id.clone())
            .collect();
        let local_datas = store.get_sync_conflict_local_data_batch(table, &candidate_ids)?;
        let mut conflict_entries: Vec<solosoul_vault::SyncConflictBatchEntry> = Vec::new();
        for (rec, local_hlc) in &conflict_candidates {
            let local_record_hlc = (*local_hlc).clone();
            let remote_record_hlc = hlc_to_record_hlc(&rec.hlc);
            let local_data = local_datas
                .get(&rec.id)
                .cloned()
                .flatten()
                .unwrap_or_else(|| serde_json::Value::Object(Default::default()));
            // 自动消解：剥除簿记字段（version/updated_at，随每次编辑与同步应用
            // 变化、与内容差异无关）后两侧内容一致 → LWW 胜者已收敛，无数据丢失，
            // 不记录冲突（避免「内容一样、仅版本/时间不同」的假冲突）。
            // 远程为删除墓碑（deleted）时删除与否是真实决策，仍照常记录冲突。
            // 会话（llm_conversations）本地快照是解密 JSON、远端是线格式信封
            // {id, accountId, data: <base64>, updatedAt}，键形错配不能直接比较，
            // 且信封 data 是随机 nonce 加密 blob（base64 逐设备不同），须先解密
            // 远端信封为明文 JSON 再与本地快照比较（无 data/解密失败保守记录冲突）。
            let content_matches = if rec.table == "llm_conversations" {
                match store.conversation_remote_content(&rec.data) {
                    Ok(Some(remote_plain)) => {
                        strip_bookkeeping(&local_data) == strip_bookkeeping(&remote_plain)
                    }
                    _ => false,
                }
            } else {
                strip_bookkeeping(&local_data) == strip_bookkeeping(&rec.data)
            };
            if !rec.deleted && content_matches {
                continue;
            }
            stats.conflicts.push(ConflictRecord {
                table: rec.table.clone(),
                id: rec.id.clone(),
                local_hlc: record_hlc_to_hlc(local_hlc),
                remote_hlc: rec.hlc,
                winner: "local".to_string(),
            });
            conflict_entries.push(solosoul_vault::SyncConflictBatchEntry {
                table: rec.table.clone(),
                record_id: rec.id.clone(),
                local_hlc: local_record_hlc,
                remote_hlc: remote_record_hlc,
                local_data,
                remote_data: rec.data.clone(),
                remote_deleted: rec.deleted,
            });
        }
        // 持久化冲突记录（单事务批量），供用户在冲突 UI 中查看并解决。
        if !conflict_entries.is_empty() {
            if let Err(e) = store.save_sync_conflicts_batch(&conflict_entries) {
                tracing::warn!("save_sync_conflicts_batch failed: {}", e);
            }
        }
    }
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hlc::Hlc;
    use solosoul_vault::{ObjectRecord, Profile, VaultConfig, VaultStore};
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
            None,
        )
        .unwrap();
        assert_eq!(page.records.len(), 1);

        let stats = apply_sync_records(&vault_b, "profiles", &page.records, "node_b").unwrap();
        assert_eq!(stats.applied, 1);

        let synced = vault_b.load_profile("p1").unwrap().unwrap();
        assert_eq!(synced.name, "Travel");
        assert_eq!(synced.data, b"encrypted payload");
    }

    // ── N-1: 生产编码路径下 keyset 分页必须无缺漏 ────────────────────────
    //
    // 会话层把本地节点规范化为 hex(parse_node_id_bytes(node_id)) 传入存储层，与
    // peer watermark 落库格式一致；否则 keyset 等值组分支永不触发（原始 UUID 与
    // hex 字节串不等），等 ms 回退行组跨页时会死循环或静默漏发。本测试走完整生产
    // 路径：generate_delta_paginated → watermark_to_vault 落库 → get_peer_watermark
    // 读回 → 解析回 sync 层，逐页收集 7 个同 updated_at 回退行，断言无缺漏、无重复。
    #[test]
    fn test_strip_bookkeeping_removes_only_bookkeeping_keys() {
        let v = serde_json::json!({
            "id": "o1",
            "name": "n",
            "version": 3,
            "updated_at": "2026-08-05T10:00:00Z",
            "updatedAt": "2026-08-05T09:00:00Z",
            "properties": { "a": 1 },
        });
        let out = strip_bookkeeping(&v);
        let obj = out.as_object().unwrap();
        assert!(!obj.contains_key("version"));
        assert!(!obj.contains_key("updated_at"));
        assert!(!obj.contains_key("updatedAt"));
        assert_eq!(obj["id"], "o1");
        assert_eq!(obj["properties"]["a"], 1);
        // 非对象值原样返回
        assert_eq!(
            strip_bookkeeping(&serde_json::json!([1, 2])),
            serde_json::json!([1, 2])
        );
    }

    /// 两台设备修改了同一对象的不同字段后被同步收敛：本地（接收方）内容与远程
    /// 一致、仅 version/updated_at 不同 → 自动消解，不产生冲突。
    #[test]
    fn test_conflict_auto_resolved_when_only_bookkeeping_differs() {
        let dir_a = TempDir::new().unwrap();
        let dir_b = TempDir::new().unwrap();
        let vault_a = open_test_vault("acc_converged", dir_a.path().to_path_buf());
        let vault_b = open_test_vault("acc_converged", dir_b.path().to_path_buf());

        let ts = "2026-08-05T10:00:00.000+00:00";
        vault_a
            .save_object(&ObjectRecord {
                id: "obj_1".to_string(),
                account_id: "acc_converged".to_string(),
                name: "测试".to_string(),
                section_type: "identity".to_string(),
                properties: serde_json::json!({ "姓名": "张三" }),
                sensitivity_level: "internal".to_string(),
                created_at: ts.to_string(),
                updated_at: "2026-08-05T09:00:00Z".to_string(),
                version: 9,
                ..Default::default()
            })
            .unwrap();

        // 本地（接收方）内容已收敛、版本/时间更新（HLC 严格新）——先用几个填充对象
        // 把本地 HLC wall 时间推到严格大于 vault_a 的记录（同一毫秒内的多次保存
        // wall 递增，保证跨 vault 比较确定）。
        for i in 0..3 {
            vault_b
                .save_object(&ObjectRecord {
                    id: format!("warm_{}", i),
                    account_id: "acc_converged".to_string(),
                    name: "warm".to_string(),
                    section_type: "identity".to_string(),
                    properties: serde_json::json!({}),
                    sensitivity_level: "internal".to_string(),
                    created_at: ts.to_string(),
                    updated_at: ts.to_string(),
                    version: 1,
                    ..Default::default()
                })
                .unwrap();
        }
        vault_b
            .save_object(&ObjectRecord {
                id: "obj_1".to_string(),
                account_id: "acc_converged".to_string(),
                name: "测试".to_string(),
                section_type: "identity".to_string(),
                properties: serde_json::json!({ "姓名": "张三" }),
                sensitivity_level: "internal".to_string(),
                created_at: ts.to_string(),
                updated_at: "2026-08-05T10:00:00Z".to_string(),
                version: 10,
                ..Default::default()
            })
            .unwrap();

        let watermark = crate::hlc::SyncWatermark::zero();
        let page = generate_delta_paginated(
            &vault_a,
            "objects",
            &watermark,
            "acc_converged",
            "node_a",
            usize::MAX,
            None,
        )
        .unwrap();
        assert_eq!(page.records.len(), 1);

        let stats = apply_sync_records(&vault_b, "objects", &page.records, "node_b").unwrap();
        assert_eq!(
            stats.conflicts.len(),
            0,
            "内容一致（仅 version/updated_at 不同）应自动消解，不产生冲突"
        );
        assert!(
            vault_b.list_sync_conflicts().unwrap().is_empty(),
            "持久化冲突表也应为空"
        );
    }

    /// 会话内容已收敛（本地赢 LWW）→ 自动消解，不产生假冲突。
    /// 本地快照（解密 JSON）与远端信封（{id, accountId, data: base64, updatedAt}）
    /// 键形错配，须解密远端信封为明文后再比较。
    #[test]
    fn test_conversation_conflict_auto_resolved_when_content_converged() {
        let dir_a = TempDir::new().unwrap();
        let dir_b = TempDir::new().unwrap();
        let vault_a = open_test_vault("acc_conv", dir_a.path().to_path_buf());
        let vault_b = open_test_vault("acc_conv", dir_b.path().to_path_buf());

        let conv = serde_json::json!({ "id": "conv_1", "name": "会话", "messages": [] })
            .to_string()
            .into_bytes();
        let ts = "2026-08-05T10:00:00.000+00:00";
        vault_a
            .save_conversation("acc_conv", "conv_1", ts, &conv)
            .unwrap();

        // 本地（接收方）内容已收敛、updated_at 更新——先用几个填充会话把本地 HLC
        // wall 时间推到严格大于 vault_a 的记录（与对象用例同法，保证跨 vault 比较确定）。
        for i in 0..3 {
            let filler =
                serde_json::json!({ "id": format!("warm_{i}"), "name": "warm", "messages": [] })
                    .to_string()
                    .into_bytes();
            vault_b
                .save_conversation("acc_conv", &format!("warm_{i}"), ts, &filler)
                .unwrap();
        }
        vault_b
            .save_conversation("acc_conv", "conv_1", "2026-08-05T11:00:00Z", &conv)
            .unwrap();

        let watermark = crate::hlc::SyncWatermark::zero();
        let page = generate_delta_paginated(
            &vault_a,
            "llm_conversations",
            &watermark,
            "acc_conv",
            "node_a",
            usize::MAX,
            None,
        )
        .unwrap();
        assert_eq!(page.records.len(), 1);

        let stats =
            apply_sync_records(&vault_b, "llm_conversations", &page.records, "node_b").unwrap();
        assert_eq!(
            stats.conflicts.len(),
            0,
            "会话内容一致（仅 updatedAt 不同）应自动消解，不产生假冲突"
        );
        assert!(
            vault_b.list_sync_conflicts().unwrap().is_empty(),
            "持久化冲突表也应为空"
        );
    }

    /// 会话内容不同（本地赢 LWW）→ 仍应记录冲突（真实内容差异不可自动消解）。
    #[test]
    fn test_conversation_conflict_recorded_when_content_differs() {
        let dir_a = TempDir::new().unwrap();
        let dir_b = TempDir::new().unwrap();
        let vault_a = open_test_vault("acc_conv", dir_a.path().to_path_buf());
        let vault_b = open_test_vault("acc_conv", dir_b.path().to_path_buf());

        let ts = "2026-08-05T10:00:00.000+00:00";
        let remote_conv = serde_json::json!({ "id": "conv_1", "name": "远端内容", "messages": [] })
            .to_string()
            .into_bytes();
        vault_a
            .save_conversation("acc_conv", "conv_1", ts, &remote_conv)
            .unwrap();

        // 本地（接收方）内容与远端不同、updated_at 更新（本地赢 LWW）——先用几个
        // 填充会话把本地 HLC wall 时间推到严格大于 vault_a 的记录（与收敛用例同法）。
        for i in 0..3 {
            let filler =
                serde_json::json!({ "id": format!("warm_{i}"), "name": "warm", "messages": [] })
                    .to_string()
                    .into_bytes();
            vault_b
                .save_conversation("acc_conv", &format!("warm_{i}"), ts, &filler)
                .unwrap();
        }
        let local_conv = serde_json::json!({ "id": "conv_1", "name": "本地内容", "messages": [] })
            .to_string()
            .into_bytes();
        vault_b
            .save_conversation("acc_conv", "conv_1", "2026-08-05T11:00:00Z", &local_conv)
            .unwrap();

        let watermark = crate::hlc::SyncWatermark::zero();
        let page = generate_delta_paginated(
            &vault_a,
            "llm_conversations",
            &watermark,
            "acc_conv",
            "node_a",
            usize::MAX,
            None,
        )
        .unwrap();
        assert_eq!(page.records.len(), 1);

        let stats =
            apply_sync_records(&vault_b, "llm_conversations", &page.records, "node_b").unwrap();
        assert_eq!(
            stats.conflicts.len(),
            1,
            "会话内容不同是真实差异，应记录冲突（不自动消解）"
        );
        assert!(
            !vault_b.list_sync_conflicts().unwrap().is_empty(),
            "持久化冲突表应含该冲突"
        );
    }

    /// 远端会话信封解密失败（数据损坏/密钥不匹配）→ 保守退化，照常记录冲突
    /// （fail-safe：任何解析/解密错误都不得自动消解，防止误判丢失真实差异）。
    #[test]
    fn test_conversation_conflict_recorded_when_remote_decrypt_fails() {
        let dir_a = TempDir::new().unwrap();
        let dir_b = TempDir::new().unwrap();
        let vault_a = open_test_vault("acc_conv", dir_a.path().to_path_buf());
        let vault_b = open_test_vault("acc_conv", dir_b.path().to_path_buf());

        let ts = "2026-08-05T10:00:00.000+00:00";
        let conv = serde_json::json!({ "id": "conv_1", "name": "会话", "messages": [] })
            .to_string()
            .into_bytes();
        vault_a
            .save_conversation("acc_conv", "conv_1", ts, &conv)
            .unwrap();

        // 本地（接收方）内容更新（本地赢 LWW）
        for i in 0..3 {
            let filler =
                serde_json::json!({ "id": format!("warm_{i}"), "name": "warm", "messages": [] })
                    .to_string()
                    .into_bytes();
            vault_b
                .save_conversation("acc_conv", &format!("warm_{i}"), ts, &filler)
                .unwrap();
        }
        vault_b
            .save_conversation("acc_conv", "conv_1", "2026-08-05T11:00:00Z", &conv)
            .unwrap();

        let watermark = crate::hlc::SyncWatermark::zero();
        let page = generate_delta_paginated(
            &vault_a,
            "llm_conversations",
            &watermark,
            "acc_conv",
            "node_a",
            usize::MAX,
            None,
        )
        .unwrap();
        assert_eq!(page.records.len(), 1);

        // 篡改远端信封的 data（替换一个合法 base64 字符，长度不变仍可解码），
        // 解码后字节损坏 → 解密失败 → 保守记录冲突。LWW 检查先于写库，篡改数据
        // 不会落库（本地较新即跳过写入）。
        let mut record = page.records.into_iter().next().unwrap();
        if let Some(obj) = record.data.as_object_mut() {
            if let Some(data_str) = obj.get_mut("data").and_then(|v| v.as_str()) {
                let mut chars: Vec<u8> = data_str.as_bytes().to_vec();
                let idx = chars.iter().position(|&c| c == b'A').unwrap_or(0);
                chars[idx] = if chars[idx] == b'A' { b'B' } else { b'A' };
                obj.insert(
                    "data".to_string(),
                    serde_json::Value::String(String::from_utf8(chars).unwrap()),
                );
            }
        }

        let stats = apply_sync_records(&vault_b, "llm_conversations", &[record], "node_b").unwrap();
        assert_eq!(
            stats.conflicts.len(),
            1,
            "远端解密失败应保守记录冲突（fail-safe，不自动消解）"
        );
        assert!(
            !vault_b.list_sync_conflicts().unwrap().is_empty(),
            "持久化冲突表应含该冲突"
        );
    }

    /// 内容真实不同（同一字段值不同）→ 照常记录冲突。
    #[test]
    fn test_conflict_recorded_when_content_differs() {
        let dir_a = TempDir::new().unwrap();
        let dir_b = TempDir::new().unwrap();
        let vault_a = open_test_vault("acc_conflict", dir_a.path().to_path_buf());
        let vault_b = open_test_vault("acc_conflict", dir_b.path().to_path_buf());

        let ts = "2026-08-05T10:00:00.000+00:00";
        vault_a
            .save_object(&ObjectRecord {
                id: "obj_1".to_string(),
                account_id: "acc_conflict".to_string(),
                name: "测试".to_string(),
                section_type: "identity".to_string(),
                properties: serde_json::json!({ "姓名": "张三" }),
                sensitivity_level: "internal".to_string(),
                created_at: ts.to_string(),
                updated_at: "2026-08-05T09:00:00Z".to_string(),
                version: 9,
                ..Default::default()
            })
            .unwrap();
        for i in 0..3 {
            vault_b
                .save_object(&ObjectRecord {
                    id: format!("warm_{}", i),
                    account_id: "acc_conflict".to_string(),
                    name: "warm".to_string(),
                    section_type: "identity".to_string(),
                    properties: serde_json::json!({}),
                    sensitivity_level: "internal".to_string(),
                    created_at: ts.to_string(),
                    updated_at: ts.to_string(),
                    version: 1,
                    ..Default::default()
                })
                .unwrap();
        }
        vault_b
            .save_object(&ObjectRecord {
                id: "obj_1".to_string(),
                account_id: "acc_conflict".to_string(),
                name: "测试".to_string(),
                section_type: "identity".to_string(),
                properties: serde_json::json!({ "姓名": "李四" }),
                sensitivity_level: "internal".to_string(),
                created_at: ts.to_string(),
                updated_at: "2026-08-05T10:00:00Z".to_string(),
                version: 10,
                ..Default::default()
            })
            .unwrap();

        let watermark = crate::hlc::SyncWatermark::zero();
        let page = generate_delta_paginated(
            &vault_a,
            "objects",
            &watermark,
            "acc_conflict",
            "node_a",
            usize::MAX,
            None,
        )
        .unwrap();
        assert_eq!(page.records.len(), 1);

        let stats = apply_sync_records(&vault_b, "objects", &page.records, "node_b").unwrap();
        assert_eq!(
            stats.conflicts.len(),
            1,
            "内容真实不同（姓名不同）应照常记录冲突"
        );
        let conflicts = vault_b.list_sync_conflicts().unwrap();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].record_id, "obj_1");
    }

    /// 远程为删除墓碑（deleted=true）时，即使剥除簿记字段后内容一致，仍应记录冲突
    /// （删除与否是真实决策，不做自动消解）。
    #[test]
    fn test_conflict_recorded_for_tombstone_even_if_content_matches() {
        let dir_a = TempDir::new().unwrap();
        let dir_b = TempDir::new().unwrap();
        let vault_a = open_test_vault("acc_tomb", dir_a.path().to_path_buf());
        let vault_b = open_test_vault("acc_tomb", dir_b.path().to_path_buf());

        let ts = "2026-08-05T10:00:00.000+00:00";
        // 发送方：保存对象后硬删（产生墓碑记录，deleted=true, data=null）
        vault_a
            .save_object(&ObjectRecord {
                id: "obj_1".to_string(),
                account_id: "acc_tomb".to_string(),
                name: "测试".to_string(),
                section_type: "identity".to_string(),
                properties: serde_json::json!({ "姓名": "张三" }),
                sensitivity_level: "internal".to_string(),
                created_at: ts.to_string(),
                updated_at: ts.to_string(),
                version: 1,
                ..Default::default()
            })
            .unwrap();
        vault_a.delete_object("obj_1", false).unwrap();

        // 接收方：本地持有同内容对象，HLC 严格新（warm 推进 wall）
        for i in 0..3 {
            vault_b
                .save_object(&ObjectRecord {
                    id: format!("warm_{}", i),
                    account_id: "acc_tomb".to_string(),
                    name: "warm".to_string(),
                    section_type: "identity".to_string(),
                    properties: serde_json::json!({}),
                    sensitivity_level: "internal".to_string(),
                    created_at: ts.to_string(),
                    updated_at: ts.to_string(),
                    version: 1,
                    ..Default::default()
                })
                .unwrap();
        }
        vault_b
            .save_object(&ObjectRecord {
                id: "obj_1".to_string(),
                account_id: "acc_tomb".to_string(),
                name: "测试".to_string(),
                section_type: "identity".to_string(),
                properties: serde_json::json!({ "姓名": "张三" }),
                sensitivity_level: "internal".to_string(),
                created_at: ts.to_string(),
                updated_at: ts.to_string(),
                version: 1,
                ..Default::default()
            })
            .unwrap();

        let watermark = crate::hlc::SyncWatermark::zero();
        let page = generate_delta_paginated(
            &vault_a,
            "objects",
            &watermark,
            "acc_tomb",
            "node_a",
            usize::MAX,
            None,
        )
        .unwrap();
        assert_eq!(page.records.len(), 1);
        assert!(page.records[0].deleted, "发送方 delta 应为删除墓碑记录");

        let stats = apply_sync_records(&vault_b, "objects", &page.records, "node_b").unwrap();
        assert_eq!(
            stats.conflicts.len(),
            1,
            "删除墓碑是真实决策，即使内容一致也应照常记录冲突（不自动消解）"
        );
        assert_eq!(vault_b.list_sync_conflicts().unwrap().len(), 1);
    }

    #[test]
    fn test_generate_delta_paginated_keyset_production_encoding() {
        let dir = TempDir::new().unwrap();
        let vault = open_test_vault("acc_n1", dir.path().to_path_buf());
        let ts = "2026-08-01T12:00:00.000+00:00";
        for i in 1..=7usize {
            vault
                .save_object(&solosoul_vault::ObjectRecord {
                    id: format!("n1_{:02}", i),
                    account_id: "acc_n1".to_string(),
                    name: format!("n1_{:02}", i),
                    section_type: "identity".to_string(),
                    properties: serde_json::json!({ "k": i }),
                    sensitivity_level: "internal".to_string(),
                    created_at: ts.to_string(),
                    updated_at: ts.to_string(),
                    ..Default::default()
                })
                .unwrap();
        }

        // 与 send_paginated_deltas 相同的节点规范化（生产编码路径）
        let node_id = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";
        let local_hlc_node = hex::encode(Hlc::parse_node_id_bytes(node_id));
        let peer_node_id = "peer_x";

        let mut watermark = crate::hlc::SyncWatermark::zero();
        let mut last_row_id: Option<String> = None;
        let mut paged_ids: Vec<String> = Vec::new();
        loop {
            let page = generate_delta_paginated(
                &vault,
                "objects",
                &watermark,
                "acc_n1",
                &local_hlc_node,
                2,
                last_row_id.as_deref(),
            )
            .unwrap();
            if page.records.is_empty() && page.finished {
                break;
            }
            for rec in &page.records {
                paged_ids.push(rec.id.clone());
            }
            if let Some(max) = max_record_hlc(&page.records) {
                // 与会话层同款：落库 watermark_to_vault(hlc_to_sync_watermark(max))
                let vwm = watermark_to_vault(&hlc_to_sync_watermark(&max));
                vault
                    .update_peer_watermark(peer_node_id, "objects", &vwm)
                    .unwrap();
                // 读回并转回 sync 层（复用 session.rs 的 vault_to_watermark，防漂移）
                let stored = vault.get_peer_watermark(peer_node_id, "objects").unwrap();
                watermark = crate::session::vault_to_watermark(&stored);
            }
            last_row_id = page.records.last().map(|r| r.id.clone());
        }

        assert_eq!(
            paged_ids,
            vec![
                "n1_01".to_string(),
                "n1_02".to_string(),
                "n1_03".to_string(),
                "n1_04".to_string(),
                "n1_05".to_string(),
                "n1_06".to_string(),
                "n1_07".to_string(),
            ],
            "生产编码路径（hex 规范化节点）下等 ms 回退行组必须无缺漏、无重复、组内按 id 稳定升序"
        );
    }
}
