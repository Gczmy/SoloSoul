//! 显式 sdk-cdp 诊断：原始主 WebView、两次只读调用，无 TCP CDP 端点连接或业务操作。
use super::{checked_dir, write_new_json, RuntimeConfig};
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::fs;
use std::path::Path;
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{webview::PageLoadPayload, Manager, WebviewWindow};
use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2, ICoreWebView2CallDevToolsProtocolMethodCompletedHandler,
    ICoreWebView2CallDevToolsProtocolMethodCompletedHandler_Impl, ICoreWebView2_4,
};
use webview2_com::{
    FrameCreatedEventHandler, NavigationStartingEventHandler, ProcessFailedEventHandler,
    SourceChangedEventHandler,
};
use windows::core::{Interface, PCWSTR, PWSTR};

pub(super) const REQUESTED_FILE: &str = "native-perf-sdk-cdp-requested.json";
const PROOF_FILE: &str = "native-perf-sdk-cdp.json";
const STAGED_PROOF_FILE: &str = ".native-perf-sdk-cdp-proof.tmp";
const EXPECTED_ORIGIN: &str = "http://tauri.localhost";
const MAX_RAW_BYTES: usize = 512 * 1024;
const MAX_PROOF_BYTES: usize = 1024 * 1024;
const MAX_ELAPSED_MS: u128 = 20_000;
const FRAME_METHOD: &str = "Page.getFrameTree";
const EVALUATE_METHOD: &str = "Runtime.evaluate";
// 固定仅读表达式；不取 input.value、DOM 文本、账户数据或 IPC body。
const EXPRESSION: &str = r#"(() => {
  const root = document.getElementById('root');
  const screen = document.getElementById('startup-screen');
  const state = screen?.dataset.state;
  const rootHasChildren = Boolean(root?.hasChildNodes());
  return {
    origin: location.origin, href: location.href, documentUrl: document.URL,
    readyState: document.readyState, mainFrame: window.top === window,
    frameCount: document.querySelectorAll('iframe,frame').length,
    uiRootPresent: rootHasChildren,
    uiRootDiagnostic: {
      schemaVersion: 1, rootExists: Boolean(root), rootHasChildren,
      reactMountMarked: performance.getEntriesByName('solosoul:react-mount', 'mark').length > 0,
      startupScreenPresent: Boolean(screen),
      startupState: ['loading', 'error', 'ready'].includes(state) ? state : 'unavailable'
    },
    timeOriginMs: performance.timeOrigin, atMs: performance.now(),
    observer: window.__SOLOSOUL_NATIVE_PERF__?.snapshot()
  };
})()"#;

pub(super) fn claim_request(config: &RuntimeConfig) -> Result<(), String> {
    if !config.sdk_cdp
        || config.chromium_log.is_some()
        || checked_dir(&config.root)?.as_os_str() != config.root.as_os_str()
    {
        return Err("SDK CDP requires its exclusive exact owned diagnostic root".into());
    }
    match fs::symlink_metadata(config.root.join(PROOF_FILE)) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        _ => return Err("SDK CDP proof path already exists or is unavailable".into()),
    }
    write_new_json(
        &config.root.join(REQUESTED_FILE),
        &json!({
            "schemaVersion": 1, "scope": "windows-native-sdk-cdp-requested", "mode": "sdk-cdp",
            "performanceSample": false, "root": config.root, "runId": config.run_id,
            "pid": std::process::id(), "port": config.port, "windowLabel": "main",
        }),
    )
}

#[derive(Default)]
struct Gate {
    loaded: bool,
    setup: bool,
    started: bool,
}
impl Gate {
    fn signal(&mut self, loaded: bool, setup: bool) -> bool {
        self.loaded |= loaded;
        self.setup |= setup;
        if self.loaded && self.setup && !self.started {
            self.started = true;
            true
        } else {
            false
        }
    }
}

