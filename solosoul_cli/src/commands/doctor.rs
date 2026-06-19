//! /doctor 诊断命令。

use std::fs;

use color_eyre::Result;
use solosoul_core::process_lock::ProcessLock;

use crate::app::{App, AppPhase};

/// 诊断报告。
#[derive(Debug, Clone, Default)]
pub struct DoctorReport {
    pub data_dir: String,
    pub data_dir_exists: bool,
    pub data_dir_writable: bool,
    pub account_count: usize,
    pub account_errors: Vec<String>,
    pub core_version: String,
    pub vault_version: String,
    pub platform: String,
    pub lock_acquired: bool,
    pub log_path: String,
}

/// 执行 `/doctor`：生成诊断报告并切换 App 状态。
pub fn run(app: &mut App) -> Result<()> {
    let report = build_report(app)?;
    app.previous_phase = Some(app.phase.clone());
    app.phase = AppPhase::Doctor { report };
    Ok(())
}

fn build_report(app: &App) -> Result<DoctorReport> {
    let base_path = app.vault_service.base_path();
    let data_dir = base_path.display().to_string();
    let data_dir_exists = base_path.exists();

    let data_dir_writable = if data_dir_exists {
        base_path
            .metadata()
            .map(|m| !m.permissions().readonly())
            .unwrap_or(false)
    } else {
        false
    };

    let mut account_errors = Vec::new();
    let mut account_count = 0;

    if data_dir_exists {
        for entry in fs::read_dir(base_path)?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let config_path = path.join("config.json");
                if config_path.exists() {
                    account_count += 1;
                    if let Err(e) = fs::read_to_string(&config_path).and_then(|s| {
                        serde_json::from_str::<serde_json::Value>(&s).map_err(|e| e.into())
                    }) {
                        account_errors.push(format!("{}: {}", config_path.display(), e));
                    }
                }
            }
        }
    }

    let lock_acquired = ProcessLock::acquire(base_path).is_ok();

    Ok(DoctorReport {
        data_dir,
        data_dir_exists,
        data_dir_writable,
        account_count,
        account_errors,
        core_version: solosoul_core::VERSION.to_string(),
        // solosoul-vault crate version at CLI compile time
        vault_version: solosoul_vault::VERSION.to_string(),
        platform: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        lock_acquired,
        log_path: app.log_path.display().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use solosoul_core::VaultService;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn app_with_temp_dir() -> (App, TempDir) {
        let _guard = crate::VAULT_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let dir = TempDir::new().unwrap();
        std::env::set_var("SOLOSOUL_DATA_DIR", dir.path());
        let vault = VaultService::new();
        let app = App::new(Arc::new(vault)).unwrap();
        (app, dir)
    }

    #[test]
    fn test_doctor_empty_data_dir() {
        let (mut app, _dir) = app_with_temp_dir();
        run(&mut app).unwrap();
        match &app.phase {
            AppPhase::Doctor { report } => {
                assert!(report.data_dir_exists);
                assert_eq!(report.account_count, 0);
            }
            _ => panic!("expected Doctor phase"),
        }
    }
}
