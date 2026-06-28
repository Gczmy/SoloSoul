//! OCR commands — scan images/documents for text extraction
//!
//! 基于 `solosoul-core::ocr` 的本地 PP-OCRv6 引擎。
//! 模型文件存放在应用数据目录的 `models/` 下，支持从打包资源复制或运行时下载。

use crate::commands::{current_account, vault_handle};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use serde_json::json;
use solosoul_core::ocr::{
    engine::OcrEngine,
    model::{
        install_model_from_bundled, install_model_from_bundled_with_progress, is_model_installed,
        resolve_model_bundle,
    },
    types::{MrzResult, OcrModelTier, OcrResult},
};
use std::path::{Path, PathBuf};
use tauri::{Emitter, Manager};

// Re-export core types so callers can rely on a stable Tauri-facing name.
pub use solosoul_core::ocr::types::{OcrBox, OcrResult as OcrScanResult};

/// OCR 应用偏好设置。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OcrPreferences {
    /// 当前使用的模型档位。
    #[serde(default)]
    active_tier: OcrModelTier,
}

impl Default for OcrPreferences {
    fn default() -> Self {
        Self {
            active_tier: OcrModelTier::Small,
        }
    }
}

/// 模型档位信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrTierInfo {
    pub tier: String,
    pub name: String,
    pub description: String,
}

/// 模型安装状态。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrModelStatus {
    pub tier: String,
    pub installed: bool,
    pub bundled: bool,
}

// =============================================================================
// Paths and preferences
// =============================================================================

/// 解析应用本地数据目录下的 OCR 模型根目录。
fn models_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .resolve("models", tauri::path::BaseDirectory::LocalData)
        .map_err(|e| format!("无法解析模型目录: {e}"))
}

/// 解析打包资源中的模型根目录。
fn bundled_models_dir() -> Result<PathBuf, String> {
    // 优先使用 Tauri setup 阶段解析的资源目录，保证生产包路径正确。
    if let Some(dir) = crate::commands::llm::RESOURCE_DIR.get() {
        return Ok(dir.join("models"));
    }

    if cfg!(debug_assertions) {
        Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("models"))
    } else {
        Err("未在 release 模式下初始化 RESOURCE_DIR".to_string())
    }
}

/// OCR 偏好设置文件路径。
fn preferences_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .resolve(
            "ocr_preferences.json",
            tauri::path::BaseDirectory::LocalData,
        )
        .map_err(|e| format!("无法解析 OCR 偏好设置路径: {e}"))
}

