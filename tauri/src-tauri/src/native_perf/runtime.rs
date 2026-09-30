//! 仅显式日志诊断允许选择 owned 副本；不修改默认 Evergreen。
use super::{
    checked_dir, read_json, require_regular, sha256_file, write_new_json, RuntimeConfig,
    CHROMIUM_LOG_MARKER,
};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

pub(super) const MANIFEST_FILE: &str = "native-perf-runtime.json";
pub(super) const SELECTED_FILE: &str = "native-perf-selected-runtime.json";
pub(super) const MAX_MANIFEST_BYTES: u64 = 4 * 1024 * 1024;
pub(super) const MAX_FILES: usize = 3000;
pub(super) const MAX_DIRECTORIES: usize = 3000;
const MAX_DEPTH: usize = 64;
pub(super) const MAX_TOTAL_BYTES: u64 = 1024 * 1024 * 1024;
pub(super) const CORE_FILES: &[&str] = &[
    "msedgewebview2.exe",
    "msedge.dll",
    "EBWebView/x64/EmbeddedBrowserWebView.dll",
];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    scope: String,
    root: PathBuf,
    run_id: String,
    source_kind: String,
    source_folder: PathBuf,
    expected_version: String,
    runtime_folder: PathBuf,
    files: Vec<FileProof>,
    directories: Vec<String>,
    core_files: Vec<CoreProof>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileProof {
    relative: String,
    bytes: u64,
    sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CoreProof {
    relative: String,
    sha256: String,
    architecture: String,
    file_version: String,
    product_version: String,
    signature_status: String,
    signer_subject: String,
    signer_thumbprint: String,
}

#[derive(Debug)]
pub(super) struct Selection {
    pub(super) browser_executable_folder: PathBuf,
    runtime_folder: PathBuf,
    source_folder: PathBuf,
    expected_version: String,
    manifest_sha256: String,
    executable_sha256: String,
}

fn is_lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_version(value: &str) -> bool {
    let parts: Vec<_> = value.split('.').collect();
    parts.len() == 4
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && part
                    .parse::<u16>()
                    .is_ok_and(|number| number.to_string() == *part)
        })
}

fn valid_component(value: &str) -> bool {
    if value.is_empty()
        || value == "."
        || value == ".."
        || value.ends_with([' ', '.'])
        || value
            .chars()
            .any(|character| character.is_control() || "<>:\"/\\|?*".contains(character))
    {
        return false;
    }
    let stem = value
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    if matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) {
        return false;
    }
    // Windows 同样将这三个上标数字视为 DOS device 数字。
    let device_digits = ['1', '2', '3', '4', '5', '6', '7', '8', '9', '¹', '²', '³'];
    !["COM", "LPT"].iter().any(|prefix| {
        stem.strip_prefix(prefix).is_some_and(|suffix| {
            suffix.chars().count() == 1 && device_digits.contains(&suffix.chars().next().unwrap())
        })
    })
}

fn valid_relative(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 1024
        && value.split('/').count() <= MAX_DEPTH
        && value.split('/').all(valid_component)
}

fn ordinary_local_path(value: &Path) -> bool {
    let Some(text) = value.to_str() else {
        return false;
    };
    let bytes = text.as_bytes();
    bytes.len() > 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && bytes[2] == b'\\'
        && text[3..].split('\\').all(valid_component)
        && value.is_absolute()
        && matches!(value.components().next(), Some(std::path::Component::Prefix(prefix)) if matches!(prefix.kind(), std::path::Prefix::Disk(_)))
}

fn ordinary_equivalent(value: &Path) -> Result<PathBuf, String> {
    let text = value
        .to_str()
        .ok_or("selected Runtime folder requires Unicode")?;
    let ordinary = PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(text));
    if !ordinary_local_path(&ordinary) || checked_dir(&ordinary)?.as_os_str() != value.as_os_str() {
        return Err("selected Runtime requires an equivalent ordinary local drive path".into());
    }
    Ok(ordinary)
}

