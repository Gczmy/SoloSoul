//! SyncManager: mDNS discovery, TCP listener, and encrypted sync sessions.

use crate::attachments::{
    collect_attachment_manifests, compute_needed_attachments, receive_attachment_manifests,
    receive_attachment_requests, receive_attachments, send_attachment_manifests,
    send_attachment_requests, send_requested_attachments, AttachmentSyncStats,
};
use crate::delta::{
    apply_sync_records, generate_delta_paginated, hlc_to_sync_watermark, max_record_hlc,
    watermark_to_vault, ApplyStats, SYNC_TABLES,
};

const DELTA_PAGE_LIMIT: usize = 100;
use crate::hlc::{Hlc, SyncWatermark};
use crate::noise::{NoiseKeys, NoiseSession};
use crate::protocol::SyncMessage;
use crate::transport::SyncTransport;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use solosoul_vault::{PeerSyncState, VaultStore};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, UdpSocket};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::task::{spawn, spawn_blocking, JoinHandle};

const SERVICE_TYPE: &str = "_solosoul._tcp.local.";
const MDNS_TIMEOUT_MS: u64 = 200;
const PEER_MAX_AGE_SECS: u64 = 300;

/// Result of a full sync session, including database deltas and attachment transfers.
#[derive(Debug, Clone, Default)]
pub struct SyncSessionResult {
    pub data: ApplyStats,
    pub attachments: AttachmentSyncStats,
}

/// Information about a discovered or known peer.
#[derive(Debug, Clone)]
pub struct SyncPeerInfo {
    pub node_id: String,
    pub account_id: String,
    pub name: String,
    pub addr: String,
    pub fingerprint: String,
    pub trusted: bool,
    pub last_seen: String,
}

#[derive(Debug, Clone)]
struct DiscoveredPeer {
    node_id: String,
    account_id: String,
    name: String,
    addr: SocketAddr,
    fingerprint: String,
    last_seen: Instant,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct PeerSession {
    node_id: String,
    started_at: Instant,
}

/// Central manager for peer-to-peer synchronization.
pub struct SyncManager {
    node_id: String,
    account_id: String,
    keys: NoiseKeys,
    vault: Arc<VaultStore>,
    listen_addr: String,
    listen_port: AtomicU16,
    running: Arc<AtomicBool>,
    discovered: Arc<Mutex<HashMap<String, DiscoveredPeer>>>,
    sessions: Arc<Mutex<HashMap<String, PeerSession>>>,
    mdns_daemon: Mutex<Option<ServiceDaemon>>,
    worker_handles: Mutex<Vec<JoinHandle<()>>>,
}

impl SyncManager {
    pub fn new(
        node_id: String,
        account_id: String,
        keys: NoiseKeys,
        vault: Arc<VaultStore>,
        listen_addr: &str,
    ) -> Self {
        Self {
            node_id,
            account_id,
            keys,
            vault,
            listen_addr: listen_addr.to_string(),
            listen_port: AtomicU16::new(0),
            running: Arc::new(AtomicBool::new(false)),
            discovered: Arc::new(Mutex::new(HashMap::new())),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            mdns_daemon: Mutex::new(None),
            worker_handles: Mutex::new(Vec::new()),
        }
    }

    /// Returns the short public-key fingerprint for manual verification.
    pub fn fingerprint(&self) -> String {
        self.keys.fingerprint()
    }

    pub fn listen_port(&self) -> u16 {
        self.listen_port.load(Ordering::SeqCst)
    }

    /// Start the TCP listener and mDNS discovery/advertisement.
    pub async fn start(&self) -> Result<u16, String> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(self.listen_port.load(Ordering::SeqCst));
        }
        self.running.store(true, Ordering::SeqCst);

        let listener =
            TcpListener::bind(&self.listen_addr).map_err(|e| format!("bind failed: {}", e))?;
        listener
            .set_nonblocking(false)
            .map_err(|e| format!("set blocking: {}", e))?;
        let port = listener
            .local_addr()
            .map_err(|e| format!("local_addr: {}", e))?
            .port();
        self.listen_port.store(port, Ordering::SeqCst);

        let running = self.running.clone();
        let node_id = self.node_id.clone();
        let account_id = self.account_id.clone();
        let keys = self.keys.clone();
        let vault = self.vault.clone();
        let discovered = self.discovered.clone();
        let sessions = self.sessions.clone();

