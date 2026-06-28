//! /ocr 本地图片 OCR 命令。
//!
//! 一次性调用 `OcrEngine::scan_image` —— 不预热 ort Session，避免 CLI
//! 启动开销。用户可通过环境变量 `SOLOSOUL_OCR_TIER` 切换 tiny/small/medium 档位。
//!
//! 子命令：
//! - `/ocr tiers` —— 列出档位（含本地安装状态）
//! - `/ocr scan <path>` —— 对本地图片执行 OCR
//! - `/ocr status` —— 显示模型目录与已安装档位
//! - `/ocr help` —— 帮助

use crate::app::App;
use color_eyre::Result;
use solosoul_core::ocr::engine::OcrEngine;
use solosoul_core::ocr::model as ocr_model;
use solosoul_core::ocr::types::{OcrModelTier, OcrResult};
use std::path::{Path, PathBuf};

#[cfg(test)]
use solosoul_core::VaultService;

pub fn handle(app: &mut App, argv: &[&str]) -> Result<()> {
    let sub = argv.first().copied().unwrap_or("status");
    match sub {
        "tiers" => {
            tiers(app);
            Ok(())
        }
        "scan" => {
            // /ocr scan [--mrz] <image-path>
            scan(app, &argv[1..]);
            Ok(())
        }
        "status" => {
            status(app);
            Ok(())
        }
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => {
            app.error_message = Some(format!("未知 /ocr 子命令: {}", other));
            Ok(())
        }
    }
}

/// 帮助文本，供 `/ocr help` 显示。
pub fn help_text() -> Vec<&'static str> {
    vec![
        "用法: /ocr <subcommand> [args]",
        "  tiers                       列出可用模型档位 (tiny/small/medium) 及本地安装状态",
        "  scan [--mrz] <image-path>   对指定本地图片执行 OCR;--mrz 触发护照 MRZ 结构化识别",
        "  status                      显示当前模型目录与已安装档位",
        "  help                        显示本帮助",
    ]
}

fn print_help() {
    for line in help_text() {
        println!("{line}");
    }
}

/// 计算 CLI 使用的 model 目录：`{base_path}/models`。
pub fn models_dir(app: &App) -> PathBuf {
    app.vault_service.base_path().join("models")
}

fn tiers(app: &mut App) {
    let base = models_dir(app);
    let entries = build_tiers(&base);
    app.previous_phase = Some(app.phase.clone());
    app.phase = crate::app::AppPhase::OcrResult {
        result: OcrResult {
            text: String::new(),
            confidence: 0.0,
            boxes: Vec::new(),
        },
        source_path: String::new(),
        tiers: Some(entries),
        mrz: None,
    };
}

fn status(app: &mut App) {
    let base = models_dir(app);
    let entries = build_tiers(&base);
    let installed: Vec<String> = entries
        .iter()
        .filter(|t| t.installed)
        .map(|t| t.name.clone())
        .collect();
    let text = if installed.is_empty() {
        format!(
            "模型目录: {}\n未安装任何档位。请从 GUI 安装或下载到该目录。",
            base.display()
        )
    } else {
        format!(
            "模型目录: {}\n已安装: {}",
            base.display(),
            installed.join(", ")
        )
    };

    app.previous_phase = Some(app.phase.clone());
    app.phase = crate::app::AppPhase::OcrResult {
        result: OcrResult {
            text,
            confidence: 0.0,
            boxes: Vec::new(),
        },
        source_path: format!("OCR Status ({} 目录)", base.display()),
        tiers: Some(entries),
        mrz: None,
    };
}

fn build_tiers(base: &Path) -> Vec<crate::screens::ocr_result::TierEntry> {
    [
        OcrModelTier::Tiny,
        OcrModelTier::Small,
        OcrModelTier::Medium,
    ]
    .iter()
    .map(|tier| crate::screens::ocr_result::TierEntry {
        name: tier.to_string(),
        installed: ocr_model::is_model_installed(base, *tier),
        size_mb: tier_size_mb(*tier),
    })
    .collect()
}

fn tier_size_mb(tier: OcrModelTier) -> f32 {
    match tier {
        OcrModelTier::Tiny => 4.5,
        OcrModelTier::Small => 30.0,
        OcrModelTier::Medium => 132.0,
    }
}

