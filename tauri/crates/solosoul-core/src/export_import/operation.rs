//! RF-022：GUI / CLI 共用的持久导入附件计划与执行器。
//! 包复制、哈希、ZIP、KDF 和 AEAD 均在会话/SQLite 门闩之外；每次发布重新核验 epoch。

use super::*;
use sha2::{Digest, Sha256};
pub use solosoul_vault::ImportSourceKind;
use solosoul_vault::{
    ImportAttachmentOwnerPlan, ImportAttachmentPhase, ImportAttachmentStep,
    ImportAttachmentStepPlan, ImportCiphertextProof, ImportOperationPhase, ImportOperationRecord,
    ImportOperationStart, ImportOwnedAttachmentMarker, ImportSourceProof, Profile,
};
use std::io::{BufReader, Seek, SeekFrom};

pub const IMPORT_OWNER_MARKER: &str = ".solosoul-import-owner.json";
const OPERATION_DIR: &str = ".import-operations";
const FORMAT_VERSION: u32 = 1;

/// 宿主先批准原路径，本类型再复制实际读取的密文；proof 和之后的解析来自同一副本。
/// 仅含密文，临时副本随持有者释放；密码和解包 key 从不写入此文件。
pub struct OwnedImportPackage {
    file: tempfile::NamedTempFile,
    proof: ImportSourceProof,
}

impl OwnedImportPackage {
    pub fn capture(path: &Path, native_root: &Path) -> Result<Self, ExportError> {
        let mut input = BufReader::new(File::open(path)?);
        let mut file = tempfile::NamedTempFile::new_in(native_root)?;
        let mut digest = Sha256::new();
        let mut length = 0u64;
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let read = input.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            digest.update(&buffer[..read]);
            file.write_all(&buffer[..read])?;
            length = length
                .checked_add(read as u64)
                .ok_or("import_source_too_large")?;
        }
        file.flush()?;
        file.as_file().sync_all()?;
        Ok(Self {
            file,
            proof: ImportSourceProof {
                sha256: hex::encode(digest.finalize()),
                length,
            },
        })
    }

    pub fn source_proof(&self) -> &ImportSourceProof {
        &self.proof
    }
    pub fn path(&self) -> &Path {
        self.file.path()
    }
    pub fn open_file(&self) -> Result<File, ExportError> {
        let mut file = self.file.as_file().try_clone()?;
        file.seek(SeekFrom::Start(0))?;
        Ok(file)
    }
    pub fn read_manifest_value(&self) -> Result<serde_json::Value, ExportError> {
        let mut archive = ZipArchive::new(self.open_file()?)?;
        let mut entry = archive
            .by_name("manifest.json")
            .map_err(|_| "缺少 manifest.json")?;
        if entry.size() > MAX_ZIP_ENTRY_SIZE {
            return Err("import_manifest_too_large".into());
        }
        let mut bytes = Vec::new();
        entry
            .by_ref()
            .take(MAX_ZIP_ENTRY_SIZE + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_ZIP_ENTRY_SIZE {
            return Err("import_manifest_too_large".into());
        }
        serde_json::from_str(&String::from_utf8_lossy(&bytes)).map_err(ExportError::from)
    }

    /// 宿主需要其原有对象/历史准备，故只公开 payload；不将 GUI 策略搬入 Core。
    pub fn decrypt(
        &self,
        password: &str,
        private_root: &Path,
    ) -> Result<OpenedImportPackage, ExportError> {
        if password.is_empty() {
            return Err(ExportError::Msg("导入密码不能为空".into()));
        }
        let value = self.read_manifest_value()?;
        let extra_files: Vec<String> = value
            .get("extra_files")
            .and_then(|item| item.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let manifest = ManifestData {
            salt_hex: value["salt_hex"].as_str().ok_or("缺少 salt_hex")?.into(),
            has_attachments: value["has_attachments"].as_bool().unwrap_or(false),
            version: value["version"].as_str().unwrap_or("1.0").into(),
            object_count: value["object_count"].as_u64().unwrap_or(0) as usize,
            password_hint: value
                .get("password_hint")
                .and_then(|item| item.as_str())
                .filter(|text| !text.is_empty())
                .map(String::from),
            kdf: kdf_from_manifest_value(value.get("kdf"))?,
            extra_files,
        };
        let salt = hex::decode(&manifest.salt_hex).map_err(|_| "import_invalid_salt")?;
        let key = derive_export_key_cfg(password, &salt, &manifest.kdf_config())?;
        let directory = tempfile::Builder::new()
            .prefix("solosoul-import-tmp-")
            .tempdir_in(private_root)?;
        let mut temporary = tempfile::NamedTempFile::new_in(directory.path())?;
        let mut archive = ZipArchive::new(self.open_file()?)?;
        let mut entry = archive
            .by_name("payload.enc")
            .map_err(|_| "ZIP 中缺少: payload.enc")?;
        if entry.size() > MAX_ZIP_ENTRY_SIZE {
            return Err("import_payload_too_large".into());
        }
        let mut limited = CountedReader {
            reader: entry.by_ref().take(MAX_ZIP_ENTRY_SIZE + 1),
            count: 0,
        };
        solosoul_crypto::cipher::decrypt_chunked_stream(&key, &mut limited, &mut temporary)
            .map_err(|_| ExportError::DecryptionFailed)?;
        std::io::copy(&mut limited, &mut std::io::sink())?;
        if limited.count > MAX_ZIP_ENTRY_SIZE {
            return Err("import_payload_too_large".into());
        }
        temporary.flush()?;
        let payload = serde_json::from_reader(temporary.reopen()?)?;
        Ok(OpenedImportPackage {
            payload,
            salt,
            key,
            has_attachments: manifest.has_attachments,
            has_preferences: manifest
                .extra_files
                .iter()
                .any(|entry| entry == "preferences.enc"),
        })
    }
}

struct CountedReader<R> {
    reader: R,
    count: u64,
}
impl<R: Read> Read for CountedReader<R> {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        let read = self.reader.read(bytes)?;
        self.count += read as u64;
        Ok(read)
    }
}

