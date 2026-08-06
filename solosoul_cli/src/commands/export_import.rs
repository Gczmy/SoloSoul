//! 加密导出/导入命令（CLI 薄封装）。
//!
//! 实现 `/export` 与 `/import`。实际编排逻辑已下沉到
//! `solosoul-core::export_import::{export_vault, import_vault, import_preview}`，
//! 本文件只负责：
//! - 参数解析
//! - 密码模态提示
//! - 密码强度校验（非主密码、长度/组成）
//! - 路径解析
//! - 终端输出

use std::path::{Path, PathBuf};

use color_eyre::Result;
use std::time::Instant;

use solosoul_core::export_import::{
    export_vault, import_preview, import_vault, ExportScope, ImportStrategy,
};

use crate::app::App;
use crate::commands::require_unlocked;
use crate::t;
use crate::widgets::prompt::{self, PromptResult, PromptSpec};

// ── 命令入口 ─────────────────────────────────────────────

/// 命令入口。`args[0]` 为 `/export` 或 `/import`。
pub fn handle(app: &mut App, args: &[&str]) -> Result<()> {
    let base = args.first().copied().unwrap_or("");
    match base {
        "/export" => handle_export(app, &args[1..]),
        "/import" => handle_import(app, &args[1..]),
        _ => {
            app.error_message = Some(t!(app.i18n, "cmd-export-import-unknown", cmd = base));
            Ok(())
        }
    }
}

// ── 导出 ──────────────────────────────────────────────────

fn handle_export(app: &mut App, args: &[&str]) -> Result<()> {
    require_unlocked(app)?;

    let (file_arg, scope) = match parse_export_args(args) {
        Ok(v) => v,
        Err(e) => {
            app.error_message = Some(e);
            return Ok(());
        }
    };

    let base = app.vault_service.base_path().to_path_buf();
    let path = match resolve_export_path(&base, file_arg) {
        Ok(p) => p,
        Err(e) => {
            app.error_message = Some(e);
            return Ok(());
        }
    };

    let vault = match app.vault_service.get_vault_store() {
        Some(v) => v,
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-vault-not-open"));
            return Ok(());
        }
    };

    let account_id = match app.vault_service.get_current_account() {
        Some(id) => id,
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-account-not-found"));
            return Ok(());
        }
    };

    let path_clone = path;
    let base_clone = base;
    prompt::open(
        app,
        PromptSpec::Text {
            label: t!(app.i18n, "prompt-export-password"),
            initial: String::new(),
            mask: true,
            allow_toggle_mask: true,
        },
        Box::new(move |app, result| {
            if let PromptResult::Text(password) = result {
                // 校验密码
                if let Err(e) = validate_export_password(app, &password) {
                    app.error_message = Some(e);
                    return;
                }

                match export_vault(
                    &vault,
                    &account_id,
                    &password,
                    &path_clone,
                    &scope,
                    &base_clone,
                ) {
                    Ok(count) => {
                        app.success_message = Some((
                            t!(
                                app.i18n,
                                "cmd-export-success",
                                count = count.to_string(),
                                path = path_clone.display().to_string()
                            ),
                            Instant::now(),
                        ));
                    }
                    Err(e) => {
                        app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e));
                    }
                }
            }
        }),
    );

    Ok(())
}

// ── 导入 ──────────────────────────────────────────────────

