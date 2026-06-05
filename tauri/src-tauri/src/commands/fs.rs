use crate::state::AppState;
use std::fs::{self as fs_std, File};
use std::io::{Read, Write};
use tauri::State;

/// Encrypt a file using chunked AES-256-GCM (SOLO blob v3)
#[tauri::command]
pub async fn encrypt_file(
    state: State<'_, AppState>,
    src_path: String,
    dst_path: String,
) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let session_key = svc.get_session_key().ok_or("Vault not unlocked")?;
    let key: [u8; 32] = session_key
        .as_slice()
        .try_into()
        .map_err(|_| "Invalid key")?;

    let chunk_size = 1024 * 1024; // 1MB chunks
    let src = fs_std::read(&src_path).map_err(|e| format!("Read failed: {}", e))?;
    let blob = solosoul_crypto::aes::encrypt_chunked_blob(&key, &src, chunk_size)
        .map_err(|e| format!("Encryption failed: {}", e))?;
    fs_std::write(&dst_path, &blob).map_err(|e| format!("Write failed: {}", e))?;
    Ok(())
}

/// Decrypt a SOLO blob file
#[tauri::command]
pub async fn decrypt_file(
    state: State<'_, AppState>,
    src_path: String,
    dst_path: String,
) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let session_key = svc.get_session_key().ok_or("Vault not unlocked")?;
    let key: [u8; 32] = session_key
        .as_slice()
        .try_into()
        .map_err(|_| "Invalid key")?;

    let blob = fs_std::read(&src_path).map_err(|e| format!("Read failed: {}", e))?;

    // Detect format
    let plaintext = if blob.len() >= 5 && &blob[0..4] == b"SOLO" && blob[4] == 0x03 {
        solosoul_crypto::aes::decrypt_chunked_blob(&key, &blob)
            .map_err(|e| format!("Decryption failed: {}", e))?
    } else {
        solosoul_crypto::aes::decrypt_blob(&key, &blob)
            .map_err(|e| format!("Decryption failed: {}", e))?
    };

    fs_std::write(&dst_path, &plaintext).map_err(|e| format!("Write failed: {}", e))?;
    Ok(())
}

/// Create a ZIP package from a directory
#[tauri::command]
pub async fn create_zip_package(src_dir: String, dst_path: String) -> Result<(), String> {
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    let file = File::create(&dst_path).map_err(|e| format!("Failed to create ZIP: {}", e))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let walkdir = walkdir::WalkDir::new(&src_dir);
    let mut buffer = Vec::new();

    for entry in walkdir.into_iter() {
        let entry = entry.map_err(|e| format!("WalkDir error: {}", e))?;
        let path = entry.path();
        let name = path
            .strip_prefix(&src_dir)
            .map_err(|e| format!("Path error: {}", e))?;

        if path.is_file() {
            let mut f = File::open(path).map_err(|e| format!("Open file error: {}", e))?;
            f.read_to_end(&mut buffer)
                .map_err(|e| format!("Read error: {}", e))?;
            zip.start_file_from_path(name, options)
                .map_err(|e| format!("ZIP start_file error: {}", e))?;
            zip.write_all(&buffer)
                .map_err(|e| format!("ZIP write error: {}", e))?;
            buffer.clear();
        }
    }
    zip.finish()
        .map_err(|e| format!("ZIP finish error: {}", e))?;
    Ok(())
}

/// Extract a ZIP package to a directory
#[tauri::command]
pub async fn extract_zip_package(zip_path: String, dst_dir: String) -> Result<Vec<String>, String> {
    use std::io::copy;
    use zip::ZipArchive;

    let file = File::open(&zip_path).map_err(|e| format!("Failed to open ZIP: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("Failed to read ZIP: {}", e))?;

    fs_std::create_dir_all(&dst_dir).map_err(|e| format!("Create dir error: {}", e))?;

    let mut extracted = Vec::new();
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("ZIP index error: {}", e))?;
        let outpath = std::path::Path::new(&dst_dir).join(file.mangled_name());

        if file.name().ends_with('/') {
            fs_std::create_dir_all(&outpath).map_err(|e| format!("Create dir error: {}", e))?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs_std::create_dir_all(p).map_err(|e| format!("Create parent error: {}", e))?;
                }
            }
            let mut outfile =
                File::create(&outpath).map_err(|e| format!("Create file error: {}", e))?;
            copy(&mut file, &mut outfile).map_err(|e| format!("Extract error: {}", e))?;
            extracted.push(outpath.to_string_lossy().to_string());
        }
    }
    Ok(extracted)
}

/// Inspect a backup file and return metadata about its contents
#[tauri::command]
pub async fn inspect_backup(
    state: State<'_, AppState>,
    backup_path: String,
) -> Result<String, String> {
    let svc = state.vault_service.read().await;
    let session_key = svc.get_session_key().ok_or("Vault not unlocked")?;
    let key: [u8; 32] = session_key
        .as_slice()
        .try_into()
        .map_err(|_| "Invalid key")?;

    let encrypted = fs_std::read(&backup_path).map_err(|e| format!("Read backup failed: {}", e))?;
    let plaintext = solosoul_crypto::aes::decrypt_blob(&key, &encrypted)
        .map_err(|e| format!("Decryption failed: {}", e))?;
    let json_str =
        String::from_utf8(plaintext.to_vec()).map_err(|e| format!("Invalid UTF-8: {}", e))?;

    let mut obj_count = 0;
    let mut type_ids: Vec<String> = Vec::new();

    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_str) {
        if let Some(arr) = val
            .get("unified_objects")
            .and_then(|u| u.get("objects"))
            .and_then(|o| o.as_array())
        {
            obj_count = arr.len();
            for obj in arr.iter().take(30) {
                if let Some(tid) = obj.get("typeId").and_then(|t| t.as_str()) {
                    type_ids.push(tid.to_string());
                }
            }
        }
    }

    Ok(format!(
        "BACKUP: len={}, obj_count={}, type_ids={:?}",
        json_str.len(),
        obj_count,
        type_ids
    ))
}
