//! RF212：直接受 Tasks 管理的下载 Future，暂存文件由同一 Future 拥有。
//! 只在原会话提交门闩内发布完整文件；网络、哈希和文件同步均在门闩外。

use crate::tasks::{TaskContext, TaskFailure, TaskOutput};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs::{self, Metadata};
use std::io::{ErrorKind, Write};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;
use tempfile::{NamedTempFile, TempPath};

const WRITE_CHUNK_SIZE: usize = 64 * 1024;

#[derive(Deserialize)]
struct RegistryEntry {
    id: String,
    #[serde(rename = "name")]
    _name: String,
    #[serde(rename = "size_mb")]
    _size_mb: f32,
    #[serde(default, rename = "description")]
    _description: String,
    sha256: String,
    download_url: String,
}

#[derive(Deserialize)]
struct RegistryFile {
    #[serde(default)]
    models: Vec<RegistryEntry>,
}

/// 各平台使用同一目录组件规则，避免 Windows 设备名、路径和别名冲突。
pub(super) fn valid_model_id(id: &str) -> bool {
    if id.is_empty()
        || id == "."
        || id == ".."
        || id.ends_with('.')
        || !id
            .chars()
            .all(|ch| ch.is_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        return false;
    }
    let mut components = Path::new(id).components();
    if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
        return false;
    }
    let stem = id
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL") {
        return false;
    }
    for prefix in ["COM", "LPT"] {
        if let Some(suffix) = stem.strip_prefix(prefix) {
            if matches!(
                suffix,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            ) {
                return false;
            }
        }
    }
    true
}

fn is_link(metadata: &Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // Junction 等 reparse point 也不能作为模型目录或模型文件。
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn ordinary_directory(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_dir() && !is_link(&metadata))
}

/// 存量格式没有持久摘要；这里只识别普通、非空的兼容模型文件。
pub(super) fn installed_size(root: &Path, id: &str) -> Option<u64> {
    if !valid_model_id(id) || !ordinary_directory(root) {
        return None;
    }
    let target = root.join(id);
    if !ordinary_directory(&target) {
        return None;
    }
    let metadata = fs::symlink_metadata(target.join("model.bin")).ok()?;
    (metadata.is_file() && !is_link(&metadata) && metadata.len() > 0).then_some(metadata.len())
}

fn ensure_models_root(root: &Path) -> Result<(), String> {
    match fs::symlink_metadata(root) {
        Ok(metadata) if metadata.is_dir() && !is_link(&metadata) => Ok(()),
        Ok(_) => Err("模型根目录不是普通目录".to_string()),
        Err(error) if error.kind() == ErrorKind::NotFound => {
            fs::create_dir_all(root).map_err(|_| "创建模型根目录失败".to_string())?;
            if ordinary_directory(root) {
                Ok(())
            } else {
                Err("模型根目录不是普通目录".to_string())
            }
        }
        Err(_) => Err("读取模型根目录失败".to_string()),
    }
}

fn check_target(root: &Path, id: &str) -> Result<PathBuf, String> {
    if !valid_model_id(id) || !ordinary_directory(root) {
        return Err("模型路径无效".to_string());
    }
    let target = root.join(id);
    match fs::symlink_metadata(&target) {
        Ok(metadata) if metadata.is_dir() && !is_link(&metadata) => {}
        Ok(_) => return Err("模型目标不是普通目录".to_string()),
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(target),
        Err(_) => return Err("读取模型目标失败".to_string()),
    }
    match fs::symlink_metadata(target.join("model.bin")) {
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(target),
        Ok(_) => Err("模型文件已存在，不能覆盖".to_string()),
        Err(_) => Err("读取模型文件状态失败".to_string()),
    }
}

struct PreparedDownload {
    path: TempPath,
    bytes: u64,
}

fn check_cancelled(context: &TaskContext) -> Result<(), String> {
    if context.is_cancel_requested() {
        Err("模型下载已取消".to_string())
    } else {
        Ok(())
    }
}

