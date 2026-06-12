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
#[tauri::command]
pub async fn check_version() -> Result<serde_json::Value, String> {
    let current = env!("CARGO_PKG_VERSION");

    match fetch_latest_release().await {
        Ok(Some(latest_ver)) => {
            let has_update = compare_versions(&latest_ver, current) > 0;
            Ok(serde_json::json!({
                "currentVersion": current,
                "latestVersion": latest_ver,
                "hasUpdate": has_update,
            }))
        }
        _ => {
            // Network failure or parse error — report "no update" silently
            Ok(serde_json::json!({
                "currentVersion": current,
                "latestVersion": null,
                "hasUpdate": false,
            }))
        }
    }
}

/// Call GitHub Releases API to find the latest release tag.
async fn fetch_latest_release() -> Result<Option<String>, String> {
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

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("JSON parse failed: {}", e))?;

    let tag = body
        .get("tag_name")
        .and_then(|v| v.as_str())
        .map(|s| s.trim_start_matches('v').to_string());

    Ok(tag)
}

/// Simple semver comparison: returns > 0 if a > b, 0 if equal, < 0 if a < b.
fn compare_versions(a: &str, b: &str) -> i32 {
    let a_parts: Vec<i32> = a
        .split('.')
        .filter_map(|s| s.parse::<i32>().ok())
        .collect();
    let b_parts: Vec<i32> = b
        .split('.')
        .filter_map(|s| s.parse::<i32>().ok())
        .collect();
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
///
/// On Windows, returns the user's *display language* (what menus/dialogs show),
/// not the regional format — those can differ. Falls back to sys-locale on other platforms.
#[tauri::command]
pub fn get_system_locale() -> Result<String, String> {
    let result = get_ui_language();
    tracing::info!("[i18n] get_system_locale command: {:?}", result);
    result.ok_or_else(|| "Failed to detect UI language".to_string())
}

#[cfg(target_os = "windows")]
pub fn get_ui_language() -> Option<String> {
    use windows::Win32::Globalization::GetUserDefaultUILanguage;
    // GetUserDefaultUILanguage returns the UI display language LANGID
    // Low 10 bits = primary language ID. 0x04 = Chinese (all variants)
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
    // macOS/Linux: sys_locale is reliable here
    sys_locale::get_locale()
}
