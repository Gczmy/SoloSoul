//! RF-024：Native 固定来源定位，Core 验证原账户 incoming 路径。
use crate::{VaultService, VaultSession};
use std::path::Path;
pub fn controlled_cloud_source(
    svc: &VaultService,
    session: &VaultSession,
    path: &Path,
) -> Result<(std::path::PathBuf, String, String), String> {
    svc.with_session(session, |_| Ok(()))?;
    let root = svc
        .base_path()
        .join("cloud_sync_incoming")
        .join(session.account_id())
        .canonicalize()
        .map_err(|_| "Invalid cloud snapshot root")?;
    let source = path
        .canonicalize()
        .map_err(|_| "Invalid cloud snapshot source")?;
    let relative = source
        .strip_prefix(&root)
        .map_err(|_| "Cloud snapshot belongs to another account")?;
    if relative.components().count() != 2
        || source.extension().and_then(|s| s.to_str()) != Some("solosoul")
    {
        return Err("Invalid cloud snapshot path".into());
    }
    let device = relative
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .ok_or("Invalid device ID")?
        .to_string();
    let hlc = relative
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Invalid snapshot HLC")?
        .to_string();
    validate_cloud_path_component(&device)?;
    validate_cloud_path_component(&hlc)?;
    Ok((source, device, hlc))
}

pub fn cloud_import_source_identity(
    svc: &VaultService,
    session: &VaultSession,
    path: &Path,
) -> Result<(String, String), String> {
    controlled_cloud_source(svc, session, path).map(|(_, device, hlc)| (device, hlc))
}

fn validate_cloud_path_component(value: &str) -> Result<(), String> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err("Invalid cloud snapshot path component".into());
    }
    Ok(())
}
