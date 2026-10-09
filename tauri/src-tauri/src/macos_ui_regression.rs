//! FE2 完整客户端验收的隔离入口。只编入 macOS debug，业务 UI / IPC 使用正式实现。
//! 与 Windows native-perf 无关；不更改 HOME，不加载或搬移正式账户。

use serde::{Deserialize, Serialize};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub(crate) const BUNDLE_IDENTIFIER: &str = "com.solosoul.fe2.macos";
const MARKER: &str = "fe2-macos-owned.json";
const PREFIX: &str = "solosoul-fe2-macos-";
const ACCOUNT_NAME: &str = "FE2 macOS 合成验收账户";
const SECOND_ACCOUNT_NAME: &str = "FE2 macOS 外观切换账户";
// 公开合成测试凭据，不用于真实账户，也不写日志 / marker。
const PASSWORD: &str = "FE2-Mac-Synthetic-2026!";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Configuration {
    version: u32,
    root: PathBuf,
    pub(crate) identifier: String,
    account_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    second_account_id: Option<String>,
}

static CONFIG: OnceLock<Configuration> = OnceLock::new();

fn private_directory(path: &Path) -> Result<(), String> {
    std::fs::create_dir(path).map_err(|error| error.to_string())?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
        .map_err(|error| error.to_string())
}

fn private_file(path: &Path, contents: &[u8]) -> Result<(), String> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|error| error.to_string())?;
    file.write_all(contents)
        .map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())
}

pub(crate) fn prepare() -> Result<PathBuf, String> {
    prepare_fixture(false)
}

pub(crate) fn prepare_account_switch() -> Result<PathBuf, String> {
    prepare_fixture(true)
}

fn prepare_fixture(account_switch: bool) -> Result<PathBuf, String> {
    let temporary = std::env::temp_dir()
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let id = uuid::Uuid::new_v4().simple().to_string();
    let root = temporary.join(format!("{PREFIX}{id}"));
    private_directory(&root)?;
    for name in ["vault", "app-data", "plugins"] {
        private_directory(&root.join(name))?;
    }
    let service = solosoul_core::VaultService::try_with_base_path(root.join("vault"))?;
    let account = service.create_account(ACCOUNT_NAME, PASSWORD, Some("仅用于 FE2 隔离验收"))?;
    let account_id = account["id"]
        .as_str()
        .ok_or("合成账户创建结果缺少 id")?
        .to_owned();
    let second_account_id = if account_switch {
        let account = service.create_account(
            SECOND_ACCOUNT_NAME,
            PASSWORD,
            Some("仅用于 FE2 账户外观切换验收"),
        )?;
        Some(
            account["id"]
                .as_str()
                .ok_or("第二合成账户创建结果缺少 id")?
                .to_owned(),
        )
    } else {
        None
    };
    service.lock();
    drop(service);
    private_file(
        &root.join("vault/ui_preferences.json"),
        br#"{"theme":"light","accentColor":"ocean","customAccentHex":"","defaultLightTheme":"warm-stone","defaultDarkTheme":"warm-stone-dark","reduceMotion":false,"androidGlass":"local","language":"zh-CN","hasSeenOnboarding":true,"notificationPermissionRequested":true}"#,
    )?;
    let configuration = Configuration {
        version: 1,
        root: root.clone(),
        identifier: format!("{BUNDLE_IDENTIFIER}.{id}"),
        account_id,
        second_account_id,
    };
    private_file(
        &root.join(MARKER),
        &serde_json::to_vec_pretty(&configuration).map_err(|error| error.to_string())?,
    )?;
    validate(&root)?;
    Ok(root)
}

