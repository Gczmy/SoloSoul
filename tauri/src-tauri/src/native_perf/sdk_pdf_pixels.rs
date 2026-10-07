//! RF-312：固定公开 PDF 首屏像素门禁；不推断其他页或任意 PDF 加载完成。
use super::super::{Capture, Outcome};
use super::{dom, encoded_component, topology, APP};
use base64::{engine::general_purpose::STANDARD, Engine};
use image::{ImageFormat, ImageReader, Limits};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{Cursor, Read, Write};
use std::sync::atomic::Ordering;
use std::time::Duration;
const ASSET: &str = "ca60313e25ffa64f848d86780201a9570a0dbcb3bf733bfb5e4a65ed73f4fdfe";
const MASK: &str = "6f307e964c1dba0a23593241d6d193823cdf8feebb3946ca80b5af9a4a165044";
const WIDTH: u32 = 1028;
const HEIGHT: u32 = 749;
const ROI: [u32; 4] = [180, 225, 430, 36];
const MAX_CAPTURES: usize = 16;
const INTERVAL_MS: u64 = 250;
fn pixels(bytes: &[u8]) -> Outcome<Value> {
    if bytes.len() < 33
        || bytes.len() > 256 * 1024
        || !bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        || bytes[8..16] != [0, 0, 0, 13, b'I', b'H', b'D', b'R']
        || u32::from_be_bytes(bytes[16..20].try_into().unwrap()) != WIDTH
        || u32::from_be_bytes(bytes[20..24].try_into().unwrap()) != HEIGHT
        || bytes[24..29] != [8, 6, 0, 0, 0]
    {
        return Err("pdf-pixel-viewport-or-format-mismatch");
    }
    let mut reader = ImageReader::with_format(Cursor::new(bytes), ImageFormat::Png);
    let mut limits = Limits::default();
    limits.max_image_width = Some(WIDTH);
    limits.max_image_height = Some(HEIGHT);
    limits.max_alloc = Some(16 * 1024 * 1024);
    reader.limits(limits);
    let image = reader
        .decode()
        .map_err(|_| "pdf-pixel-decode-invalid")?
        .to_rgba8();
    if image.dimensions() != (WIDTH, HEIGHT) {
        return Err("pdf-pixel-viewport-or-format-mismatch");
    }
    let mut mask = Vec::with_capacity((ROI[2] * ROI[3]) as usize);
    for y in ROI[1]..ROI[1] + ROI[3] {
        for x in ROI[0]..ROI[0] + ROI[2] {
            let p = image.get_pixel(x, y).0;
            mask.push(u8::from(p[3] >= 240 && p[..3].iter().all(|n| *n <= 64)));
        }
    }
    let count: usize = mask.iter().map(|n| *n as usize).sum();
    let hash = format!("{:x}", Sha256::digest(&mask));
    Ok(
        json!({"width":WIDTH,"height":HEIGHT,"darkPixels":count,"maskSha256":hash,"matches":count==530 && hash==MASK}),
    )
}
fn resource_identity(p: &std::path::Path, expected: &str) -> Outcome<Value> {
    super::super::super::require_regular(p, false).map_err(|_| "pdf-public-asset-replaced")?;
    let m = std::fs::metadata(p).map_err(|_| "pdf-public-asset-replaced")?;
    if !(3036..=8192).contains(&m.len()) {
        return Err("pdf-public-asset-replaced");
    }
    let mut bytes = Vec::new();
    std::fs::File::open(p)
        .map_err(|_| "pdf-public-asset-replaced")?
        .take(8193)
        .read_to_end(&mut bytes)
        .map_err(|_| "pdf-public-asset-replaced")?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    if bytes.len() as u64 != m.len() || !bytes.starts_with(b"SOLC") || digest != expected {
        return Err("pdf-public-asset-replaced");
    }
    Ok(json!({"kind":"prepared-vault-ciphertext","bytes":bytes.len(),"sha256":digest}))
}
fn asset_intact(c: &Capture) -> Outcome<Value> {
    resource_identity(
        c.config
            .pdf_resource
            .as_ref()
            .ok_or("pdf-prelaunch-resource-unavailable")?,
        c.config
            .pdf_ciphertext_sha256
            .as_deref()
            .ok_or("pdf-prelaunch-ciphertext-binding-unavailable")?,
    )
}

