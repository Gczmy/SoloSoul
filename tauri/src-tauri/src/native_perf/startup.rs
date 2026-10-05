//! RF-312：同一私有profile的只读双进程启动；旧consumed不撤销，重启票据独立且一次消费。
use super::{
    checked_dir, checked_startup_manifest, read_json, sha256_file, write_new_json, KnownFolders,
    OwnedManifest, RuntimeConfig, CONSUMED_FILE, OWNED_FILE,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const STOPPED: &str = "native-perf-startup-stopped.json";
const TICKET: &str = "native-perf-startup-restart-ticket.json";
const USED: &str = "native-perf-startup-restart-consumed.json";
const PROOF: &str = "native-perf-sdk-journey.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Launch {
    pub generation: u8,
    pub owner_run_id: String,
    pub evidence_root: PathBuf,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Identity {
    pid: u32,
    creation_ms: i64,
    executable_name: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Stopped {
    schema_version: u32,
    scope: String,
    root: PathBuf,
    owner_run_id: String,
    pid: u32,
    browser_pid: u32,
    owned_sha256: String,
    consumed_sha256: String,
    proof_sha256: String,
    exe_sha256: String,
    ui_preferences_sha256: String,
    identities: Vec<Identity>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Ticket {
    schema_version: u32,
    scope: String,
    root: PathBuf,
    owner_run_id: String,
    run_id: String,
    evidence_root: PathBuf,
    owned_sha256: String,
    consumed_sha256: String,
    proof_sha256: String,
    stopped_sha256: String,
    exe_sha256: String,
    ui_preferences_sha256: String,
    update_source_candidates: Value,
    prior_pid: u32,
    prior_browser_pid: u32,
    prior_port: u16,
    exit_check: Value,
}
fn now_ms() -> Result<u64, String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis() as u64)
}
fn id(text: &str) -> bool {
    text.len() == 32
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn hash(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn keys(value: &Value, count: usize) -> bool {
    value.as_object().is_some_and(|m| m.len() == count)
}
fn integer(value: &Value) -> Result<u32, String> {
    value
        .as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .filter(|n| *n > 0)
        .ok_or_else(|| "startup PID is invalid".into())
}
fn finite(value: &Value, limit: f64) -> bool {
    value
        .as_f64()
        .is_some_and(|n| n.is_finite() && n >= 0.0 && n <= limit)
}
fn launch(root: &Path, owner: &str, generation: u8) -> Launch {
    Launch {
        generation,
        owner_run_id: owner.into(),
        evidence_root: root.join(format!("startup-{generation:02}")),
    }
}
pub(super) fn configure_initial(config: &mut RuntimeConfig) -> Result<(), String> {
    let descriptor = launch(&config.root, &config.run_id, 1);
    fs::create_dir(&descriptor.evidence_root)
        .map_err(|e| format!("cannot exclusively claim first startup evidence directory: {e}"))?;
    checked_dir(&descriptor.evidence_root)?;
    config.startup = Some(descriptor);
    Ok(())
}
pub(super) fn unmeasured(generation: u8) -> Value {
    let mut values = vec![
        "password-unlock",
        "workspace",
        "search-needle",
        "application-lock",
        "password-reunlock",
        "OCR",
        "attachment-preview",
        "system-sleep",
        "other-platforms",
    ];
    if generation == 1 {
        values.push("same-profile-warm-start");
    }
    json!(values)
}
// 与SDK模块共用真正的探针/observer校验；只读启动不能接纳完整输入行程或失败证明。
fn validate_prime(proof: &Value, owned: &OwnedManifest) -> Result<(), String> {
    let clock = &proof["timeOriginMs"];
    if !keys(proof, 21)
        || proof["schemaVersion"] != 1
        || proof["scope"] != "windows-native-sdk-startup"
        || proof["root"] != json!(owned.root)
        || proof["runId"] != owned.run_id
        || proof["objectCount"] != owned.fixture.object_count
        || proof["inputMethod"] != "SDK-CDP-read-only"
        || proof["success"] != true
        || !proof["reason"].is_null()
        || !proof["failedStep"].is_null()
        || proof["calls"] != json!(["Runtime.evaluate", "Page.getFrameTree"])
        || proof["startup"] != json!(launch(&owned.root, &owned.run_id, 1))
        || proof["unmeasured"] != unmeasured(1)
        || !finite(&proof["elapsedMs"], 300000.0)
        || !clock.as_f64().is_some_and(|n| n.is_finite() && n > 0.0)
    {
        return Err("first startup proof is not a successful exclusive read-only prime".into());
    }
    let pid = integer(&proof["pid"])?;
    let browser = integer(&proof["browserPid"])?;
    if pid == browser
        || !proof["port"]
            .as_u64()
            .is_some_and(|n| (1024..=65535).contains(&n))
    {
        return Err("first startup process/port binding invalid".into());
    }
    let probe = super::sdk_journey::validate_probe_fields(
        &proof["lastProbe"],
        "startup",
        &owned.run_id,
        Some(clock),
        "ready",
    )
    .map_err(str::to_owned)?;
    if probe["href"] != "http://tauri.localhost/login"
        || probe["inputTrust"] != json!({"pointer":0,"text":0,"untrusted":0})
        || !probe["target"].is_null()
    {
        return Err("first startup must be a no-input login checkpoint".into());
    }
    let binding = &proof["binding"];
    if !keys(binding, 6)
        || binding["source"] != probe["href"]
        || !binding["timeOriginMs"].is_null()
        || binding["navigationEvents"] != 0
        || binding["frameCreatedEvents"] != 0
        || ["mainFrameId", "loaderId"].iter().any(|key| {
            !binding[key].as_str().is_some_and(|s| {
                !s.is_empty()
                    && s.len() <= 128
                    && s.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
            })
        })
    {
        return Err("first startup main frame binding invalid".into());
    }
    let phases = proof["phases"]
        .as_array()
        .filter(|p| p.len() == 1)
        .ok_or("first startup requires exactly one phase")?;
    let phase = &phases[0];
    let zero = json!({"commands":[],"timeOriginMs":clock,"runId":owned.run_id,"valid":true});
    let ipc = super::sdk_journey::ipc_delta(&zero, &probe["observer"]).map_err(str::to_owned)?;
    if !keys(phase, 7)
        || phase["name"] != "startup"
        || phase["success"] != true
        || !finite(&phase["durationMs"], 300000.0)
        || phase["fromAtMs"] != 0
        || phase["toAtMs"] != probe["atMs"]
        || phase["inputTrust"] != probe["inputTrust"]
        || phase["ipc"] != ipc
        || proof["ipcAll"] != ipc
    {
        return Err("first startup phase/IPC does not match its validated probe".into());
    }
    Ok(())
}
fn checked_prime(
    root: &Path,
    folders: &KnownFolders,
) -> Result<(OwnedManifest, Value, Stopped), String> {
    let owned = checked_startup_manifest(root, folders)?;
    let root = &owned.root;
    let proof_path = root.join("startup-01").join(PROOF);
    checked_dir(&root.join("startup-01"))?;
    let proof = read_json(&proof_path)?;
    validate_prime(&proof, &owned)?;
    let consumed = read_json(&root.join(CONSUMED_FILE))?;
    if !keys(&consumed, 6)
        || consumed["schemaVersion"] != 1
        || consumed["scope"] != "windows-native-perf-consumed"
        || consumed["root"] != json!(root)
        || consumed["runId"] != owned.run_id
        || consumed["pid"] != proof["pid"]
        || consumed["port"] != proof["port"]
    {
        return Err("first consumed marker and startup proof disagree".into());
    }
    let stopped: Stopped = serde_json::from_value(read_json(&root.join(STOPPED))?)
        .map_err(|e| format!("invalid bounded startup stop receipt: {e}"))?;
    if stopped.schema_version != 1
        || stopped.scope != "windows-native-sdk-startup-stopped"
        || stopped.root != *root
        || stopped.owner_run_id != owned.run_id
        || stopped.pid != integer(&proof["pid"])?
        || stopped.browser_pid != integer(&proof["browserPid"])?
        || stopped.owned_sha256 != sha256_file(&root.join(OWNED_FILE))?
        || stopped.consumed_sha256 != sha256_file(&root.join(CONSUMED_FILE))?
        || stopped.proof_sha256 != sha256_file(&proof_path)?
        || stopped.ui_preferences_sha256 != sha256_file(&owned.vault.join("ui_preferences.json"))?
        || stopped.exe_sha256 != sha256_file(&std::env::current_exe().map_err(|e| e.to_string())?)?
    {
        return Err(
            "startup stop receipt does not bind the original owner/consumed/proof/EXE".into(),
        );
    }
    validate_identities(&stopped.identities, stopped.pid, stopped.browser_pid)?;
    Ok((owned, proof, stopped))
}
fn validate_identities(rows: &[Identity], pid: u32, browser: u32) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    let executable = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .file_name()
        .ok_or("current executable name unavailable")?
        .to_string_lossy()
        .to_lowercase();
    let now = now_ms()?;
    if rows.len() < 2
        || rows.len() > 128
        || rows.iter().any(|r| {
            r.pid == 0
                || r.creation_ms <= 0
                || r.creation_ms as u64 > now
                || r.executable_name.is_empty()
                || r.executable_name.len() > 260
                || r.executable_name
                    .chars()
                    .any(|c| c.is_control() || "\\/:".contains(c))
                || !seen.insert((r.pid, r.creation_ms, r.executable_name.to_lowercase()))
        })
        || rows
            .iter()
            .filter(|r| r.pid == pid && r.executable_name.to_lowercase() == executable)
            .count()
            != 1
        || rows
            .iter()
            .filter(|r| {
                r.pid == browser && r.executable_name.eq_ignore_ascii_case("msedgewebview2.exe")
            })
            .count()
            != 1
    {
        return Err(
            "startup stop receipt lacks bounded unique verified root/browser identities".into(),
        );
    }
    Ok(())
}
pub(super) fn prepare_restart(root: &Path, folders: &KnownFolders) -> Result<Value, String> {
    let (owned, proof, stopped) = checked_prime(root, folders)?;
    let root = &owned.root;
    for path in [root.join(TICKET), root.join(USED), root.join("startup-02")] {
        if fs::symlink_metadata(path).is_ok() {
            return Err("startup restart already prepared or consumed; no reuse".into());
        }
    }
    let exit_check = fresh_exit_check(&stopped.identities)?;
    let ticket = Ticket {
        schema_version: 1,
        scope: "windows-native-sdk-startup-restart-ticket".into(),
        root: root.clone(),
        owner_run_id: owned.run_id,
        run_id: uuid::Uuid::new_v4().simple().to_string(),
        evidence_root: root.join("startup-02"),
        owned_sha256: stopped.owned_sha256,
        consumed_sha256: stopped.consumed_sha256,
        proof_sha256: stopped.proof_sha256,
        stopped_sha256: sha256_file(&root.join(STOPPED))?,
        exe_sha256: stopped.exe_sha256,
        ui_preferences_sha256: stopped.ui_preferences_sha256,
        update_source_candidates: crate::commands::update::native_perf_cache_candidates()?,
        prior_pid: stopped.pid,
        prior_browser_pid: stopped.browser_pid,
        prior_port: proof["port"].as_u64().unwrap() as u16,
        exit_check,
    };
    fs::create_dir(&ticket.evidence_root)
        .map_err(|e| format!("cannot exclusively claim second startup evidence directory: {e}"))?;
    write_new_json(&root.join(TICKET), &json!(ticket))?;
    Ok(
        json!({"schemaVersion":1,"scope":"windows-native-sdk-startup-restart-prepared","success":true,"root":root,"ownerRunId":ticket.owner_run_id,"runId":ticket.run_id,"evidenceRoot":ticket.evidence_root,"ticketSha256":sha256_file(&root.join(TICKET))?}),
    )
}
pub(super) fn consume_restart(
    root: &Path,
    port: u16,
    folders: &KnownFolders,
) -> Result<RuntimeConfig, String> {
    let (owned, proof, stopped) = checked_prime(root, folders)?;
    let root = &owned.root;
    if fs::symlink_metadata(root.join(USED)).is_ok() {
        return Err("startup restart has already been consumed".into());
    }
    let ticket: Ticket = serde_json::from_value(read_json(&root.join(TICKET))?)
        .map_err(|e| format!("invalid startup restart ticket: {e}"))?;
    if ticket.schema_version != 1
        || ticket.scope != "windows-native-sdk-startup-restart-ticket"
        || ticket.root != *root
        || ticket.owner_run_id != owned.run_id
        || !id(&ticket.run_id)
        || ticket.run_id == owned.run_id
        || ticket.evidence_root != root.join("startup-02")
        || ticket.owned_sha256 != sha256_file(&root.join(OWNED_FILE))?
        || ticket.consumed_sha256 != sha256_file(&root.join(CONSUMED_FILE))?
        || ticket.proof_sha256 != sha256_file(&root.join("startup-01").join(PROOF))?
        || ticket.stopped_sha256 != sha256_file(&root.join(STOPPED))?
        || ticket.exe_sha256 != stopped.exe_sha256
        || ticket.ui_preferences_sha256 != stopped.ui_preferences_sha256
        || ticket.update_source_candidates
            != crate::commands::update::native_perf_cache_candidates()?
        || !hash(&ticket.exe_sha256)
        || ticket.prior_pid != stopped.pid
        || ticket.prior_browser_pid != stopped.browser_pid
        || json!(ticket.prior_port) != proof["port"]
        || port == ticket.prior_port
        || !keys(&ticket.exit_check, 4)
        || ticket.exit_check["verified"] != true
        || integer(&ticket.exit_check["queryPid"]).is_err()
        || ticket.exit_check["checkedIdentities"] != stopped.identities.len()
        || !ticket.exit_check["checkedAtUnixMs"]
            .as_u64()
            .is_some_and(|at| now_ms().is_ok_and(|now| at <= now && now - at <= 300000))
    {
        return Err("startup restart ticket no longer matches its exact primed owner".into());
    }
    checked_dir(&ticket.evidence_root)?;
    if fs::read_dir(&ticket.evidence_root)
        .map_err(|e| e.to_string())?
        .next()
        .is_some()
    {
        return Err("warm evidence directory must be unused".into());
    }
    let exit_check = fresh_exit_check(&stopped.identities)?;
    let listener = std::net::TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| format!("warm startup port unavailable: {e}"))?;
    drop(listener);
    let used = json!({"schemaVersion":1,"scope":"windows-native-sdk-startup-restart-consumed","root":root,"ownerRunId":owned.run_id,"runId":ticket.run_id,"pid":std::process::id(),"port":port,"ticketSha256":sha256_file(&root.join(TICKET))?,"evidenceRoot":ticket.evidence_root,"exitCheck":exit_check});
    write_new_json(&root.join(USED), &used)?;
    Ok(RuntimeConfig {
        root: root.clone(),
        vault: owned.vault,
        identifier: owned.identifier,
        webview: owned.webview,
        port,
        run_id: ticket.run_id,
        chromium_log: None,
        sdk_cdp: false,
        sdk_journey: false,
        media_journey: false,
        object_count: owned.fixture.object_count,
        startup: Some(launch(root, &owned.run_id, 2)),
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ExitQuery {
    query_pid: u32,
    rows: Vec<Identity>,
}

// 只查询票据内已验证的PID及本次查询子进程；不读取其他进程命令行，不终止应用进程。
fn query_exits(known: &[Identity]) -> Result<(ExitQuery, u32, u64), String> {
    use std::io::Read;
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    let filter = known
        .iter()
        .map(|r| format!("ProcessId={}", r.pid))
        .collect::<Vec<_>>()
        .join(" OR ");
    let script = format!(
        r#"$ErrorActionPreference='Stop';[Console]::OutputEncoding=New-Object System.Text.UTF8Encoding($false);$q=$PID;$filter='{}';if($filter){{$filter+=' OR '}};$filter+='ProcessId='+$q;$rows=@(Get-CimInstance Win32_Process -Filter $filter -Property ProcessId,CreationDate,Name | ForEach-Object {{if($null -eq $_.CreationDate -or $null -eq $_.Name){{throw 'missing metadata'}};[pscustomobject]@{{pid=[uint32]$_.ProcessId;creationMs=([DateTimeOffset]$_.CreationDate.ToUniversalTime()).ToUnixTimeMilliseconds();executableName=$_.Name}}}});ConvertTo-Json -InputObject ([pscustomobject]@{{queryPid=$q;rows=$rows}}) -Depth 4 -Compress"#,
        filter
    );
    let began = now_ms()?;
    let mut child = Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &script,
        ])
        .creation_flags(0x08000000)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("startup exit query could not start: {e}"))?;
    let pid = child.id();
    let out = child
        .stdout
        .take()
        .ok_or("startup exit query has no stdout")?;
    let err = child
        .stderr
        .take()
        .ok_or("startup exit query has no stderr")?;
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        out.take(131073).read_to_end(&mut bytes).map(|_| bytes)
    });
    let errors = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        err.take(8193).read_to_end(&mut bytes).map(|_| bytes)
    });
    let deadline = Instant::now() + Duration::from_secs(15);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(
                    "startup exit query timed out; only its own helper was stopped".to_owned(),
                );
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(format!("startup exit query wait failed: {e}"));
            }
        }
    };
    let stdout = reader
        .join()
        .map_err(|_| "startup exit query reader failed")?
        .map_err(|e| e.to_string())?;
    let stderr = errors
        .join()
        .map_err(|_| "startup exit query stderr reader failed")?
        .map_err(|e| e.to_string())?;
    if !status?.success() || stdout.len() > 131072 || !stderr.is_empty() {
        return Err("startup exit query failed or exceeded its output bounds".into());
    }
    let result: ExitQuery = serde_json::from_slice(&stdout)
        .map_err(|_| "startup exit query returned invalid bounded metadata")?;
    Ok((result, pid, began))
}
fn validate_exit_query(
    query: &ExitQuery,
    helper: u32,
    began: u64,
    known: &[Identity],
    now: u64,
) -> Result<(), String> {
    let requested = known.iter().map(|r| r.pid).collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    if query.query_pid != helper
        || helper == 0
        || requested.contains(&helper)
        || query.rows.is_empty()
        || query.rows.len() > requested.len() + 1
        || query.rows.iter().any(|r| {
            r.pid == 0
                || !seen.insert(r.pid)
                || (r.pid != helper && !requested.contains(&r.pid))
                || r.creation_ms <= 0
                || r.creation_ms as u64 > now
                || r.executable_name.is_empty()
                || r.executable_name.len() > 260
                || r.executable_name
                    .chars()
                    .any(|c| c.is_control() || "\\/:".contains(c))
        })
        || query
            .rows
            .iter()
            .filter(|r| {
                r.pid == helper
                    && r.creation_ms as u64 >= began
                    && r.executable_name.eq_ignore_ascii_case("powershell.exe")
            })
            .count()
            != 1
    {
        return Err(
            "startup exit query metadata is incomplete or not bound to its own helper".into(),
        );
    }
    if known.iter().any(|prior| {
        query.rows.iter().any(|row| {
            row.pid == prior.pid
                && row.creation_ms == prior.creation_ms
                && row
                    .executable_name
                    .eq_ignore_ascii_case(&prior.executable_name)
        })
    }) {
        return Err("a verified primed application identity is still live; restart refused".into());
    }
    Ok(())
}
fn fresh_exit_check(known: &[Identity]) -> Result<Value, String> {
    let (query, pid, began) = query_exits(known)?;
    let now = now_ms()?;
    validate_exit_query(&query, pid, began, known, now)?;
    Ok(
        json!({"verified":true,"queryPid":pid,"checkedIdentities":known.len(),"checkedAtUnixMs":now}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn row(pid: u32, birth: i64, name: &str) -> Identity {
        Identity {
            pid,
            creation_ms: birth,
            executable_name: name.into(),
        }
    }
    fn query(rows: Vec<Identity>) -> ExitQuery {
        ExitQuery {
            query_pid: 10,
            rows,
        }
    }
    #[test]
    fn warm_exit_metadata_rejects_live_and_ambiguous_identities_but_accepts_pid_reuse() {
        let known = vec![
            row(20, 10, "solo_soul.exe"),
            row(30, 11, "msedgewebview2.exe"),
        ];
        let helper = row(10, 100, "powershell.exe");
        assert!(validate_exit_query(&query(vec![helper.clone()]), 10, 100, &known, 200).is_ok());
        assert!(validate_exit_query(
            &query(vec![helper.clone(), row(20, 101, "other.exe")]),
            10,
            100,
            &known,
            200
        )
        .is_ok());
        for rows in [
            vec![],
            vec![row(10, 99, "powershell.exe")],
            vec![row(10, 100, "other.exe")],
            vec![helper.clone(), known[0].clone()],
            vec![helper.clone(), row(40, 101, "other.exe")],
            vec![helper.clone(), row(20, 101, "")],
            vec![helper.clone(), row(20, 201, "other.exe")],
            vec![
                helper.clone(),
                row(20, 101, "other.exe"),
                row(20, 102, "other.exe"),
            ],
        ] {
            assert!(validate_exit_query(&query(rows), 10, 100, &known, 200).is_err());
        }
        assert!(validate_exit_query(&query(vec![helper]), 11, 100, &known, 200).is_err());
    }
    #[test]
    fn warm_fresh_cim_query_is_bound_to_actual_helper_and_rejects_live_test_process() {
        let requested = vec![row(std::process::id(), 1, "placeholder")];
        let (query, pid, began) = query_exits(&requested).unwrap();
        let actual = query
            .rows
            .iter()
            .find(|r| r.pid == std::process::id())
            .unwrap()
            .clone();
        let now = now_ms().unwrap();
        assert!(
            validate_exit_query(&query, pid, began, std::slice::from_ref(&actual), now)
                .unwrap_err()
                .contains("still live")
        );
        let reused = row(actual.pid, actual.creation_ms - 1, &actual.executable_name);
        assert!(validate_exit_query(&query, pid, began, &[reused], now).is_ok());
    }
    #[test]
    fn warm_modes_cannot_be_combined_with_original_input_or_prepare() {
        fn args(s: &[&str]) -> Vec<std::ffi::OsString> {
            s.iter().map(std::ffi::OsString::from).collect()
        }
        for bad in [
            vec!["--native-perf-restart", "startup"],
            vec![
                "--native-perf-root",
                "C:/owned",
                "--native-perf-port",
                "9222",
                "--native-perf-journey",
                "sdk-input",
                "--native-perf-restart",
                "startup",
            ],
            vec![
                "--native-perf-warm-prepare",
                "C:/owned",
                "--native-perf-root",
                "C:/owned",
            ],
            vec![
                "--native-perf-warm-prepare",
                "C:/owned",
                "--fixture",
                "C:/fixture",
            ],
        ] {
            assert!(super::super::parse_args(&args(&bad)).is_err());
        }
        assert!(matches!(
            super::super::parse_args(&args(&["--native-perf-warm-prepare", "C:/owned"])),
            Ok(super::super::Mode::WarmPrepare { .. })
        ));
        assert!(matches!(
            super::super::parse_args(&args(&[
                "--native-perf-root",
                "C:/owned",
                "--native-perf-port",
                "9222",
                "--native-perf-journey",
                "sdk-startup",
                "--native-perf-restart",
                "startup"
            ])),
            Ok(super::super::Mode::Run {
                startup_only: true,
                restart: true,
                ..
            })
        ));
    }
}
