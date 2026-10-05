//! 固定合成 fixture 契约；源仅仅读，解锁只发生在新拷贝。
use super::{read_json, require_regular, sha256_file};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use solosoul_core::{vault_service::AccountConfig, VaultService};
use solosoul_crypto::KdfConfig;
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

const PASSWORD: &str = "perf-baseline-only-password";
const ACCOUNT_NAME: &str = "Performance Fixture";
const MARKER: &str = "rf312-fixture.json";

pub(super) struct Contract {
    count: usize,
    account_id: String,
    marker: Value,
    files: Vec<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct FileProof {
    relative_path: PathBuf,
    sha256: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Proof {
    pub account_id: String,
    pub object_count: usize,
    search_matches: usize,
    marker: Value,
    files: Vec<FileProof>,
}

fn ui_preferences() -> Value {
    json!({
        "theme": "light", "accentColor": "ocean", "customAccentHex": "",
        "reduceMotion": false, "androidGlass": "local", "language": "en-US",
        "hasSeenOnboarding": true, "notificationPermissionRequested": true
    })
}

fn marker(count: usize, build_profile: &str) -> Value {
    let kdf = KdfConfig::production();
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
        "kdf": { "memoryKiB": kdf.memory_kb, "iterations": kdf.iterations, "parallelism": kdf.parallelism },
        "includesProfile": true,
        "includesUiPreferences": true,
        "includesAttachments": false,
        "includesOcrFixture": false
    })
}

// 启动后只承认正常更新器写入的公开缓存；固定的八项外观/引导偏好仍逐项相等。
fn check_startup_preferences(value: &Value) -> Result<(), String> {
    let mut actual = value.clone();
    let cache = actual
        .as_object_mut()
        .ok_or("invalid startup UI preferences")?
        .remove("updateSources");
    if actual != ui_preferences() {
        return Err("startup changed fixed UI preferences".into());
    }
    let Some(cache) = cache else {
        return Ok(());
    };
    let sources = crate::commands::update::native_perf_cache_candidates()?;
    if !cache.as_object().is_some_and(|m| {
        m.len() == 3
            && ["manifest", "release", "lastChannel"]
                .iter()
                .all(|key| m.contains_key(*key))
    }) || !["manifest", "release"].contains(&cache["lastChannel"].as_str().unwrap_or(""))
        || cache[cache["lastChannel"].as_str().unwrap()].is_null()
    {
        return Err("invalid startup update-source cache".into());
    }
    for channel in ["manifest", "release"] {
        let slot = &cache[channel];
        if slot.is_null() {
            continue;
        }
        if slot.as_object().map(|m| m.len()) != Some(2)
            || slot["url"].as_str().is_none()
            || !sources[channel]
                .as_array()
                .is_some_and(|urls| urls.contains(&slot["url"]))
            || !slot["probedAt"]
                .as_i64()
                .is_some_and(|at| at > 0 && at <= chrono::Utc::now().timestamp())
        {
            return Err("startup update-source cache is not an allowed bounded source".into());
        }
    }
    Ok(())
}

pub(super) fn check_contract(base: &Path) -> Result<Contract, String> {
    check_contract_mode(base, false)
}

fn check_contract_mode(base: &Path, startup: bool) -> Result<Contract, String> {
    require_regular(base, true)?;
    let value = read_json(&base.join(MARKER))?;
    let count = value["objectCount"]
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())
        .filter(|value| matches!(value, 100 | 5000))
        .ok_or("native-perf fixture count must be 100 or 5000")?;
    let build_profile = value["buildProfile"]
        .as_str()
        .filter(|value| matches!(*value, "release" | "debug"))
        .ok_or("native-perf fixture build profile is invalid")?;
    if value != marker(count, build_profile) {
        return Err(
            "native-perf fixture completion marker does not match the fixed schema/scope".into(),
        );
    }
    let account_id = format!("acc_rf312_{count}");
    let account_dir = base.join(&account_id);
    require_regular(&account_dir, true)?;
    let config: AccountConfig =
        serde_json::from_value(read_json(&account_dir.join("config.json"))?)
            .map_err(|e| format!("invalid synthetic account config: {e}"))?;
    let kdf = KdfConfig::production();
    if config.account_id != account_id
        || config.name != ACCOUNT_NAME
        || config.crypto_version != 3
        || config.biometric_enabled
        || config.pin_enabled
        || config.password_hint.is_some()
        || config.kdf_memory_kb != Some(kdf.memory_kb)
        || config.kdf_iterations != Some(kdf.iterations)
        || config.kdf_parallelism != Some(kdf.parallelism)
    {
        return Err(
            "native-perf requires the fixed synthetic account and explicit production KDF".into(),
        );
    }
    let accounts = read_json(&base.join("accounts.json"))?;
    if accounts.as_array().map(Vec::len) != Some(1)
        || accounts[0]["id"] != account_id
        || accounts[0]["name"] != ACCOUNT_NAME
    {
        return Err("native-perf fixture account catalog/UI preferences mismatch".into());
    }
    let preferences = read_json(&base.join("ui_preferences.json"))?;
    if startup {
        check_startup_preferences(&preferences)?;
    } else if preferences != ui_preferences() {
        return Err("native-perf fixture UI preferences mismatch".into());
    }
    let files = closed_files(base, &account_id)?;
    Ok(Contract {
        count,
        account_id,
        marker: value,
        files,
    })
}