/// 派生密钥只随本次 worker 的内存持有者存活；不可序列化。
pub struct OpenedImportPackage {
    pub payload: serde_json::Value,
    salt: Vec<u8>,
    key: Zeroizing<[u8; 32]>,
    pub has_attachments: bool,
    pub has_preferences: bool,
}

impl OpenedImportPackage {
    pub fn salt(&self) -> &[u8] {
        &self.salt
    }
    pub fn key(&self) -> &[u8; 32] {
        &self.key
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "camelCase")]
pub enum ImportPreferencesPlan {
    None,
    Package,
    Ready(Vec<u8>),
    Error(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeImportPlan {
    pub format_version: u32,
    pub source_kind: ImportSourceKind,
    pub source_proof: ImportSourceProof,
    pub root_binding: String,
    pub native_root: String,
    pub request_options: serde_json::Value,
    pub request_fingerprint: String,
    pub preferences: ImportPreferencesPlan,
    pub zip_entry_count: usize,
}

/// root 来自 Native VaultService，不能由 IPC 提供；账户 DB root 与全局附件 root 均被冻结。
pub fn import_root_binding(native_root: &Path, vault: &VaultStore) -> Result<String, ExportError> {
    let root = std::fs::canonicalize(native_root)?;
    let account_root = std::fs::canonicalize(vault.base_path())?;
    if !account_root.starts_with(&root) {
        return Err("import_root_binding_mismatch".into());
    }
    vault.import_root_binding().map_err(ExportError::from)
}

fn canonical_value(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            let mut sorted = serde_json::Map::new();
            for key in keys {
                sorted.insert(key.clone(), canonical_value(&map[key]));
            }
            serde_json::Value::Object(sorted)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(canonical_value).collect())
        }
        _ => value.clone(),
    }
}

pub fn import_request_fingerprint(options: &serde_json::Value) -> Result<String, ExportError> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(
        &canonical_value(options),
    )?)))
}