        // TCP accept loop (blocking std listener)
        let accept_handle = spawn_blocking(move || loop {
            if !running.load(Ordering::SeqCst) {
                break;
            }
            match listener.accept() {
                Ok((stream, addr)) => {
                    if !running.load(Ordering::SeqCst) {
                        break;
                    }
                    let node_id = node_id.clone();
                    let account_id = account_id.clone();
                    let keys = keys.clone();
                    let vault = vault.clone();
                    let discovered = discovered.clone();
                    let sessions = sessions.clone();
                    spawn_blocking(move || {
                        let mut transport = SyncTransport::from_stream(stream);
                        let _ = handle_inbound(
                            &mut transport,
                            &node_id,
                            &account_id,
                            &keys,
                            vault,
                            discovered,
                            sessions,
                            addr.to_string(),
                        );
                    });
                }
                Err(e) => {
                    tracing::warn!("accept error: {}", e);
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        });

        // mDNS
        let mdns_daemon = ServiceDaemon::new().map_err(|e| format!("mdns daemon: {}", e))?;
        self.register_mdns(&mdns_daemon, port)?;
        *self.mdns_daemon.lock().unwrap() = Some(mdns_daemon.clone());

        let mdns_handle = self.spawn_mdns_discovery(mdns_daemon);

        if let Ok(mut handles) = self.worker_handles.lock() {
            handles.push(accept_handle);
            handles.push(mdns_handle);
        }

        Ok(port)
    }

    /// Stop all background workers and mDNS.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        // Unblock the blocking accept loop by connecting to ourselves.
        if let Ok(mut handles) = self.worker_handles.lock() {
            let port = self.listen_port.load(Ordering::SeqCst);
            if port != 0 {
                let _ = std::net::TcpStream::connect(format!("127.0.0.1:{}", port));
            }
            for h in handles.drain(..) {
                h.abort();
            }
        }
        if let Ok(mut daemon) = self.mdns_daemon.lock() {
            if let Some(d) = daemon.take() {
                let _ = d.shutdown();
            }
        }
    }

    fn register_mdns(&self, daemon: &ServiceDaemon, port: u16) -> Result<(), String> {
        let ips: Vec<IpAddr> = Self::local_ips().into_iter().map(IpAddr::V4).collect();
        if ips.is_empty() {
            return Err("No local IP address available for mDNS".to_string());
        }
        let mut txt = HashMap::<String, String>::new();
        txt.insert("node_id".to_string(), self.node_id.clone());
        txt.insert("account_id".to_string(), self.account_id.clone());
        txt.insert("fingerprint".to_string(), self.keys.fingerprint());

        let hostname = format!("{}.local.", self.node_id);
        let service = ServiceInfo::new(SERVICE_TYPE, &self.node_id, &hostname, &ips[..], port, txt)
            .map_err(|e| format!("ServiceInfo: {}", e))?;
        daemon
            .register(service)
            .map_err(|e| format!("mDNS register: {}", e))?;
        Ok(())
    }