fn closed_files(base: &Path, account_id: &str) -> Result<Vec<PathBuf>, String> {
    let mut files = vec![];
    for entry in fs::read_dir(base).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        // unlock 保存账户目录时，原子写会留下 accounts.bak；它不参与
        // 当前合成数据口径，也不进入副本或 proof。其余未知条目继续拒绝。
        if name == "accounts.bak" || name == ".lock" {
            // RF905 锁文件只属于当前 root，不能进入副本或数据 proof。
            require_regular(&entry.path(), false)?;
            continue;
        }
        if name == account_id {
            require_regular(&entry.path(), true)?;
            continue;
        }
        if ![MARKER, "accounts.json", "ui_preferences.json"]
            .iter()
            .any(|expected| name == *expected)
        {
            return Err("native-perf fixture contains an unexpected root entry".into());
        }
        require_regular(&entry.path(), false)?;
        files.push(PathBuf::from(name));
    }
    for entry in fs::read_dir(base.join(account_id)).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        // 新建账户的数据库迁移会留下 pre_enc 备份；GUI 副本已加密，
        // 不需要旧备份。只承认此普通文件的存在，不读取/拷贝其内容。
        if name == "vault.db.pre_enc.bak" {
            require_regular(&entry.path(), false)?;
            continue;
        }
        if !["config.json", "vault.db", "vault.db-wal", "vault.db-shm"]
            .iter()
            .any(|expected| name == *expected)
        {
            return Err("native-perf fixture contains an unexpected account entry".into());
        }
        require_regular(&entry.path(), false)?;
        if entry.metadata().map_err(|e| e.to_string())?.len() > 256 * 1024 * 1024 {
            return Err("native-perf fixture file exceeds 256 MiB".into());
        }
        files.push(PathBuf::from(account_id).join(name));
    }
    require_regular(&base.join(account_id).join("vault.db"), false)?;
    files.sort();
    Ok(files)
}