/// 每个 ordinal 都产生单独 UUID；重复 ZIP 名称保留原顺序，metadata 取 payload 最后一条。
/// 同 operation 重试使用持久 plan；新的 Fresh 同包保留独立 UUID，不做跨任务 dedup。
#[allow(clippy::too_many_arguments)]
pub fn prepare_import_operation(
    operation_id: &str,
    source_kind: ImportSourceKind,
    request_options: serde_json::Value,
    owned: &OwnedImportPackage,
    payload: &serde_json::Value,
    imported_source_ids: &HashSet<String>,
    id_map: &HashMap<String, String>,
    selected_attachment_ids: Option<&HashSet<String>>,
    now: &str,
    native_root: &Path,
    vault: &VaultStore,
    view: &ImportReadView,
    batch: &mut ImportDatabaseBatch,
    include_attachments: bool,
    include_preferences: bool,
) -> Result<ImportOperationStart, ExportError> {
    uuid::Uuid::parse_str(operation_id).map_err(|_| "import_invalid_operation_id")?;
    let root_binding = import_root_binding(native_root, vault)?;
    let request_options = canonical_value(&request_options);
    let request_fingerprint = import_request_fingerprint(&request_options)?;
    let mut plan = NativeImportPlan {
        format_version: FORMAT_VERSION,
        source_kind,
        source_proof: owned.proof.clone(),
        root_binding: root_binding.clone(),
        native_root: std::fs::canonicalize(native_root)?
            .to_string_lossy()
            .into_owned(),
        request_options,
        request_fingerprint: request_fingerprint.clone(),
        preferences: if include_preferences {
            ImportPreferencesPlan::Package
        } else {
            ImportPreferencesPlan::None
        },
        zip_entry_count: 0,
    };
    let meta_map = build_attachment_meta_map(payload);
    let mut archive = ZipArchive::new(owned.open_file()?)?;
    plan.zip_entry_count = archive.len();
    let mut steps = Vec::new();
    if include_attachments {
        for ordinal in 0..archive.len() {
            let entry = archive.by_index(ordinal)?;
            let name = entry.name();
            let Some(relative) = name.strip_prefix("attachments/") else {
                continue;
            };
            let Some((source_object, encrypted_name)) = relative.split_once('/') else {
                continue;
            };
            let Some(source_attachment) = encrypted_name.strip_suffix(".enc") else {
                continue;
            };
            if source_attachment.contains('/')
                || validate_import_id(source_object).is_err()
                || validate_import_id(source_attachment).is_err()
                || !imported_source_ids.contains(source_object)
                || selected_attachment_ids.is_some_and(|ids| !ids.contains(source_attachment))
            {
                continue;
            }
            let Some(metadata) =
                meta_map.get(&(source_object.to_owned(), source_attachment.to_owned()))
            else {
                continue;
            };
            let target_object = id_map
                .get(source_object)
                .map(String::as_str)
                .unwrap_or(source_object);
            validate_import_id(target_object)?;
            let attachment_id = uuid::Uuid::new_v4().to_string();
            let safe_file_name = sanitize_import_file_name(&metadata.file_name)?;
            let destination = native_root
                .join("attachments")
                .join(target_object)
                .join(&attachment_id)
                .join(&safe_file_name);
            let attachment = AttachmentMeta {
                id: attachment_id.clone(),
                object_id: target_object.into(),
                file_name: safe_file_name.clone(),
                mime_type: metadata.mime_type.clone(),
                size_bytes: 0,
                created_at: now.into(),
                deleted_at: None,
                src_path: Some(destination.to_string_lossy().into_owned()),
                vault_path: Some(destination.to_string_lossy().into_owned()),
                description: None,
                tags: Vec::new(),
            };
            steps.push(ImportAttachmentStepPlan {
                entry_ordinal: u32::try_from(ordinal).map_err(|_| "import_too_many_zip_entries")?,
                source_object_id: source_object.into(),
                source_attachment_id: source_attachment.into(),
                owner_id: target_object.into(),
                attachment_id,
                safe_file_name,
                metadata: serde_json::to_value(attachment)?,
                initial_staged_proof: None,
            });
        }
    }
    // 改动仅限 __attachments：已有对象保持原可用集合，新对象不泄漏导出设备路径。
    // 零选附件同样保持本地集合，不把空选择解释成删除用户原附件。
    let mut owners = Vec::new();
    let mut baselines = HashMap::new();
    let selected_owners: HashSet<_> = steps.iter().map(|step| step.owner_id.as_str()).collect();
    for write in &batch.objects {
        let owner_id = &write.record.id;
        if baselines.contains_key(owner_id) {
            continue;
        }
        let previous = vault.load_import_view_object(view, owner_id)?;
        let expected = previous
            .as_ref()
            .and_then(|obj| obj.properties.get("__attachments"))
            .cloned();
        baselines.insert(owner_id.clone(), expected.clone());
        if selected_owners.contains(owner_id.as_str()) {
            owners.push(ImportAttachmentOwnerPlan {
                owner_id: owner_id.clone(),
                expected_attachments: expected,
            });
        }
    }
    for write in &mut batch.objects {
        if let Some(map) = write.record.properties.as_object_mut() {
            match baselines.get(&write.record.id).cloned().flatten() {
                Some(value) => {
                    map.insert("__attachments".into(), value);
                }
                None => {
                    map.remove("__attachments");
                }
            }
        }
    }
    Ok(ImportOperationStart {
        operation_id: operation_id.into(),
        source_kind: plan.source_kind,
        source: owned.proof.clone(),
        request_fingerprint,
        root_binding,
        plan: serde_json::to_value(plan)?,
        owners,
        steps,
        preferences_required: include_preferences,
        source_ready: None,
    })
}

fn marker(
    account_id: &str,
    operation_id: &str,
    root_binding: &str,
    step: &ImportAttachmentStepPlan,
) -> ImportOwnedAttachmentMarker {
    ImportOwnedAttachmentMarker {
        account_id: account_id.into(),
        operation_id: operation_id.into(),
        entry_ordinal: step.entry_ordinal,
        owner_id: step.owner_id.clone(),
        attachment_id: step.attachment_id.clone(),
        root_binding: root_binding.into(),
    }
}

fn ensure_safe_child(root: &Path, relative: &Path, create: bool) -> Result<PathBuf, ExportError> {
    use std::path::Component;
    let canonical_root = std::fs::canonicalize(root)?;
    let mut path = canonical_root.clone();
    for component in relative.components() {
        let Component::Normal(part) = component else {
            return Err("import_unsafe_path".into());
        };
        path.push(part);
        match std::fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_symlink() || !meta.is_dir() => {
                return Err("import_unsafe_path".into())
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && create => {
                std::fs::create_dir(&path)?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))?;
                }
            }
            Err(error) => return Err(error.into()),
        }
    }
    if !std::fs::canonicalize(&path)?.starts_with(canonical_root) {
        return Err("import_unsafe_path".into());
    }
    Ok(path)
}

fn operation_root(native_root: &Path, operation_id: &str) -> Result<PathBuf, ExportError> {
    uuid::Uuid::parse_str(operation_id).map_err(|_| "import_invalid_operation_id")?;
    ensure_safe_child(
        native_root,
        &Path::new(OPERATION_DIR).join(operation_id),
        true,
    )
}

fn attempt_directory(
    native_root: &Path,
    operation_id: &str,
    epoch: u64,
    ordinal: u64,
) -> Result<PathBuf, ExportError> {
    let root = operation_root(native_root, operation_id)?;
    ensure_safe_child(
        &root,
        &PathBuf::from(format!("attempt-{epoch}")).join(ordinal.to_string()),
        true,
    )
}

fn target_directory(
    native_root: &Path,
    step: &ImportAttachmentStepPlan,
) -> Result<PathBuf, ExportError> {
    validate_import_id(&step.owner_id)?;
    validate_import_id(&step.attachment_id)?;
    ensure_safe_child(
        native_root,
        &Path::new("attachments").join(&step.owner_id),
        true,
    )
    .map(|parent| parent.join(&step.attachment_id))
}