fn read_manifest(path: &Path) -> Result<(Manifest, String), String> {
    require_regular(path, false)?;
    let file = File::open(path).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() > MAX_MANIFEST_BYTES {
        return Err("copied Runtime manifest exceeds 4 MiB".into());
    }
    let mut contents = Vec::new();
    file.take(MAX_MANIFEST_BYTES + 1)
        .read_to_end(&mut contents)
        .map_err(|e| e.to_string())?;
    if contents.len() as u64 > MAX_MANIFEST_BYTES {
        return Err("copied Runtime manifest exceeds 4 MiB".into());
    }
    let manifest = serde_json::from_slice(&contents)
        .map_err(|e| format!("invalid copied Runtime manifest: {e}"))?;
    Ok((manifest, hex::encode(Sha256::digest(&contents))))
}

fn verify_tree(
    directory: &Path,
    prefix: &str,
    expected: &BTreeMap<String, &FileProof>,
    directories: &BTreeSet<String>,
    seen: &mut BTreeSet<String>,
    seen_directories: &mut BTreeSet<String>,
) -> Result<(), String> {
    for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "copied Runtime entry requires Unicode")?;
        if !valid_component(&name) {
            return Err("copied Runtime contains an unsupported entry name".into());
        }
        let relative = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        require_regular(&path, metadata.is_dir())?;
        if path.canonicalize().map_err(|e| e.to_string())?.as_os_str() != path.as_os_str() {
            return Err("copied Runtime entry resolved outside its exact owned path".into());
        }
        if metadata.is_dir() {
            if !directories.contains(&relative) {
                return Err("copied Runtime contains an extra directory".into());
            }
            seen_directories.insert(relative.clone());
            verify_tree(
                &path,
                &relative,
                expected,
                directories,
                seen,
                seen_directories,
            )?;
        } else {
            let proof = expected
                .get(&relative)
                .ok_or("copied Runtime contains an extra file")?;
            if metadata.len() != proof.bytes || sha256_file(&path)? != proof.sha256 {
                return Err(format!(
                    "copied Runtime file size or SHA mismatch: {relative}"
                ));
            }
            seen.insert(relative);
        }
    }
    Ok(())
}

fn require_amd64(path: &Path) -> Result<(), String> {
    require_regular(path, false)?;
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let size = file.metadata().map_err(|e| e.to_string())?.len();
    let mut header = [0_u8; 64];
    file.read_exact(&mut header)
        .map_err(|_| "copied Runtime core is not a PE image")?;
    let offset = u32::from_le_bytes(header[60..64].try_into().unwrap()) as u64;
    if &header[..2] != b"MZ" || offset < 64 || offset.checked_add(6).is_none_or(|end| end > size) {
        return Err("copied Runtime core is not a PE image".into());
    }
    file.seek(SeekFrom::Start(offset))
        .map_err(|e| e.to_string())?;
    let mut signature = [0_u8; 6];
    file.read_exact(&mut signature)
        .map_err(|_| "copied Runtime core is not a PE image")?;
    if &signature[..4] != b"PE\0\0" || u16::from_le_bytes([signature[4], signature[5]]) != 0x8664 {
        return Err("copied Runtime core must be an AMD64 PE image".into());
    }
    Ok(())
}

