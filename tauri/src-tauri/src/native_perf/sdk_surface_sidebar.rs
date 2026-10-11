//! Optional fixed light/collapsed scene on the already consumed synthetic account.
//! No new route, arbitrary selector, script, input text or production command is accepted.
use super::{bytes, exact, number, parse_frame, publish, Capture, Outcome};
use serde_json::{json, Value};

const REQUEST: &str = "native-perf-surface-scene-request.json";
const SCRIPT: &str = include_str!("sdk_surface_sidebar.js");

fn request(value: &Value, parent: &Value) -> Outcome<()> {
    let mut expected = parent.clone();
    expected["scope"] = json!("windows-native-sidebar-scene-request");
    expected["scene"] = json!("light-collapsed");
    if *value != expected {
        return Err("sidebar-scene-request-invalid");
    }
    Ok(())
}

pub(super) fn requested(c: &Capture, parent: &Value) -> Outcome<Option<Value>> {
    let path = c.config.root.join(REQUEST);
    match std::fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("sidebar-scene-request-invalid"),
        Ok(_) => {}
    }
    let value = serde_json::from_slice(&bytes(&path, 4096)?)
        .map_err(|_| "sidebar-scene-request-invalid")?;
    request(&value, parent)?;
    Ok(Some(value))
}

fn probe(raw: &Value, clock: &Value, minimum_ms: f64) -> Outcome<Value> {
    let value = &raw["result"]["value"];
    if raw.get("exceptionDetails").is_some()
        || raw["result"]["type"] != "object"
        || !exact(
            value,
            &[
                "schemaVersion",
                "scope",
                "href",
                "timeOriginMs",
                "atMs",
                "theme",
                "platform",
                "expanded",
                "viewport",
                "focused",
                "visibility",
                "layout",
                "settled",
                "target",
                "inputTrust",
            ],
        )
        || value["schemaVersion"] != 1
        || value["scope"] != "windows-native-sidebar-probe"
        || value["href"] != "http://tauri.localhost/"
        || value["timeOriginMs"] != *clock
        || !number(&value["atMs"], minimum_ms, 300_000.0)
        || value["theme"] != "light"
        || value["platform"] != "windows"
        || !value["expanded"].is_boolean()
        || value["focused"] != true
        || value["visibility"] != "visible"
        || value["settled"] != true
        || !exact(&value["target"], &["x", "y", "actionable"])
        || value["target"]["actionable"] != true
        || !exact(&value["inputTrust"], &["pointer", "text", "untrusted"])
        || value["inputTrust"]["pointer"]
            .as_u64()
            .is_none_or(|n| !(10..=128).contains(&n))
        || value["inputTrust"]["text"] != 3
        || value["inputTrust"]["untrusted"] != 0
    {
        return Err("sidebar-scene-probe-invalid");
    }
    let viewport = value["viewport"]
        .as_array()
        .ok_or("sidebar-scene-probe-invalid")?;
    let layout = value["layout"]
        .as_array()
        .ok_or("sidebar-scene-probe-invalid")?;
    if viewport.len() != 3
        || !number(&viewport[0], 1.0, 16384.0)
        || !number(&viewport[1], 1.0, 16384.0)
        || !number(&viewport[2], 0.25, 8.0)
        || !number(
            &value["target"]["x"],
            0.0,
            viewport[0].as_f64().unwrap() - 1.0,
        )
        || !number(
            &value["target"]["y"],
            0.0,
            viewport[1].as_f64().unwrap() - 1.0,
        )
        || layout.len() != 3
        || layout.iter().any(|r| {
            !r.as_array().is_some_and(|r| {
                r.len() == 4
                    && r[..2].iter().all(|n| number(n, -16384.0, 16384.0))
                    && r[2..].iter().all(|n| number(n, 1.0, 16384.0))
            })
        })
    {
        return Err("sidebar-scene-probe-invalid");
    }
    Ok(value.clone())
}

pub(super) async fn observe(c: &mut Capture, minimum_ms: f64) -> Outcome<Value> {
    let clock = c.time_origin.clone().ok_or("surface-binding-unavailable")?;
    let raw = c
        .protocol(
            "Runtime.evaluate",
            json!({"expression":SCRIPT,"returnByValue":true,"awaitPromise":true}),
        )
        .await?;
    let value = probe(&raw, &clock, minimum_ms)?;
    let tree = c.protocol("Page.getFrameTree", json!({})).await?;
    let frame = parse_frame(&tree, "http://tauri.localhost/")?;
    let expected = c.frame.as_ref().ok_or("surface-binding-unavailable")?;
    if frame["mainFrameId"] != expected["mainFrameId"] || frame["loaderId"] != expected["loaderId"]
    {
        return Err("surface-document-replaced");
    }
    Ok(value)
}

pub(super) fn initial(value: &Value) -> Outcome<()> {
    if value["expanded"] != true {
        return Err("sidebar-scene-initial-state-invalid");
    }
    Ok(())
}