/// 可跨线程的门闩只持有普通数据；COM 只在 with_webview 及其 UI 回调中创建。
#[derive(Clone)]
pub struct Diagnostic {
    config: RuntimeConfig,
    gate: Arc<Mutex<Gate>>,
}
impl Diagnostic {
    pub fn new(config: &RuntimeConfig) -> Option<Self> {
        config.sdk_cdp.then(|| Self {
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
        let launch = self
            .gate
            .lock()
            .map(|mut gate| gate.signal(loaded, setup))
            .unwrap_or(false);
        if !launch {
            return;
        }
        let config = self.config.clone();
        let app = window.app_handle().clone();
        let on_error = self.config.clone();
        if window
            .with_webview(move |platform| {
                let start = Instant::now();
                let windows = app.webview_windows();
                if windows.len() != 1 || !windows.contains_key("main") {
                    publish_early_failure(&config, start, "multiple-webviews");
                    return;
                }
                let core = unsafe { platform.controller().CoreWebView2() };
                match core {
                    Ok(core) => Session::begin(config, core, start),
                    Err(_) => publish_early_failure(&config, start, "sdk-core-unavailable"),
                }
            })
            .is_err()
        {
            publish_early_failure(&on_error, Instant::now(), "main-thread-dispatch-failed");
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Ready,
    FramePending,
    FrameComplete,
    EvaluationPending,
    Done,
}
#[derive(Debug)]
struct Capture {
    stage: Stage,
    calls: Vec<&'static str>,
    reason: Option<&'static str>,
    navigation_events: u32,
    frame_created_events: u32,
    binding: Option<Value>,
    document: Option<Value>,
    observer: Option<Value>,
    ui_root_diagnostic: Option<Value>,
}
impl Default for Capture {
    fn default() -> Self {
        Self {
            stage: Stage::Ready,
            calls: vec![],
            reason: None,
            navigation_events: 0,
            frame_created_events: 0,
            binding: None,
            document: None,
            observer: None,
            ui_root_diagnostic: None,
        }
    }
}
impl Capture {
    fn issue(&mut self, method: &'static str) -> bool {
        if self.reason.is_some() {
            return false;
        }
        let next = match (self.stage, method) {
            (Stage::Ready, FRAME_METHOD) => Stage::FramePending,
            (Stage::FrameComplete, EVALUATE_METHOD) => Stage::EvaluationPending,
            _ => return false,
        };
        self.stage = next;
        self.calls.push(method);
        true
    }
    fn accepts(&self, method: &str) -> bool {
        matches!(
            (self.stage, method),
            (Stage::FramePending, FRAME_METHOD) | (Stage::EvaluationPending, EVALUATE_METHOD)
        )
    }
    fn invalidate(&mut self, reason: &'static str) {
        if self.stage == Stage::Done {
            return;
        }
        if reason == "navigation-changed" {
            self.navigation_events = self.navigation_events.saturating_add(1);
        }
        if reason == "frame-created" {
            self.frame_created_events = self.frame_created_events.saturating_add(1);
        }
        self.reason.get_or_insert(reason);
    }
    fn finish(&mut self) -> bool {
        if self.stage == Stage::Done {
            false
        } else {
            self.stage = Stage::Done;
            true
        }
    }
}

fn capture_health(capture: &Capture, elapsed_ms: u128) -> Result<(), &'static str> {
    if capture.stage == Stage::Done {
        return Err("capture-finished");
    }
    if let Some(reason) = capture.reason {
        return Err(reason);
    }
    if elapsed_ms > MAX_ELAPSED_MS {
        return Err("callback-timeout");
    }
    Ok(())
}

fn app_source(source: &str) -> bool {
    if source.len() > 2048
        || !(source == EXPECTED_ORIGIN || source.starts_with("http://tauri.localhost/"))
        || source
            .chars()
            .any(|character| character <= ' ' || character == '\\')
    {
        return false;
    }
    url::Url::parse(source).is_ok_and(|url| {
        url.origin().ascii_serialization() == EXPECTED_ORIGIN
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
    })
}
fn identifier(value: &Value) -> bool {
    value.as_str().is_some_and(|text| {
        !text.is_empty()
            && text.len() <= 128
            && text
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"_-.:".contains(&byte))
    })
}
fn parse_frame(value: &Value, source: &str) -> Result<Value, &'static str> {
    if value.get("error").is_some() {
        return Err("cdp-error");
    }
    let tree = &value["frameTree"];
    let frame = &tree["frame"];
    if !app_source(source)
        || !tree.is_object()
        || !frame.is_object()
        || frame.get("parentId").is_some()
        || !identifier(&frame["id"])
        || !identifier(&frame["loaderId"])
        || frame["url"] != source
        || frame
            .get("securityOrigin")
            .is_some_and(|origin| origin != EXPECTED_ORIGIN)
        || tree.get("childFrames").is_some_and(|children| {
            !children
                .as_array()
                .is_some_and(|children| children.is_empty())
        })
    {
        return Err("frame-tree-mismatch");
    }
    Ok(json!({
        "source": source, "mainFrameId": frame["id"], "loaderId": frame["loaderId"],
        "timeOriginMs": null, "navigationEvents": 0, "frameCreatedEvents": 0,
    }))
}
fn finite_nonnegative(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .filter(|number| number.is_finite() && *number >= 0.0)
}
fn parse_evaluation(
    value: &Value,
    source: &str,
    run_id: &str,
) -> Result<(Value, Value, Value), &'static str> {
    if value.get("error").is_some() || value.get("exceptionDetails").is_some() {
        return Err("evaluation-exception");
    }
    let result = &value["result"];
    let payload = &result["value"];
    // 只输出固定拒绝码，不保存被拒绝的 URL、DOM、时间或 observer 原值。
    if result["type"] != "object" {
        return Err("document-result-type");
    }
    if !payload.is_object() {
        return Err("document-payload-type");
    }
    if payload["origin"] != EXPECTED_ORIGIN {
        return Err("document-origin-mismatch");
    }
    if payload["href"] != source {
        return Err("document-href-mismatch");
    }
    if payload["documentUrl"] != source {
        return Err("document-url-mismatch");
    }
    if payload["readyState"] != "complete" {
        return Err("document-ready-state");
    }
    if payload["mainFrame"] != true {
        return Err("document-not-main-frame");
    }
    if payload["frameCount"] != 0 {
        return Err("document-child-frames");
    }
    if payload["uiRootPresent"] != true {
        return Err("document-ui-root-absent");
    }
    if finite_nonnegative(&payload["atMs"]).is_none() {
        return Err("document-clock-invalid");
    }
    let observer = &payload["observer"];
    if observer["schemaVersion"] != 1
        || observer["scope"] != "windows-native-tauri-invoke-observer"
        || observer["runId"] != run_id
        || observer["valid"] != true
        || !finite_nonnegative(&payload["timeOriginMs"]).is_some_and(|origin| origin > 0.0)
        || payload["timeOriginMs"] != observer["timeOriginMs"]
    {
        return Err("observer-mismatch");
    }
    Ok((
        json!({
            "origin": EXPECTED_ORIGIN, "readyState": "complete", "mainFrame": true,
            "frameCount": 0, "uiRootPresent": true,
        }),
        observer.clone(),
        payload["timeOriginMs"].clone(),
    ))
}

