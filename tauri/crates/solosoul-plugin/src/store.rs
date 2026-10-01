//! 插件本地存储。
//!
//! 新安装使用不可变版本目录和原子替换的 current.json；没有指针时兼容旧两文件布局。
//! 暂存目录由准备结果持有，取消/失败自动清理；未引用版本不作为已安装插件。

use super::{PluginError, PluginManifest};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use solosoul_core::import_activity::{begin_owned_root_activity, RootActivityGuard};
use solosoul_vault::root_owner::VaultRootOwner;
use std::fs;
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::SystemTime;
use tempfile::{TempDir, TempPath};

pub(crate) const MAX_WASM_SIZE: usize = 10 * 1024 * 1024;
const MAX_POINTER_SIZE: usize = 4096;
const MAX_MANIFEST_SIZE: usize = 1024 * 1024;
// 所有 Store 实例共同协调配对读取、发布与删除；准备阶段的网络和正文写入不持锁。
static STORE_ACCESS: Mutex<()> = Mutex::new(());

fn store_lock() -> Result<MutexGuard<'static, ()>, PluginError> {
    STORE_ACCESS
        .lock()
        .map_err(|_| PluginError::StoreError("Plugin store lock poisoned".into()))
}

pub(crate) fn validate_plugin_id(id: &str) -> Result<(), PluginError> {
    let stem = id
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'));
    if id.is_empty()
        || id.len() > 64
        || id.ends_with('.')
        || reserved
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err(PluginError::StoreError("Invalid plugin id".into()));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CurrentPointer {
    schema_version: u32,
    generation: String,
    manifest_sha256: String,
    wasm_sha256: String,
    wasm_size: u64,
}

impl CurrentPointer {
    fn validate(&self) -> Result<(), PluginError> {
        let suffix = self.generation.strip_prefix("v-").unwrap_or_default();
        if self.schema_version != 1
            || suffix.len() < 8
            || suffix.len() > 64
            || !suffix.bytes().all(|c| c.is_ascii_alphanumeric())
            || !valid_sha256(&self.manifest_sha256)
            || !valid_sha256(&self.wasm_sha256)
            || self.wasm_size == 0
            || self.wasm_size > MAX_WASM_SIZE as u64
        {
            return Err(PluginError::StoreError(
                "Invalid plugin current pointer".into(),
            ));
        }
        Ok(())
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileStamp {
    len: u64,
    modified: SystemTime,
    created: Option<SystemTime>,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}

impl FileStamp {
    fn read(path: &Path) -> Result<Self, PluginError> {
        let metadata = regular_file(path)?;
        Ok(Self {
            len: metadata.len(),
            modified: metadata.modified()?,
            created: metadata.created().ok(),
            #[cfg(unix)]
            device: std::os::unix::fs::MetadataExt::dev(&metadata),
            #[cfg(unix)]
            inode: std::os::unix::fs::MetadataExt::ino(&metadata),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Selection {
    pointer: Option<CurrentPointer>,
    manifest: FileStamp,
    wasm: FileStamp,
}

#[derive(Debug)]
enum PreparedKind {
    Staged {
        directory: TempDir,
        pointer: TempPath,
        pointer_bytes: Vec<u8>,
        manifest_stamp: FileStamp,
        wasm_stamp: FileStamp,
    },
    Reuse(Selection),
}

/// 准备结果只能由其所属 Store 发布；Drop 不会触碰已经安装的版本。
pub(crate) struct PreparedStoredPlugin {
    store_root: PathBuf,
    manifest: PluginManifest,
    kind: PreparedKind,
    // 按字段声明顺序 Drop：真实 stage/临时指针先清理，再释放根 owner/活动许可。
    _root_activity: Option<Arc<RootActivityGuard>>,
}

impl std::fmt::Debug for PreparedStoredPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedStoredPlugin")
            .field("store_root", &self.store_root)
            .field("manifest", &self.manifest)
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

impl PreparedStoredPlugin {
    pub(crate) fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
}

pub struct PluginStore {
    base_dir: PathBuf,
    // raw 构造器保留兼容；Native/CLI 生产入口显式注入真实 data root owner。
    root_owner: Option<Arc<VaultRootOwner>>,
}

impl PluginStore {
    pub fn new() -> Result<Self, PluginError> {
        Self::new_with_data_dir(Self::data_dir()?)
    }

    pub fn new_with_data_dir(data_dir: PathBuf) -> Result<Self, PluginError> {
        let _guard = store_lock()?;
        let base_dir = data_dir.join("plugins");
        ensure_dir(&base_dir)?;
        Ok(Self {
            base_dir: base_dir.canonicalize()?,
            root_owner: None,
        })
    }

    /// data_dir 必须恰好等于 owner 根；不隐式猜测或复用其它路径的 owner。
    pub fn new_with_data_dir_owned(
        data_dir: PathBuf,
        owner: Arc<VaultRootOwner>,
    ) -> Result<Self, PluginError> {
        if data_dir.canonicalize()? != owner.root() {
            return Err(PluginError::StoreError("VAULT_ROOT_MISMATCH".into()));
        }
        let _activity =
            begin_owned_root_activity(owner.clone()).map_err(PluginError::StoreError)?;
        let mut store = Self::new_with_data_dir(data_dir)?;
        store.root_owner = Some(owner);
        Ok(store)
    }

    pub(crate) fn begin_root_activity(
        &self,
    ) -> Result<Option<Arc<RootActivityGuard>>, PluginError> {
        self.root_owner
            .as_ref()
            .map(|owner| begin_owned_root_activity(owner.clone()).map(Arc::new))
            .transpose()
            .map_err(PluginError::StoreError)
    }

    pub fn data_dir() -> Result<PathBuf, PluginError> {
        #[cfg(any(target_os = "android", target_os = "ios"))]
        {
            let base = std::env::current_dir()
                .map_err(|e| PluginError::StoreError(format!("无法获取当前目录: {}", e)))?;
            Ok(base.join(".solosoul"))
        }
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            let home = dirs::home_dir()
                .ok_or_else(|| PluginError::StoreError("无法获取主目录".to_string()))?;
            Ok(home.join(".solosoul"))
        }
    }

    fn plugin_dir(&self, id: &str) -> Result<PathBuf, PluginError> {
        validate_plugin_id(id)?;
        regular_dir(&self.base_dir)?;
        Ok(self.base_dir.join(id))
    }

    pub(crate) fn prepare_plugin(
        &self,
        manifest: &PluginManifest,
        wasm_bytes: &[u8],
    ) -> Result<PreparedStoredPlugin, PluginError> {
        let root_activity = self.begin_root_activity()?;
        validate_plugin_id(&manifest.id)?;
        validate_wasm(wasm_bytes, manifest.wasm_hash_sha256.as_deref())?;
        let manifest_bytes = serde_json::to_vec_pretty(manifest)?;
        if manifest_bytes.len() > MAX_MANIFEST_SIZE {
            return Err(PluginError::InvalidManifest(
                "Plugin manifest exceeds size limit".into(),
            ));
        }
        let (directory, mut pointer) = {
            let _guard = store_lock()?;
            let plugin_dir = self.plugin_dir(&manifest.id)?;
            ensure_dir(&plugin_dir)?;
            let versions = plugin_dir.join("versions");
            ensure_dir(&versions)?;
            let directory = tempfile::Builder::new()
                .prefix("v-")
                .rand_bytes(16)
                .tempdir_in(&versions)?;
            let pointer = tempfile::Builder::new()
                .prefix(".current-")
                .suffix(".tmp")
                .tempfile_in(&plugin_dir)?;
            (directory, pointer)
        };
        // 正文只写入私有版本目录；完成所有写入并关闭句柄后才产生可提交结果。
        let manifest_path = directory.path().join("manifest.json");
        let wasm_path = directory.path().join("plugin.wasm");
        write_private_file(&manifest_path, &manifest_bytes)?;
        write_private_file(&wasm_path, wasm_bytes)?;
        let current = CurrentPointer {
            schema_version: 1,
            generation: directory
                .path()
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| PluginError::StoreError("Invalid plugin generation".into()))?
                .to_owned(),
            manifest_sha256: compute_sha256(&manifest_bytes),
            wasm_sha256: compute_sha256(wasm_bytes),
            wasm_size: wasm_bytes.len() as u64,
        };
        current.validate()?;
        let pointer_bytes = serde_json::to_vec(&current)?;
        #[cfg(unix)]
        pointer
            .as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))?;
        pointer.write_all(&pointer_bytes)?;
        pointer.flush()?;
        pointer.as_file().sync_all()?;
        let manifest_stamp = FileStamp::read(&manifest_path)?;
        let wasm_stamp = FileStamp::read(&wasm_path)?;
        Ok(PreparedStoredPlugin {
            store_root: self.base_dir.clone(),
            manifest: manifest.clone(),
            kind: PreparedKind::Staged {
                directory,
                pointer: pointer.into_temp_path(),
                pointer_bytes,
                manifest_stamp,
                wasm_stamp,
            },
            _root_activity: root_activity,
        })
    }

    /// 一次读取绑定复用版本；提交时只核对选择器/文件元数据，不在会话门闩内重算大文件摘要。
    pub(crate) fn prepare_reuse(
        &self,
        id: &str,
        version: &str,
        expected_hash: &str,
    ) -> Result<Option<PreparedStoredPlugin>, PluginError> {
        let root_activity = self.begin_root_activity()?;
        let _guard = store_lock()?;
        let (manifest, bytes, selection) = self.load_plugin_locked(id)?;
        if manifest.version != version || compute_sha256(&bytes) != expected_hash {
            return Ok(None);
        }
        Ok(Some(PreparedStoredPlugin {
            store_root: self.base_dir.clone(),
            manifest,
            kind: PreparedKind::Reuse(selection),
            _root_activity: root_activity,
        }))
    }

    pub(crate) fn publish_prepared(
        &self,
        prepared: PreparedStoredPlugin,
    ) -> Result<(), PluginError> {
        let _activity = self.begin_root_activity()?;
        let _guard = store_lock()?;
        if prepared.store_root != self.base_dir {
            return Err(PluginError::StoreError(
                "Prepared plugin belongs to another store".into(),
            ));
        }
        let plugin_dir = self.plugin_dir(&prepared.manifest.id)?;
        regular_dir(&plugin_dir)?;
        match prepared.kind {
            PreparedKind::Reuse(expected) => {
                let (selection, _) = self.select_locked(&prepared.manifest.id)?;
                if selection != expected {
                    return Err(PluginError::StoreError(
                        "Prepared plugin is no longer current".into(),
                    ));
                }
            }
            PreparedKind::Staged {
                directory,
                pointer,
                pointer_bytes,
                manifest_stamp,
                wasm_stamp,
            } => {
                let versions = plugin_dir.join("versions");
                regular_dir(&versions)?;
                regular_dir(directory.path())?;
                if directory.path().parent() != Some(versions.as_path())
                    || pointer.parent() != Some(plugin_dir.as_path())
                    || FileStamp::read(&directory.path().join("manifest.json"))? != manifest_stamp
                    || FileStamp::read(&directory.path().join("plugin.wasm"))? != wasm_stamp
                    || read_limited(&pointer, MAX_POINTER_SIZE)? != pointer_bytes
                {
                    return Err(PluginError::StoreError(
                        "Prepared plugin changed before publication".into(),
                    ));
                }
                let target = plugin_dir.join("current.json");
                if metadata_if_present(&target)?.is_some() {
                    regular_file(&target)?;
                }
                // 文件句柄已关闭。Windows 替换失败保留旧指针，禁止先删除旧目标的回退。
                pointer.persist(&target).map_err(|error| {
                    PluginError::StoreError(format!("Plugin publication failed: {}", error.error))
                })?;
                // 唯一发布点之后无可失败业务步骤；保留完整新代及所有未删除的旧代。
                let _ = directory.keep();
            }
        }
        Ok(())
    }

    pub fn save_plugin(&self, manifest: &PluginManifest, bytes: &[u8]) -> Result<(), PluginError> {
        self.publish_prepared(self.prepare_plugin(manifest, bytes)?)
    }

    /// 在一次共享锁内读取同一选择器的 manifest 和 WASM，避免运行时权限/字节混代。
    pub fn load_plugin(&self, id: &str) -> Result<(PluginManifest, Vec<u8>), PluginError> {
        let _guard = store_lock()?;
        let (manifest, bytes, _) = self.load_plugin_locked(id)?;
        Ok((manifest, bytes))
    }

    pub fn load_manifest(&self, id: &str) -> Result<PluginManifest, PluginError> {
        let _guard = store_lock()?;
        self.load_manifest_locked(id)
            .map(|(manifest, _, _)| manifest)
    }

    pub fn load_wasm(&self, id: &str) -> Result<Vec<u8>, PluginError> {
        self.load_plugin(id).map(|(_, bytes)| bytes)
    }

    fn select_locked(&self, id: &str) -> Result<(Selection, PathBuf), PluginError> {
        let plugin_dir = self.plugin_dir(id)?;
        if metadata_if_present(&plugin_dir)?.is_none() {
            return Err(PluginError::NotFound(id.to_owned()));
        }
        regular_dir(&plugin_dir)?;
        let current = plugin_dir.join("current.json");
        let (pointer, directory) = if metadata_if_present(&current)?.is_some() {
            // 指针存在但损坏时拒绝读取，不退回旧布局或猜测最新版本目录。
            let pointer: CurrentPointer =
                serde_json::from_slice(&read_limited(&current, MAX_POINTER_SIZE)?)?;
            pointer.validate()?;
            let versions = plugin_dir.join("versions");
            regular_dir(&versions)?;
            let directory = versions.join(&pointer.generation);
            regular_dir(&directory)?;
            (Some(pointer), directory)
        } else {
            (None, plugin_dir)
        };
        Ok((
            Selection {
                pointer,
                manifest: FileStamp::read(&directory.join("manifest.json"))?,
                wasm: FileStamp::read(&directory.join("plugin.wasm"))?,
            },
            directory,
        ))
    }

    // 市场/已安装列表仅检查 manifest 和包结构，不在输入线程读/哈希每个大 WASM。
    fn load_manifest_locked(
        &self,
        id: &str,
    ) -> Result<(PluginManifest, Selection, PathBuf), PluginError> {
        let (selection, directory) = self.select_locked(id)?;
        if selection.wasm.len > MAX_WASM_SIZE as u64 {
            return Err(PluginError::WasmTooLarge(
                usize::try_from(selection.wasm.len).unwrap_or(usize::MAX),
            ));
        }
        if selection.wasm.len == 0 {
            return Err(PluginError::InvalidManifest("Empty plugin WASM".into()));
        }
        let manifest_bytes = read_limited(&directory.join("manifest.json"), MAX_MANIFEST_SIZE)?;
        let manifest: PluginManifest = serde_json::from_slice(&manifest_bytes)?;
        if manifest.id != id {
            return Err(PluginError::InvalidManifest(
                "Plugin manifest id does not match directory".into(),
            ));
        }
        if let Some(pointer) = &selection.pointer {
            if compute_sha256(&manifest_bytes) != pointer.manifest_sha256
                || selection.wasm.len != pointer.wasm_size
            {
                return Err(PluginError::ChecksumMismatch);
            }
        }
        if self.select_locked(id)?.0 != selection {
            return Err(PluginError::StoreError("Plugin changed during read".into()));
        }
        Ok((manifest, selection, directory))
    }

    fn load_plugin_locked(
        &self,
        id: &str,
    ) -> Result<(PluginManifest, Vec<u8>, Selection), PluginError> {
        let (manifest, selection, directory) = self.load_manifest_locked(id)?;
        let bytes = read_limited(&directory.join("plugin.wasm"), MAX_WASM_SIZE)?;
        validate_wasm(&bytes, manifest.wasm_hash_sha256.as_deref())?;
        if let Some(pointer) = &selection.pointer {
            if compute_sha256(&bytes) != pointer.wasm_sha256 {
                return Err(PluginError::ChecksumMismatch);
            }
        }
        if self.select_locked(id)?.0 != selection {
            return Err(PluginError::StoreError("Plugin changed during read".into()));
        }
        Ok((manifest, bytes, selection))
    }
    pub fn delete_plugin(&self, id: &str) -> Result<(), PluginError> {
        let _activity = self.begin_root_activity()?;
        let _guard = store_lock()?;
        let directory = self.plugin_dir(id)?;
        if metadata_if_present(&directory)?.is_some() {
            regular_dir(&directory)?;
            fs::remove_dir_all(directory)?;
        }
        Ok(())
    }

    pub fn installed_manifests(&self) -> Result<Vec<PluginManifest>, PluginError> {
        let _guard = store_lock()?;
        let mut manifests = Vec::new();
        if metadata_if_present(&self.base_dir)?.is_none() {
            return Ok(manifests);
        }
        regular_dir(&self.base_dir)?;
        for entry in fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            let id = entry.file_name().to_string_lossy().to_string();
            if let Ok((mut manifest, _, _)) = self.load_manifest_locked(&id) {
                let field_bindings = manifest.field_bindings.clone();
                for contract in &mut manifest.contracts {
                    contract.roles = contract.effective_roles(&field_bindings);
                }
                manifests.push(manifest);
            }
        }
        Ok(manifests)
    }
}

fn metadata_if_present(path: &Path) -> Result<Option<fs::Metadata>, PluginError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn is_link(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_type().is_symlink() || metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

fn regular_dir(path: &Path) -> Result<(), PluginError> {
    let metadata = fs::symlink_metadata(path)?;
    if is_link(&metadata) || !metadata.is_dir() {
        return Err(PluginError::StoreError(
            "Plugin directory is not a regular directory".into(),
        ));
    }
    Ok(())
}

fn regular_file(path: &Path) -> Result<fs::Metadata, PluginError> {
    let metadata = fs::symlink_metadata(path)?;
    if is_link(&metadata) || !metadata.is_file() {
        return Err(PluginError::StoreError(
            "Plugin file is not a regular file".into(),
        ));
    }
    Ok(metadata)
}

fn ensure_dir(path: &Path) -> Result<(), PluginError> {
    if metadata_if_present(path)?.is_none() {
        fs::create_dir_all(path)?;
        #[cfg(unix)]
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    regular_dir(path)
}

fn write_private_file(path: &Path, bytes: &[u8]) -> Result<(), PluginError> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    #[cfg(unix)]
    file.set_permissions(fs::Permissions::from_mode(0o600))?;
    file.write_all(bytes)?;
    file.flush()?;
    file.sync_all()?;
    Ok(())
}

fn read_limited(path: &Path, limit: usize) -> Result<Vec<u8>, PluginError> {
    let metadata = regular_file(path)?;
    if metadata.len() > limit as u64 {
        return Err(PluginError::StoreError(
            "Plugin file exceeds size limit".into(),
        ));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(PluginError::StoreError(
            "Plugin file exceeds size limit".into(),
        ));
    }
    Ok(bytes)
}

fn validate_wasm(bytes: &[u8], expected_hash: Option<&str>) -> Result<(), PluginError> {
    if bytes.len() > MAX_WASM_SIZE {
        return Err(PluginError::WasmTooLarge(bytes.len()));
    }
    if bytes.is_empty() {
        return Err(PluginError::InvalidManifest("Empty plugin WASM".into()));
    }
    if expected_hash.is_some_and(|hash| compute_sha256(bytes) != hash) {
        return Err(PluginError::ChecksumMismatch);
    }
    Ok(())
}

pub fn compute_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(id: &str) -> PluginManifest {
        serde_json::from_value(serde_json::json!({
            "id": id, "name": "Test", "version": "1.0.0", "description": "Test"
        }))
        .unwrap()
    }

    #[test]
    fn invalid_ids_cannot_read_write_or_delete_outside_plugin() {
        let root = tempfile::tempdir().unwrap();
        let store = PluginStore::new_with_data_dir(root.path().to_owned()).unwrap();
        let sentinel = root.path().join("account-sentinel");
        fs::write(&sentinel, b"account data").unwrap();
        store
            .save_plugin(&manifest("safe.plugin-1"), b"wasm")
            .unwrap();
        for id in [
            "",
            ".",
            "..",
            "...",
            "safe.plugin-1.",
            "../account-sentinel",
            "./safe.plugin-1",
            "a/b",
            "a\\b",
            "/tmp",
            "C:\\tmp",
        ] {
            assert!(
                store.save_plugin(&manifest(id), b"bad").is_err(),
                "save {id:?}"
            );
            assert!(store.load_manifest(id).is_err(), "manifest {id:?}");
            assert!(store.load_wasm(id).is_err(), "wasm {id:?}");
            assert!(store.delete_plugin(id).is_err(), "delete {id:?}");
            assert_eq!(fs::read(&sentinel).unwrap(), b"account data");
            assert_eq!(store.load_wasm("safe.plugin-1").unwrap(), b"wasm");
            assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
            assert_eq!(store.installed_manifests().unwrap().len(), 1);
        }
        store.delete_plugin("safe.plugin-1").unwrap();
        assert!(store.installed_manifests().unwrap().is_empty());
        assert_eq!(fs::read(sentinel).unwrap(), b"account data");
    }

    #[test]
    fn test_compute_sha256_known_value() {
        let hash = compute_sha256(b"hello");
        assert_eq!(
            hash,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }
}

#[cfg(test)]
#[path = "store/rf214_tests.rs"]
mod rf214_tests;
