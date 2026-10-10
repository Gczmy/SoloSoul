//! RF-121 optional owned-file handshake around a fixed, read-only DOM observation.
//! Absent request leaves all existing native journeys unchanged. No production command is added.
use super::{parse_frame, publish, Capture, Outcome};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs::OpenOptions, io::Read, os::windows::fs::OpenOptionsExt, path::Path, time::Duration};

const SCRIPT: &str = include_str!("sdk_surface.js");
const REQUEST: &str = "native-perf-surface-request.json";
const DONE: &str = "native-perf-surface-done.json";
const BEFORE: &str = "native-perf-surface-before.json";
const AFTER: &str = "native-perf-surface-after.json";
const KEYS: &[&str] = &[
    "schemaVersion",
    "scope",
    "runId",
    "href",
    "timeOriginMs",
    "atMs",
    "theme",
    "platform",
    "material",
    "highContrast",
    "forcedColors",
    "reducedTransparency",
    "focused",
    "visibility",
    "viewport",
    "expanded",
    "palette",
    "surfaces",
];

fn exact(value: &Value, keys: &[&str]) -> bool {
    value.as_object().is_some_and(|object| {
        object.len() == keys.len() && keys.iter().all(|k| object.contains_key(*k))
    })
}
fn hex(value: &Value, len: usize) -> bool {
    value.as_str().is_some_and(|s| {
        s.len() == len
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
fn number(value: &Value, min: f64, max: f64) -> bool {
    value
        .as_f64()
        .is_some_and(|n| n.is_finite() && (min..=max).contains(&n))
}
fn color(value: &Value) -> bool {
    value.as_array().is_some_and(|a| {
        a.len() == 4 && a[..3].iter().all(|n| number(n, 0.0, 255.0)) && number(&a[3], 0.0, 1.0)
    })
}
fn bytes(path: &Path, max: u64) -> Outcome<Vec<u8>> {
    // Open the terminal path without following a reparse point; all parent paths are the consumed owned root.
    super::super::require_regular(path, false).map_err(|_| "surface-file-invalid")?;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(0x0020_0000)
        .open(path)
        .map_err(|_| "surface-file-invalid")?;
    let mut data = Vec::new();
    file.take(max + 1)
        .read_to_end(&mut data)
        .map_err(|_| "surface-file-invalid")?;
    if data.len() as u64 > max {
        return Err("surface-file-invalid");
    }
    Ok(data)
}
fn request(value: &Value, run_id: &str, pid: u32, browser_pid: u32) -> Outcome<()> {
    if !exact(
        value,
        &[
            "schemaVersion",
            "scope",
            "runId",
            "pid",
            "browserPid",
            "token",
        ],
    ) || value["schemaVersion"] != 1
        || value["scope"] != "windows-native-surface-request"
        || value["runId"] != run_id
        || value["pid"] != pid
        || value["browserPid"] != browser_pid
        || !hex(&value["token"], 32)
    {
        return Err("surface-request-invalid");
    }
    Ok(())
}
fn state(raw: &Value, run_id: &str, clock: &Value, minimum_ms: f64) -> Outcome<Value> {
    let value = &raw["result"]["value"];
    if raw.get("exceptionDetails").is_some()
        || raw["result"]["type"] != "object"
        || !exact(value, KEYS)
        || value["schemaVersion"] != 1
        || value["scope"] != "windows-native-surface-state"
        || value["runId"] != run_id
        || value["href"] != "http://tauri.localhost/"
        || value["timeOriginMs"] != *clock
        || !number(&value["atMs"], minimum_ms, 300_000.0)
        || !matches!(value["theme"].as_str(), Some("light" | "dark"))
        || value["platform"] != "windows"
        || !matches!(value["material"].as_str(), Some("mica" | "solid"))
        || [
            "highContrast",
            "forcedColors",
            "reducedTransparency",
            "focused",
            "expanded",
        ]
        .iter()
        .any(|k| !value[k].is_boolean())
        || !matches!(value["visibility"].as_str(), Some("visible" | "hidden"))
    {
        return Err("surface-state-invalid");
    }
    let viewport = value["viewport"]
        .as_array()
        .ok_or("surface-state-invalid")?;
    if viewport.len() != 3
        || !number(&viewport[0], 1.0, 16384.0)
        || !number(&viewport[1], 1.0, 16384.0)
        || !number(&viewport[2], 0.25, 8.0)
        || !exact(&value["palette"], &["base", "elevated", "text"])
        || ["base", "elevated", "text"]
            .iter()
            .any(|k| !color(&value["palette"][k]))
        || !exact(
            &value["surfaces"],
            &["navigation", "appbar", "content", "card"],
        )
    {
        return Err("surface-state-invalid");
    }
    for key in ["navigation", "appbar", "content", "card"] {
        let surface = &value["surfaces"][key];
        let rect = surface["rect"].as_array().ok_or("surface-state-invalid")?;
        if !exact(surface, &["rect", "background", "text", "radius"])
            || rect.len() != 4
            || !number(&rect[0], -16384.0, 16384.0)
            || !number(&rect[1], -16384.0, 16384.0)
            || !number(&rect[2], 1.0, 16384.0)
            || !number(&rect[3], 1.0, 16384.0)
            || !color(&surface["background"])
            || !color(&surface["text"])
            || !number(&surface["radius"], 0.0, 128.0)
        {
            return Err("surface-state-invalid");
        }
    }
    Ok(value.clone())
}
fn stable(before: &Value, after: &Value) -> bool {
    let mut a = before.clone();
    let mut b = after.clone();
    a.as_object_mut().unwrap().remove("atMs");
    b.as_object_mut().unwrap().remove("atMs");
    after["atMs"]
        .as_f64()
        .zip(before["atMs"].as_f64())
        .is_some_and(|(b, a)| b >= a)
        && a == b
}
fn acknowledgment(value: &Value, req: &Value, before_sha: &str, pixel_sha: &str) -> Outcome<()> {
    let mut expected = req.clone();
    expected["scope"] = json!("windows-native-surface-done");
    expected["beforeSha256"] = json!(before_sha);
    expected["pixelSha256"] = json!(pixel_sha);
    if *value != expected {
        return Err("surface-acknowledgment-invalid");
    }
    Ok(())
}

async fn observe(c: &mut Capture, minimum_ms: f64) -> Outcome<Value> {
    let clock = c.time_origin.clone().ok_or("surface-binding-unavailable")?;
    let frame = c.frame.clone().ok_or("surface-binding-unavailable")?;
    let expression = SCRIPT.replace("__REQUEST__", &json!({"runId":c.config.run_id}).to_string());
    let raw = c
        .protocol(
            "Runtime.evaluate",
            json!({"expression":expression,"returnByValue":true}),
        )
        .await?;
    let value = state(&raw, &c.config.run_id, &clock, minimum_ms)?;
    let tree = c.protocol("Page.getFrameTree", json!({})).await?;
    let actual = parse_frame(&tree, "http://tauri.localhost/")?;
    if actual["mainFrameId"] != frame["mainFrameId"] || actual["loaderId"] != frame["loaderId"] {
        return Err("surface-document-replaced");
    }
    Ok(value)
}
pub(super) async fn capture(c: &mut Capture) -> Outcome<()> {
    let path = c.config.root.join(REQUEST);
    match std::fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err("surface-request-invalid"),
        Ok(_) => {}
    }
    if c.config.startup.is_some()
        || c.config.media_journey
        || c.config.sdk_cdp
        || !c.config.sdk_journey
    {
        return Err("surface-mode-invalid");
    }
    let req: Value =
        serde_json::from_slice(&bytes(&path, 4096)?).map_err(|_| "surface-request-invalid")?;
    request(
        &req,
        &c.config.run_id,
        std::process::id(),
        c.browser_pid.ok_or("surface-binding-unavailable")?,
    )?;
    let min = c
        .last
        .as_ref()
        .and_then(|v| v["atMs"].as_f64())
        .ok_or("surface-binding-unavailable")?;
    let before = observe(c, min).await?;
    let base = c.config.root.clone();
    let before_proof = json!({"schemaVersion":1,"scope":"windows-native-surface-before","request":req,
        "binding":c.frame,"state":before});
    publish(&base, BEFORE, &before_proof)?;
    let before_sha = format!("{:x}", Sha256::digest(bytes(&base.join(BEFORE), 262144)?));
    let began = std::time::Instant::now();
    loop {
        if c.invalidated.load(std::sync::atomic::Ordering::SeqCst) {
            return Err("document-invalidated");
        }
        if began.elapsed() > Duration::from_secs(10) {
            return Err("surface-capture-timeout");
        }
        match std::fs::symlink_metadata(base.join(DONE)) {
            Ok(_) => break,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                tokio::time::sleep(Duration::from_millis(10)).await
            }
            Err(_) => return Err("surface-acknowledgment-invalid"),
        }
    }
    let pixel_name = match before["theme"].as_str() {
        Some("light") => "rf121-wgc-light-visible.bgra",
        Some("dark") => "rf121-wgc-dark-visible.bgra",
        _ => return Err("surface-state-invalid"),
    };
    let pixel_sha = format!(
        "{:x}",
        Sha256::digest(bytes(&base.join(pixel_name), 32 * 1024 * 1024)?)
    );
    let ack: Value = serde_json::from_slice(&bytes(&base.join(DONE), 4096)?)
        .map_err(|_| "surface-acknowledgment-invalid")?;
    acknowledgment(&ack, &req, &before_sha, &pixel_sha)?;
    let after = observe(c, before["atMs"].as_f64().unwrap()).await?;
    let unchanged = stable(&before, &after);
    publish(
        &base,
        AFTER,
        &json!({"schemaVersion":1,"scope":"windows-native-surface-after","request":req,
        "binding":c.frame,"beforeSha256":before_sha,"pixelSha256":pixel_sha,"state":after,"stable":unchanged,
        "applicationMaterialAccepted":false,"performanceMetrics":null}),
    )?;
    if !unchanged {
        return Err("surface-state-changed-during-capture");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn surface_file_reads_actual_bytes_and_rejects_oversize() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("public.json");
        std::fs::write(&path, b"public-fixture").unwrap();
        assert_eq!(bytes(&path, 20).unwrap(), b"public-fixture");
        assert_eq!(bytes(&path, 5).unwrap_err(), "surface-file-invalid");
    }
    #[test]
    fn surface_file_rejects_directory_and_missing_marker() {
        let temp = tempfile::tempdir().unwrap();
        assert_eq!(
            bytes(temp.path(), 4096).unwrap_err(),
            "surface-file-invalid"
        );
        assert_eq!(
            bytes(&temp.path().join("missing.json"), 4096).unwrap_err(),
            "surface-file-invalid"
        );
    }
    fn sample() -> Value {
        let surface = json!({"rect":[0,0,100,100],"background":[255,255,255,1],"text":[17,17,17,1],"radius":12});
        json!({"result":{"type":"object","value":{"schemaVersion":1,"scope":"windows-native-surface-state","runId":"run",
            "href":"http://tauri.localhost/","timeOriginMs":10,"atMs":100,"theme":"light","platform":"windows","material":"mica",
            "highContrast":false,"forcedColors":false,"reducedTransparency":false,"focused":true,"visibility":"visible","viewport":[1024,750,1],"expanded":true,
            "palette":{"base":[250,250,246,1],"elevated":[253,252,249,1],"text":[17,17,17,1]},
            "surfaces":{"navigation":surface,"appbar":surface,"content":surface,"card":surface}}}})
    }
    #[test]
    fn surface_state_accepts_bounded_palette_geometry_and_actual_identity() {
        assert!(state(&sample(), "run", &json!(10), 99.0).is_ok());
    }
    #[test]
    fn surface_state_rejects_payload_unknown_fields_and_invalid_colors() {
        for (pointer, value) in [
            ("/result/value/payload", json!("secret")),
            ("/result/value/palette/extra", json!([])),
            ("/result/value/palette/base", json!([256, 0, 0, 1])),
            (
                "/result/value/surfaces/card/background",
                json!([0, 0, 0, 2]),
            ),
        ] {
            let mut v = sample();
            let (parent, key) = pointer.rsplit_once('/').unwrap();
            v.pointer_mut(parent).unwrap()[key] = value;
            assert!(state(&v, "run", &json!(10), 0.0).is_err());
        }
    }
    #[test]
    fn surface_state_rejects_document_clock_mode_and_missing_targets() {
        for (key, value) in [
            ("href", json!("http://tauri.localhost/search")),
            ("runId", json!("old")),
            ("timeOriginMs", json!(11)),
            ("atMs", json!(98)),
            ("theme", json!("system")),
            ("platform", json!("macos")),
            ("material", json!("liquid-glass")),
            ("viewport", json!([1024, 750, 9])),
            ("surfaces", json!({})),
        ] {
            let mut v = sample();
            v["result"]["value"][key] = value;
            assert!(state(&v, "run", &json!(10), 99.0).is_err());
        }
    }
    #[test]
    fn surface_request_rejects_replay_and_arbitrary_input() {
        let valid = json!({"schemaVersion":1,"scope":"windows-native-surface-request","runId":"run","pid":1,"browserPid":2,"token":"0123456789abcdef0123456789abcdef"});
        assert!(request(&valid, "run", 1, 2).is_ok());
        for (key, value) in [
            ("runId", json!("old")),
            ("pid", json!(2)),
            ("browserPid", json!(1)),
            ("token", json!("../path")),
            ("expression", json!("arbitrary")),
        ] {
            let mut v = valid.clone();
            v[key] = value;
            assert!(request(&v, "run", 1, 2).is_err());
        }
    }
    #[test]
    fn surface_acknowledgment_binds_original_request_and_exact_pixel_bytes() {
        let req = json!({"scope":"windows-native-surface-request","token":"token"});
        let mut ack = req.clone();
        ack["scope"] = json!("windows-native-surface-done");
        ack["beforeSha256"] = json!("before");
        ack["pixelSha256"] = json!("pixel");
        assert!(acknowledgment(&ack, &req, "before", "pixel").is_ok());
        for (key, value) in [
            ("token", json!("old")),
            ("beforeSha256", json!("other")),
            ("pixelSha256", json!("other")),
            ("payload", json!("secret")),
        ] {
            let mut v = ack.clone();
            v[key] = value;
            assert!(acknowledgment(&v, &req, "before", "pixel").is_err());
        }
    }
    #[test]
    fn surface_stability_rejects_theme_focus_geometry_and_time_changes() {
        let before = sample()["result"]["value"].clone();
        let mut after = before.clone();
        after["atMs"] = json!(101);
        assert!(stable(&before, &after));
        for (key, value) in [
            ("theme", json!("dark")),
            ("focused", json!(false)),
            ("viewport", json!([1100, 750, 1])),
            ("atMs", json!(99)),
            ("expanded", json!(false)),
        ] {
            let mut v = after.clone();
            v[key] = value;
            assert!(!stable(&before, &v));
        }
    }
}
