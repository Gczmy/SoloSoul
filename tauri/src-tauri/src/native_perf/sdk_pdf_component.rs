//! 只附加已观察到且与主目标同context的唯一组件webview，不接受任意目标。
use super::{identifier, info, Broker, Capture, Outcome, Target};
use serde_json::{json, Value};

pub(super) struct Attachment {
    session: String,
    target: Target,
    snapshot: usize,
    detached: bool,
    mode: &'static str,
}
impl Attachment {
    pub(super) fn session(&self) -> &str {
        &self.session
    }
    pub(super) fn detached(&self) -> bool {
        self.detached
    }
    pub(super) fn proof(&self) -> Value {
        json!({"target":self.target.proof,"sessionId":self.session,"beforeSnapshotIndex":self.snapshot,"parentVerified":true,"browserContextVerified":true,"detached":self.detached,"detachMode":self.mode})
    }
}
fn component_bound(main: &Value, target: &Value) -> bool {
    identifier(&main["id"])
        && identifier(&main["browserContextId"])
        && main["type"] == "page"
        && main["urlClass"] == "application"
        && identifier(&target["id"])
        && target["id"] != main["id"]
        && target["type"] == "webview"
        && target["urlClass"] == "component-extension"
        && target["parentId"] == main["id"]
        && target["browserContextId"] == main["browserContextId"]
}
fn attach_reply(raw: &Value) -> Outcome<String> {
    if raw.as_object().map_or(0, |x| x.len()) != 1 || !identifier(&raw["sessionId"]) {
        return Err("pdf-component-attach-reply-invalid");
    }
    Ok(raw["sessionId"].as_str().unwrap().to_owned())
}
impl Broker {
    pub(super) async fn attach_component(
        &mut self,
        c: &mut Capture,
        pdf: &str,
        main: &Target,
        raw: &[Value],
        observed: &[Value],
        index: usize,
    ) -> Outcome<()> {
        if let Some(current) = &self.component {
            if current.detached
                || !observed.iter().any(|t| {
                    t["id"] == current.target.proof["id"] && component_bound(&main.proof, t)
                })
            {
                return Err("pdf-component-document-replaced");
            }
            return Ok(());
        }
        let candidates: Vec<_> = observed
            .iter()
            .filter(|t| component_bound(&main.proof, t))
            .collect();
        if candidates.is_empty() {
            return Ok(());
        }
        if candidates.len() != 1 || candidates[0]["attached"] != false {
            return Err("pdf-component-ambiguous-or-already-attached");
        }
        if !self.sessions.values().any(|s| {
            s.active
                && s.target.proof["type"] == "iframe"
                && s.target.proof["urlClass"] == "owned-pdf"
                && s.target.proof["parentId"] == main.proof["id"]
                && s.target.proof["browserContextId"] == main.proof["browserContextId"]
        }) {
            return Err("pdf-component-source-unbound");
        }
        let id = candidates[0]["id"]
            .as_str()
            .ok_or("pdf-component-source-unbound")?;
        let previous = info(
            raw.iter()
                .find(|v| v["targetId"] == id)
                .ok_or("pdf-component-unlisted")?,
            pdf,
        )?;
        let current = c
            .protocol("Target.getTargetInfo", json!({"targetId":id}))
            .await?;
        let current = info(&current["targetInfo"], pdf)?;
        if current.proof != previous.proof
            || current.url != previous.url
            || !component_bound(&main.proof, &current.proof)
        {
            return Err("pdf-component-document-replaced");
        }
        let reply = c
            .pdf_lifecycle(
                "Target.attachToTarget",
                json!({"targetId":id,"flatten":true}),
            )
            .await?;
        let session = attach_reply(&reply)?;
        // 回复之后立即保存清理责任，即使后续事件或目标核验失败也会尝试解除。
        self.component = Some(Attachment {
            session: session.clone(),
            target: current,
            snapshot: index,
            detached: false,
            mode: "pending",
        });
        self.sync()?;
        let row = self
            .sessions
            .get(&session)
            .ok_or("pdf-component-attach-event-unconfirmed")?;
        let attached = self.component.as_ref().unwrap();
        if !row.active
            || row.target.proof["id"] != attached.target.proof["id"]
            || row.target.url != attached.target.url
            || !component_bound(&main.proof, &row.target.proof)
        {
            return Err("pdf-component-attach-event-unconfirmed");
        }
        Ok(())
    }
    pub(in super::super) async fn detach_component(&mut self, c: &mut Capture) -> Outcome<()> {
        if self.component.as_ref().is_none_or(|x| x.detached) {
            return Ok(());
        }
        self.sync()?;
        let id = self.component.as_ref().unwrap().session.clone();
        if self.sessions.get(&id).is_some_and(|x| !x.active) {
            let attachment = self.component.as_mut().unwrap();
            attachment.detached = true;
            attachment.mode = "event";
            return Ok(());
        }
        let raw = c
            .pdf_lifecycle("Target.detachFromTarget", json!({"sessionId":id}))
            .await?;
        if raw != json!({}) {
            return Err("pdf-component-detach-reply-invalid");
        }
        let attachment = self.component.as_mut().unwrap();
        attachment.detached = true;
        attachment.mode = "explicit-reply";
        self.sync()?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn main() -> Value {
        json!({"id":"MAIN","type":"page","urlClass":"application","browserContextId":"CTX"})
    }
    fn target() -> Value {
        json!({"id":"COMP","type":"webview","urlClass":"component-extension","parentId":"MAIN","browserContextId":"CTX"})
    }
    #[test]
    fn component_requires_exact_main_parent_context_and_document_class() {
        assert!(component_bound(&main(), &target()));
        for (key, value) in [
            ("id", json!("MAIN")),
            ("type", json!("iframe")),
            ("urlClass", json!("other")),
            ("parentId", json!("FOREIGN")),
            ("browserContextId", json!("FOREIGN")),
        ] {
            let mut bad = target();
            bad[key] = value;
            assert!(!component_bound(&main(), &bad));
        }
    }
    #[test]
    fn missing_context_or_opener_alone_cannot_bind_component() {
        let mut m = main();
        m["browserContextId"] = Value::Null;
        let mut t = target();
        t["browserContextId"] = Value::Null;
        assert!(!component_bound(&m, &t));
        t = target();
        t["parentId"] = Value::Null;
        t["openerId"] = json!("MAIN");
        assert!(!component_bound(&main(), &t));
    }
    #[test]
    fn component_reply_requires_only_one_typed_session_identifier() {
        assert_eq!(attach_reply(&json!({"sessionId":"S"})).unwrap(), "S");
        for v in [
            json!({}),
            json!({"sessionId":"BAD SPACE"}),
            json!({"sessionId":"S","private":"TEXT"}),
        ] {
            assert!(attach_reply(&v).is_err());
        }
    }
    #[test]
    fn explicit_detach_reply_keeps_session_inactive_when_events_are_replayed() {
        let mut b = Broker::new();
        let mut raw = target();
        raw["targetId"] = json!("COMP");
        raw["url"] = json!("chrome-extension://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/index.html");
        raw["attached"] = json!(true);
        let target = info(&raw, "PDF").unwrap();
        b.component = Some(Attachment {
            session: "S".into(),
            target: target.clone(),
            snapshot: 0,
            detached: true,
            mode: "explicit-reply",
        });
        b.events
            .lock()
            .unwrap()
            .push(super::super::Event::Attached("".into(), "S".into(), target));
        b.sync().unwrap();
        assert!(!b.sessions["S"].active);
        assert_eq!(b.component.unwrap().proof()["detachMode"], "explicit-reply");
    }
}