fn handle_import(app: &mut App, args: &[&str]) -> Result<()> {
    let (file_arg, preview, strategy) = match parse_import_args(args) {
        Ok(v) => v,
        Err(e) => {
            app.error_message = Some(e);
            return Ok(());
        }
    };

    let file_arg = match file_arg {
        Some(f) => f,
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-provide-import-path"));
            return Ok(());
        }
    };
    let path = PathBuf::from(file_arg);

    if preview {
        match import_preview(&path) {
            Ok(info) => {
                app.error_message = Some(t!(
                    app.i18n,
                    "cmd-import-preview",
                    version = info.version,
                    count = info.object_count.to_string(),
                    has = if info.has_attachments {
                        t!(app.i18n, "generic-yes")
                    } else {
                        t!(app.i18n, "generic-no")
                    },
                    hint = info
                        .password_hint
                        .unwrap_or_else(|| t!(app.i18n, "generic-none"))
                ));
            }
            Err(e) => {
                app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e));
            }
        }
        return Ok(());
    }

    // 非预览模式需要解锁
    require_unlocked(app)?;

    let vault = match app.vault_service.get_vault_store() {
        Some(v) => v,
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-vault-not-open"));
            return Ok(());
        }
    };

    let account_id = match app.vault_service.get_current_account() {
        Some(id) => id,
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-account-not-found"));
            return Ok(());
        }
    };

    let base = app.vault_service.base_path().to_path_buf();
    prompt::open(
        app,
        PromptSpec::Text {
            label: t!(app.i18n, "prompt-import-password"),
            initial: String::new(),
            mask: true,
            allow_toggle_mask: true,
        },
        Box::new(move |app, result| {
            if let PromptResult::Text(password) = result {
                match import_vault(&vault, &account_id, &path, &password, strategy, &base) {
                    Ok(count) => {
                        app.success_message = Some((
                            t!(app.i18n, "cmd-import-success", count = count.to_string()),
                            Instant::now(),
                        ));
                    }
                    Err(e) => {
                        app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e));
                    }
                }
            }
        }),
    );

    Ok(())
}

// ── 参数解析 ──────────────────────────────────────────────

fn parse_export_args<'a>(
    args: &[&'a str],
) -> std::result::Result<(Option<&'a str>, ExportScope), String> {
    let mut file_arg: Option<&str> = None;
    let mut scope = ExportScope::default();
    let mut iter = args.iter().peekable();

    while let Some(arg) = iter.next() {
        if arg.starts_with("--") {
            match *arg {
                "--full" => scope.full = true,
                "--include-attachments" => scope.include_attachments = true,
                "--pages" => {
                    let list = iter
                        .next()
                        .ok_or("--pages requires a comma-separated list of page IDs")?;
                    scope.selected_page_ids = list.split(',').map(String::from).collect();
                }
                "--objects" => {
                    let list = iter
                        .next()
                        .ok_or("--objects requires a comma-separated list of object IDs")?;
                    scope.selected_object_ids = list.split(',').map(String::from).collect();
                }
                other => return Err(format!("Unknown export option: {}", other)),
            }
        } else if file_arg.is_none() {
            file_arg = Some(*arg);
        } else {
            return Err("Extra file argument".to_string());
        }
    }

    if !scope.full && scope.selected_page_ids.is_empty() && scope.selected_object_ids.is_empty() {
        return Err("Please specify one of: --full, --pages, or --objects".to_string());
    }

    Ok((file_arg, scope))
}

fn parse_import_args<'a>(
    args: &[&'a str],
) -> std::result::Result<(Option<&'a str>, bool, ImportStrategy), String> {
    let mut file_arg: Option<&str> = None;
    let mut preview = false;
    let mut strategy = ImportStrategy::Overwrite;
    let mut iter = args.iter().peekable();

    while let Some(arg) = iter.next() {
        if arg.starts_with("--") {
            match *arg {
                "--preview" => preview = true,
                "--strategy" => {
                    let value = iter.next().ok_or("--strategy requires a strategy value")?;
                    strategy = match *value {
                        "skip" => ImportStrategy::SkipExisting,
                        "overwrite" => ImportStrategy::Overwrite,
                        "merge" => ImportStrategy::Merge,
                        other => return Err(format!("Unknown import strategy: {}", other)),
                    };
                }
                other => return Err(format!("Unknown import option: {}", other)),
            }
        } else if file_arg.is_none() {
            file_arg = Some(*arg);
        } else {
            return Err("Extra file argument".to_string());
        }
    }

    Ok((file_arg, preview, strategy))
}

// ── 路径解析 ──────────────────────────────────────────────

fn resolve_export_path(
    base: &Path,
    file_arg: Option<&str>,
) -> std::result::Result<PathBuf, String> {
    match file_arg {
        None => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| base.to_path_buf());
            let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
            Ok(cwd.join(format!("solosoul_export_{}.solosoul", ts)))
        }
        Some(arg) => {
            let exports_dir = base.join("exports");
            std::fs::create_dir_all(&exports_dir).map_err(|e| e.to_string())?;
            let file_name = Path::new(arg)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "export.solosoul".to_string());
            let mut path = exports_dir.join(file_name);
            if path.extension() != Some(std::ffi::OsStr::new("solosoul")) {
                path.set_extension("solosoul");
            }
            Ok(path)
        }
    }
}

