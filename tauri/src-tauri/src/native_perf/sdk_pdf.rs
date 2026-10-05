//! 固定公开PDF的能力诊断。数据完整不等于PDF渲染或性能验收。
use super::super::sdk_cdp::bounded_callback_json;
use super::{parse_frame, Capture, Outcome};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::io::Write;
use std::sync::{atomic::Ordering, Arc, Mutex};
use std::time::Duration;
use tokio::sync::oneshot;
use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2DevToolsProtocolEventReceivedEventArgs2,
    ICoreWebView2DevToolsProtocolEventReceiver,
};
use webview2_com::{CoTaskMemPWSTR, DevToolsProtocolEventReceivedEventHandler};
use windows::core::{Interface, PCWSTR, PWSTR};
#[path = "sdk_pdf_structure.rs"]
mod structure;
#[path = "sdk_pdf_targets.rs"]
mod targets;
const DOM: &str = include_str!("sdk_pdf_dom.js");
const VIEWER: &str = include_str!("sdk_pdf_viewer.js");
const APP: &str = "http://tauri.localhost/settings/attachments";
thread_local! {
    static WATCH: RefCell<Option<(ICoreWebView2DevToolsProtocolEventReceiver, i64)>> = const { RefCell::new(None) };
}
fn identifier(v: &Value) -> bool {
    v.as_str().is_some_and(|s| {
        !s.is_empty()
            && s.len() <= 128
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-.:".contains(&b))
    })
}
fn encoded_component(s: &str) -> String {
    let mut result = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
            result.push(char::from(b));
        } else {
            result.push_str(&format!("%{b:02X}"));
        }
    }
    result
}
fn url_class(s: &str, pdf: &str) -> &'static str {
    if s == APP {
        "application"
    } else if s == pdf {
        "owned-pdf"
    } else if s == "about:blank" {
        "blank"
    } else if url::Url::parse(s).is_ok_and(|u| {
        u.scheme() == "chrome-extension"
            && u.host_str()
                .is_some_and(|h| h.len() == 32 && h.bytes().all(|b| (b'a'..=b'p').contains(&b)))
            && u.username().is_empty()
            && u.password().is_none()
    }) {
        "component-extension"
    } else {
        "other"
    }
}
fn origin_class(s: &str) -> &'static str {
    if s == "http://tauri.localhost" {
        "application"
    } else if s == "http://solosoul-pdf.localhost" {
        "owned-pdf"
    } else if url::Url::parse(s).is_ok_and(|u| {
        u.scheme() == "chrome-extension"
            && u.host_str()
                .is_some_and(|h| h.len() == 32 && h.bytes().all(|b| (b'a'..=b'p').contains(&b)))
    }) {
        "component-extension"
    } else {
        "other"
    }
}
fn context_event(v: &Value) -> Outcome<Value> {
    let c = &v["context"];
    if c["id"]
        .as_u64()
        .is_none_or(|id| id == 0 || id > i32::MAX as u64)
        || !identifier(&c["uniqueId"])
        || !identifier(&c["auxData"]["frameId"])
        || !c["auxData"]["isDefault"].is_boolean()
        || !c["origin"].is_string()
    {
        return Err("pdf-context-event-invalid");
    }
    Ok(
        json!({"id":c["id"],"uniqueId":c["uniqueId"],"frameId":c["auxData"]["frameId"],"isDefault":c["auxData"]["isDefault"],"originClass":origin_class(c["origin"].as_str().unwrap())}),
    )
}
async fn watch(c: &Capture, events: Arc<Mutex<Vec<Value>>>) -> Outcome<()> {
    let invalid = c.invalidated.clone();
    let (send, recv) = oneshot::channel();
    c.window
        .with_webview(move |platform| {
            let result = (|| -> Outcome<()> {
                if WATCH.with(|s| s.borrow().is_some()) {
                    return Err("pdf-context-watch-already-installed");
                }
                let core = unsafe { platform.controller().CoreWebView2() }
                    .map_err(|_| "pdf-context-watch-unavailable")?;
                let name: Vec<_> = "Runtime.executionContextCreated"
                    .encode_utf16()
                    .chain(Some(0))
                    .collect();
                let receiver =
                    unsafe { core.GetDevToolsProtocolEventReceiver(PCWSTR(name.as_ptr())) }
                        .map_err(|_| "pdf-context-watch-unavailable")?;
                let handler =
                    DevToolsProtocolEventReceivedEventHandler::create(Box::new(move |_, args| {
                        let parsed = (|| -> Outcome<Option<Value>> {
                            let args = args.ok_or("pdf-context-event-invalid")?;
                            let args2 = args
                                .cast::<ICoreWebView2DevToolsProtocolEventReceivedEventArgs2>()
                                .map_err(|_| "pdf-context-session-unavailable")?;
                            let mut session = PWSTR::null();
                            let hr = unsafe { args2.SessionId(&mut session) };
                            let owned = CoTaskMemPWSTR::from(session);
                            hr.map_err(|_| "pdf-context-event-invalid")?;
                            let session = unsafe { targets::session_id(session) }?;
                            drop(owned);
                            // 该订阅只归档主session上下文；子session由broker独立绑定。
                            if !session.is_empty() {
                                return Ok(None);
                            }
                            let mut raw = PWSTR::null();
                            let hr = unsafe { args.ParameterObjectAsJson(&mut raw) };
                            // SDK分配的字符串由RAII释放；先限制长度，再复制JSON。
                            let owned = CoTaskMemPWSTR::from(raw);
                            hr.map_err(|_| "pdf-context-event-invalid")?;
                            let value = unsafe { bounded_callback_json(&PCWSTR(raw.0)) }?;
                            drop(owned);
                            context_event(&value).map(Some)
                        })();
                        match (parsed, events.lock()) {
                            (Ok(Some(value)), Ok(mut list)) if list.len() < 64 => list.push(value),
                            (Ok(None), _) => {}
                            _ => invalid.store(true, Ordering::SeqCst),
                        }
                        Ok(())
                    }));
                let mut token = 0;
                unsafe { receiver.add_DevToolsProtocolEventReceived(&handler, &mut token) }
                    .map_err(|_| "pdf-context-watch-unavailable")?;
                WATCH.with(|s| *s.borrow_mut() = Some((receiver, token)));
                Ok(())
            })();
            let _ = send.send(result);
        })
        .map_err(|_| "pdf-context-watch-unavailable")?;
    tokio::time::timeout(Duration::from_secs(10), recv)
        .await
        .map_err(|_| "pdf-context-watch-timeout")?
        .map_err(|_| "pdf-context-watch-unavailable")?
}
async fn unwatch(c: &Capture) -> Outcome<()> {
    let (send, recv) = oneshot::channel();
    c.window
        .with_webview(move |_| {
            let result = WATCH.with(|s| {
                s.borrow_mut().take().map_or(Ok(()), |(receiver, token)| {
                    unsafe { receiver.remove_DevToolsProtocolEventReceived(token) }
                        .map_err(|_| "pdf-context-watch-cleanup-failed")
                })
            });
            let _ = send.send(result);
        })
        .map_err(|_| "pdf-context-watch-cleanup-failed")?;
    tokio::time::timeout(Duration::from_secs(10), recv)
        .await
        .map_err(|_| "pdf-context-watch-cleanup-failed")?
        .map_err(|_| "pdf-context-watch-cleanup-failed")?
}
// 仅保持同一主文档后记录有界树；陌生来源明确分类，不接受为渲染证据。
fn topology(raw: &Value, expected: &Value, pdf: &str) -> Outcome<Value> {
    let tree = &raw["frameTree"];
    let root = &tree["frame"];
    if root["id"] != expected["mainFrameId"]
        || root["loaderId"] != expected["loaderId"]
        || root["url"] != APP
        || root.get("parentId").is_some()
        || root["securityOrigin"] != "http://tauri.localhost"
    {
        return Err("pdf-main-frame-replaced");
    }
    let mut seen = BTreeSet::new();
    seen.insert(root["id"].as_str().ok_or("pdf-frame-invalid")?.to_owned());
    let mut frames = vec![];
    fn visit(
        node: &Value,
        parent: &Value,
        pdf: &str,
        depth: usize,
        seen: &mut BTreeSet<String>,
        out: &mut Vec<Value>,
    ) -> Outcome<()> {
        if depth > 4 || out.len() >= 6 {
            return Err("pdf-frame-tree-over-budget");
        }
        let f = &node["frame"];
        if !identifier(&f["id"])
            || !identifier(&f["loaderId"])
            || f["parentId"] != *parent
            || !f["url"].is_string()
            || !seen.insert(f["id"].as_str().unwrap().to_owned())
        {
            return Err("pdf-frame-invalid");
        }
        out.push(json!({"id":f["id"],"parentId":f["parentId"],"loaderId":f["loaderId"],"urlClass":url_class(f["url"].as_str().unwrap(),pdf),"originClass":origin_class(f["securityOrigin"].as_str().unwrap_or(""))}));
        if let Some(children) = node.get("childFrames") {
            for child in children.as_array().ok_or("pdf-frame-invalid")? {
                visit(child, &f["id"], pdf, depth + 1, seen, out)?;
            }
        }
        Ok(())
    }
    if let Some(children) = tree.get("childFrames") {
        for child in children.as_array().ok_or("pdf-frame-invalid")? {
            visit(child, &root["id"], pdf, 1, &mut seen, &mut frames)?;
        }
    }
    Ok(json!({"mainVerified":true,"frames":frames}))
}
fn dom_value(raw: &Value, phase: &str) -> Outcome<Value> {
    let v = &raw["result"]["value"];
    if raw.get("exceptionDetails").is_some()
        || v.as_object().map_or(0, |v| v.len()) != 10
        || v["schemaVersion"] != 1
        || v["scope"] != "windows-native-sdk-pdf-dom"
        || v["phase"] != phase
        || v["outcome"] != "ready"
        || v["mainVerified"] != true
        || !v["embedded"].is_boolean()
        || v["frameElementCount"].as_u64().is_none_or(|n| n > 4)
        || v["atMs"].as_f64().is_none_or(|n| !n.is_finite() || n < 0.0)
        || v["inputTrust"]["untrusted"] != 0
        || ["pointer", "text"]
            .iter()
            .any(|k| v["inputTrust"][k].as_u64().is_none_or(|n| n > 128))
        || v["inputTrust"].as_object().map_or(0, |v| v.len()) != 3
    {
        return Err("pdf-dom-proof-invalid");
    }
    if phase == "opened" && v["embedded"] != true
        || phase != "opened" && (v["embedded"] != false || v["frameElementCount"] != 0)
    {
        return Err("pdf-dom-proof-invalid");
    }
    if phase == "closed" {
        if !v["target"].is_null() {
            return Err("pdf-dom-proof-invalid");
        }
    } else {
        let p = &v["target"];
        if p.as_object().map_or(0, |v| v.len()) != 3
            || p["actionable"] != true
            || ["x", "y"].iter().any(|k| {
                p[k].as_f64()
                    .is_none_or(|n| !n.is_finite() || !(0.0..=16384.0).contains(&n))
            })
        {
            return Err("pdf-target-not-actionable");
        }
    }
    Ok(v.clone())
}
async fn dom(c: &mut Capture, phase: &str, pdf: &str) -> Outcome<Value> {
    let request =
        json!({"phase":phase,"runId":c.config.run_id,"timeOriginMs":c.time_origin,"pdfUrl":pdf});
    let raw=c.protocol("Runtime.evaluate",json!({"expression":DOM.replace("__REQUEST__",&request.to_string()),"returnByValue":true,"awaitPromise":true})).await?;
    dom_value(&raw, phase)
}
fn candidate(raw: &Value) -> Outcome<Value> {
    let v = &raw["result"]["value"];
    if raw.get("exceptionDetails").is_some()
        || v.as_object().map_or(0, |v| v.len()) != 11
        || v["schemaVersion"] != 2
        || v["scope"] != "windows-native-sdk-pdf-viewer-candidate"
        || v["documentMatches"] != true
        || [
            "viewerPresent",
            "loadSucceededMethodPresent",
            "documentDimensionsPresent",
        ]
        .iter()
        .any(|k| !v[k].is_boolean())
        || !(v["loadSucceeded"].is_null() || v["loadSucceeded"].is_boolean())
        || !(v["pageCount"].is_null() || v["pageCount"].as_u64().is_some_and(|n| n <= 100))
        || !matches!(v["paintedFrames"].as_u64(), Some(0 | 2))
        || v["timeOriginMs"]
            .as_f64()
            .is_none_or(|n| !n.is_finite() || n <= 0.0)
        || (v["loadSucceededMethodPresent"] == false && !v["loadSucceeded"].is_null())
        || (v["documentDimensionsPresent"] == true) == v["pageCount"].is_null()
        || (v["paintedFrames"] == 2 && v["loadSucceeded"] != true)
        || !structure::valid(&v["structure"])
    {
        return Err("pdf-renderer-candidate-invalid");
    }
    Ok(v.clone())
}
async fn inspect(
    c: &mut Capture,
    events: &Arc<Mutex<Vec<Value>>>,
    pdf: &str,
    broker: &mut targets::Broker,
) -> Outcome<()> {
    let target = dom(c, "target", pdf).await?;
    let tree = c.protocol("Page.getFrameTree", json!({})).await?;
    parse_frame(&tree, APP)?;
    let expected = c.frame.clone().ok_or("pdf-binding-unavailable")?;
    topology(&tree, &expected, pdf)?;
    c.pdf_frame_window.store(true, Ordering::SeqCst);
    broker.start(c, pdf).await?;
    c.pdf_diagnostic.as_mut().unwrap()["openingStartedAtMs"] =
        json!(c.started.elapsed().as_secs_f64() * 1000.0);
    for kind in ["mousePressed", "mouseReleased"] {
        c.protocol("Input.dispatchMouseEvent",json!({"type":kind,"x":target["target"]["x"],"y":target["target"]["y"],"button":"left","clickCount":1})).await?;
    }
    c.step = "pdf-capability";
    let opened = dom(c, "opened", pdf).await?;
    c.pdf_diagnostic.as_mut().unwrap()["openedDom"] = opened;
    for (snapshot_index, delay) in [0, 250, 750, 1750].into_iter().enumerate() {
        tokio::time::sleep(Duration::from_millis(delay)).await;
        dom(c, "opened", pdf).await?;
        let raw = c.protocol("Page.getFrameTree", json!({})).await?;
        let frames = topology(&raw, &expected, pdf)?;
        c.pdf_diagnostic.as_mut().unwrap()["frameSnapshots"]
            .as_array_mut()
            .unwrap()
            .push(frames.clone());
        let contexts = events
            .lock()
            .map_err(|_| "pdf-context-event-invalid")?
            .clone();
        c.pdf_diagnostic.as_mut().unwrap()["contexts"] = json!(contexts);
        // 同一context在后续快照中可能才加载完成，逐快照重新观察。
        let mut evaluated = BTreeSet::new();
        for context in &contexts {
            let id = context["frameId"]
                .as_str()
                .ok_or("pdf-context-event-invalid")?;
            let eligible = frames["frames"].as_array().unwrap().iter().any(|f| {
                f["id"] == id
                    && ["owned-pdf", "component-extension"]
                        .contains(&f["urlClass"].as_str().unwrap_or(""))
                    && context["originClass"] == f["urlClass"]
            });
            if !eligible
                || context["isDefault"] != true
                || !evaluated.insert(context["uniqueId"].as_str().unwrap().to_owned())
            {
                continue;
            }
            if evaluated.len() > 4 {
                return Err("pdf-renderer-candidate-over-budget");
            }
            fn find<'a>(n: &'a Value, id: &str) -> Option<&'a Value> {
                if n["frame"]["id"] == id {
                    return Some(&n["frame"]);
                }
                n.get("childFrames")
                    .and_then(Value::as_array)
                    .and_then(|children| children.iter().find_map(|v| find(v, id)))
            }
            let url = find(&raw["frameTree"], id).ok_or("pdf-context-frame-unbound")?["url"]
                .as_str()
                .ok_or("pdf-frame-invalid")?;
            let expression = VIEWER.replace(
                "__REQUEST__",
                &json!({"expectedUrl":url,"expectedPdfUrl":pdf}).to_string(),
            );
            let result=c.protocol("Runtime.evaluate",json!({"expression":expression,"uniqueContextId":context["uniqueId"],"returnByValue":true,"awaitPromise":true})).await?;
            let value = candidate(&result)?;
            let after = c.protocol("Page.getFrameTree", json!({})).await?;
            if topology(&after, &expected, pdf)? != frames {
                return Err("pdf-renderer-document-replaced");
            }
            c.pdf_diagnostic.as_mut().unwrap()["readinessCandidates"]
                .as_array_mut()
                .unwrap()
                .push(json!({"snapshotIndex":snapshot_index,"context":context,"state":value}));
        }
        broker.sample(c, pdf, snapshot_index).await?;
    }
    let raw = c
        .protocol(
            "Page.captureScreenshot",
            json!({"format":"png","fromSurface":true,"captureBeyondViewport":false}),
        )
        .await?;
    let bytes = STANDARD
        .decode(raw["data"].as_str().ok_or("pdf-screenshot-invalid")?)
        .map_err(|_| "pdf-screenshot-invalid")?;
    if bytes.len() < 24
        || bytes.len() > 256 * 1024
        || !bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        || [16, 20]
            .iter()
            .any(|i| u32::from_be_bytes(bytes[*i..*i + 4].try_into().unwrap()) > 4096)
    {
        return Err("pdf-screenshot-invalid");
    }
    let file = c
        .config
        .evidence_root()
        .join("native-perf-sdk-pdf-screenshot.png");
    let mut out = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&file)
        .map_err(|_| "pdf-screenshot-publication-failed")?;
    out.write_all(&bytes)
        .and_then(|()| out.sync_all())
        .map_err(|_| "pdf-screenshot-publication-failed")?;
    c.pdf_diagnostic.as_mut().unwrap()["screenshot"] = json!({"fileName":"native-perf-sdk-pdf-screenshot.png","bytes":bytes.len(),"sha256":format!("{:x}",Sha256::digest(&bytes)),"scope":"owned-public-fixture-WebView-pixels; excludes-DWM"});
    let opened = dom(c, "opened", pdf).await?;
    broker.detach_component(c).await?;
    for kind in ["mousePressed", "mouseReleased"] {
        c.protocol("Input.dispatchMouseEvent",json!({"type":kind,"x":opened["target"]["x"],"y":opened["target"]["y"],"button":"left","clickCount":1})).await?;
    }
    let closed = dom(c, "closed", pdf).await?;
    let tree = c.protocol("Page.getFrameTree", json!({})).await?;
    let actual = parse_frame(&tree, APP)?;
    if actual["mainFrameId"] != expected["mainFrameId"]
        || actual["loaderId"] != expected["loaderId"]
    {
        return Err("pdf-main-frame-replaced");
    }
    c.pdf_diagnostic.as_mut().unwrap()["closedDom"] = closed;
    c.pdf_diagnostic.as_mut().unwrap()["mainVerifiedAfterClose"] = json!(true);
    c.pdf_diagnostic.as_mut().unwrap()["frameCreatedEvents"] =
        json!(c.pdf_frame_events.load(Ordering::SeqCst));
    Ok(())
}
pub(super) async fn diagnose(c: &mut Capture) -> Outcome<()> {
    if !c.config.pdf_diagnostic || !c.config.media_journey || c.config.startup.is_some() {
        return Err("pdf-exclusive-mode-required");
    }
    let file = c
        .config
        .pdf_resource
        .as_ref()
        .ok_or("pdf-prelaunch-resource-unavailable")?;
    let pdf = format!(
        "http://solosoul-pdf.localhost/{}",
        encoded_component(file.to_str().ok_or("pdf-prelaunch-resource-unavailable")?)
    );
    c.pdf_diagnostic = Some(
        json!({"schemaVersion":4,"scope":"windows-native-sdk-public-pdf-capability","assetSha256":"ca60313e25ffa64f848d86780201a9570a0dbcb3bf733bfb5e4a65ed73f4fdfe","renderVerified":false,"performanceMetrics":null,"frameSnapshots":[],"contexts":[],"readinessCandidates":[],"openedDom":null,"closedDom":null,"screenshot":null,"mainVerifiedAfterClose":false,"frameCreatedEvents":0,"contextWatchCleaned":false,"targetDiagnostic":targets::empty(),"openingStartedAtMs":null}),
    );
    let events = Arc::new(Mutex::new(vec![]));
    watch(c, events.clone()).await?;
    let mut broker = targets::Broker::new();
    let outcome = async {
        c.protocol("Runtime.enable", json!({})).await?;
        inspect(c, &events, &pdf, &mut broker).await
    }
    .await;
    let target_cleanup = broker.finish(c).await;
    c.pdf_frame_window.store(false, Ordering::SeqCst);
    let cleanup = unwatch(c).await;
    c.pdf_diagnostic.as_mut().unwrap()["contextWatchCleaned"] = json!(cleanup.is_ok());
    // 两族订阅清理都执行，不因其中一次失败而遗留另一族。
    let runtime_cleanup = c.protocol("Runtime.disable", json!({})).await;
    outcome?;
    target_cleanup?;
    cleanup?;
    // 诊断的固定协议订阅结束；旧输入与只读测量从未启用它。
    runtime_cleanup?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pdf_url_encoding_matches_javascript_component_and_classifies_exact_owned_resource() {
        assert_eq!(
            encoded_component("C:\\Public Space\\a_b-中.pdf"),
            "C%3A%5CPublic%20Space%5Ca_b-%E4%B8%AD.pdf"
        );
        let expected = "http://solosoul-pdf.localhost/C%3A%5Ca.pdf";
        assert_eq!(url_class(expected, expected), "owned-pdf");
        for other in [
            "http://solosoul-pdf.localhost/C%3A%5Cb.pdf",
            "https://solosoul-pdf.localhost/C%3A%5Ca.pdf",
            "http://evil.invalid/",
            "chrome-extension://evil/index.html",
        ] {
            assert_eq!(url_class(other, expected), "other");
        }
    }
    #[test]
    fn pdf_topology_rejects_main_replacement_duplicates_and_unknown_parent() {
        let expected = json!({"mainFrameId":"MAIN","loaderId":"LOADER"});
        let raw = json!({"frameTree":{"frame":{"id":"MAIN","loaderId":"LOADER","url":APP,"securityOrigin":"http://tauri.localhost"},"childFrames":[{"frame":{"id":"PDF","loaderId":"PDFLOADER","parentId":"MAIN","url":"http://solosoul-pdf.localhost/x","securityOrigin":"http://solosoul-pdf.localhost"}}]}});
        assert_eq!(
            topology(&raw, &expected, "http://solosoul-pdf.localhost/x").unwrap()["frames"][0]
                ["urlClass"],
            "owned-pdf"
        );
        for (field, value) in [
            ("id", json!("NEW")),
            ("loaderId", json!("NEW")),
            ("url", json!("http://evil.invalid/")),
        ] {
            let mut bad = raw.clone();
            bad["frameTree"]["frame"][field] = value;
            assert!(topology(&bad, &expected, "x").is_err());
        }
        for (field, value) in [
            ("parentId", json!("OTHER")),
            ("id", json!("MAIN")),
            ("id", json!(null)),
        ] {
            let mut bad = raw.clone();
            bad["frameTree"]["childFrames"][0]["frame"][field] = value;
            assert!(topology(&bad, &expected, "x").is_err());
        }
    }
    #[test]
    fn pdf_context_records_structure_without_names_or_raw_origins() {
        let event = json!({"context":{"id":1,"uniqueId":"UNIQUE","origin":"http://private.invalid/secret","name":"PRIVATE_SENTINEL","auxData":{"frameId":"MAIN","isDefault":true}}});
        let proof = context_event(&event).unwrap();
        assert_eq!(proof["originClass"], "other");
        assert!(!proof.to_string().contains("private"));
        assert!(!proof.to_string().contains("PRIVATE_SENTINEL"));
        for field in ["id", "uniqueId", "auxData"] {
            let mut bad = event.clone();
            bad["context"][field] = Value::Null;
            assert!(context_event(&bad).is_err());
        }
    }
    #[test]
    fn pdf_dom_requires_actual_ready_main_document_and_actionable_target() {
        let good = json!({"result":{"value":{"schemaVersion":1,"scope":"windows-native-sdk-pdf-dom","phase":"opened","outcome":"ready","mainVerified":true,"embedded":true,"frameElementCount":0,"atMs":1,"target":{"x":10,"y":20,"actionable":true},"inputTrust":{"pointer":7,"text":1,"untrusted":0}}}});
        assert!(dom_value(&good, "opened").is_ok());
        for (key, value) in [
            ("mainVerified", json!(false)),
            ("embedded", json!(false)),
            ("outcome", json!("timeout")),
            ("phase", json!("closed")),
            ("frameElementCount", json!(5)),
            ("target", Value::Null),
        ] {
            let mut bad = good.clone();
            bad["result"]["value"][key] = value;
            assert!(dom_value(&bad, "opened").is_err());
        }
        let mut extra = good.clone();
        extra["result"]["value"]["PRIVATE_SENTINEL"] = json!("secret");
        assert!(dom_value(&extra, "opened").is_err());
    }
    #[test]
    fn pdf_renderer_candidates_are_typed_and_do_not_prove_final_rendering() {
        let mut good = json!({"result":{"value":{"schemaVersion":2,"scope":"windows-native-sdk-pdf-viewer-candidate","documentMatches":true,"viewerPresent":false,"loadSucceededMethodPresent":false,"loadSucceeded":null,"documentDimensionsPresent":false,"pageCount":null,"paintedFrames":0,"timeOriginMs":1}}});
        good["result"]["value"]["structure"] = structure::tests::empty();
        assert!(candidate(&good).is_ok());
        for (key, value) in [
            ("documentMatches", json!(false)),
            ("loadSucceeded", json!(true)),
            ("documentDimensionsPresent", json!(true)),
            ("paintedFrames", json!(2)),
            ("pageCount", json!(101)),
            ("timeOriginMs", json!(0)),
        ] {
            let mut bad = good.clone();
            bad["result"]["value"][key] = value;
            assert!(candidate(&bad).is_err());
        }
    }
}
