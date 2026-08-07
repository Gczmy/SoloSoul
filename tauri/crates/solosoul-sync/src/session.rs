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
use crate::types::{
    ApplyStats, AttachmentSyncStats, InboundSessionOutcome, NewPeerInfo, PeerCallback,
    SyncSessionResult,
};
use solosoul_vault::{PeerSyncState, VaultStore};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

const DELTA_PAGE_LIMIT: usize = 100;
/// 当前实现支持的同步协议版本。
/// 版本 1：初始协议（Delta 同步 + 附件交换）。
/// 版本 2：引入 account_id 双向校验 + 会话总超时 + 流式附件写入。
/// 旧版客户端（v2.6.1 及更早）不发送 protocol_version 字段，默认为 1。
const PROTOCOL_VERSION: u32 = 2;

/// 本机客户端类型，随 Hello/HelloAck 广播给对端（已知设备卡片展示用），
/// 同时用于 mDNS/NSD TXT 广播（对端「已发现设备」直接展示对应图标）。
/// 值域：macos / windows / linux / android / ios / unknown。
/// 编译期根据目标平台确定，桌面与移动共用同一 crate，无需外部注入。
pub fn local_client_type() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "android") {
        "android"
    } else if cfg!(target_os = "ios") {
        "ios"
    } else {
        "unknown"
    }
}
/// 允许的最低协议版本。低于此版本的 peer 将被拒绝，防止与不兼容的旧版客户端交互。
const MIN_PROTOCOL_VERSION: u32 = 1;
/// 同步会话总超时（5 分钟）。防止恶意 peer 通过每隔 29 秒发送一个字节
/// 来无限期保持连接（slowloris 攻击），占用 `spawn_blocking` 线程。
const SESSION_TIMEOUT: Duration = Duration::from_secs(300);

/// 检查会话是否已超过总超时。在每次 `recv_msg` 后调用。
fn check_session_deadline(start: Instant) -> Result<(), String> {
    if start.elapsed() > SESSION_TIMEOUT {
        Err(format!(
            "Sync session timed out after {}s",
            SESSION_TIMEOUT.as_secs()
        ))
    } else {
        Ok(())
    }
}

/// P103: 构造 pairing_pending 最小错误帧——只含对端 node_id，
/// 不含本地 account_id 与指纹（信任确认前不泄露敏感信息）。
/// 注意：这是 **B→A 线上错误帧** 的格式，必须保持旧版（无 sas）不变，
/// 否则旧版客户端解析 `parse_pairing_pending` 会失败（配对流程断裂）。
fn pairing_pending_message(node_id: &str) -> String {
    format!("__SYNC_ERR__:pairing_pending:{}", node_id)
}

/// 构造返回给 **本端前端** 的 pairing_pending 错误：在最小帧基础上附加
/// 本地派生的 6 位 SAS 验证码（`__SYNC_ERR__:pairing_pending:{pid}:{sas}`）。
/// A 侧前端据此在配对卡片两侧展示同一验证码供用户目视比对。
/// 仅用于 A 侧（发起方）→ 本地前端，不跨设备传输；线上帧仍用旧格式。
fn pairing_pending_frontend_message(node_id: &str, sas_code: &str) -> String {
    format!("__SYNC_ERR__:pairing_pending:{}:{}", node_id, sas_code)
}

/// P103: 从错误帧中解析 pairing_pending 携带的对端 node_id。
/// 非 pairing_pending 帧或空 id 返回 None。
/// 对 A 侧前端帧（`{node_id}:{sas}`）取第一个 `:` 之前的部分，
/// 避免 node_id 被 sas 污染（防御性——线上帧本不应携带 sas）。
fn parse_pairing_pending(message: &str) -> Option<&str> {
    message
        .strip_prefix("__SYNC_ERR__:pairing_pending:")
        .filter(|s| !s.is_empty())
        .map(|s| s.split(':').next().unwrap_or(s))
        .filter(|s| !s.is_empty())
}

/// 从 A 侧前端帧（`{node_id}:{sas}`）中解析 6 位 SAS 验证码。
/// 非前端帧（无 sas 部分）返回 None。
/// 仅测试使用：生产路径中 sas 由前端直接解析错误串（`{node_id}:{sas}`）。
#[cfg(test)]
fn parse_pairing_pending_sas(message: &str) -> Option<&str> {
    let rest = message.strip_prefix("__SYNC_ERR__:pairing_pending:")?;
    let sas = rest.split(':').nth(1)?;
    if sas.len() == 6 && sas.chars().all(|c| c.is_ascii_digit()) {
        Some(sas)
    } else {
        None
    }
}