/// 拒绝外部路径、软链接和不属于合成夹具的账户；不尝试修复或回退目录。
fn validate(root: &Path) -> Result<Configuration, String> {
    let temporary = std::env::temp_dir()
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let canonical = root.canonicalize().map_err(|error| error.to_string())?;
    if canonical != root || canonical.parent() != Some(temporary.as_path()) {
        return Err("验收根目录必须是临时目录的规范化直接子目录".into());
    }
    let id = root
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_prefix(PREFIX))
        .ok_or("验收根目录前缀无效")?;
    let uuid = uuid::Uuid::parse_str(id).map_err(|_| "验收根目录 id 无效")?;
    if uuid.simple().to_string() != id {
        return Err("验收根目录 id 格式无效".into());
    }
    reject_links(root)?;
    let owner = std::fs::metadata(&temporary)
        .map_err(|error| error.to_string())?
        .uid();
    for name in ["", "vault", "app-data", "plugins"] {
        let metadata = std::fs::metadata(root.join(name)).map_err(|error| error.to_string())?;
        if !metadata.is_dir() || metadata.uid() != owner || metadata.mode() & 0o077 != 0 {
            return Err("验收目录必须由当前临时目录用户持有且仅用户可访问".into());
        }
    }
    let marker = root.join(MARKER);
    let metadata = std::fs::metadata(&marker).map_err(|error| error.to_string())?;
    if !metadata.is_file() || metadata.uid() != owner || metadata.mode() & 0o077 != 0 {
        return Err("验收 marker 权限无效".into());
    }
    let configuration: Configuration =
        serde_json::from_slice(&std::fs::read(&marker).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    if configuration.version != 1
        || configuration.root != root
        || configuration.identifier != format!("{BUNDLE_IDENTIFIER}.{id}")
    {
        return Err("验收 marker 与根目录不匹配".into());
    }
    let accounts: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join("vault/accounts.json")).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let accounts = accounts.as_array().ok_or("验收账户列表格式无效")?;
    // 双账户也只允许 marker 中明确列出的合成身份；旧单账户 marker 保持兼容。
    let mut expected = vec![(configuration.account_id.as_str(), ACCOUNT_NAME)];
    if let Some(second_id) = configuration.second_account_id.as_deref() {
        if second_id == configuration.account_id {
            return Err("验收 marker 的账户 id 重复".into());
        }
        expected.push((second_id, SECOND_ACCOUNT_NAME));
    }
    if accounts.len() != expected.len()
        || expected.iter().any(|(id, name)| {
            accounts
                .iter()
                .filter(|account| account["id"] == *id && account["name"] == *name)
                .count()
                != 1
        })
    {
        return Err("验收目录包含非夹具账户".into());
    }
    for entry in std::fs::read_dir(root.join("vault")).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if (name.starts_with("acc_") || name.starts_with("acc-"))
            && !expected.iter().any(|(id, _)| name == *id)
        {
            return Err("验收目录包含非夹具账户目录".into());
        }
    }
    Ok(configuration)
}

fn reject_links(directory: &Path) -> Result<(), String> {
    let metadata = std::fs::symlink_metadata(directory).map_err(|error| error.to_string())?;
    if metadata.file_type().is_symlink() {
        return Err("验收目录不能包含软链接".into());
    }
    if metadata.is_dir() {
        for entry in std::fs::read_dir(directory).map_err(|error| error.to_string())? {
            reject_links(&entry.map_err(|error| error.to_string())?.path())?;
        }
    }
    Ok(())
}

pub(crate) fn configure() -> Result<Configuration, String> {
    let root = match std::env::var_os("SOLOSOUL_FE2_MACOS_ROOT") {
        Some(path) => PathBuf::from(path),
        // LaunchServices / 电脑控制工具启动应用时不继承调用进程的环境变量。
        // 只接受显式绑定到独立测试 bundle 的私有路径文件，仍执行完整校验。
        None => bundled_root()?,
    };
    let configuration = validate(&root)?;
    if let Some(other) = std::env::var_os("SOLOSOUL_DATA_DIR") {
        if Path::new(&other) != root.join("vault") {
            return Err("SOLOSOUL_DATA_DIR 指向隔离根以外，不启动验收客户端".into());
        }
    }
    CONFIG
        .set(configuration.clone())
        .map_err(|_| "验收路径不能重复配置")?;
    Ok(configuration)
}

fn bundled_root() -> Result<PathBuf, String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let contents = executable
        .parent()
        .and_then(Path::parent)
        .ok_or("测试 bundle 路径无效")?;
    if contents.file_name().and_then(|name| name.to_str()) != Some("Contents")
        || contents
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            != Some("SoloSoulFE2Mac.app")
    {
        return Err("未指定隔离目录，且不在独立测试 bundle 中".into());
    }
    let pointer = contents.join("Resources/fe2-macos-root.txt");
    let metadata = std::fs::symlink_metadata(&pointer).map_err(|_| "测试 bundle 未绑定隔离目录")?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.mode() & 0o077 != 0
        || metadata.len() > 4096
    {
        return Err("测试 bundle 路径文件无效".into());
    }
    let root = std::fs::read_to_string(pointer).map_err(|error| error.to_string())?;
    Ok(PathBuf::from(root.trim()))
}

