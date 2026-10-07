//! RF-312：固定公开 PNG 的真实原生选择器与首次 OCR 结果观测。
use super::{Capture, Outcome, RuntimeConfig};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Instant,
};
#[path = "sdk_ocr_picker.rs"]
mod picker;
const PUBLIC: &[u8] = include_bytes!("../../../crates/solosoul-core/tests/fixtures/ocr_test.png");
const SHA: &str = "56a9d54d9b70a3ee1b22e0b08b755b6cfc15ee125df4b11d52f8523583dcacaa";
const TEXT: &str = "helloppocrv6solosoulocr1234567890";
pub(super) fn input_path(root: &Path) -> PathBuf {
    root.join("profile/Documents/ocr_test.png")
}
pub(super) fn prepare(config: &RuntimeConfig) -> Result<(), String> {
    if !config.ocr_journey
        || !config.media_journey
        || config.startup.is_some()
        || config.pdf_preview
        || config.pdf_diagnostic
    {
        return Err("OCR input requires exclusive prepared media mode".into());
    }
    let path = input_path(&config.root);
    let parent = path.parent().unwrap();
    if super::super::checked_dir(parent)? != parent
        || PUBLIC.len() != 6274
        || format!("{:x}", Sha256::digest(PUBLIC)) != SHA
    {
        return Err("Fixed public OCR input identity differs".into());
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|_| "Fixed OCR input already exists or cannot be created")?;
    file.write_all(PUBLIC)
        .and_then(|()| file.sync_all())
        .map_err(|_| "Cannot publish fixed OCR input")?;
    asset_intact(&config.root).map_err(str::to_owned)?;
    Ok(())
}
fn asset_intact(root: &Path) -> Outcome<Value> {
    let path = input_path(root);
    let parent = path.parent().unwrap();
    if super::super::checked_dir(parent).map_err(|_| "ocr-public-image-replaced")? != parent {
        return Err("ocr-public-image-replaced");
    }
    super::super::require_regular(&path, false).map_err(|_| "ocr-public-image-replaced")?;
    let mut bytes = vec![];
    std::fs::File::open(&path)
        .map_err(|_| "ocr-public-image-replaced")?
        .take(6275)
        .read_to_end(&mut bytes)
        .map_err(|_| "ocr-public-image-replaced")?;
    if bytes != PUBLIC || root.join("ocr_preferences.json").exists() {
        return Err("ocr-public-image-or-default-tier-replaced");
    }
    Ok(
        json!({"relativePath":"profile/Documents/ocr_test.png","bytes":6274,"sha256":SHA,"width":500,"height":200,"tier":"small","preferencesAbsent":true}),
    )
}
pub(super) fn validate_end_state(value: &Value) -> Outcome<()> {
    let text = value["text"].as_str().ok_or("ocr-public-result-invalid")?;
    let normalized: String = text
        .bytes()
        .filter(u8::is_ascii_alphanumeric)
        .map(|b| b.to_ascii_lowercase() as char)
        .collect();
    if value.as_object().map_or(0, |v| v.len()) != 7
        || text.len() > 256
        || text.is_empty()
        || !text.is_ascii()
        || normalized != TEXT
        || value["normalizedText"] != TEXT
        || value["kind"] != "fixed-public-image"
        || value["visible"] != true
        || value["paintedFrames"] != 2
        || value["tier"] != "small"
        || value["firstScanInvokeCount"] != 1
    {
        return Err("ocr-public-result-invalid");
    }
    Ok(())
}
pub(super) async fn run(c: &mut Capture) -> Outcome<()> {
    if !c.config.ocr_journey
        || !c.config.media_journey
        || c.config.pdf_preview
        || c.config.pdf_diagnostic
        || c.config.startup.is_some()
    {
        return Err("ocr-exclusive-mode-required");
    }
    let input = asset_intact(&c.config.root)?;
    c.ocr = Some(
        json!({"schemaVersion":1,"scope":"windows-native-sdk-public-first-ocr","resource":input,"firstPerProcess":true,"picker":null,"result":null,"resultObservedAtMs":null,"performanceMetrics":null,"closedPickerVerified":false}),
    );
    let before = c.last.clone().ok_or("probe-invalid")?;
    let began = Instant::now();
    c.click("ocrCard").await?;
    c.probe("ocrReady").await?;
    c.record("ocr-page", began, &before)?;
    let before = c.last.clone().unwrap();
    let began = Instant::now();
    c.probe("ocrSelect").await?;
    let opening = c.started.elapsed().as_secs_f64() * 1000.0;
    c.click("ocrSelect").await?;
    c.step = "ocr-native-picker";
    let main = c.window.hwnd().map_err(|_| "ocr-main-hwnd-unavailable")?.0 as usize;
    let root = c.config.root.clone();
    let started = c.started;
    let chosen = tokio::task::spawn_blocking(move || picker::choose(main, &root, started, opening))
        .await
        .map_err(|_| "ocr-picker-worker-failed")??;
    let selected = chosen["selectionSubmittedAtMs"]
        .as_f64()
        .ok_or("ocr-picker-clock-invalid")?;
    let opened = chosen["openedObservedAtMs"]
        .as_f64()
        .ok_or("ocr-picker-clock-invalid")?;
    let closed = chosen["closedObservedAtMs"]
        .as_f64()
        .ok_or("ocr-picker-clock-invalid")?;
    c.ocr.as_mut().unwrap()["picker"] = chosen;
    for attempt in 0..=3 {
        let probe = c.probe("ocrResultText").await?;
        let target = &probe["target"];
        if target["actionable"] == true {
            break;
        }
        if attempt == 3 {
            return Err("ocr-result-not-onscreen");
        }
        super::validate_scroll_target(target, "ocrResultText")?;
        let wheel = &target["scroll"];
        c.protocol("Input.dispatchMouseEvent",json!({"type":"mouseWheel","x":wheel["x"],"y":wheel["y"],"deltaX":0,"deltaY":wheel["deltaY"]})).await?;
    }
    let result = c.probe("ocrResultReady").await?;
    let observed = c.started.elapsed().as_secs_f64() * 1000.0;
    asset_intact(&c.config.root)?;
    let scans = super::ipc_delta(
        &json!({"commands":[],"timeOriginMs":result["timeOriginMs"],"runId":c.config.run_id}),
        &result["observer"],
    )?;
    if scans["commands"]["ocr_scan_image"] != 1 {
        return Err("ocr-first-scan-invoke-count-invalid");
    }
    let details = c.ocr.as_mut().unwrap();
    details["result"] = result["ocr"].clone();
    details["resultObservedAtMs"] = json!(observed);
    details["closedPickerVerified"] = json!(true);
    details["performanceMetrics"] = json!({"pickerOpenedObservedMs":opened-opening,"pickerSelectionClosedMs":closed-opened,"firstOcrResultObservedMs":observed-selected,"totalOcrObservedMs":observed-opening});
    c.record("first-ocr", began, &before)?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_fixed_public_three_line_result_is_accepted() {
        let v = json!({"kind":"fixed-public-image","text":"Hello PP-OCRv6\nSoloSoul OCR\n1234567890","normalizedText":TEXT,"visible":true,"paintedFrames":2,"tier":"small","firstScanInvokeCount":1});
        assert!(validate_end_state(&v).is_ok());
        for (key, value) in [
            ("text", json!("Hello")),
            ("visible", json!(false)),
            ("paintedFrames", json!(1)),
            ("firstScanInvokeCount", json!(2)),
            ("tier", json!("medium")),
        ] {
            let mut bad = v.clone();
            bad[key] = value;
            assert!(validate_end_state(&bad).is_err());
        }
        let mut bad = v;
        bad["private"] = json!("not allowed");
        assert!(validate_end_state(&bad).is_err());
    }
    #[test]
    fn selected_plaintext_image_and_default_tier_cannot_be_replaced() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        std::fs::create_dir_all(root.join("profile/Documents")).unwrap();
        let path = input_path(&root);
        std::fs::write(&path, PUBLIC).unwrap();
        assert!(asset_intact(&root).is_ok());
        let mut changed = PUBLIC.to_vec();
        changed[100] ^= 1;
        std::fs::write(&path, changed).unwrap();
        assert!(asset_intact(&root).is_err());
        std::fs::write(&path, PUBLIC).unwrap();
        std::fs::write(root.join("ocr_preferences.json"), "{}").unwrap();
        assert!(asset_intact(&root).is_err());
    }
    #[test]
    fn source_is_fixed_public_png_not_vault_ciphertext() {
        assert_eq!(PUBLIC.len(), 6274);
        assert_eq!(format!("{:x}", Sha256::digest(PUBLIC)), SHA);
        assert!(PUBLIC.starts_with(b"\x89PNG\r\n\x1a\n"));
    }
}
