//! 固定公开PDF的related target/session诊断；只记录分类，不保存URL、标题或文档文本。
use super::super::super::sdk_cdp::bounded_callback_json;
use super::super::{Capture, Outcome};
use super::{candidate, identifier, structure, url_class, APP, VIEWER};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{atomic::Ordering, Arc, Mutex};
use std::time::Duration;
use tokio::sync::oneshot;
use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2DevToolsProtocolEventReceivedEventArgs2,
    ICoreWebView2DevToolsProtocolEventReceiver,
};
use webview2_com::{CoTaskMemPWSTR, DevToolsProtocolEventReceivedEventHandler};
use windows::core::{Interface, PCWSTR, PWSTR};
#[path = "sdk_pdf_component.rs"]
mod component;
thread_local! { static WATCH: RefCell<Vec<(ICoreWebView2DevToolsProtocolEventReceiver,i64)>> = const { RefCell::new(Vec::new()) }; }
#[derive(Clone)]
struct Target {
    proof: Value,
    url: String,
}
#[derive(Clone)]
enum Event {
    Attached(String, String, Target),
    Changed(String, Target),
    Detached(String, String),
}
#[derive(Clone)]
struct Session {
    id: String,
    target: Target,
    active: bool,
    enabled: bool,
}
pub(super) struct Broker {
    events: Arc<Mutex<Vec<Event>>>,
    sessions: BTreeMap<String, Session>,
    main: Option<Value>,
    calls: Vec<Value>,
    installed: bool,
    auto: bool,
    component: Option<component::Attachment>,
}
fn info(v: &Value, pdf: &str) -> Outcome<Target> {
    let url = v["url"].as_str().ok_or("pdf-target-info-invalid")?;
    if !identifier(&v["targetId"])
        || url.len() > 4096
        || !v["attached"].is_boolean()
        || !v["type"].is_string()
    {
        return Err("pdf-target-info-invalid");
    }
    for key in ["parentId", "openerId", "parentFrameId", "browserContextId"] {
        if v.get(key).is_some_and(|x| !x.is_null() && !identifier(x)) {
            return Err("pdf-target-info-invalid");
        }
    }
    let kind = match v["type"].as_str().unwrap() {
        "page" => "page",
        "iframe" => "iframe",
        "webview" => "webview",
        _ => "other",
    };
    Ok(Target {
        url: url.to_owned(),
        proof: json!({"id":v["targetId"],"type":kind,"urlClass":url_class(url,pdf),"attached":v["attached"],"parentId":v.get("parentId").unwrap_or(&Value::Null),"openerId":v.get("openerId").unwrap_or(&Value::Null),"parentFrameId":v.get("parentFrameId").unwrap_or(&Value::Null),"browserContextId":v.get("browserContextId").unwrap_or(&Value::Null)}),
    })
}
fn parse_event(kind: &str, source: String, v: &Value, pdf: &str) -> Outcome<Event> {
    if !source.is_empty() && !identifier(&json!(source)) {
        return Err("pdf-target-event-invalid");
    }
    match kind {
        "Target.attachedToTarget"
            if identifier(&v["sessionId"]) && v["waitingForDebugger"] == false =>
        {
            Ok(Event::Attached(
                source,
                v["sessionId"].as_str().unwrap().to_owned(),
                info(&v["targetInfo"], pdf)?,
            ))
        }
        "Target.targetInfoChanged" => Ok(Event::Changed(source, info(&v["targetInfo"], pdf)?)),
        "Target.detachedFromTarget" if identifier(&v["sessionId"]) => Ok(Event::Detached(
            source,
            v["sessionId"].as_str().unwrap().to_owned(),
        )),
        _ => Err("pdf-target-event-invalid"),
    }
}
// SDK分配、NUL终止字符串；逐字符有界读取后交由RAII释放。
pub(super) unsafe fn session_id(raw: PWSTR) -> Outcome<String> {
    if raw.is_null() {
        return Ok(String::new());
    }
    let mut bytes = Vec::new();
    for offset in 0..=128 {
        let ch = unsafe { *raw.0.add(offset) };
        if ch == 0 {
            return String::from_utf16(&bytes).map_err(|_| "pdf-target-event-invalid");
        }
        if offset == 128
            || ch > 127
            || !(u8::try_from(ch).unwrap().is_ascii_alphanumeric() || b"_-.:".contains(&(ch as u8)))
        {
            return Err("pdf-target-event-invalid");
        }
        bytes.push(ch);
    }
    Err("pdf-target-event-invalid")
}
impl Broker {
    pub(super) fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            sessions: BTreeMap::new(),
            main: None,
            calls: Vec::new(),
            installed: false,
            auto: false,
            component: None,
        }
    }
    pub(super) async fn start(&mut self, c: &mut Capture, pdf: &str) -> Outcome<()> {
        let raw = c.protocol("Target.getTargetInfo", json!({})).await?;
        let main = info(&raw["targetInfo"], pdf)?;
        if main.url != APP || main.proof["type"] != "page" {
            return Err("pdf-main-target-mismatch");
        }
        self.main = Some(main.proof.clone());
        c.pdf_diagnostic.as_mut().unwrap()["targetDiagnostic"]["mainTarget"] = main.proof;
        let events = self.events.clone();
        let invalid = c.invalidated.clone();
        let pdf = pdf.to_owned();
        let (send, recv) = oneshot::channel();
        c.window.with_webview(move |platform| {
            let result=(||->Outcome<()> {
                if WATCH.with(|x|!x.borrow().is_empty()){return Err("pdf-target-watch-already-installed");}
                let core=unsafe{platform.controller().CoreWebView2()}.map_err(|_|"pdf-target-watch-unavailable")?;
                for kind in ["Target.attachedToTarget","Target.targetInfoChanged","Target.detachedFromTarget"] {
                    let name:Vec<_>=kind.encode_utf16().chain(Some(0)).collect();
                    let receiver=unsafe{core.GetDevToolsProtocolEventReceiver(PCWSTR(name.as_ptr()))}.map_err(|_|"pdf-target-watch-unavailable")?;
                    let events=events.clone();let invalid=invalid.clone();let pdf=pdf.clone();
                    let handler=DevToolsProtocolEventReceivedEventHandler::create(Box::new(move |_,args| {
                        let event=(||->Outcome<Event>{
                            let args=args.ok_or("pdf-target-event-invalid")?;let args2=args.cast::<ICoreWebView2DevToolsProtocolEventReceivedEventArgs2>().map_err(|_|"pdf-target-session-event-unavailable")?;
                            let mut source=PWSTR::null();let hr=unsafe{args2.SessionId(&mut source)};let owned=CoTaskMemPWSTR::from(source);hr.map_err(|_|"pdf-target-event-invalid")?;let source=unsafe{session_id(source)}?;drop(owned);
                            let mut raw=PWSTR::null();let hr=unsafe{args.ParameterObjectAsJson(&mut raw)};let owned=CoTaskMemPWSTR::from(raw);hr.map_err(|_|"pdf-target-event-invalid")?;let value=unsafe{bounded_callback_json(&PCWSTR(raw.0))}?;drop(owned);
                            parse_event(kind,source,&value,&pdf)
                        })();
                        match (event,events.lock()) {(Ok(event),Ok(mut list)) if list.len()<64=>list.push(event),_=>invalid.store(true,Ordering::SeqCst)}
                        Ok(())
                    }));
                    let mut token=0;unsafe{receiver.add_DevToolsProtocolEventReceived(&handler,&mut token)}.map_err(|_|"pdf-target-watch-unavailable")?;
                    WATCH.with(|x|x.borrow_mut().push((receiver,token)));
                }
                Ok(())
            })();let _=send.send(result);
        }).map_err(|_|"pdf-target-watch-unavailable")?;
        // 若安装到一半失败，finish仍负责释放已保存的订阅。
        self.installed = true;
        tokio::time::timeout(Duration::from_secs(10), recv)
            .await
            .map_err(|_| "pdf-target-watch-timeout")?
            .map_err(|_| "pdf-target-watch-unavailable")??;
        self.auto = true;
        c.protocol("Target.setAutoAttach",json!({"autoAttach":true,"waitForDebuggerOnStart":false,"flatten":true,"filter":[{"type":"iframe"},{"type":"page"},{"type":"webview"},{"exclude":true}]})).await?;
        Ok(())
    }
    fn sync(&mut self) -> Outcome<usize> {
        let events = self
            .events
            .lock()
            .map_err(|_| "pdf-target-event-invalid")?
            .clone();
        let mut sessions = BTreeMap::<String, Session>::new();
        for event in &events {
            match event {
                Event::Attached(source, id, target) if source.is_empty() => {
                    if sessions.len() >= 4
                        || sessions.contains_key(id)
                        || sessions
                            .values()
                            .any(|x| x.target.proof["id"] == target.proof["id"])
                    {
                        return Err("pdf-related-target-over-budget");
                    }
                    let enabled = self.sessions.get(id).is_some_and(|x| x.enabled);
                    sessions.insert(
                        id.clone(),
                        Session {
                            id: id.clone(),
                            target: target.clone(),
                            active: true,
                            enabled,
                        },
                    );
                }
                Event::Changed(source, target) if source.is_empty() => {
                    for value in sessions.values_mut() {
                        if value.target.proof["id"] == target.proof["id"] {
                            value.target = target.clone();
                        }
                    }
                }
                Event::Detached(source, id) if source.is_empty() => {
                    if let Some(value) = sessions.get_mut(id) {
                        value.active = false;
                    }
                }
                _ => {}
            }
        }
        // 明确解除命令的成功回复也是session结束依据，不假称已收到回调。
        if let Some(attachment) = &self.component {
            if attachment.detached() {
                if let Some(row) = sessions.get_mut(attachment.session()) {
                    row.active = false;
                }
            }
        }
        self.sessions = sessions;
        Ok(events.len())
    }
    async fn call(
        &mut self,
        c: &mut Capture,
        id: &str,
        method: &'static str,
        args: Value,
    ) -> Outcome<Value> {
        if self.sessions.get(id).is_none_or(|x| !x.active) {
            return Err("pdf-session-detached");
        }
        self.calls.push(json!({"sessionId":id,"method":method}));
        c.pdf_session(id, method, args).await
    }
    pub(super) async fn sample(&mut self, c: &mut Capture, pdf: &str, index: usize) -> Outcome<()> {
        let main = c.protocol("Target.getTargetInfo", json!({})).await?;
        let main = info(&main["targetInfo"], pdf)?;
        if main.url != APP
            || self
                .main
                .as_ref()
                .is_none_or(|x| x["id"] != main.proof["id"])
        {
            return Err("pdf-main-target-replaced");
        }
        let raw = c.protocol("Target.getTargets", json!({})).await?;
        let list = raw["targetInfos"]
            .as_array()
            .ok_or("pdf-target-list-invalid")?;
        if list.len() > 8 {
            return Err("pdf-target-list-over-budget");
        }
        let mut targets = Vec::new();
        let mut seen = BTreeSet::new();
        for value in list {
            let value = info(value, pdf)?;
            if !seen.insert(value.proof["id"].as_str().unwrap().to_owned()) {
                return Err("pdf-target-list-invalid");
            }
            targets.push(value.proof);
        }
        if !targets
            .iter()
            .any(|x| x["id"] == main.proof["id"] && x["urlClass"] == "application")
        {
            return Err("pdf-main-target-unlisted");
        }
        self.sync()?;
        self.attach_component(c, pdf, &main, list, &targets, index)
            .await?;
        let count = self.sync()?;
        c.pdf_diagnostic.as_mut().unwrap()["targetDiagnostic"]["snapshots"]
            .as_array_mut()
            .unwrap()
            .push(json!({"snapshotIndex":index,"mainVerified":true,"targets":targets}));
        let mut selected: Vec<_> = self
            .sessions
            .values()
            .filter(|x| {
                x.active
                    && ["page", "iframe", "webview"]
                        .contains(&x.target.proof["type"].as_str().unwrap_or(""))
                    && ["owned-pdf", "component-extension"]
                        .contains(&x.target.proof["urlClass"].as_str().unwrap_or(""))
            })
            .cloned()
            .collect();
        selected.sort_by_key(|x| self.component.as_ref().is_none_or(|a| a.session() != x.id));
        selected.truncate(2);
        for session in selected {
            if !targets.iter().any(|t| {
                t["id"] == session.target.proof["id"]
                    && t["urlClass"] == session.target.proof["urlClass"]
                    && t["type"] == session.target.proof["type"]
            }) {
                return Err("pdf-session-target-unlisted");
            }
            if !session.enabled {
                self.call(c, &session.id, "Runtime.enable", json!({}))
                    .await?;
                self.sessions.get_mut(&session.id).unwrap().enabled = true;
            }
            let current = self
                .call(c, &session.id, "Target.getTargetInfo", json!({}))
                .await?;
            let current = info(&current["targetInfo"], pdf)?;
            if current.proof["id"] != session.target.proof["id"]
                || current.url != session.target.url
            {
                return Err("pdf-session-target-replaced");
            }
            let before = self
                .call(c, &session.id, "Page.getFrameTree", json!({}))
                .await?;
            let frame = session_frame(&before, &current.url, pdf)?;
            let expression = VIEWER.replace(
                "__REQUEST__",
                &json!({"expectedUrl":current.url,"expectedPdfUrl":pdf}).to_string(),
            );
            let raw = self
                .call(
                    c,
                    &session.id,
                    "Runtime.evaluate",
                    json!({"expression":expression,"returnByValue":true,"awaitPromise":true}),
                )
                .await?;
            let state = candidate(&raw)?;
            let after = self
                .call(c, &session.id, "Page.getFrameTree", json!({}))
                .await?;
            if session_frame(&after, &current.url, pdf)? != frame {
                return Err("pdf-session-document-replaced");
            }
            let trees = json!({"before":structure::frames(&before,&frame,pdf)?,"after":structure::frames(&after,&frame,pdf)?});
            c.pdf_diagnostic.as_mut().unwrap()["targetDiagnostic"]["candidates"].as_array_mut().unwrap().push(json!({"snapshotIndex":index,"sessionId":session.id,"targetId":current.proof["id"],"frame":frame,"state":state,"observedAtMs":c.started.elapsed().as_secs_f64()*1000.0,"frameTrees":trees}));
        }
        self.publish(c, count);
        Ok(())
    }
    fn publish(&self, c: &mut Capture, count: usize) {
        let related:Vec<_>=self.sessions.values().map(|x|json!({"sessionId":x.id,"parentSessionId":"","target":x.target.proof,"active":x.active,"enabled":x.enabled})).collect();
        let d = &mut c.pdf_diagnostic.as_mut().unwrap()["targetDiagnostic"];
        d["relatedSessions"] = json!(related);
        d["sessionCalls"] = json!(self.calls);
        d["eventCount"] = json!(count);
        d["componentAttachment"] = self.component.as_ref().map_or(Value::Null, |x| x.proof());
    }
    pub(super) async fn finish(&mut self, c: &mut Capture) -> Outcome<()> {
        let component_outcome = self.detach_component(c).await;
        let mut outcome = Ok(());
        if self.auto {
            outcome = c
                .protocol(
                    "Target.setAutoAttach",
                    json!({"autoAttach":false,"waitForDebuggerOnStart":false,"flatten":true}),
                )
                .await
                .map(|_| ());
            c.pdf_diagnostic.as_mut().unwrap()["targetDiagnostic"]["autoAttachCleaned"] =
                json!(outcome.is_ok());
        }
        if self.installed {
            let (send, recv) = oneshot::channel();
            c.window
                .with_webview(move |_| {
                    let result = WATCH.with(|x| {
                        let mut failed = false;
                        for (receiver, token) in x.borrow_mut().drain(..) {
                            if unsafe { receiver.remove_DevToolsProtocolEventReceived(token) }
                                .is_err()
                            {
                                failed = true;
                            }
                        }
                        if failed {
                            Err("pdf-target-watch-cleanup-failed")
                        } else {
                            Ok(())
                        }
                    });
                    let _ = send.send(result);
                })
                .map_err(|_| "pdf-target-watch-cleanup-failed")?;
            let cleanup = tokio::time::timeout(Duration::from_secs(10), recv)
                .await
                .map_err(|_| "pdf-target-watch-cleanup-failed")?
                .map_err(|_| "pdf-target-watch-cleanup-failed")?;
            c.pdf_diagnostic.as_mut().unwrap()["targetDiagnostic"]["watcherCleaned"] =
                json!(cleanup.is_ok());
            cleanup?;
        }
        let count = self.sync()?;
        self.publish(c, count);
        component_outcome?;
        if self.component.is_none() {
            return Err("pdf-component-not-observed");
        }
        outcome
    }
}
fn session_frame(raw: &Value, url: &str, pdf: &str) -> Outcome<Value> {
    let f = &raw["frameTree"]["frame"];
    if !identifier(&f["id"]) || !identifier(&f["loaderId"]) || f["url"] != url {
        return Err("pdf-session-frame-invalid");
    }
    Ok(json!({"id":f["id"],"loaderId":f["loaderId"],"urlClass":url_class(url,pdf)}))
}
pub(super) fn empty() -> Value {
    json!({"schemaVersion":3,"scope":"windows-native-sdk-pdf-related-targets","mainTarget":null,"snapshots":[],"relatedSessions":[],"candidates":[],"sessionCalls":[],"eventCount":0,"autoAttachCleaned":false,"watcherCleaned":false,"componentAttachment":null})
}
#[cfg(test)]
mod tests {
    use super::*;
    fn raw() -> Value {
        json!({"targetId":"T","type":"iframe","title":"discarded","url":"http://solosoul-pdf.localhost/PDF","attached":true})
    }
    #[test]
    fn target_metadata_redacts_payload_and_rejects_invalid_identifiers() {
        let t = info(&raw(), "http://solosoul-pdf.localhost/PDF").unwrap();
        assert_eq!(t.proof["urlClass"], "owned-pdf");
        assert!(t.proof.get("url").is_none());
        assert!(t.proof.get("title").is_none());
        for key in ["targetId", "parentId", "browserContextId"] {
            let mut v = raw();
            v[key] = json!("BAD SPACE");
            assert!(info(&v, "PDF").is_err());
        }
    }
    #[test]
    fn only_direct_related_sessions_can_become_probe_candidates() {
        let mut b = Broker::new();
        let t = info(&raw(), "http://solosoul-pdf.localhost/PDF").unwrap();
        b.events.lock().unwrap().extend([
            Event::Attached("FOREIGN".into(), "NO".into(), t.clone()),
            Event::Attached("".into(), "YES".into(), t),
        ]);
        b.sync().unwrap();
        assert_eq!(b.sessions.len(), 1);
        assert!(b.sessions.contains_key("YES"));
        b.events
            .lock()
            .unwrap()
            .push(Event::Detached("".into(), "YES".into()));
        b.sync().unwrap();
        assert!(!b.sessions["YES"].active);
    }
    #[test]
    fn repeated_or_changed_session_identity_is_rejected() {
        let mut b = Broker::new();
        let t = info(&raw(), "PDF").unwrap();
        b.events.lock().unwrap().extend([
            Event::Attached("".into(), "S".into(), t.clone()),
            Event::Attached("".into(), "S".into(), t),
        ]);
        assert!(b.sync().is_err());
    }
    #[test]
    fn session_frame_rejects_document_or_loader_loss() {
        let v = json!({"frameTree":{"frame":{"id":"F","loaderId":"L","url":"PDF"}}});
        assert!(session_frame(&v, "PDF", "PDF").is_ok());
        assert!(session_frame(&v, "OTHER", "PDF").is_err());
        let mut v = v;
        v["frameTree"]["frame"]["loaderId"] = Value::Null;
        assert!(session_frame(&v, "PDF", "PDF").is_err());
    }
}