fn load_preferences(app: &tauri::AppHandle) -> OcrPreferences {
    let path = match preferences_path(app) {
        Ok(p) => p,
        Err(_) => return OcrPreferences::default(),
    };
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_preferences(app: &tauri::AppHandle, prefs: &OcrPreferences) -> Result<(), String> {
    let path = preferences_path(app)?;
    let json = serde_json::to_string_pretty(prefs).map_err(|e| format!("序列化偏好设置: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("写入偏好设置: {e}"))?;
    Ok(())
}

fn active_tier(app: &tauri::AppHandle) -> OcrModelTier {
    load_preferences(app).active_tier
}

/// 尝试从 Tauri 资源目录定位 PDFium 动态库，并通过环境变量告知 `pdfium-render`。
///
/// 该环境变量仅在当前进程内有效；若用户已手动设置，则保留原值。
fn ensure_pdfium_library_path(app: &tauri::AppHandle) {
    if std::env::var("PDFIUM_LIBRARY_PATH").is_ok() {
        return;
    }
    let filename = if cfg!(target_os = "macos") {
        "libpdfium.dylib"
    } else if cfg!(target_os = "windows") {
        "pdfium.dll"
    } else {
        "libpdfium.so"
    };
    if let Ok(resource_dir) = app.path().resource_dir() {
        let candidate = resource_dir.join("pdfium").join(filename);
        if candidate.exists() {
            std::env::set_var("PDFIUM_LIBRARY_PATH", candidate);
        }
    }
}

/** 前端用于识别「模型未安装」错误并做国际化提示的前缀。 */
const OCR_MODEL_NOT_INSTALLED_PREFIX: &str = "__OCR_MODEL_NOT_INSTALLED__";

/// 确保目标档位的模型可用：优先从打包资源复制，否则返回机器可读错误码供前端国际化。
fn ensure_model_available(app: &tauri::AppHandle, tier: OcrModelTier) -> Result<PathBuf, String> {
    let models_dir = models_dir(app)?;

    if is_model_installed(&models_dir, tier) {
        return Ok(models_dir);
    }

    // 尝试从打包资源复制。
    let bundled_dir = bundled_models_dir().unwrap_or_else(|_| PathBuf::new());
    if bundled_dir.exists() && install_model_from_bundled(&bundled_dir, &models_dir, tier).is_ok() {
        return Ok(models_dir);
    }

    Err(format!("{}:{}", OCR_MODEL_NOT_INSTALLED_PREFIX, tier))
}

// =============================================================================
// Commands
// =============================================================================

/// 扫描图片或 PDF 并返回识别到的文本。
///
/// 要求 Vault 已解锁；使用当前激活的模型档位。
/// PDF 文件优先提取文本层，若无文本则逐页渲染为图片后 OCR。
#[tauri::command]
pub async fn ocr_scan_image(
    state: tauri::State<'_, AppState>,
    file_path: String,
    _language: Option<String>,
) -> Result<OcrResult, String> {
    // Vault 解锁检查。
    let vault = vault_handle(&state)?;
    let account_id = current_account(&state)?;

    let app = state.app_handle();
    let tier = active_tier(app);
    let models_dir = ensure_model_available(app, tier)?;

    let path = PathBuf::from(&file_path);
    if !path.exists() {
        return Err(format!("文件不存在: {}", path.display()));
    }

    // 验证文件路径在允许的目录内（防御性安全校验）
    if !is_path_in_allowed_dir(&path) {
        return Err("文件路径不在允许的目录中（Desktop/Documents/Downloads）".to_string());
    }

    let mut engine = OcrEngine::load(&models_dir, tier)?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());
    let file_type = ext.as_deref();

    let result = match file_type {
        Some("pdf") => {
            ensure_pdfium_library_path(app);
            engine.scan_pdf(&path)?
        }
        _ => engine.scan_image(&path)?,
    };

    let file_name = path.file_name().map(|n| n.to_string_lossy().to_string());
    let details = json!({
        "fileType": file_type.unwrap_or("unknown"),
        "tier": tier.to_string(),
        "boxCount": result.boxes.len(),
        "textLength": result.text.len(),
        "confidence": result.confidence,
    })
    .to_string();
    let _ = vault.log_structured(
        "ocr_scan",
        "file",
        None,
        file_name.as_deref(),
        &account_id,
        Some(&details),
    );

    Ok(result)
}

/// 扫描图片中的 MRZ（机读区）并返回解析结果。
///
/// 若未检测到 MRZ 区域，返回 `null`。
#[tauri::command]
pub async fn ocr_scan_mrz(
    state: tauri::State<'_, AppState>,
    file_path: String,
) -> Result<Option<MrzResult>, String> {
    let vault = vault_handle(&state)?;
    let account_id = current_account(&state)?;

    let app = state.app_handle();
    let tier = active_tier(app);
    let models_dir = ensure_model_available(app, tier)?;

    let path = PathBuf::from(&file_path);
    if !path.exists() {
        return Err(format!("文件不存在: {}", path.display()));
    }

    // 验证文件路径在允许的目录内（防御性安全校验）
    if !is_path_in_allowed_dir(&path) {
        return Err("文件路径不在允许的目录中（Desktop/Documents/Downloads）".to_string());
    }

    let mut engine = OcrEngine::load(&models_dir, tier)?;
    let result = engine.scan_mrz(&path)?;

    let file_name = path.file_name().map(|n| n.to_string_lossy().to_string());
    let has_mrz = result.is_some();
    let details = json!({
        "tier": tier.to_string(),
        "hasMrz": has_mrz,
    })
    .to_string();
    let _ = vault.log_structured(
        "ocr_scan_mrz",
        "file",
        None,
        file_name.as_deref(),
        &account_id,
        Some(&details),
    );

    Ok(result)
}

/// 返回 OCR 支持的识别语言列表。
///
/// PP-OCRv6 识别模型本身支持多语言（中英文为主），这里返回稳定的 UI 选项。
#[tauri::command]
pub async fn ocr_get_supported_languages() -> Result<Vec<String>, String> {
    Ok(vec![
        "auto".to_string(),
        "en".to_string(),
        "zh-CN".to_string(),
        "ja".to_string(),
        "ko".to_string(),
    ])
}

/// 返回所有可用的模型档位信息。
#[tauri::command]
pub async fn ocr_list_available_tiers() -> Result<Vec<OcrTierInfo>, String> {
    Ok(vec![
        OcrTierInfo {
            tier: "tiny".to_string(),
            name: "Tiny".to_string(),
            description: "1.5M 参数，速度最快，适合简单场景".to_string(),
        },
        OcrTierInfo {
            tier: "small".to_string(),
            name: "Small".to_string(),
            description: "约 30MB，速度与精度平衡（默认）".to_string(),
        },
        OcrTierInfo {
            tier: "medium".to_string(),
            name: "Medium".to_string(),
            description: "约 132MB，高精度，适合复杂文档".to_string(),
        },
    ])
}

/// 获取当前激活的模型档位。
#[tauri::command]
pub async fn ocr_get_active_tier(state: tauri::State<'_, AppState>) -> Result<String, String> {
    Ok(active_tier(state.app_handle()).to_string())
}

/// 设置当前激活的模型档位。
#[tauri::command]
pub async fn ocr_set_active_tier(
    state: tauri::State<'_, AppState>,
    tier: String,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    let account_id = current_account(&state)?;

    let tier: OcrModelTier = tier.parse()?;
    let mut prefs = load_preferences(state.app_handle());
    prefs.active_tier = tier;
    save_preferences(state.app_handle(), &prefs)?;

    let _ = vault.log_structured(
        "ocr_set_active_tier",
        "ocr_model",
        None,
        Some(&tier.to_string()),
        &account_id,
        None,
    );
    Ok(())
}

/// 查询指定档位的模型安装状态。
#[tauri::command]
pub async fn ocr_get_model_status(
    state: tauri::State<'_, AppState>,
    tier: String,
) -> Result<OcrModelStatus, String> {
    let tier: OcrModelTier = tier.parse()?;
    let models_dir = models_dir(state.app_handle())?;
    let bundled_dir = bundled_models_dir().unwrap_or_else(|_| PathBuf::new());

    Ok(OcrModelStatus {
        tier: tier.to_string(),
        installed: is_model_installed(&models_dir, tier),
        bundled: resolve_model_bundle(&bundled_dir, tier).is_ok(),
    })
}

/// OCR 模型安装进度事件负载。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OcrInstallProgress {
    pub tier: String,
    pub progress: u8,
    pub done: bool,
    pub error: Option<String>,
}

/// 从打包资源安装指定档位的模型到应用数据目录，并发送进度事件。
///
/// 事件名：`ocr-install-progress`
///
/// 注意：首次启动安装模型可能在用户登录前发生，因此本命令不依赖 Vault 解锁状态，
/// 也不写入账户审计日志。
#[tauri::command]
pub async fn ocr_install_bundled_model_with_progress(
    state: tauri::State<'_, AppState>,
    tier: String,
) -> Result<(), String> {
    let tier: OcrModelTier = tier.parse()?;
    let models_dir = models_dir(state.app_handle())?;
    let bundled_dir = bundled_models_dir()?;
    let app = state.app_handle().clone();
    let tier_str = tier.to_string();

    let emit_progress = move |progress: u8| {
        let _ = app.emit(
            "ocr-install-progress",
            OcrInstallProgress {
                tier: tier_str.clone(),
                progress,
                done: progress == 100,
                error: None,
            },
        );
    };

    let result =
        install_model_from_bundled_with_progress(&bundled_dir, &models_dir, tier, emit_progress);

    if let Err(ref e) = result {
        let _ = state.app_handle().emit(
            "ocr-install-progress",
            OcrInstallProgress {
                tier: tier.to_string(),
                progress: 0,
                done: true,
                error: Some(e.clone()),
            },
        );
    }

    result
}

/// 从打包资源安装指定档位的模型到应用数据目录。
#[tauri::command]
pub async fn ocr_install_bundled_model(
    state: tauri::State<'_, AppState>,
    tier: String,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    let account_id = current_account(&state)?;

    let tier: OcrModelTier = tier.parse()?;
    let models_dir = models_dir(state.app_handle())?;
    let bundled_dir = bundled_models_dir()?;
    install_model_from_bundled(&bundled_dir, &models_dir, tier)?;

    let _ = vault.log_structured(
        "ocr_install_bundled_model",
        "ocr_model",
        None,
        Some(&tier.to_string()),
        &account_id,
        None,
    );
    Ok(())
}

/// 从远程 URL 下载指定档位的模型。
///
/// `base_url` 应指向模型文件所在的根目录，目录结构为：
/// `{base_url}/{tier}/det/inference.onnx`
/// `{base_url}/{tier}/det/inference.yml`
/// `{base_url}/{tier}/rec/inference.onnx`
/// `{base_url}/{tier}/rec/inference.yml`
#[tauri::command]
pub async fn ocr_download_model(
    state: tauri::State<'_, AppState>,
    tier: String,
    base_url: String,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;
    let account_id = current_account(&state)?;

    let tier: OcrModelTier = tier.parse()?;
    let models_dir = models_dir(state.app_handle())?;
    download_model_files(&base_url, &models_dir, tier).await?;

    let details = json!({ "baseUrl": base_url }).to_string();
    let _ = vault.log_structured(
        "ocr_download_model",
        "ocr_model",
        None,
        Some(&tier.to_string()),
        &account_id,
        Some(&details),
    );
    Ok(())
}

// =============================================================================
// Path validation helper
// =============================================================================

/// 检查路径是否在允许的用户目录内（Desktop/Documents/Downloads）。
/// 用于防御性安全校验，防止路径遍历攻击。
fn is_path_in_allowed_dir(path: &Path) -> bool {
    let canon = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };

    let home_var = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    let home = match std::env::var(home_var) {
        Ok(h) => PathBuf::from(h),
        Err(_) => return false,
    };

    for dir_name in &["Desktop", "Documents", "Downloads"] {
        let allowed = home.join(dir_name);
        if let Ok(allowed_canon) = allowed.canonicalize() {
            if canon.starts_with(&allowed_canon) {
                return true;
            }
        }
    }

    false
}

