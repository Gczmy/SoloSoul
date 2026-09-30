//! RF-312 合成 GUI 数据准备。只使用显式路径，不访问默认用户 Vault。

use super::{make_object, PASSWORD};
use serde_json::{json, Value};
use solosoul_core::{vault_service::AccountConfig, VaultService};
use solosoul_crypto::KdfConfig;
use solosoul_vault::Profile;
use std::collections::HashSet;
use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};

const ACCOUNT_NAME: &str = "Performance Fixture";
const MARKER_NAME: &str = "rf312-fixture.json";

enum Mode {
    Generate(PathBuf, usize),
    Verify(PathBuf),
}

pub(super) fn run_if_requested() -> Option<Result<Value, String>> {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    if !args
        .iter()
        .any(|arg| arg == "--fixture-output" || arg == "--verify-fixture")
    {
        return None;
    }
    Some(parse_mode(&args).and_then(|mode| match mode {
        Mode::Generate(path, count) => generate(&path, count),
        Mode::Verify(path) => verify(&path),
    }))
}

fn parse_mode(args: &[OsString]) -> Result<Mode, String> {
    let mut output = None;
    let mut input = None;
    let mut objects = None;
    let mut index = 0;
    while index < args.len() {
        let flag = args[index]
            .to_str()
            .ok_or_else(|| "fixture option must be UTF-8".to_string())?;
        let value = args
            .get(index + 1)
            .filter(|value| !value.to_string_lossy().starts_with("--"))
            .ok_or_else(|| format!("{flag} requires a value"))?;
        match flag {
            "--fixture-output" if output.is_none() => output = Some(PathBuf::from(value)),
            "--verify-fixture" if input.is_none() => input = Some(PathBuf::from(value)),
            "--objects" if objects.is_none() => {
                let count = value
                    .to_str()
                    .and_then(|text| text.parse::<usize>().ok())
                    .filter(|count| matches!(count, 100 | 5000))
                    .ok_or_else(|| "fixture --objects must be 100 or 5000".to_string())?;
                objects = Some(count);
            }
            _ => return Err(format!("unknown or duplicate fixture option: {flag}")),
        }
        index += 2;
    }
    match (output, input, objects) {
        (Some(path), None, count) => Ok(Mode::Generate(path, count.unwrap_or(100))),
        (None, Some(path), None) => Ok(Mode::Verify(path)),
        _ => Err("use --fixture-output [--objects 100|5000] or --verify-fixture alone".into()),
    }
}

fn profile_data() -> Value {
    json!({ "sections": [], "preferences": {} })
}

fn ui_preferences() -> Value {
    // 对齐 src-tauri::commands::settings::UiPreferences 的 camelCase 文件契约。
    json!({
        "theme": "light",
        "accentColor": "ocean",
        "customAccentHex": "",
        "reduceMotion": false,
        "androidGlass": "local",
        "language": "en-US",
        "hasSeenOnboarding": true,
        "notificationPermissionRequested": true
    })
}

fn manifest(count: usize, kdf: KdfConfig, build_profile: &str) -> Value {
    json!({
        "schemaVersion": 1,
        "scope": "synthetic-native-vault-fixture",
        "generator": "solosoul-core/examples/perf_baseline",
        "fixture": "deterministic-20th-object-property-match",
        "determinism": "object-content-only; salt, encryption nonce and account/profile timestamps vary",
        "accountId": format!("acc_rf312_{count}"),
        "accountName": ACCOUNT_NAME,
        "objectCount": count,
        "searchQuery": "needle",
        "expectedSearchMatches": count.div_ceil(20),
        "buildProfile": build_profile,
        "kdf": {
            "memoryKiB": kdf.memory_kb,
            "iterations": kdf.iterations,
            "parallelism": kdf.parallelism
        },
        "includesProfile": true,
        "includesUiPreferences": true,
        "includesAttachments": false,
        "includesOcrFixture": false
    })
}

