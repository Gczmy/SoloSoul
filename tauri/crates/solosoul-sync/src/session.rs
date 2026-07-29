//! Shared sync session logic for both desktop and mobile platforms.
//!
//! 该模块包含 Noise 握手、分页 Delta 同步、附件交换等与会话相关的逻辑，
//! 不依赖任何平台特定的发现机制（mDNS/NSD/Bonjour），供 manager.rs 与 mobile.rs 复用。

use crate::attachments::{
    collect_attachment_manifests, compute_needed_attachments, receive_attachment_manifests,
    receive_attachment_requests, receive_attachments, send_attachment_manifests,
    send_attachment_requests, send_requested_attachments,
};
use crate::delta::{
    generate_delta_paginated, hlc_to_sync_watermark, max_record_hlc, watermark_to_vault,
    SYNC_TABLES,
};
use crate::hlc::{Hlc, SyncWatermark};
use crate::noise::{NoiseKeys, NoiseSession};
use crate::protocol::SyncMessage;
use crate::transport::SyncTransport;
use crate::types::{ApplyStats, AttachmentSyncStats, SyncSessionResult};
use solosoul_vault::{PeerSyncState, VaultStore};
use std::path::Path;
use std::sync::Arc;

const DELTA_PAGE_LIMIT: usize = 100;

/// 作为发起方与对端建立 Noise 会话并同步数据。
pub fn run_initiator_session(
    transport: &mut SyncTransport,
    node_id: &str,
    account_id: &str,
    keys: &NoiseKeys,
    vault: Arc<VaultStore>,
    peer_addr: String,
) -> Result<SyncSessionResult, String> {
    let mut session = NoiseSession::handshake_initiator(transport, keys)?;

    send_msg(
        &mut session,
        transport,
        &SyncMessage::Hello {
            node_id: node_id.to_string(),
            account_id: account_id.to_string(),
            public_key_fingerprint: keys.fingerprint(),
        },
    )?;

    let (peer_node_id, _trusted) = match recv_msg(&mut session, transport)? {
        SyncMessage::HelloAck {
            node_id: pid,
            account_id: peer_account_id,
            trusted: t,
            public_key_fingerprint,
        } => {
            // 校验响应方 account_id 与本地一致，防止已信任的 peer 被重新配置为
            // 不同账户后，发起方仍向其同步数据（违反账户隔离原则）。
            // 与 handle_inbound 中响应方校验发起方 account_id 的逻辑对称。
            if peer_account_id != account_id {
                send_msg(
                    &mut session,
                    transport,
                    &SyncMessage::Error {
                        message: "Account mismatch".to_string(),
                    },
                )?;
                return Err("Account mismatch".to_string());
            }
            record_peer(&vault, &pid, &peer_addr, &public_key_fingerprint)?;
            if !t {
                return Err("Peer is not trusted".to_string());
            }
            (pid, t)
        }
        SyncMessage::Error { message } => return Err(message),
        _ => return Err("Unexpected message during handshake".to_string()),
    };

    // 先发送本地变更。
    send_paginated_deltas(
        &mut session,
        transport,
        &vault,
        account_id,
        node_id,
        &peer_node_id,
    )?;

    // 接收对端变更并逐批应用。
    let mut data_stats = ApplyStats::default();
    loop {
        let msg = recv_msg(&mut session, transport)?;
        match msg {
            SyncMessage::Batch { table, records, .. } => {
                let stats = crate::delta::apply_sync_records(&vault, &table, &records, node_id)?;
                data_stats.examined += stats.examined;
                data_stats.applied += stats.applied;
                data_stats.skipped += stats.skipped;
                data_stats.errors.extend(stats.errors);
                for (t, s) in stats.per_table {
                    let entry = data_stats.per_table.entry(t).or_default();
                    entry.examined += s.examined;
                    entry.applied += s.applied;
                    entry.skipped += s.skipped;
                }
                data_stats.conflicts.extend(stats.conflicts);
                send_msg(
                    &mut session,
                    transport,
                    &SyncMessage::Ack {
                        table,
                        count: data_stats.examined,
                    },
                )?;
            }
            SyncMessage::Done => break,
            SyncMessage::Error { message } => return Err(message),
            _ => return Err("Unexpected message while receiving batches".to_string()),
        }
    }

    let base = vault.base_path();
    let attachment_stats =
        exchange_attachments(&mut session, transport, &vault, account_id, base, false)
            .unwrap_or_else(|e| {
                tracing::warn!("Attachment exchange failed: {}", e);
                AttachmentSyncStats {
                    errors: vec![e],
                    ..Default::default()
                }
            });

    Ok(SyncSessionResult {
        data: data_stats,
        attachments: attachment_stats,
    })
}

