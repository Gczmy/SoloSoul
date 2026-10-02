//! Android 内置资源后台准备屏障；只有资源消费者等待，启动窗口与其他功能不等待。

#[cfg(target_os = "android")]
use tauri::Manager;

#[cfg(target_os = "android")]
static APP: std::sync::OnceLock<tauri::AppHandle> = std::sync::OnceLock::new();

#[cfg(target_os = "android")]
struct ResourceHandle(tauri::plugin::PluginHandle<tauri::Wry>);

pub fn init() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri::plugin::Builder::new("resource-preparation")
        .setup(|_app, _api| {
            #[cfg(target_os = "android")]
            _app.manage(ResourceHandle(_api.register_android_plugin(
                "com.solosoul.app",
                "ResourcePreparationPlugin",
            )?));
            Ok(())
        })
        .build()
}

pub fn initialize(_app: &tauri::AppHandle) {
    #[cfg(target_os = "android")]
    let _ = APP.set(_app.clone());
}

pub async fn await_ready() -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        let app = APP.get().ok_or("ANDROID_RESOURCES_UNAVAILABLE")?;
        let handle = app
            .try_state::<ResourceHandle>()
            .ok_or("ANDROID_RESOURCES_UNAVAILABLE")?;
        let result: serde_json::Value = handle
            .0
            .run_mobile_plugin_async("waitUntilReady", serde_json::json!({}))
            .await
            .map_err(|_| "ANDROID_RESOURCES_UNAVAILABLE")?;
        validate_ready(&result)?;
    }
    Ok(())
}

#[cfg(any(target_os = "android", test))]
fn validate_ready(reply: &serde_json::Value) -> Result<(), String> {
    match reply["status"].as_str() {
        Some("ready") => Ok(()),
        Some("error") => Err("ANDROID_RESOURCES_FAILED".into()),
        _ => Err("ANDROID_RESOURCES_UNAVAILABLE".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rf207_consumers_only_accept_complete_ready_response() {
        assert!(validate_ready(&serde_json::json!({"status":"ready"})).is_ok());
        for status in ["idle", "preparing", "error", "unknown"] {
            assert!(validate_ready(&serde_json::json!({"status":status})).is_err());
        }
        assert!(validate_ready(&serde_json::json!({})).is_err());
    }
}