/// 仅读 owned 副本和完成清单；不读取/执行原始安装目录，不更改环境。
pub(super) fn validate(config: &RuntimeConfig, requested: &Path) -> Result<Selection, String> {
    if config.chromium_log.is_none() {
        return Err(
            "copied Runtime selection is diagnostic only and requires Chromium logging".into(),
        );
    }
    let runtime_folder = config.root.join("runtime");
    if !requested.is_absolute()
        || checked_dir(requested)?.as_os_str() != runtime_folder.as_os_str()
        || checked_dir(&runtime_folder)?.as_os_str() != runtime_folder.as_os_str()
    {
        return Err("--native-perf-runtime requires the exact canonical owned root/runtime".into());
    }
    if fs::symlink_metadata(config.root.join(SELECTED_FILE)).is_ok() {
        return Err(
            "selected Runtime marker already exists; diagnostic runs cannot be reused".into(),
        );
    }
    let (manifest, manifest_sha256) = read_manifest(&config.root.join(MANIFEST_FILE))?;
    if manifest.schema_version != 1
        || manifest.scope != "windows-native-perf-copied-runtime"
        || manifest.root.as_os_str() != config.root.as_os_str()
        || manifest.run_id != config.run_id
        || manifest.source_kind != "copied-local-evergreen"
        || !ordinary_local_path(&manifest.source_folder)
        || !is_version(&manifest.expected_version)
        || manifest.runtime_folder.as_os_str() != runtime_folder.as_os_str()
        || manifest.files.is_empty()
        || manifest.files.len() > MAX_FILES
        || manifest.directories.len() > MAX_DIRECTORIES
        || manifest.core_files.len() != CORE_FILES.len()
    {
        return Err(
            "copied Runtime manifest does not describe this exact owned diagnostic run".into(),
        );
    }
    let mut expected = BTreeMap::new();
    let mut folded_names = BTreeSet::new();
    let mut directories = BTreeSet::new();
    let mut previous_directory: Option<&str> = None;
    for relative in &manifest.directories {
        if !valid_relative(relative)
            || previous_directory.is_some_and(|previous| {
                previous.encode_utf16().cmp(relative.encode_utf16()) != std::cmp::Ordering::Less
            })
            || !folded_names.insert(relative.to_lowercase())
            || !directories.insert(relative.clone())
        {
            return Err(
                "copied Runtime manifest has invalid, unsorted or duplicate directory proofs"
                    .into(),
            );
        }
        previous_directory = Some(relative);
    }
    let mut total_bytes = 0_u64;
    let mut previous_file: Option<&str> = None;
    for proof in &manifest.files {
        if !valid_relative(&proof.relative)
            || previous_file.is_some_and(|previous| {
                previous.encode_utf16().cmp(proof.relative.encode_utf16())
                    != std::cmp::Ordering::Less
            })
            || !is_lower_sha256(&proof.sha256)
            || !folded_names.insert(proof.relative.to_lowercase())
            || expected.insert(proof.relative.clone(), proof).is_some()
        {
            return Err("copied Runtime manifest has an invalid or duplicate file proof".into());
        }
        previous_file = Some(&proof.relative);
        total_bytes = total_bytes
            .checked_add(proof.bytes)
            .ok_or("copied Runtime file bytes overflow")?;
        if total_bytes > MAX_TOTAL_BYTES {
            return Err("copied Runtime files exceed 1 GiB".into());
        }
        let mut parent = proof.relative.as_str();
        while let Some((ancestor, _)) = parent.rsplit_once('/') {
            if !directories.contains(ancestor) {
                return Err("copied Runtime manifest is missing a parent directory proof".into());
            }
            parent = ancestor;
        }
    }
    for relative in &manifest.directories {
        let mut parent = relative.as_str();
        while let Some((ancestor, _)) = parent.rsplit_once('/') {
            if !directories.contains(ancestor) {
                return Err("copied Runtime manifest is missing a parent directory proof".into());
            }
            parent = ancestor;
        }
    }
    let mut core_names = BTreeSet::new();
    for core in &manifest.core_files {
        if !CORE_FILES.contains(&core.relative.as_str())
            || !core_names.insert(core.relative.as_str())
            || !is_lower_sha256(&core.sha256)
            || expected
                .get(&core.relative)
                .is_none_or(|proof| proof.sha256 != core.sha256)
            || core.architecture != "AMD64"
            || core.file_version != manifest.expected_version
            || core.product_version != manifest.expected_version
            || core.signature_status != "Valid"
            || core.signer_subject.len() > 1024
            || !core.signer_subject.contains("Microsoft Corporation")
            || core.signer_thumbprint.len() != 40
            || !core
                .signer_thumbprint
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(
                "copied Runtime core metadata does not match its file/version contract".into(),
            );
        }
    }
    let mut seen = BTreeSet::new();
    let mut seen_directories = BTreeSet::new();
    verify_tree(
        &runtime_folder,
        "",
        &expected,
        &directories,
        &mut seen,
        &mut seen_directories,
    )?;
    if seen.len() != expected.len() || seen_directories != directories {
        return Err("copied Runtime is missing a declared file or directory".into());
    }
    for relative in CORE_FILES {
        require_amd64(&runtime_folder.join(relative))?;
    }
    // core metadata 的签名/文件版本来自 Node 的来源认证声明；
    // Rust 独立核对字节、PE 架构和后续 SDK 可用版本，不冒充 Authenticode 重验。
    let executable_sha256 = expected.get("msedgewebview2.exe").unwrap().sha256.clone();
    Ok(Selection {
        browser_executable_folder: ordinary_equivalent(&runtime_folder)?,
        runtime_folder,
        source_folder: manifest.source_folder,
        expected_version: manifest.expected_version,
        manifest_sha256,
        executable_sha256,
    })
}