/// 作为响应方处理入站同步连接。
#[allow(clippy::too_many_arguments)]
pub fn handle_inbound(
    transport: &mut SyncTransport,
    node_id: &str,
    account_id: &str,
    keys: &NoiseKeys,
    vault: Arc<VaultStore>,
    peer_addr: String,
) -> Result<SyncSessionResult, String> {
    let mut session = NoiseSession::handshake_responder(transport, keys)?;

    let (peer_node_id, _peer_account, _fingerprint) = match recv_msg(&mut session, transport)? {
        SyncMessage::Hello {
            node_id: pid,
            account_id: pacc,
            public_key_fingerprint,
        } => {
            if pacc != account_id {
                send_msg(
                    &mut session,
                    transport,
                    &SyncMessage::Error {
                        message: "Account mismatch".to_string(),
                    },
                )?;
                return Err("Account mismatch".to_string());
            }
            record_peer(&vault, &pid, &peer_addr, &public_key_fingerprint)?;
            (pid, pacc, public_key_fingerprint)
        }
        _ => return Err("Expected Hello".to_string()),
    };

    let trusted = vault
        .load_peer_state(&peer_node_id)?
        .map(|p| p.trusted)
        .unwrap_or(false);

    send_msg(
        &mut session,
        transport,
        &SyncMessage::HelloAck {
            node_id: node_id.to_string(),
            account_id: account_id.to_string(),
            public_key_fingerprint: keys.fingerprint(),
            trusted,
        },
    )?;

    if !trusted {
        send_msg(
            &mut session,
            transport,
            &SyncMessage::Error {
                message: "Peer is not trusted".to_string(),
            },
        )?;
        return Err("Peer not trusted".to_string());
    }

    // 先接收对端变更。
    let mut apply_stats = ApplyStats::default();
    loop {
        let msg = recv_msg(&mut session, transport)?;
        match msg {
            SyncMessage::Batch { table, records, .. } => {
                let stats = crate::delta::apply_sync_records(&vault, &table, &records, node_id)?;
                apply_stats.examined += stats.examined;
                apply_stats.applied += stats.applied;
                apply_stats.skipped += stats.skipped;
                apply_stats.errors.extend(stats.errors);
                for (t, s) in stats.per_table {
                    let entry = apply_stats.per_table.entry(t).or_default();
                    entry.examined += s.examined;
                    entry.applied += s.applied;
                    entry.skipped += s.skipped;
                }
                apply_stats.conflicts.extend(stats.conflicts);
                send_msg(
                    &mut session,
                    transport,
                    &SyncMessage::Ack {
                        table,
                        count: apply_stats.examined,
                    },
                )?;
            }
            SyncMessage::Done => break,
            SyncMessage::Error { message } => return Err(message),
            _ => return Err("Unexpected message while receiving batches".to_string()),
        }
    }

    // 再发送本地变更。
    send_paginated_deltas(
        &mut session,
        transport,
        &vault,
        account_id,
        node_id,
        &peer_node_id,
    )?;

    let base = vault.base_path();
    let attachment_stats =
        exchange_attachments(&mut session, transport, &vault, account_id, base, true)
            .unwrap_or_else(|e| {
                tracing::warn!("Inbound attachment exchange failed: {}", e);
                AttachmentSyncStats {
                    errors: vec![e],
                    ..Default::default()
                }
            });

    tracing::info!(
        "Inbound sync from {} applied {} records, received {} attachments",
        peer_node_id,
        apply_stats.applied,
        attachment_stats.received
    );

    Ok(SyncSessionResult {
        data: apply_stats,
        attachments: attachment_stats,
    })
}