    fn spawn_mdns_discovery(&self, daemon: ServiceDaemon) -> JoinHandle<()> {
        let running = self.running.clone();
        let discovered = self.discovered.clone();
        let account_id = self.account_id.clone();

        spawn(async move {
            let receiver = match daemon.browse(SERVICE_TYPE) {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("mDNS browse failed: {}", e);
                    return;
                }
            };
            while running.load(Ordering::SeqCst) {
                match receiver.recv_timeout(Duration::from_millis(MDNS_TIMEOUT_MS)) {
                    Ok(ServiceEvent::ServiceResolved(info)) => {
                        let props = info.get_properties();
                        let peer_account = props
                            .get("account_id")
                            .map(|v| v.to_string())
                            .unwrap_or_default();
                        if peer_account != account_id {
                            continue;
                        }
                        let node_id = props
                            .get("node_id")
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| info.get_fullname().to_string());
                        let fingerprint = props
                            .get("fingerprint")
                            .map(|v| v.to_string())
                            .unwrap_or_default();
                        let addrs: Vec<IpAddr> = info.get_addresses().iter().cloned().collect();
                        if let Some(addr) = addrs.first() {
                            let socket = SocketAddr::new(*addr, info.get_port());
                            let peer = DiscoveredPeer {
                                node_id: node_id.clone(),
                                account_id: peer_account,
                                name: info.get_fullname().to_string(),
                                addr: socket,
                                fingerprint,
                                last_seen: Instant::now(),
                            };
                            if let Ok(mut map) = discovered.lock() {
                                map.insert(node_id, peer);
                            }
                        }
                    }
                    Ok(ServiceEvent::ServiceRemoved(_, fullname)) => {
                        if let Ok(mut map) = discovered.lock() {
                            map.retain(|_, p| p.name != fullname);
                        }
                    }
                    _ => {}
                }
            }
        })
    }

    /// Connect to a discovered peer or a direct `host:port` address and sync.
    pub async fn sync_with_peer(
        &self,
        device_id_or_addr: &str,
    ) -> Result<SyncSessionResult, String> {
        if !self.running.load(Ordering::SeqCst) {
            return Err("Sync manager is not running".to_string());
        }
        let addr = if let Ok(socket) = device_id_or_addr.parse::<SocketAddr>() {
            socket
        } else {
            let map = self.discovered.lock().map_err(|e| e.to_string())?;
            let peer = map
                .get(device_id_or_addr)
                .ok_or_else(|| format!("Peer not discovered: {}", device_id_or_addr))?;
            peer.addr
        };

        let node_id = self.node_id.clone();
        let account_id = self.account_id.clone();
        let keys = self.keys.clone();
        let vault = self.vault.clone();

        let result = spawn_blocking(move || {
            let stream = std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(10))
                .map_err(|e| format!("connect: {}", e))?;
            let mut transport = SyncTransport::from_stream(stream);
            run_initiator_session(
                &mut transport,
                &node_id,
                &account_id,
                &keys,
                vault,
                addr.to_string(),
            )
        })
        .await
        .map_err(|e| format!("spawn blocking: {}", e))?;

        result
    }

    /// Return discovered and persisted peers visible right now.
    pub fn known_peers(&self) -> Result<Vec<SyncPeerInfo>, String> {
        let mut out = Vec::new();
        let now = Instant::now();
        let map = self.discovered.lock().map_err(|e| e.to_string())?;
        for (_, p) in map.iter() {
            if now.duration_since(p.last_seen).as_secs() > PEER_MAX_AGE_SECS {
                continue;
            }
            out.push(SyncPeerInfo {
                node_id: p.node_id.clone(),
                account_id: p.account_id.clone(),
                name: p.name.clone(),
                addr: p.addr.to_string(),
                fingerprint: p.fingerprint.clone(),
                trusted: self.is_peer_trusted(&p.node_id)?,
                last_seen: format_duration_since(p.last_seen),
            });
        }
        // Include persisted peers even if not currently discovered.
        let persisted = self.vault.list_peers()?;
        for p in persisted {
            if out.iter().any(|i| i.node_id == p.peer_node_id) {
                continue;
            }
            out.push(SyncPeerInfo {
                node_id: p.peer_node_id.clone(),
                account_id: self.account_id.clone(),
                name: p
                    .peer_name
                    .clone()
                    .unwrap_or_else(|| p.peer_node_id.clone()),
                addr: String::new(),
                fingerprint: p.public_key_fingerprint.clone().unwrap_or_default(),
                trusted: p.trusted,
                last_seen: String::new(),
            });
        }
        Ok(out)
    }

    fn is_peer_trusted(&self, peer_node_id: &str) -> Result<bool, String> {
        match self.vault.load_peer_state(peer_node_id)? {
            Some(p) => Ok(p.trusted),
            None => Ok(false),
        }
    }

    /// Mark a peer as trusted or untrusted.
    pub fn trust_peer(&self, peer_node_id: &str, trusted: bool) -> Result<(), String> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut peer = self
            .vault
            .load_peer_state(peer_node_id)?
            .unwrap_or_else(|| PeerSyncState {
                peer_node_id: peer_node_id.to_string(),
                peer_name: None,
                trusted: false,
                public_key_fingerprint: None,
                last_seen: None,
                created_at: now.clone(),
                updated_at: now.clone(),
            });
        peer.trusted = trusted;
        peer.updated_at = now;
        self.vault.save_peer_state(&peer)
    }

    /// Remove a peer from persisted state.
    pub fn forget_peer(&self, peer_node_id: &str) -> Result<(), String> {
        self.vault.delete_peer(peer_node_id)
    }

    fn local_ips() -> Vec<Ipv4Addr> {
        let mut ips = Vec::new();
        // Best-effort primary local IP via UDP.
        if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
            if socket.connect("8.8.8.8:80").is_ok() {
                if let Ok(local) = socket.local_addr() {
                    if let IpAddr::V4(v4) = local.ip() {
                        if !v4.is_loopback() {
                            ips.push(v4);
                        }
                    }
                }
            }
        }
        // Always include loopback for local testing.
        ips.push(Ipv4Addr::new(127, 0, 0, 1));
        ips
    }
}

