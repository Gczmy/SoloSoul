//! RF-903：仅清理原会话密钥域内、完整恢复引用均不存在的非空加密附件。
//!
//! Root 维护窗口阻止可信写入穿越文件发布与元数据提交之间；同用户外部进程
//! 仍可能在系统调用间更改文件。删除前复核句柄身份、路径、内容头与数据库视图，
//! 任一不确定就保留，不声称实现对恶意外部进程的原子安全擦除。
use crate::import_activity::{begin_owned_root_maintenance, RootMaintenanceGuard};
use crate::{VaultService, VaultSession};
use solosoul_crypto::cipher::{authenticate_nonempty_chunked_v2_stream, ChunkedOwnership};
use solosoul_vault::{AttachmentReferenceCandidate, ImportOwnedAttachmentMarker};
use std::collections::BTreeSet;
use std::fs::{self, File, Metadata};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;

mod identity;
use identity::{
    file_identity, link_or_reparse, open_directory, open_regular_file, FileIdentity, FileStamp,
};

/// removed 只计完整移除的附件目录；freed_bytes/files_removed 只计实际成功移除的
/// regular files，含已验证的 import 控制文件。字节数为逻辑长度，不是介质擦除量。
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct OrphanCleanupReport {
    pub removed: usize,
    pub freed_bytes: u64,
    pub files_removed: usize,
    pub preserved: usize,
    pub failed: usize,
    /// 仅表示整轮提前停止；单候选 IO 失败允许继续，通过 failed 和固定审计码报告。
    pub failure_code: Option<String>,
}

pub fn cleanup_orphan_attachments_for_session(
    service: &VaultService,
    session: &VaultSession,
) -> Result<OrphanCleanupReport, String> {
    service.with_session(session, |_| Ok(()))?;
    let guard = begin_owned_root_maintenance(session.root_owner())?;
    cleanup_orphan_attachments_with_maintenance(service, session, &guard)
}

/// 调用方可在同一排他窗口先重试 RF-016；不得携带普通 activity 再申请维护。
pub fn cleanup_orphan_attachments_with_maintenance(
    service: &VaultService,
    session: &VaultSession,
    guard: &RootMaintenanceGuard,
) -> Result<OrphanCleanupReport, String> {
    cleanup_in_window(service, session, guard, |_| {})
}

fn cleanup_in_window(
    service: &VaultService,
    session: &VaultSession,
    guard: &RootMaintenanceGuard,
    observer: impl FnMut(&Path),
) -> Result<OrphanCleanupReport, String> {
    cleanup_with_observers(service, session, guard, |_| {}, observer)
}

