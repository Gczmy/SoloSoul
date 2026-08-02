//! 移动端 OCR 插件（Android ML Kit Text Recognition）
//!
//! 桌面端无实际功能，仅提供占位句柄以满足类型约束。
//! Android 端通过 Kotlin 插件调用 ML Kit Text Recognition v2，
//! 将识别结果映射为与桌面端一致的 `OcrResult` 结构。

use serde::{Deserialize, Serialize};
use tauri::{
    plugin::{Builder, PluginApi, TauriPlugin},
    AppHandle, Manager, Runtime,
};

#[cfg(target_os = "android")]
use tauri::plugin::PluginHandle;

use solosoul_core::ocr::types::{OcrBox, OcrResult};

/// Android 插件包名。
#[cfg(target_os = "android")]
const PLUGIN_IDENTIFIER: &str = "com.solosoul.app";

/// 调用 Kotlin 插件时传入的参数。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanImagePayload {
    pub file_path: String,
}

/// Kotlin 插件返回的单个文本块。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileOcrBox {
    pub text: String,
    pub confidence: f64,
    pub points: [(f32, f32); 4],
}

/// Kotlin 插件返回的识别结果。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileOcrResult {
    pub text: String,
    pub confidence: f64,
    pub boxes: Vec<MobileOcrBox>,
}

impl From<MobileOcrResult> for OcrResult {
    fn from(result: MobileOcrResult) -> Self {
        Self {
            text: result.text,
            confidence: result.confidence,
            boxes: result
                .boxes
                .into_iter()
                .map(|b| OcrBox {
                    text: b.text,
                    confidence: b.confidence,
                    points: b.points,
                })
                .collect(),
        }
    }
}

/// 插件句柄包装，便于在 command 中通过 Tauri state 获取。
pub struct MobileOcrPluginHandle<R: Runtime> {
    #[cfg(target_os = "android")]
    handle: PluginHandle<R>,
    #[cfg(not(target_os = "android"))]
    _phantom: std::marker::PhantomData<fn() -> R>,
}

/// Kotlin 拍照插件返回的结果。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TakePhotoResult {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl<R: Runtime> MobileOcrPluginHandle<R> {
    /// 在 Android 端通过 ML Kit 识别图片中的文字。
    /// 非 Android 平台直接返回不支持错误。
    pub fn scan_image(&self, payload: ScanImagePayload) -> Result<MobileOcrResult, String> {
        #[cfg(target_os = "android")]
        {
            self.handle
                .run_mobile_plugin("scanImage", payload)
                .map_err(|e| e.to_string())
                .and_then(|v| {
                    serde_json::from_value::<MobileOcrResult>(v).map_err(|e| e.to_string())
                })
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = payload;
            Err("mobile_ocr_scan_image is only supported on Android".to_string())
        }
    }

    /// 启动系统相机拍照，返回临时文件路径（file:// URI）。
    /// 非 Android 平台直接返回不支持错误。
    /// Kotlin 侧持有 invoke 直到 Activity 回调，无需 spawn_blocking。
    pub fn take_photo(&self) -> Result<TakePhotoResult, String> {
        #[cfg(target_os = "android")]
        {
            // takePhoto 命令不读取 payload，() 序列化为 null 不影响 Kotlin 端
            self.handle
                .run_mobile_plugin("takePhoto", &())
                .map_err(|e| e.to_string())
                .and_then(|v| {
                    serde_json::from_value::<TakePhotoResult>(v).map_err(|e| e.to_string())
                })
        }
        #[cfg(not(target_os = "android"))]
        {
            Err("mobile_ocr_take_photo is only supported on Android".to_string())
        }
    }
}

/// 初始化插件：注册 Android Kotlin 插件并将句柄存入 state。
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("mobile-ocr")
        .setup(|_app, api| {
            register_plugin::<R>(_app, api)?;
            Ok(())
        })
        .build()
}

#[cfg(target_os = "android")]
fn register_plugin<R: Runtime>(
    app: &AppHandle<R>,
    api: PluginApi<R, ()>,
) -> Result<(), Box<dyn std::error::Error>> {
    let handle = api.register_android_plugin(PLUGIN_IDENTIFIER, "MobileOcrPlugin")?;
    app.manage(MobileOcrPluginHandle { handle });
    Ok(())
}

#[cfg(not(target_os = "android"))]
fn register_plugin<R: Runtime>(
    app: &AppHandle<R>,
    _api: PluginApi<R, ()>,
) -> Result<(), Box<dyn std::error::Error>> {
    app.manage(MobileOcrPluginHandle {
        _phantom: std::marker::PhantomData::<fn() -> R>,
    });
    Ok(())
}

/// 识别图片中的文字（移动端入口）。
/// 注意：本函数由 ocr.rs 直接作为普通 Rust 函数调用，不注册为 Tauri 命令，
/// 故不带 #[tauri::command] 属性。
pub async fn mobile_ocr_scan_image<R: Runtime>(
    app: AppHandle<R>,
    file_path: String,
) -> Result<OcrResult, String> {
    // ML Kit 识别是 IO/CPU 密集型操作，放到 spawn_blocking 避免阻塞 tokio runtime
    let result = tokio::task::spawn_blocking(move || {
        let handle = app.state::<MobileOcrPluginHandle<R>>();
        handle.scan_image(ScanImagePayload { file_path })
    })
    .await
    .map_err(|e| format!("mobile ocr task failed: {e}"))??;
    Ok(result.into())
}

/// 启动系统相机拍照（移动端入口）。
/// 返回临时文件路径（file:// URI），取消时返回 None。
#[tauri::command]
pub async fn mobile_ocr_take_photo<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Option<String>, String> {
    let handle = app.state::<MobileOcrPluginHandle<R>>();
    handle.take_photo().map(|r| r.path)
}
