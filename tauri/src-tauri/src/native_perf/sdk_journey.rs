//! RF-312：仅非默认native-perf构建、公开合成Vault的固定SDK输入行程。
//! 不开放任意表达式、账号、密码或命令入口；默认GUI与两次只读SDK诊断保持原规则。
use super::sdk_cdp::{bounded_callback_json, sdk_binding};
use super::{checked_dir, write_new_json, RuntimeConfig};
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{webview::PageLoadPayload, Manager, WebviewWindow};
use tokio::sync::oneshot;
use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2CallDevToolsProtocolMethodCompletedHandler,
    ICoreWebView2CallDevToolsProtocolMethodCompletedHandler_Impl, ICoreWebView2_4,
};
use webview2_com::{
    FrameCreatedEventHandler, NavigationStartingEventHandler, ProcessFailedEventHandler,
};
use windows::core::{Interface, PCWSTR};

const REQUESTED_FILE: &str = "native-perf-sdk-journey-requested.json";
const PROOF_FILE: &str = "native-perf-sdk-journey.json";
const PROBE: &str = include_str!("sdk_journey.js");
const PASSWORD: &str = "perf-baseline-only-password";
const MAX_CALLS: usize = 128;
const MAX_PHASE_MS: u64 = 50_000;
const MAX_RUN_MS: u64 = 300_000;
static STARTED: OnceLock<Instant> = OnceLock::new();

type Outcome<T> = Result<T, &'static str>;
type CallbackSender = oneshot::Sender<Outcome<Value>>;

pub(super) fn claim_request(config: &RuntimeConfig) -> Result<(), String> {
    if !config.sdk_journey
        || config.sdk_cdp
        || config.chromium_log.is_some()
        || ![100, 5000].contains(&config.object_count)
        || checked_dir(&config.root)? != config.root
    {
        return Err("SDK journey requires its exclusive exact synthetic owned root".into());
    }
    STARTED
        .set(Instant::now())
        .map_err(|_| "SDK journey already requested")?;
    write_new_json(
        &config.root.join(REQUESTED_FILE),
        &json!({
            "schemaVersion": 1, "scope": "windows-native-sdk-ui-journey-requested",
            "root": config.root, "runId": config.run_id, "pid": std::process::id(),
            "port": config.port, "objectCount": config.object_count,
            "inputMethod": "SDK-CDP-Input", "defaultBuild": false,
        }),
    )
}

#[derive(Default)]
struct Gate {
    loaded: bool,
    setup: bool,
    started: bool,
}
#[derive(Clone)]
pub struct Journey {
    config: RuntimeConfig,
    gate: Arc<Mutex<Gate>>,
}
impl Journey {
    pub fn new(config: &RuntimeConfig) -> Option<Self> {
        config.sdk_journey.then(|| Self {
            config: config.clone(),
            gate: Arc::default(),
        })
    }
    pub fn page_loaded(&self, window: WebviewWindow, payload: PageLoadPayload<'_>) {
        if payload.event() == tauri::webview::PageLoadEvent::Finished {
            self.signal(window, true, false);
        }
    }
    pub fn setup_completed(&self, window: WebviewWindow) {
        self.signal(window, false, true);
    }
    fn signal(&self, window: WebviewWindow, loaded: bool, setup: bool) {
        if window.label() != "main" {
            return;
        }
        let begin = self
            .gate
            .lock()
            .map(|mut gate| {
                gate.loaded |= loaded;
                gate.setup |= setup;
                if gate.loaded && gate.setup && !gate.started {
                    gate.started = true;
                    true
                } else {
                    false
                }
            })
            .unwrap_or(false);
        if begin {
            let config = self.config.clone();
            tauri::async_runtime::spawn(async move {
                run(config, window).await;
            });
        }
    }
}

