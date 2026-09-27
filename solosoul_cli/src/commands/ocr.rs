//! /ocr 本地图片/PDF OCR 命令。
//!
//! 模型加载及识别由 Tasks 直接拥有的受限阻塞任务执行；取消在加载/推理阶段与页边界
//! 生效，主循环持续处理输入、Tick 和锁定。原会话结果只经任务事件接纳。
//!
//! 子命令：
//! - `/ocr tiers` —— 列出档位（含本地安装状态）
//! - `/ocr scan <path>` —— 对本地图片执行 OCR
//! - `/ocr status` —— 显示模型目录与已安装档位
//! - `/ocr help` —— 帮助

use crate::app::{App, AppPhase};
use crate::t;
use crate::tasks::{BlockingTaskState, TaskFailure, TaskId, TaskOutput, BLOCKING_QUEUE_FULL};
use color_eyre::Result;
use solosoul_core::ocr::control::{OcrCancellation, OCR_CANCELLED};
use solosoul_core::ocr::engine::OcrEngine;
use solosoul_core::ocr::model as ocr_model;
use solosoul_core::ocr::types::{MrzResult, OcrModelTier, OcrResult};
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
        "jobs" => {
            show_jobs(app);
            Ok(())
        }
        "result" => {
            show_result(app);
            Ok(())
        }
        "cancel" => {
            cancel(app, &argv[1..]);
            Ok(())
        }
        "help" | "--help" | "-h" => {
            app.info_message = Some(help_text().join("\n"));
            Ok(())
        }
        other => {
            app.error_message = Some(t!(app.i18n, "cmd-unknown-subcommand", cmd = other));
            Ok(())
        }
    }
}

/// 帮助文本，供 `/ocr help` 显示。
pub fn help_text() -> Vec<&'static str> {
    vec![
        "用法: /ocr <subcommand> [args]",
        "  tiers                       列出可用模型档位 (tiny/small/medium) 及本地安装状态",
        "  scan [--mrz] <path>          对本地图片/PDF 执行 OCR；--mrz 识别护照图片",
        "  jobs                        显示后台任务；Esc 请求取消",
        "  cancel [task-id]             取消指定任务；省略 ID 取消全部 OCR",
        "  result                      查看本会话最近的识别结果",
        "  status                      显示当前模型目录与已安装档位",
        "  help                        显示本帮助",
    ]
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
        t!(
            app.i18n,
            "cmd-ocr-no-models",
            path = &base.display().to_string()
        )
    } else {
        t!(
            app.i18n,
            "cmd-ocr-models-status",
            path = &base.display().to_string(),
            installed = &installed.join(", ")
        )
    };

    app.previous_phase = Some(app.phase.clone());
    app.phase = crate::app::AppPhase::OcrResult {
        result: OcrResult {
            text,
            confidence: 0.0,
            boxes: Vec::new(),
        },
        source_path: t!(
            app.i18n,
            "cmd-ocr-status-title",
            path = &base.display().to_string()
        ),
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
        // P133: Vision 为 macOS 系统内置引擎（GUI 专用），CLI 模型清单不包含该档位。
        OcrModelTier::Vision => 0.0,
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
            app.error_message = Some(t!(app.i18n, "cmd-ocr-unknown-flag", flag = a));
            return;
        } else if image_path.is_none() {
            image_path = Some(a);
        } else {
            app.error_message = Some(t!(app.i18n, "cmd-ocr-extra-arg", arg = a));
            return;
        }
    }

    let image_path = match image_path {
        Some(p) if !p.is_empty() => p,
        _ => {
            app.error_message = Some(t!(app.i18n, "cmd-ocr-usage"));
            return;
        }
    };
    let path = Path::new(image_path);
    if !path.exists() {
        app.error_message = Some(t!(app.i18n, "cmd-ocr-image-not-found", path = image_path));
        return;
    }

    let tier: OcrModelTier = match std::env::var("SOLOSOUL_OCR_TIER") {
        Ok(s) => match s.parse() {
            Ok(t) => t,
            Err(e) => {
                app.error_message = Some(t!(app.i18n, "cmd-ocr-env-parse-failed", err = e));
                return;
            }
        },
        Err(_) => OcrModelTier::Small,
    };

    let base = models_dir(app);
    if !ocr_model::is_model_installed(&base, tier) {
        app.error_message = Some(t!(
            app.i18n,
            "cmd-ocr-tier-not-installed",
            tier = tier.to_string(),
            path = format!("{}/{}", base.display(), tier.dir_name())
        ));
        return;
    }

    let request = OcrRequest {
        path: path.to_path_buf(),
        models_dir: base,
        tier,
        mrz: mrz_mode,
    };
    if let Err(error) = start_scan_with(app, request, OcrEngine::load) {
        app.error_message = Some(if error == BLOCKING_QUEUE_FULL {
            t!(app.i18n, "ocr-queue-full")
        } else {
            error
        });
    }
}

