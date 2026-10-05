//! RF-312：仅 Windows、非默认构建的原生测量预检，不触及默认用户 Vault。
#[cfg(not(target_os = "windows"))]
compile_error!("native-perf is supported only on Windows");

mod fixture;
mod media_fixture;
#[cfg(test)]
mod preflight_tests;
mod runtime;
pub mod sdk_cdp;
pub mod sdk_journey;
mod startup;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub const OBSERVER_SCRIPT: &str = include_str!("native_perf/observer.js");
const OWNED_FILE: &str = "native-perf-owned.json";
const READY_FILE: &str = "native-perf-ready.json";
const CONSUMED_FILE: &str = "native-perf-consumed.json";
const CHROMIUM_LOG_MARKER: &str = "native-perf-chromium-log.json";
const CHROMIUM_LOG_NAME: &str = "chromium-diagnostics.log";
const ORDINARY_TMP_MARKER: &str = "native-perf-ordinary-tmp.json";
const IDENTIFIER_PREFIX: &str = "com.solosoul.rf312perf.";
const CHILD_DIRS: &[&str] = &[
    "app-data",
    "plugins",
    "models",
    "webview",
    "profile",
    "profile/Desktop",
    "profile/Documents",
    "profile/Downloads",
    "temp",
];

// RF-312：离线检查依赖完整字节标记。优化器可能把参数比较内联成机器指令，
// 因此在预检入口保留不透明引用；仅此非默认功能的模块包含该标记块。
#[used]
static BINARY_FEATURE_MARKERS: &[u8] = b"--native-perf-prepare\0windows-native-perf-owned\0windows-native-perf-ready\0windows-native-perf-consumed\0windows-native-sdk-startup-requested\0windows-native-sdk-startup-restart-ticket\0--native-perf-media-prepare\0windows-native-sdk-media-journey-requested";

static RUNTIME: OnceLock<RuntimeConfig> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub root: PathBuf,
    pub vault: PathBuf,
    pub identifier: String,
    pub webview: PathBuf,
    pub port: u16,
    pub run_id: String,
    pub chromium_log: Option<PathBuf>,
    pub sdk_cdp: bool,
    pub sdk_journey: bool,
    pub media_journey: bool,
    pub object_count: usize,
    pub startup: Option<startup::Launch>,
}

impl RuntimeConfig {
    pub fn evidence_root(&self) -> &Path {
        self.startup
            .as_ref()
            .map_or(self.root.as_path(), |launch| launch.evidence_root.as_path())
    }

    fn remote_arguments(&self) -> String {
        let mut args = format!(
            "--remote-debugging-port={} --remote-debugging-address=127.0.0.1",
            self.port
        );
        if let Some(log) = &self.chromium_log {
            args.push_str(&format!(
                " --enable-logging --v=1 --log-file=\"{}\"",
                log.display()
            ));
        }
        args
    }

    pub fn browser_arguments(&self) -> String {
        format!(
            "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection {}",
            self.remote_arguments()
        )
    }