#[windows::core::implement(ICoreWebView2CallDevToolsProtocolMethodCompletedHandler, Agile = false)]
struct Completion {
    sender: RefCell<Option<CallbackSender>>,
    invoked: Cell<bool>,
    invalidated: Arc<AtomicBool>,
}
impl ICoreWebView2CallDevToolsProtocolMethodCompletedHandler_Impl for Completion_Impl {
    fn Invoke(&self, code: windows::core::HRESULT, value: &PCWSTR) -> windows::core::Result<()> {
        if self.invoked.replace(true) {
            self.invalidated.store(true, Ordering::SeqCst);
            return Ok(());
        }
        if let Some(sender) = self.sender.borrow_mut().take() {
            if sender.is_closed() {
                return Ok(());
            }
            let result = if self.invalidated.load(Ordering::SeqCst) {
                Err("document-invalidated")
            } else if code.is_err() {
                Err("sdk-callback-failed")
            } else {
                unsafe { bounded_callback_json(value) }
            };
            let _ = sender.send(result);
        }
        Ok(())
    }
}

// 固定业务行程的完整URL，包括Identity的实际SPA参数；不接受任意查询或片段。
fn allowed_source(source: &str) -> bool {
    matches!(
        source,
        "http://tauri.localhost/"
            | "http://tauri.localhost/login"
            | "http://tauri.localhost/workspace"
            | "http://tauri.localhost/workspace?section=identity"
            | "http://tauri.localhost/search"
    )
}
fn parse_frame(value: &Value, source: &str) -> Outcome<Value> {
    let tree = &value["frameTree"];
    let frame = &tree["frame"];
    let identifier = |value: &Value| {
        value.as_str().is_some_and(|text| {
            !text.is_empty()
                && text.len() <= 128
                && text
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"_-.:".contains(&byte))
        })
    };
    if value.get("error").is_some()
        || !allowed_source(source)
        || !tree.is_object()
        || !frame.is_object()
        || frame.get("parentId").is_some()
        || !identifier(&frame["id"])
        || !identifier(&frame["loaderId"])
        || frame["url"] != source
        || frame
            .get("securityOrigin")
            .is_some_and(|origin| origin != "http://tauri.localhost")
        || tree
            .get("childFrames")
            .is_some_and(|children| !children.as_array().is_some_and(|v| v.is_empty()))
    {
        return Err("frame-tree-mismatch");
    }
    Ok(
        json!({"source":source,"mainFrameId":frame["id"],"loaderId":frame["loaderId"],"timeOriginMs":null,"navigationEvents":0,"frameCreatedEvents":0}),
    )
}