fn ciphertext_proof(
    path: &Path,
    stage_epoch: u64,
    plaintext_length: u64,
) -> Result<ImportCiphertextProof, ExportError> {
    let mut file = BufReader::new(File::open(path)?);
    let mut digest = Sha256::new();
    let mut length = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
        length += read as u64;
    }
    Ok(ImportCiphertextProof {
        sha256: hex::encode(digest.finalize()),
        length,
        stage_epoch,
        plaintext_length,
    })
}

struct PlaintextCounter(u64);
impl Write for PlaintextCounter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self
            .0
            .checked_add(bytes.len() as u64)
            .ok_or_else(|| std::io::Error::other("import_attachment_too_large"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn verify_ciphertext(
    path: &Path,
    expected: &ImportCiphertextProof,
    current_key: &[u8; 32],
) -> Result<(), ExportError> {
    if std::fs::symlink_metadata(path)?.file_type().is_symlink()
        || !crate::attachment_crypto::is_encrypted_file(path)
        || &ciphertext_proof(path, expected.stage_epoch, expected.plaintext_length)? != expected
    {
        return Err("import_staged_ciphertext_changed".into());
    }
    let mut file = BufReader::new(File::open(path)?);
    let mut count = PlaintextCounter(0);
    solosoul_crypto::cipher::decrypt_chunked_stream(current_key, &mut file, &mut count)
        .map_err(|_| ExportError::Msg("import_staged_key_mismatch".into()))?;
    if count.0 != expected.plaintext_length {
        return Err("import_staged_length_mismatch".into());
    }
    Ok(())
}

fn sync_directory(path: &Path) -> Result<(), ExportError> {
    #[cfg(unix)]
    {
        File::open(path)?.sync_all()?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

fn write_marker(path: &Path, value: &ImportOwnedAttachmentMarker) -> Result<(), ExportError> {
    let marker_path = path.join(IMPORT_OWNER_MARKER);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(marker_path)?;
    serde_json::to_writer(&mut file, value)?;
    file.flush()?;
    file.sync_all()?;
    sync_directory(path)
}

fn marker_matches(
    directory: &Path,
    expected: &ImportOwnedAttachmentMarker,
) -> Result<bool, ExportError> {
    let Ok(meta) = std::fs::symlink_metadata(directory) else {
        return Ok(false);
    };
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Ok(false);
    }
    let path = directory.join(IMPORT_OWNER_MARKER);
    let Ok(meta) = std::fs::symlink_metadata(&path) else {
        return Ok(false);
    };
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > 4096 {
        return Ok(false);
    }
    let value = serde_json::from_reader::<_, ImportOwnedAttachmentMarker>(File::open(path)?);
    Ok(value.as_ref().is_ok_and(|value| value == expected))
}

/// Marker 是识别候选，不是删除授权。只有 Vault Immediate 重读能批准 orphan 删除。
pub fn import_owner_marker_identity(path: &Path) -> Option<ImportOwnedAttachmentMarker> {
    let directory = std::fs::symlink_metadata(path).ok()?;
    if !directory.is_dir() || directory.file_type().is_symlink() {
        return None;
    }
    let marker_path = path.join(IMPORT_OWNER_MARKER);
    let meta = std::fs::symlink_metadata(&marker_path).ok()?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > 4096 {
        return None;
    }
    let value: ImportOwnedAttachmentMarker =
        serde_json::from_reader(File::open(marker_path).ok()?).ok()?;
    if uuid::Uuid::parse_str(&value.operation_id).is_err() {
        return None;
    }
    Some(value)
}

#[allow(clippy::too_many_arguments)]
fn stage_attachment(
    owned: &OwnedImportPackage,
    opened: &OpenedImportPackage,
    native_root: &Path,
    account_id: &str,
    operation_id: &str,
    root_binding: &str,
    epoch: u64,
    step: &ImportAttachmentStepPlan,
    current_key: &[u8; 32],
) -> Result<ImportCiphertextProof, ExportError> {
    let directory = attempt_directory(
        native_root,
        operation_id,
        epoch,
        u64::from(step.entry_ordinal),
    )?;
    let marker_value = marker(account_id, operation_id, root_binding, step);
    // Each attempt has a private immutable ciphertext slot; an old worker cannot rewrite a new epoch.
    if directory.join("attachment").exists() || directory.join(IMPORT_OWNER_MARKER).exists() {
        return Err("import_stage_slot_exists".into());
    }
    let mut archive = ZipArchive::new(owned.open_file()?)?;
    let mut entry = archive.by_index(step.entry_ordinal as usize)?;
    let expected_name = format!(
        "attachments/{}/{}.enc",
        step.source_object_id, step.source_attachment_id
    );
    if entry.name() != expected_name {
        return Err("import_source_entry_changed".into());
    }
    let export_attachment_key = Zeroizing::new(
        solosoul_crypto::hkdf_ext::derive_hkdf_key(
            opened.key(),
            opened.salt(),
            b"solosoul:attachments:v1",
        )
        .map_err(|_| "import_attachment_key_derivation_failed")?,
    );
    // NamedTempFile creates 0600 plaintext from the start, and drop removes it on every ordinary error.
    let plaintext_directory = tempfile::Builder::new()
        .prefix("solosoul-import-tmp-")
        .tempdir_in(native_root)?;
    let mut plaintext = tempfile::NamedTempFile::new_in(plaintext_directory.path())?;
    solosoul_crypto::cipher::decrypt_chunked_stream(
        &export_attachment_key,
        &mut entry,
        &mut plaintext,
    )
    .map_err(|_| "import_attachment_decryption_failed")?;
    plaintext.flush()?;
    let plaintext_length = plaintext.as_file().metadata()?.len();
    let ciphertext_path = directory.join("attachment");
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&ciphertext_path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        output.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    let mut reader = BufReader::new(plaintext.reopen()?);
    solosoul_crypto::cipher::encrypt_chunked_stream(
        current_key,
        plaintext_length,
        &mut reader,
        &mut output,
    )
    .map_err(|_| "import_attachment_encryption_failed")?;
    output.flush()?;
    output.sync_all()?;
    drop(output);
    write_marker(&directory, &marker_value)?;
    sync_directory(&directory)?;
    let proof = ciphertext_proof(&ciphertext_path, epoch, plaintext_length)?;
    verify_ciphertext(&ciphertext_path, &proof, current_key)?;
    Ok(proof)
}

fn sidecar_path(directory: &Path) -> Result<PathBuf, ExportError> {
    let name = directory
        .file_name()
        .ok_or("import_unsafe_path")?
        .to_string_lossy();
    Ok(directory.with_file_name(format!("{name}.import-owner")))
}

fn read_marker_file(path: &Path) -> Result<Option<ImportOwnedAttachmentMarker>, ExportError> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > 4096 => {
            Ok(None)
        }
        Ok(_) => Ok(serde_json::from_reader(File::open(path)?).ok()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// The sidecar reserves ownership before creating the directory, closing the crash-before-marker window.
/// Files are persisted with no-clobber semantics. Sidecar/inner marker only prove identity; stored journal
/// and epoch provide write authority. Foreign directories/markers are preserved.
fn publish_file(
    native_root: &Path,
    account_id: &str,
    operation_id: &str,
    root_binding: &str,
    step: &ImportAttachmentStep,
    on_published: impl FnOnce(),
) -> Result<(), ExportError> {
    let proof = step
        .staged_proof
        .as_ref()
        .ok_or("import_attachment_not_staged")?;
    let plan = &step.plan;
    let expected = marker(account_id, operation_id, root_binding, plan);
    let destination = target_directory(native_root, plan)?;
    let sidecar = sidecar_path(&destination)?;
    if sidecar.exists() {
        if read_marker_file(&sidecar)?.as_ref() != Some(&expected) {
            return Err("import_target_collision".into());
        }
    } else {
        if destination.exists() {
            return Err("import_target_collision".into());
        }
        let parent = destination.parent().ok_or("import_unsafe_path")?;
        let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
        serde_json::to_writer(&mut temporary, &expected)?;
        temporary.flush()?;
        temporary.as_file().sync_all()?;
        temporary
            .persist_noclobber(&sidecar)
            .map_err(|_| "import_target_collision")?;
        sync_directory(parent)?;
    }
    if !destination.exists() {
        std::fs::create_dir(&destination)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&destination, std::fs::Permissions::from_mode(0o700))?;
        }
    } else {
        let meta = std::fs::symlink_metadata(&destination)?;
        if !meta.is_dir() || meta.file_type().is_symlink() {
            return Err("import_target_collision".into());
        }
    }
    let inner_marker = destination.join(IMPORT_OWNER_MARKER);
    if inner_marker.exists() {
        if !marker_matches(&destination, &expected)? {
            return Err("import_target_collision".into());
        }
    } else {
        write_marker(&destination, &expected)?;
    }
    let final_file = destination.join(&plan.safe_file_name);
    let stage = attempt_directory(
        native_root,
        operation_id,
        proof.stage_epoch,
        u64::from(plan.entry_ordinal),
    )?
    .join("attachment");
    if final_file.exists() {
        // The expensive proof/key check already happened outside this bounded transaction.
        if !marker_matches(&destination, &expected)? {
            return Err("import_target_collision".into());
        }
        on_published();
        return Ok(());
    }
    let mut owned_stage = tempfile::TempPath::try_from_path(&stage)?;
    // A failed no-clobber publication must retain the previously confirmed ciphertext for retry.
    owned_stage.disable_cleanup(true);
    owned_stage
        .persist_noclobber(&final_file)
        .map_err(|error| ExportError::Io(error.error))?;
    // 真实发布点先计数；后续 flush / SQL 失败仍保留可验证的最终密文。
    on_published();
    // Windows FlushFileBuffers 要求可写句柄，不能用 File::open 的只读句柄。
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&final_file)?
        .sync_all()?;
    sync_directory(&destination)?;
    Ok(())
}

fn verified_publish_candidate(
    native_root: &Path,
    account_id: &str,
    operation_id: &str,
    root_binding: &str,
    step: &ImportAttachmentStep,
    current_key: &[u8; 32],
) -> Result<(), ExportError> {
    let proof = step
        .staged_proof
        .as_ref()
        .ok_or("import_attachment_not_staged")?;
    let destination = target_directory(native_root, &step.plan)?;
    let final_file = destination.join(&step.plan.safe_file_name);
    if std::fs::symlink_metadata(&destination).is_ok() {
        let expected = marker(account_id, operation_id, root_binding, &step.plan);
        if !marker_matches(&destination, &expected)? {
            // A valid sidecar can reconstruct the inner marker after a creation crash, but an
            // existing file without its inner ownership marker can never be adopted.
            if final_file.exists()
                || read_marker_file(&sidecar_path(&destination)?)?.as_ref() != Some(&expected)
            {
                return Err("import_target_collision".into());
            }
        }
        if final_file.exists() {
            return verify_ciphertext(&final_file, proof, current_key);
        }
    }
    let stage_directory = attempt_directory(
        native_root,
        operation_id,
        proof.stage_epoch,
        u64::from(step.plan.entry_ordinal),
    )?;
    if !marker_matches(
        &stage_directory,
        &marker(account_id, operation_id, root_binding, &step.plan),
    )? {
        return Err("import_staged_owner_mismatch".into());
    }
    verify_ciphertext(&stage_directory.join("attachment"), proof, current_key)
}

fn decoded_plan(
    operation: &ImportOperationRecord,
    native_root: &Path,
    vault: &VaultStore,
) -> Result<NativeImportPlan, ExportError> {
    let plan: NativeImportPlan = serde_json::from_value(operation.start.plan.clone())?;
    let root = std::fs::canonicalize(native_root)?
        .to_string_lossy()
        .into_owned();
    if plan.format_version != FORMAT_VERSION
        || plan.native_root != root
        || plan.root_binding != vault.import_root_binding()?
        || plan.root_binding != operation.start.root_binding
        || plan.source_proof != operation.start.source
        || plan.source_kind != operation.start.source_kind
        || plan.request_fingerprint != operation.start.request_fingerprint
    {
        return Err("import_operation_plan_mismatch".into());
    }
    Ok(plan)
}

fn source_credentials<'a>(
    operation: &ImportOperationRecord,
    owned: Option<&'a OwnedImportPackage>,
    password: Option<&str>,
    native_root: &Path,
) -> Result<(&'a OwnedImportPackage, OpenedImportPackage), ExportError> {
    if operation.start.source_kind == ImportSourceKind::Recovery {
        return Err("import_recovery_handoff_unavailable".into());
    }
    let owned = owned.ok_or("import_source_required")?;
    if owned.source_proof() != &operation.start.source {
        return Err("import_source_changed".into());
    }
    let opened = owned.decrypt(
        password
            .filter(|value| !value.is_empty())
            .ok_or("import_package_password_required")?,
        native_root,
    )?;
    Ok((owned, opened))
}