    fn ordinary_owned_temp(&self) -> Result<PathBuf, String> {
        let expected = self.root.join("temp");
        let actual = checked_dir(&expected)?;
        if actual != expected {
            return Err("ordinary TMP requires the exact owned canonical TEMP directory".into());
        }
        let text = actual.to_str().ok_or("ordinary TMP requires Unicode")?;
        let ordinary = PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(text));
        if !ordinary.is_absolute()
            || !matches!(ordinary.components().next(), Some(std::path::Component::Prefix(prefix)) if matches!(prefix.kind(), std::path::Prefix::Disk(_)))
            || checked_dir(&ordinary)? != actual
        {
            return Err("ordinary TMP requires an equivalent ordinary local drive path".into());
        }
        Ok(ordinary)
    }

    fn record_ordinary_tmp(
        &self,
        temp: &std::ffi::OsStr,
        tmp: &std::ffi::OsStr,
        profile: &std::ffi::OsStr,
        webview: &std::ffi::OsStr,
    ) -> Result<(), String> {
        if self.chromium_log.is_none() {
            return Err("ordinary TMP is diagnostic only and requires Chromium logging".into());
        }
        let expected_temp = self.root.join("temp");
        let expected_tmp = self.ordinary_owned_temp()?;
        let expected_profile = self.root.join("profile");
        if temp != expected_temp.as_os_str()
            || tmp != expected_tmp.as_os_str()
            || profile != expected_profile.as_os_str()
            || webview != self.webview.as_os_str()
            || checked_dir(&expected_profile)? != expected_profile
            || checked_dir(&self.webview)? != self.webview
        {
            return Err(
                "ordinary TMP diagnostic must preserve exact TEMP/profile/WebView values".into(),
            );
        }
        // 只记录这个新进程实际设置后的值，不能将申请值当作环境证据。
        write_new_json(
            &self.root.join(ORDINARY_TMP_MARKER),
            &json!({
                "schemaVersion": 1, "scope": "windows-native-perf-ordinary-tmp",
                "mode": "ordinary-tmp", "performanceSample": false,
                "root": self.root, "runId": self.run_id, "pid": std::process::id(),
                "port": self.port, "temp": Path::new(temp), "tmp": Path::new(tmp),
                "userProfile": Path::new(profile), "webview": Path::new(webview),
            }),
        )
    }

    fn enable_chromium_diagnostics(&mut self) -> Result<(), String> {
        let expected_temp = self.root.join("temp");
        let actual_temp = checked_dir(&expected_temp)?;
        if actual_temp != expected_temp {
            return Err(
                "Chromium diagnostic TEMP must be the exact owned canonical directory".into(),
            );
        }
        let text = actual_temp
            .to_str()
            .ok_or("Chromium diagnostic TEMP requires Unicode")?;
        // 日志参数使用等价普通本地绝对路径；不更改原生进程 TEMP/TMP。
        let ordinary_temp = PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(text));
        if !matches!(ordinary_temp.components().next(), Some(std::path::Component::Prefix(prefix)) if matches!(prefix.kind(), std::path::Prefix::Disk(_)))
            || checked_dir(&ordinary_temp)? != actual_temp
        {
            return Err(
                "Chromium diagnostic log requires an equivalent ordinary local drive path".into(),
            );
        }
        let log = ordinary_temp.join(CHROMIUM_LOG_NAME);
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&log)
            .map_err(|e| format!("cannot exclusively claim Chromium diagnostic log: {e}"))?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        require_regular(&log, false)?;
        if log.canonicalize().map_err(|e| e.to_string())? != expected_temp.join(CHROMIUM_LOG_NAME) {
            return Err("Chromium diagnostic log resolved outside the exact owned path".into());
        }
        write_new_json(
            &self.root.join(CHROMIUM_LOG_MARKER),
            &json!({
                "schemaVersion": 1, "scope": "windows-native-perf-chromium-log",
                "root": self.root, "runId": self.run_id, "pid": std::process::id(),
                "port": self.port, "mode": "chromium-log", "logFile": log,
                "nativeTempUnchanged": true, "performanceSample": false,
            }),
        )?;
        self.chromium_log = Some(log);
        Ok(())
    }
}
#[derive(Debug)]
struct KnownFolders {
    roaming: PathBuf,
    local: PathBuf,
}

impl KnownFolders {
    fn resolve() -> Result<Self, String> {
        Ok(Self {
            roaming: checked_dir(&dirs::data_dir().ok_or("Roaming Known Folder unavailable")?)?,
            local: checked_dir(&dirs::data_local_dir().ok_or("Local Known Folder unavailable")?)?,
        })
    }
}

