//! RF-312 独立媒体 fixture：固定公开素材、真实附件加密、源仅读与路径重定位。
//! v1 启动 fixture 不接纳本合同；真实 GUI 行程尚需单独接入和测量。
use super::{fixture, make_object, PASSWORD};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use solosoul_core::{attachment_crypto, objects, VaultService};
use solosoul_crypto::KdfConfig;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

const MARKER: &str = "rf312-media-fixture.json";
const OBJECT: &str = "obj_perf_00000000";
const ASSETS: [(&str, &str, &[u8]); 4] = [
    (
        "ocr_test.png",
        "image/png",
        include_bytes!("../../tests/fixtures/ocr_test.png"),
    ),
    (
        "text_only.pdf",
        "application/pdf",
        include_bytes!("../../tests/fixtures/text_only.pdf"),
    ),
    (
        "scanned.pdf",
        "application/pdf",
        include_bytes!("../../tests/fixtures/scanned.pdf"),
    ),
    (
        "preview.txt",
        "text/plain",
        b"RF-312 public synthetic attachment preview.\nHello SoloSoul 1234567890.\n",
    ),
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ClosedFile {
    relative_path: String,
    sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Attachment {
    id: String,
    relative_path: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    scope: String,
    generator: String,
    base_fixture: Value,
    media_object_id: String,
    includes_attachments: bool,
    includes_ocr_fixture: bool,
    public_assets: Value,
    attachments: Vec<Attachment>,
    closed_files: Vec<ClosedFile>,
}

enum Mode {
    Generate(PathBuf, usize),
    Verify(PathBuf),
    Copy(PathBuf, PathBuf),
}

pub(super) fn run_if_requested() -> Option<Result<Value, String>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if !args.iter().any(|a| {
        [
            "--media-fixture-output",
            "--verify-media-fixture",
            "--copy-media-fixture",
        ]
        .iter()
        .any(|flag| a == flag)
    }) {
        return None;
    }
    Some(parse(&args).and_then(|m| match m {
        Mode::Generate(p, n) => generate(&p, n),
        Mode::Verify(p) => verify(&p),
        Mode::Copy(p, out) => copy(&p, &out),
    }))
}
fn parse(args: &[OsString]) -> Result<Mode, String> {
    let (mut out, mut input, mut source, mut count) = (None, None, None, None);
    for pair in args.chunks(2) {
        let flag = pair[0].to_str().ok_or("media flag must be UTF-8")?;
        let value = pair
            .get(1)
            .filter(|v| !v.to_string_lossy().starts_with("--"))
            .ok_or("media flag requires a value")?;
        match flag {
            "--media-fixture-output" if out.is_none() => out = Some(PathBuf::from(value)),
            "--verify-media-fixture" if input.is_none() => input = Some(PathBuf::from(value)),
            "--copy-media-fixture" if source.is_none() => source = Some(PathBuf::from(value)),
            "--objects" if count.is_none() => {
                count = Some(
                    value
                        .to_str()
                        .and_then(|s| s.parse::<usize>().ok())
                        .filter(|n| [100, 5000].contains(n))
                        .ok_or("media count must be 100 or 5000")?,
                )
            }
            _ => return Err("unknown or duplicate media fixture option".into()),
        }
    }
    match (out, input, source, count) {
        (Some(p), None, None, n) => Ok(Mode::Generate(p, n.unwrap_or(100))),
        (None, Some(p), None, None) => Ok(Mode::Verify(p)),
        (Some(p), None, Some(src), None) => Ok(Mode::Copy(src, p)),
        _ => Err("use exclusive media generation, verification, or copy with a new output".into()),
    }
}
fn sha(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
fn hash_file(path: &Path) -> Result<String, String> {
    fixture::require_path(path, false)?;
    if fs::metadata(path).map_err(|e| e.to_string())?.len() > 256 * 1024 * 1024 {
        return Err("media file exceeds 256 MiB".into());
    }
    fs::read(path)
        .map(|bytes| sha(&bytes))
        .map_err(|e| e.to_string())
}
fn assets() -> Value {
    json!(ASSETS.iter().map(|(name, mime, bytes)| json!({"fileName":name,"mimeType":mime,"bytes":bytes.len(),"sha256":sha(bytes)})).collect::<Vec<_>>())
}
fn descriptor(id: &str, name: &str) -> Attachment {
    Attachment {
        id: id.into(),
        relative_path: format!("attachments/{OBJECT}/{id}/{name}"),
    }
}
fn expected_paths(account: &str, attachments: &[Attachment]) -> BTreeSet<String> {
    [
        "accounts.json".into(),
        "ui_preferences.json".into(),
        format!("{account}/config.json"),
        format!("{account}/vault.db"),
    ]
    .into_iter()
    .chain(attachments.iter().map(|a| a.relative_path.clone()))
    .collect()
}
fn files(
    base: &Path,
    account: &str,
    attachments: &[Attachment],
) -> Result<Vec<ClosedFile>, String> {
    expected_paths(account, attachments)
        .into_iter()
        .map(|relative_path| {
            Ok(ClosedFile {
                sha256: hash_file(&base.join(&relative_path))?,
                relative_path,
            })
        })
        .collect()
}
fn manifest(
    base_fixture: Value,
    attachments: Vec<Attachment>,
    closed_files: Vec<ClosedFile>,
) -> Manifest {
    Manifest {
        schema_version: 2,
        scope: "synthetic-native-media-vault-fixture".into(),
        generator: "solosoul-core/examples/perf_baseline --media-fixture-output".into(),
        base_fixture,
        media_object_id: OBJECT.into(),
        includes_attachments: true,
        includes_ocr_fixture: true,
        public_assets: assets(),
        attachments,
        closed_files,
    }
}
// 目录白名单来自固定 account 和四个已校验 UUID；不接受额外文件或 reparse point。
fn inventory(base: &Path, m: &Manifest, marker_present: bool) -> Result<Vec<ClosedFile>, String> {
    let account = m.base_fixture["accountId"]
        .as_str()
        .ok_or("media account missing")?;
    let mut allowed = expected_paths(account, &m.attachments);
    allowed.extend([
        ".lock".into(),
        "accounts.bak".into(),
        format!("{account}/vault.db.pre_enc.bak"),
    ]);
    if marker_present {
        allowed.insert(MARKER.into());
    }
    let mut dirs = BTreeSet::new();
    for p in &allowed {
        let mut parent = Path::new(p).parent();
        while let Some(p) = parent.filter(|p| !p.as_os_str().is_empty()) {
            dirs.insert(p.to_string_lossy().replace('\\', "/"));
            parent = p.parent();
        }
    }
    let mut stack = vec![base.to_path_buf()];
    let mut found = vec![];
    while let Some(dir) = stack.pop() {
        fixture::require_path(&dir, true)?;
        for item in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let p = item.map_err(|e| e.to_string())?.path();
            let relative = p
                .strip_prefix(base)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            if dirs.contains(&relative) {
                fixture::require_path(&p, true)?;
                stack.push(p);
            } else if allowed.contains(&relative) {
                found.push(ClosedFile {
                    relative_path: relative,
                    sha256: hash_file(&p)?,
                });
            } else {
                return Err("media fixture contains an unexpected path".into());
            }
        }
    }
    found.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    if !expected_paths(account, &m.attachments)
        .iter()
        .all(|p| found.iter().any(|f| &f.relative_path == p))
    {
        return Err("media fixture missing a required file".into());
    }
    Ok(found)
}
fn validate(base: &Path, value: Value) -> Result<Manifest, String> {
    let m: Manifest = serde_json::from_value(value.clone())
        .map_err(|e| format!("invalid media fixture marker: {e}"))?;
    let count = m.base_fixture["objectCount"]
        .as_u64()
        .filter(|n| [100, 5000].contains(n))
        .ok_or("invalid media count")? as usize;
    let profile = m.base_fixture["buildProfile"]
        .as_str()
        .filter(|p| ["debug", "release"].contains(p))
        .ok_or("invalid media build profile")?;
    let k = &m.base_fixture["kdf"];
    let kdf = KdfConfig {
        memory_kb: k["memoryKiB"]
            .as_u64()
            .and_then(|n| n.try_into().ok())
            .ok_or("invalid media KDF")?,
        iterations: k["iterations"]
            .as_u64()
            .and_then(|n| n.try_into().ok())
            .ok_or("invalid media KDF")?,
        parallelism: k["parallelism"]
            .as_u64()
            .and_then(|n| n.try_into().ok())
            .ok_or("invalid media KDF")?,
    };
    if kdf != KdfConfig::production() && kdf != KdfConfig::development()
        || !cfg!(debug_assertions) && kdf != KdfConfig::production()
    {
        return Err("release media fixture requires production KDF".into());
    }
    if m.attachments.len() != ASSETS.len() {
        return Err("media fixture needs exactly four attachments".into());
    }
    let ids: BTreeSet<_> = m.attachments.iter().map(|a| &a.id).collect();
    if ids.len() != 4 {
        return Err("duplicate media attachment ID".into());
    }
    for (a, (name, _, _)) in m.attachments.iter().zip(ASSETS) {
        let id =
            a.id.strip_prefix("att_")
                .and_then(|s| uuid::Uuid::parse_str(s).ok())
                .filter(|u| a.id == format!("att_{u}"));
        if id.is_none() || *a != descriptor(&a.id, name) {
            return Err("invalid media attachment ID/path".into());
        }
    }
    let expected = manifest(
        fixture::manifest(count, kdf, profile),
        m.attachments.clone(),
        files(base, &format!("acc_rf312_{count}"), &m.attachments)?,
    );
    if value != serde_json::to_value(expected).map_err(|e| e.to_string())? {
        return Err("media marker/content SHA differs from fixed contract".into());
    }
    inventory(base, &m, true)?;
    Ok(m)
}
fn checked_input(path: &Path) -> Result<(PathBuf, Manifest), String> {
    if !path.is_absolute() {
        return Err("media input must be an absolute fixture directory".into());
    }
    fixture::require_path(path, true)?;
    let base = path.canonicalize().map_err(|e| e.to_string())?;
    let marker_path = base.join(MARKER);
    fixture::require_path(&marker_path, false)?;
    if fs::metadata(&marker_path).map_err(|e| e.to_string())?.len() > 128 * 1024 {
        return Err("media marker exceeds 128 KiB".into());
    }
    let m = validate(&base, fixture::read_json(&marker_path)?)?;
    Ok((base, m))
}
fn copy_files(base: &Path, out: &Path, m: &Manifest) -> Result<(), String> {
    for f in &m.closed_files {
        let input_path = base.join(&f.relative_path);
        if hash_file(&input_path)? != f.sha256 {
            return Err("media source changed before closed-file copy".into());
        }
        let p = out.join(&f.relative_path);
        fs::create_dir_all(p.parent().ok_or("invalid media copy parent")?)
            .map_err(|e| e.to_string())?;
        let mut input = fs::File::open(&input_path).map_err(|e| e.to_string())?;
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&p)
            .map_err(|e| e.to_string())?;
        std::io::copy(&mut input, &mut output).map_err(|e| e.to_string())?;
        output.sync_all().map_err(|e| e.to_string())?;
        if hash_file(&p)? != f.sha256 {
            return Err("media closed-file copy differs from original proof".into());
        }
    }
    Ok(())
}
// 所有解锁/读取仅在副本；逐对象完整比较固定内容，附件仅允许四个固定公开素材。
fn inspect(copy: &Path, declared: &Path, m: &Manifest, rebase: bool) -> Result<Value, String> {
    let checks = fixture::verify_data(copy, &m.base_fixture)?;
    let account = m.base_fixture["accountId"]
        .as_str()
        .ok_or("missing media account")?;
    let count = m.base_fixture["objectCount"]
        .as_u64()
        .ok_or("missing media count")? as usize;
    let service = VaultService::try_with_base_path(copy.to_path_buf())?;
    service.load_accounts();
    service.unlock(account, PASSWORD)?;
    let vault = service
        .get_vault_store()
        .ok_or("media vault did not unlock")?;
    let key = service.attachment_encryption_key()?;
    let mut observations = vec![];
    for index in 0..count {
        let expected = make_object(account, index);
        let mut record = vault
            .load_object(&expected.id)?
            .ok_or("missing deterministic media object")?;
        if index == 0 {
            let raw = record
                .properties
                .get("__attachments")
                .ok_or("missing media attachments")?
                .clone();
            let mut atts = objects::load_attachments(&record.properties);
            if atts.len() != 4
                || serde_json::to_value(&atts).map_err(|e| e.to_string())? != raw
                || record.version != 5
                || chrono::DateTime::parse_from_rfc3339(&record.updated_at).is_err()
            {
                return Err("invalid media metadata shape/version".into());
            }
            for ((a, descriptor), (name, mime, bytes)) in
                atts.iter_mut().zip(&m.attachments).zip(ASSETS)
            {
                let original = declared
                    .join(&descriptor.relative_path)
                    .to_string_lossy()
                    .into_owned();
                if a.id != descriptor.id
                    || a.object_id != OBJECT
                    || a.file_name != name
                    || a.mime_type != mime
                    || a.size_bytes != bytes.len() as u64
                    || a.src_path.is_some()
                    || a.vault_path.as_deref() != Some(original.as_str())
                    || a.deleted_at.is_some()
                    || a.description.is_some()
                    || !a.tags.is_empty()
                    || chrono::DateTime::parse_from_rfc3339(&a.created_at).is_err()
                {
                    return Err(
                        "media metadata differs from fixed public attachment contract".into(),
                    );
                }
                let file = copy.join(&descriptor.relative_path);
                if !attachment_crypto::is_encrypted_file(&file) {
                    return Err("media attachment must be encrypted SOLC".into());
                }
                let decoded = attachment_crypto::read_file_decrypted(&key, &file, 1024 * 1024)?;
                if decoded != bytes {
                    return Err("media decrypted content differs from public fixture".into());
                }
                observations.push(json!({"fileName":name,"encrypted":true,"bytes":decoded.len(),"sha256":sha(&decoded)}));
                if rebase {
                    a.vault_path = Some(file.to_string_lossy().into_owned());
                }
            }
            if rebase {
                objects::save_attachments(&mut record.properties, &atts);
                vault.save_object(&record)?;
            }
            record
                .properties
                .as_object_mut()
                .ok_or("invalid media properties")?
                .remove("__attachments");
            record.version = 1;
            record.updated_at = expected.updated_at.clone();
        }
        if serde_json::to_value(&record).map_err(|e| e.to_string())?
            != serde_json::to_value(expected).map_err(|e| e.to_string())?
        {
            return Err("media deterministic object content mismatch".into());
        }
    }
    drop(vault);
    service.lock();
    drop(service);
    Ok(
        json!({"baseVerification":checks,"allObjectContentsVerified":count,"attachmentCount":observations.len(),"attachments":observations,"sourcePathsCleared":true,"metadataPathsVerified":true,"rebasedOnOwnedCopy":rebase,"success":true}),
    )
}
fn verify_owned_source(base: &Path, m: &Manifest, marker_present: bool) -> Result<Value, String> {
    let before = inventory(base, m, marker_present)?;
    let temporary = tempfile::tempdir().map_err(|e| e.to_string())?;
    let copy = temporary.path().join("closed-copy");
    fs::create_dir(&copy).map_err(|e| e.to_string())?;
    copy_files(base, &copy, m)?;
    let result = inspect(&copy, base, m, false);
    let after = inventory(base, m, marker_present)?;
    if before != after {
        return Err("media source changed during independent copy verification".into());
    }
    result.map(|mut v| {
        v["sourceFilesUnchanged"] = json!(true);
        v["sourceFileProofs"] = json!(before);
        v
    })
}
fn generate(path: &Path, count: usize) -> Result<Value, String> {
    if !cfg!(debug_assertions) && KdfConfig::from_env() != KdfConfig::production() {
        return Err("set SOLOSOUL_SECURE=1 for production media fixture generation".into());
    }
    let (out, base_marker) = fixture::populate(path, count)?;
    let source = tempfile::tempdir().map_err(|e| e.to_string())?;
    let service = VaultService::try_with_base_path(out.clone())?;
    let account = base_marker["accountId"]
        .as_str()
        .ok_or("missing generated account")?;
    service.load_accounts();
    service.unlock(account, PASSWORD)?;
    let vault = service
        .get_vault_store()
        .ok_or("media vault did not unlock")?;
    let key = service.attachment_encryption_key()?;
    let mut descriptors = vec![];
    for (name, _, bytes) in ASSETS {
        let path = source.path().join(name);
        fs::write(&path, bytes).map_err(|e| e.to_string())?;
        let a = objects::add_attachments(&vault, account, OBJECT, &path, &out, Some(&key))?;
        descriptors.push(descriptor(&a.id, name));
    }
    let mut record = vault.load_object(OBJECT)?.ok_or("missing media object")?;
    let mut atts = objects::load_attachments(&record.properties);
    for a in &mut atts {
        a.src_path = None;
    }
    objects::save_attachments(&mut record.properties, &atts);
    vault.save_object(&record)?;
    drop(vault);
    service.lock();
    drop(service);
    let proof = files(&out, account, &descriptors)?;
    let m = manifest(base_marker, descriptors, proof);
    let checks = verify_owned_source(&out, &m, false)?;
    fixture::write_new_json(
        &out.join(MARKER),
        &serde_json::to_value(&m).map_err(|e| e.to_string())?,
    )?;
    Ok(
        json!({"scope":"synthetic-media-fixture-generation","fixturePath":out,"manifest":m,"verification":checks,"nativeUiMeasured":false,"success":true}),
    )
}
fn verify(path: &Path) -> Result<Value, String> {
    let (base, m) = checked_input(path)?;
    let checks = verify_owned_source(&base, &m, true)?;
    Ok(
        json!({"scope":"synthetic-media-fixture-read-only-verification","fixturePath":base,"verification":checks,"nativeUiMeasured":false,"success":true}),
    )
}
fn copy(path: &Path, output: &Path) -> Result<Value, String> {
    let (base, m) = checked_input(path)?;
    let before = inventory(&base, &m, true)?;
    let verified = verify_owned_source(&base, &m, true)?;
    // 必须新目录，且源之外；失败不删除输入、不发布完成标记。
    if !output.is_absolute() {
        return Err("media copy output must be absolute".into());
    }
    let parent = output
        .parent()
        .ok_or("media copy output needs parent")?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if parent.starts_with(&base) {
        return Err("media copy must be outside source fixture".into());
    }
    let out = fixture::create_output(output)?;
    copy_files(&base, &out, &m)?;
    let rebase = inspect(&out, &base, &m, true)?;
    let account = m.base_fixture["accountId"]
        .as_str()
        .ok_or("missing media account")?;
    let proof = files(&out, account, &m.attachments)?;
    let final_marker = manifest(m.base_fixture.clone(), m.attachments.clone(), proof);
    let independent = verify_owned_source(&out, &final_marker, false)?;
    if before != inventory(&base, &m, true)? {
        return Err("media source changed during copy/rebase".into());
    }
    fixture::write_new_json(
        &out.join(MARKER),
        &serde_json::to_value(&final_marker).map_err(|e| e.to_string())?,
    )?;
    Ok(
        json!({"scope":"synthetic-media-fixture-owned-copy","fixturePath":out,"sourceFixture":base,"sourceVerification":verified,"rebase":rebase,"verification":independent,"sourceFilesUnchanged":true,"nativeUiMeasured":false,"success":true}),
    )
}

