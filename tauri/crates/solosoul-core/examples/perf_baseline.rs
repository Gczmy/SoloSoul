//! RF-312: 隔离临时 Vault 的原生后端基线；不访问用户数据目录。
//! cargo run -p solosoul-core --release --example perf_baseline -- --objects 100 --samples 10
//! --fixture-output <绝对新目录> 只准备持久化数据；--verify-fixture <目录> 独立重开验证。

#[path = "perf_baseline/fixture.rs"]
mod fixture;

use solosoul_core::VaultService;
use solosoul_vault::ObjectRecord;
use std::time::Instant;

const PASSWORD: &str = "perf-baseline-only-password";

fn arg_usize(name: &str, default: usize) -> Result<usize, String> {
    let args: Vec<String> = std::env::args().collect();
    match args.windows(2).find(|pair| pair[0] == name) {
        Some(pair) => pair[1]
            .parse::<usize>()
            .map_err(|_| format!("{name} must be an integer"))
            .and_then(|value| {
                if value == 0 {
                    Err(format!("{name} must be positive"))
                } else {
                    Ok(value)
                }
            }),
        None => Ok(default),
    }
}

fn make_object(account_id: &str, index: usize) -> ObjectRecord {
    let needle = if index.is_multiple_of(20) {
        "needle"
    } else {
        "haystack"
    };
    let now = "2026-09-28T00:00:00Z".to_string();
    ObjectRecord {
        id: format!("obj_perf_{index:08}"),
        account_id: account_id.to_string(),
        type_id: "note".to_string(),
        section_type: "identity".to_string(),
        name: format!("Record {index:08}"),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({
            "title": format!("Synthetic {index:08}"),
            "body": format!("{needle} reproducible vault performance sample {index:08}"),
            "category": format!("group-{}", index % 10),
            "fields": ["alpha", "beta", "gamma", "delta"]
        }),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: None,
        template_type: None,
        contract_type_id: None,
        template_hash: None,
        ignored_template_hash: None,
        created_at: now.clone(),
        updated_at: now,
        version: 1,
    }
}

fn percentiles(samples: &[f64]) -> serde_json::Value {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let middle = sorted.len() / 2;
    let median = if sorted.len().is_multiple_of(2) {
        (sorted[middle - 1] + sorted[middle]) / 2.0
    } else {
        sorted[middle]
    };
    let p95_index = ((sorted.len() as f64 * 0.95).ceil() as usize - 1).min(sorted.len() - 1);
    serde_json::json!({
        "medianMs": median,
        "p95Ms": sorted[p95_index],
        "minMs": sorted[0],
        "maxMs": sorted[sorted.len() - 1]
    })
}

fn measure<F>(samples: usize, mut operation: F) -> Result<serde_json::Value, String>
where
    F: FnMut() -> Result<usize, String>,
{
    let mut timings = Vec::with_capacity(samples);
    let mut observations = Vec::with_capacity(samples);
    for _ in 0..samples {
        let start = Instant::now();
        let outcome = operation();
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        match outcome {
            Ok(count) => {
                timings.push(elapsed_ms);
                observations.push(serde_json::json!({"ms": elapsed_ms, "resultCount": count}));
            }
            Err(error) => {
                observations.push(serde_json::json!({"ms": elapsed_ms, "error": error}));
            }
        }
    }
    let mut summary = if timings.is_empty() {
        serde_json::json!({})
    } else {
        percentiles(&timings)
    };
    summary["failureCount"] = serde_json::json!(samples - timings.len());
    summary["samples"] = serde_json::json!(observations);
    Ok(summary)
}

fn run() -> Result<serde_json::Value, String> {
    if let Some(result) = fixture::run_if_requested() {
        return result;
    }
    let object_count = arg_usize("--objects", 100)?;
    let sample_count = arg_usize("--samples", 10)?;
    let kdf = solosoul_crypto::KdfConfig::from_env();
    let temp = tempfile::tempdir().map_err(|error| error.to_string())?;
    let service = VaultService::with_base_path(temp.path().to_path_buf());
    let account = service.create_account("Performance Fixture", PASSWORD, None)?;
    let account_id = account["id"]
        .as_str()
        .ok_or_else(|| "created account has no id".to_string())?
        .to_string();
    let vault = service
        .get_vault_store()
        .ok_or_else(|| "vault is not unlocked after account creation".to_string())?;

    for index in 0..object_count {
        vault.save_object(&make_object(&account_id, index))?;
    }
    drop(vault);

    // 预热文件与 SQLite 缓存；之后每次重新锁定，包含真实 KDF + Vault 打开。
    service.lock();
    service.unlock(&account_id, PASSWORD)?;
    let unlock = measure(sample_count, || {
        service.lock();
        service.unlock(&account_id, PASSWORD)?;
        Ok(1)
    })?;
    let vault = service
        .get_vault_store()
        .ok_or_else(|| "vault is not unlocked after unlock".to_string())?;

    let account_catalog = measure(sample_count, || {
        let cold_service = VaultService::with_base_path(temp.path().to_path_buf());
        cold_service.load_accounts();
        Ok(cold_service.list_accounts().len())
    })?;

    vault.list_object_metadata(&account_id, None, None, false, false)?;
    let list = measure(sample_count, || {
        vault
            .list_object_metadata(&account_id, None, None, false, false)
            .and_then(|items| {
                if items.len() == object_count {
                    Ok(items.len())
                } else {
                    Err(format!(
                        "expected {object_count} objects, got {}",
                        items.len()
                    ))
                }
            })
    })?;
    vault.search_objects(&account_id, "needle")?;
    let expected_matches = object_count.div_ceil(20);
    let search = measure(sample_count, || {
        vault
            .search_objects(&account_id, "needle")
            .and_then(|items| {
                if items.len() == expected_matches {
                    Ok(items.len())
                } else {
                    Err(format!(
                        "expected {expected_matches} matches, got {}",
                        items.len()
                    ))
                }
            })
    })?;

    Ok(serde_json::json!({
        "schemaVersion": 1,
        "scope": "native-vault-backend-only",
        "platform": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "buildProfile": if cfg!(debug_assertions) { "debug" } else { "release" },
        "kdf": {
            "memoryKiB": kdf.memory_kb,
            "iterations": kdf.iterations,
            "parallelism": kdf.parallelism
        },
        "objectCount": object_count,
        "sampleCount": sample_count,
        "fixture": "deterministic-20th-object-property-match",
        "accountCatalogLoad": account_catalog,
        "unlock": unlock,
        "listMetadata": list,
        "searchDecrypted": search,
    }))
}

fn main() {
    match run() {
        Ok(result) => {
            println!("{result}");
            if result["scope"] == "native-vault-backend-only"
                && [
                    "accountCatalogLoad",
                    "unlock",
                    "listMetadata",
                    "searchDecrypted",
                ]
                .iter()
                .any(|key| result[key]["failureCount"].as_u64().unwrap_or(1) > 0)
            {
                std::process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("perf baseline failed: {error}");
            std::process::exit(1);
        }
    }
}