#[derive(Debug)]
enum Mode {
    Prepare {
        root: PathBuf,
        fixture: PathBuf,
    },
    MediaPrepare {
        root: PathBuf,
        fixture: PathBuf,
    },
    WarmPrepare {
        root: PathBuf,
    },
    Run {
        root: PathBuf,
        port: u16,
        chromium_log: bool,
        ordinary_tmp: bool,
        copied_runtime: Option<PathBuf>,
        sdk_cdp: bool,
        sdk_journey: bool,
        media_journey: bool,
        startup_only: bool,
        restart: bool,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OwnedManifest {
    schema_version: u32,
    scope: String,
    app_version: String,
    native_perf_feature: bool,
    preparation_status: String,
    run_id: String,
    root: PathBuf,
    vault: PathBuf,
    identifier: String,
    webview: PathBuf,
    roaming: PathBuf,
    local: PathBuf,
    profile: PathBuf,
    fixture_source: PathBuf,
    fixture: fixture::Proof,
    owned_paths: Vec<PathBuf>,
}

/// 准备模式在任何 Tauri Builder 创建之前返回；调用方打印并退出。
pub fn prepare_from_args() -> Option<Result<Value, String>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if !args.iter().any(|arg| {
        arg == "--native-perf-prepare"
            || arg == "--native-perf-warm-prepare"
            || arg == "--native-perf-media-prepare"
    }) {
        return None;
    }
    Some(parse_args(&args).and_then(|mode| match mode {
        Mode::Prepare { root, fixture } => {
            let folders = KnownFolders::resolve()?;
            prepare(&root, &fixture, &folders)
        }
        Mode::MediaPrepare { root, fixture } => {
            prepare_mode(&root, &fixture, &KnownFolders::resolve()?, true)
        }
        Mode::WarmPrepare { root } => startup::prepare_restart(&root, &KnownFolders::resolve()?),
        Mode::Run { .. } => Err("prepare mode is required".into()),
    }))
}

/// 仅在当前 binary 主线程、任何 Tauri/runtime/worker 之前调用。
pub fn configure_runtime() -> Result<RuntimeConfig, String> {
    if RUNTIME.get().is_some() {
        return Err("native-perf runtime is already configured".into());
    }
    if std::env::vars_os().any(|(key, _)| is_webview_override(&key)) {
        return Err("native-perf refuses inherited WEBVIEW2_* overrides".into());
    }
    if std::env::vars_os().any(|(key, _)| {
        key.to_string_lossy()
            .eq_ignore_ascii_case("PDFIUM_LIBRARY_PATH")
    }) {
        return Err("native-perf refuses inherited PDFIUM_LIBRARY_PATH".into());
    }
    let mode = parse_args(&std::env::args_os().skip(1).collect::<Vec<_>>())?;
    let Mode::Run {
        root,
        port,
        chromium_log,
        ordinary_tmp,
        copied_runtime,
        sdk_cdp,
        sdk_journey,
        media_journey,
        startup_only,
        restart,
    } = mode
    else {
        return Err("prepare mode must exit before configuring a GUI runtime".into());
    };
    let folders = KnownFolders::resolve()?;
    let mut config = if restart {
        startup::consume_restart(&root, port, &folders)?
    } else {
        if media_journey {
            consume_mode(&root, port, &folders, true)?
        } else {
            consume(&root, port, &folders)?
        }
    };
    if startup_only && !restart {
        startup::configure_initial(&mut config)?;
    }
    config.sdk_cdp = sdk_cdp;
    config.sdk_journey = sdk_journey;
    config.media_journey = media_journey;
    if sdk_journey {
        sdk_journey::claim_request(&config)?;
    }
    if sdk_cdp {
        sdk_cdp::claim_request(&config)?;
    }
    if chromium_log {
        config.enable_chromium_diagnostics()?;
    }
    let selected_runtime = copied_runtime
        .as_deref()
        .map(|path| runtime::validate(&config, path))
        .transpose()?;
    let ordinary_tmp = ordinary_tmp
        .then(|| config.ordinary_owned_temp())
        .transpose()?;
    // 仅修改此新进程的环境，不修改系统环境或 Registry。路径均已完成预检。
    std::env::set_var("SOLOSOUL_DATA_DIR", &config.vault);
    std::env::set_var("USERPROFILE", config.root.join("profile"));
    std::env::set_var("SOLOSOUL_FS_BASE", &config.root);
    // 显式 owned 值遮蔽 WebView2 Registry UDF policy；不存在外部目录回退。
    std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", &config.webview);
    std::env::set_var(
        "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
        config.remote_arguments(),
    );
    std::env::set_var("TEMP", config.root.join("temp"));
    std::env::set_var(
        "TMP",
        ordinary_tmp
            .clone()
            .unwrap_or_else(|| config.root.join("temp")),
    );
    if ordinary_tmp.is_some() {
        config.record_ordinary_tmp(
            &std::env::var_os("TEMP").ok_or("native TEMP is missing")?,
            &std::env::var_os("TMP").ok_or("native TMP is missing")?,
            &std::env::var_os("USERPROFILE").ok_or("native USERPROFILE is missing")?,
            &std::env::var_os("WEBVIEW2_USER_DATA_FOLDER")
                .ok_or("native WebView directory is missing")?,
        )?;
    }
    if let Some(selection) = selected_runtime {
        // 先严格拒绝全部继承 WEBVIEW2_*，此处只新增预检后的本进程 owned 值。
        std::env::set_var(
            "WEBVIEW2_BROWSER_EXECUTABLE_FOLDER",
            &selection.browser_executable_folder,
        );
        let available = runtime::available_version(&selection.browser_executable_folder)?;
        selection.record_actual(
            &config,
            &std::env::var_os("WEBVIEW2_BROWSER_EXECUTABLE_FOLDER")
                .ok_or("selected native Runtime folder is missing")?,
            &available,
        )?;
    }
    std::env::remove_var("SOLOSOUL_REGISTRY_PUBKEY");
    RUNTIME
        .set(config.clone())
        .map_err(|_| "native-perf runtime cannot be configured twice")?;
    Ok(config)
}

/// 未配置时直接失败，不回退至生产路径或环境。
pub fn root() -> Result<&'static Path, String> {
    RUNTIME
        .get()
        .map(|config| config.root.as_path())
        .ok_or_else(|| "native-perf paths were not configured before startup".into())
}

