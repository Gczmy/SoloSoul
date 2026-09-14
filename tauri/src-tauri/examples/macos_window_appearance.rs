//! macOS 原生窗口回归（需图形会话，不读取 Vault 或用户偏好）。
//! 运行：cargo run -p solo_soul --example macos_window_appearance
//! 追加 -- --manual 可在自动检查后保留窗口，验收台前调度、最小化和全屏切换。
//! --native-only 隐藏 WebView，仅观察原生材质；--unthrottled 禁用测试 WebView 的后台节流。
//! --vibrancy 在回归检查后替换为始终活跃的传统毛玻璃，供前后台切换对照。
//! --solid 使用生产不透明兼容模式；所有模式都检查透明/不透明往返切换。
//! 玻璃底色 alpha=0.001 用于修复后台恢复闪黑，必须跨主题/尺寸/材质切换保留。
//! 几何断言只验证恢复后的布局，不证明恢复过程没有瞬间黑帧。

#[cfg(target_os = "macos")]
use solo_soul::commands::window;

#[cfg(target_os = "macos")]
struct RegressionAssets;

#[cfg(target_os = "macos")]
const HTML: &str = r#"<!doctype html><meta charset="utf-8"><style>
    html, body { margin: 0; background: transparent; }
    body { height: 100vh; display: grid; place-items: center; color: #eee; font: 16px system-ui; }
    main { padding: 64px; background: #292929; border-radius: 20px; text-align: center; }
    </style><main><h1>SoloSoul</h1><p>macOS window appearance regression</p>
    <p>Liquid Glass · title bar · window restore</p></main>"#;

#[cfg(target_os = "macos")]
impl tauri::Assets<tauri::Wry> for RegressionAssets {
    fn get(&self, _: &tauri::utils::assets::AssetKey) -> Option<std::borrow::Cow<'_, [u8]>> {
        Some(std::borrow::Cow::Borrowed(HTML.as_bytes()))
    }

    fn iter(&self) -> Box<tauri::utils::assets::AssetsIter<'_>> {
        Box::new(std::iter::once((
            "index.html".into(),
            HTML.as_bytes().into(),
        )))
    }

    fn csp_hashes(
        &self,
        _: &tauri::utils::assets::AssetKey,
    ) -> Box<dyn Iterator<Item = tauri::utils::assets::CspHash<'_>> + '_> {
        Box::new(std::iter::empty())
    }
}

#[cfg(target_os = "macos")]
async fn snapshot(window: &tauri::WebviewWindow, material: &str) -> (usize, usize) {
    use objc2_app_kit::{NSColor, NSView, NSWindow};
    let glass = material != "solid";
    let (tx, rx) = tokio::sync::oneshot::channel();
    window
        .with_webview(move |native| {
            // SAFETY: 仅在 Tauri 的主线程回调中借用原生窗口和 WebView。
            let ns_window = unsafe { &*native.ns_window().cast::<NSWindow>() };
            let webview = unsafe { &*native.inner().cast::<NSView>() };
            let root = ns_window.contentView().unwrap();
            assert_eq!(ns_window.isOpaque(), !glass);
            assert_eq!(
                ns_window.backgroundColor().alphaComponent(),
                if glass { 0.001 } else { 1.0 },
                "玻璃不能被主题同步或窗口恢复改回全透明底色"
            );
            assert!(ns_window.hasShadow(), "修复不得移除原生窗口阴影");
            if glass {
                assert_eq!(
                    ns_window.backgroundColor(),
                    NSColor::whiteColor().colorWithAlphaComponent(0.001),
                    "保持与独立 AppKit 验证一致的微量白色底色"
                );
            }
            // SAFETY: 父视图由同一存活 NSWindow 持有，且当前正在主线程读取。
            assert_eq!(unsafe { webview.superview() }.as_deref(), Some(&*root));
            assert_eq!(root.frame().size.height, ns_window.frame().size.height);
            assert_eq!(
                webview.frame(),
                root.convertRect_fromView(ns_window.contentLayoutRect(), None),
                "WebView 必须留在标题栏下方的可用内容区"
            );
            let backgrounds: Vec<_> = root
                .subviews()
                .iter()
                .filter(|view| &**view != webview)
                .collect();
            assert_eq!(backgrounds.len(), usize::from(glass));
            let background_id = backgrounds.first().map_or(0, |background| {
                assert_eq!(background.frame(), root.bounds(), "材质必须覆盖完整窗口");
                &**background as *const NSView as usize
            });
            tx.send((webview as *const NSView as usize, background_id))
                .unwrap();
        })
        .unwrap();
    rx.await.unwrap()
}

#[cfg(target_os = "macos")]
async fn install_vibrancy(window: &tauri::WebviewWindow) {
    use window_vibrancy::{
        apply_vibrancy, clear_liquid_glass, clear_vibrancy, NSVisualEffectMaterial,
        NSVisualEffectState,
    };
    let target = window.clone();
    let (tx, rx) = tokio::sync::oneshot::channel();
    window
        .with_webview(move |_| {
            // 仅用于诊断：替换后不再调用生产外观同步，避免与生产材质缓存不一致。
            let result = (|| {
                clear_liquid_glass(&target)?;
                clear_vibrancy(&target)?;
                apply_vibrancy(
                    &target,
                    NSVisualEffectMaterial::Sidebar,
                    Some(NSVisualEffectState::Active),
                    Some(16.0),
                )
            })();
            tx.send(result.map_err(|error| error.to_string())).unwrap();
        })
        .unwrap();
    rx.await.unwrap().unwrap();
}