fn create_output(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err("--fixture-output requires an absolute new directory".into());
    }
    let name = path
        .file_name()
        .ok_or_else(|| "fixture output must have a directory name".to_string())?;
    let parent = path
        .parent()
        .ok_or_else(|| "fixture output must have an existing parent".to_string())?
        .canonicalize()
        .map_err(|error| format!("cannot resolve fixture parent: {error}"))?;
    if !parent.is_dir() {
        return Err("fixture output parent must be a directory".into());
    }
    let output = parent.join(name);
    // create_dir 原子拒绝既有文件、目录及链接；失败后不删除或接纳原路径。
    std::fs::create_dir(&output)
        .map_err(|error| format!("cannot create new fixture directory: {error}"))?;
    Ok(output)
}

fn write_new_json(path: &Path, value: &Value) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "fixture JSON requires a parent directory".to_string())?;
    let mut staged = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut staged, value).map_err(|e| e.to_string())?;
    staged.write_all(b"\n").map_err(|e| e.to_string())?;
    staged.as_file().sync_all().map_err(|e| e.to_string())?;
    staged.persist_noclobber(path).map_err(|e| e.to_string())?;
    Ok(())
}

fn generate(path: &Path, count: usize) -> Result<Value, String> {
    let output = create_output(path)?;
    let account_id = format!("acc_rf312_{count}");
    let service = VaultService::with_base_path(output.clone());
    service.create_account_with_id(&account_id, ACCOUNT_NAME, PASSWORD, None)?;
    let vault = service
        .get_vault_store()
        .ok_or_else(|| "fixture account creation did not unlock vault".to_string())?;
    for index in 0..count {
        vault.save_object(&make_object(&account_id, index))?;
    }
    vault.save_profile(&Profile::new_with_id(
        &account_id,
        ACCOUNT_NAME,
        serde_json::to_vec(&profile_data()).map_err(|e| e.to_string())?,
    ))?;
    drop(vault);
    service.lock();
    drop(service);
    write_new_json(&output.join("ui_preferences.json"), &ui_preferences())?;
    let marker = manifest(
        count,
        KdfConfig::from_env(),
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
    );
    let checks = verify_data(&output, &marker)?;
    // 完成标记最后原子发布；错误时保留新目录供检查，不删除任意输入路径。
    write_new_json(&output.join(MARKER_NAME), &marker)?;
    Ok(json!({
        "scope": "synthetic-fixture-generation",
        "fixturePath": output,
        "manifest": marker,
        "verification": checks,
        "success": true
    }))
}

fn require_path(path: &Path, directory: bool) -> Result<(), String> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| format!("missing fixture path {}: {error}", path.display()))?;
    let is_link = metadata.file_type().is_symlink();
    #[cfg(windows)]
    let is_link = {
        use std::os::windows::fs::MetadataExt;
        // Windows junction 等 reparse point 同样不作为可验证的普通 fixture 文件。
        is_link || metadata.file_attributes() & 0x400 != 0
    };
    if is_link || (directory && !metadata.is_dir()) || (!directory && !metadata.is_file()) {
        return Err(format!(
            "fixture path is not a regular {}: {}",
            if directory { "directory" } else { "file" },
            path.display()
        ));
    }
    Ok(())
}

fn read_json(path: &Path) -> Result<Value, String> {
    require_path(path, false)?;
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes).map_err(|e| format!("invalid fixture JSON: {e}"))
}

fn verify(path: &Path) -> Result<Value, String> {
    if !path.is_absolute() {
        return Err("--verify-fixture requires an absolute fixture directory".into());
    }
    require_path(path, true)?;
    let input = path.canonicalize().map_err(|e| e.to_string())?;
    let marker = read_json(&input.join(MARKER_NAME))?;
    let mut checks = verify_data(&input, &marker)?;
    checks["fixturePath"] = json!(input);
    Ok(checks)
}