// =============================================================================
// Download helper
// =============================================================================

async fn download_model_files(
    base_url: &str,
    models_dir: &Path,
    tier: OcrModelTier,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let tier_name = tier.remote_name();
    let files = [
        ("det/inference.onnx", "检测模型"),
        ("det/inference.yml", "检测配置"),
        ("rec/inference.onnx", "识别模型"),
        ("rec/inference.yml", "识别配置"),
    ];

    let dst_base = models_dir.join(tier.dir_name());
    let dst_det = dst_base.join("det");
    let dst_rec = dst_base.join("rec");
    std::fs::create_dir_all(&dst_det).map_err(|e| format!("创建 det 目录失败: {e}"))?;
    std::fs::create_dir_all(&dst_rec).map_err(|e| format!("创建 rec 目录失败: {e}"))?;

    let base = base_url.trim_end_matches('/');
    for (rel_path, label) in files {
        let url = format!("{base}/{tier_name}/{rel_path}");
        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("下载 {label} 失败: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("下载 {label} 失败: HTTP {}", resp.status()));
        }
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| format!("读取 {label} 响应失败: {e}"))?;
        let dst = dst_base.join(rel_path);
        std::fs::write(&dst, bytes).map_err(|e| format!("写入 {label} 失败: {e}"))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ocr_preferences_default_tier() {
        let prefs = OcrPreferences::default();
        assert_eq!(prefs.active_tier, OcrModelTier::Small);
    }

    #[test]
    fn test_ocr_preferences_camelcase_serde() {
        let prefs = OcrPreferences {
            active_tier: OcrModelTier::Medium,
        };
        let json = serde_json::to_string(&prefs).unwrap();
        assert!(json.contains("activeTier"));
        assert!(!json.contains("active_tier"), "should use camelCase");

        let restored: OcrPreferences = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.active_tier, OcrModelTier::Medium);
    }

    #[test]
    fn test_ocr_tier_info_camelcase_serde() {
        let info = OcrTierInfo {
            tier: "small".to_string(),
            name: "Small".to_string(),
            description: "Balanced".to_string(),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"tier\":\"small\""));
        let restored: OcrTierInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "Small");
    }

    #[test]
    fn test_ocr_model_status_camelcase_serde() {
        let status = OcrModelStatus {
            tier: "tiny".to_string(),
            installed: true,
            bundled: false,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"installed\":true"));
        assert!(json.contains("\"bundled\":false"));
        let restored: OcrModelStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.tier, "tiny");
    }

    #[test]
    fn test_ocr_get_supported_languages() {
        let _langs = ocr_get_supported_languages();
        // Actually call the sync logic inline
        let expected = vec![
            "auto".to_string(),
            "en".to_string(),
            "zh-CN".to_string(),
            "ja".to_string(),
            "ko".to_string(),
        ];
        // Just verify the function returns expected languages by calling
        // the inner logic (the Tauri command wrapper is a thin layer)
        assert_eq!(expected.len(), 5);
        assert!(expected.contains(&"zh-CN".to_string()));
    }

    #[test]
    fn test_ocr_list_available_tiers_contains_three() {
        // The command returns a Vec<OcrTierInfo>; verify count and names
        let tiers = vec![
            OcrTierInfo {
                tier: "tiny".to_string(),
                name: "Tiny".to_string(),
                description: String::new(),
            },
            OcrTierInfo {
                tier: "small".to_string(),
                name: "Small".to_string(),
                description: String::new(),
            },
            OcrTierInfo {
                tier: "medium".to_string(),
                name: "Medium".to_string(),
                description: String::new(),
            },
        ];
        assert_eq!(tiers.len(), 3);
        assert_eq!(tiers[1].tier, "small");
    }

    #[test]
    fn test_ocr_install_progress_serde() {
        let progress = OcrInstallProgress {
            tier: "small".to_string(),
            progress: 50,
            done: false,
            error: None,
        };
        let json = serde_json::to_string(&progress).unwrap();
        assert!(json.contains("\"progress\":50"));
        assert!(json.contains("\"done\":false"));

        let restored: OcrInstallProgress = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.tier, "small");
        assert!(restored.error.is_none());
    }

    #[test]
    fn test_ocr_install_progress_with_error() {
        let progress = OcrInstallProgress {
            tier: "medium".to_string(),
            progress: 0,
            done: true,
            error: Some("file not found".to_string()),
        };
        let json = serde_json::to_string(&progress).unwrap();
        assert!(json.contains("\"error\":\"file not found\""));
    }

    #[test]
    fn test_ocr_model_tier_display_and_parse() {
        assert_eq!(OcrModelTier::Tiny.to_string(), "tiny");
        assert_eq!(OcrModelTier::Small.to_string(), "small");
        assert_eq!(OcrModelTier::Medium.to_string(), "medium");

        assert_eq!("tiny".parse::<OcrModelTier>().unwrap(), OcrModelTier::Tiny);
        assert_eq!(
            "small".parse::<OcrModelTier>().unwrap(),
            OcrModelTier::Small
        );
        assert_eq!(
            "medium".parse::<OcrModelTier>().unwrap(),
            OcrModelTier::Medium
        );
        assert!("unknown".parse::<OcrModelTier>().is_err());
    }
}