struct Capture {
    config: RuntimeConfig,
    window: WebviewWindow,
    started: Instant,
    browser_pid: Option<u32>,
    frame: Option<Value>,
    time_origin: Option<Value>,
    invalidated: Arc<AtomicBool>,
    tokens: Option<(i64, i64, i64)>,
    calls: Vec<&'static str>,
    phases: Vec<Value>,
    last: Option<Value>,
    step: &'static str,
}
impl Capture {
    async fn protocol(&mut self, method: &'static str, parameters: Value) -> Outcome<Value> {
        if self.calls.len() >= MAX_CALLS || self.started.elapsed().as_millis() > MAX_RUN_MS.into() {
            return Err("sdk-budget-exceeded");
        }
        if self.invalidated.load(Ordering::SeqCst) {
            return Err("document-invalidated");
        }
        self.calls.push(method);
        let (sender, receiver) = oneshot::channel();
        let expected_pid = self.browser_pid;
        let invalidated = self.invalidated.clone();
        let app = self.window.app_handle().clone();
        self.window
            .with_webview(move |platform| {
                let fail = |sender: CallbackSender, reason| {
                    let _ = sender.send(Err(reason));
                };
                if invalidated.load(Ordering::SeqCst) {
                    fail(sender, "document-invalidated");
                    return;
                }
                if app.webview_windows().len() != 1 {
                    fail(sender, "multiple-webviews");
                    return;
                }
                let Ok(core) = (unsafe { platform.controller().CoreWebView2() }) else {
                    fail(sender, "sdk-core-unavailable");
                    return;
                };
                let Ok((pid, source)) = sdk_binding(&core) else {
                    fail(sender, "sdk-binding-unavailable");
                    return;
                };
                if !allowed_source(&source)
                    || expected_pid.is_some_and(|expected| pid != expected)
                    || pid == 0
                {
                    fail(sender, "sdk-binding-mismatch");
                    return;
                }
                if invalidated.load(Ordering::SeqCst) {
                    fail(sender, "document-invalidated");
                    return;
                }
                let method_wide: Vec<_> = method.encode_utf16().chain(Some(0)).collect();
                let parameter_wide: Vec<_> = parameters
                    .to_string()
                    .encode_utf16()
                    .chain(Some(0))
                    .collect();
                let completion: ICoreWebView2CallDevToolsProtocolMethodCompletedHandler =
                    Completion {
                        sender: RefCell::new(Some(sender)),
                        invoked: Cell::new(false),
                        invalidated,
                    }
                    .into();
                // HRESULT失败时丢弃handler会关闭oneshot；调用方固定拒绝，不重试。
                let _ = unsafe {
                    core.CallDevToolsProtocolMethod(
                        PCWSTR(method_wide.as_ptr()),
                        PCWSTR(parameter_wide.as_ptr()),
                        &completion,
                    )
                };
            })
            .map_err(|_| "sdk-dispatch-failed")?;
        let result = tokio::time::timeout(Duration::from_millis(MAX_PHASE_MS), receiver)
            .await
            .map_err(|_| "sdk-callback-timeout")?
            .map_err(|_| "sdk-dispatch-failed")??;
        if result.get("error").is_some()
            || !result.is_object()
            || (method.starts_with("Input.") && result != json!({}))
        {
            return Err("sdk-protocol-error");
        }
        if self.invalidated.load(Ordering::SeqCst) {
            return Err("document-invalidated");
        }
        if self.started.elapsed().as_millis() > MAX_RUN_MS.into() {
            return Err("sdk-budget-exceeded");
        }
        Ok(result)
    }
    async fn bind(&mut self, probe: &Value) -> Outcome<()> {
        let (sender, receiver) = oneshot::channel();
        let invalidated = self.invalidated.clone();
        self.window
            .with_webview(move |platform| {
                let result = (|| -> Outcome<_> {
                    let core = unsafe { platform.controller().CoreWebView2() }
                        .map_err(|_| "sdk-core-unavailable")?;
                    let (pid, source) =
                        sdk_binding(&core).map_err(|_| "sdk-binding-unavailable")?;
                    if pid == 0 || !allowed_source(&source) {
                        return Err("sdk-binding-mismatch");
                    }
                    let core4: ICoreWebView2_4 =
                        core.cast().map_err(|_| "sdk-guard-unavailable")?;
                    let navigation_flag = invalidated.clone();
                    let navigation =
                        NavigationStartingEventHandler::create(Box::new(move |_, _| {
                            navigation_flag.store(true, Ordering::SeqCst);
                            Ok(())
                        }));
                    let process_flag = invalidated.clone();
                    let process = ProcessFailedEventHandler::create(Box::new(move |_, _| {
                        process_flag.store(true, Ordering::SeqCst);
                        Ok(())
                    }));
                    let frame = FrameCreatedEventHandler::create(Box::new(move |_, _| {
                        invalidated.store(true, Ordering::SeqCst);
                        Ok(())
                    }));
                    let (mut nav, mut proc, mut child) = (0, 0, 0);
                    unsafe {
                        core.add_NavigationStarting(&navigation, &mut nav)
                            .map_err(|_| "sdk-guard-unavailable")?;
                        if core.add_ProcessFailed(&process, &mut proc).is_err() {
                            let _ = core.remove_NavigationStarting(nav);
                            return Err("sdk-guard-unavailable");
                        }
                        if core4.add_FrameCreated(&frame, &mut child).is_err() {
                            let _ = core.remove_NavigationStarting(nav);
                            let _ = core.remove_ProcessFailed(proc);
                            return Err("sdk-guard-unavailable");
                        }
                    }
                    Ok((pid, source, (nav, proc, child)))
                })();
                let _ = sender.send(result);
            })
            .map_err(|_| "sdk-dispatch-failed")?;
        let (pid, source, tokens) = tokio::time::timeout(Duration::from_secs(10), receiver)
            .await
            .map_err(|_| "sdk-binding-timeout")?
            .map_err(|_| "sdk-dispatch-failed")??;
        self.browser_pid = Some(pid);
        self.tokens = Some(tokens);
        if probe["href"] != source {
            return Err("sdk-binding-mismatch");
        }
        let tree = self.protocol("Page.getFrameTree", json!({})).await?;
        self.frame = Some(parse_frame(&tree, &source)?);
        self.time_origin = Some(probe["timeOriginMs"].clone());
        Ok(())
    }
    async fn probe(&mut self, step: &'static str) -> Outcome<Value> {
        self.step = step;
        let expression = PROBE.replace(
            "__REQUEST__",
            &json!({"step":step,"runId":self.config.run_id,"objectCount":self.config.object_count})
                .to_string(),
        );
        let raw = self
            .protocol(
                "Runtime.evaluate",
                json!({"expression":expression,"returnByValue":true,"awaitPromise":true}),
            )
            .await?;
        let probe = validate_probe(&raw, step, &self.config.run_id, self.time_origin.as_ref())?;
        // 每个操作前后都核实同一主frame与loader；SPA Source变化只允许固定应用路径。
        if let Some(expected) = self.frame.clone() {
            let tree = self.protocol("Page.getFrameTree", json!({})).await?;
            let frame = parse_frame(&tree, probe["href"].as_str().ok_or("probe-invalid")?)?;
            if frame["mainFrameId"] != expected["mainFrameId"]
                || frame["loaderId"] != expected["loaderId"]
            {
                return Err("document-replaced");
            }
        }
        self.last = Some(probe.clone());
        Ok(probe)
    }
    async fn click(&mut self, target: &'static str) -> Outcome<()> {
        let probe = self.probe(target).await?;
        let point = &probe["target"];
        if point["actionable"] != true
            || !point["x"]
                .as_f64()
                .is_some_and(|x| x.is_finite() && x >= 0.0)
            || !point["y"]
                .as_f64()
                .is_some_and(|y| y.is_finite() && y >= 0.0)
        {
            return Err("target-not-actionable");
        }
        for kind in ["mousePressed", "mouseReleased"] {
            self.protocol(
                "Input.dispatchMouseEvent",
                json!({"type":kind,"x":point["x"],"y":point["y"],"button":"left","clickCount":1}),
            )
            .await?;
        }
        Ok(())
    }
    async fn insert(&mut self, text: &'static str) -> Outcome<()> {
        if !matches!(text, PASSWORD | "needle") {
            return Err("unsupported-input");
        }
        self.protocol("Input.insertText", json!({"text":text}))
            .await?;
        Ok(())
    }
    async fn unlock(&mut self) -> Outcome<()> {
        self.click("password").await?;
        self.insert(PASSWORD).await?;
        self.click("submit").await?;
        self.probe("home").await?;
        Ok(())
    }
    fn record(&mut self, name: &'static str, began: Instant, before: &Value) -> Outcome<()> {
        let after = self.last.as_ref().ok_or("probe-invalid")?;
        let ipc = ipc_delta(&before["observer"], &after["observer"])?;
        self.phases.push(json!({"name":name,"success":true,"durationMs":began.elapsed().as_secs_f64()*1000.0,
            "ipc":ipc,"inputTrust":after["inputTrust"],"fromAtMs":before["atMs"],"toAtMs":after["atMs"]}));
        publish(
            &self.config.root,
            &format!("native-perf-sdk-phase-{:02}.json", self.phases.len()),
            self.phases.last().unwrap(),
        )
    }
    async fn journey(&mut self) -> Outcome<()> {
        let startup = self.probe("startup").await?;
        self.bind(&startup).await?;
        let beginning = STARTED.get().copied().ok_or("missing-start-boundary")?;
        let zero = json!({"observer":{"commands":[],"timeOriginMs":startup["timeOriginMs"],"runId":self.config.run_id,"valid":true},"atMs":0});
        self.record("startup", beginning, &zero)?;
        publish(
            &self.config.root,
            "native-perf-sdk-journey-bound.json",
            &json!({
                "schemaVersion":1,"scope":"windows-native-sdk-ui-bound","runId":self.config.run_id,
                "root":self.config.root,"pid":std::process::id(),"port":self.config.port,
                "browserPid":self.browser_pid,"binding":self.frame,"timeOriginMs":self.time_origin,
            }),
        )?;
        self.await_authorization().await?;
        let before = self.last.clone().unwrap();
        let began = Instant::now();
        self.unlock().await?;
        self.record("password-unlock", began, &before)?;
        let before = self.last.clone().unwrap();
        let began = Instant::now();
        self.click("identity").await?;
        self.click("clear").await?;
        self.probe("workspace").await?;
        self.record("workspace", began, &before)?;
        self.click("homeButton").await?;
        self.probe("home").await?;
        self.click("searchCard").await?;
        self.probe("searchInput").await?;
        let before = self.last.clone().unwrap();
        let began = Instant::now();
        self.click("searchInput").await?;
        self.insert("needle").await?;
        self.probe("search").await?;
        self.record("search-needle", began, &before)?;
        let before = self.last.clone().unwrap();
        let began = Instant::now();
        self.click("lockButton").await?;
        self.probe("locked").await?;
        self.record("application-lock", began, &before)?;
        let before = self.last.clone().unwrap();
        let began = Instant::now();
        self.unlock().await?;
        self.record("password-reunlock", began, &before)?;
        let trust = &self.last.as_ref().unwrap()["inputTrust"];
        if trust["text"].as_u64().unwrap_or(0) < 3
            || trust["pointer"].as_u64().unwrap_or(0) < 10
            || trust["untrusted"] != 0
        {
            return Err("input-trust-mismatch");
        }
        Ok(())
    }
    async fn await_authorization(&self) -> Outcome<()> {
        let file = self
            .config
            .root
            .join("native-perf-sdk-journey-authorized.json");
        let began = Instant::now();
        loop {
            if self.invalidated.load(Ordering::SeqCst) {
                return Err("document-invalidated");
            }
            if began.elapsed() > Duration::from_secs(45) {
                return Err("input-authorization-timeout");
            }
            match std::fs::symlink_metadata(&file) {
                Ok(meta) => {
                    if !meta.is_file()
                        || meta.len() > 512
                        || super::require_regular(&file, false).is_err()
                    {
                        return Err("input-authorization-invalid");
                    }
                    let value =
                        super::read_json(&file).map_err(|_| "input-authorization-invalid")?;
                    if value.as_object().map_or(0, |v| v.len()) != 5
                        || value["schemaVersion"] != 1
                        || value["scope"] != "windows-native-sdk-ui-input-authorized"
                        || value["runId"] != self.config.run_id
                        || value["pid"] != std::process::id()
                        || value["browserPid"] != self.browser_pid.unwrap_or(0)
                    {
                        return Err("input-authorization-invalid");
                    }
                    return Ok(());
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    tokio::time::sleep(Duration::from_millis(25)).await
                }
                Err(_) => return Err("input-authorization-invalid"),
            }
        }
    }
    async fn release_guards(&mut self) -> Outcome<()> {
        let Some((nav, proc, child)) = self.tokens.take() else {
            return Ok(());
        };
        let (sender, receiver) = oneshot::channel();
        self.window
            .with_webview(move |platform| {
                let result = (|| -> Outcome<()> {
                    let core = unsafe { platform.controller().CoreWebView2() }
                        .map_err(|_| "guard-cleanup-failed")?;
                    let core4: ICoreWebView2_4 = core.cast().map_err(|_| "guard-cleanup-failed")?;
                    unsafe {
                        core.remove_NavigationStarting(nav)
                            .map_err(|_| "guard-cleanup-failed")?;
                        core.remove_ProcessFailed(proc)
                            .map_err(|_| "guard-cleanup-failed")?;
                        core4
                            .remove_FrameCreated(child)
                            .map_err(|_| "guard-cleanup-failed")?;
                    }
                    Ok(())
                })();
                let _ = sender.send(result);
            })
            .map_err(|_| "guard-cleanup-failed")?;
        tokio::time::timeout(Duration::from_secs(10), receiver)
            .await
            .map_err(|_| "guard-cleanup-failed")?
            .map_err(|_| "guard-cleanup-failed")?
    }
}

