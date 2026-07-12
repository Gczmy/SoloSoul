//! Attachment file synchronization over an existing Noise session.
//!
//! Attachments are referenced from object `properties.__attachments`. Metadata
//! (id, file_name, size) is synced as part of the object record; this module
//! transfers the actual file bytes in 64 KiB chunks over the same encrypted
//! channel used for the main sync batch.

use crate::noise::NoiseSession;
use crate::protocol::{AttachmentInfo, SyncMessage};
use crate::transport::SyncTransport;
use crate::types::AttachmentSyncStats;
use serde_json::Value;
use solosoul_vault::VaultStore;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const CHUNK_SIZE: usize = 64 * 1024;

fn attachments_dir(base: &Path) -> PathBuf {
    base.join("attachments")
}

fn validate_attachment_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.len() > 64
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!("Invalid attachment id: {}", id));
    }
    Ok(())
}

fn sanitize_file_name(name: &str) -> Result<String, String> {
    Path::new(name)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .ok_or_else(|| format!("Invalid attachment file name: {}", name))
}

fn attachment_file_path(
    base: &Path,
    object_id: &str,
    attachment_id: &str,
    file_name: &str,
) -> Result<PathBuf, String> {
    validate_attachment_id(object_id)?;
    validate_attachment_id(attachment_id)?;
    let safe_name = sanitize_file_name(file_name)?;
    Ok(attachments_dir(base)
        .join(object_id)
        .join(attachment_id)
        .join(safe_name))
}