/// 将本轮身份注入独立观察器；不修改 Tauri readonly 内部对象。
pub fn observer_script() -> Result<String, String> {
    let config = RUNTIME
        .get()
        .ok_or("native-perf runtime is not configured")?;
    let run_id = serde_json::to_string(&config.run_id).map_err(|e| e.to_string())?;
    Ok(format!(
        "Object.defineProperty(window, '__SOLOSOUL_NATIVE_PERF_RUN_ID__', {{ value: {run_id} }});\n{OBSERVER_SCRIPT}"
    ))
}

fn is_webview_override(key: &std::ffi::OsStr) -> bool {
    key.to_string_lossy()
        .to_ascii_uppercase()
        .starts_with("WEBVIEW2_")
}

fn parse_args(args: &[OsString]) -> Result<Mode, String> {
    std::hint::black_box(BINARY_FEATURE_MARKERS);
    let mut prepare_root = None;
    let mut media_prepare = false;
    let mut media_journey = false;
    let mut run_root = None;
    let mut input_fixture = None;
    let mut port = None;
    let mut chromium_log = false;
    let mut ordinary_tmp = false;
    let mut copied_runtime = None;
    let mut sdk_cdp = false;
    let mut sdk_journey = false;
    let mut startup_only = false;
    let mut restart = false;
    let mut warm_prepare_root = None;
    let mut index = 0;
    while index < args.len() {
        let flag = args[index]
            .to_str()
            .ok_or("native-perf option must be UTF-8")?;
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("{flag} requires a value"))?;
        if value.to_string_lossy().starts_with("--") {
            return Err(format!("{flag} requires a value"));
        }
        match flag {
            "--native-perf-prepare" if prepare_root.is_none() => {
                prepare_root = Some(PathBuf::from(value))
            }
            "--native-perf-media-prepare" if prepare_root.is_none() => {
                prepare_root = Some(PathBuf::from(value));
                media_prepare = true;
            }
            "--native-perf-warm-prepare" if warm_prepare_root.is_none() => {
                warm_prepare_root = Some(PathBuf::from(value))
            }
            "--native-perf-restart" if !restart && value == "startup" => restart = true,
            "--fixture" if input_fixture.is_none() => input_fixture = Some(PathBuf::from(value)),
            "--native-perf-root" if run_root.is_none() => run_root = Some(PathBuf::from(value)),
            "--native-perf-runtime" if copied_runtime.is_none() => {
                copied_runtime = Some(PathBuf::from(value));
            }
            "--native-perf-port" if port.is_none() => {
                port = Some(
                    value
                        .to_str()
                        .and_then(|value| value.parse::<u16>().ok())
                        .filter(|value| *value >= 1024)
                        .ok_or("native-perf port must be an integer between 1024 and 65535")?,
                );
            }
            "--native-perf-journey"
                if !sdk_journey
                    && (value == "sdk-input" || value == "sdk-startup" || value == "sdk-media") =>
            {
                sdk_journey = true;
                startup_only = value == "sdk-startup";
                media_journey = value == "sdk-media";
            }
            "--native-perf-diagnostics"
                if !chromium_log
                    && !sdk_cdp
                    && (value == "chromium-log"
                        || value == "chromium-log-ordinary-tmp"
                        || value == "sdk-cdp") =>
            {
                sdk_cdp = value == "sdk-cdp";
                chromium_log = !sdk_cdp;
                ordinary_tmp = value == "chromium-log-ordinary-tmp";
            }
            _ => return Err(format!("unknown or duplicate native-perf option: {flag}")),
        }
        index += 2;
    }
    if copied_runtime.is_some() && (!chromium_log || ordinary_tmp) {
        return Err("--native-perf-runtime requires chromium-log and forbids ordinary TMP".into());
    }
    if sdk_journey && (chromium_log || sdk_cdp || copied_runtime.is_some()) {
        return Err("SDK journey is exclusive and cannot use diagnostic overrides".into());
    }
    if restart && !startup_only {
        return Err("restart requires the exclusive sdk-startup mode".into());
    }
    if let Some(root) = warm_prepare_root {
        if prepare_root.is_some()
            || input_fixture.is_some()
            || run_root.is_some()
            || port.is_some()
            || chromium_log
            || sdk_cdp
            || sdk_journey
            || copied_runtime.is_some()
            || restart
        {
            return Err("warm prepare is exclusive and does not configure a GUI".into());
        }
        return Ok(Mode::WarmPrepare { root });
    }
    if let Some(root) = prepare_root {
        if let Some(fixture) = input_fixture {
            if run_root.is_none()
                && port.is_none()
                && !chromium_log
                && !sdk_cdp
                && !sdk_journey
                && copied_runtime.is_none()
                && !restart
            {
                return Ok(if media_prepare {
                    Mode::MediaPrepare { root, fixture }
                } else {
                    Mode::Prepare { root, fixture }
                });
            }
        }
        return Err("prepare requires only a new root and fixture".into());
    }
    match (run_root, port, input_fixture) {
        (Some(root), Some(port), None) => Ok(Mode::Run {
            root,
            port,
            chromium_log,
            ordinary_tmp,
            copied_runtime,
            sdk_cdp,
            sdk_journey,
            media_journey,
            startup_only,
            restart,
        }),
        _ => Err(
            "use explicit prepare, warm-prepare, or root/port run arguments; no defaults".into(),
        ),
    }
}