// 仅在已通过文档绑定、被原有 root 条件拒绝时保存白名单结构；
// 不解释错误原因，不接纳私有原值，也不改变 parse_evaluation 的成功标准。
fn ui_root_diagnostic(value: &Value) -> Option<Value> {
    let payload = &value["result"]["value"];
    let probe = &payload["uiRootDiagnostic"];
    if payload["uiRootPresent"] != false
        || probe.as_object()?.len() != 6
        || probe["schemaVersion"] != 1
    {
        return None;
    }
    let root_exists = probe["rootExists"].as_bool()?;
    let root_has_children = probe["rootHasChildren"].as_bool()?;
    let react_mount_marked = probe["reactMountMarked"].as_bool()?;
    let startup_screen_present = probe["startupScreenPresent"].as_bool()?;
    let startup_state = probe["startupState"].as_str()?;
    if root_has_children
        || !matches!(startup_state, "loading" | "error" | "ready" | "unavailable")
        || (!startup_screen_present && startup_state != "unavailable")
    {
        return None;
    }
    let classification = if !root_exists {
        "root-container-absent"
    } else if react_mount_marked {
        "react-mount-marked-root-empty"
    } else if startup_state == "error" {
        "startup-error-before-react-mount"
    } else {
        "root-empty-before-react-mount"
    };
    Some(json!({
        "schemaVersion": 1, "scope": "windows-native-ui-root-diagnostic",
        "rootExists": root_exists, "rootHasChildren": root_has_children,
        "reactMountMarked": react_mount_marked,
        "startupScreenPresent": startup_screen_present, "startupState": startup_state,
        "classification": classification,
    }))
}

/// SDK 提供有效 NUL 结尾缓冲区。先限制原始 UTF-16 和转换后 UTF-8 字节，
/// 再分配 owned String；借用 PCWSTR 从不释放，不使用无界 as_wide/to_string。
unsafe fn bounded_callback_json(source: &PCWSTR) -> Result<Value, &'static str> {
    if source.is_null() {
        return Err("empty-callback");
    }
    let max_units = MAX_RAW_BYTES / 2;
    let mut length = 0;
    loop {
        let unit = unsafe { *source.0.add(length) };
        if unit == 0 {
            break;
        }
        if length == max_units {
            return Err("callback-too-large");
        }
        length += 1;
    }
    let units = unsafe { std::slice::from_raw_parts(source.0, length) };
    let mut utf8_bytes = 0;
    for character in char::decode_utf16(units.iter().copied()) {
        utf8_bytes += character.map_err(|_| "invalid-callback-utf16")?.len_utf8();
        if utf8_bytes > MAX_RAW_BYTES {
            return Err("callback-too-large");
        }
    }
    let text = String::from_utf16(units).map_err(|_| "invalid-callback-utf16")?;
    serde_json::from_str(&text).map_err(|_| "invalid-callback-json")
}

