//! 系统信息命令：/about、/help。

use color_eyre::Result;

use crate::app::{App, AppPhase};

/// 应用信息。
#[derive(Debug, Clone)]
pub struct AboutInfo {
    pub app_name: String,
    pub version: String,
    pub os: String,
    pub arch: String,
    pub data_dir: String,
    pub lock_acquired: bool,
}

/// 执行 `/about`：显示应用与运行环境信息。
pub fn about(app: &mut App) -> Result<()> {
    let info = AboutInfo {
        app_name: "SoloSoul CLI".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        data_dir: app.vault_service.base_path().display().to_string(),
        lock_acquired: app.process_lock.is_some(),
    };
    app.previous_phase = Some(app.phase.clone());
    app.phase = AppPhase::About { info };
    Ok(())
}

/// 执行 `/help [command]`：显示命令帮助。
pub fn help(app: &mut App, topic: Option<&str>) -> Result<()> {
    let topic = topic.map(|s| s.to_lowercase());
    app.previous_phase = Some(app.phase.clone());
    app.phase = AppPhase::Help { topic };
    Ok(())
}

/// 分组命令列表（用于无参数 /help）。
pub const HELP_GROUPS: &[(&str, &[&str])] = &[
    (
        "账户与会话",
        &["/account_list", "/unlock", "/lock (或 /logout)", "/exit"],
    ),
    (
        "Vault 读取",
        &[
            "/list [页面名]",
            "/open <对象ID>",
            "/size",
            "/search <关键词>",
            "/history <对象ID>",
        ],
    ),
    (
        "Vault 写入",
        &[
            "/newpage <名称>",
            "/newobject [页面名]",
            "/edit <对象ID>",
            "/delete <对象ID>",
            "/rollback <对象ID> <快照ID>",
        ],
    ),
    (
        "回收站",
        &["/trash", "/restore <trash_id>", "/purge <trash_id>"],
    ),
    (
        "审计与诊断",
        &[
            "/operation_log [条数]",
            "/export_log [文件名]",
            "/debug_log [文件名]",
            "/doctor",
            "/about",
            "/help [命令]",
        ],
    ),
    (
        "数据可移植性",
        &[
            "/attach list [对象ID] | add <文件> | rename <id> <新名> | delete <id> | restore <id> | purge <id> | cleanup",
            "/backup list | create <名称> | restore <id> | delete <id>",
            "/export [文件] --full | --pages a,b | --objects id1,id2 [--include-attachments]",
            "/import [文件] --preview | --strategy skip|overwrite|merge",
        ],
    ),
    (
        "设置与安全",
        &[
            "/language [语言]",
            "/theme [主题]",
            "/setting <键> <值>",
            "/security password|hint|trash-retention|delete-account",
        ],
    ),
];