pub(crate) struct OcrTask {
    pub source_path: String,
    pub status: BlockingTaskState,
}

struct OcrRequest {
    path: PathBuf,
    models_dir: PathBuf,
    tier: OcrModelTier,
    mrz: bool,
}

// 只替换推理边界的测试接缝；生产路径始终调用共享引擎及其原生取消点。
trait ScanEngine {
    fn image(&mut self, path: &Path, cancel: &OcrCancellation) -> Result<OcrResult, String>;
    fn pdf(&mut self, path: &Path, cancel: &OcrCancellation) -> Result<OcrResult, String>;
    fn mrz(&mut self, path: &Path, cancel: &OcrCancellation) -> Result<Option<MrzResult>, String>;
}

impl ScanEngine for OcrEngine {
    fn image(&mut self, path: &Path, cancel: &OcrCancellation) -> Result<OcrResult, String> {
        self.scan_image_cancellable(path, cancel)
    }
    fn pdf(&mut self, path: &Path, cancel: &OcrCancellation) -> Result<OcrResult, String> {
        self.scan_pdf_cancellable(path, cancel)
    }
    fn mrz(&mut self, path: &Path, cancel: &OcrCancellation) -> Result<Option<MrzResult>, String> {
        self.scan_mrz_cancellable(path, cancel)
    }
}

fn start_scan_with<E: ScanEngine + 'static>(
    app: &mut App,
    request: OcrRequest,
    load: impl FnOnce(&Path, OcrModelTier) -> Result<E, String> + Send + 'static,
) -> Result<TaskId, String> {
    clear_stale(app);
    let account = super::require_unlocked(app).map_err(|e| e.to_string())?;
    let session = app
        .vault_service
        .capture_session(&account)
        .map_err(|e| e.to_string())?;
    let owner = session.clone();
    let source_path = if request.mrz {
        format!("{} (MRZ)", request.path.display())
    } else {
        request.path.display().to_string()
    };
    let worker_path = source_path.clone();
    let locale = app.i18n.locale.clone();
    let id = app.tasks.spawn_blocking(session, move |context, cancel| {
        let i18n = crate::i18n::I18n::new(&locale);
        let check = || cancel.check().map_err(|_| TaskFailure::Cancelled);
        let fail = |key: &str, error: String| {
            if cancel.is_cancelled() || error == OCR_CANCELLED {
                TaskFailure::Cancelled
            } else {
                TaskFailure::Failed(i18n.t_args(key, &[("err", &error)]))
            }
        };
        check()?;
        let loaded = load(&request.models_dir, request.tier);
        // 模型加载不可被强杀；返回后先检查取消，再解析结果或进入推理。
        check()?;
        let mut engine = loaded.map_err(|e| fail("cmd-ocr-engine-failed", e))?;
        let (result, mrz) = if request.mrz {
            let mrz = engine
                .mrz(&request.path, &cancel)
                .map_err(|e| fail("cmd-ocr-mrz-failed", e))?;
            check()?;
            let mrz = mrz.ok_or_else(|| TaskFailure::Failed(i18n.t("cmd-ocr-mrz-not-found")))?;
            (
                OcrResult {
                    text: String::new(),
                    confidence: mrz.confidence,
                    boxes: Vec::new(),
                },
                Some(mrz),
            )
        } else {
            let is_pdf = request
                .path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"));
            let result = if is_pdf {
                engine.pdf(&request.path, &cancel)
            } else {
                engine.image(&request.path, &cancel)
            };
            (result.map_err(|e| fail("cmd-ocr-scan-failed", e))?, None)
        };
        drop(engine);
        check()?;
        // JSON 只用于线程间拥有完整 DTO；原会话门闩内仅提交已完成的值。
        let result_json =
            serde_json::to_string(&result).map_err(|e| TaskFailure::Failed(e.to_string()))?;
        let mrz_json = mrz
            .map(|m| serde_json::to_string(&m))
            .transpose()
            .map_err(|e| TaskFailure::Failed(e.to_string()))?;
        check()?;
        context.commit(|| {
            Ok(TaskOutput::OcrCompleted {
                result_json,
                source_path: worker_path,
                mrz_json,
            })
        })
    })?;
    app.ocr_session = Some(owner);
    app.ocr_tasks.insert(
        id,
        OcrTask {
            source_path,
            status: BlockingTaskState::Queued,
        },
    );
    show_jobs(app);
    Ok(id)
}

