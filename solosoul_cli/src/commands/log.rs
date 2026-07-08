//! 审计日志命令 /operation_log、/export_log。

use color_eyre::Result;

use crate::app::{App, AppPhase};
use crate::commands::require_unlocked;

/// 执行 `/operation_log [limit]`：列出审计日志。
pub fn operation_log(app: &mut App, limit: Option<&str>) -> Result<()> {
    let account_id = require_unlocked(app)?;
    let limit = limit
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(100)
        .max(1);

    let vault = app
        .vault_service
        .get_vault_store()
        .ok_or_else(|| color_eyre::eyre::eyre!("Vault 未打开"))?;

    let entries = vault
        .list_audit_log(limit)
        .map_err(|e| color_eyre::eyre::eyre!(e))?;
    app.previous_phase = Some(app.phase.clone());
    app.phase = AppPhase::OperationLog {
        account_id,
        entries,
        selected: 0,
    };
    Ok(())
}

/// 执行 `/export_log [file-name]`：导出审计日志到数据目录 logs/ 下。
pub fn export_log(app: &mut App, file_name: Option<&str>) -> Result<()> {
    let _account_id = require_unlocked(app)?;
    let vault = app
        .vault_service
        .get_vault_store()
        .ok_or_else(|| color_eyre::eyre::eyre!("Vault 未打开"))?;

    let entries = vault
        .list_audit_log(10000)
        .map_err(|e| color_eyre::eyre::eyre!(e))?;
    let json = serde_json::to_string_pretty(&entries).map_err(|e| {
        app.error_message = Some(format!("序列化日志失败: {}", e));
        color_eyre::eyre::eyre!(e)
    })?;

    let logs_dir = app.vault_service.base_path().join("logs");
    std::fs::create_dir_all(&logs_dir).map_err(|e| {
        app.error_message = Some(format!("创建日志目录失败: {}", e));
        color_eyre::eyre::eyre!(e)
    })?;

    let file_name = file_name.unwrap_or("export_audit_log.json");
    let file_name = std::path::Path::new(file_name)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "export_audit_log.json".to_string());
    let path = logs_dir.join(&file_name);

    std::fs::write(&path, &json).map_err(|e| {
        app.error_message = Some(format!("写入导出文件失败: {}", e));
        color_eyre::eyre::eyre!(e)
    })?;

    app.error_message = Some(format!("审计日志已导出至: {}", path.display()));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn test_operation_log_requires_unlock() {
        let _guard = crate::VAULT_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::TempDir::new().unwrap();
        let vault = VaultService::with_base_path(dir.path().to_path_buf());
        let account = vault
            .create_account("Test", crate::TEST_PASSWORD, None)
            .unwrap();
        let account_id = account["id"].as_str().unwrap().to_string();
        vault.lock();
        let mut app = App::new(Arc::new(vault)).unwrap();

        assert!(operation_log(&mut app, None).is_err());
        assert!(app.error_message.is_some());
        assert!(!matches!(app.phase, AppPhase::OperationLog { .. }));

        // 解锁后应成功
        crate::commands::auth::unlock(&mut app).unwrap();
        for c in crate::TEST_PASSWORD.chars() {
            app.handle_event(crate::events::Event::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char(c),
            )))
            .unwrap();
        }
        app.handle_event(crate::events::Event::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Enter,
        )))
        .unwrap();
        assert_eq!(app.vault_service.get_current_account(), Some(account_id));

        operation_log(&mut app, Some("10")).unwrap();
        assert!(matches!(app.phase, AppPhase::OperationLog { .. }));
    }

    #[test]
    fn test_export_log_creates_file() {
        let (mut app, _id, _dir) = unlocked_app();
        export_log(&mut app, Some("test_export.json")).unwrap();
        let path = app
            .vault_service
            .base_path()
            .join("logs")
            .join("test_export.json");
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed.is_array());
    }
}