fn format_duration_since(instant: Instant) -> String {
    let secs = instant.elapsed().as_secs();
    if secs < 60 {
        format!("{}s ago", secs)
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else {
        format!("{}h ago", secs / 3600)
    }
}

fn run_initiator_session(
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

    let (peer_node_id, trusted) = match recv_msg(&mut session, transport)? {
        SyncMessage::HelloAck {
            node_id: pid,
            trusted: t,
            public_key_fingerprint,
            ..
        } => {
            record_peer(&vault, &pid, &peer_addr, &public_key_fingerprint)?;
            if !t {
                return Err("Peer is not trusted".to_string());
            }
            (pid, t)
        }
        SyncMessage::Error { message } => return Err(message),
        _ => return Err("Unexpected message during handshake".to_string()),
    };
    let _ = trusted;

    // Send our changes first, paginated by table.
    for table in SYNC_TABLES {
        loop {
            let watermark = vault_to_watermark(&vault.get_peer_watermark(&peer_node_id, table)?);
            let page = generate_delta_paginated(
                &vault,
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
            let record_count = page.records.len();
            send_msg(
                &mut session,
                transport,
                &SyncMessage::Batch {
                    table: table.to_string(),
                    records: page.records,
                    finished,
                },
            )?;

            let ack = recv_msg(&mut session, transport)?;
            if let SyncMessage::Ack {
                table: ack_table,
                count,
            } = ack
            {
                if ack_table != *table {
                    return Err(format!("Ack for wrong table: {}", ack_table));
                }
                tracing::info!("Sent {} {} records to peer", count, table);
            } else {
                return Err("Expected Ack after Batch".to_string());
            }

            // Advance peer watermark so the next page resumes after what we just sent.
            if let Some(max) = max_hlc {
                vault.update_peer_watermark(
                    &peer_node_id,
                    table,
                    &watermark_to_vault(&hlc_to_sync_watermark(&max)),
                )?;
            }

            if finished {
                break;
            }
            // If the peer is still behind, another page will be fetched starting from
            // the updated watermark. The loop count is bounded by table size / DELTA_PAGE_LIMIT.
            let _ = record_count;
        }
    }
    send_msg(&mut session, transport, &SyncMessage::Done)?;

    // Receive peer changes and apply each batch incrementally to bound memory.
    let mut data_stats = ApplyStats::default();
    loop {
        let msg = recv_msg(&mut session, transport)?;
        match msg {
            SyncMessage::Batch { table, records, .. } => {
                let stats = apply_sync_records(&vault, &table, &records, node_id)?;
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

/// Exchange attachment files after the main record sync.
/// `is_responder` controls who sends manifests first:
/// - initiator sends manifests first, then receives responder's manifests.
pub(crate) fn exchange_attachments(
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
        // Responder: receive remote manifests first, then send ours.
        let remote = receive_attachment_manifests(session, transport)?;
        send_attachment_manifests(session, transport, &local_manifests)?;
        remote
    } else {
        // Initiator: send our manifests first, then receive remote manifests.
        send_attachment_manifests(session, transport, &local_manifests)?;
        receive_attachment_manifests(session, transport)?
    };

    let local_needed = compute_needed_attachments(base, &remote_manifests);
    send_attachment_requests(session, transport, &local_needed)?;
    let requested = receive_attachment_requests(session, transport)?;

    // Transfer attachment chunks sequentially to avoid interleaving Ack/Done messages.
    if is_responder {
        receive_attachments(session, transport, base, &remote_manifests, &mut stats)?;
        send_requested_attachments(session, transport, base, &requested, &local_map, &mut stats)?;
    } else {
        send_requested_attachments(session, transport, base, &requested, &local_map, &mut stats)?;
        receive_attachments(session, transport, base, &remote_manifests, &mut stats)?;
    }

    Ok(stats)
}

#[allow(clippy::too_many_arguments)]
fn handle_inbound(
    transport: &mut SyncTransport,
    node_id: &str,
    account_id: &str,
    keys: &NoiseKeys,
    vault: Arc<VaultStore>,
    discovered: Arc<Mutex<HashMap<String, DiscoveredPeer>>>,
    sessions: Arc<Mutex<HashMap<String, PeerSession>>>,
    peer_addr: String,
) -> Result<SyncSessionResult, String> {
    let mut session = NoiseSession::handshake_responder(transport, keys)?;

    let (peer_node_id, peer_account, fingerprint) = match recv_msg(&mut session, transport)? {
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

    if let Ok(mut map) = discovered.lock() {
        map.insert(
            peer_node_id.clone(),
            DiscoveredPeer {
                node_id: peer_node_id.clone(),
                account_id: peer_account,
                name: peer_node_id.clone(),
                addr: peer_addr
                    .parse()
                    .unwrap_or_else(|_| SocketAddr::from(([127, 0, 0, 1], 0))),
                fingerprint,
                last_seen: Instant::now(),
            },
        );
    }

    {
        let mut sess = sessions.lock().map_err(|e| e.to_string())?;
        sess.insert(
            peer_node_id.clone(),
            PeerSession {
                node_id: peer_node_id.clone(),
                started_at: Instant::now(),
            },
        );
    }

    // Receive peer changes first and apply incrementally.
    let mut apply_stats = ApplyStats::default();
    loop {
        let msg = recv_msg(&mut session, transport)?;
        match msg {
            SyncMessage::Batch { table, records, .. } => {
                let stats = apply_sync_records(&vault, &table, &records, node_id)?;
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

    // Send our changes back, paginated by table.
    for table in SYNC_TABLES {
        loop {
            let watermark = vault_to_watermark(&vault.get_peer_watermark(&peer_node_id, table)?);
            let page = generate_delta_paginated(
                &vault,
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
                &mut session,
                transport,
                &SyncMessage::Batch {
                    table: table.to_string(),
                    records: page.records,
                    finished,
                },
            )?;
            let ack = recv_msg(&mut session, transport)?;
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
                    &peer_node_id,
                    table,
                    &watermark_to_vault(&hlc_to_sync_watermark(&max)),
                )?;
            }

            if finished {
                break;
            }
        }
    }
    send_msg(&mut session, transport, &SyncMessage::Done)?;

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

    {
        let mut sess = sessions.lock().map_err(|e| e.to_string())?;
        sess.remove(&peer_node_id);
    }
    Ok(SyncSessionResult {
        data: apply_stats,
        attachments: attachment_stats,
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use solosoul_vault::{ObjectRecord, PeerSyncState, Profile, VaultConfig, VaultStore};
    use std::thread;
    use tempfile::TempDir;

    fn trust_peer(vault: &VaultStore, peer_node_id: &str, fingerprint: &str) {
        let now = chrono::Utc::now().to_rfc3339();
        vault
            .save_peer_state(&PeerSyncState {
                peer_node_id: peer_node_id.to_string(),
                peer_name: None,
                trusted: true,
                public_key_fingerprint: Some(fingerprint.to_string()),
                last_seen: None,
                created_at: now.clone(),
                updated_at: now,
            })
            .unwrap();
    }

    fn open_test_vault(account_id: &str, path: std::path::PathBuf) -> VaultStore {
        let config = VaultConfig::new(account_id, path).with_data_key([1u8; 32]);
        VaultStore::open(config).unwrap()
    }

    fn make_object_with_attachment(
        object_id: &str,
        account_id: &str,
        attachment_id: &str,
        file_name: &str,
    ) -> ObjectRecord {
        ObjectRecord {
            contract_type_id: None,
            id: object_id.to_string(),
            account_id: account_id.to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "Note".to_string(),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({
                "__attachments": [{
                    "id": attachment_id,
                    "objectId": object_id,
                    "fileName": file_name,
                    "mimeType": "text/plain",
                    "sizeBytes": 13,
                    "createdAt": "2024-01-01T00:00:00Z"
                }]
            }),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
        }
    }

    #[test]
    fn test_exchange_attachments_over_noise() {
        let dir_a = TempDir::new().unwrap();
        let dir_b = TempDir::new().unwrap();
        let vault_a = open_test_vault("acc_att", dir_a.path().to_path_buf());
        let vault_b = open_test_vault("acc_att", dir_b.path().to_path_buf());

        // Prepare attachment on A.
        let object_id = "obj-1";
        let attachment_id = "att-1";
        let file_name = "hello.txt";
        let record = make_object_with_attachment(object_id, "acc_att", attachment_id, file_name);
        vault_a.save_object(&record).unwrap();
        let att_dir = dir_a
            .path()
            .join("attachments")
            .join(object_id)
            .join(attachment_id);
        std::fs::create_dir_all(&att_dir).unwrap();
        let att_path = att_dir.join(file_name);
        std::fs::write(&att_path, b"hello world!\n").unwrap();

        let manifests = collect_attachment_manifests(&vault_a, "acc_att").unwrap();
        assert_eq!(
            manifests.len(),
            1,
            "expected one manifest, got {:?}",
            manifests
        );

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();

        let server_keys = NoiseKeys::generate();
        let client_keys = NoiseKeys::generate();

        let server_thread = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut transport = SyncTransport::from_stream(stream);
            let mut session =
                NoiseSession::handshake_responder(&mut transport, &server_keys).unwrap();
            exchange_attachments(
                &mut session,
                &mut transport,
                &vault_b,
                "acc_att",
                vault_b.base_path(),
                true,
            )
            .unwrap()
        });

        let client_thread = thread::spawn(move || {
            let stream = std::net::TcpStream::connect(&addr).unwrap();
            let mut transport = SyncTransport::from_stream(stream);
            let mut session =
                NoiseSession::handshake_initiator(&mut transport, &client_keys).unwrap();
            exchange_attachments(
                &mut session,
                &mut transport,
                &vault_a,
                "acc_att",
                vault_a.base_path(),
                false,
            )
            .unwrap()
        });

        let server_stats = server_thread.join().unwrap();
        let client_stats = client_thread.join().unwrap();

        // A sent one attachment, B received one.
        assert_eq!(client_stats.sent, 1);
        assert_eq!(server_stats.received, 1);

        let received_path = dir_b
            .path()
            .join("attachments")
            .join(object_id)
            .join(attachment_id)
            .join(file_name);
        assert!(received_path.exists());
        assert_eq!(
            std::fs::read_to_string(&received_path).unwrap(),
            "hello world!\n"
        );
    }

    #[test]
    fn test_profile_tombstone_propagation_over_noise() {
        let dir_a = TempDir::new().unwrap();
        let dir_b = TempDir::new().unwrap();
        let vault_a = Arc::new(open_test_vault("acc_tomb", dir_a.path().to_path_buf()));
        let vault_b = Arc::new(open_test_vault("acc_tomb", dir_b.path().to_path_buf()));
        vault_a.set_sync_node_id("node_a").unwrap();
        vault_b.set_sync_node_id("node_b").unwrap();

        let server_keys = NoiseKeys::generate();
        let client_keys = NoiseKeys::generate();
        let server_keys2 = server_keys.clone();
        let client_keys2 = client_keys.clone();
        trust_peer(&vault_a, "node_b", &server_keys.fingerprint());
        trust_peer(&vault_b, "node_a", &client_keys.fingerprint());

        // 1. Create profile on A.
        let profile = Profile::new_with_id("p1", "Travel", b"encrypted".to_vec());
        vault_a.save_profile(&profile).unwrap();
        assert!(vault_a.load_profile("p1").unwrap().is_some());

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let vault_a2 = Arc::clone(&vault_a);
        let vault_b2 = Arc::clone(&vault_b);
        let server_thread = thread::spawn(move || {
            let (stream, peer_addr) = listener.accept().unwrap();
            let mut transport = SyncTransport::from_stream(stream);
            handle_inbound(
                &mut transport,
                "node_b",
                "acc_tomb",
                &server_keys,
                vault_b2,
                Arc::new(Mutex::new(HashMap::new())),
                Arc::new(Mutex::new(HashMap::new())),
                peer_addr.to_string(),
            )
            .unwrap()
        });

        let client_thread = thread::spawn(move || {
            let stream =
                std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(5)).unwrap();
            let mut transport = SyncTransport::from_stream(stream);
            run_initiator_session(
                &mut transport,
                "node_a",
                "acc_tomb",
                &client_keys,
                vault_a2,
                addr.to_string(),
            )
            .unwrap()
        });

        let _server_result = server_thread.join().unwrap();
        let _client_result = client_thread.join().unwrap();

        // Profile should now exist on B.
        assert!(
            vault_b.load_profile("p1").unwrap().is_some(),
            "profile should be synced to B"
        );

        // 2. Delete profile on A.
        vault_a.delete_profile("p1").unwrap();
        assert!(vault_a.load_profile("p1").unwrap().is_none());

        // 3. Sync again to propagate the tombstone.
        let listener2 = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr2 = listener2.local_addr().unwrap();

        let vault_a3 = Arc::clone(&vault_a);
        let vault_b3 = Arc::clone(&vault_b);
        let server_thread2 = thread::spawn(move || {
            let (stream, peer_addr) = listener2.accept().unwrap();
            let mut transport = SyncTransport::from_stream(stream);
            handle_inbound(
                &mut transport,
                "node_b",
                "acc_tomb",
                &server_keys2,
                vault_b3,
                Arc::new(Mutex::new(HashMap::new())),
                Arc::new(Mutex::new(HashMap::new())),
                peer_addr.to_string(),
            )
            .unwrap()
        });

        let client_thread2 = thread::spawn(move || {
            let stream =
                std::net::TcpStream::connect_timeout(&addr2, Duration::from_secs(5)).unwrap();
            let mut transport = SyncTransport::from_stream(stream);
            run_initiator_session(
                &mut transport,
                "node_a",
                "acc_tomb",
                &client_keys2,
                vault_a3,
                addr2.to_string(),
            )
            .unwrap()
        });

        let _server_result2 = server_thread2.join().unwrap();
        let _client_result2 = client_thread2.join().unwrap();

        assert!(
            vault_b.load_profile("p1").unwrap().is_none(),
            "profile should be deleted on B after tombstone sync"
        );
    }

    #[test]
    fn test_large_profile_table_paginated_sync() {
        let dir_a = TempDir::new().unwrap();
        let dir_b = TempDir::new().unwrap();
        let vault_a = Arc::new(open_test_vault("acc_pag", dir_a.path().to_path_buf()));
        let vault_b = Arc::new(open_test_vault("acc_pag", dir_b.path().to_path_buf()));
        vault_a.set_sync_node_id("node_a").unwrap();
        vault_b.set_sync_node_id("node_b").unwrap();

        let server_keys = NoiseKeys::generate();
        let client_keys = NoiseKeys::generate();
        trust_peer(&vault_a, "node_b", &server_keys.fingerprint());
        trust_peer(&vault_b, "node_a", &client_keys.fingerprint());

        // Create enough profiles to exceed the 500-record page limit.
        const PROFILE_COUNT: usize = 550;
        for i in 0..PROFILE_COUNT {
            let profile = Profile::new_with_id(
                &format!("p-{}", i),
                &format!("Profile {}", i),
                b"encrypted payload".to_vec(),
            );
            vault_a.save_profile(&profile).unwrap();
        }

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let vault_a2 = Arc::clone(&vault_a);
        let vault_b2 = Arc::clone(&vault_b);
        let server_thread = thread::spawn(move || {
            let (stream, peer_addr) = listener.accept().unwrap();
            let mut transport = SyncTransport::from_stream(stream);
            handle_inbound(
                &mut transport,
                "node_b",
                "acc_pag",
                &server_keys,
                vault_b2,
                Arc::new(Mutex::new(HashMap::new())),
                Arc::new(Mutex::new(HashMap::new())),
                peer_addr.to_string(),
            )
            .unwrap()
        });

        let client_thread = thread::spawn(move || {
            let stream =
                std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(5)).unwrap();
            let mut transport = SyncTransport::from_stream(stream);
            run_initiator_session(
                &mut transport,
                "node_a",
                "acc_pag",
                &client_keys,
                vault_a2,
                addr.to_string(),
            )
            .unwrap()
        });

        let server_result = server_thread.join().unwrap();
        let _client_result = client_thread.join().unwrap();

        assert_eq!(server_result.data.applied, PROFILE_COUNT as u64);
        assert_eq!(vault_b.list_profiles().unwrap().len(), PROFILE_COUNT);

        // A second sync should be a no-op because the watermark has advanced.
        let watermark = vault_a.get_peer_watermark("node_b", "profiles").unwrap();
        assert!(watermark.wall_time_ms > 0);
    }
}