pub(crate) fn decode_result(
    result_json: &str,
    source_path: &str,
    mrz_json: Option<&str>,
) -> Option<AppPhase> {
    let result: OcrResult = serde_json::from_str(result_json).ok()?;
    let mrz: Option<MrzResult> = mrz_json.map(serde_json::from_str).transpose().ok()?;
    Some(AppPhase::OcrResult {
        result,
        source_path: source_path.to_string(),
        tiers: None,
        mrz,
    })
}

pub(crate) fn clear_stale(app: &mut App) {
    if app
        .ocr_session
        .as_ref()
        .is_some_and(|session| app.vault_service.with_session(session, |_| Ok(())).is_err())
    {
        app.ocr_tasks.clear();
        app.last_ocr_result = None;
        app.ocr_session = None;
        if matches!(app.phase, AppPhase::OcrResult { .. } | AppPhase::OcrTasks) {
            app.phase = AppPhase::Locked;
        }
        if matches!(
            app.previous_phase,
            Some(AppPhase::OcrResult { .. } | AppPhase::OcrTasks)
        ) {
            app.previous_phase = None;
        }
    }
}

fn show_jobs(app: &mut App) {
    clear_stale(app);
    if super::require_unlocked(app).is_err() {
        return;
    }
    if !matches!(app.phase, AppPhase::OcrTasks) {
        app.previous_phase = Some(app.phase.clone());
        app.phase = AppPhase::OcrTasks;
    }
}

fn show_result(app: &mut App) {
    clear_stale(app);
    if super::require_unlocked(app).is_err() {
        return;
    }
    if let Some(result) = app.last_ocr_result.clone() {
        if !matches!(app.phase, AppPhase::OcrResult { .. }) {
            app.previous_phase = Some(app.phase.clone());
        }
        app.phase = result;
    } else {
        app.info_message = Some(t!(app.i18n, "ocr-no-result"));
    }
}

pub(crate) fn cancel(app: &mut App, args: &[&str]) {
    clear_stale(app);
    let ids: Vec<TaskId> = match args {
        [] => app.ocr_tasks.keys().copied().collect(),
        [id] => match uuid::Uuid::parse_str(id)
            .ok()
            .map(TaskId)
            .filter(|id| app.ocr_tasks.contains_key(id))
        {
            Some(id) => vec![id],
            None => {
                app.error_message = Some(t!(app.i18n, "ocr-task-not-found"));
                return;
            }
        },
        _ => {
            app.error_message = Some(t!(app.i18n, "ocr-cancel-usage"));
            return;
        }
    };
    for id in ids {
        app.tasks.request_cancel(id);
    }
    // 状态由 poll_events 的真实 BlockingState/终态发布，取消请求不提前删除任务。
}

#[cfg(test)]
mod rf215_pdf_tests;
#[cfg(test)]
mod rf215_tests;

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
        let vault = VaultService::with_base_path(dir.path().to_path_buf());
        vault
            .create_account("OcrTest", crate::TEST_PASSWORD, None)
            .unwrap();
        let mut app = App::new(Arc::new(vault)).unwrap();
        // 这些用例断言中文文案，不依赖运行测试的系统语言。
        app.i18n.set_locale("zh-CN");
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