fn preference_bytes(
    owned: &OwnedImportPackage,
    opened: &OpenedImportPackage,
) -> Result<Vec<u8>, ExportError> {
    let key = Zeroizing::new(
        solosoul_crypto::hkdf_ext::derive_hkdf_key(
            opened.key(),
            opened.salt(),
            b"solosoul:preferences:v1",
        )
        .map_err(|_| "import_preferences_key_derivation_failed")?,
    );
    let mut archive = ZipArchive::new(owned.open_file()?)?;
    let mut entry = archive
        .by_name("preferences.enc")
        .map_err(|_| "ZIP 中缺少: preferences.enc")?;
    if entry.size() > MAX_ZIP_ENTRY_SIZE {
        return Err("import_preferences_too_large".into());
    }
    let mut encrypted = Vec::new();
    entry
        .by_ref()
        .take(MAX_ZIP_ENTRY_SIZE + 1)
        .read_to_end(&mut encrypted)?;
    if encrypted.len() as u64 > MAX_ZIP_ENTRY_SIZE {
        return Err("import_preferences_too_large".into());
    }
    let bytes = solosoul_crypto::cipher::decrypt_from_bytes(&key, &encrypted, None)
        .map_err(|_| "解密偏好设置失败")?;
    Ok(bytes.to_vec())
}