#[derive(Clone, Copy)]
enum Guard {
    Navigation,
    Source,
    Process,
    Frame,
}
struct Session {
    config: RuntimeConfig,
    core: ICoreWebView2,
    core4: ICoreWebView2_4,
    browser_pid: u32,
    source: String,
    started: Instant,
    capture: RefCell<Capture>,
    guards: RefCell<Vec<(Guard, i64)>>,
}
impl Session {
    fn begin(config: RuntimeConfig, core: ICoreWebView2, started: Instant) {
        let binding = sdk_binding(&core);
        let core4 = core.cast::<ICoreWebView2_4>();
        let (browser_pid, source, core4) = match (binding, core4) {
            (Ok((pid, source)), Ok(core4)) if pid != 0 && app_source(&source) => {
                (pid, source, core4)
            }
            _ => {
                publish_early_failure(&config, started, "sdk-binding-mismatch");
                return;
            }
        };
        let session = Rc::new(Self {
            config,
            core,
            core4,
            browser_pid,
            source,
            started,
            capture: RefCell::default(),
            guards: RefCell::default(),
        });
        if session.install_guards().is_err() {
            session.finish(false, "guards", Some("sdk-guard-unavailable"));
            return;
        }
        session.issue(FRAME_METHOD);
    }
    fn event(&self, reason: &'static str) {
        self.capture.borrow_mut().invalidate(reason);
    }
    fn install_guards(self: &Rc<Self>) -> windows::core::Result<()> {
        let weak = Rc::downgrade(self);
        let navigation = NavigationStartingEventHandler::create(Box::new(move |_, _| {
            invalidate_weak(&weak, "navigation-changed");
            Ok(())
        }));
        let mut token = 0;
        unsafe {
            self.core.add_NavigationStarting(&navigation, &mut token)?;
        }
        self.guards.borrow_mut().push((Guard::Navigation, token));
        let weak = Rc::downgrade(self);
        let source = SourceChangedEventHandler::create(Box::new(move |_, _| {
            invalidate_weak(&weak, "source-changed");
            Ok(())
        }));
        unsafe {
            self.core.add_SourceChanged(&source, &mut token)?;
        }
        self.guards.borrow_mut().push((Guard::Source, token));
        let weak = Rc::downgrade(self);
        let process = ProcessFailedEventHandler::create(Box::new(move |_, _| {
            invalidate_weak(&weak, "process-failed");
            Ok(())
        }));
        unsafe {
            self.core.add_ProcessFailed(&process, &mut token)?;
        }
        self.guards.borrow_mut().push((Guard::Process, token));
        let weak = Rc::downgrade(self);
        let frame = FrameCreatedEventHandler::create(Box::new(move |_, _| {
            invalidate_weak(&weak, "frame-created");
            Ok(())
        }));
        unsafe {
            self.core4.add_FrameCreated(&frame, &mut token)?;
        }
        self.guards.borrow_mut().push((Guard::Frame, token));
        Ok(())
    }
    fn healthy(&self) -> Result<(), &'static str> {
        capture_health(&self.capture.borrow(), self.started.elapsed().as_millis())?;
        // COM getter 可在 UI 线程重入 guard/callback；调用期间不持 RefCell 借用。
        let binding = sdk_binding(&self.core);
        capture_health(&self.capture.borrow(), self.started.elapsed().as_millis())?;
        match binding {
            Ok((pid, source)) if pid == self.browser_pid && source == self.source => Ok(()),
            _ => Err("sdk-binding-changed"),
        }
    }
    fn issue(self: &Rc<Self>, method: &'static str) {
        if let Err(reason) = self.healthy() {
            self.finish(false, "binding", Some(reason));
            return;
        }
        if !self.capture.borrow_mut().issue(method) {
            self.finish(false, "ordering", Some("callback-order"));
            return;
        }
        let parameters = if method == FRAME_METHOD {
            "{}".to_string()
        } else {
            json!({"expression": EXPRESSION, "returnByValue": true, "awaitPromise": false})
                .to_string()
        };
        let method_wide: Vec<_> = method.encode_utf16().chain(Some(0)).collect();
        let params_wide: Vec<_> = parameters.encode_utf16().chain(Some(0)).collect();
        let completion: ICoreWebView2CallDevToolsProtocolMethodCompletedHandler = Completion {
            session: self.clone(),
            method,
            invoked: Cell::new(false),
        }
        .into();
        if unsafe {
            self.core.CallDevToolsProtocolMethod(
                PCWSTR(method_wide.as_ptr()),
                PCWSTR(params_wide.as_ptr()),
                &completion,
            )
        }
        .is_err()
        {
            self.finish(false, "dispatch", Some("sdk-dispatch-failed"));
        }
    }
    fn complete(self: &Rc<Self>, method: &'static str, result: Result<Value, &'static str>) {
        if self.capture.borrow().stage == Stage::Done {
            return;
        }
        if !self.capture.borrow().accepts(method) {
            self.finish(false, "ordering", Some("callback-order"));
            return;
        }
        if let Err(reason) = self.healthy() {
            self.finish(false, "binding", Some(reason));
            return;
        }
        let value = match result {
            Ok(value) => value,
            Err(reason) => {
                self.finish(false, "callback", Some(reason));
                return;
            }
        };
        if method == FRAME_METHOD {
            match parse_frame(&value, &self.source) {
                Ok(binding) => {
                    let mut capture = self.capture.borrow_mut();
                    capture.binding = Some(binding);
                    capture.stage = Stage::FrameComplete;
                }
                Err(reason) => {
                    self.finish(false, "frame-tree", Some(reason));
                    return;
                }
            }
            self.issue(EVALUATE_METHOD);
        } else {
            match parse_evaluation(&value, &self.source, &self.config.run_id) {
                Ok((document, observer, time_origin)) => {
                    let mut capture = self.capture.borrow_mut();
                    capture.document = Some(document);
                    capture.observer = Some(observer);
                    capture.binding.as_mut().unwrap()["timeOriginMs"] = time_origin;
                }
                Err(reason) => {
                    if reason == "document-ui-root-absent" {
                        self.capture.borrow_mut().ui_root_diagnostic = ui_root_diagnostic(&value);
                    }
                    self.finish(false, "evaluation", Some(reason));
                    return;
                }
            }
            self.finish(true, "complete", None);
        }
    }
    fn finish(&self, success: bool, stage: &'static str, reason: Option<&'static str>) {
        let (success, stage, reason) = if success {
            match self.healthy() {
                Ok(()) => (true, stage, None),
                Err(reason) => (false, "binding", Some(reason)),
            }
        } else {
            (false, stage, reason)
        };
        let proof = {
            let mut capture = self.capture.borrow_mut();
            // getter 返回后封住 guard 失效；此后不再调用 COM getter。
            let (success, stage, reason) = if success {
                match capture_health(&capture, self.started.elapsed().as_millis()) {
                    Ok(()) => (true, stage, reason),
                    Err(reason) => (false, "binding", Some(reason)),
                }
            } else {
                (false, stage, reason)
            };
            if !capture.finish() {
                return;
            }
            let navigation_events = capture.navigation_events;
            let frame_created_events = capture.frame_created_events;
            if let Some(binding) = &mut capture.binding {
                binding["navigationEvents"] = json!(navigation_events);
                binding["frameCreatedEvents"] = json!(frame_created_events);
            }
            proof_value(
                &self.config,
                Some(self.browser_pid),
                &capture,
                success,
                stage,
                reason,
                self.started.elapsed().as_millis(),
            )
        };
        publish(&self.config.root, &proof);
        self.remove_guards();
    }
    fn remove_guards(&self) {
        let guards = std::mem::take(&mut *self.guards.borrow_mut());
        for (guard, token) in guards {
            let _ = unsafe {
                match guard {
                    Guard::Navigation => self.core.remove_NavigationStarting(token),
                    Guard::Source => self.core.remove_SourceChanged(token),
                    Guard::Process => self.core.remove_ProcessFailed(token),
                    Guard::Frame => self.core4.remove_FrameCreated(token),
                }
            };
        }
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.remove_guards();
    }
}
fn invalidate_weak(weak: &Weak<Session>, reason: &'static str) {
    if let Some(session) = weak.upgrade() {
        session.event(reason);
    }
}
fn sdk_binding(core: &ICoreWebView2) -> Result<(u32, String), ()> {
    let mut pid = 0;
    unsafe { core.BrowserProcessId(&mut pid) }.map_err(|_| ())?;
    let mut source = PWSTR::null();
    let result = unsafe { core.Source(&mut source) };
    // 无论 HRESULT 是否成功都释放 SDK 分配的 PWSTR；回调 PCWSTR 是另一种所有权。
    let text = webview2_com::take_pwstr(source);
    result.map_err(|_| ())?;
    Ok((pid, text))
}