fn prepare(root: &Path, source: &Path, folders: &KnownFolders) -> Result<Value, String> {
    prepare_mode(root, source, folders, false)
}
fn prepare_mode(
    root: &Path,
    source: &Path,
    folders: &KnownFolders,
    media: bool,
) -> Result<Value, String> {
    if !source.is_absolute() {
        return Err("--fixture requires an absolute verified synthetic fixture".into());
    }
    let source = checked_dir(source)?;
    // 拷贝前仅读取公开 marker/config，拒绝非合成或不完整输入。
    let contract = if media {
        fixture::check_media_contract(&source)?
    } else {
        fixture::check_contract(&source)?
    };
    let candidate = new_root_candidate(root)?;
    if candidate.starts_with(&source) {
        return Err("native-perf root must be outside the read-only source fixture".into());
    }
    let root = create_new_root(&candidate)?;
    let run_id = uuid::Uuid::new_v4().simple().to_string();
    let identifier = format!("{IDENTIFIER_PREFIX}{run_id}");
    let roaming = folders.roaming.join(&identifier);
    let local = folders.local.join(&identifier);
    // 提前记录当前确实拥有的路径；后续失败保留证据，不发布 ready。
    let mut owned_paths = vec![root.clone()];
    let mut preparing = json!({
        "schemaVersion": 1, "scope": "windows-native-perf-owned",
        "appVersion": env!("CARGO_PKG_VERSION"), "nativePerfFeature": true,
        "preparationStatus": "preparing", "runId": run_id, "root": root,
        "vault": root.join("vault"), "identifier": identifier,
        "webview": root.join("webview"), "roaming": roaming, "local": local,
        "profile": root.join("profile"), "fixtureSource": source,
        "fixture": null, "ownedPaths": owned_paths,
    });
    write_new_json(&root.join(OWNED_FILE), &preparing)?;
    // 一切目录均独占创建；任何冲突都停止，不接纳或覆盖已有路径。
    for (key, path) in [("roaming", &roaming), ("local", &local)] {
        create_new_dir(path)?;
        // Known Folder 逻辑路径可能被 Windows 映射；以已创建句柄的实际
        // 路径统一记录字段和 ownership，不能仅比较申请时的字符串。
        let claimed = checked_dir(path)?;
        owned_paths.push(claimed.clone());
        preparing[key] = json!(claimed);
        preparing["ownedPaths"] = json!(owned_paths);
        replace_owned_json(&root.join(OWNED_FILE), &preparing)?;
    }
    for child in CHILD_DIRS {
        create_new_dir(&root.join(child))?;
    }
    let vault = root.join("vault");
    let proof = if media {
        media_fixture::copy(&source, &vault)?;
        fixture::verify_media_copy(&vault, &contract)?
    } else {
        create_new_dir(&vault)?;
        fixture::copy_closed_fixture(&source, &vault, &contract)?;
        fixture::verify_copy(&vault, &contract)?
    };
    let manifest = OwnedManifest {
        schema_version: 1,
        scope: "windows-native-perf-owned".into(),
        app_version: env!("CARGO_PKG_VERSION").into(),
        native_perf_feature: true,
        preparation_status: "ready".into(),
        run_id,
        root: root.clone(),
        vault: vault.clone(),
        identifier,
        webview: root.join("webview"),
        roaming: owned_paths[1].clone(),
        local: owned_paths[2].clone(),
        profile: root.join("profile"),
        fixture_source: source,
        fixture: proof,
        owned_paths,
    };
    replace_owned_json(
        &root.join(OWNED_FILE),
        &serde_json::to_value(&manifest).map_err(|e| e.to_string())?,
    )?;
    let ready = json!({
        "schemaVersion": 1,
        "scope": "windows-native-perf-ready",
        "root": root,
        "runId": manifest.run_id,
        "ownedSha256": sha256_file(&root.join(OWNED_FILE))?,
    });
    // 完成标记最后发布。失败目录保留供审查，不自动删除任意输入目录。
    write_new_json(&root.join(READY_FILE), &ready)?;
    Ok(json!({
        "schemaVersion": 1,
        "scope": "windows-native-perf-preparation",
        "success": true,
        "nativePerfFeature": true,
        "appVersion": manifest.app_version,
        "runId": manifest.run_id,
        "root": manifest.root,
        "vault": manifest.vault,
        "identifier": manifest.identifier,
        "webview": manifest.webview,
        "ownedPaths": manifest.owned_paths,
        "fixtureSource": manifest.fixture_source,
        "objectCount": manifest.fixture.object_count,
        "accountId": manifest.fixture.account_id,
    }))
}