/// Reopens the original account operation with a new current Session/key. Password is only needed
/// when unconfirmed source-dependent work remains. Committed records are never prepared a second time.
#[allow(clippy::too_many_arguments)]
pub fn resume_import_operation(
    service: &crate::VaultService,
    session: &crate::VaultSession,
    operation_id: &str,
    native_root: &Path,
    owned: Option<&OwnedImportPackage>,
    package_password: Option<&str>,
    attachment_key: &[u8; 32],
    progress: Option<&(dyn Fn(u8) + Send + Sync)>,
    committed: &mut AttachmentImportProgress,
) -> Result<ImportOperationRecord, ExportError> {
    let _activity = crate::import_activity::begin_import_activity(native_root)?;
    let operation = service
        .with_session(session, |vault| {
            vault.load_import_operation(session.account_id(), operation_id)
        })?
        .ok_or("import_operation_missing")?;
    let plan = decoded_plan(&operation, native_root, session.vault())?;
    committed.committed_count = operation.attachment_count;
    committed.written_file_count = operation
        .steps
        .iter()
        .filter(|step| {
            matches!(
                step.phase,
                ImportAttachmentPhase::Published | ImportAttachmentPhase::MetadataCommitted
            )
        })
        .count();
    let mut counted_files: HashSet<u32> = operation
        .steps
        .iter()
        .filter(|step| {
            matches!(
                step.phase,
                ImportAttachmentPhase::Published | ImportAttachmentPhase::MetadataCommitted
            )
        })
        .map(|step| step.plan.entry_ordinal)
        .collect();
    // A prior process may have published a file while its phase COMMIT failed. Only matching
    // marker + stored ciphertext proof + current AEAD key allow counting that actual publication.
    for step in &operation.steps {
        if step.phase != ImportAttachmentPhase::Staged {
            continue;
        }
        let Some(proof) = step.staged_proof.as_ref() else {
            continue;
        };
        let destination = target_directory(native_root, &step.plan)?;
        if marker_matches(
            &destination,
            &marker(
                session.account_id(),
                operation_id,
                &plan.root_binding,
                &step.plan,
            ),
        )? && verify_ciphertext(
            &destination.join(&step.plan.safe_file_name),
            proof,
            attachment_key,
        )
        .is_ok()
            && counted_files.insert(step.plan.entry_ordinal)
        {
            committed.written_file_count += 1;
        }
    }
    if owned.is_some_and(|package| package.source_proof() != &operation.start.source) {
        return Err("import_source_changed".into());
    }
    if operation.phase == ImportOperationPhase::Complete {
        return Ok(operation);
    }
    if operation.phase == ImportOperationPhase::Abandoned {
        return Err("import_operation_abandoned".into());
    }
    let lease = service.with_session(session, |vault| {
        vault.claim_import_operation(session.account_id(), operation_id, &plan.root_binding)
    })?;
    let needs_source = operation
        .steps
        .iter()
        .any(|step| step.phase == ImportAttachmentPhase::Planned)
        || operation.start.preferences_required
            && !operation.preferences_imported
            && matches!(plan.preferences, ImportPreferencesPlan::Package);
    let opened_source = if needs_source {
        Some(source_credentials(
            &operation,
            owned,
            package_password,
            native_root,
        )?)
    } else {
        None
    };
    let mut steps = operation.steps.clone();
    // Preserve ZIP order and the old publication-before-all-metadata stage boundary.
    steps.sort_by_key(|step| step.plan.entry_ordinal);
    for mut step in steps {
        if let Some(callback) = progress {
            callback(
                ((u64::from(step.plan.entry_ordinal) * 100) / plan.zip_entry_count.max(1) as u64)
                    .min(100) as u8,
            );
        }
        if step.phase == ImportAttachmentPhase::MetadataCommitted {
            continue;
        }
        if step.phase == ImportAttachmentPhase::Planned {
            let (owned, opened) = opened_source.as_ref().ok_or("import_source_required")?;
            let proof = stage_attachment(
                owned,
                opened,
                native_root,
                session.account_id(),
                operation_id,
                &plan.root_binding,
                lease.epoch(),
                &step.plan,
                attachment_key,
            )?;
            service.with_session(session, |vault| {
                vault.confirm_import_attachment_staged(&lease, step.plan.entry_ordinal, &proof)
            })?;
            step.staged_proof = Some(proof);
            step.phase = ImportAttachmentPhase::Staged;
        }
        verified_publish_candidate(
            native_root,
            session.account_id(),
            operation_id,
            &plan.root_binding,
            &step,
            attachment_key,
        )?;
        if step.phase == ImportAttachmentPhase::Staged {
            service.with_session(session, |vault| {
                vault.publish_import_attachment(&lease, step.plan.entry_ordinal, |stored| {
                    if stored.plan != step.plan || stored.staged_proof != step.staged_proof {
                        return Err("import_stage_changed".into());
                    }
                    publish_file(
                        native_root,
                        session.account_id(),
                        operation_id,
                        &plan.root_binding,
                        stored,
                        || {
                            if counted_files.insert(stored.plan.entry_ordinal) {
                                committed.written_file_count += 1;
                            }
                        },
                    )
                    .map_err(|error| error.to_string())?;
                    Ok(())
                })
            })?;
        }
    }
    let mut owners = Vec::new();
    for step in &operation.start.steps {
        if !owners.contains(&step.owner_id) {
            owners.push(step.owner_id.clone());
        }
    }
    for owner in owners {
        let updated = service.with_session(session, |vault| {
            vault.commit_import_attachment_metadata(&lease, &owner)
        })?;
        committed.committed_count = updated.attachment_count;
    }
    if operation.start.preferences_required && !operation.preferences_imported {
        let bytes = match &plan.preferences {
            ImportPreferencesPlan::Ready(bytes) => bytes.clone(),
            ImportPreferencesPlan::Error(code) => return Err(ExportError::Msg(code.clone())),
            ImportPreferencesPlan::Package => {
                let (owned, opened) = opened_source.as_ref().ok_or("import_source_required")?;
                preference_bytes(owned, opened)?
            }
            ImportPreferencesPlan::None => return Err("import_preferences_plan_missing".into()),
        };
        let profile = Profile::new_with_id(session.account_id(), session.account_id(), bytes);
        service.with_session(session, |vault| {
            vault.commit_import_preferences(&lease, &profile)
        })?;
    }
    let complete =
        service.with_session(session, |vault| vault.complete_import_operation(&lease))?;
    if let Some(callback) = progress {
        callback(100);
    }
    Ok(complete)
}