/// `/ocr scan [--mrz] <image-path>` — 解析参数、执行 OCR。
///
/// 接受的参数顺序：可任意排列的 flag 集合 + 唯一非 flag 位置参数作为图片路径。
/// 未知 flag 与多余的位置参数都会立即返回错误，不会被静默忽略。
fn scan(app: &mut App, args: &[&str]) {
    let mut mrz_mode = false;
    let mut image_path: Option<&str> = None;
    for &a in args {
        if a == "--mrz" {
            mrz_mode = true;
        } else if a.starts_with("--") {
            app.error_message = Some(format!(
                "/ocr scan: 未知 flag {}。用法: /ocr scan [--mrz] <image-path>",
                a
            ));
            return;
        } else if image_path.is_none() {
            image_path = Some(a);
        } else {
            app.error_message = Some(format!(
                "/ocr scan: 拒绝多余参数 {}。用法: /ocr scan [--mrz] <image-path>",
                a
            ));
            return;
        }
    }

    let image_path = match image_path {
        Some(p) if !p.is_empty() => p,
        _ => {
            app.error_message = Some("用法: /ocr scan [--mrz] <image-path>".to_string());
            return;
        }
    };
    let path = Path::new(image_path);
    if !path.exists() {
        app.error_message = Some(format!("图片不存在: {}", image_path));
        return;
    }

    let tier: OcrModelTier = match std::env::var("SOLOSOUL_OCR_TIER") {
        Ok(s) => match s.parse() {
            Ok(t) => t,
            Err(e) => {
                app.error_message = Some(format!("SOLOSOUL_OCR_TIER 解析失败: {}", e));
                return;
            }
        },
        Err(_) => OcrModelTier::Small,
    };

    let base = models_dir(app);
    if !ocr_model::is_model_installed(&base, tier) {
        app.error_message = Some(format!(
            "{} 档位模型未安装。请先通过 GUI 安装或手动放置到 {}/{}。",
            tier,
            base.display(),
            tier.dir_name()
        ));
        return;
    }

    let mut engine = match OcrEngine::load(&base, tier) {
        Ok(e) => e,
        Err(e) => {
            app.error_message = Some(format!("加载 OCR engine 失败: {}", e));
            return;
        }
    };

    if mrz_mode {
        let mrz_result = match engine.scan_mrz(path) {
            Ok(Some(m)) => m,
            Ok(None) => {
                app.error_message = Some("未在图片中识别到 MRZ 区域".to_string());
                return;
            }
            Err(e) => {
                app.error_message = Some(format!("MRZ 识别失败: {}", e));
                return;
            }
        };

        app.previous_phase = Some(app.phase.clone());
        app.phase = crate::app::AppPhase::OcrResult {
            result: OcrResult {
                // MRZ 模式下 text 仅作为占位回退（render_mrz 专用分支配主导展示）。
                text: String::new(),
                confidence: mrz_result.confidence,
                boxes: Vec::new(),
            },
            source_path: format!("{} (MRZ)", image_path),
            tiers: None,
            mrz: Some(mrz_result),
        };
        return;
    }

    let result = match engine.scan_image(path) {
        Ok(r) => r,
        Err(e) => {
            app.error_message = Some(format!("OCR 扫描失败: {}", e));
            return;
        }
    };

    let source_path = image_path.to_string();
    app.previous_phase = Some(app.phase.clone());
    app.phase = crate::app::AppPhase::OcrResult {
        result,
        source_path,
        tiers: None,
        mrz: None,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppPhase;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn setup_app() -> (App, TempDir) {
        let _guard = crate::VAULT_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let dir = TempDir::new().unwrap();
        std::env::set_var("SOLOSOUL_DATA_DIR", dir.path());
        let vault = VaultService::new();
        vault
            .create_account("OcrTest", crate::TEST_PASSWORD, None)
            .unwrap();
        let app = App::new(Arc::new(vault)).unwrap();
        (app, dir)
    }

    #[test]
    fn ocr_status_no_models_installed() {
        let (mut app, _dir) = setup_app();
        status(&mut app);
        if let AppPhase::OcrResult {
            result,
            source_path,
            ..
        } = &app.phase
        {
            assert!(result.text.contains("未安装"));
            assert!(source_path.contains("Status"));
        } else {
            panic!("expected OcrResult phase, got {:?}", app.phase);
        }
    }

    #[test]
    fn ocr_tiers_lists_three() {
        let (mut app, _dir) = setup_app();
        tiers(&mut app);
        if let AppPhase::OcrResult { tiers: Some(t), .. } = &app.phase {
            assert_eq!(t.len(), 3);
            assert_eq!(t[0].name, "tiny");
            assert_eq!(t[1].name, "small");
            assert_eq!(t[2].name, "medium");
            for entry in t {
                assert!(!entry.installed);
            }
        } else {
            panic!("expected OcrResult with tiers");
        }
    }

    #[test]
    fn ocr_scan_missing_path_sets_error() {
        let (mut app, _dir) = setup_app();
        scan(&mut app, &["/nonexistent/path.png"]);
        assert!(app.error_message.is_some());
    }

    #[test]
    fn ocr_scan_empty_args_sets_error() {
        let (mut app, _dir) = setup_app();
        scan(&mut app, &[]);
        assert!(app.error_message.is_some());
    }

    #[test]
    fn ocr_scan_mrz_flag_without_path_sets_error() {
        let (mut app, _dir) = setup_app();
        scan(&mut app, &["--mrz"]);
        assert!(app.error_message.is_some());
    }

    #[test]
    fn ocr_scan_unknown_extra_arg_sets_error() {
        let (mut app, _dir) = setup_app();
        scan(&mut app, &["/nonexistent/path.png", "--bogus"]);
        // 多余参数必须立即返回"拒绝多余参数"错误，路径是否存在无关。
        // 注: 由于 `--bogus` 含 `--` 前缀,会先命中"未知 flag"分支,
        // 因此断言允许三种文案：拒绝多余参数/未知 flag/拒绝均可。
        let err = app.error_message.expect("error_message 应被设置");
        assert!(
            err.contains("拒绝多余参数") || err.contains("未知 flag") || err.contains("拒绝"),
            "unexpected error message: {err}"
        );
    }

    #[test]
    fn ocr_scan_unknown_flag_sets_error() {
        let (mut app, _dir) = setup_app();
        scan(&mut app, &["--mr", "/some/path.png"]);
        let err = app.error_message.expect("error_message 应被设置");
        assert!(
            err.contains("未知 flag"),
            "expected '未知 flag' in error, got: {err}"
        );
    }

    #[test]
    fn ocr_scan_extra_positional_arg_sets_error() {
        // 两个非 flag 位置参数：第二个命中"拒绝多余参数"分支。
        let (mut app, _dir) = setup_app();
        scan(&mut app, &["/some/path.png", "/another/path.png"]);
        let err = app.error_message.expect("error_message 应被设置");
        assert!(
            err.contains("拒绝多余参数"),
            "expected '拒绝多余参数' in error, got: {err}"
        );
    }

    #[test]
    fn ocr_models_dir_under_vault_base() {
        let (app, dir) = setup_app();
        let expected = dir.path().join("models");
        assert_eq!(models_dir(&app), expected);
    }
}