#[cfg(target_os = "macos")]
fn main() {
    use std::time::{Duration, Instant};
    use tauri::{
        utils::config::{BackgroundThrottlingPolicy, WindowConfig},
        WebviewUrl, WebviewWindowBuilder,
    };
    let manual = std::env::args().any(|arg| arg == "--manual");
    let native_only = std::env::args().any(|arg| arg == "--native-only");
    let unthrottled = std::env::args().any(|arg| arg == "--unthrottled");
    let vibrancy = std::env::args().any(|arg| arg == "--vibrancy");
    let solid = std::env::args().any(|arg| arg == "--solid");
    let mut context = tauri::test::mock_context(RegressionAssets);
    context.config_mut().identifier = "com.solosoul.appearance-regression".into();
    context.config_mut().app.macos_private_api = true;
    tauri::Builder::default()
        .setup(move |app| {
            // 使用生产 macOS 配置，避免测试里的标题栏配置与发布包分离。
            let config: serde_json::Value =
                serde_json::from_str(include_str!("../tauri.macos.conf.json"))?;
            let mut config: WindowConfig =
                serde_json::from_value(config["app"]["windows"][0].clone())?;
            config.label = "appearance-regression".into();
            config.title = "SoloSoul — macOS appearance regression".into();
            config.url = WebviewUrl::App("index.html".into());
            if unthrottled {
                config.background_throttling = Some(BackgroundThrottlingPolicy::Disabled);
            }
            let window = WebviewWindowBuilder::from_config(app, &config)?
                .build()?;
            let started = Instant::now();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::Focused(focused) = event {
                    println!("[appearance +{:?}] focused={focused}; native-only={native_only}; unthrottled={unthrottled}", started.elapsed());
                }
            });
            let app = app.handle().clone();
            let checks = tauri::async_runtime::spawn(async move {
                let dark = window::TitlebarColor { red: 28, green: 28, blue: 30 };
                let light = window::TitlebarColor { red: 250, green: 250, blue: 248 };
                let appearance = window::set_titlebar_color(window.clone(), dark, Some(solid)).await.unwrap();
                println!("[appearance] material={}; native-only={native_only}; unthrottled={unthrottled}", appearance.material);
                if native_only {
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    window.with_webview(move |native| {
                        // SAFETY: 测试窗口的 WebView 在主线程回调中有效；隐藏不改变其父视图。
                        let webview = unsafe { &*native.inner().cast::<objc2_app_kit::NSView>() };
                        webview.setHidden(true);
                        tx.send(()).unwrap();
                    }).unwrap();
                    rx.await.unwrap();
                }
                window.show().unwrap();
                tokio::time::sleep(Duration::from_millis(1200)).await;
                let original = snapshot(&window, appearance.material).await;
                // 覆盖首帧后约一秒的重复外观同步，以及浅深色切换。
                for color in [dark, dark, light, dark] {
                    let appearance = window::set_titlebar_color(window.clone(), color, Some(solid)).await.unwrap();
                    tokio::time::sleep(Duration::from_millis(300)).await;
                    assert_eq!(snapshot(&window, appearance.material).await, original);
                }
                for (width, height) in [(960.0, 640.0), (1200.0, 800.0)] {
                    window.hide().unwrap();
                    window.set_size(tauri::LogicalSize::new(width, height)).unwrap();
                    window.show().unwrap();
                    tokio::time::sleep(Duration::from_millis(1200)).await;
                    assert_eq!(snapshot(&window, appearance.material).await, original);
                }
                println!("PASS: native title bar coverage, content layout, stable WebView/material identity, theme sync, hide/show and resize");
                for opaque in [true, false, true, solid] {
                    let mode = window::set_titlebar_color(window.clone(), dark, Some(opaque)).await.unwrap();
                    if opaque { assert_eq!(mode.material, "solid"); }
                    window.hide().unwrap();
                    window.show().unwrap();
                    tokio::time::sleep(Duration::from_millis(300)).await;
                    let current = snapshot(&window, mode.material).await;
                    assert_eq!(current.0, original.0, "切换兼容模式不得重建 WebView");
                    window::set_titlebar_color(window.clone(), dark, Some(opaque)).await.unwrap();
                    assert_eq!(snapshot(&window, mode.material).await, current);
                }
                println!("PASS: glass/opaque transitions, native opacity and background alpha, stable WebView and layout");
                if vibrancy {
                    if appearance.material == "solid" {
                        println!("SKIP: vibrancy comparison respects accessibility settings requiring a solid background");
                    } else {
                        install_vibrancy(&window).await;
                        window.set_title("SoloSoul — 传统毛玻璃对照（始终活跃）").unwrap();
                        tokio::time::sleep(Duration::from_millis(300)).await;
                        let comparison = snapshot(&window, "vibrancy").await;
                        assert_eq!(comparison.0, original.0, "替换材质不得重建 WebView");
                        window.hide().unwrap();
                        window.show().unwrap();
                        tokio::time::sleep(Duration::from_millis(1200)).await;
                        assert_eq!(snapshot(&window, "vibrancy").await, comparison);
                        println!("READY: traditional vibrancy (Sidebar, always active); native-only={native_only}. Compare background restoration via Dock and Stage Manager.");
                    }
                }
            });
            tauri::async_runtime::spawn(async move {
                match tokio::time::timeout(Duration::from_secs(30), checks).await {
                    Ok(Ok(())) => {
                        if !manual { app.exit(0); }
                    }
                    failure => {
                        eprintln!("FAIL: native window regression did not complete: {failure:?}");
                        app.exit(1);
                    }
                }
            });
            Ok(())
        })
        .run(context)
        .expect("macOS appearance regression app failed");
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("此原生回归示例仅支持 macOS 图形会话。");
}