fn checked_manifest(root: &Path, folders: &KnownFolders) -> Result<OwnedManifest, String> {
    checked_manifest_mode(root, folders, false, false)
}

fn checked_startup_manifest(root: &Path, folders: &KnownFolders) -> Result<OwnedManifest, String> {
    checked_manifest_mode(root, folders, true, false)
}

fn checked_manifest_mode(
    root: &Path,
    folders: &KnownFolders,
    startup: bool,
    media: bool,
) -> Result<OwnedManifest, String> {
    let root = checked_dir(root)?;
    let manifest: OwnedManifest = serde_json::from_value(read_json(&root.join(OWNED_FILE))?)
        .map_err(|e| format!("invalid native-perf owned manifest: {e}"))?;
    if manifest.run_id.len() != 32
        || !manifest
            .run_id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || manifest.identifier != format!("{IDENTIFIER_PREFIX}{}", manifest.run_id)
    {
        return Err("native-perf manifest does not describe this explicitly owned test run".into());
    }
    // 只解析当前 Known Folder 下的固定身份候选目录；不把 manifest 中
    // 任意路径 canonicalize 后当作归属证明，也不放宽 regular/reparse 校验。
    let expected_roaming = checked_dir(&folders.roaming.join(&manifest.identifier))?;
    let expected_local = checked_dir(&folders.local.join(&manifest.identifier))?;
    if manifest.schema_version != 1
        || manifest.scope != "windows-native-perf-owned"
        || !manifest.native_perf_feature
        || manifest.preparation_status != "ready"
        || manifest.app_version != env!("CARGO_PKG_VERSION")
        || manifest.root != root
        || manifest.vault != root.join("vault")
        || manifest.webview != root.join("webview")
        || manifest.profile != root.join("profile")
        || manifest.roaming != expected_roaming
        || manifest.local != expected_local
        || manifest.owned_paths
            != vec![
                root.clone(),
                manifest.roaming.clone(),
                manifest.local.clone(),
            ]
    {
        return Err("native-perf manifest does not describe this explicitly owned test run".into());
    }
    checked_dir(&manifest.roaming)?;
    checked_dir(&manifest.local)?;
    for child in CHILD_DIRS {
        checked_dir(&root.join(child))?;
    }
    if media {
        fixture::check_media_proof(&manifest.vault, &manifest.fixture)?;
    } else if startup {
        fixture::check_startup_proof(&manifest.vault, &manifest.fixture)?;
    } else {
        let contract = fixture::check_contract(&manifest.vault)?;
        fixture::check_proof(&manifest.vault, &manifest.fixture, &contract)?;
    }
    Ok(manifest)
}

