use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use tauri::State;
use zip::write::SimpleFileOptions;
use zip::ZipArchive;
use zip::ZipWriter;

// ── Export types ──

#[derive(Serialize)]
pub struct ExportScopeNode {
    pub id: String,
    pub name: String,
    pub node_type: String,
    pub children: Vec<ExportScopeNode>,
    pub item_count: usize,
    pub attachment_count: usize,
}

#[derive(Serialize)]
pub struct ExportEstimate {
    pub object_count: usize,
    pub estimated_bytes: u64,
}

#[derive(Deserialize)]
pub struct ExportRequest {
    pub profile_ids: Vec<String>,
    pub include_preferences: bool,
    pub include_audit_log: bool,
    pub password: String,
    pub save_path: String,
}

// ── Import types ──

#[derive(Serialize)]
pub struct ImportPreview {
    pub file_path: String,
    pub export_time: Option<String>,
    pub profile_count: usize,
    pub profile_names: Vec<String>,
}

// ── Export commands ──

#[tauri::command]
pub async fn export_get_scope_tree(
    state: State<'_, AppState>,
) -> Result<Vec<ExportScopeNode>, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
    let profiles = vault.list_profiles().map_err(|e| e.to_string())?;
    Ok(profiles
        .iter()
        .map(|p| ExportScopeNode {
            id: p.id.clone(),
            name: p.name.clone(),
            node_type: "profile".to_string(),
            children: vec![],
            item_count: 1,
            attachment_count: 0,
        })
        .collect())
}

#[tauri::command]
pub async fn export_estimate_size(
    state: State<'_, AppState>,
    profile_ids: Vec<String>,
) -> Result<ExportEstimate, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
    let mut total = 0u64;
    let mut count = 0;
    for id in &profile_ids {
        if let Ok(Some(p)) = vault.load_profile(id) {
            total += p.data.len() as u64;
            count += 1;
        }
    }
    Ok(ExportEstimate {
        object_count: count,
        estimated_bytes: total,
    })
}

#[tauri::command]
pub async fn export_execute(
    state: State<'_, AppState>,
    req: ExportRequest,
) -> Result<String, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    let zip_path = if req.save_path.ends_with(".solosoul") {
        req.save_path.clone()
    } else {
        format!("{}.solosoul", req.save_path)
    };

    let file = File::create(&zip_path).map_err(|e| format!("Create ZIP: {}", e))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let manifest = serde_json::json!({
        "version": "2.0",
        "export_time": chrono::Utc::now().to_rfc3339(),
        "profile_count": req.profile_ids.len(),
    });
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
    zip.start_file("manifest.json", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(&manifest_bytes).map_err(|e| e.to_string())?;

    for id in &req.profile_ids {
        if let Ok(Some(profile)) = vault.load_profile(id) {
            let filename = format!("profiles/{}.enc", id);
            zip.start_file(&filename, options)
                .map_err(|e| e.to_string())?;
            zip.write_all(&profile.data).map_err(|e| e.to_string())?;
        }
    }

    zip.finish().map_err(|e| format!("ZIP finish: {}", e))?;
    Ok(zip_path)
}

// ── Import commands ──

/// Read the manifest from a .solosoul file and return a preview of what's inside
#[tauri::command]
pub async fn import_preview_package(file_path: String) -> Result<ImportPreview, String> {
    let path = Path::new(&file_path);
    let file = File::open(path).map_err(|e| format!("Open: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("Read ZIP: {}", e))?;

    // Read manifest
    let mut manifest_str = String::new();
    let mut got_manifest = false;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();

        if name == "manifest.json" {
            entry
                .read_to_string(&mut manifest_str)
                .map_err(|e| e.to_string())?;
            got_manifest = true;
            break;
        }
    }

    if !got_manifest {
        return Err("Invalid .solosoul file: no manifest.json found".to_string());
    }

    let manifest: serde_json::Value =
        serde_json::from_str(&manifest_str).map_err(|e| format!("Invalid manifest: {}", e))?;

    let export_time = manifest
        .get("export_time")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let profile_count = manifest
        .get("profile_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    // Collect profile names from the archive
    let mut profile_names = Vec::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        if name.starts_with("profiles/") && name.ends_with(".enc") {
            // Extract profile ID from filename
            let id = name
                .trim_start_matches("profiles/")
                .trim_end_matches(".enc");
            profile_names.push(id.to_string());
        }
    }

    Ok(ImportPreview {
        file_path,
        export_time,
        profile_count,
        profile_names,
    })
}

/// Execute import of profiles from a .solosoul file
#[tauri::command]
pub async fn import_execute(
    state: State<'_, AppState>,
    file_path: String,
    password: String,
) -> Result<usize, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    if password.is_empty() {
        return Err("Password is required".to_string());
    }

    let file = File::open(&file_path).map_err(|e| format!("Open ZIP: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("Read ZIP: {}", e))?;

    let mut imported = 0usize;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();

        if name.starts_with("profiles/") && name.ends_with(".enc") {
            let profile_id = name
                .trim_start_matches("profiles/")
                .trim_end_matches(".enc")
                .to_string();
            let mut data = Vec::new();
            entry.read_to_end(&mut data).map_err(|e| e.to_string())?;

            // Don't overwrite existing profiles
            if vault
                .load_profile(&profile_id)
                .map_err(|e| e.to_string())?
                .is_some()
            {
                continue;
            }

            let profile = solosoul_vault::Profile::new_with_id(&profile_id, &profile_id, data);
            vault.save_profile(&profile).map_err(|e| e.to_string())?;
            imported += 1;
        }
    }

    Ok(imported)
}