// ── 密码校验 ──────────────────────────────────────────────

/// 校验导出密码强度并确认其不是主密码。
fn validate_export_password(app: &App, password: &str) -> std::result::Result<(), String> {
    if password.len() < 8 {
        return Err(t!(app.i18n, "cmd-export-password-too-short"));
    }
    let has_letter = password.chars().any(|c| c.is_ascii_alphabetic());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    if !has_letter || !has_digit {
        return Err(t!(app.i18n, "cmd-export-password-complexity"));
    }

    let account_id = app
        .vault_service
        .get_current_account()
        .ok_or_else(|| t!(app.i18n, "cmd-account-not-found-generic"))?;
    match app.vault_service.verify_password(&account_id, password) {
        Ok(true) => Err(t!(app.i18n, "cmd-export-password-same-as-master")),
        Ok(false) => Ok(()),
        Err(e) => Err(t!(app.i18n, "cmd-verify-master-failed", err = &e)),
    }
}

// ════════════════════════════════════════════════════════════════
// Tests
// ════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use solosoul_core::VaultService;
    use std::sync::Arc;

    fn unlocked_app() -> (App, String, tempfile::TempDir) {
        let _guard = crate::VAULT_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::TempDir::new().unwrap();
        let vault = VaultService::with_base_path(dir.path().to_path_buf());
        let account = vault
            .create_account("Test", crate::TEST_PASSWORD, None)
            .unwrap();
        let account_id = account["id"].as_str().unwrap().to_string();
        let app = App::new(Arc::new(vault)).unwrap();
        (app, account_id, dir)
    }

    #[test]
    fn test_export_password_same_as_master_rejected() {
        let (app, _account_id, _dir) = unlocked_app();

        // validate_export_password 需要 App 引用，这里直接验证逻辑。
        // 主密码本身应通过 validate_export_password （返回错误即通过）。
        let result = validate_export_password(&app, crate::TEST_PASSWORD);
        assert!(
            result.is_err(),
            "应拒绝与主密码相同的导出密码: {:?}",
            result
        );

        // 正确的导出密码应通过校验
        let result = validate_export_password(&app, crate::TEST_EXPORT_PASSWORD);
        assert!(result.is_ok(), "正确导出密码应通过校验: {:?}", result);

        // 过短的密码应被拒绝
        let result = validate_export_password(&app, "Ab1");
        assert!(result.is_err(), "过短密码应被拒绝");

        // 纯字母应被拒绝
        let result = validate_export_password(&app, "abcdefgh");
        assert!(result.is_err(), "纯字母密码应被拒绝");
    }

    #[test]
    fn test_parse_export_args_full() {
        let args = vec!["output.solosoul", "--full", "--include-attachments"];
        let (file, scope) = parse_export_args(&args).unwrap();
        assert_eq!(file, Some("output.solosoul"));
        assert!(scope.full);
        assert!(scope.include_attachments);
    }

    #[test]
    fn test_parse_export_args_pages() {
        let args = vec!["--pages", "identity,travel", "--objects", "obj1"];
        let (file, scope) = parse_export_args(&args).unwrap();
        assert!(file.is_none());
        assert!(!scope.full);
        assert_eq!(scope.selected_page_ids, vec!["identity", "travel"]);
        assert_eq!(scope.selected_object_ids, vec!["obj1"]);
    }

    #[test]
    fn test_parse_export_args_no_scope_errors() {
        let result = parse_export_args(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_import_args_preview() {
        let args = vec!["test.solosoul", "--preview"];
        let (file, preview, strategy) = parse_import_args(&args).unwrap();
        assert_eq!(file, Some("test.solosoul"));
        assert!(preview);
        assert!(matches!(strategy, ImportStrategy::Overwrite));
    }

    #[test]
    fn test_parse_import_args_strategy() {
        let args = vec!["test.solosoul", "--strategy", "skip"];
        let (file, _preview, strategy) = parse_import_args(&args).unwrap();
        assert_eq!(file, Some("test.solosoul"));
        assert!(matches!(strategy, ImportStrategy::SkipExisting));
    }
}