pub(super) fn copy_closed_fixture(
    source: &Path,
    output: &Path,
    contract: &Contract,
) -> Result<(), String> {
    fs::create_dir(output.join(&contract.account_id)).map_err(|e| e.to_string())?;
    for relative in &contract.files {
        let input = source.join(relative);
        require_regular(&input, false)?;
        let mut reader = File::open(&input).map_err(|e| e.to_string())?;
        let mut writer = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output.join(relative))
            .map_err(|e| e.to_string())?;
        std::io::copy(&mut reader, &mut writer).map_err(|e| e.to_string())?;
        writer.sync_all().map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn file_proofs(base: &Path, account_id: &str) -> Result<Vec<FileProof>, String> {
    closed_files(base, account_id)?
        .into_iter()
        .map(|relative_path| {
            Ok(FileProof {
                sha256: sha256_file(&base.join(&relative_path))?,
                relative_path,
            })
        })
        .collect()
}

pub(super) fn verify_copy(base: &Path, contract: &Contract) -> Result<Proof, String> {
    let copy = check_contract(base)?;
    if copy.marker != contract.marker {
        return Err("synthetic fixture changed during preparation".into());
    }
    let service = VaultService::try_with_base_path(base.to_path_buf())?;
    service.load_accounts();
    let accounts = service.list_accounts();
    if accounts.len() != 1
        || accounts[0].id != contract.account_id
        || accounts[0].name != ACCOUNT_NAME
    {
        return Err("synthetic account could not be reopened".into());
    }
    service.unlock(&contract.account_id, PASSWORD)?;
    let vault = service
        .get_vault_store()
        .ok_or("synthetic vault did not unlock")?;
    let objects = vault.list_object_metadata(&contract.account_id, None, None, false, false)?;
    let ids: HashSet<_> = objects.iter().map(|object| object.id.as_str()).collect();
    if objects.len() != contract.count
        || !(0..contract.count).all(|index| ids.contains(format!("obj_perf_{index:08}").as_str()))
    {
        return Err("synthetic object IDs/count mismatch".into());
    }
    let results = vault.search_objects(&contract.account_id, "needle")?;
    let result_ids: HashSet<_> = results.iter().map(|object| object.id.as_str()).collect();
    if results.len() != contract.count.div_ceil(20)
        || !(0..contract.count)
            .step_by(20)
            .all(|index| result_ids.contains(format!("obj_perf_{index:08}").as_str()))
    {
        return Err("synthetic needle matches mismatch".into());
    }
    let profile = vault
        .load_profile(&contract.account_id)?
        .ok_or("synthetic Profile is missing")?;
    let data: Value = serde_json::from_slice(&profile.data).map_err(|e| e.to_string())?;
    if profile.id != contract.account_id
        || profile.name != ACCOUNT_NAME
        || data != json!({ "sections": [], "preferences": {} })
    {
        return Err("synthetic Profile mismatch".into());
    }
    drop(vault);
    service.lock();
    drop(service);
    Ok(Proof {
        account_id: contract.account_id.clone(),
        object_count: objects.len(),
        search_matches: results.len(),
        marker: contract.marker.clone(),
        files: file_proofs(base, &contract.account_id)?,
    })
}

pub(super) fn check_proof(base: &Path, proof: &Proof, contract: &Contract) -> Result<(), String> {
    check_proof_mode(base, proof, contract, false)
}

pub(super) fn check_startup_proof(base: &Path, proof: &Proof) -> Result<(), String> {
    let contract = check_contract_mode(base, true)?;
    check_proof_mode(base, proof, &contract, true)
}

fn check_proof_mode(
    base: &Path,
    proof: &Proof,
    contract: &Contract,
    startup: bool,
) -> Result<(), String> {
    let current = file_proofs(base, &contract.account_id)?;
    let files_match = proof.files.len() == current.len()
        && proof.files.iter().zip(&current).all(|(original, actual)| {
            original.relative_path == actual.relative_path
                && (original.sha256 == actual.sha256
                    || (startup && actual.relative_path == Path::new("ui_preferences.json")))
        });
    if proof.account_id != contract.account_id
        || proof.object_count != contract.count
        || proof.search_matches != contract.count.div_ceil(20)
        || proof.marker != contract.marker
        || !files_match
    {
        return Err(
            "prepared synthetic fixture differs from the independently verified copy".into(),
        );
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn write_test_fixture(base: &Path) {
    use solosoul_vault::{ObjectRecord, Profile};
    // 测试调用方持有 VAULT_TEST_LOCK；只临时修改本测试进程的 KDF 选择。
    let previous = std::env::var_os("SOLOSOUL_SECURE");
    std::env::set_var("SOLOSOUL_SECURE", "1");
    let service = VaultService::with_base_path(base.to_path_buf());
    service
        .create_account_with_id("acc_rf312_100", ACCOUNT_NAME, PASSWORD, None)
        .unwrap();
    match previous {
        Some(value) => std::env::set_var("SOLOSOUL_SECURE", value),
        None => std::env::remove_var("SOLOSOUL_SECURE"),
    }
    let vault = service.get_vault_store().unwrap();
    for index in 0..100 {
        let object = ObjectRecord {
            id: format!("obj_perf_{index:08}"),
            account_id: "acc_rf312_100".into(),
            type_id: "note".into(),
            section_type: "identity".into(),
            name: format!("Record {index:08}"),
            properties: json!({ "body": if index % 20 == 0 { "needle" } else { "haystack" } }),
            sensitivity_level: "internal".into(),
            version: 1,
            created_at: "2026-09-28T00:00:00Z".into(),
            updated_at: "2026-09-28T00:00:00Z".into(),
            ..Default::default()
        };
        vault.save_object(&object).unwrap();
    }
    vault
        .save_profile(&Profile::new_with_id(
            "acc_rf312_100",
            ACCOUNT_NAME,
            serde_json::to_vec(&json!({ "sections": [], "preferences": {} })).unwrap(),
        ))
        .unwrap();
    drop(vault);
    service.lock();
    drop(service);
    super::write_new_json(&base.join("ui_preferences.json"), &ui_preferences()).unwrap();
    super::write_new_json(&base.join(MARKER), &marker(100, "debug")).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn startup_cache_preserves_preferences_and_rejects_unknown_sources_and_fields() {
        let _guard = crate::VAULT_TEST_LOCK.lock().unwrap();
        let sources = crate::commands::update::native_perf_cache_candidates().unwrap();
        let url = sources["manifest"][0].clone();
        let mut valid = ui_preferences();
        valid["updateSources"] = json!({"manifest":{"url":url,"probedAt":chrono::Utc::now().timestamp()},"release":null,"lastChannel":"manifest"});
        assert!(check_startup_preferences(&ui_preferences()).is_ok());
        assert!(check_startup_preferences(&valid).is_ok());
        let mut bad = valid.clone();
        bad["theme"] = json!("dark");
        assert!(check_startup_preferences(&bad).is_err());
        let mut bad = valid.clone();
        bad["updateSources"]["manifest"]["url"] = json!("https://unapproved.invalid/latest.json");
        assert!(check_startup_preferences(&bad).is_err());
        let mut bad = valid.clone();
        bad["updateSources"]["manifest"]["probedAt"] = json!(chrono::Utc::now().timestamp() + 3600);
        assert!(check_startup_preferences(&bad).is_err());
        let mut bad = valid.clone();
        bad["updateSources"]["manifest"]["private"] = json!("sentinel");
        assert!(check_startup_preferences(&bad).is_err());
        let mut bad = valid.clone();
        bad["updateSources"]
            .as_object_mut()
            .unwrap()
            .remove("release");
        bad["updateSources"]["private"] = Value::Null;
        assert!(check_startup_preferences(&bad).is_err());
        let mut bad = valid.clone();
        bad["updateSources"]["lastChannel"] = json!("release");
        assert!(check_startup_preferences(&bad).is_err());
        let mut bad = valid;
        bad["unknown"] = Value::Null;
        assert!(check_startup_preferences(&bad).is_err());
    }
}