async fn prepare_download(
    context: &TaskContext,
    model_id: &str,
    models_dir: &Path,
    registry_url: &str,
) -> Result<PreparedDownload, String> {
    check_cancelled(context)?;
    if !valid_model_id(model_id) {
        return Err("模型 ID 无效".to_string());
    }
    ensure_models_root(models_dir)?;
    check_target(models_dir, model_id)?;
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(30))
        .build()
        .map_err(|_| "构造模型 HTTP 客户端失败".to_string())?;
    // 不使用总下载超时：大文件只要持续收到数据即可继续。
    // 错误不附带 reqwest 的 Display，避免 registry/download URL 凭证进入 UI 或日志。
    let registry_text = client
        .get(registry_url)
        .send()
        .await
        .map_err(|_| "拉取模型注册表失败".to_string())?
        .error_for_status()
        .map_err(|_| "模型注册表返回错误".to_string())?
        .text()
        .await
        .map_err(|_| "读取模型注册表失败".to_string())?;
    check_cancelled(context)?;
    let registry: RegistryFile =
        serde_json::from_str(&registry_text).map_err(|_| "模型注册表格式无效".to_string())?;
    let entry = registry
        .models
        .into_iter()
        .find(|entry| entry.id == model_id)
        .ok_or_else(|| "注册表中未找到模型".to_string())?;
    let mut response = client
        .get(&entry.download_url)
        .send()
        .await
        .map_err(|_| "下载模型失败".to_string())?
        .error_for_status()
        .map_err(|_| "模型下载返回错误".to_string())?;
    check_cancelled(context)?;
    let total = response.content_length();
    let mut temp =
        NamedTempFile::new_in(models_dir).map_err(|_| "创建模型暂存文件失败".to_string())?;
    let mut hasher = Sha256::new();
    let mut bytes = 0u64;
    context.report_progress(0, total);
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "读取模型下载流失败".to_string())?
    {
        // 有界同步文件写入不派生隐蔽任务；每块主动让出执行权以响应 abort。
        for part in chunk.chunks(WRITE_CHUNK_SIZE) {
            check_cancelled(context)?;
            temp.write_all(part)
                .map_err(|_| "写入模型暂存文件失败".to_string())?;
            hasher.update(part);
            bytes = bytes
                .checked_add(part.len() as u64)
                .ok_or_else(|| "模型文件长度超出范围".to_string())?;
            context.report_progress(bytes, total);
            tokio::task::yield_now().await;
        }
    }
    check_cancelled(context)?;
    if bytes == 0 {
        return Err("模型下载内容为空".to_string());
    }
    if total.is_some_and(|total| total != bytes) {
        return Err("模型下载内容不完整".to_string());
    }
    let got = format!("{:x}", hasher.finalize());
    // 保持旧 registry 兼容：空摘要表示未提供校验，非空时按原格式精确比较。
    if !entry.sha256.is_empty() && entry.sha256 != got {
        return Err("模型 sha256 校验失败".to_string());
    }
    temp.flush()
        .map_err(|_| "刷新模型暂存文件失败".to_string())?;
    temp.as_file()
        .sync_all()
        .map_err(|_| "同步模型暂存文件失败".to_string())?;
    check_cancelled(context)?;
    // into_temp_path 关闭 File，Windows 发布与取消清理不再被自身句柄阻挡。
    Ok(PreparedDownload {
        path: temp.into_temp_path(),
        bytes,
    })
}

fn publish_download(
    prepared: PreparedDownload,
    model_id: String,
    models_dir: &Path,
) -> Result<TaskOutput, String> {
    let target = check_target(models_dir, &model_id)?;
    match fs::create_dir(&target) {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
        Err(_) => return Err("创建模型目标目录失败".to_string()),
    }
    // create_dir 与校验间可能已有目标变化；不得跟随符号链接或覆盖任意现存文件。
    check_target(models_dir, &model_id)?;
    let temp_name = prepared.path.to_path_buf();
    prepared
        .path
        .persist_noclobber(target.join("model.bin"))
        .map_err(|_| "发布模型文件失败，目标未被覆盖".to_string())?;
    // 某些 Unix 平台的 no-clobber fallback 可能留下源硬链接。仅清理本次临时项，
    // 此时模型已成功发布，清理失败不能回滚模型或把成功改报为下载失败。
    if let Err(error) = fs::remove_file(temp_name) {
        if error.kind() != ErrorKind::NotFound {
            tracing::warn!("模型已发布，但本次暂存项清理失败");
        }
    }
    Ok(TaskOutput::EmbedModelInstalled {
        model_id,
        bytes: prepared.bytes,
    })
}

pub(super) async fn download_model(
    context: TaskContext,
    model_id: String,
    models_dir: PathBuf,
    registry_url: String,
) -> Result<TaskOutput, TaskFailure> {
    let prepared = prepare_download(&context, &model_id, &models_dir, &registry_url).await;
    // 包括准备失败在内，终态都核对原会话与取消许可；提交后直接返回，不追加 await。
    context.commit(move || publish_download(prepared?, model_id, &models_dir))
}

#[cfg(test)]
mod tests;