/// 命令详细用法。
pub fn command_usage(command: &str) -> Option<&'static str> {
    match command {
        "account_list" | "/account_list" => Some("/account_list\n  列出本地已创建的账户。"),
        "unlock" | "/unlock" => Some("/unlock\n  启动登录向导。单账户时直接进入密码输入。"),
        "lock" | "/lock" | "logout" | "/logout" => {
            Some("/lock 或 /logout\n  立即锁定 Vault 并清除内存中的会话密钥。")
        }
        "exit" | "/exit" => Some("/exit\n  锁定 Vault 并退出 CLI。"),
        "list" | "/list" => Some("/list [页面名]\n  列出所有页面；若提供页面名则列出该页面下的对象。"),
        "open" | "/open" => Some("/open <对象ID>\n  查看指定对象的详情。"),
        "size" | "/size" => Some("/size\n  显示账户统计信息。"),
        "search" | "/search" => Some(
            "/search <关键词>\n  按名称或属性值搜索对象与页面。\n  支持引号包裹多词关键词，例如 /search \"project alpha\"。",
        ),
        "history" | "/history" => Some("/history <对象ID>\n  列出对象的历史快照。"),
        "rollback" | "/rollback" => {
            Some("/rollback <对象ID> <快照ID>\n  将对象恢复到指定快照。操作前会要求确认。")
        }
        "newpage" | "/newpage" => Some("/newpage <名称>\n  创建新页面。"),
        "newobject" | "/newobject" => {
            Some("/newobject [页面名]\n  启动创建对象向导；若指定页面名则跳过页面选择。")
        }
        "edit" | "/edit" => Some("/edit <对象ID>\n  启动对象编辑向导。"),
        "delete" | "/delete" => Some("/delete <对象ID>\n  将对象或页面移入回收站。"),
        "trash" | "/trash" => Some("/trash\n  列出回收站项目。"),
        "restore" | "/restore" => Some("/restore <trash_id>\n  从回收站恢复对象或页面。"),
        "purge" | "/purge" => Some("/purge <trash_id>\n  彻底删除回收站项目，不可恢复。"),
        "operation_log" | "/operation_log" => {
            Some("/operation_log [条数]\n  列出审计日志，默认 100 条。需要已解锁。")
        }
        "export_log" | "/export_log" => Some(
            "/export_log [文件名]\n  将审计日志导出到数据目录的 logs/ 文件夹。\n  默认文件名为 export_audit_log.json。需要已解锁。",
        ),
        "debug_log" | "/debug_log" => Some(
            "/debug_log [文件名]\n  导出包含审计日志与系统信息的诊断包到数据目录的 logs/ 文件夹。",
        ),
        "attach" | "/attach" => Some(
            "/attach list [对象ID] | add <文件> | rename <id> <新名> | delete <id> | restore <id> | purge <id> | cleanup\n  管理对象附件。",
        ),
        "backup" | "/backup" => Some(
            "/backup list | create <名称> | restore <id> | delete <id>\n  创建、列出、恢复或删除 Vault 备份。",
        ),
        "export" | "/export" => Some(
            "/export [文件] --full | --pages a,b | --objects id1,id2 [--include-attachments]\n  将对象加密导出为 .solosoul 包。",
        ),
        "import" | "/import" => Some(
            "/import [文件] --preview | --strategy skip|overwrite|merge\n  从 .solosoul 包导入对象。",
        ),
        "language" | "/language" => Some("/language [语言]\n  获取或设置界面语言，保存于 ui_preferences.json。"),
        "theme" | "/theme" => Some("/theme [主题]\n  获取或设置界面主题，保存于 ui_preferences.json。"),
        "setting" | "/setting" => {
            Some("/setting <键> <值>\n  更新当前账户的加密偏好设置。值会尝试按 JSON 解析，失败则保存为字符串。")
        }
        "security" | "/security" => Some(
            "/security password|hint|trash-retention|delete-account\n  修改主密码、密码提示、回收站保留天数或删除账户。",
        ),
        "doctor" | "/doctor" => Some("/doctor\n  生成数据目录健康诊断报告。"),
        "about" | "/about" => Some("/about\n  显示应用版本、系统与数据目录信息。"),
        "help" | "/help" => Some("/help [命令]\n  显示命令分组列表或指定命令的用法。"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solosoul_core::VaultService;
    use std::sync::Arc;

    fn app_with_temp_dir() -> (App, tempfile::TempDir) {
        let _guard = crate::VAULT_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::TempDir::new().unwrap();
        std::env::set_var("SOLOSOUL_DATA_DIR", dir.path());
        let vault = VaultService::new();
        let app = App::new(Arc::new(vault)).unwrap();
        (app, dir)
    }

    #[test]
    fn test_about_sets_phase() {
        let (mut app, _dir) = app_with_temp_dir();
        about(&mut app).unwrap();
        assert!(matches!(app.phase, AppPhase::About { .. }));
    }

    #[test]
    fn test_help_group_lists_commands() {
        let (mut app, _dir) = app_with_temp_dir();
        help(&mut app, None).unwrap();
        match &app.phase {
            AppPhase::Help { topic } => assert!(topic.is_none()),
            _ => panic!("expected Help phase"),
        }
    }

    #[test]
    fn test_command_usage_known() {
        assert!(command_usage("/search").is_some());
        assert!(command_usage("search").is_some());
    }

    #[test]
    fn test_command_usage_unknown() {
        assert!(command_usage("/foobar").is_none());
    }
}
