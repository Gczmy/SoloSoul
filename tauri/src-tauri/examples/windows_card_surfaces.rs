//! RF-121 独立 Windows 原生验收。仅生产 Card/CSS 与窗口材质命令；无账户、日志或插件 setup。
//! 使用 native-perf 中已存在的 WebView2 SDK 依赖，只在此测试例程调用只读截图方法。
#[cfg(all(target_os = "windows", feature = "native-perf"))]
mod regression {
    use base64::Engine;
    use serde_json::{json, Value};
    use solo_soul::commands::window::{self, TitlebarColor, WindowAppearance};
    use std::path::{Path, PathBuf};
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use std::time::Duration;
    use tauri::{utils::config::WindowConfig, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
    use webview2_com::{
        CallDevToolsProtocolMethodCompletedHandler,
        Microsoft::Web::WebView2::Win32::{ICoreWebView2Controller2, COREWEBVIEW2_COLOR},
    };
    use windows::{
        core::{w, Interface, PWSTR},
        Win32::{
            Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_SYSTEMBACKDROP_TYPE},
            System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD},
            UI::WindowsAndMessaging::{GetWindowLongPtrW, GWL_STYLE, WS_CAPTION},
        },
    };

    struct Assets(String);
    impl tauri::Assets<tauri::Wry> for Assets {
        fn get(&self, key: &tauri::utils::assets::AssetKey) -> Option<std::borrow::Cow<'_, [u8]>> {
            (key.as_ref() == "/index.html").then_some(std::borrow::Cow::Borrowed(self.0.as_bytes()))
        }
        fn iter(&self) -> Box<tauri::utils::assets::AssetsIter<'_>> {
            Box::new(std::iter::once((
                "index.html".into(),
                self.0.as_bytes().into(),
            )))
        }
        fn csp_hashes(
            &self,
            _: &tauri::utils::assets::AssetKey,
        ) -> Box<dyn Iterator<Item = tauri::utils::assets::CspHash<'_>> + '_> {
            Box::new(std::iter::empty())
        }
    }
    struct Reports(tokio::sync::mpsc::Sender<Value>);
    #[tauri::command]
    fn card_surface_report(report: Value, state: tauri::State<'_, Reports>) {
        // 有界回传只来自独立合成页面。迟到/重复报告不允许使后续样本匹配。
        let _ = state.0.try_send(report);
    }
    fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
        use std::io::Write;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .and_then(|mut file| file.write_all(bytes))
            .map_err(|e| e.to_string())
    }
    fn transparency_enabled() -> Result<bool, String> {
        let mut value = 0u32;
        let mut size = std::mem::size_of::<u32>() as u32;
        // SAFETY: 仅读当前用户偏好，DWORD 输出缓冲区大小正确。
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
                w!("EnableTransparency"),
                RRF_RT_REG_DWORD,
                None,
                Some((&mut value as *mut u32).cast()),
                Some(&mut size),
            )
        };
        if status.0 != 0 {
            return Err(format!(
                "Cannot verify actual transparency preference: {}",
                status.0
            ));
        }
        Ok(value != 0)
    }
    fn native_frame(window: &WebviewWindow) -> Result<Value, String> {
        let hwnd = window.hwnd().map_err(|e| e.to_string())?;
        let mut backdrop = 0u32;
        // SAFETY: HWND 属于独立测试窗口，SYSTEMBACKDROP_TYPE 是官方可读的 DWORD 属性。
        unsafe {
            DwmGetWindowAttribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE,
                (&mut backdrop as *mut u32).cast(),
                std::mem::size_of::<u32>() as u32,
            )
        }
        .map_err(|e| format!("DwmGetWindowAttribute(SYSTEMBACKDROP_TYPE): {e}"))?;
        // CAPTION_COLOR / USE_IMMERSIVE_DARK_MODE 官方只支持 Set，不用错误的 Get 当验收门禁。
        // SAFETY: 只读取本测试窗口的系统 style，不修改全局或其他窗口。
        let caption =
            unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 } & WS_CAPTION.0 == WS_CAPTION.0;
        let size = window.inner_size().map_err(|e| e.to_string())?;
        Ok(
            json!({"backdropType":backdrop, "windowTheme":window.theme().map_err(|e|e.to_string())?, "captionColorReadback":null, "immersiveDarkModeReadback":null, "readbackLimit":"CAPTION_COLOR and USE_IMMERSIVE_DARK_MODE are documented set-only", "systemCaption":caption, "physicalSize":{"width":size.width,"height":size.height}, "scaleFactor":window.scale_factor().map_err(|e|e.to_string())?}),
        )
    }
    async fn webview_state(window: &WebviewWindow) -> Result<Value, String> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        window.with_webview(move |native| {
            let result = (|| -> windows::core::Result<Value> {
                let controller = native.controller().cast::<ICoreWebView2Controller2>()?;
                let mut color = COREWEBVIEW2_COLOR { A: 0, R: 0, G: 0, B: 0 };
                let mut version = PWSTR::null();
                // SAFETY: SDK output pointers are valid for this UI-thread callback.
                unsafe { controller.DefaultBackgroundColor(&mut color)?; }
                let result = unsafe { native.environment().BrowserVersionString(&mut version) };
                let version = webview2_com::take_pwstr(version);
                result?;
                Ok(json!({"background":{"red":color.R,"green":color.G,"blue":color.B,"alpha":color.A},"runtimeVersion":version}))
            })();
            let _ = tx.send(result.map_err(|e|e.to_string()));
        }).map_err(|e|e.to_string())?;
        tokio::time::timeout(Duration::from_secs(5), rx)
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?
    }
    async fn screenshot(window: &WebviewWindow) -> Result<Vec<u8>, String> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        window
            .with_webview(move |native| {
                let callback = CallDevToolsProtocolMethodCompletedHandler::create(Box::new(
                    move |status, text| {
                        let result = status.map(|()| text).map_err(|e| e.to_string());
                        let _ = tx.send(result);
                        Ok(())
                    },
                ));
                // SAFETY: SDK objects/callback used only on their owning WebView2 UI thread.
                let dispatch = unsafe {
                    native.controller().CoreWebView2().and_then(|core| {
                        core.CallDevToolsProtocolMethod(
                            w!("Page.captureScreenshot"),
                            w!(r#"{"format":"png","captureBeyondViewport":false}"#),
                            &callback,
                        )
                    })
                };
                if let Err(error) = dispatch {
                    eprintln!("screenshot-dispatch-failed: {error}");
                }
            })
            .map_err(|e| e.to_string())?;
        let text = tokio::time::timeout(Duration::from_secs(5), rx)
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())??;
        if text.len() > 8 * 1024 * 1024 {
            return Err("Screenshot exceeds bounded response".into());
        }
        let result: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        let data = result["data"].as_str().ok_or("Screenshot data missing")?;
        base64::engine::general_purpose::STANDARD
            .decode(data)
            .map_err(|e| e.to_string())
    }
    fn css_rgba(text: &str) -> Result<[u8; 4], String> {
        let raw = text
            .strip_prefix("rgba(")
            .or_else(|| text.strip_prefix("rgb("))
            .and_then(|s| s.strip_suffix(')'))
            .ok_or("Expected computed RGB color")?;
        let values: Vec<f64> = raw
            .split(',')
            .map(|x| x.trim().parse::<f64>().map_err(|e| e.to_string()))
            .collect::<Result<_, _>>()?;
        if !(3..=4).contains(&values.len()) || values.iter().any(|v| !v.is_finite()) {
            return Err("Invalid computed RGB color".into());
        }
        Ok([
            values[0] as u8,
            values[1] as u8,
            values[2] as u8,
            if values.len() == 4 {
                (values[3] * 255.0).round() as u8
            } else {
                255
            },
        ])
    }
    fn same_appearance(value: &Value, native: &WindowAppearance) -> bool {
        let Some(fields) = value.as_object() else {
            return false;
        };
        // JS JSON.stringify 将 0.0 写成 0。几何值按严格数值比较，其余类型/字段保持精确。
        fields.len() == 6
            && value["material"] == native.material
            && value["platform"] == native.platform
            && value["reduceMotion"] == native.reduce_motion
            && value["highContrast"] == native.high_contrast
            && value["titlebarHeight"].as_f64() == Some(native.titlebar_height)
            && value["trafficLightsRight"].as_f64() == Some(native.traffic_lights_right)
    }
    fn validate(
        report: &Value,
        appearance: &WindowAppearance,
        sample: &str,
        theme: &str,
    ) -> Result<(), String> {
        let require = |ok, text: &str| if ok { Ok(()) } else { Err(text.to_string()) };
        require(report.get("error").is_none(), "No actual paint frames")?;
        require(
            report["sampleId"] == sample
                && report["theme"] == theme
                && report["platform"] == "windows",
            "Report sample identity mismatch",
        )?;
        require(
            same_appearance(&report["appearance"], appearance),
            "Appearance did not come from native host",
        )?;
        require(
            report["visibility"] == "visible" && report["frameCount"].as_u64().unwrap_or(0) >= 2,
            "WebView not visibly painting",
        )?;
        require(report["pageOverflow"] == false, "Page overflow")?;
        require(
            report["media"]["forcedColors"] == appearance.high_contrast,
            "Actual OS/browser high contrast disagree",
        )?;
        if !appearance.high_contrast {
            require(
                css_rgba(
                    report["navigationBackground"]
                        .as_str()
                        .ok_or("Missing navigation color")?,
                )?[3]
                    == 0,
                "Synthetic navigation must expose native background",
            )?;
            require(
                css_rgba(
                    report["contentBackground"]
                        .as_str()
                        .ok_or("Missing content color")?,
                )?[3]
                    == 255,
                "Synthetic content must stay opaque",
            )?;
        }
        let cards = report["cards"]
            .as_array()
            .ok_or("Missing real Card measurements")?;
        require(cards.len() == 2, "Both real Card surfaces required")?;
        for (card, surface) in cards.iter().zip(["default", "floating"]) {
            require(
                card["surface"] == surface && card["visible"] == true && card["overflow"] == false,
                "Card identity, visibility or long content failed",
            )?;
            require(
                card["background"] == card["expectedBackground"],
                "Production surface token mismatch",
            )?;
            require(
                card["color"] != card["background"],
                "Foreground and background equal",
            )?;
            require(
                card["backdropFilter"] == "none" || card["backdropFilter"] == "",
                "Clear Card must not apply backdrop blur",
            )?;
        }
        Ok(())
    }
    fn pixels(report: &Value, png: &[u8]) -> Result<Value, String> {
        let image = image::load_from_memory_with_format(png, image::ImageFormat::Png)
            .map_err(|e| e.to_string())?
            .to_rgba8();
        let width = report["viewport"]["width"]
            .as_f64()
            .filter(|v| *v > 0.0)
            .ok_or("Invalid viewport width")?;
        let height = report["viewport"]["height"]
            .as_f64()
            .filter(|v| *v > 0.0)
            .ok_or("Invalid viewport height")?;
        let mut points = Vec::new();
        for card in report["cards"].as_array().ok_or("Missing cards")? {
            let x = card["pixelSample"]["x"].as_f64().ok_or("Missing x")?;
            let y = card["pixelSample"]["y"].as_f64().ok_or("Missing y")?;
            if !x.is_finite() || !y.is_finite() || x < 0.0 || y < 0.0 || x >= width || y >= height {
                return Err("Card pixel sample is outside visible viewport".into());
            }
            let px = (x * f64::from(image.width()) / width).floor() as u32;
            let py = (y * f64::from(image.height()) / height).floor() as u32;
            let actual = image.get_pixel(px, py).0;
            let expected = css_rgba(
                card["background"]
                    .as_str()
                    .ok_or("Missing computed background")?,
            )?;
            if actual.iter().zip(expected).any(|(a, b)| a.abs_diff(b) > 2) {
                return Err(format!(
                    "Painted pixel mismatch for {}: actual={actual:?} expected={expected:?}",
                    card["surface"]
                ));
            }
            points.push(json!({"surface":card["surface"],"x":px,"y":py,"actual":actual,"expected":expected}));
        }
        Ok(
            json!({"width":image.width(),"height":image.height(),"points":points,"scope":"actual WebView pixels; excludes DWM/non-client pixels"}),
        )
    }
    async fn check(
        window: WebviewWindow,
        mut reports: tokio::sync::mpsc::Receiver<Value>,
        output: PathBuf,
        expected_mode: String,
    ) -> Result<Value, String> {
        let transparency = transparency_enabled()?;
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        tokio::time::sleep(Duration::from_millis(1400)).await;
        let mut results = Vec::new();
        for phase in ["initial", "hide-show", "resize", "minimize-restore"] {
            match phase {
                "hide-show" => {
                    window.hide().map_err(|e| e.to_string())?;
                    tokio::time::sleep(Duration::from_millis(300)).await;
                    window.show().map_err(|e| e.to_string())?;
                }
                "resize" => {
                    window
                        .set_size(tauri::LogicalSize::new(960.0, 720.0))
                        .map_err(|e| e.to_string())?;
                }
                "minimize-restore" => {
                    window.minimize().map_err(|e| e.to_string())?;
                    tokio::time::sleep(Duration::from_millis(400)).await;
                    window.unminimize().map_err(|e| e.to_string())?;
                }
                _ => {}
            }
            window.set_focus().map_err(|e| e.to_string())?;
            tokio::time::sleep(Duration::from_millis(500)).await;
            for theme in ["light", "dark"] {
                let color = if theme == "light" {
                    TitlebarColor {
                        red: 250,
                        green: 250,
                        blue: 248,
                    }
                } else {
                    TitlebarColor {
                        red: 28,
                        green: 28,
                        blue: 30,
                    }
                };
                let appearance = window::set_titlebar_color(window.clone(), color).await?;
                match expected_mode.as_str() {
                    "high-contrast" if !appearance.high_contrast => {
                        return Err("Requested real high-contrast setting is not active".into())
                    }
                    "transparency-off" if transparency => {
                        return Err("Requested real transparency-off setting is not active".into())
                    }
                    "mica" if appearance.material != "mica" => {
                        return Err("Requested actual Mica is unavailable".into())
                    }
                    _ => {}
                }
                if (!transparency || appearance.high_contrast) && appearance.material != "solid" {
                    return Err("Native accessibility fallback did not become solid".into());
                }
                let sample = format!("{phase}-{theme}");
                window
                    .eval(format!(
                        "window.__runCardSample('{theme}', {}, 'windows', '{sample}')",
                        serde_json::to_string(&appearance).map_err(|e| e.to_string())?
                    ))
                    .map_err(|e| e.to_string())?;
                let report = tokio::time::timeout(Duration::from_secs(5), reports.recv())
                    .await
                    .map_err(|e| e.to_string())?
                    .ok_or("Report channel closed")?;
                write_new(
                    &output.join(format!("{sample}-dom.json")),
                    serde_json::to_string_pretty(&report)
                        .map_err(|e| e.to_string())?
                        .as_bytes(),
                )?;
                validate(&report, &appearance, &sample, theme)?;
                let frame = native_frame(&window)?;
                let webview = webview_state(&window).await?;
                write_new(
                    &output.join(format!("{sample}-native.json")),
                    serde_json::to_string_pretty(&json!({"frame":frame,"webview":webview}))
                        .map_err(|e| e.to_string())?
                        .as_bytes(),
                )?;
                let scale = frame["scaleFactor"]
                    .as_f64()
                    .ok_or("Missing native scale")?;
                for dimension in ["width", "height"] {
                    let physical = frame["physicalSize"][dimension]
                        .as_f64()
                        .ok_or("Missing native size")?;
                    let viewport = report["viewport"][dimension]
                        .as_f64()
                        .ok_or("Missing viewport size")?;
                    if (physical / scale - viewport).abs() > 2.0 {
                        return Err("Native client/WebView geometry mismatch".into());
                    }
                }
                if frame["systemCaption"] != true || frame["windowTheme"] != theme {
                    return Err("Native caption or window theme mismatch".into());
                }
                if appearance.material == "mica"
                    && (frame["backdropType"] != 2 || webview["background"]["alpha"] != 0)
                {
                    return Err("Actual DWM Mica or WebView transparency mismatch".into());
                }
                if appearance.material == "solid" && webview["background"]["alpha"] != 255 {
                    return Err("Actual solid WebView fallback not opaque".into());
                }
                let png = screenshot(&window).await?;
                write_new(&output.join(format!("{sample}.png")), &png)?;
                let pixel = pixels(&report, &png)?;
                results.push(json!({"sampleId":sample,"dom":report,"nativeFrame":frame,"webview":webview,"pixels":pixel}));
                println!("PASS: {sample}");
            }
        }
        for pair in results.chunks_exact(2) {
            if pair[0]["dom"]["appearance"]["highContrast"] == false
                && pair[0]["dom"]["cards"][0]["background"]
                    == pair[1]["dom"]["cards"][0]["background"]
            {
                return Err("Light/dark Card surfaces did not change".into());
            }
        }
        if transparency_enabled()? != transparency {
            return Err("System transparency changed during run".into());
        }
        Ok(
            json!({"success":true,"scope":"independent Windows production Card and native window appearance","systemTransparencyEnabled":transparency,"systemSettingsModified":false,"requestedMode":expected_mode,"samples":results,"limitations":["Synthetic navigation/content layout; not full Sidebar/AppBar","No all-text pixel proof or transient restoration black-frame proof","Only actual current host accessibility settings","DWM caption color/immersive dark mode are set-only; no visual titlebar-color proof","Other-platform accessibility matrices still pending"]}),
        )
    }
    pub fn run() -> Result<(), String> {
        let args: Vec<_> = std::env::args().skip(1).collect();
        if args.len() != 4 && args.len() != 6 {
            return Err("Expected --card-fixture ABS_HTML --output NEW_ABS_DIRECTORY [--expect mica|high-contrast|transparency-off]".into());
        }
        if args[0] != "--card-fixture"
            || args[2] != "--output"
            || (args.len() == 6 && args[4] != "--expect")
        {
            return Err("Unexpected regression arguments".into());
        }
        let fixture = PathBuf::from(&args[1]);
        let output = PathBuf::from(&args[3]);
        let mode = if args.len() == 6 {
            args[5].clone()
        } else {
            "current".into()
        };
        if !["current", "mica", "high-contrast", "transparency-off"].contains(&mode.as_str()) {
            return Err("Unknown native setting expectation".into());
        }
        let metadata = std::fs::symlink_metadata(&fixture).map_err(|e| e.to_string())?;
        if !fixture.is_absolute() || !metadata.is_file() || metadata.len() > 2 * 1024 * 1024 {
            return Err("Fixture must be bounded ordinary absolute HTML, no symlink".into());
        }
        let html = std::fs::read_to_string(&fixture).map_err(|e| e.to_string())?;
        if !html.starts_with("<!-- SoloSoul RF-121 generated fixture -->") {
            return Err("Unrecognized fixture generator marker".into());
        }
        if !output.is_absolute() {
            return Err("Output must be absolute and new".into());
        }
        std::fs::create_dir(&output).map_err(|e| e.to_string())?;
        let (tx, rx) = tokio::sync::mpsc::channel(16);
        let mut context = tauri::test::mock_context(Assets(html));
        context.config_mut().identifier = "com.solosoul.windows-card-regression".into();
        let passed = Arc::new(AtomicBool::new(false));
        let checked = passed.clone();
        tauri::Builder::default().manage(Reports(tx)).invoke_handler(tauri::generate_handler![card_surface_report]).setup(move |app| {
            let config:Value=serde_json::from_str(include_str!("../tauri.windows.conf.json"))?;
            let mut config:WindowConfig=serde_json::from_value(config["app"]["windows"][0].clone())?;
            config.label="windows-card-regression".into(); config.title="RF-121 — Windows 独立 Card 验收".into(); config.url=WebviewUrl::App("index.html".into());
            let window=WebviewWindowBuilder::from_config(app,&config)?.data_directory(output.join("webview-profile")).build()?;
            let handle=app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let result=tokio::time::timeout(Duration::from_secs(50),check(window,rx,output.clone(),mode)).await;
                let result=match result {Ok(result)=>result,Err(error)=>Err(error.to_string())};
                let record=match &result {Ok(value)=>value.clone(),Err(error)=>json!({"success":false,"error":error,"performanceSample":false})};
                if let Err(error)=write_new(&output.join("result.json"),serde_json::to_string_pretty(&record).unwrap().as_bytes()) {eprintln!("FAIL: result write: {error}");std::process::exit(1);}
                match result {Ok(_)=>{checked.store(true,Ordering::SeqCst);println!("PASS: 8 real Windows Card samples, DWM readback and captured pixels");handle.exit(0);},Err(error)=>{eprintln!("FAIL: {error}");std::process::exit(1);}}
            });
            Ok(())
        }).run(context).map_err(|e|e.to_string())?;
        if !passed.load(Ordering::SeqCst) {
            return Err("Native regression exited without complete proof".into());
        }
        Ok(())
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn javascript_integer_geometry_round_trip_preserves_native_appearance() {
            let native = WindowAppearance {
                material: "mica",
                platform: "windows",
                reduce_motion: false,
                high_contrast: false,
                titlebar_height: 0.0,
                traffic_lights_right: 0.0,
            };
            let mut actual = serde_json::to_value(&native).unwrap();
            actual["titlebarHeight"] = json!(0);
            actual["trafficLightsRight"] = json!(0);
            assert_ne!(actual, serde_json::to_value(&native).unwrap());
            assert!(same_appearance(&actual, &native));
            for (key, wrong) in [
                ("titlebarHeight", json!("0")),
                ("trafficLightsRight", json!(1)),
                ("material", json!("solid")),
                ("platform", json!("macos")),
                ("reduceMotion", json!(true)),
                ("highContrast", json!(true)),
            ] {
                let mut altered = actual.clone();
                altered[key] = wrong;
                assert!(!same_appearance(&altered, &native), "accepted wrong {key}");
            }
            let mut missing = actual.clone();
            missing.as_object_mut().unwrap().remove("platform");
            assert!(!same_appearance(&missing, &native));
            actual["extra"] = json!(false);
            assert!(!same_appearance(&actual, &native));
        }
        #[test]
        fn computed_color_and_alpha() {
            assert_eq!(css_rgba("rgb(12, 30, 40)").unwrap(), [12, 30, 40, 255]);
            assert_eq!(css_rgba("rgba(0, 0, 0, 0)").unwrap(), [0, 0, 0, 0]);
            assert!(css_rgba("rgb(NaN, 0, 0)").is_err());
        }
        #[test]
        fn screenshot_pixels_reject_wrong_color() {
            let report = json!({"viewport":{"width":20,"height":20},"cards":[{"surface":"default","background":"rgb(0, 0, 0)","pixelSample":{"x":12,"y":12}}]});
            let image = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
                20,
                20,
                image::Rgba([255, 255, 255, 255]),
            ));
            let mut bytes = std::io::Cursor::new(Vec::new());
            image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
            assert!(pixels(&report, bytes.get_ref())
                .unwrap_err()
                .contains("Painted pixel mismatch"));
        }
        #[test]
        fn late_or_missing_report_is_rejected() {
            let appearance = WindowAppearance {
                material: "mica",
                platform: "windows",
                reduce_motion: false,
                high_contrast: false,
                titlebar_height: 0.0,
                traffic_lights_right: 0.0,
            };
            assert!(validate(&json!({"sampleId":"old"}), &appearance, "current", "light").is_err());
            assert!(validate(
                &json!({"error":"paint-frame-timeout"}),
                &appearance,
                "current",
                "light"
            )
            .is_err());
        }
    }
}
#[cfg(all(target_os = "windows", feature = "native-perf"))]
fn main() {
    if let Err(error) = regression::run() {
        eprintln!("FAIL: {error}");
        std::process::exit(1);
    }
}
#[cfg(not(all(target_os = "windows", feature = "native-perf")))]
fn main() {
    eprintln!("This independent regression requires Windows and --features native-perf");
    std::process::exit(1);
}