#[allow(dead_code)] // CLI 与非默认原生功能共享入口。
pub(crate) fn native_manifest(path: &Path) -> Result<Value, String> {
    let (_, m) = checked_input(path)?;
    serde_json::to_value(m).map_err(|e| e.to_string())
}
#[allow(dead_code)] // CLI 与非默认原生功能共享入口。
pub(crate) fn native_copy(source: &Path, output: &Path) -> Result<Value, String> {
    copy(source, output)
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn native_test_fixture(path: &Path) -> Result<(), String> {
    generate(path, 100)?;
    let mut marker = fixture::read_json(&path.join(MARKER))?;
    // 原生单测使用真实生产 KDF 的合成数据；Debug 构建仅改测试标记。
    marker["baseFixture"]["buildProfile"] = json!("release");
    fs::write(
        path.join(MARKER),
        serde_json::to_vec_pretty(&marker).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    native_manifest(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(items: &[&str]) -> Vec<OsString> {
        items.iter().map(OsString::from).collect()
    }
    fn generated() -> (tempfile::TempDir, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().join("media");
        generate(&base, 100).unwrap();
        (temp, base)
    }
    #[test]
    fn fixed_assets_match_repository_receipts() {
        assert_eq!(
            sha(ASSETS[0].2),
            "56a9d54d9b70a3ee1b22e0b08b755b6cfc15ee125df4b11d52f8523583dcacaa"
        );
        assert_eq!(
            sha(ASSETS[1].2),
            "ca60313e25ffa64f848d86780201a9570a0dbcb3bf733bfb5e4a65ed73f4fdfe"
        );
        assert_eq!(
            sha(ASSETS[2].2),
            "8fda928a9940813a4a245a2db1e943c64b4e717d20274f9b4f27ee5b9b3fc4a1"
        );
    }
    #[test]
    fn parser_rejects_mixed_duplicate_unbounded_and_incomplete_modes() {
        for items in [
            &["--media-fixture-output", "x", "--objects", "1"][..],
            &["--verify-media-fixture", "x", "--objects", "100"],
            &["--copy-media-fixture", "x"],
            &["--media-fixture-output", "x", "--media-fixture-output", "y"],
            &["--verify-media-fixture", "x", "--fixture-output", "y"],
            &["--verify-media-fixture"],
        ] {
            assert!(parse(&args(items)).is_err());
        }
        assert!(matches!(
            parse(&args(&[
                "--copy-media-fixture",
                "x",
                "--media-fixture-output",
                "y"
            ]))
            .unwrap(),
            Mode::Copy(..)
        ));
    }
    #[test]
    fn independent_reopen_preserves_every_source_file_and_rebases_owned_copy() {
        let (_temp, base) = generated();
        let (_, m) = checked_input(&base).unwrap();
        let before = inventory(&base, &m, true).unwrap();
        assert!(
            verify(&base).unwrap()["verification"]["sourceFilesUnchanged"]
                .as_bool()
                .unwrap()
        );
        let target = base.parent().unwrap().join("rebased");
        let copied = copy(&base, &target).unwrap();
        assert_eq!(copied["rebase"]["rebasedOnOwnedCopy"], true);
        assert_eq!(inventory(&base, &m, true).unwrap(), before);
        assert_eq!(
            verify(&target).unwrap()["verification"]["attachmentCount"],
            4
        );
        assert!(copy(&base, &target).is_err());
        assert!(copy(&base, &base.join("forbidden")).is_err());
        assert!(!base.join("rf312-fixture.json").exists());
    }
    #[test]
    fn rejects_unknown_marker_fields_extra_files_and_raw_digest_tampering() {
        let (_temp, base) = generated();
        let original = fs::read(base.join(MARKER)).unwrap();
        let mut marker: Value = serde_json::from_slice(&original).unwrap();
        marker["unexpected"] = json!(true);
        fs::write(base.join(MARKER), serde_json::to_vec(&marker).unwrap()).unwrap();
        assert!(verify(&base)
            .unwrap_err()
            .contains("invalid media fixture marker"));
        fs::write(base.join(MARKER), &original).unwrap();
        fs::write(base.join("unknown.txt"), b"unexpected").unwrap();
        assert!(verify(&base).unwrap_err().contains("unexpected path"));
        fs::remove_file(base.join("unknown.txt")).unwrap();
        let (_, m) = checked_input(&base).unwrap();
        fs::write(base.join(&m.attachments[0].relative_path), b"tampered").unwrap();
        assert!(verify(&base).unwrap_err().contains("SHA"));
    }
    #[test]
    fn rejects_source_change_before_copy_and_unbounded_marker_before_parsing() {
        let (_temp, base) = generated();
        let (_, m) = checked_input(&base).unwrap();
        fs::write(
            base.join(&m.attachments[0].relative_path),
            b"changed after validated marker",
        )
        .unwrap();
        let out = base.parent().unwrap().join("new-copy");
        fs::create_dir(&out).unwrap();
        assert!(copy_files(&base, &out, &m)
            .unwrap_err()
            .contains("changed before"));
        fs::write(base.join(MARKER), vec![b' '; 128 * 1024 + 1]).unwrap();
        assert!(verify(&base).unwrap_err().contains("exceeds 128 KiB"));
    }

    #[test]
    fn rejects_plaintext_even_with_consistent_file_digest() {
        let (_temp, base) = generated();
        let (_, mut m) = checked_input(&base).unwrap();
        fs::write(base.join(&m.attachments[0].relative_path), ASSETS[0].2).unwrap();
        m.closed_files = files(
            &base,
            m.base_fixture["accountId"].as_str().unwrap(),
            &m.attachments,
        )
        .unwrap();
        fs::write(base.join(MARKER), serde_json::to_vec(&m).unwrap()).unwrap();
        assert!(verify(&base).unwrap_err().contains("encrypted SOLC"));
    }
    #[test]
    fn rejects_changed_object_content_even_with_consistent_file_digest() {
        let (_temp, base) = generated();
        let (_, mut m) = checked_input(&base).unwrap();
        let account = m.base_fixture["accountId"].as_str().unwrap();
        let service = VaultService::try_with_base_path(base.clone()).unwrap();
        service.load_accounts();
        service.unlock(account, PASSWORD).unwrap();
        let vault = service.get_vault_store().unwrap();
        let mut record = vault.load_object("obj_perf_00000001").unwrap().unwrap();
        record.properties["body"] = json!("unexpected body without changing search matches");
        vault.save_object(&record).unwrap();
        drop(vault);
        service.lock();
        drop(service);
        m.closed_files = files(&base, account, &m.attachments).unwrap();
        fs::write(base.join(MARKER), serde_json::to_vec(&m).unwrap()).unwrap();
        assert!(verify(&base)
            .unwrap_err()
            .contains("object content mismatch"));
    }
}

// 原生 benchmark 共用相同纯数据合同，默认应用不包含这些入口。