fn verify_data(base: &Path, marker: &Value) -> Result<Value, String> {
    let count = marker["objectCount"]
        .as_u64()
        .and_then(|count| usize::try_from(count).ok())
        .filter(|count| matches!(count, 100 | 5000))
        .ok_or_else(|| "invalid fixture object count".to_string())?;
    let account_id = format!("acc_rf312_{count}");
    let account_dir = base.join(&account_id);
    require_path(&account_dir, true)?;
    require_path(&account_dir.join("vault.db"), false)?;
    let config: AccountConfig =
        serde_json::from_value(read_json(&account_dir.join("config.json"))?)
            .map_err(|e| format!("invalid fixture account config: {e}"))?;
    let kdf = config.kdf_config();
    if config.account_id != account_id
        || config.name != ACCOUNT_NAME
        || config.crypto_version != 3
        || config.biometric_enabled
        || config.pin_enabled
        || (kdf != KdfConfig::development() && kdf != KdfConfig::production())
    {
        return Err("fixture account config does not match synthetic fixture".into());
    }
    let build_profile = marker["buildProfile"]
        .as_str()
        .filter(|profile| matches!(*profile, "debug" | "release"))
        .ok_or_else(|| "invalid fixture build profile".to_string())?;
    if *marker != manifest(count, kdf, build_profile) {
        return Err("fixture completion marker does not match fixture contract".into());
    }
    if !cfg!(debug_assertions) && kdf != KdfConfig::production() {
        return Err(
            "release verifier requires a production-KDF fixture; refusing implicit upgrade".into(),
        );
    }
    let accounts = read_json(&base.join("accounts.json"))?;
    if accounts.as_array().map(Vec::len) != Some(1) || accounts[0]["id"] != account_id {
        return Err("fixture account manifest must contain exactly the synthetic account".into());
    }
    if read_json(&base.join("ui_preferences.json"))? != ui_preferences() {
        return Err("fixture UI preferences do not match GUI fixture contract".into());
    }
    let service = VaultService::with_base_path(base.to_path_buf());
    service.load_accounts();
    let accounts = service.list_accounts();
    if accounts.len() != 1 || accounts[0].id != account_id || accounts[0].name != ACCOUNT_NAME {
        return Err("fixture account catalog could not be reopened".into());
    }
    service.unlock(&account_id, PASSWORD)?;
    let vault = service
        .get_vault_store()
        .ok_or_else(|| "fixture could not reopen vault".to_string())?;
    let objects = vault.list_object_metadata(&account_id, None, None, false, false)?;
    let ids: HashSet<_> = objects.iter().map(|record| record.id.as_str()).collect();
    if objects.len() != count
        || !(0..count).all(|index| ids.contains(format!("obj_perf_{index:08}").as_str()))
    {
        return Err(format!(
            "fixture object IDs/count mismatch; expected {count}, got {}",
            objects.len()
        ));
    }
    let matches = vault.search_objects(&account_id, "needle")?;
    let match_ids: HashSet<_> = matches.iter().map(|record| record.id.as_str()).collect();
    if matches.len() != count.div_ceil(20)
        || !(0..count)
            .step_by(20)
            .all(|index| match_ids.contains(format!("obj_perf_{index:08}").as_str()))
    {
        return Err("fixture needle matches do not match deterministic object IDs/count".into());
    }
    let profile = vault
        .load_profile(&account_id)?
        .ok_or("fixture Profile is missing")?;
    let data: Value = serde_json::from_slice(&profile.data).map_err(|e| e.to_string())?;
    if profile.id != account_id || profile.name != ACCOUNT_NAME || data != profile_data() {
        return Err("fixture Profile does not match GUI fixture contract".into());
    }
    drop(vault);
    service.lock();
    Ok(json!({
        "scope": "synthetic-fixture-reopen-verification",
        "accountId": account_id,
        "accountCount": accounts.len(),
        "objectCount": objects.len(),
        "searchMatches": matches.len(),
        "objectIdsVerified": true,
        "searchResultIdsVerified": true,
        "profileVerified": true,
        "uiPreferencesVerified": true,
        "success": true
    }))
}
