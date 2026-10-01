//! 加密导出/导入命令（CLI 薄封装）。
//!
//! 实现 `/export` 与 `/import`。实际编排逻辑已下沉到
//! `solosoul-core::export_import::{export_vault, import_vault_into, import_preview}`，
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
    export_vault, import_preview, import_vault_resumable, resume_vault_import,
    CoreImportOperationOutcome, ExportScope, ImportStrategy,
};

use solosoul_core::export_import::operation::{
    import_credential_requirements, ImportCredentialState,
};
use solosoul_core::VaultSession;
use solosoul_vault::ImportOperationRecord;
use zeroize::Zeroizing;

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
    let (file_arg, preview, strategy) = match parse_import_command(args) {
        Ok(ImportCommand::Fresh {
            file_arg,
            preview,
            strategy,
        }) => (file_arg, preview, strategy),
        Ok(ImportCommand::Pending) => return handle_pending_imports(app),
        Ok(ImportCommand::Resume {
            operation_id,
            source_path,
        }) => {
            return handle_resume_import(app, operation_id, source_path.map(PathBuf::from));
        }
        Err(e) => {
            app.error_message = Some(
                if args.contains(&"--resume") || args.contains(&"--pending") {
                    t!(app.i18n, "cmd-import-resume-usage")
                } else {
                    e
                },
            );
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
                app.info_message = Some(t!(
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

    // 非预览模式需要解锁；密码提示等待期间仅保留原会话令牌。
    let account_id = require_unlocked(app)?;
    let session = match app.vault_service.capture_session(&account_id) {
        Ok(session) => session,
        Err(e) => {
            app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e));
            return Ok(());
        }
    };

    let base = app.vault_service.base_path().to_path_buf();
    // RF021：附件密钥绑定原会话，用 Zeroizing 承载；失败不降级为明文导入。
    let vault_att_key = match app.vault_service.attachment_key_for_session(&session) {
        Ok(key) => key,
        Err(e) => {
            app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e));
            return Ok(());
        }
    };
    // Fresh 的身份在首个用户动作同步确定，密码仅在 Zeroizing prompt 回调内存中存在。
    let operation_id = uuid::Uuid::new_v4().to_string();
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
                let result = import_vault_resumable(
                    &app.vault_service,
                    &session,
                    &operation_id,
                    &path,
                    &password,
                    strategy,
                    &base,
                    &vault_att_key,
                );
                publish_import_outcome(app, result);
            }
        }),
    );

    Ok(())
}

/// Resume/Pending 不经过 Fresh 的文件/preview/strategy gating。
enum ImportCommand<'a> {
    Fresh {
        file_arg: Option<&'a str>,
        preview: bool,
        strategy: ImportStrategy,
    },
    Pending,
    Resume {
        operation_id: &'a str,
        source_path: Option<&'a str>,
    },
}

fn parse_import_command<'a>(args: &[&'a str]) -> std::result::Result<ImportCommand<'a>, String> {
    if args.contains(&"--pending") {
        return if args == ["--pending"] {
            Ok(ImportCommand::Pending)
        } else {
            Err("Usage: /import --pending".into())
        };
    }
    if args.contains(&"--resume") {
        return match args {
            ["--resume", operation_id] | ["--resume", operation_id, _]
                if uuid::Uuid::parse_str(operation_id).is_ok()
                    && args.get(2).is_none_or(|path| !path.starts_with("--")) => {
                Ok(ImportCommand::Resume { operation_id, source_path: args.get(2).copied() })
            }
            _ => Err("Usage: /import --resume <operation-id> [source-path]; no strategy or selection options".into()),
        };
    }
    let (file_arg, preview, strategy) = parse_import_args(args)?;
    Ok(ImportCommand::Fresh {
        file_arg,
        preview,
        strategy,
    })
}

fn handle_pending_imports(app: &mut App) -> Result<()> {
    let account_id = require_unlocked(app)?;
    let result = app
        .vault_service
        .capture_session(&account_id)
        .and_then(|session| {
            app.vault_service
                .with_session(&session, |vault| vault.list_import_operations(&account_id))
        });
    match result {
        Ok(operations) if operations.is_empty() => {
            app.info_message = Some(t!(app.i18n, "cmd-import-pending-empty"));
        }
        Ok(operations) => {
            let mut rows = Vec::new();
            for operation in operations {
                let requirements = match import_credential_requirements(&operation) {
                    Ok(value) => value,
                    Err(error) => {
                        app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = error));
                        return Ok(());
                    }
                };
                let count =
                    match solosoul_core::export_import::import_operation_object_count(&operation) {
                        Ok(value) => value.to_string(),
                        Err(error) => {
                            app.error_message =
                                Some(t!(app.i18n, "cmd-operation-failed", err = error));
                            return Ok(());
                        }
                    };
                let source_name = operation
                    .start
                    .plan
                    .get("sourceName")
                    .unwrap_or(&operation.start.plan["requestOptions"]["sourceName"])
                    .as_str()
                    .unwrap_or("");
                rows.push(t!(
                    app.i18n,
                    "cmd-import-pending-entry",
                    id = operation.start.operation_id,
                    name = source_name,
                    phase = match operation.phase {
                        solosoul_vault::ImportOperationPhase::RecordsCommitted =>
                            t!(app.i18n, "cmd-import-phase-records"),
                        solosoul_vault::ImportOperationPhase::Attachments =>
                            t!(app.i18n, "cmd-import-phase-attachments"),
                        solosoul_vault::ImportOperationPhase::Preferences =>
                            t!(app.i18n, "cmd-import-phase-preferences"),
                        solosoul_vault::ImportOperationPhase::Complete =>
                            t!(app.i18n, "cmd-import-phase-complete"),
                        solosoul_vault::ImportOperationPhase::Abandoned =>
                            t!(app.i18n, "cmd-import-phase-abandoned"),
                    },
                    count = count,
                    attachments = operation.attachment_count.to_string(),
                    source = if requirements.source_required {
                        t!(app.i18n, "generic-yes")
                    } else {
                        t!(app.i18n, "generic-no")
                    },
                    password = if requirements.password_required {
                        t!(app.i18n, "generic-yes")
                    } else {
                        t!(app.i18n, "generic-no")
                    }
                ));
            }
            app.info_message = Some(rows.join("\n"));
        }
        Err(error) => app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = error)),
    }
    Ok(())
}