// Rc/RefCell 仅由 WebView2 UI STA 回调使用；不宣称可跨 apartment 的 Agile 对象。
#[windows::core::implement(ICoreWebView2CallDevToolsProtocolMethodCompletedHandler, Agile = false)]
struct Completion {
    session: Rc<Session>,
    method: &'static str,
    invoked: Cell<bool>,
}
impl ICoreWebView2CallDevToolsProtocolMethodCompletedHandler_Impl for Completion_Impl {
    fn Invoke(&self, code: windows::core::HRESULT, value: &PCWSTR) -> windows::core::Result<()> {
        if self.session.capture.borrow().stage == Stage::Done {
            return Ok(());
        }
        if self.invoked.replace(true) {
            self.session
                .finish(false, "callback", Some("duplicate-callback"));
            return Ok(());
        }
        // 迟到或 guard 失效时不复制结果，也不推进下一次调用。
        let result = self.session.healthy().and_then(|()| {
            if code.is_err() {
                Err("sdk-callback-failed")
            } else {
                unsafe { bounded_callback_json(value) }
            }
        });
        self.session.complete(self.method, result);
        Ok(())
    }
}
fn proof_value(
    config: &RuntimeConfig,
    browser_pid: Option<u32>,
    capture: &Capture,
    success: bool,
    stage: &str,
    reason: Option<&str>,
    elapsed_ms: u128,
) -> Value {
    let mut proof = json!({
        "schemaVersion": 1, "scope": "windows-native-sdk-cdp-diagnostic", "mode": "sdk-cdp",
        "performanceSample": false, "root": config.root, "runId": config.run_id,
        "pid": std::process::id(), "port": config.port, "windowLabel": "main",
        "browserPid": browser_pid, "expectedOrigin": EXPECTED_ORIGIN, "success": success,
        "stage": stage, "reason": reason, "calls": capture.calls, "binding": capture.binding,
        "document": capture.document, "observer": capture.observer, "elapsedMs": elapsed_ms as u64,
    });
    if !success && reason == Some("document-ui-root-absent") {
        if let Some(diagnostic) = &capture.ui_root_diagnostic {
            proof["uiRootDiagnostic"] = diagnostic.clone();
        }
    }
    proof
}
fn publish_early_failure(config: &RuntimeConfig, start: Instant, reason: &'static str) {
    publish(
        &config.root,
        &proof_value(
            config,
            None,
            &Capture::default(),
            false,
            "binding",
            Some(reason),
            start.elapsed().as_millis(),
        ),
    );
}
fn bounded_proof(proof: &Value) -> Result<Value, &'static str> {
    if serde_json::to_vec_pretty(proof)
        .map_err(|_| "proof-serialization-failed")?
        .len()
        < MAX_PROOF_BYTES
    {
        return Ok(proof.clone());
    }
    // 不截断成功样本；整个过大 payload 拒绝，保留实际已发出的调用和身份。
    let mut failure = proof.clone();
    failure["success"] = json!(false);
    failure["stage"] = json!("publication");
    failure["reason"] = json!("proof-too-large");
    failure["binding"] = Value::Null;
    failure["document"] = Value::Null;
    failure["observer"] = Value::Null;
    failure.as_object_mut().unwrap().remove("uiRootDiagnostic");
    if serde_json::to_vec_pretty(&failure)
        .map_err(|_| "proof-serialization-failed")?
        .len()
        >= MAX_PROOF_BYTES
    {
        return Err("proof-too-large");
    }
    Ok(failure)
}
fn publish_once(root: &Path, proof: &Value) -> Result<(), &'static str> {
    if !checked_dir(root).is_ok_and(|actual| actual.as_os_str() == root.as_os_str()) {
        return Err("proof-root-mismatch");
    }
    let proof = bounded_proof(proof)?;
    let staged = root.join(STAGED_PROOF_FILE);
    // create_new + sync_all 后 hard_link 排他发布；不让 Node 读到半写 JSON。
    // 失败保留 owned 暂存文件，不删除或覆盖预先存在的路径。
    write_new_json(&staged, &proof).map_err(|_| "proof-staging-failed")?;
    super::require_regular(&staged, false).map_err(|_| "proof-staging-mismatch")?;
    fs::hard_link(&staged, root.join(PROOF_FILE)).map_err(|_| "proof-publication-failed")?;
    fs::remove_file(staged).map_err(|_| "proof-staging-cleanup-failed")?;
    Ok(())
}
fn publish(root: &Path, proof: &Value) {
    if publish_once(root, proof).is_err() {
        eprintln!("native-perf SDK CDP proof publication failed");
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    const SOURCE: &str = "http://tauri.localhost/";
    const RUN_ID: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    fn test_config(root: &Path) -> RuntimeConfig {
        RuntimeConfig {
            root: root.to_path_buf(),
            vault: root.join("vault"),
            identifier: format!("com.solosoul.rf312perf.{RUN_ID}"),
            webview: root.join("webview"),
            port: 44123,
            run_id: RUN_ID.into(),
            chromium_log: None,
            sdk_cdp: true,
        }
    }
    fn frame() -> Value {
        json!({"frameTree": {"frame": {
            "id": "FRAME-1", "loaderId": "LOADER-1", "url": SOURCE,
            "securityOrigin": EXPECTED_ORIGIN
        }}})
    }
    fn evaluation() -> Value {
        json!({"result": {"type": "object", "value": {
            "origin": EXPECTED_ORIGIN, "href": SOURCE, "documentUrl": SOURCE,
            "readyState": "complete", "mainFrame": true, "frameCount": 0,
            "uiRootPresent": true, "timeOriginMs": 1000.0, "atMs": 10.0,
            "observer": {"schemaVersion": 1, "scope": "windows-native-tauri-invoke-observer",
                "runId": RUN_ID, "valid": true, "timeOriginMs": 1000.0,
                "total": 0, "observedCount": 0, "commands": [],
                "installedAtMs": 0.0, "maxEvents": 16384, "invalidReasons": []}
        }}})
    }
    #[test]
    fn sdk_cdp_gates_require_load_and_setup_and_only_start_once() {
        for load_first in [false, true] {
            let mut gate = Gate::default();
            assert!(!gate.signal(load_first, !load_first));
            assert!(!gate.signal(load_first, !load_first));
            assert!(gate.signal(!load_first, load_first));
            assert!(!gate.signal(true, true));
        }
        assert!(Gate::default().signal(true, true));
    }
    #[test]
    fn sdk_cdp_capture_is_ordered_one_shot_and_invalidated_before_progress() {
        let mut capture = Capture::default();
        assert!(!capture.issue(EVALUATE_METHOD));
        assert!(capture.issue(FRAME_METHOD));
        assert!(!capture.issue(FRAME_METHOD));
        assert!(capture.accepts(FRAME_METHOD));
        capture.stage = Stage::FrameComplete;
        assert!(capture.issue(EVALUATE_METHOD));
        assert!(capture.accepts(EVALUATE_METHOD));
        capture.invalidate("navigation-changed");
        capture.invalidate("frame-created");
        assert_eq!(capture.reason, Some("navigation-changed"));
        assert_eq!(capture.navigation_events, 1);
        assert_eq!(capture.frame_created_events, 1);
        assert!(!capture.issue(EVALUATE_METHOD));
        assert!(capture.finish());
        assert!(!capture.finish());
        capture.invalidate("navigation-changed");
        assert_eq!(capture.navigation_events, 1);
        assert_eq!(capture.calls, [FRAME_METHOD, EVALUATE_METHOD]);
    }
    #[test]
    fn sdk_cdp_health_rechecks_guard_and_elapsed_after_possible_com_reentry() {
        let mut capture = Capture::default();
        assert_eq!(capture_health(&capture, MAX_ELAPSED_MS), Ok(()));
        capture.invalidate("process-failed"); // 模拟 SDK getter 重入 guard。
        assert_eq!(
            capture_health(&capture, MAX_ELAPSED_MS),
            Err("process-failed")
        );
        capture = Capture::default();
        assert_eq!(
            capture_health(&capture, MAX_ELAPSED_MS + 1),
            Err("callback-timeout")
        );
        assert!(!capture.issue(EVALUATE_METHOD));
        assert!(capture.finish());
        assert_eq!(capture_health(&capture, 0), Err("capture-finished"));
    }
    #[test]
    fn sdk_cdp_frame_tree_requires_single_bound_main_document() {
        let valid = frame();
        let binding = parse_frame(&valid, SOURCE).unwrap();
        assert_eq!(binding["mainFrameId"], "FRAME-1");
        assert_eq!(binding["source"], SOURCE);
        for (key, replacement) in [
            ("id", json!("")),
            ("loaderId", Value::Null),
            ("parentId", json!("OTHER")),
            ("url", json!("http://tauri.localhost/else")),
            ("securityOrigin", json!("https://external.invalid")),
        ] {
            let mut bad = valid.clone();
            bad["frameTree"]["frame"][key] = replacement;
            assert!(parse_frame(&bad, SOURCE).is_err(), "{key}");
        }
        for children in [Value::Null, json!([{"frame": {"id": "CHILD"}}])] {
            let mut bad = valid.clone();
            bad["frameTree"]["childFrames"] = children;
            assert!(parse_frame(&bad, SOURCE).is_err());
        }
        let mut empty = valid.clone();
        empty["frameTree"]["childFrames"] = json!([]);
        assert!(parse_frame(&empty, SOURCE).is_ok());
        assert!(parse_frame(&json!({"error": {}}), SOURCE).is_err());
        for source in [
            "about:blank",
            "http://tauri.localhost/?secret=x",
            "https://tauri.localhost/",
        ] {
            assert!(!app_source(source));
            assert!(parse_frame(&valid, source).is_err());
        }
    }
    #[test]
    fn sdk_cdp_evaluation_requires_matching_document_and_observer() {
        let valid = evaluation();
        let (document, observer, time_origin) = parse_evaluation(&valid, SOURCE, RUN_ID).unwrap();
        assert_eq!(document.as_object().unwrap().len(), 5);
        assert_eq!(document["uiRootPresent"], true);
        assert_eq!(observer["commands"], json!([]));
        assert_eq!(time_origin, 1000.0);
        for (key, replacement) in [
            ("origin", json!("https://external.invalid")),
            ("href", json!("http://tauri.localhost/else")),
            ("documentUrl", json!("about:blank")),
            ("readyState", json!("loading")),
            ("mainFrame", json!(false)),
            ("frameCount", json!(1)),
            ("uiRootPresent", json!(false)),
            ("timeOriginMs", json!(1001)),
            ("timeOriginMs", json!(0)),
            ("atMs", json!(-1)),
        ] {
            let mut bad = valid.clone();
            bad["result"]["value"][key] = replacement;
            assert!(parse_evaluation(&bad, SOURCE, RUN_ID).is_err(), "{key}");
        }
        for (key, replacement) in [
            ("schemaVersion", json!(2)),
            ("scope", json!("other")),
            ("runId", json!("wrong")),
            ("valid", json!(false)),
            ("timeOriginMs", Value::Null),
        ] {
            let mut bad = valid.clone();
            bad["result"]["value"]["observer"][key] = replacement;
            assert!(parse_evaluation(&bad, SOURCE, RUN_ID).is_err(), "{key}");
        }
        let mut bad = valid.clone();
        bad["exceptionDetails"] = json!({});
        assert!(parse_evaluation(&bad, SOURCE, RUN_ID).is_err());
        bad = valid.clone();
        bad["result"]["type"] = json!("string");
        assert!(parse_evaluation(&bad, SOURCE, RUN_ID).is_err());
        bad = valid.clone();
        bad["result"]["subtype"] = json!("null");
        bad["result"]["value"] = Value::Null;
        assert!(parse_evaluation(&bad, SOURCE, RUN_ID).is_err());
    }
    #[test]
    fn sdk_cdp_document_rejections_are_distinct_fixed_codes_without_rejected_values() {
        let valid = evaluation();
        for (key, rejected, expected) in [
            (
                "origin",
                json!("private-origin-sentinel"),
                "document-origin-mismatch",
            ),
            (
                "href",
                json!("private-href-sentinel"),
                "document-href-mismatch",
            ),
            (
                "documentUrl",
                json!("private-url-sentinel"),
                "document-url-mismatch",
            ),
            (
                "readyState",
                json!("private-state-sentinel"),
                "document-ready-state",
            ),
            ("mainFrame", json!(false), "document-not-main-frame"),
            ("frameCount", json!(1), "document-child-frames"),
            ("uiRootPresent", json!(false), "document-ui-root-absent"),
            (
                "atMs",
                json!("private-clock-sentinel"),
                "document-clock-invalid",
            ),
        ] {
            let mut changed = valid.clone();
            changed["result"]["value"][key] = rejected;
            let reason = parse_evaluation(&changed, SOURCE, RUN_ID).unwrap_err();
            assert_eq!(reason, expected);
            assert!(!reason.contains("private"));
        }
        let mut result_type = valid.clone();
        result_type["result"]["type"] = json!("private-type-sentinel");
        assert_eq!(
            parse_evaluation(&result_type, SOURCE, RUN_ID).unwrap_err(),
            "document-result-type"
        );
        let mut payload_type = valid.clone();
        payload_type["result"]["value"] = json!("private-payload-sentinel");
        assert_eq!(
            parse_evaluation(&payload_type, SOURCE, RUN_ID).unwrap_err(),
            "document-payload-type"
        );
        // 第一个不符项确定，不依赖把被拒绝的值发布给诊断调用方。
        let mut multiple = valid.clone();
        multiple["result"]["value"]["origin"] = json!("private-origin-sentinel");
        multiple["result"]["value"]["uiRootPresent"] = json!(false);
        assert_eq!(
            parse_evaluation(&multiple, SOURCE, RUN_ID).unwrap_err(),
            "document-origin-mismatch"
        );
        assert!(parse_evaluation(&valid, SOURCE, RUN_ID).is_ok());
    }
    #[test]
    fn sdk_cdp_ui_root_diagnostic_is_typed_bounded_and_does_not_accept_failed_document() {
        let mut value = evaluation();
        value["result"]["value"]["uiRootPresent"] = json!(false);
        value["result"]["value"]["uiRootDiagnostic"] = json!({
            "schemaVersion": 1, "rootExists": true, "rootHasChildren": false,
            "reactMountMarked": false, "startupScreenPresent": true, "startupState": "loading",
        });
        let expected = "root-empty-before-react-mount";
        assert_eq!(
            ui_root_diagnostic(&value).unwrap()["classification"],
            expected
        );
        assert_eq!(
            parse_evaluation(&value, SOURCE, RUN_ID).unwrap_err(),
            "document-ui-root-absent"
        );
        for (key, changed, expected) in [
            ("rootExists", json!(false), "root-container-absent"),
            (
                "reactMountMarked",
                json!(true),
                "react-mount-marked-root-empty",
            ),
            (
                "startupState",
                json!("error"),
                "startup-error-before-react-mount",
            ),
        ] {
            let mut variant = value.clone();
            variant["result"]["value"]["uiRootDiagnostic"][key] = changed;
            assert_eq!(
                ui_root_diagnostic(&variant).unwrap()["classification"],
                expected
            );
        }
        for (key, changed) in [
            ("rootExists", json!("private-root-sentinel")),
            ("reactMountMarked", json!(null)),
            ("startupState", json!("private-state-sentinel")),
            ("startupScreenPresent", json!(false)),
            ("rootHasChildren", json!(true)),
            ("schemaVersion", json!(2)),
            ("private", json!("private-extra-sentinel")),
        ] {
            let mut variant = value.clone();
            variant["result"]["value"]["uiRootDiagnostic"][key] = changed;
            assert!(ui_root_diagnostic(&variant).is_none());
        }
        let mut missing = value.clone();
        missing["result"]["value"]["uiRootDiagnostic"]
            .as_object_mut()
            .unwrap()
            .remove("rootExists");
        assert!(ui_root_diagnostic(&missing).is_none());
        value["result"]["value"]["uiRootPresent"] = json!(true);
        assert!(ui_root_diagnostic(&value).is_none());
    }
    #[test]
    fn sdk_cdp_failed_ui_root_probe_never_changes_success_or_other_failure_proofs() {
        let owned = tempfile::tempdir().unwrap();
        let config = test_config(&owned.path().canonicalize().unwrap());
        let capture = Capture {
            ui_root_diagnostic: Some(json!({"classification": "root-container-absent"})),
            ..Default::default()
        };
        for (success, reason, contains) in [
            (false, Some("document-ui-root-absent"), true),
            (false, Some("document-origin-mismatch"), false),
            (true, None, false),
        ] {
            let proof = proof_value(&config, Some(1), &capture, success, "evaluation", reason, 1);
            assert_eq!(proof.get("uiRootDiagnostic").is_some(), contains);
            assert_eq!(proof["success"], success);
            assert!(proof["document"].is_null());
            assert!(proof["observer"].is_null());
        }
    }
    #[test]
    fn sdk_cdp_callback_is_bounded_before_copy_and_strict_utf16_json() {
        let valid: Vec<u16> = r#"{"ok":true}"#.encode_utf16().chain(Some(0)).collect();
        assert_eq!(
            unsafe { bounded_callback_json(&PCWSTR(valid.as_ptr())) }.unwrap(),
            json!({"ok": true})
        );
        assert_eq!(
            unsafe { bounded_callback_json(&PCWSTR::null()) },
            Err("empty-callback")
        );
        let invalid = [0xd800, 0];
        assert_eq!(
            unsafe { bounded_callback_json(&PCWSTR(invalid.as_ptr())) },
            Err("invalid-callback-utf16")
        );
        let not_json = [b'x' as u16, 0];
        assert_eq!(
            unsafe { bounded_callback_json(&PCWSTR(not_json.as_ptr())) },
            Err("invalid-callback-json")
        );
        let mut too_many = vec![b'x' as u16; MAX_RAW_BYTES / 2 + 1];
        too_many.push(0);
        assert_eq!(
            unsafe { bounded_callback_json(&PCWSTR(too_many.as_ptr())) },
            Err("callback-too-large")
        );
        let mut too_many_utf8 = vec![0x0800; MAX_RAW_BYTES / 3 + 1];
        too_many_utf8.push(0);
        assert_eq!(
            unsafe { bounded_callback_json(&PCWSTR(too_many_utf8.as_ptr())) },
            Err("callback-too-large")
        );
    }
    #[test]
    fn sdk_cdp_proof_is_atomic_exclusive_and_oversize_is_failure() {
        let work = tempfile::tempdir().unwrap();
        let root = work.path().canonicalize().unwrap();
        let config = test_config(&root);
        let mut capture = Capture::default();
        assert!(capture.issue(FRAME_METHOD));
        capture.stage = Stage::FrameComplete;
        assert!(capture.issue(EVALUATE_METHOD));
        let proof = proof_value(&config, Some(123), &capture, true, "complete", None, 2);
        assert_eq!(proof["calls"], json!([FRAME_METHOD, EVALUATE_METHOD]));
        assert_eq!(proof["performanceSample"], false);
        publish_once(&root, &proof).unwrap();
        let path = root.join(PROOF_FILE);
        let published = fs::read(&path).unwrap();
        assert_eq!(serde_json::from_slice::<Value>(&published).unwrap(), proof);
        assert!(!root.join(STAGED_PROOF_FILE).exists());
        assert_eq!(publish_once(&root, &proof), Err("proof-publication-failed"));
        assert_eq!(fs::read(path).unwrap(), published);
        let mut oversized = proof.clone();
        oversized["observer"] = json!({"commands": "x".repeat(MAX_PROOF_BYTES)});
        let failure = bounded_proof(&oversized).unwrap();
        assert_eq!(failure["success"], false);
        assert_eq!(failure["reason"], "proof-too-large");
        assert_eq!(failure["calls"], proof["calls"]);
        assert!(failure["observer"].is_null());
        assert!(serde_json::to_vec_pretty(&failure).unwrap().len() < MAX_PROOF_BYTES);
    }
    #[test]
    fn sdk_cdp_request_and_staging_reject_preexisting_paths() {
        let work = tempfile::tempdir().unwrap();
        let root = work.path().canonicalize().unwrap();
        let config = test_config(&root);
        fs::write(root.join(STAGED_PROOF_FILE), b"existing").unwrap();
        let proof = proof_value(
            &config,
            None,
            &Capture::default(),
            false,
            "binding",
            Some("sdk-binding-mismatch"),
            0,
        );
        assert_eq!(publish_once(&root, &proof), Err("proof-staging-failed"));
        assert_eq!(fs::read(root.join(STAGED_PROOF_FILE)).unwrap(), b"existing");
        assert!(!root.join(PROOF_FILE).exists());
        claim_request(&config).unwrap();
        assert!(claim_request(&config).is_err());
        let mut logging = config.clone();
        logging.chromium_log = Some(root.join("temp/chromium-diagnostics.log"));
        assert!(claim_request(&logging).is_err());
        let other = tempfile::tempdir().unwrap();
        let other_root = other.path().canonicalize().unwrap();
        fs::create_dir(other_root.join(PROOF_FILE)).unwrap();
        assert!(claim_request(&test_config(&other_root)).is_err());
    }
}
