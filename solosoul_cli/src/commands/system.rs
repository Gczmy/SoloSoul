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

/// 帮助条目（命令 + 描述）。
#[derive(Debug, Clone, Copy)]
pub struct HelpEntry {
    pub command: &'static str,
    pub description: &'static str,
}

/// 执行 `/help [command]`：显示命令帮助。
pub fn help(app: &mut App, topic: Option<&str>) -> Result<()> {
    let topic = topic.map(|s| s.to_lowercase());
    app.previous_phase = Some(app.phase.clone());
    app.phase = AppPhase::Help {
        topic,
        scroll_offset: 0,
    };
    Ok(())
}

/// 分组命令列表（用于无参数 /help）。
pub const HELP_GROUPS: &[(&str, &[HelpEntry])] = &[
    (
        "账户与会话",
        &[
            HelpEntry {
                command: "/account_list",
                description: "显示本地所有账户",
            },
            HelpEntry {
                command: "/unlock (或 /login)",
                description: "启动登录向导解锁 Vault",
            },
            HelpEntry {
                command: "/lock (或 /logout)",
                description: "锁定 Vault 并清除会话密钥",
            },
            HelpEntry {
                command: "/exit",
                description: "锁定 Vault 并退出 CLI",
            },
            HelpEntry {
                command: "Ctrl+L",
                description: "手动锁定 Vault",
            },
        ],
    ),
    (
        "Vault 读取",
        &[
            HelpEntry {
                command: "/list [页面名]",
                description: "列出页面或对象",
            },
            HelpEntry {
                command: "/open <对象ID>",
                description: "查看对象详情",
            },
            HelpEntry {
                command: "/size (或 /status)",
                description: "显示账户统计信息",
            },
            HelpEntry {
                command: "/search <关键词>",
                description: "全局搜索对象与属性",
            },
            HelpEntry {
                command: "/history <对象ID>",
                description: "查看对象历史快照",
            },
        ],
    ),
    (
        "Vault 写入",
        &[
            HelpEntry {
                command: "/newpage <名称>",
                description: "创建新页面",
            },
            HelpEntry {
                command: "/newobject [页面名]",
                description: "创建新对象（向导）",
            },
            HelpEntry {
                command: "/edit <对象ID>",
                description: "编辑对象字段",
            },
            HelpEntry {
                command: "/delete <对象ID>",
                description: "移入回收站",
            },
            HelpEntry {
                command: "/rollback <ID> <快照ID>",
                description: "恢复到指定历史版本",
            },
        ],
    ),
    (
        "回收站",
        &[
            HelpEntry {
                command: "/trash (或 /bin)",
                description: "列出回收站项目",
            },
            HelpEntry {
                command: "/restore <trash_id>",
                description: "从回收站恢复",
            },
            HelpEntry {
                command: "/purge <trash_id>",
                description: "彻底删除（不可恢复）",
            },
        ],
    ),
    (
        "审计与诊断",
        &[
            HelpEntry {
                command: "/operation_log [条数]",
                description: "查看审计日志",
            },
            HelpEntry {
                command: "/export_log [文件名]",
                description: "导出审计日志到文件",
            },
            HelpEntry {
                command: "/debug_log [文件名]",
                description: "导出诊断包",
            },
            HelpEntry {
                command: "/doctor",
                description: "健康诊断报告",
            },
            HelpEntry {
                command: "/about (或 /version)",
                description: "应用信息与版本",
            },
            HelpEntry {
                command: "/help [命令]",
                description: "查看命令帮助",
            },
        ],
    ),
    (
        "数据可移植性",
        &[
            HelpEntry {
                command: "/attach <子命令>",
                description: "管理对象附件",
            },
            HelpEntry {
                command: "/backup <子命令>",
                description: "备份与恢复 Vault",
            },
            HelpEntry {
                command: "/template <子命令>",
                description: "对象模板管理",
            },
            HelpEntry {
                command: "/export [文件] [选项]",
                description: "加密导出 .solosoul 包",
            },
            HelpEntry {
                command: "/import [文件] [选项]",
                description: "从 .solosoul 包导入",
            },
        ],
    ),
    (
        "设置与安全",
        &[
            HelpEntry {
                command: "/language [语言]",
                description: "切换界面语言",
            },
            HelpEntry {
                command: "/theme [主题]",
                description: "切换界面主题",
            },
            HelpEntry {
                command: "/setting <键> <值>",
                description: "修改加密偏好设置",
            },
            HelpEntry {
                command: "/security <子命令>",
                description: "密码/保留期/删除账户",
            },
            HelpEntry {
                command: "/profile [子命令]",
                description: "查看或编辑 Profile",
            },
        ],
    ),
];

/// 命令详细用法。
///
/// 返回完整的用法字符串（含参数说明、示例），用于 `/help <command>` 展示。
/// 映射表通过编译期 phf 或惰性初始化构建，避免手写 40+ 行 match 语句。
pub fn command_usage(command: &str) -> Option<&'static str> {
    let cmd = command.trim_start_matches('/');
    USAGE_MAP.get(cmd).copied()
}