fn validate_probe(
    raw: &Value,
    step: &str,
    run_id: &str,
    time_origin: Option<&Value>,
) -> Outcome<Value> {
    if raw.get("exceptionDetails").is_some() || raw["result"]["type"] != "object" {
        return Err("probe-evaluation-failed");
    }
    let probe = &raw["result"]["value"];
    if probe["outcome"] == "timeout" {
        return Err("probe-timeout");
    }
    if probe["outcome"] == "document-mismatch" {
        return Err("probe-document-mismatch");
    }
    if probe.as_object().map_or(0, |v| v.len()) != 14
        || probe["schemaVersion"] != 1
        || probe["scope"] != "windows-native-sdk-ui-probe"
        || probe["runId"] != run_id
        || probe["step"] != step
        || probe["outcome"] != "ready"
        || probe["origin"] != "http://tauri.localhost"
        || probe["rootPresent"] != true
        || probe["frameCount"] != 0
        || !probe["href"].as_str().is_some_and(allowed_source)
        || !probe["timeOriginMs"]
            .as_f64()
            .is_some_and(|n| n.is_finite() && n > 0.0)
        || !probe["atMs"]
            .as_f64()
            .is_some_and(|n| n.is_finite() && n >= 0.0)
        || time_origin.is_some_and(|expected| expected != &probe["timeOriginMs"])
    {
        return Err("probe-invalid");
    }
    validate_observer(&probe["observer"], run_id, &probe["timeOriginMs"])?;
    let target = &probe["target"];
    if !target.is_null()
        && (target.as_object().map_or(0, |v| v.len()) != 3
            || !target["actionable"].is_boolean()
            || ["x", "y"].iter().any(|key| {
                target[key]
                    .as_f64()
                    .is_none_or(|n| !n.is_finite() || n < 0.0)
            }))
    {
        return Err("probe-invalid");
    }
    let at = probe["atMs"].as_f64().unwrap();
    if probe["observer"]["installedAtMs"].as_f64().unwrap() > at
        || probe["observer"]["commands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["atMs"].as_f64().unwrap() > at)
    {
        return Err("observer-invalid");
    }
    let trust = &probe["inputTrust"];
    if trust.as_object().map_or(0, |v| v.len()) != 3
        || ["pointer", "text", "untrusted"]
            .iter()
            .any(|key| trust[key].as_u64().is_none_or(|n| n > 128))
    {
        return Err("input-trust-mismatch");
    }
    Ok(probe.clone())
}
fn validate_observer(value: &Value, run_id: &str, time_origin: &Value) -> Outcome<()> {
    let commands = value["commands"].as_array().ok_or("observer-invalid")?;
    if value.as_object().map_or(0, |v| v.len()) != 11
        || value["observedCount"] != value["total"]
        || value["maxEvents"] != 16384
        || value["schemaVersion"] != 1
        || value["scope"] != "windows-native-tauri-invoke-observer"
        || value["runId"] != run_id
        || value["valid"] != true
        || value["timeOriginMs"] != *time_origin
        || value["invalidReasons"] != json!([])
        || value["total"].as_u64() != Some(commands.len() as u64)
        || commands.len() > 16384
        || !value["installedAtMs"]
            .as_f64()
            .is_some_and(|n| n.is_finite() && n >= 0.0)
        || commands.iter().any(|v| {
            v.as_object().map_or(0, |v| v.len()) != 2
                || !v["command"]
                    .as_str()
                    .is_some_and(|s| !s.is_empty() && s.len() <= 256)
                || !v["atMs"]
                    .as_f64()
                    .is_some_and(|n| n.is_finite() && n >= value["installedAtMs"].as_f64().unwrap())
        })
    {
        return Err("observer-invalid");
    }
    Ok(())
}
fn ipc_delta(before: &Value, after: &Value) -> Outcome<Value> {
    let a = before["commands"].as_array().ok_or("observer-invalid")?;
    let b = after["commands"].as_array().ok_or("observer-invalid")?;
    if before["timeOriginMs"] != after["timeOriginMs"]
        || before["runId"] != after["runId"]
        || a.len() > b.len()
        || a != &b[..a.len()]
    {
        return Err("observer-prefix-changed");
    }
    let mut commands = std::collections::BTreeMap::<&str, usize>::new();
    for entry in &b[a.len()..] {
        *commands
            .entry(entry["command"].as_str().ok_or("observer-invalid")?)
            .or_default() += 1;
    }
    Ok(json!({"attempts":b.len()-a.len(),"commands":commands,"reason":null}))
}
async fn run(config: RuntimeConfig, window: WebviewWindow) {
    let mut capture = Capture {
        config,
        window,
        started: Instant::now(),
        browser_pid: None,
        frame: None,
        time_origin: None,
        invalidated: Arc::new(AtomicBool::new(false)),
        tokens: None,
        calls: vec![],
        phases: vec![],
        last: None,
        step: "startup",
    };
    let mut outcome = tokio::time::timeout(Duration::from_millis(MAX_RUN_MS), capture.journey())
        .await
        .unwrap_or(Err("sdk-budget-exceeded"));
    if capture.invalidated.load(Ordering::SeqCst) {
        outcome = Err("document-invalidated");
    }
    if let Err(reason) = capture.release_guards().await {
        outcome = Err(reason);
    }
    let ipc_all=capture.last.as_ref().and_then(|last|ipc_delta(&json!({"commands":[],"timeOriginMs":last["timeOriginMs"],"runId":capture.config.run_id}),&last["observer"]).ok());
    let proof = json!({"schemaVersion":1,"scope":"windows-native-sdk-ui-journey","runId":capture.config.run_id,
        "root":capture.config.root,"pid":std::process::id(),"port":capture.config.port,"objectCount":capture.config.object_count,
        "browserPid":capture.browser_pid,"binding":capture.frame,"timeOriginMs":capture.time_origin,
        "inputMethod":"SDK-CDP-Input","success":outcome.is_ok(),"reason":outcome.err(),"failedStep":if outcome.is_err(){Some(capture.step)}else{None},
        "calls":capture.calls,"phases":capture.phases,"lastProbe":capture.last,"ipcAll":ipc_all,
        "elapsedMs":capture.started.elapsed().as_secs_f64()*1000.0,
        "unmeasured":["OCR","attachment-preview","system-sleep","same-profile-warm-start","other-platforms"]});
    let _ = publish(&capture.config.root, PROOF_FILE, &proof);
}

fn publish(root: &std::path::Path, name: &str, value: &Value) -> Outcome<()> {
    let staged = root.join(format!(".{name}.tmp"));
    let final_file = root.join(name);
    if std::fs::symlink_metadata(&final_file).is_ok() {
        return Err("proof-already-exists");
    }
    write_new_json(&staged, value).map_err(|_| "proof-publication-failed")?;
    std::fs::hard_link(&staged, final_file).map_err(|_| "proof-publication-failed")?;
    std::fs::remove_file(staged).map_err(|_| "proof-publication-failed")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sdk_journey_only_allows_fixed_app_routes() {
        for source in [
            "http://tauri.localhost/",
            "http://tauri.localhost/login",
            "http://tauri.localhost/workspace?section=identity",
            "http://tauri.localhost/search",
        ] {
            assert!(allowed_source(source));
        }
        for source in [
            "https://tauri.localhost/login",
            "http://tauri.localhost:3000/login",
            "http://user@tauri.localhost/login",
            "http://tauri.localhost/settings",
            "http://example.org/login",
        ] {
            assert!(!allowed_source(source));
        }
    }
    #[test]
    fn sdk_journey_rejects_changed_observer_prefix() {
        let before =
            json!({"runId":"abc","timeOriginMs":1,"commands":[{"command":"auth","atMs":1}]});
        let after = json!({"runId":"abc","timeOriginMs":1,"commands":[{"command":"auth","atMs":1},{"command":"list","atMs":2}]});
        assert_eq!(ipc_delta(&before, &after).unwrap()["attempts"], 1);
        let mut replaced = after.clone();
        replaced["commands"][0]["command"] = json!("other");
        assert_eq!(
            ipc_delta(&before, &replaced).unwrap_err(),
            "observer-prefix-changed"
        );
    }
    #[test]
    fn sdk_journey_requires_valid_complete_observer_and_no_payload() {
        let value = json!({"schemaVersion":1,"scope":"windows-native-tauri-invoke-observer","runId":"abc","valid":true,"timeOriginMs":1,"installedAtMs":0,"invalidReasons":[],"total":1,"observedCount":1,"maxEvents":16384,"commands":[{"command":"auth","atMs":1}]});
        assert!(validate_observer(&value, "abc", &json!(1)).is_ok());
        for (key, bad) in [
            ("valid", json!(false)),
            ("total", json!(2)),
            ("timeOriginMs", json!(2)),
            ("runId", json!("wrong")),
            ("invalidReasons", json!(["transport-fallback"])),
        ] {
            let mut changed = value.clone();
            changed[key] = bad;
            assert!(validate_observer(&changed, "abc", &json!(1)).is_err());
        }
        let mut leaked = value;
        leaked["commands"][0]["payload"] = json!("private");
        assert!(validate_observer(&leaked, "abc", &json!(1)).is_err());
    }
}

#[cfg(test)]
mod probe_tests {
    use super::*;
    fn probe() -> Value {
        json!({"result":{"type":"object","value":{
            "schemaVersion":1,"scope":"windows-native-sdk-ui-probe","runId":"abc","step":"home","outcome":"ready",
            "href":"http://tauri.localhost/","origin":"http://tauri.localhost","timeOriginMs":10,"atMs":3,
            "rootPresent":true,"frameCount":0,"target":null,"inputTrust":{"pointer":10,"text":3,"untrusted":0},
            "observer":{"schemaVersion":1,"scope":"windows-native-tauri-invoke-observer","runId":"abc","valid":true,
                "total":1,"observedCount":1,"invalidReasons":[],"installedAtMs":0,"timeOriginMs":10,"maxEvents":16384,"commands":[{"command":"auth","atMs":1}]}
        }}})
    }
    #[test]
    fn fixed_probe_rejects_extra_fields_changed_clock_and_future_events() {
        assert!(validate_probe(&probe(), "home", "abc", Some(&json!(10))).is_ok());
        for (key, value) in [
            ("private", json!("sentinel")),
            ("timeOriginMs", json!(11)),
            ("frameCount", json!(1)),
            (
                "target",
                json!({"x":1,"y":1,"actionable":true,"private":"sentinel"}),
            ),
        ] {
            let mut changed = probe();
            changed["result"]["value"][key] = value;
            assert!(validate_probe(&changed, "home", "abc", Some(&json!(10))).is_err());
        }
        let mut future = probe();
        future["result"]["value"]["observer"]["commands"][0]["atMs"] = json!(4);
        assert_eq!(
            validate_probe(&future, "home", "abc", Some(&json!(10))).unwrap_err(),
            "observer-invalid"
        );
    }
    #[test]
    fn fixed_probe_timeout_never_becomes_success() {
        let mut timed = probe();
        timed["result"]["value"]["outcome"] = json!("timeout");
        assert_eq!(
            validate_probe(&timed, "home", "abc", None).unwrap_err(),
            "probe-timeout"
        );
    }
}

#[cfg(test)]
mod frame_tests {
    use super::*;
    #[test]
    fn journey_accepts_exact_identity_spa_route_and_keeps_single_main_document_rules() {
        let source = "http://tauri.localhost/workspace?section=identity";
        let tree = json!({"frameTree":{"frame":{"id":"FRAME","loaderId":"LOADER","url":source,"securityOrigin":"http://tauri.localhost"}}});
        assert!(parse_frame(&tree, source).is_ok());
        for bad_source in [
            "http://tauri.localhost/workspace?section=travel",
            "http://tauri.localhost/workspace?section=identity&private=sentinel",
            "http://tauri.localhost/workspace#other",
            "http://tauri.localhost/workspace?category=identity",
        ] {
            assert!(!allowed_source(bad_source));
        }
        for (key, bad) in [
            ("url", json!("http://tauri.localhost/workspace")),
            ("parentId", json!("PARENT")),
            ("loaderId", json!(null)),
            ("securityOrigin", json!("http://other")),
        ] {
            let mut changed = tree.clone();
            changed["frameTree"]["frame"][key] = bad;
            assert!(parse_frame(&changed, source).is_err());
        }
        let mut children = tree;
        children["frameTree"]["childFrames"] = json!([{}]);
        assert!(parse_frame(&children, source).is_err());
    }
}