/// Before accepting a Recovery business batch, all package-dependent files/preferences are prepared.
/// This guard owns only this fresh precommit operation directory; accepted handoff survives process exit.
pub struct PreparedImportHandoff {
    directory: PathBuf,
    marker: ImportOwnedAttachmentMarker,
    accepted: bool,
    _activity: crate::import_activity::RootActivityGuard,
}

impl PreparedImportHandoff {
    pub fn accept(mut self) {
        self.accepted = true;
    }
}

impl Drop for PreparedImportHandoff {
    fn drop(&mut self) {
        if self.accepted {
            return;
        }
        let check = read_marker_file(&self.directory.join(".prepared-owner"));
        if check
            .as_ref()
            .is_ok_and(|value| value.as_ref() == Some(&self.marker))
        {
            // Exact marker identity and canonical private operation directory authorize this cleanup.
            let _ = std::fs::remove_dir_all(&self.directory);
        }
    }
}

pub fn prepare_recovery_handoff(
    service: &crate::VaultService,
    session: &crate::VaultSession,
    native_root: &Path,
    owned: &OwnedImportPackage,
    opened: &OpenedImportPackage,
    start: &mut ImportOperationStart,
    attachment_key: &[u8; 32],
) -> Result<PreparedImportHandoff, ExportError> {
    // 返回的 handoff 可能比 Service/Session 活得更久；其 Drop 清理也必须持原 root owner。
    let owner = service.root_owner();
    if std::fs::canonicalize(native_root)?.as_path() != owner.root() {
        return Err("VAULT_ROOT_MISMATCH".into());
    }
    let activity = crate::import_activity::begin_owned_root_activity(owner)?;
    service.with_session(session, |_| Ok(()))?;
    if start.source_kind != ImportSourceKind::Recovery
        || start.source != *owned.source_proof()
        || start.root_binding != session.vault().import_root_binding()?
    {
        return Err("import_recovery_handoff_mismatch".into());
    }
    let mut plan: NativeImportPlan = serde_json::from_value(start.plan.clone())?;
    if plan.native_root != std::fs::canonicalize(native_root)?.to_string_lossy() {
        return Err("import_root_binding_mismatch".into());
    }
    let parent = ensure_safe_child(native_root, Path::new(OPERATION_DIR), true)?;
    let directory = parent.join(&start.operation_id);
    // Fresh preparation does not reuse unknown old files, even when a caller reuses an operation UUID.
    std::fs::create_dir(&directory)?;
    let marker_value = ImportOwnedAttachmentMarker {
        account_id: session.account_id().into(),
        operation_id: start.operation_id.clone(),
        entry_ordinal: 0,
        owner_id: session.account_id().into(),
        attachment_id: start.operation_id.clone(),
        root_binding: start.root_binding.clone(),
    };
    let mut marker_file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join(".prepared-owner"))?;
    serde_json::to_writer(&mut marker_file, &marker_value)?;
    marker_file.flush()?;
    marker_file.sync_all()?;
    let handoff = PreparedImportHandoff {
        directory,
        marker: marker_value,
        accepted: false,
        _activity: activity,
    };
    for step in &mut start.steps {
        step.initial_staged_proof = Some(stage_attachment(
            owned,
            opened,
            native_root,
            session.account_id(),
            &start.operation_id,
            &start.root_binding,
            0,
            step,
            attachment_key,
        )?);
        service.with_session(session, |_| Ok(()))?;
    }
    plan.preferences = if start.preferences_required {
        match preference_bytes(owned, opened) {
            Ok(bytes) => ImportPreferencesPlan::Ready(bytes),
            Err(_) => return Err("import_recovery_preferences_failed".into()),
        }
    } else {
        ImportPreferencesPlan::None
    };
    start.plan = serde_json::to_value(plan)?;
    start.source_ready = Some(
        serde_json::json!({ "version": FORMAT_VERSION, "sourceIndependent": true, "preferencesReady": true }),
    );
    service.with_session(session, |_| Ok(()))?;
    Ok(handoff)
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ImportCredentialState {
    Ready,
    PackagePassword,
    RecoveryUnavailable,
}