// 两个真实观察窗口：before_scan 在所有后代句柄打开前；after_auth 保留原认证后窗口。
// 生产入口两者均为空；不能为了测试根替换而释放已经冻结的身份句柄。
fn cleanup_with_observers(
    service: &VaultService,
    session: &VaultSession,
    guard: &RootMaintenanceGuard,
    before_scan: impl FnOnce(&Path),
    mut observer: impl FnMut(&Path),
) -> Result<OrphanCleanupReport, String> {
    service.with_session(session, |_| Ok(()))?;
    let owner = session.root_owner();
    if !Arc::ptr_eq(&owner, &guard.root_owner()) || !Arc::ptr_eq(&owner, &service.root_owner()) {
        return Err("attachment_cleanup_owner_mismatch".into());
    }
    let root = FrozenDirectory::open(owner.root())?;
    let attachment_path = owner.root().join("attachments");
    match fs::symlink_metadata(&attachment_path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return service.with_session(session, |_| Ok(OrphanCleanupReport::default()));
        }
        Err(e) => return Err(io_code(&e)),
        Ok(_) => {}
    }
    let attachments = FrozenDirectory::open(&attachment_path)?;
    let key = service.attachment_key_for_session(session)?;
    // 完整扫描不持会话门闩；锁定会关闭原 Store，后续检查不会迁移到新账户。
    let view = session
        .vault()
        .read_attachment_reference_view(session.account_id())?;
    service.with_session(session, |_| Ok(()))?;
    let mut report = OrphanCleanupReport::default();
    // 完整引用视图已冻结且原会话已验证，此时尚未打开 attachments 的任何后代。
    // 必须先复核再枚举：被替换成 empty 目录不能因零候选而错误报告正常完成。
    before_scan(&attachment_path);
    if root.validate().is_err() || attachments.validate().is_err() {
        stop(&mut report, "attachment_cleanup_root_changed");
        audit_report(service, session, &report);
        return Ok(report);
    }
    let objects = read_children(&attachment_path)?;
    'scan: for object_path in objects {
        if root.validate().is_err() || attachments.validate().is_err() {
            stop(&mut report, "attachment_cleanup_root_changed");
            break;
        }
        let object = match FrozenDirectory::open(&object_path) {
            Ok(directory) if path_id(&object_path).is_some() => directory,
            Ok(_) => {
                report.preserved += 1;
                continue;
            }
            Err(code) => {
                record_candidate_failure(&mut report, &code);
                continue;
            }
        };
        let children = match read_children(&object_path) {
            Ok(children) => children,
            Err(code) => {
                report.failed += 1;
                trace_failure(&code);
                continue;
            }
        };
        for candidate_path in children {
            // 已知 sidecar 仅随其对应已认证目录一起处理；不单独扫描控制文件。
            if candidate_path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(".import-owner"))
            {
                continue;
            }
            let Some(attachment_id) = path_id(&candidate_path) else {
                report.preserved += 1;
                continue;
            };
            let Some(storage_object_id) = path_id(&object_path) else {
                unreachable!()
            };
            if service.with_session(session, |_| Ok(())).is_err() {
                stop(&mut report, "attachment_cleanup_session_stale");
                break 'scan;
            }
            let preparation = prepare_directory(
                session,
                &candidate_path,
                &storage_object_id,
                &attachment_id,
                &key,
            );
            if service.with_session(session, |_| Ok(())).is_err() {
                stop(&mut report, "attachment_cleanup_session_stale");
                break 'scan;
            }
            let mut prepared = match preparation {
                Ok(Some(prepared)) => prepared,
                Ok(None) => {
                    report.preserved += 1;
                    continue;
                }
                Err(code) if code == "Vault session is no longer current" => {
                    stop(&mut report, "attachment_cleanup_session_stale");
                    break 'scan;
                }
                Err(code) if is_file_error(&code) => {
                    record_candidate_failure(&mut report, &code);
                    continue;
                }
                Err(_) => {
                    stop(&mut report, "attachment_cleanup_reference_error");
                    break 'scan;
                }
            };
            let literal_aliases = vec![service
                .base_path()
                .join("attachments")
                .join(&storage_object_id)
                .join(&attachment_id)
                .to_string_lossy()
                .into_owned()];
            let candidate = AttachmentReferenceCandidate {
                storage_object_id,
                attachment_id,
                canonical_path: candidate_path.clone(),
                literal_aliases,
            };
            // 生产调用为空；回归观察点位于真实认证之后，未持会话/DB锁，可实际锁定、
            // 变更数据库或替换文件，验证最终门闩而不是模拟成功路径。
            observer(&candidate_path);
            if report
                .freed_bytes
                .checked_add(prepared.total_bytes())
                .is_none()
                || report
                    .files_removed
                    .checked_add(prepared.files.len())
                    .is_none()
            {
                stop(&mut report, "attachment_cleanup_count_overflow");
                break 'scan;
            }
            let result = service.with_session(session, |vault| {
                root.validate()
                    .map_err(|_| "attachment_cleanup_root_changed")?;
                attachments
                    .validate()
                    .map_err(|_| "attachment_cleanup_root_changed")?;
                vault.with_unreferenced_attachment_guard(
                    session.account_id(),
                    &view,
                    &candidate,
                    || {
                        // 事务内只做有限的身份/头部检查和 unlink/rmdir，不重做昂贵 AEAD。
                        root.validate()
                            .map_err(|_| "attachment_cleanup_root_changed")?;
                        attachments
                            .validate()
                            .map_err(|_| "attachment_cleanup_root_changed")?;
                        object.validate()?;
                        prepared.remove()
                    },
                )
            });
            match result {
                Ok(None) => report.preserved += 1,
                Ok(Some(outcome)) => {
                    report.freed_bytes += outcome.bytes;
                    report.files_removed += outcome.files;
                    if outcome.complete {
                        report.removed += 1;
                    }
                    if let Some(code) = outcome.error {
                        report.failed += 1;
                        trace_failure(&code);
                    }
                }
                Err(code) if code == "Vault session is no longer current" => {
                    stop(&mut report, "attachment_cleanup_session_stale");
                    break 'scan;
                }
                Err(code) if code == "attachment_cleanup_root_changed" => {
                    stop(&mut report, "attachment_cleanup_root_changed");
                    break 'scan;
                }
                Err(code) if code == "attachment_cleanup_view_changed" => {
                    stop(&mut report, "attachment_cleanup_view_changed");
                    break 'scan;
                }
                Err(code) if is_file_error(&code) => {
                    report.failed += 1;
                    trace_failure(&code);
                }
                Err(_) => {
                    stop(&mut report, "attachment_cleanup_reference_error");
                    break 'scan;
                }
            }
        }
        // 本轮不认领空对象目录，也不删除未知 sidecar；只处理已认证附件目录。
    }
    // 零候选或最后一个候选之后也核对原会话，不能因循环未执行而报告旧会话成功。
    if report.failure_code.is_none() && service.with_session(session, |_| Ok(())).is_err() {
        stop(&mut report, "attachment_cleanup_session_stale");
    }
    audit_report(service, session, &report);
    Ok(report)
}