fn stable(samples: &[Value]) -> bool {
    samples.len() >= 2
        && samples[samples.len() - 2..]
            .iter()
            .all(|s| s["matches"] == true && s["frameStable"] == true)
        && samples.last().unwrap()["captureStartedAtMs"]
            .as_f64()
            .unwrap()
            - samples[samples.len() - 2]["verifiedAtMs"].as_f64().unwrap()
            >= INTERVAL_MS as f64
}
fn metrics(samples: &[Value], opening: f64, wait: f64) -> Outcome<Value> {
    if !stable(samples) {
        return Err("pdf-first-page-not-stable");
    }
    let first = samples
        .iter()
        .find(|s| s["matches"] == true && s["frameStable"] == true)
        .unwrap();
    let last = samples.last().unwrap();
    let sum = |end: &str, start: &str| {
        samples
            .iter()
            .map(|s| s[end].as_f64().unwrap() - s[start].as_f64().unwrap())
            .sum::<f64>()
    };
    Ok(
        json!({"firstMatchObservedMs":first["verifiedAtMs"].as_f64().unwrap()-opening,
        "stableReadyObservedMs":last["verifiedAtMs"].as_f64().unwrap()-opening,
        "captureOverheadMs":sum("captureFinishedAtMs","captureStartedAtMs"),
        "decodeOverheadMs":sum("decodeFinishedAtMs","captureFinishedAtMs"),
        "pollWaitMs":wait,"captures":samples.len()}),
    )
}
async fn inspect(c: &mut Capture, pdf: &str) -> Outcome<()> {
    let resource = asset_intact(c)?;
    c.pdf_preview.as_mut().unwrap()["resourceBinding"] = resource;
    let target = dom(c, "target", pdf).await?;
    let expected = c.frame.clone().ok_or("pdf-binding-unavailable")?;
    let tree = c.protocol("Page.getFrameTree", json!({})).await?;
    super::super::parse_frame(&tree, APP)?;
    c.step = "pdf-first-page";
    c.pdf_frame_window.store(true, Ordering::SeqCst);
    let opening = c.started.elapsed().as_secs_f64() * 1000.0;
    c.pdf_preview.as_mut().unwrap()["openingStartedAtMs"] = json!(opening);
    for kind in ["mousePressed", "mouseReleased"] {
        c.protocol("Input.dispatchMouseEvent", json!({"type":kind,"x":target["target"]["x"],"y":target["target"]["y"],"button":"left","clickCount":1})).await?;
    }
    let mut samples = vec![];
    let mut wait = 0.0;
    for index in 0..MAX_CAPTURES {
        if c.started.elapsed().as_secs_f64() * 1000.0 - opening > 25_000.0 {
            return Err("pdf-first-page-timeout");
        }
        if index > 0 {
            let began = c.started.elapsed();
            tokio::time::sleep(Duration::from_millis(INTERVAL_MS)).await;
            wait += (c.started.elapsed() - began).as_secs_f64() * 1000.0;
        }
        asset_intact(c)?;
        let before_dom = dom(c, "opened", pdf).await?;
        let before_raw = c.protocol("Page.getFrameTree", json!({})).await?;
        let before = topology(&before_raw, &expected, pdf)?;
        let t0 = c.started.elapsed().as_secs_f64() * 1000.0;
        let raw = c
            .protocol(
                "Page.captureScreenshot",
                json!({"format":"png","fromSurface":true,"captureBeyondViewport":false}),
            )
            .await?;
        let t1 = c.started.elapsed().as_secs_f64() * 1000.0;
        let encoded = raw["data"]
            .as_str()
            .filter(|s| s.len() <= 350_000)
            .ok_or("pdf-screenshot-invalid")?;
        let bytes = STANDARD
            .decode(encoded)
            .map_err(|_| "pdf-screenshot-invalid")?;
        let state = pixels(&bytes)?;
        let t2 = c.started.elapsed().as_secs_f64() * 1000.0;
        let after_raw = c.protocol("Page.getFrameTree", json!({})).await?;
        let after = topology(&after_raw, &expected, pdf)?;
        // 主 frame/loader 已由两次 topology 严格核验；PDF 加载产生的子树变化
        // 只将本次捕获标为不稳定并保留原件，不得计为就绪。预算与后续双次稳定门禁不变。
        let frame_stable = before == after;
        let after_dom = dom(c, "opened", pdf).await?;
        asset_intact(c)?;
        let verified = c.started.elapsed().as_secs_f64() * 1000.0;
        let name = format!("native-perf-sdk-pdf-first-page-{:02}.png", index + 1);
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(c.config.evidence_root().join(&name))
            .map_err(|_| "pdf-screenshot-publication-failed")?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|_| "pdf-screenshot-publication-failed")?;
        let mut sample = state;
        for (key, value) in [
            ("index", json!(index)),
            ("fileName", json!(name)),
            ("bytes", json!(bytes.len())),
            ("sha256", json!(format!("{:x}", Sha256::digest(&bytes)))),
            ("captureStartedAtMs", json!(t0)),
            ("captureFinishedAtMs", json!(t1)),
            ("decodeFinishedAtMs", json!(t2)),
            ("verifiedAtMs", json!(verified)),
            ("frameStable", json!(frame_stable)),
            ("frameBefore", before),
            ("frameAfter", after),
            ("domBefore", before_dom),
            ("domAfter", after_dom),
        ] {
            sample[key] = value;
        }
        samples.push(sample);
        c.pdf_preview.as_mut().unwrap()["samples"] = json!(samples);
        if stable(&samples) {
            c.pdf_preview.as_mut().unwrap()["performanceMetrics"] =
                metrics(&samples, opening, wait)?;
            c.pdf_preview.as_mut().unwrap()["renderVerified"] = json!(true);
            break;
        }
    }
    if !stable(&samples) {
        return Err("pdf-first-page-not-observed");
    }
    let opened = dom(c, "opened", pdf).await?;
    for kind in ["mousePressed", "mouseReleased"] {
        c.protocol("Input.dispatchMouseEvent",json!({"type":kind,"x":opened["target"]["x"],"y":opened["target"]["y"],"button":"left","clickCount":1})).await?;
    }
    c.pdf_preview.as_mut().unwrap()["closedDom"] = dom(c, "closed", pdf).await?;
    let tree = c.protocol("Page.getFrameTree", json!({})).await?;
    let actual = super::super::parse_frame(&tree, APP)?;
    if actual["mainFrameId"] != expected["mainFrameId"]
        || actual["loaderId"] != expected["loaderId"]
    {
        return Err("pdf-main-frame-replaced");
    }
    c.pdf_preview.as_mut().unwrap()["mainVerifiedAfterClose"] = json!(true);
    c.pdf_preview.as_mut().unwrap()["frameCreatedEvents"] =
        json!(c.pdf_frame_events.load(Ordering::SeqCst));
    Ok(())
}
pub(super) async fn run(c: &mut Capture) -> Outcome<()> {
    if !c.config.pdf_preview
        || c.config.pdf_diagnostic
        || !c.config.media_journey
        || c.config.startup.is_some()
    {
        return Err("pdf-first-page-exclusive-mode-required");
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
    c.pdf_preview = Some(
        json!({"schemaVersion":2,"scope":"windows-native-sdk-public-pdf-first-page","assetSha256":ASSET,"resourceBinding":null,"renderVerified":false,"viewport":{"width":WIDTH,"height":HEIGHT},"roi":{"x":ROI[0],"y":ROI[1],"width":ROI[2],"height":ROI[3]},"referenceMaskSha256":MASK,"threshold":{"alphaMin":240,"rgbMax":64},"maxCaptures":MAX_CAPTURES,"intervalMs":INTERVAL_MS,"requiredConsecutiveMatches":2,"openingStartedAtMs":null,"samples":[],"performanceMetrics":null,"closedDom":null,"mainVerifiedAfterClose":false,"frameCreatedEvents":0}),
    );
    let result = inspect(c, &pdf).await;
    c.pdf_frame_window.store(false, Ordering::SeqCst);
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    const REFERENCE: &[u8] = include_bytes!("fixtures/pdf-first-page.png");
    fn encode(image: image::RgbaImage) -> Vec<u8> {
        let mut out = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut out, ImageFormat::Png)
            .unwrap();
        out.into_inner()
    }
    #[test]
    fn physical_resource_uses_checked_ciphertext_digest_and_rejects_plaintext_or_changes() {
        let temp = tempfile::tempdir().unwrap();
        let p = temp.path().join("text_only.pdf");
        let mut ciphertext = vec![7u8; 3076];
        ciphertext[..5].copy_from_slice(b"SOLC\x02");
        std::fs::write(&p, &ciphertext).unwrap();
        let digest = format!("{:x}", Sha256::digest(&ciphertext));
        assert_eq!(resource_identity(&p, &digest).unwrap()["bytes"], 3076);
        assert!(resource_identity(&p, ASSET).is_err());
        ciphertext[100] ^= 1;
        std::fs::write(&p, &ciphertext).unwrap();
        assert!(resource_identity(&p, &digest).is_err());
        std::fs::write(&p, vec![b' '; 3035]).unwrap();
        assert!(resource_identity(&p, ASSET).is_err());
        std::fs::write(&p, vec![0; 8193]).unwrap();
        assert!(resource_identity(&p, &digest).is_err());
    }
    #[test]
    fn fixed_public_reference_accepts_only_exact_text_mask() {
        let result = pixels(REFERENCE).unwrap();
        assert_eq!(result["matches"], true);
        assert_eq!(result["darkPixels"], 530);
        let original = image::load_from_memory_with_format(REFERENCE, ImageFormat::Png)
            .unwrap()
            .to_rgba8();
        let white = image::Rgba([255, 255, 255, 255]);
        let mut blank = original.clone();
        for y in ROI[1]..ROI[1] + ROI[3] {
            for x in ROI[0]..ROI[0] + ROI[2] {
                blank.put_pixel(x, y, white);
            }
        }
        assert_eq!(pixels(&encode(blank.clone())).unwrap()["matches"], false);
        let mut shifted = blank.clone();
        for y in ROI[1]..ROI[1] + ROI[3] {
            for x in ROI[0]..ROI[0] + ROI[2] - 1 {
                shifted.put_pixel(x + 1, y, *original.get_pixel(x, y));
            }
        }
        assert_eq!(pixels(&encode(shifted)).unwrap()["matches"], false);
        let mut partial = original;
        for y in ROI[1]..ROI[1] + ROI[3] {
            for x in 330..ROI[0] + ROI[2] {
                partial.put_pixel(x, y, white);
            }
        }
        assert_eq!(pixels(&encode(partial)).unwrap()["matches"], false);
    }
    #[test]
    fn pixel_decoder_rejects_corruption_wrong_viewport_and_over_budget() {
        for bytes in [vec![], vec![0; 300_000], REFERENCE[..100].to_vec()] {
            assert!(pixels(&bytes).is_err());
        }
        for (at, value) in [(19, 0), (24, 16), (25, 2), (28, 1), (32, 0)] {
            let mut bad = REFERENCE.to_vec();
            bad[at] = value;
            assert!(pixels(&bad).is_err());
        }
    }
    #[test]
    fn two_matching_observations_require_actual_poll_gap_and_reset_after_mismatch() {
        let s = |matches, t| json!({"matches":matches,"frameStable":true,"captureStartedAtMs":t,"captureFinishedAtMs":t+10.0,"decodeFinishedAtMs":t+12.0,"verifiedAtMs":t+20.0});
        let a = s(true, 100.0);
        assert!(!stable(std::slice::from_ref(&a)));
        assert!(!stable(&[a.clone(), s(true, 200.0)]));
        assert!(!stable(&[a.clone(), s(false, 400.0)]));
        assert!(stable(&[
            a.clone(),
            s(false, 400.0),
            s(true, 700.0),
            s(true, 1000.0)
        ]));
        let mut changing = s(true, 100.0);
        changing["frameStable"] = json!(false);
        assert!(!stable(&[changing.clone(), s(true, 400.0)]));
        assert!(!stable(&[s(true, 100.0), changing.clone()]));
        let retried = metrics(&[changing, s(true, 400.0), s(true, 700.0)], 50.0, 500.0).unwrap();
        assert_eq!(retried["firstMatchObservedMs"], 370.0);
        assert_eq!(retried["stableReadyObservedMs"], 670.0);
        assert_eq!(retried["captureOverheadMs"], 30.0);
        assert_eq!(retried["captures"], 3);
        let m = metrics(&[a, s(true, 400.0)], 50.0, 250.0).unwrap();
        assert_eq!(m["firstMatchObservedMs"], 70.0);
        assert_eq!(m["stableReadyObservedMs"], 370.0);
        assert_eq!(m["captureOverheadMs"], 20.0);
    }
}