fn consume(root: &Path, port: u16, folders: &KnownFolders) -> Result<RuntimeConfig, String> {
    consume_mode(root, port, folders, false)
}
fn consume_mode(
    root: &Path,
    port: u16,
    folders: &KnownFolders,
    media: bool,
) -> Result<RuntimeConfig, String> {
    if !root.is_absolute() {
        return Err("--native-perf-root requires an absolute prepared root".into());
    }
    let root = checked_dir(root)?;
    if fs::symlink_metadata(root.join(CONSUMED_FILE)).is_ok() {
        return Err("native-perf prepared root has already been consumed".into());
    }
    let ready = read_json(&root.join(READY_FILE))?;
    let manifest = if media {
        checked_manifest_mode(&root, folders, false, true)?
    } else {
        checked_manifest(&root, folders)?
    };
    let expected_ready = json!({
        "schemaVersion": 1,
        "scope": "windows-native-perf-ready",
        "root": root,
        "runId": manifest.run_id,
        "ownedSha256": sha256_file(&root.join(OWNED_FILE))?,
    });
    if ready != expected_ready {
        return Err("native-perf ready marker does not match owned manifest".into());
    }
    // 只检查 loopback 端口未被占用；关闭监听后仍有竞争，runner 必须再核对 PID/runId。
    let probe = TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| format!("native-perf CDP port is unavailable: {e}"))?;
    drop(probe);
    let consumed = json!({
        "schemaVersion": 1,
        "scope": "windows-native-perf-consumed",
        "root": root,
        "runId": manifest.run_id,
        "port": port,
        "pid": std::process::id(),
    });
    // create_new 是一次运行的原子竞争点；失败后不退回 ready、不重用样本。
    write_new_json(&root.join(CONSUMED_FILE), &consumed)?;
    fs::remove_file(root.join(READY_FILE))
        .map_err(|e| format!("cannot consume native-perf ready marker: {e}"))?;
    Ok(RuntimeConfig {
        root,
        vault: manifest.vault,
        identifier: manifest.identifier,
        webview: manifest.webview,
        port,
        run_id: manifest.run_id,
        chromium_log: None,
        sdk_cdp: false,
        sdk_journey: false,
        media_journey: media,
        object_count: manifest.fixture.object_count,
        startup: None,
    })
}