fn stop(report: &mut OrphanCleanupReport, code: &str) {
    report.failed += 1;
    report.failure_code = Some(code.into());
    trace_failure(code);
}
fn trace_failure(code: &str) {
    tracing::warn!(reason = code, "attachment_orphan_cleanup_preserved");
}
fn is_file_error(code: &str) -> bool {
    matches!(
        code,
        "attachment_cleanup_io_error"
            | "attachment_cleanup_permission_denied"
            | "attachment_cleanup_path_changed"
            | "attachment_cleanup_file_changed"
            | "attachment_cleanup_unsafe_path"
            | "attachment_cleanup_identity_unavailable"
    )
}
fn record_candidate_failure(report: &mut OrphanCleanupReport, code: &str) {
    if code == "attachment_cleanup_unsafe_path" {
        report.preserved += 1;
    } else {
        report.failed += 1;
    }
    trace_failure(code);
}
fn audit_report(service: &VaultService, session: &VaultSession, report: &OrphanCleanupReport) {
    let detail = format!(
        "removed={} files_removed={} freed={} preserved={} failed={} stopped={}",
        report.removed,
        report.files_removed,
        report.freed_bytes,
        report.preserved,
        report.failed,
        report.failure_code.as_deref().unwrap_or("none")
    );
    let logged = service.with_session(session, |vault| {
        vault.log_structured(
            "attachment_cleanup",
            "attachment",
            None,
            None,
            "system",
            Some(&detail),
        )
    });
    if logged.is_err() {
        tracing::warn!("attachment_orphan_cleanup_audit_unavailable");
    }
}
fn io_code(error: &io::Error) -> String {
    if error.kind() == io::ErrorKind::PermissionDenied {
        "attachment_cleanup_permission_denied"
    } else {
        "attachment_cleanup_io_error"
    }
    .into()
}
fn path_id(path: &Path) -> Option<String> {
    let id = path.file_name()?.to_str()?;
    if id.is_empty()
        || id.len() > 64
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return None;
    }
    let upper = id.to_ascii_uppercase();
    if matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (upper.len() == 4
            && (upper.starts_with("COM") || upper.starts_with("LPT"))
            && matches!(upper.as_bytes()[3], b'1'..=b'9'))
    {
        return None;
    }
    Some(id.into())
}
fn regular_node(path: &Path, directory: bool) -> Result<Metadata, String> {
    let meta = fs::symlink_metadata(path).map_err(|e| io_code(&e))?;
    if link_or_reparse(&meta) || (directory && !meta.is_dir()) || (!directory && !meta.is_file()) {
        return Err("attachment_cleanup_unsafe_path".into());
    }
    if fs::canonicalize(path).map_err(|e| io_code(&e))? != path {
        return Err("attachment_cleanup_unsafe_path".into());
    }
    Ok(meta)
}
fn read_children(path: &Path) -> Result<Vec<PathBuf>, String> {
    let mut paths = fs::read_dir(path)
        .map_err(|e| io_code(&e))?
        .map(|e| e.map(|e| e.path()).map_err(|e| io_code(&e)))
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();
    Ok(paths)
}