use std::sync::LazyLock;
use std::collections::HashMap;

/// 命令 → 详细用法映射（与 HELP_GROUPS 数据源同步维护）。
///
/// 当在 HELP_GROUPS 中添加新命令时，请同时在此处添加对应的用法文本。
/// 此映射负责提供详细用法（含参数、示例）；HELP_GROUPS 负责提供简短描述。
static USAGE_MAP: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m: HashMap<&'static str, &'static str> = HashMap::new();
    // 别名映射：保持与 HELP_GROUPS 中命令名的关系。
    // 主命令名（无斜杠）→ 用法文本
    macro_rules! usage {
        ($cmd:expr, $text:expr) => { m.insert($cmd, $text); };
    }
    usage!("account_list", "/account_list\n  列出本地已创建的账户。");
    usage!("unlock", "/unlock 或 /login\n  启动登录向导。单账户时直接进入密码输入。");
    usage!("lock", "/lock 或 /logout\n  立即锁定 Vault 并清除内存中的会话密钥。");
    usage!("exit", "/exit\n  锁定 Vault 并退出 CLI。");
    usage!("list", "/list [页面名]\n  列出所有页面；若提供页面名则列出该页面下的对象。");
    usage!("open", "/open <对象ID>\n  查看指定对象的详情。");
    usage!("size", "/size (或 /status /state)\n  显示账户统计信息。");
    usage!("search", "/search <关键词>\n  按名称或属性值搜索对象与页面。\n  支持引号包裹多词关键词，例如 /search \"project alpha\"。");
    usage!("history", "/history <对象ID>\n  列出对象的历史快照。");
    usage!("rollback", "/rollback <对象ID> <快照ID>\n  将对象恢复到指定快照。操作前会要求确认。");
    usage!("newpage", "/newpage <名称>\n  创建新页面。");
    usage!("newobject", "/newobject [页面名]\n  启动创建对象向导；若指定页面名则跳过页面选择。");
    usage!("edit", "/edit <对象ID>\n  启动对象编辑向导。");
    usage!("delete", "/delete <对象ID>\n  将对象或页面移入回收站。");
    usage!("trash", "/trash (或 /bin)\n  列出回收站项目。");
    usage!("restore", "/restore <trash_id>\n  从回收站恢复对象或页面。");
    usage!("purge", "/purge <trash_id>\n  彻底删除回收站项目，不可恢复。");
    usage!("operation_log", "/operation_log [条数]\n  列出审计日志，默认 100 条。需要已解锁。");
    usage!("export_log", "/export_log [文件名]\n  将审计日志导出到数据目录的 logs/ 文件夹。");
    usage!("debug_log", "/debug_log [文件名]\n  导出诊断包到 logs/。");
    usage!("attach", "/attach list|add|rename|delete|restore|purge|cleanup\n  管理对象附件。");
    usage!("backup", "/backup list|create|restore|delete\n  创建、列出、恢复或删除 Vault 备份。");
    usage!("export", "/export [文件] --full|--pages|--objects [--include-attachments]\n  加密导出为 .solosoul 包。");
    usage!("import", "/import [文件] --preview|--strategy skip|overwrite|merge\n  从 .solosoul 包导入对象。");
    usage!("language", "/language [语言]\n  获取或设置界面语言，保存于 ui_preferences.json。");
    usage!("theme", "/theme [主题]\n  获取或设置界面主题，保存于 ui_preferences.json。");
    usage!("setting", "/setting <键> <值>\n  更新当前账户的加密偏好设置。");
    usage!("security", "/security password|hint|trash-retention|delete-account|biometric\n  安全设置。");
    usage!("profile", "/profile | /profile rename|set\n  查看、重命名或编辑当前账户的加密 Profile。");
    usage!("doctor", "/doctor\n  生成数据目录健康诊断报告。");
    usage!("template", "/template | /template show|delete\n  列出、查看或删除对象模板。");
    usage!("about", "/about (或 /version)\n  显示应用版本、系统与数据目录信息。");
    usage!("help", "/help [命令]\n  显示命令分组列表或指定命令的用法。");
    // 别名映射
    for (alias, primary) in &[
        ("login", "unlock"), ("logout", "lock"), ("status", "size"),
        ("state", "size"), ("version", "about"), ("bin", "trash"),
    ] {
        if let Some(usage) = m.get(primary) {
            m.insert(alias, usage);
        }
    }
    m
});

    #[test]
    fn test_command_usage_from_groups() {
        assert!(command_usage("search").is_some());
        assert!(command_usage("/search").is_some());
        assert!(command_usage("/foobar").is_none());
        // 别名
        assert!(command_usage("login").is_some());
        assert!(command_usage("status").is_some());
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
        let vault = VaultService::with_base_path(dir.path().to_path_buf());
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
            AppPhase::Help { topic, .. } => assert!(topic.is_none()),
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

    #[test]
    fn test_command_usage_aliases() {
        assert!(command_usage("login").is_some());
        assert!(command_usage("status").is_some());
    }
}