fn new_root_candidate(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err("native-perf prepare root must be an absolute new directory".into());
    }
    let name = path
        .file_name()
        .ok_or("native-perf root requires a directory name")?;
    let parent = checked_dir(
        path.parent()
            .ok_or("native-perf root requires an existing parent")?,
    )?;
    Ok(parent.join(name))
}

fn create_new_root(path: &Path) -> Result<PathBuf, String> {
    let output = new_root_candidate(path)?;
    create_new_dir(&output)?;
    checked_dir(&output)
}

fn create_new_dir(path: &Path) -> Result<(), String> {
    fs::create_dir(path).map_err(|e| {
        format!(
            "cannot exclusively create native-perf directory {}: {e}",
            path.display()
        )
    })?;
    require_regular(path, true)
}

fn checked_dir(path: &Path) -> Result<PathBuf, String> {
    require_regular(path, true)?;
    path.canonicalize()
        .map_err(|e| format!("cannot resolve native-perf directory: {e}"))
}

fn require_regular(path: &Path, directory: bool) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|e| format!("missing native-perf path {}: {e}", path.display()))?;
    let is_link = metadata.file_type().is_symlink();
    #[cfg(windows)]
    let is_link = {
        use std::os::windows::fs::MetadataExt;
        is_link || metadata.file_attributes() & 0x400 != 0
    };
    if is_link || (directory && !metadata.is_dir()) || (!directory && !metadata.is_file()) {
        return Err(format!(
            "native-perf path is not a regular {}: {}",
            if directory { "directory" } else { "file" },
            path.display()
        ));
    }
    Ok(())
}

fn read_json(path: &Path) -> Result<Value, String> {
    require_regular(path, false)?;
    let file = File::open(path).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() > 64 * 1024 {
        return Err("native-perf metadata exceeds 64 KiB".into());
    }
    serde_json::from_reader(file).map_err(|e| format!("invalid native-perf JSON: {e}"))
}

fn write_new_json(path: &Path, value: &Value) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("cannot exclusively publish native-perf marker: {e}"))?;
    serde_json::to_writer_pretty(&mut file, value).map_err(|e| e.to_string())?;
    file.write_all(b"\n").map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())
}

fn replace_owned_json(path: &Path, value: &Value) -> Result<(), String> {
    require_regular(path, false)?;
    let parent = path.parent().ok_or("owned manifest requires a parent")?;
    let mut staged = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut staged, value).map_err(|e| e.to_string())?;
    staged.write_all(b"\n").map_err(|e| e.to_string())?;
    staged.as_file().sync_all().map_err(|e| e.to_string())?;
    staged.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String, String> {
    require_regular(path, false)?;
    let mut input = File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let size = input.read(&mut buffer).map_err(|e| e.to_string())?;
        if size == 0 {
            break;
        }
        hash.update(&buffer[..size]);
    }
    Ok(hex::encode(hash.finalize()))
}