async fn toggle(c: &mut Capture, before: &Value) -> Outcome<Value> {
    for kind in ["mousePressed", "mouseReleased"] {
        c.protocol(
            "Input.dispatchMouseEvent",
            json!({"type":kind,"x":before["target"]["x"],
            "y":before["target"]["y"],"button":"left","clickCount":1}),
        )
        .await?;
    }
    let after = observe(c, before["atMs"].as_f64().unwrap()).await?;
    transition(before, &after)?;
    Ok(after)
}

fn transition(before: &Value, after: &Value) -> Outcome<()> {
    if after["expanded"].as_bool() != before["expanded"].as_bool().map(|b| !b)
        || after["inputTrust"]["pointer"].as_u64()
            != before["inputTrust"]["pointer"]
                .as_u64()
                .and_then(|n| n.checked_add(1))
        || after["viewport"] != before["viewport"]
    {
        return Err("sidebar-scene-transition-invalid");
    }
    Ok(())
}

pub(super) async fn prepare(c: &mut Capture, req: &Value, initial: &Value) -> Outcome<()> {
    let prepared = toggle(c, initial).await?;
    publish(
        &c.config.root,
        "native-perf-surface-scene-prepared.json",
        &json!({
            "schemaVersion":1,"scope":"windows-native-sidebar-scene-prepared","request":req,
            "binding":c.frame,"initial":initial,"prepared":prepared,"performanceMetrics":null,
        }),
    )
}

pub(super) async fn restore(c: &mut Capture, req: &Value, initial: &Value) -> Outcome<()> {
    let before = observe(c, initial["atMs"].as_f64().unwrap()).await?;
    let restored = if before["expanded"] == false {
        toggle(c, &before).await?
    } else {
        before.clone()
    };
    if restored["expanded"] != initial["expanded"] || restored["viewport"] != initial["viewport"] {
        return Err("sidebar-scene-restore-invalid");
    }
    publish(
        &c.config.root,
        "native-perf-surface-scene-restored.json",
        &json!({
            "schemaVersion":1,"scope":"windows-native-sidebar-scene-restored","request":req,
            "binding":c.frame,"before":before,"restored":restored,"performanceMetrics":null,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sidebar_request_rejects_replay_unknown_fields_and_other_scenes() {
        let parent = json!({"schemaVersion":1,"scope":"windows-native-surface-request","runId":"run","pid":1,"browserPid":2,"token":"0123456789abcdef0123456789abcdef"});
        let mut valid = parent.clone();
        valid["scope"] = json!("windows-native-sidebar-scene-request");
        valid["scene"] = json!("light-collapsed");
        assert!(request(&valid, &parent).is_ok());
        for (key, value) in [
            ("runId", json!("old")),
            ("pid", json!(2)),
            ("browserPid", json!(1)),
            ("token", json!("old")),
            ("scene", json!("dark-collapsed")),
            ("expression", json!("arbitrary")),
        ] {
            let mut bad = valid.clone();
            bad[key] = value;
            assert!(request(&bad, &parent).is_err());
        }
    }
    fn sample() -> Value {
        json!({"result":{"type":"object","value":{"schemaVersion":1,"scope":"windows-native-sidebar-probe",
            "href":"http://tauri.localhost/","timeOriginMs":10,"atMs":100,"theme":"light","platform":"windows",
            "expanded":true,"viewport":[1024,750,1],"focused":true,"visibility":"visible","settled":true,
            "layout":[[0,0,250,750],[250,0,774,48],[250,48,774,702]],"target":{"x":220,"y":24,"actionable":true},
            "inputTrust":{"pointer":10,"text":3,"untrusted":0}}}})
    }
    #[test]
    fn sidebar_probe_rejects_untrusted_offscreen_stale_and_unsettled_input() {
        assert!(probe(&sample(), &json!(10), 99.0).is_ok());
        for (pointer, value) in [
            ("href", json!("http://tauri.localhost/settings")),
            ("timeOriginMs", json!(11)),
            ("atMs", json!(98)),
            ("theme", json!("dark")),
            ("focused", json!(false)),
            ("settled", json!(false)),
            ("target", json!({"x":1024,"y":24,"actionable":true})),
            ("inputTrust", json!({"pointer":11,"text":3,"untrusted":1})),
            ("layout", json!([[0, 0, 0, 1]])),
            ("payload", json!("secret")),
        ] {
            let mut bad = sample();
            bad["result"]["value"][pointer] = value;
            assert!(probe(&bad, &json!(10), 99.0).is_err());
        }
    }
    #[test]
    fn sidebar_transition_requires_one_real_pointer_and_changed_sidebar_only() {
        let before = sample()["result"]["value"].clone();
        let mut after = before.clone();
        after["expanded"] = json!(false);
        after["inputTrust"]["pointer"] = json!(11);
        assert!(transition(&before, &after).is_ok());
        for (key, value) in [
            ("expanded", json!(true)),
            ("viewport", json!([1100, 750, 1])),
            ("inputTrust", json!({"pointer":10,"text":3,"untrusted":0})),
        ] {
            let mut bad = after.clone();
            bad[key] = value;
            assert!(transition(&before, &bad).is_err());
        }
    }
}