pub fn import_credential_state(
    operation: &ImportOperationRecord,
) -> Result<ImportCredentialState, ExportError> {
    let plan: NativeImportPlan = serde_json::from_value(operation.start.plan.clone())?;
    if operation.phase == ImportOperationPhase::Complete {
        return Ok(ImportCredentialState::Ready);
    }
    let source_dependent = operation
        .steps
        .iter()
        .any(|step| step.phase == ImportAttachmentPhase::Planned)
        || operation.start.preferences_required
            && !operation.preferences_imported
            && matches!(plan.preferences, ImportPreferencesPlan::Package);
    Ok(if !source_dependent {
        ImportCredentialState::Ready
    } else if operation.start.source_kind == ImportSourceKind::Recovery {
        ImportCredentialState::RecoveryUnavailable
    } else {
        ImportCredentialState::PackagePassword
    })
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportCredentialRequirements {
    pub state: ImportCredentialState,
    pub source_required: bool,
    pub password_required: bool,
}

pub fn import_credential_requirements(
    operation: &ImportOperationRecord,
) -> Result<ImportCredentialRequirements, ExportError> {
    let state = import_credential_state(operation)?;
    Ok(ImportCredentialRequirements {
        state,
        source_required: state != ImportCredentialState::Ready,
        password_required: state == ImportCredentialState::PackagePassword,
    })
}

#[cfg(test)]
mod tests;