pub(crate) fn root() -> Result<PathBuf, String> {
    CONFIG
        .get()
        .map(|configuration| configuration.root.clone())
        .ok_or_else(|| "macOS 验收路径尚未初始化".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            Self(prepare().unwrap())
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn prepared_fixture_has_only_synthetic_account_and_is_locked() {
        let fixture = Fixture::new();
        let configuration = validate(&fixture.0).unwrap();
        let service =
            solosoul_core::VaultService::try_with_base_path(fixture.0.join("vault")).unwrap();
        service.load_accounts();
        assert_eq!(service.list_accounts().len(), 1);
        service.unlock(&configuration.account_id, PASSWORD).unwrap();
        service.lock();
        assert!(fixture.0.join("app-data").is_dir());
        assert!(fixture.0.join("plugins").is_dir());
        // 固定深浅模式的原生验收不能因夹具给两种模式配置同一浅色色板而失真。
        let preferences: serde_json::Value = serde_json::from_slice(
            &std::fs::read(fixture.0.join("vault/ui_preferences.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(preferences["theme"], "light");
        assert_eq!(preferences["defaultLightTheme"], "warm-stone");
        assert_eq!(preferences["defaultDarkTheme"], "warm-stone-dark");
        assert!(configuration.second_account_id.is_none());
        let marker: serde_json::Value =
            serde_json::from_slice(&std::fs::read(fixture.0.join(MARKER)).unwrap()).unwrap();
        assert!(marker.get("second_account_id").is_none());
    }

    #[test]
    fn account_switch_fixture_has_two_unlockable_synthetic_accounts() {
        let fixture = Fixture(prepare_account_switch().unwrap());
        let configuration = validate(&fixture.0).unwrap();
        let service =
            solosoul_core::VaultService::try_with_base_path(fixture.0.join("vault")).unwrap();
        service.load_accounts();
        assert_eq!(service.list_accounts().len(), 2);
        assert!(!service.is_unlocked());
        for id in [
            configuration.account_id.as_str(),
            configuration.second_account_id.as_deref().unwrap(),
        ] {
            service.unlock(id, PASSWORD).unwrap();
            service.lock();
            assert!(!service.is_unlocked());
        }
    }

    #[test]
    fn account_switch_fixture_rejects_foreign_identity_and_extra_directory() {
        let fixture = Fixture(prepare_account_switch().unwrap());
        let path = fixture.0.join("vault/accounts.json");
        let original = std::fs::read(&path).unwrap();
        let mut accounts: serde_json::Value = serde_json::from_slice(&original).unwrap();
        accounts[1]["name"] = serde_json::json!("Other");
        std::fs::write(&path, serde_json::to_vec(&accounts).unwrap()).unwrap();
        assert!(validate(&fixture.0).unwrap_err().contains("非夹具账户"));
        std::fs::write(&path, original).unwrap();
        std::fs::create_dir(fixture.0.join("vault/acc_orphan")).unwrap();
        assert!(validate(&fixture.0).unwrap_err().contains("非夹具账户目录"));
    }

    #[test]
    fn account_switch_fixture_rejects_duplicate_marker_identity() {
        let fixture = Fixture(prepare_account_switch().unwrap());
        let mut configuration = validate(&fixture.0).unwrap();
        configuration.second_account_id = Some(configuration.account_id.clone());
        std::fs::write(
            fixture.0.join(MARKER),
            serde_json::to_vec(&configuration).unwrap(),
        )
        .unwrap();
        assert!(validate(&fixture.0).unwrap_err().contains("id 重复"));
    }

    #[test]
    fn rejects_symlinked_root_and_child_without_following_them() {
        use std::os::unix::fs::symlink;
        let fixture = Fixture::new();
        let link = fixture.0.with_extension("link");
        symlink(&fixture.0, &link).unwrap();
        assert!(validate(&link).is_err());
        std::fs::remove_file(link).unwrap();
        symlink(std::env::temp_dir(), fixture.0.join("plugins/escape")).unwrap();
        assert!(validate(&fixture.0).unwrap_err().contains("软链接"));
    }

    #[test]
    fn rejects_foreign_account_in_prepared_root() {
        let fixture = Fixture::new();
        let path = fixture.0.join("vault/accounts.json");
        let mut accounts: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        accounts
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"id":"acc_foreign", "name":"Other"}));
        std::fs::write(path, serde_json::to_vec(&accounts).unwrap()).unwrap();
        assert!(validate(&fixture.0).unwrap_err().contains("非夹具账户"));
    }

    #[test]
    fn rejects_orphan_account_even_if_not_in_manifest() {
        let fixture = Fixture::new();
        std::fs::create_dir(fixture.0.join("vault/acc_orphan")).unwrap();
        assert!(validate(&fixture.0).unwrap_err().contains("非夹具账户目录"));
    }

    #[test]
    fn rejects_marker_for_another_root() {
        let fixture = Fixture::new();
        let marker = fixture.0.join(MARKER);
        let mut configuration = validate(&fixture.0).unwrap();
        configuration.root = std::env::temp_dir();
        std::fs::write(marker, serde_json::to_vec(&configuration).unwrap()).unwrap();
        assert!(validate(&fixture.0).unwrap_err().contains("不匹配"));
    }

    #[test]
    fn rejects_directory_accessible_to_other_users() {
        let fixture = Fixture::new();
        std::fs::set_permissions(
            fixture.0.join("plugins"),
            std::fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        assert!(validate(&fixture.0).unwrap_err().contains("仅用户可访问"));
    }
}
