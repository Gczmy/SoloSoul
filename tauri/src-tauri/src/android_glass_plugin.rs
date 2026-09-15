//! Android 材质能力和原生快捷菜单。仅传递展示文案/主题与固定动作，不读取 Vault 字段。
use serde::{Deserialize, Serialize};
use tauri::{
    plugin::{Builder, PluginApi, TauriPlugin},
    AppHandle, Manager, Runtime,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlassMenuPayload {
    request_id: String,
    title: String,
    description: String,
    close_label: String,
    footer: String,
    labels: MenuLabels,
    descriptions: MenuLabels,
    dark: bool,
    reduce_motion: bool,
    background: String,
    foreground: String,
    secondary: String,
    accent: String,
    container: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MenuLabels {
    object: String,
    page: String,
    scan: String,
}

fn valid_request_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
}

impl GlassMenuPayload {
    fn validate(&self) -> Result<(), String> {
        if !valid_request_id(&self.request_id) {
            return Err("Invalid menu request ID".into());
        }
        for text in [
            &self.title,
            &self.description,
            &self.close_label,
            &self.footer,
            &self.labels.object,
            &self.labels.page,
            &self.labels.scan,
            &self.descriptions.object,
            &self.descriptions.page,
            &self.descriptions.scan,
        ] {
            if text.len() > 600 {
                return Err("Menu label too long".into());
            }
        }
        for color in [
            &self.background,
            &self.foreground,
            &self.secondary,
            &self.accent,
            &self.container,
        ] {
            if color.len() != 7
                || !color.starts_with('#')
                || !color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
            {
                return Err("Invalid menu color".into());
            }
        }
        Ok(())
    }
}

pub struct AndroidGlassHandle<R: Runtime> {
    #[cfg(target_os = "android")]
    handle: tauri::plugin::PluginHandle<R>,
    #[cfg(not(target_os = "android"))]
    _phantom: std::marker::PhantomData<fn() -> R>,
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("android-glass")
        .setup(|app, api| {
            register(app, api)?;
            Ok(())
        })
        .build()
}

fn register<R: Runtime>(
    app: &AppHandle<R>,
    _api: PluginApi<R, ()>,
) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "android")]
    app.manage(AndroidGlassHandle {
        handle: _api.register_android_plugin("com.solosoul.app", "AndroidGlassPlugin")?,
    });
    #[cfg(not(target_os = "android"))]
    app.manage(AndroidGlassHandle {
        _phantom: std::marker::PhantomData::<fn() -> R>,
    });
    Ok(())
}

#[tauri::command]
pub async fn android_glass_capabilities<R: Runtime>(
    _app: AppHandle<R>,
) -> Result<serde_json::Value, String> {
    #[cfg(target_os = "android")]
    return _app
        .state::<AndroidGlassHandle<R>>()
        .handle
        .run_mobile_plugin_async("capabilities", serde_json::json!({}))
        .await
        .map_err(|e| e.to_string());
    #[cfg(not(target_os = "android"))]
    Ok(serde_json::json!({ "apiLevel": 0, "windowBlur": false, "webViewVersion": "" }))
}

#[tauri::command]
pub async fn android_show_glass_menu<R: Runtime>(
    _app: AppHandle<R>,
    payload: GlassMenuPayload,
) -> Result<serde_json::Value, String> {
    payload.validate()?;
    #[cfg(target_os = "android")]
    return _app
        .state::<AndroidGlassHandle<R>>()
        .handle
        .run_mobile_plugin_async("showMenu", payload)
        .await
        .map_err(|e| e.to_string());
    #[cfg(not(target_os = "android"))]
    Ok(serde_json::json!({ "requestId": payload.request_id, "action": "unavailable" }))
}

#[tauri::command]
pub async fn android_close_glass_menu<R: Runtime>(
    _app: AppHandle<R>,
    request_id: String,
) -> Result<(), String> {
    if !valid_request_id(&request_id) {
        return Err("Invalid menu request ID".into());
    }
    #[cfg(target_os = "android")]
    _app.state::<AndroidGlassHandle<R>>()
        .handle
        .run_mobile_plugin_async::<serde_json::Value>(
            "closeMenu",
            serde_json::json!({ "requestId": request_id }),
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn menu_payload_rejects_unbounded_labels_and_invalid_colors() {
        let mut value = serde_json::json!({
            "requestId": "a-1", "title": "新建", "description": "", "closeLabel": "关闭", "footer": "本地",
            "labels": {"object":"对象", "page":"页面", "scan":"扫描"},
            "descriptions": {"object":"", "page":"", "scan":""}, "dark":false, "reduceMotion":false,
            "background":"#FAFAF8", "foreground":"#20251F", "secondary":"#626B62", "accent":"#405F82", "container":"#D9E7F8"
        });
        assert!(serde_json::from_value::<GlassMenuPayload>(value.clone())
            .unwrap()
            .validate()
            .is_ok());
        value["background"] = "transparent".into();
        assert!(serde_json::from_value::<GlassMenuPayload>(value.clone())
            .unwrap()
            .validate()
            .is_err());
        value["background"] = "#FAFAF8".into();
        value["title"] = "x".repeat(601).into();
        assert!(serde_json::from_value::<GlassMenuPayload>(value)
            .unwrap()
            .validate()
            .is_err());
        assert!(!valid_request_id(""));
        assert!(!valid_request_id("../../request"));
    }
}