struct FrozenDirectory {
    path: PathBuf,
    file: File,
    identity: FileIdentity,
}
impl FrozenDirectory {
    fn open(path: &Path) -> Result<Self, String> {
        regular_node(path, true)?;
        let file = open_directory(path).map_err(|e| io_code(&e))?;
        if !file.metadata().map_err(|e| io_code(&e))?.is_dir() {
            return Err("attachment_cleanup_unsafe_path".into());
        }
        let identity =
            file_identity(&file).map_err(|_| "attachment_cleanup_identity_unavailable")?;
        let result = Self {
            path: path.into(),
            file,
            identity,
        };
        result.validate()?;
        Ok(result)
    }
    fn validate(&self) -> Result<(), String> {
        regular_node(&self.path, true)?;
        let current = open_directory(&self.path).map_err(|e| io_code(&e))?;
        if file_identity(&self.file).map_err(|_| "attachment_cleanup_identity_unavailable")?
            != self.identity
            || file_identity(&current).map_err(|_| "attachment_cleanup_identity_unavailable")?
                != self.identity
        {
            return Err("attachment_cleanup_path_changed".into());
        }
        Ok(())
    }
}

struct FrozenFile {
    path: PathBuf,
    file: File,
    stamp: FileStamp,
    header: Option<[u8; 25]>,
    control: Option<Vec<u8>>,
}
impl FrozenFile {
    fn open(path: &Path) -> Result<Self, String> {
        regular_node(path, false)?;
        let file = open_regular_file(path).map_err(|e| io_code(&e))?;
        let meta = file.metadata().map_err(|e| io_code(&e))?;
        if !meta.is_file() || link_or_reparse(&meta) {
            return Err("attachment_cleanup_unsafe_path".into());
        }
        let stamp =
            FileStamp::read(&file).map_err(|_| "attachment_cleanup_identity_unavailable")?;
        // 不认领多 hardlink 文件，也不把 unlink 一个别名统计为释放整个文件。
        if !stamp.single_link() {
            return Err("attachment_cleanup_unsafe_path".into());
        }
        let mut result = Self {
            path: path.into(),
            file,
            stamp,
            header: None,
            control: None,
        };
        result.validate()?;
        Ok(result)
    }
    fn validate(&mut self) -> Result<(), String> {
        regular_node(&self.path, false)?;
        let mut current = open_regular_file(&self.path).map_err(|e| io_code(&e))?;
        if FileStamp::read(&self.file).map_err(|_| "attachment_cleanup_identity_unavailable")?
            != self.stamp
            || FileStamp::read(&current).map_err(|_| "attachment_cleanup_identity_unavailable")?
                != self.stamp
        {
            return Err("attachment_cleanup_file_changed".into());
        }
        if let Some(expected) = &self.header {
            let mut actual = [0; 25];
            current.read_exact(&mut actual).map_err(|e| io_code(&e))?;
            if &actual != expected {
                return Err("attachment_cleanup_file_changed".into());
            }
        }
        if let Some(expected) = &self.control {
            current.seek(SeekFrom::Start(0)).map_err(|e| io_code(&e))?;
            let mut actual = vec![0; expected.len()];
            current.read_exact(&mut actual).map_err(|e| io_code(&e))?;
            if &actual != expected {
                return Err("attachment_cleanup_file_changed".into());
            }
        }
        Ok(())
    }
}