fn handle_resume_import(
    app: &mut App,
    operation_id: &str,
    source_path: Option<PathBuf>,
) -> Result<()> {
    let account_id = require_unlocked(app)?;
    // 在源路径和密码两个提示之前捕获身份，不根据回调时的新账户重新取 Session。
    let session = match app.vault_service.capture_session(&account_id) {
        Ok(value) => value,
        Err(error) => {
            app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = error));
            return Ok(());
        }
    };
    let operation = match app.vault_service.with_session(&session, |vault| {
        vault
            .load_import_operation(&account_id, operation_id)?
            .ok_or_else(|| "__IMPORT_ERR__:OPERATION_NOT_FOUND".into())
    }) {
        Ok(value) => value,
        Err(error) => {
            app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = error));
            return Ok(());
        }
    };
    let attachment_key = match app.vault_service.attachment_key_for_session(&session) {
        Ok(value) => value,
        Err(error) => {
            app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = error));
            return Ok(());
        }
    };
    let requirements = match import_credential_requirements(&operation) {
        Ok(value) => value,
        Err(error) => {
            app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = error));
            return Ok(());
        }
    };
    if requirements.state == ImportCredentialState::RecoveryUnavailable {
        app.error_message = Some(t!(app.i18n, "cmd-import-recovery-unavailable"));
        return Ok(());
    }
    if requirements.source_required && source_path.is_none() {
        prompt::open(
            app,
            PromptSpec::Text {
                label: t!(app.i18n, "prompt-import-resume-source"),
                initial: String::new(),
                mask: false,
                allow_toggle_mask: false,
            },
            Box::new(move |app, result| {
                if let PromptResult::Text(path) = result {
                    let path = path.trim();
                    if path.is_empty() {
                        app.error_message = Some(t!(app.i18n, "cmd-provide-import-path"));
                        return;
                    }
                    continue_resume_import(
                        app,
                        session,
                        operation,
                        Some(PathBuf::from(path)),
                        attachment_key,
                    );
                }
            }),
        );
    } else {
        continue_resume_import(app, session, operation, source_path, attachment_key);
    }
    Ok(())
}

fn continue_resume_import(
    app: &mut App,
    session: VaultSession,
    operation: ImportOperationRecord,
    source_path: Option<PathBuf>,
    attachment_key: Zeroizing<[u8; 32]>,
) {
    if let Err(error) = app.vault_service.with_session(&session, |_| Ok(())) {
        app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = error));
        return;
    }
    let requirements = match import_credential_requirements(&operation) {
        Ok(value) => value,
        Err(error) => {
            app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = error));
            return;
        }
    };
    let base = app.vault_service.base_path().to_path_buf();
    if requirements.password_required {
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
                    let result = resume_vault_import(
                        &app.vault_service,
                        &session,
                        &operation.start.operation_id,
                        source_path.as_deref(),
                        Some(&password),
                        &base,
                        &attachment_key,
                    );
                    publish_import_outcome(app, result);
                }
            }),
        );
    } else {
        // ready staging / Complete 可在原源文件已删除时继续，不再打开密码提示。
        let result = resume_vault_import(
            &app.vault_service,
            &session,
            &operation.start.operation_id,
            source_path.as_deref(),
            None,
            &base,
            &attachment_key,
        );
        publish_import_outcome(app, result);
    }
}

fn publish_import_outcome(
    app: &mut App,
    result: std::result::Result<
        CoreImportOperationOutcome,
        solosoul_core::export_import::ExportError,
    >,
) {
    match result {
        Ok(outcome) if outcome.complete => {
            app.error_message = None;
            app.success_message = Some((
                format!(
                    "{} {}",
                    t!(
                        app.i18n,
                        "cmd-import-success",
                        count = outcome.object_write_count.to_string()
                    ),
                    t!(
                        app.i18n,
                        "cmd-import-complete-operation",
                        id = outcome.operation_id,
                        attachments = outcome.attachment_count.to_string()
                    )
                ),
                Instant::now(),
            ));
        }
        Ok(outcome) => {
            app.success_message = None;
            app.error_message = Some(t!(
                app.i18n,
                "cmd-import-partial-operation",
                id = outcome.operation_id,
                count = outcome.object_write_count.to_string(),
                attachments = outcome.attachment_count.to_string(),
                files = outcome.written_file_count.to_string(),
                err = outcome
                    .error_code
                    .unwrap_or_else(|| "import_operation_incomplete".into())
            ));
        }
        Err(error) => app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = error)),
    }
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

#[cfg(test)]
mod rf021_tests;

#[cfg(test)]
mod rf022_tests;

#[cfg(test)]
mod rf015;