/// 交换附件文件。
pub fn exchange_attachments(
    session: &mut NoiseSession,
    transport: &mut SyncTransport,
    vault: &VaultStore,
    account_id: &str,
    base: &Path,
    is_responder: bool,
) -> Result<AttachmentSyncStats, String> {
    let mut stats = AttachmentSyncStats::default();
    let local_manifests = collect_attachment_manifests(vault, account_id)?;
    let local_map: std::collections::HashMap<String, Vec<crate::protocol::AttachmentInfo>> =
        local_manifests.iter().cloned().collect();

    let remote_manifests = if is_responder {
        let remote = receive_attachment_manifests(session, transport)?;
        send_attachment_manifests(session, transport, &local_manifests)?;
        remote
    } else {
        send_attachment_manifests(session, transport, &local_manifests)?;
        receive_attachment_manifests(session, transport)?
    };

    let local_needed = compute_needed_attachments(base, &remote_manifests);
    send_attachment_requests(session, transport, &local_needed)?;
    let requested = receive_attachment_requests(session, transport)?;

    if is_responder {
        receive_attachments(session, transport, base, &remote_manifests, &mut stats)?;
        send_requested_attachments(session, transport, base, &requested, &local_map, &mut stats)?;
    } else {
        send_requested_attachments(session, transport, base, &requested, &local_map, &mut stats)?;
        receive_attachments(session, transport, base, &remote_manifests, &mut stats)?;
    }

    Ok(stats)
}

fn send_paginated_deltas(
    session: &mut NoiseSession,
    transport: &mut SyncTransport,
    vault: &VaultStore,
    account_id: &str,
    node_id: &str,
    peer_node_id: &str,
) -> Result<(), String> {
    for table in SYNC_TABLES {
        loop {
            let watermark = vault_to_watermark(&vault.get_peer_watermark(peer_node_id, table)?);
            let page = generate_delta_paginated(
                vault,
                table,
                &watermark,
                account_id,
                node_id,
                DELTA_PAGE_LIMIT,
                0,
            )?;
            if page.records.is_empty() && page.finished {
                break;
            }

            let finished = page.finished;
            let max_hlc = max_record_hlc(&page.records);

            send_msg(
                session,
                transport,
                &SyncMessage::Batch {
                    table: table.to_string(),
                    records: page.records,
                    finished,
                },
            )?;

            let ack = recv_msg(session, transport)?;
            if let SyncMessage::Ack {
                table: ack_table, ..
            } = ack
            {
                if ack_table != *table {
                    return Err(format!("Ack for wrong table: {}", ack_table));
                }
            } else {
                return Err("Expected Ack after Batch".to_string());
            }

            if let Some(max) = max_hlc {
                vault.update_peer_watermark(
                    peer_node_id,
                    table,
                    &watermark_to_vault(&hlc_to_sync_watermark(&max)),
                )?;
            }

            if finished {
                break;
            }
        }
    }
    send_msg(session, transport, &SyncMessage::Done)
}

fn record_peer(
    vault: &VaultStore,
    peer_node_id: &str,
    addr: &str,
    fingerprint: &str,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    let existing = vault.load_peer_state(peer_node_id)?;
    let mut peer = existing.unwrap_or_else(|| PeerSyncState {
        peer_node_id: peer_node_id.to_string(),
        peer_name: Some(addr.to_string()),
        trusted: false,
        public_key_fingerprint: Some(fingerprint.to_string()),
        last_seen: Some(chrono::Utc::now().timestamp()),
        created_at: now.clone(),
        updated_at: now.clone(),
    });
    peer.peer_name = Some(addr.to_string());
    peer.public_key_fingerprint = Some(fingerprint.to_string());
    peer.last_seen = Some(chrono::Utc::now().timestamp());
    peer.updated_at = now;
    vault.save_peer_state(&peer)
}

fn send_msg(
    session: &mut NoiseSession,
    transport: &mut SyncTransport,
    msg: &SyncMessage,
) -> Result<(), String> {
    let bytes = msg.encode()?;
    session.send(transport, &bytes)
}

fn recv_msg(
    session: &mut NoiseSession,
    transport: &mut SyncTransport,
) -> Result<SyncMessage, String> {
    let bytes = session.receive(transport)?;
    SyncMessage::decode(&bytes)
}

fn vault_to_watermark(wm: &solosoul_vault::SyncWatermark) -> SyncWatermark {
    SyncWatermark {
        wall_time_ms: wm.wall_time_ms,
        counter: wm.counter,
        node_id: Hlc::parse_node_id_bytes(&wm.node_id),
    }
}