struct PreparedDirectory {
    directory: FrozenDirectory,
    files: Vec<FrozenFile>,
    entries: BTreeSet<PathBuf>,
    sidecar: PathBuf,
    has_sidecar: bool,
}
struct RemovalOutcome {
    complete: bool,
    files: usize,
    bytes: u64,
    error: Option<String>,
}
impl PreparedDirectory {
    fn total_bytes(&self) -> u64 {
        self.files.iter().map(|f| f.stamp.length()).sum()
    }
    fn remove(&mut self) -> Result<RemovalOutcome, String> {
        self.directory.validate()?;
        // 同样冻结“不存在”；新 sidecar 可能是未知/其他归属，不能只验证已打开文件。
        if !self.has_sidecar {
            match fs::symlink_metadata(&self.sidecar) {
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(io_code(&e)),
                Ok(_) => return Err("attachment_cleanup_file_changed".into()),
            }
        }
        // sidecar 位于 object 中，不属于内层目录的 entries。
        if read_children(&self.directory.path)?
            .into_iter()
            .collect::<BTreeSet<_>>()
            != self.entries
        {
            return Err("attachment_cleanup_file_changed".into());
        }
        for file in &mut self.files {
            file.validate()?;
        }
        let mut outcome = RemovalOutcome {
            complete: false,
            files: 0,
            bytes: 0,
            error: None,
        };
        // payload 全部通过认证后才执行；控制文件最后删，部分失败保留可追溯状态。
        self.files.sort_by_key(|file| file.control.is_some());
        for file in &mut self.files {
            if let Err(code) = file.validate() {
                outcome.error = Some(code);
                return Ok(outcome);
            }
            match fs::remove_file(&file.path) {
                Ok(()) => {
                    outcome.files += 1;
                    outcome.bytes += file.stamp.length();
                }
                Err(error) => {
                    outcome.error = Some(io_code(&error));
                    return Ok(outcome);
                }
            }
        }
        match fs::remove_dir(&self.directory.path) {
            Ok(()) => outcome.complete = true,
            Err(error) => outcome.error = Some(io_code(&error)),
        }
        Ok(outcome)
    }
}

fn prepare_directory(
    session: &VaultSession,
    path: &Path,
    object_id: &str,
    attachment_id: &str,
    key: &[u8; 32],
) -> Result<Option<PreparedDirectory>, String> {
    let directory = FrozenDirectory::open(path)?;
    let children = read_children(path)?;
    let entries: BTreeSet<PathBuf> = children.iter().cloned().collect();
    let inner = path.join(crate::export_import::operation::IMPORT_OWNER_MARKER);
    let sidecar = path.with_file_name(format!("{attachment_id}.import-owner"));
    let mut files = Vec::new();
    let mut markers = Vec::new();
    let mut authenticated = 0;
    for child in children.into_iter().chain(std::iter::once(sidecar.clone())) {
        let control = child == inner || child == sidecar;
        if control && !entries.contains(&child) {
            match fs::symlink_metadata(&child) {
                Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                Err(e) => return Err(io_code(&e)),
                Ok(_) => {}
            }
        }
        let mut file = FrozenFile::open(&child)?;
        if control {
            if file.stamp.length() > 4096 {
                return Ok(None);
            }
            let mut bytes = vec![0; file.stamp.length() as usize];
            file.file.read_exact(&mut bytes).map_err(|e| io_code(&e))?;
            let Ok(marker) = serde_json::from_slice::<ImportOwnedAttachmentMarker>(&bytes) else {
                return Ok(None);
            };
            file.control = Some(bytes);
            file.validate()?;
            markers.push(marker);
        } else {
            file.file
                .seek(SeekFrom::Start(0))
                .map_err(|e| io_code(&e))?;
            match authenticate_nonempty_chunked_v2_stream(key, &mut file.file, file.stamp.length())
                .map_err(|e| io_code(&e))?
            {
                ChunkedOwnership::Retained(_) => return Ok(None),
                ChunkedOwnership::Authenticated(proof) => {
                    file.header = Some(proof.header);
                    authenticated += 1;
                }
            }
            file.validate()?;
        }
        files.push(file);
    }
    if authenticated == 0 {
        return Ok(None);
    }
    if let Some(marker) = markers.first() {
        if markers.iter().any(|m| m != marker)
            || marker.owner_id != object_id
            || marker.attachment_id != attachment_id
        {
            return Ok(None);
        }
        // 已知 marker 需要真实已认证 journal 的终态授权；此处不删除。最终 guard
        // 会复核包含 journal 的完整视图，任何期间变更都会拒绝文件操作。
        if session
            .vault()
            .with_import_orphan_delete_guard(session.account_id(), marker, || Ok(()))?
            .is_none()
        {
            return Ok(None);
        }
    }
    if files
        .iter()
        .try_fold(0u64, |total, file| total.checked_add(file.stamp.length()))
        .is_none()
    {
        return Err("attachment_cleanup_count_overflow".into());
    }
    let has_sidecar = files.iter().any(|file| file.path == sidecar);
    Ok(Some(PreparedDirectory {
        directory,
        files,
        entries,
        sidecar,
        has_sidecar,
    }))
}

#[cfg(test)]
mod tests;
