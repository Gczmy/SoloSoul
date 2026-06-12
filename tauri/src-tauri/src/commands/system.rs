use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

/// Information about a downloadable release asset.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAsset {
    pub name: String,
    pub download_url: String,
    pub size: u64,
}

#[tauri::command]
pub async fn get_app_info() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "appName": "SoloSoul",
        "version": env!("CARGO_PKG_VERSION"),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
    }))
}

/// Fetch the latest release version from GitHub and compare with local version.
/// Uses the public repo (Gczmy/SoloSoul) releases API.
/// Returns version info + downloadable assets for the current platform.
#[tauri::command]
pub async fn check_version() -> Result<serde_json::Value, String> {
    let current = env!("CARGO_PKG_VERSION");

    match fetch_latest_release().await {
        Ok(Some((latest_ver, assets))) => {
            let has_update = compare_versions(&latest_ver, current) > 0;
            // Filter assets to match current platform
            let platform_assets = filter_platform_assets(assets);
            Ok(serde_json::json!({
                "currentVersion": current,
                "latestVersion": latest_ver,
                "hasUpdate": has_update,
                "assets": platform_assets,
            }))
        }
        _ => {
            // Network failure — report no update silently
            Ok(serde_json::json!({
                "currentVersion": current,
                "latestVersion": null,
                "hasUpdate": false,
                "assets": [],
            }))
        }
    }
}

/// Download the update asset for the current platform.
/// Streams to a temp file in the system Downloads directory,
/// emitting `update-download-progress` Tauri events during download.
/// Returns the final file path on success.
#[tauri::command]
pub async fn download_update(
    app: AppHandle,
    asset_name: String,
    asset_url: String,
) -> Result<String, String> {
    // Determine download destination
    let dest = download_path(&asset_name)?;

    let client = reqwest::Client::new();
    let resp = client
        .get(&asset_url)
        .header("User-Agent", "SoloSoul")
        .timeout(std::time::Duration::from_secs(300))
        .send()
        .await
        .map_err(|e| format!("Download request failed: {}", e))?;

    let total_size = resp.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut stream = resp.bytes_stream();

    use futures::StreamExt;
    use std::io::Write;

    let file = std::fs::File::create(&dest).map_err(|e| format!("Failed to create file: {}", e))?;
    let mut writer = std::io::BufWriter::new(file);
    // Report every ~500 KB
    let report_interval = 500 * 1024u64;
    let mut next_report = report_interval;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download stream error: {}", e))?;
        writer
            .write_all(&chunk)
            .map_err(|e| format!("Write error: {}", e))?;
        downloaded += chunk.len() as u64;

        if downloaded >= next_report {
            let _ = app.emit(
                "update-download-progress",
                serde_json::json!({
                    "downloaded": downloaded,
                    "total": total_size,
                }),
            );
            next_report = downloaded + report_interval;
        }
    }

    writer.flush().map_err(|e| format!("Flush error: {}", e))?;

    // Ensure the file is executable on macOS (DMG needs it)
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(&dest) {
            let mut perms = meta.permissions();
            perms.set_mode(0o644);
            let _ = std::fs::set_permissions(&dest, perms);
        }
    }

    // Emit final 100%
    let _ = app.emit(
        "update-download-progress",
        serde_json::json!({
            "downloaded": downloaded,
            "total": total_size,
        }),
    );

    Ok(dest.to_string_lossy().to_string())
}

/// Build the download destination path in ~/Downloads.
fn download_path(filename: &str) -> Result<PathBuf, String> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| "Cannot determine home directory".to_string())?;
    let dir = PathBuf::from(home).join("Downloads");
    std::fs::create_dir_all(&dir).map_err(|e| format!("Cannot create Downloads dir: {}", e))?;
    Ok(dir.join(filename))
}

/// Pick the correct asset for the current platform.
fn filter_platform_assets(assets: Vec<ReleaseAsset>) -> Vec<UpdateAsset> {
    let (os, arch) = (std::env::consts::OS, std::env::consts::ARCH);
    let pattern: Option<&str> = match (os, arch) {
        ("macos", _) => Some(".dmg"),
        ("windows", _) => Some("_x64-setup.exe"),
        ("linux", _) => Some(".AppImage"),
        _ => None,
    };
    let pattern = match pattern {
        Some(p) => p,
        None => return vec![],
    };
    assets
        .into_iter()
        .filter(|a| {
            if !a.name.ends_with(pattern) {
                return false;
            }
            // On macOS prefer the arch-specific DMG
            if os == "macos" && arch == "aarch64" {
                a.name.contains("arm64")
            } else if os == "macos" {
                !a.name.contains("arm64")
            } else {
                true
            }
        })
        .map(|a| UpdateAsset {
            name: a.name.clone(),
            download_url: a.browser_download_url.clone(),
            size: a.size,
        })
        .collect()
}

// ── GitHub API types ──────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct ReleaseResponse {
    tag_name: String,
    assets: Vec<ReleaseAsset>,
}

#[derive(serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ReleaseAsset {
    name: String,
    size: u64,
    browser_download_url: String,
}

/// Fetch the latest release tag + assets from GitHub.
async fn fetch_latest_release() -> Result<Option<(String, Vec<ReleaseAsset>)>, String> {
    let url = "https://api.github.com/repos/Gczmy/SoloSoul/releases/latest";
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "SoloSoul")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !resp.status().is_success() {
        return Ok(None);
    }

    let body: ReleaseResponse = resp
        .json()
        .await
        .map_err(|e| format!("JSON parse failed: {}", e))?;

    let tag = body.tag_name.trim_start_matches('v').to_string();
    Ok(Some((tag, body.assets)))
}

/// Simple semver comparison: returns > 0 if a > b, 0 if equal, < 0 if a < b.
fn compare_versions(a: &str, b: &str) -> i32 {
    let a_parts: Vec<i32> = a.split('.').filter_map(|s| s.parse::<i32>().ok()).collect();
    let b_parts: Vec<i32> = b.split('.').filter_map(|s| s.parse::<i32>().ok()).collect();
    for i in 0..3 {
        let av = a_parts.get(i).copied().unwrap_or(0);
        let bv = b_parts.get(i).copied().unwrap_or(0);
        if av != bv {
            return av - bv;
        }
    }
    0
}

/// Get the OS UI display language (e.g. "zh-CN", "en-US").
#[tauri::command]
pub fn get_system_locale() -> Result<String, String> {
    let result = get_ui_language();
    tracing::info!("[i18n] get_system_locale command: {:?}", result);
    result.ok_or_else(|| "Failed to detect UI language".to_string())
}

#[cfg(target_os = "windows")]
pub fn get_ui_language() -> Option<String> {
    // Windows impl unchanged
    use windows::Win32::Globalization::GetUserDefaultUILanguage;
    let lang_id = unsafe { GetUserDefaultUILanguage() };
    let primary_id = lang_id & 0x3FF;
    if primary_id == 0x04 {
        Some("zh-CN".to_string())
    } else {
        Some("en-US".to_string())
    }
}

#[cfg(not(target_os = "windows"))]
pub fn get_ui_language() -> Option<String> {
    sys_locale::get_locale()
}