/// 显式非空 folder，不查询默认安装。此同步 getter 不创建 COM 环境或线程。
pub(super) fn available_version(folder: &Path) -> Result<String, String> {
    use std::os::windows::ffi::OsStrExt;
    use webview2_com::Microsoft::Web::WebView2::Win32::GetAvailableCoreWebView2BrowserVersionString;
    use windows::core::{PCWSTR, PWSTR};
    if !ordinary_local_path(folder) {
        return Err("SDK Runtime query requires the explicit ordinary owned folder".into());
    }
    let wide: Vec<_> = folder
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut version = PWSTR::null();
    let result = unsafe {
        GetAvailableCoreWebView2BrowserVersionString(PCWSTR(wide.as_ptr()), &mut version)
    };
    // 成功或失败均接管返回指针；take_pwstr 使用 CoTaskMemPWSTR RAII 释放，
    // 包括 null 输出、HRESULT 失败和后续版本契约失败，不泄漏 SDK 分配。
    let available = webview2_com::take_pwstr(version);
    result.map_err(|error| {
        format!(
            "explicit copied Runtime SDK query failed: HRESULT 0x{:08X}",
            error.code().0 as u32
        )
    })?;
    Ok(available)
}

impl Selection {
    /// 只记录当前主进程设置后读回的目录，不证明 browser 已加载该 Runtime。
    pub(super) fn record_actual(
        &self,
        config: &RuntimeConfig,
        actual_folder: &OsStr,
        available: &str,
    ) -> Result<(), String> {
        let log = read_json(&config.root.join(CHROMIUM_LOG_MARKER))?;
        if config.chromium_log.is_none()
            || log["schemaVersion"] != 1
            || log["scope"] != "windows-native-perf-chromium-log"
            || log["root"] != json!(config.root)
            || log["runId"] != config.run_id
            || log["pid"] != std::process::id()
            || log["port"] != config.port
            || log["performanceSample"] != false
            || actual_folder != self.browser_executable_folder.as_os_str()
            || !is_version(available)
            || available != self.expected_version
        {
            return Err("selected Runtime environment or SDK version differs from this owned diagnostic contract".into());
        }
        write_new_json(
            &config.root.join(SELECTED_FILE),
            &json!({
                "schemaVersion": 1, "scope": "windows-native-perf-selected-runtime",
                "mode": "copied-local-evergreen", "performanceSample": false,
                "root": config.root, "runId": config.run_id, "pid": std::process::id(),
                "port": config.port, "runtimeFolder": self.runtime_folder,
                "browserExecutableFolder": Path::new(actual_folder),
                "expectedVersion": self.expected_version, "availableVersion": available,
                "sourceFolder": self.source_folder, "manifestSha256": self.manifest_sha256,
                "executableSha256": self.executable_sha256,
            }),
        )
    }
}