/// 把同步会话/连接阶段返回的英文错误包装为 `__SYNC_ERR__:` 前缀，
/// 供前端 `resolveBackendErrorMessage` 翻译（P2：握手/连接错误 i18n）。
/// 已带 `__SYNC_ERR__:` 前缀的错误原样透传，避免二次包装。
/// 桌面（manager.rs）与移动（mobile.rs）共用。
pub(crate) fn wrap_session_error(err: String) -> String {
    if err.starts_with("__SYNC_ERR__:") {
        return err;
    }
    format!("__SYNC_ERR__:handshake_failed:{}", err)
}

/// 作为发起方与对端建立 Noise 会话并同步数据。
pub fn run_initiator_session(
    transport: &mut SyncTransport,
    node_id: &str,
    account_id: &str,
    keys: &NoiseKeys,
    vault: Arc<VaultStore>,
    peer_addr: String,
) -> Result<SyncSessionResult, String> {
    let session_start = Instant::now();
    let mut session = NoiseSession::handshake_initiator(transport, keys)?;
    // P001: Noise 握手完成后，对端静态公钥指纹已由握手密码学认证。
    // 后续所有身份比对一律以它为准，而非 Hello/HelloAck 中的自报值
    // （自报值可被攻击者伪造）。
    let remote_fingerprint = session
        .remote_fingerprint()
        .ok_or("Peer did not present a static public key during handshake")?;

    send_msg(
        &mut session,
        transport,
        &SyncMessage::Hello {
            node_id: node_id.to_string(),
            account_id: account_id.to_string(),
            public_key_fingerprint: keys.fingerprint(),
            protocol_version: PROTOCOL_VERSION,
            client_type: local_client_type().to_string(),
        },
    )?;

    let (peer_node_id, _trusted, _peer_client_type) = match recv_msg(&mut session, transport)? {
        SyncMessage::HelloAck {
            node_id: pid,
            account_id: peer_account_id,
            trusted: t,
            public_key_fingerprint,
            protocol_version: peer_version,
            client_type: peer_client_type,
        } => {
            check_session_deadline(session_start)?;
            // 校验响应方协议版本是否兼容。
            if peer_version < MIN_PROTOCOL_VERSION {
                send_msg(
                    &mut session,
                    transport,
                    &SyncMessage::Error {
                        message: format!(
                            "Unsupported protocol version {} (minimum required: {})",
                            peer_version, MIN_PROTOCOL_VERSION
                        ),
                    },
                )?;
                return Err(format!(
                    "Peer protocol version {} is below minimum supported {}",
                    peer_version, MIN_PROTOCOL_VERSION
                ));
            }
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
            // P001: 校验对端身份（自报指纹 == 握手派生指纹；已信任 peer 必须
            // 使用与配对时相同的静态公钥）。失败时回 Error 帧并中止。
            if let Err(e) =
                verify_peer_identity(&vault, &pid, &public_key_fingerprint, &remote_fingerprint)
            {
                send_msg(
                    &mut session,
                    transport,
                    &SyncMessage::Error { message: e.clone() },
                )?;
                return Err(e);
            }
            // P001: 落库以握手认证指纹为准（不再信任对端自报值）。
            record_peer(
                &vault,
                &pid,
                &peer_addr,
                &remote_fingerprint,
                &peer_client_type,
            )?;
            if !t {
                // 发起方侧：响应方尚未信任本设备。返回带 peer_node_id 的配对中错误码，
                // 前端据此进入「双侧确认配对」流程（不裸报英文，走 __SYNC_ERR__ 前缀 i18n）。
                // 附加本地派生的 SAS 验证码：前端配对卡片两侧展示同一 6 位数字。
                return Err(pairing_pending_frontend_message(&pid, &session.sas_code()));
            }
            (pid, t, peer_client_type)
        }
        SyncMessage::Error { message } => {
            // P103: 响应方未信任本设备时只回最小错误帧（pairing_pending + node_id）。
            // 发起方连接是用户显式发起（非入站洪水向量），仍以握手认证指纹落库对端，
            // 使前端配对对话框能展示对端信息，并保证 P001 指纹绑定。
            // 落库失败不阻断配对信号：记录在用户确认时由 trust_peer 重新创建，
            // 此处吞掉避免 DB 错误掩盖 __SYNC_ERR__:pairing_pending 让前端无法进入配对流程。
            if let Some(pid) = parse_pairing_pending(&message) {
                let _ = record_peer(&vault, pid, &peer_addr, &remote_fingerprint, "");
                // 附加本地派生的 SAS 验证码：前端配对卡片两侧展示同一 6 位数字。
                // B 侧旧版不发送 sas（线上帧格式未变），A 侧从本地会话派生即可。
                return Err(pairing_pending_frontend_message(pid, &session.sas_code()));
            }
            return Err(message);
        }
        _ => return Err("Unexpected message during handshake".to_string()),
    };

    // 先发送本地变更。
    // B：返回值为本次会话发往对端的记录条数（发起方侧不展示，忽略即可）。
    let _outbound_records = send_paginated_deltas(
        &mut session,
        transport,
        &vault,
        account_id,
        node_id,
        &peer_node_id,
        session_start,
    )?;

    // 接收对端变更并逐批应用。
    let mut data_stats = ApplyStats::default();
    loop {
        let msg = recv_msg(&mut session, transport)?;
        check_session_deadline(session_start)?;
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

    // 检查会话总超时（附件交换可能耗时较长）。
    check_session_deadline(session_start)?;

    Ok(SyncSessionResult {
        data: data_stats,
        attachments: attachment_stats,
    })
}

/// 作为响应方处理入站同步连接。
///
/// `peer_callback`：入站 Hello 来自**未信任** peer（无论是否已有记录）时触发，
/// 供 GUI 向所有页面广播配对请求（B 用户不在同步页也能收到配对确认对话框）。
/// 若仅对「新」peer 触发，已存在旧版遗留未信任记录的对端重连时响应方不弹框，
/// 发起方将永远等不到双向确认。
#[allow(clippy::too_many_arguments)]
pub fn handle_inbound(
    transport: &mut SyncTransport,
    node_id: &str,
    account_id: &str,
    keys: &NoiseKeys,
    vault: Arc<VaultStore>,
    peer_addr: String,
    peer_callback: Option<PeerCallback>,
) -> Result<InboundSessionOutcome, String> {
    let session_start = Instant::now();
    let mut session = NoiseSession::handshake_responder(transport, keys)?;
    // P001: Noise 握手完成后，对端静态公钥指纹已由握手密码学认证。
    // 后续所有身份比对一律以它为准（自报值可被攻击者伪造）。
    let remote_fingerprint = session
        .remote_fingerprint()
        .ok_or("Peer did not present a static public key during handshake")?;

    let (peer_node_id, peer_client_type) = match recv_msg(&mut session, transport)? {
        SyncMessage::Hello {
            node_id: pid,
            account_id: pacc,
            public_key_fingerprint,
            protocol_version: peer_version,
            client_type: peer_client_type,
        } => {
            check_session_deadline(session_start)?;
            // 校验发起方协议版本是否兼容。
            if peer_version < MIN_PROTOCOL_VERSION {
                send_msg(
                    &mut session,
                    transport,
                    &SyncMessage::Error {
                        message: format!(
                            "Unsupported protocol version {} (minimum required: {})",
                            peer_version, MIN_PROTOCOL_VERSION
                        ),
                    },
                )?;
                return Err(format!(
                    "Peer protocol version {} is below minimum supported {}",
                    peer_version, MIN_PROTOCOL_VERSION
                ));
            }
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
            // P001: 校验对端身份（自报指纹 == 握手派生指纹；已信任 peer 必须
            // 使用与配对时相同的静态公钥）。失败时回 Error 帧并中止。
            if let Err(e) =
                verify_peer_identity(&vault, &pid, &public_key_fingerprint, &remote_fingerprint)
            {
                send_msg(
                    &mut session,
                    transport,
                    &SyncMessage::Error { message: e.clone() },
                )?;
                return Err(e);
            }
            (pid, peer_client_type)
        }
        _ => return Err("Expected Hello".to_string()),
    };

    let trusted = vault
        .load_peer_state(&peer_node_id)?
        .map(|p| p.trusted)
        .unwrap_or(false);

    if !trusted {
        // P103: 未信任 peer——不落库、不回 HelloAck（不回 account_id 与指纹）。
        // 只要未信任（无论是否已存在记录——含旧版遗留的未信任记录/已撤销信任的记录），
        // 都触发配对请求回调（数据取自握手认证值，纯内存传递）。
        // 若仅对新 peer 触发，已存在未信任记录的对端重连时响应方不弹确认框，
        // 发起方将永远等不到双向确认（表现为「扫描方在等待、响应方毫无反应」）。
        // 重复事件由前端按 node_id 去重，用户确认/忽略后清除才允许重新弹出。
        if let Some(cb) = &peer_callback {
            let device_name = peer_display_name(&remote_fingerprint, &peer_addr);
            cb(NewPeerInfo {
                node_id: peer_node_id.clone(),
                fingerprint: remote_fingerprint.clone(),
                addr: peer_addr.clone(),
                device_name,
                client_type: peer_client_type.clone(),
                // 响应方从本地握手哈希派生 SAS 验证码，与发起方各自派生的值一致，
                // 供 B 侧配对卡片展示（两侧显示同一 6 位数字供目视比对）。
                sas_code: session.sas_code(),
            });
        }
        send_msg(
            &mut session,
            transport,
            &SyncMessage::Error {
                message: pairing_pending_message(node_id),
            },
        )?;
        return Err("Peer not trusted".to_string());
    }

    // 已信任：此刻才刷新 last_seen/指纹落库（信任已确认，落库安全），
    // 并回 HelloAck 进入同步。
    record_peer(
        &vault,
        &peer_node_id,
        &peer_addr,
        &remote_fingerprint,
        &peer_client_type,
    )?;
    send_msg(
        &mut session,
        transport,
        &SyncMessage::HelloAck {
            node_id: node_id.to_string(),
            account_id: account_id.to_string(),
            public_key_fingerprint: keys.fingerprint(),
            trusted,
            protocol_version: PROTOCOL_VERSION,
            client_type: local_client_type().to_string(),
        },
    )?;

    // 先接收对端变更。
    let mut apply_stats = ApplyStats::default();
    loop {
        let msg = recv_msg(&mut session, transport)?;
        check_session_deadline(session_start)?;
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
    // B：统计响应方发回给发起方的记录条数，随会话结果通知前端展示完整交换量
    // （响应方 toast 此前只含入站方向计数，发回方向缺失导致「检查 0 条」误导）。
    let outbound_records = send_paginated_deltas(
        &mut session,
        transport,
        &vault,
        account_id,
        node_id,
        &peer_node_id,
        session_start,
    )? as u64;

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

    Ok(InboundSessionOutcome {
        peer_node_id: peer_node_id.clone(),
        // B：本次响应方会话发回给发起方的记录条数（完整交换量的一侧）。
        outbound_records,
        result: SyncSessionResult {
            data: apply_stats,
            attachments: attachment_stats,
        },
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
    session_start: Instant,
) -> Result<usize, String> {
    // B：统计本次发送的记录条数（所有表/页的 records 总和），供响应方
    // 完成事件携带「发回对端条数」，使两侧展示完整交换量。
    let mut sent_records = 0usize;
    // N-1：回退行节点必须与 peer watermark 落库格式一致（hex 编码的 16 字节节点）。
    // 原始 node_id（随机 UUID）与 hex 字节串永不相等，会让存储层 keyset 等值组分支
    // 永远不触发——等 ms 回退行组在页边界处要么死循环、要么静默漏发。
    let local_hlc_node = hex::encode(Hlc::parse_node_id_bytes(node_id));
    for table in SYNC_TABLES {
        // N-1: keyset 页游标——推进到本页最后一条记录的 id。等值 HLC 组跨页时，
        // 水印虽推进到组最大值，游标仍保证组尾行被下一页继续投递（不重不漏）。
        // R-3: 游标不再仅存内存——从持久化水印旁恢复（会话中断后等值 HLC 组
        // 尾部跨会话续传，不再从 NULL 游标重查而跳过三元组 == 水印的组尾行）。
        let mut last_row_id: Option<String> =
            vault.get_peer_watermark_cursor(peer_node_id, table)?;
        loop {
            let watermark = vault_to_watermark(&vault.get_peer_watermark(peer_node_id, table)?);
            let page = generate_delta_paginated(
                vault,
                table,
                &watermark,
                account_id,
                &local_hlc_node,
                DELTA_PAGE_LIMIT,
                last_row_id.as_deref(),
            )?;
            if page.records.is_empty() && page.finished {
                // 无新记录：清空残留游标，保持严格 > 语义（防陈旧游标跳过未来同 ms 行）
                vault.update_peer_watermark_with_cursor(
                    peer_node_id,
                    table,
                    &watermark_to_vault(&watermark),
                    None,
                )?;
                break;
            }

            let finished = page.finished;
            let max_hlc = max_record_hlc(&page.records);
            if let Some(last) = page.records.last() {
                last_row_id = Some(last.id.clone());
            }
            sent_records += page.records.len();

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
            check_session_deadline(session_start)?;
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
                // R-3: 水印与页游标同刻落库（游标 = 本页最后一条 id；finished 时
                // 清空游标——表已同步完，下一会话从严格 > 开始）。
                let cursor = if finished {
                    None
                } else {
                    last_row_id.as_deref()
                };
                vault.update_peer_watermark_with_cursor(
                    peer_node_id,
                    table,
                    &watermark_to_vault(&hlc_to_sync_watermark(&max)),
                    cursor,
                )?;
            }

            if finished {
                break;
            }
        }
    }
    // §4.5.1 方案 C：四表全部同步完成后清理可安全删除的墓碑。此时所有 peer
    // 水位已推进到本次会话的最大 HLC——已收到对应删除的 peer 不再需要墓碑，
    // 清理立即可生效；未达水位的 peer（含离线）其墓碑被正确保留。
    // 清理失败不阻断同步会话（记录日志，下次会话重试）。
    if let Err(e) = vault.cleanup_expired_tombstones() {
        tracing::warn!(
            "[cleanup_expired_tombstones] failed after delta sync: {}",
            e
        );
    }
    send_msg(session, transport, &SyncMessage::Done)?;
    Ok(sent_records)
}

/// 派生对端显示名：fingerprint 非空 → SoloSoul-<fp 前 8 位>，否则回退到地址。
/// 与移动端 NSD 注册 / QR 卡片的设备名规则保持一致。
fn peer_display_name(fingerprint: &str, addr: &str) -> String {
    if fingerprint.is_empty() {
        addr.to_string()
    } else {
        format!("SoloSoul-{}", &fingerprint[..fingerprint.len().min(8)])
    }
}

/// P001: 校验对端身份（握手密码学认证后调用）。
///
/// ① 对端在 Hello/HelloAck 中**自报**的指纹必须与 Noise 握手派生的指纹一致——
///    自报值在加密通道内由对端书写、可被攻击者伪造，一律以握手认证值为准；
/// ② 已信任 peer 必须使用与配对时相同的静态公钥：其落库指纹必须与本次
///    握手指纹一致，防止 LAN 攻击者复用 node_id 冒充已信任节点拉取数据。
///
/// 历史记录无指纹（升级前配对）时不做比对，放行并由 record_peer 本次绑定。
/// 失败时返回错误（调用方回 Error 帧并中止），不落任何 peer 记录。
fn verify_peer_identity(
    vault: &VaultStore,
    peer_node_id: &str,
    reported_fingerprint: &str,
    handshake_fingerprint: &str,
) -> Result<(), String> {
    // ① 自报指纹必须与握手认证指纹一致。
    if reported_fingerprint != handshake_fingerprint {
        return Err(format!(
            "__SYNC_ERR__:handshake_failed:Peer fingerprint mismatch (reported {} != handshake {})",
            reported_fingerprint, handshake_fingerprint
        ));
    }
    // ② 已信任 peer 必须保持配对时的静态公钥不变。
    if let Some(peer) = vault.load_peer_state(peer_node_id)? {
        if peer.trusted {
            if let Some(stored) = &peer.public_key_fingerprint {
                if stored != handshake_fingerprint {
                    return Err(format!(
                        "__SYNC_ERR__:handshake_failed:Peer key changed since pairing (stored {} != handshake {}); re-pair required",
                        stored, handshake_fingerprint
                    ));
                }
            }
        }
    }
    Ok(())
}

/// 记录 peer。返回该 peer 是否为**新**记录（此前不存在）。
/// 新记录的设备名改为 SoloSoul-<fp 前 8 位>（老数据由前端 formatPeerName 派生兼容，无需迁移）。
fn record_peer(
    vault: &VaultStore,
    peer_node_id: &str,
    addr: &str,
    fingerprint: &str,
    client_type: &str,
) -> Result<bool, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let existing = vault.load_peer_state(peer_node_id)?;
    let is_new = existing.is_none();
    // 空串（配对中场景）存 None，避免 `Some("")` 污染已知设备客户端类型显示。
    let client_type_opt = if client_type.is_empty() {
        None
    } else {
        Some(client_type.to_string())
    };
    let mut peer = existing.unwrap_or_else(|| PeerSyncState {
        peer_node_id: peer_node_id.to_string(),
        peer_name: Some(peer_display_name(fingerprint, addr)),
        trusted: false,
        public_key_fingerprint: Some(fingerprint.to_string()),
        last_seen: Some(chrono::Utc::now().timestamp()),
        created_at: now.clone(),
        updated_at: now.clone(),
        client_type: client_type_opt.clone(),
        trusted_at: None,
        // P1#7/#8：成功握手/同步即证明对端 LAN 可达，把连接地址落库——
        // 即使 mDNS/NSD 发现链中断，known_peers 也能凭 fresh last_addr 显示在线。
        last_addr: Some(addr.to_string()),
    });
    // 已有记录不覆盖名字（可能被用户重命名），仅刷新指纹、客户端类型与最近在线时间。
    peer.public_key_fingerprint = Some(fingerprint.to_string());
    if let Some(ct) = client_type_opt {
        peer.client_type = Some(ct);
    }
    peer.last_seen = Some(chrono::Utc::now().timestamp());
    // 连接地址随每次成功会话刷新（对端可能换 IP/端口）。
    if !addr.is_empty() {
        peer.last_addr = Some(addr.to_string());
    }
    peer.updated_at = now;
    vault.save_peer_state(&peer)?;
    Ok(is_new)
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

pub(crate) fn vault_to_watermark(wm: &solosoul_vault::SyncWatermark) -> SyncWatermark {
    SyncWatermark {
        wall_time_ms: wm.wall_time_ms,
        counter: wm.counter,
        node_id: Hlc::parse_node_id_bytes(&wm.node_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_display_name_uses_fingerprint_prefix() {
        assert_eq!(
            peer_display_name("a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6", "192.168.0.1:42069"),
            "SoloSoul-a1b2c3d4"
        );
    }

    #[test]
    fn test_peer_display_name_short_fingerprint() {
        assert_eq!(peer_display_name("ab12", "10.0.0.2:42069"), "SoloSoul-ab12");
    }

    #[test]
    fn test_peer_display_name_empty_fingerprint_falls_back_to_addr() {
        assert_eq!(
            peer_display_name("", "192.168.0.1:42069"),
            "192.168.0.1:42069"
        );
    }

    /// P2：pairing_pending 错误走 `__SYNC_ERR__:` 前缀，不被 wrap_session_error 二次包装。
    #[test]
    fn test_wrap_session_error_passes_through_pairing_pending() {
        let err = "__SYNC_ERR__:pairing_pending:node-abc".to_string();
        assert_eq!(wrap_session_error(err.clone()), err);
    }

    // ---------------------------------------------------------------------
    // P103 防回归单测：最小错误帧构造/解析。
    // ---------------------------------------------------------------------

    /// pairing_pending 错误帧只含 node_id，不含 account_id 与指纹（信任确认前不泄露敏感信息）。
    #[test]
    fn test_pairing_pending_message_contains_only_node_id() {
        let msg = pairing_pending_message("node-b");
        assert_eq!(msg, "__SYNC_ERR__:pairing_pending:node-b");
        // 不应包含 account_id 或指纹片段
        assert!(!msg.contains("account"));
        assert!(!msg.contains("fingerprint"));
    }

    /// parse_pairing_pending 正确解析最小错误帧中的 node_id。
    /// 注意：线上帧格式永远是 `__SYNC_ERR__:pairing_pending:{node_id}`（无 sas），
    /// sas 只存在于 A 侧返回给本地前端的帧中，不会经线上传输。
    #[test]
    fn test_parse_pairing_pending_extracts_node_id() {
        assert_eq!(
            parse_pairing_pending("__SYNC_ERR__:pairing_pending:node-b"),
            Some("node-b")
        );
    }

    /// 线上帧解析器对携带 sas 的前端帧：node_id 是 sas 之前的部分。
    /// （防御性——线上帧不应带 sas，但若未来格式漂移仍能解析出 node_id。）
    #[test]
    fn test_parse_pairing_pending_handles_frontend_frame_with_sas() {
        assert_eq!(
            parse_pairing_pending("__SYNC_ERR__:pairing_pending:node-b:482913"),
            Some("node-b")
        );
    }

    /// 前端帧构造：最小帧 + 6 位 sas，node_id 与 sas 均可回读。
    #[test]
    fn test_pairing_pending_frontend_message_includes_sas() {
        let msg = pairing_pending_frontend_message("node-b", "482913");
        assert_eq!(msg, "__SYNC_ERR__:pairing_pending:node-b:482913");
        // 不包含 account_id 与指纹（信任确认前不泄露敏感信息）
        assert!(!msg.contains("account"));
        assert!(!msg.contains("fingerprint"));
        // node_id 可回读（用于前端定位 peer）
        assert_eq!(parse_pairing_pending(&msg), Some("node-b"));
        // sas 可回读
        assert_eq!(parse_pairing_pending_sas(&msg), Some("482913"));
    }

    /// 线上最小帧无 sas 部分，sas 解析器应返回 None（避免旧客户端帧被误判）。
    #[test]
    fn test_parse_pairing_pending_sas_rejects_online_frame() {
        assert_eq!(
            parse_pairing_pending_sas("__SYNC_ERR__:pairing_pending:node-b"),
            None
        );
        // 非 6 位 / 非纯数字的 sas 拒绝
        assert_eq!(
            parse_pairing_pending_sas("__SYNC_ERR__:pairing_pending:node-b:abc"),
            None
        );
        assert_eq!(
            parse_pairing_pending_sas("__SYNC_ERR__:pairing_pending:node-b:123"),
            None
        );
        // 非 pairing_pending 帧拒绝
        assert_eq!(
            parse_pairing_pending_sas("__SYNC_ERR__:handshake_failed:x"),
            None
        );
    }

    /// 非 pairing_pending 错误（如密钥不匹配）不解析出 node_id，避免误落库。
    #[test]
    fn test_parse_pairing_pending_rejects_other_errors() {
        assert_eq!(
            parse_pairing_pending("__SYNC_ERR__:handshake_failed:Peer fingerprint mismatch"),
            None
        );
        assert_eq!(parse_pairing_pending("Peer is not trusted"), None);
        assert_eq!(parse_pairing_pending("__SYNC_ERR__:pairing_pending:"), None);
    }

    // ---------------------------------------------------------------------
    // 双向配对回调防回归：未信任 peer（无论新记录还是已存在旧记录）都触发
    // peer_callback。回归场景：响应方已存在旧版遗留的未信任记录时，旧实现
    // 以 is_new=false 不触发回调 → 发起方永远等不到双向确认
    // （「扫描方在等待、响应方毫无反应」）。
    // ---------------------------------------------------------------------

    /// 运行一次「客户端连接未信任响应方」的完整握手，返回响应方 peer_callback 是否被触发。
    fn inbound_pairing_callback_fired(pre_seed_untrusted_peer: bool) -> bool {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let server_keys = NoiseKeys::generate();
        let client_keys = NoiseKeys::generate();

        let (server_vault, _server_dir) = test_vault();
        let (client_vault, _client_dir) = test_vault();
        if pre_seed_untrusted_peer {
            // 旧版遗留：响应方已存在未信任记录（P103 前落库）。
            save_peer(&server_vault, "node-client", false, Some("legacy-fp"));
        }
        let fired = Arc::new(std::sync::Mutex::new(false));
        let fired_cb = fired.clone();
        let server_vault_arc = server_vault.clone();

        let server_thread = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut transport = SyncTransport::from_stream(stream);
            let cb = fired_cb.clone();
            let result = handle_inbound(
                &mut transport,
                "node-server",
                "acct",
                &server_keys,
                server_vault_arc,
                "127.0.0.1:0".to_string(),
                Some(Arc::new(move |_info: NewPeerInfo| {
                    *cb.lock().unwrap() = true;
                })),
            );
            assert!(result.is_err(), "未信任 peer 应返回错误");
        });

        let client_thread = std::thread::spawn(move || {
            let stream = std::net::TcpStream::connect(&addr).unwrap();
            let mut transport = SyncTransport::from_stream(stream);
            let err = run_initiator_session(
                &mut transport,
                "node-client",
                "acct",
                &client_keys,
                client_vault,
                "127.0.0.1:1".to_string(),
            )
            .expect_err("未信任响应方应返回 pairing_pending");
            assert!(
                err.starts_with("__SYNC_ERR__:pairing_pending:"),
                "got: {err}"
            );
        });

        server_thread.join().unwrap();
        client_thread.join().unwrap();
        let fired_val = *fired.lock().unwrap();
        fired_val
    }

    #[test]
    fn test_inbound_callback_fires_for_new_untrusted_peer() {
        assert!(
            inbound_pairing_callback_fired(false),
            "新未信任 peer 也应触发配对回调"
        );
    }

    #[test]
    fn test_inbound_callback_fires_for_existing_untrusted_record() {
        assert!(
            inbound_pairing_callback_fired(true),
            "已存在旧版未信任记录的对端重连也应触发配对回调（回归）"
        );
    }

    #[test]
    fn test_wrap_session_error_wraps_plain_error() {
        let wrapped = wrap_session_error("Peer is not trusted".to_string());
        assert!(wrapped.starts_with("__SYNC_ERR__:handshake_failed:"));
        assert!(wrapped.ends_with("Peer is not trusted"));
    }

    // ---------------------------------------------------------------------
    // P001 防回归单测：verify_peer_identity 身份绑定。
    // ---------------------------------------------------------------------

    fn test_vault() -> (Arc<solosoul_vault::VaultStore>, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = Arc::new(
            solosoul_vault::VaultStore::open(solosoul_vault::VaultConfig {
                path: dir.path().to_path_buf(),
                account_id: "acct".to_string(),
                data_key: Some([0u8; 32]),
            })
            .expect("open vault"),
        );
        (vault, dir)
    }

    fn save_peer(
        vault: &solosoul_vault::VaultStore,
        node_id: &str,
        trusted: bool,
        fingerprint: Option<&str>,
    ) {
        let now = chrono::Utc::now().to_rfc3339();
        let peer = solosoul_vault::PeerSyncState {
            peer_node_id: node_id.to_string(),
            peer_name: Some(format!("SoloSoul-{}", &node_id[..node_id.len().min(8)])),
            trusted,
            public_key_fingerprint: fingerprint.map(|f| f.to_string()),
            last_seen: Some(chrono::Utc::now().timestamp()),
            created_at: now.clone(),
            updated_at: now,
            client_type: None,
            trusted_at: None,
            last_addr: None,
        };
        vault.save_peer_state(&peer).expect("save peer");
    }

    /// ① 自报指纹与握手派生指纹不一致必须拒绝（自报值可被伪造）。
    #[test]
    fn test_verify_peer_identity_rejects_reported_mismatch() {
        let (vault, _dir) = test_vault();
        let err = verify_peer_identity(&vault, "node-a", "reported-fake", "handshake-real")
            .expect_err("reported != handshake 应失败");
        assert!(err.contains("fingerprint mismatch"), "got: {}", err);
        assert!(
            err.starts_with("__SYNC_ERR__:handshake_failed:"),
            "got: {}",
            err
        );
    }

    /// ② 已信任 peer 的静态公钥变化必须拒绝（防冒充已信任节点）。
    #[test]
    fn test_verify_peer_identity_rejects_trusted_key_change() {
        let (vault, _dir) = test_vault();
        save_peer(&vault, "node-trusted", true, Some("stored-key-aaaa"));
        // 自报 == 握手（通过检查①），但落库指纹与握手指纹不同（触发检查②）。
        let err = verify_peer_identity(&vault, "node-trusted", "new-key-bbbb", "new-key-bbbb")
            .expect_err("已信任 peer 换钥应失败");
        assert!(err.contains("key changed since pairing"), "got: {}", err);
        assert!(
            err.starts_with("__SYNC_ERR__:handshake_failed:"),
            "got: {}",
            err
        );
    }

    /// ③ 已信任 peer 指纹一致（自报 == 握手 == 落库）必须通过。
    #[test]
    fn test_verify_peer_identity_accepts_trusted_matching_key() {
        let (vault, _dir) = test_vault();
        save_peer(&vault, "node-trusted", true, Some("stable-key-1111"));
        verify_peer_identity(&vault, "node-trusted", "stable-key-1111", "stable-key-1111")
            .expect("指纹一致应通过");
    }

    /// ④ 未信任/未记录的 peer：仅校验自报 == 握手，不校验落库。
    #[test]
    fn test_verify_peer_identity_accepts_new_peer() {
        let (vault, _dir) = test_vault();
        // 无任何记录的新 peer
        verify_peer_identity(&vault, "node-new", "fp-ok", "fp-ok").expect("新 peer 应通过");
        // 已记录但未信任（配对中）的 peer
        save_peer(&vault, "node-pending", false, Some("fp-old"));
        verify_peer_identity(&vault, "node-pending", "fp-ok", "fp-ok").expect("配对中 peer 应通过");
    }

    /// ⑤ 历史记录无指纹（升级前配对）放行，由 record_peer 本次绑定。
    #[test]
    fn test_verify_peer_identity_accepts_trusted_without_stored_fingerprint() {
        let (vault, _dir) = test_vault();
        save_peer(&vault, "node-legacy", true, None);
        verify_peer_identity(&vault, "node-legacy", "fp-first", "fp-first")
            .expect("无落库指纹的历史 peer 应通过");
    }

    /// record_peer 必须落库握手认证指纹（而非对端自报值）。
    #[test]
    fn test_record_peer_stores_handshake_fingerprint() {
        let (vault, _dir) = test_vault();
        let is_new = record_peer(&vault, "node-a", "10.0.0.1:42069", "handshake-fp", "macos")
            .expect("record");
        assert!(is_new, "首次记录应为新 peer");
        let peer = vault
            .load_peer_state("node-a")
            .expect("load")
            .expect("peer 应存在");
        assert_eq!(peer.public_key_fingerprint.as_deref(), Some("handshake-fp"));
        assert!(!peer.trusted);
        assert_eq!(peer.client_type.as_deref(), Some("macos"));
        assert!(peer.trusted_at.is_none());
    }
}