pub(crate) fn sha256_file(path: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let mut file = fs::File::open(path).map_err(|e| format!("open: {}", e))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf).map_err(|e| format!("read: {}", e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Parse `__attachments` from object properties and build attachment manifests.
pub fn collect_attachment_manifests(
    vault: &VaultStore,
    account_id: &str,
) -> Result<Vec<(String, Vec<AttachmentInfo>)>, String> {
    let objects = vault
        .list_objects(account_id, None, None, None, false, false)
        .map_err(|e| format!("list_objects: {}", e))?;
    let base = vault.base_path();
    let mut manifests = Vec::new();
    for summary in objects {
        if let Ok(Some(rec)) = vault.load_object(&summary.id) {
            if let Some(attachments) = parse_attachments(&rec.properties) {
                let infos: Vec<AttachmentInfo> = attachments
                    .into_iter()
                    .filter(|a| a.deleted_at.is_none())
                    .filter_map(|a| {
                        let path =
                            attachment_file_path(base, &a.object_id, &a.id, &a.file_name).ok()?;
                        if !path.exists() {
                            return None;
                        }
                        let sha256 = sha256_file(&path).ok()?;
                        Some(AttachmentInfo {
                            id: a.id,
                            object_id: a.object_id,
                            file_name: a.file_name,
                            size_bytes: a.size_bytes,
                            sha256,
                        })
                    })
                    .collect();
                if !infos.is_empty() {
                    manifests.push((summary.id.clone(), infos));
                }
            }
        }
    }
    Ok(manifests)
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AttachmentMeta {
    id: String,
    object_id: String,
    file_name: String,
    #[serde(default)]
    size_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    deleted_at: Option<String>,
}

fn parse_attachments(props: &Value) -> Option<Vec<AttachmentMeta>> {
    props
        .get("__attachments")
        .and_then(|v| serde_json::from_value::<Vec<AttachmentMeta>>(v.clone()).ok())
}

/// Send all attachment manifests, then a manifest-done marker.
pub fn send_attachment_manifests(
    session: &mut NoiseSession,
    transport: &mut SyncTransport,
    manifests: &[(String, Vec<AttachmentInfo>)],
) -> Result<(), String> {
    for (object_id, attachments) in manifests {
        send_msg(
            session,
            transport,
            &SyncMessage::AttachmentManifest {
                object_id: object_id.clone(),
                attachments: attachments.clone(),
            },
        )?;
    }
    send_msg(session, transport, &SyncMessage::AttachmentManifestDone)
}

/// Receive attachment manifests until `AttachmentManifestDone`.
pub fn receive_attachment_manifests(
    session: &mut NoiseSession,
    transport: &mut SyncTransport,
) -> Result<HashMap<String, Vec<AttachmentInfo>>, String> {
    let mut manifests = HashMap::new();
    loop {
        match recv_msg(session, transport)? {
            SyncMessage::AttachmentManifest {
                object_id,
                attachments,
            } => {
                manifests.insert(object_id, attachments);
            }
            SyncMessage::AttachmentManifestDone => break,
            SyncMessage::Error { message } => return Err(message),
            _ => return Err("Unexpected message during attachment manifests".to_string()),
        }
    }
    Ok(manifests)
}

/// Decide which attachments the local side needs from the remote manifests.
pub fn compute_needed_attachments(
    base: &Path,
    remote_manifests: &HashMap<String, Vec<AttachmentInfo>>,
) -> Vec<(String, String)> {
    let mut needed = Vec::new();
    for (object_id, infos) in remote_manifests {
        for info in infos {
            let local_path = match attachment_file_path(base, object_id, &info.id, &info.file_name)
            {
                Ok(p) => p,
                Err(_) => continue,
            };
            let need = if !local_path.exists() {
                true
            } else {
                sha256_file(&local_path)
                    .map(|h| h != info.sha256)
                    .unwrap_or(true)
            };
            if need {
                needed.push((object_id.clone(), info.id.clone()));
            }
        }
    }
    needed
}

/// Send attachment requests (one per object) then a request-done marker.
pub fn send_attachment_requests(
    session: &mut NoiseSession,
    transport: &mut SyncTransport,
    needed: &[(String, String)],
) -> Result<(), String> {
    let mut by_object: HashMap<String, Vec<String>> = HashMap::new();
    for (object_id, att_id) in needed {
        by_object
            .entry(object_id.clone())
            .or_default()
            .push(att_id.clone());
    }
    for (object_id, attachment_ids) in by_object {
        send_msg(
            session,
            transport,
            &SyncMessage::AttachmentRequest {
                object_id,
                attachment_ids,
            },
        )?;
    }
    send_msg(session, transport, &SyncMessage::AttachmentRequestDone)
}

/// Receive attachment requests until `AttachmentRequestDone`.
pub fn receive_attachment_requests(
    session: &mut NoiseSession,
    transport: &mut SyncTransport,
) -> Result<Vec<(String, String)>, String> {
    let mut requested: HashMap<String, Vec<String>> = HashMap::new();
    loop {
        match recv_msg(session, transport)? {
            SyncMessage::AttachmentRequest {
                object_id,
                attachment_ids,
            } => {
                requested
                    .entry(object_id)
                    .or_default()
                    .extend(attachment_ids);
            }
            SyncMessage::AttachmentRequestDone => break,
            SyncMessage::Error { message } => return Err(message),
            _ => return Err("Unexpected message during attachment requests".to_string()),
        }
    }
    let mut out = Vec::new();
    for (object_id, ids) in requested {
        for id in ids {
            out.push((object_id.clone(), id));
        }
    }
    Ok(out)
}

/// Send requested attachments as chunked messages, then `AttachmentDone`.
pub fn send_requested_attachments(
    session: &mut NoiseSession,
    transport: &mut SyncTransport,
    base: &Path,
    requested: &[(String, String)],
    manifests: &HashMap<String, Vec<AttachmentInfo>>,
    stats: &mut AttachmentSyncStats,
) -> Result<(), String> {
    let mut info_map: HashMap<(String, String), AttachmentInfo> = HashMap::new();
    for (object_id, infos) in manifests {
        for info in infos {
            info_map.insert((object_id.clone(), info.id.clone()), info.clone());
        }
    }
    for (object_id, att_id) in requested {
        let info = info_map
            .get(&(object_id.clone(), att_id.clone()))
            .ok_or_else(|| format!("Unknown attachment {}:{}", object_id, att_id))?;
        let path = attachment_file_path(base, object_id, att_id, &info.file_name)?;
        let mut file =
            fs::File::open(&path).map_err(|e| format!("open {}: {}", path.display(), e))?;
        let total_size = fs::metadata(&path)
            .map_err(|e| format!("metadata {}: {}", path.display(), e))?
            .len() as usize;
        let total_chunks = total_size.div_ceil(CHUNK_SIZE).max(1) as u32;
        let mut buf = vec![0u8; CHUNK_SIZE];
        let mut chunk_index = 0u32;
        loop {
            let n = file
                .read(&mut buf)
                .map_err(|e| format!("read {}: {}", path.display(), e))?;
            if n == 0 {
                if total_size == 0 {
                    // Empty file: send one empty chunk.
                    send_msg(
                        session,
                        transport,
                        &SyncMessage::AttachmentChunk {
                            object_id: object_id.clone(),
                            attachment_id: att_id.clone(),
                            chunk_index: 0,
                            total_chunks: 1,
                            data: Vec::new(),
                        },
                    )?;
                    stats.bytes_transferred += 0;
                    stats.sent += 1;
                }
                break;
            }
            let data = buf[..n].to_vec();
            stats.bytes_transferred += n as u64;
            send_msg(
                session,
                transport,
                &SyncMessage::AttachmentChunk {
                    object_id: object_id.clone(),
                    attachment_id: att_id.clone(),
                    chunk_index,
                    total_chunks,
                    data,
                },
            )?;
            stats.sent += 1;

            // Wait for ack.
            let ack = recv_msg(session, transport)?;
            if let SyncMessage::AttachmentAck { .. } = ack {
                // ok
            } else {
                return Err(format!("Expected AttachmentAck, got {:?}", ack));
            }
            chunk_index += 1;
            if n < CHUNK_SIZE {
                break;
            }
        }
    }
    send_msg(session, transport, &SyncMessage::AttachmentDone)
}

/// Receive attachment chunks until `AttachmentDone`, verify sha256, save files.
pub fn receive_attachments(
    session: &mut NoiseSession,
    transport: &mut SyncTransport,
    base: &Path,
    remote_manifests: &HashMap<String, Vec<AttachmentInfo>>,
    stats: &mut AttachmentSyncStats,
) -> Result<(), String> {
    let mut info_map: HashMap<(String, String), AttachmentInfo> = HashMap::new();
    for (object_id, infos) in remote_manifests {
        for info in infos {
            info_map.insert((object_id.clone(), info.id.clone()), info.clone());
        }
    }

    type ChunkEntry = (u32, u32, Vec<u8>);
    let mut chunks_by_attachment: HashMap<(String, String), Vec<ChunkEntry>> = HashMap::new();

    loop {
        match recv_msg(session, transport)? {
            SyncMessage::AttachmentChunk {
                object_id,
                attachment_id,
                chunk_index,
                total_chunks,
                data,
            } => {
                stats.bytes_transferred += data.len() as u64;
                chunks_by_attachment
                    .entry((object_id.clone(), attachment_id.clone()))
                    .or_default()
                    .push((chunk_index, total_chunks, data));
                send_msg(
                    session,
                    transport,
                    &SyncMessage::AttachmentAck {
                        object_id,
                        attachment_id,
                        chunk_index,
                    },
                )?;
            }
            SyncMessage::AttachmentDone => break,
            SyncMessage::Error { message } => return Err(message),
            _ => return Err("Unexpected message during attachment transfer".to_string()),
        }
    }

    for ((object_id, att_id), chunks) in chunks_by_attachment {
        let info = info_map
            .get(&(object_id.clone(), att_id.clone()))
            .ok_or_else(|| format!("Received unknown attachment {}:{}", object_id, att_id))?;
        let path = attachment_file_path(base, &object_id, &att_id, &info.file_name)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {}", parent.display(), e))?;
        }
        let tmp = path.with_extension("tmp");
        let mut file =
            fs::File::create(&tmp).map_err(|e| format!("create tmp {}: {}", tmp.display(), e))?;
        let total_chunks = chunks.first().map(|(_, total, _)| *total).unwrap_or(1);
        if chunks.len() as u32 != total_chunks {
            let _ = fs::remove_file(&tmp);
            return Err(format!(
                "Attachment {}:{} chunks mismatch: {} / {}",
                object_id,
                att_id,
                chunks.len(),
                total_chunks
            ));
        }
        let mut sorted = chunks;
        sorted.sort_by_key(|(idx, _, _)| *idx);
        for (_, _, data) in &sorted {
            file.write_all(data)
                .map_err(|e| format!("write {}: {}", tmp.display(), e))?;
        }
        file.flush()
            .map_err(|e| format!("flush {}: {}", tmp.display(), e))?;
        drop(file);

        let actual_sha = sha256_file(&tmp)?;
        if actual_sha != info.sha256 {
            let _ = fs::remove_file(&tmp);
            return Err(format!(
                "Attachment {}:{} sha256 mismatch (expected {} got {})",
                object_id, att_id, info.sha256, actual_sha
            ));
        }
        fs::rename(&tmp, &path).map_err(|e| format!("rename {}: {}", path.display(), e))?;
        stats.received += 1;
    }

    Ok(())
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
