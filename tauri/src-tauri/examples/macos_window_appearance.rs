//! macOS 原生窗口回归（需图形会话，不读取 Vault 或用户偏好）。
//! 运行：cargo run -p solo_soul --example macos_window_appearance
//! 追加 -- --manual 可在自动检查后保留窗口，验收台前调度、最小化和全屏切换。
//! --native-only 隐藏 WebView，仅观察原生材质；--unthrottled 禁用测试 WebView 的后台节流。
//! --vibrancy 在回归检查后替换为始终活跃的传统毛玻璃，供前后台切换对照。
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
async fn snapshot(
    window: &tauri::WebviewWindow,
    material: &str,
) -> (usize, usize, usize, [usize; 3]) {
    use objc2_app_kit::{NSColor, NSView, NSWindow, NSWindowButton, NSWindowStyleMask};
    use objc2_foundation::NSPoint;
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
            if !ns_window.styleMask().contains(NSWindowStyleMask::FullScreen) {
                assert!(ns_window.hasShadow(), "普通窗口必须保留原生阴影");
            }
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
                root.bounds(),
                "网页背景必须覆盖包括标题栏在内的完整窗口"
            );
            let subviews = root.subviews();
            let drag_views: Vec<_> = subviews
                .iter()
                .filter(|view| view.tag() == 0x53535442)
                .collect();
            assert_eq!(drag_views.len(), 1, "标题栏拖拽区域只安装一次");
            let drag = &*drag_views[0];
            let layout = root.convertRect_fromView(ns_window.contentLayoutRect(), None);
            let expected_height = root.bounds().size.height - layout.origin.y - layout.size.height;
            assert_eq!(drag.frame().size.width, root.bounds().size.width);
            assert_eq!(drag.frame().size.height, expected_height);
            assert_eq!(drag.frame().origin.y, layout.origin.y + layout.size.height);
            assert!(
                drag.acceptsFirstMouse(None),
                "只有顶部空白带接收后台首次拖拽"
            );
            let buttons = [
                NSWindowButton::CloseButton,
                NSWindowButton::MiniaturizeButton,
                NSWindowButton::ZoomButton,
            ]
            .map(|kind| {
                    let button = ns_window.standardWindowButton(kind).unwrap();
                    let owner = button.window().unwrap();
                    if !ns_window.styleMask().contains(NSWindowStyleMask::FullScreen)
                        && owner.isVisible() && !button.isHiddenOrHasHiddenAncestor() {
                        // 从窗口最外层命中测试，保证透明拖拽视图不会吞掉交通灯点击。
                        // 全屏的交通灯由系统隐藏工具栏管理，不能用普通窗口的可见命中断言。
                    let frame = button.convertRect_toView(button.bounds(), None);
                    let root_frame = root.convertRect_fromView(button.bounds(), Some(&button));
                    assert!((root.bounds().size.height - root_frame.origin.y - root_frame.size.height / 2.0 - expected_height / 2.0).abs() <= 1.0,
                        "交通灯必须由系统居中于整条顶部栏");
                    let point = NSPoint::new(
                        frame.origin.x + frame.size.width / 2.0,
                        frame.origin.y + frame.size.height / 2.0,
                    );
                        let frame_view = unsafe { owner.contentView().unwrap().superview() }.unwrap();
                    let hit = frame_view.hitTest(frame_view.convertPoint_fromView(point, None));
                        assert!(
                            hit.as_ref().is_some_and(|hit| hit.isDescendantOf(&button)),
                            "交通灯必须仍可点击: button={button:?}; hit={hit:?}; frame={frame:?}; window={:?}; style={:?}",
                            ns_window.frame(), ns_window.styleMask()
                        );
                }
                &*button as *const _ as usize
            });
            let backgrounds: Vec<_> = root
                .subviews()
                .iter()
                .filter(|view| &**view != webview && view.tag() != 0x53535442)
                .collect();
            assert_eq!(backgrounds.len(), usize::from(glass));
            let background_id = backgrounds.first().map_or(0, |background| {
                assert_eq!(background.frame(), root.bounds(), "材质必须覆盖完整窗口");
                &**background as *const NSView as usize
            });
            tx.send((
                webview as *const NSView as usize,
                background_id,
                drag as *const NSView as usize,
                buttons,
            ))
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
async fn check_titlebar_controls(window: &tauri::WebviewWindow) {
    use objc2_app_kit::{NSView, NSWindow};
    use objc2_foundation::NSPoint;
    for enabled in [true, false] {
        let regions = if enabled {
            vec![window::TitlebarControlRect {
                x: 400.0,
                y: 0.0,
                width: 80.0,
                height: 52.0,
            }]
        } else {
            vec![]
        };
        window::set_titlebar_controls(window.clone(), regions)
            .await
            .unwrap();
        let (tx, rx) = tokio::sync::oneshot::channel();
        window
            .with_webview(move |native| {
                // SAFETY: 仅在主线程借用测试窗口的原生视图。
                let window = unsafe { &*native.ns_window().cast::<NSWindow>() };
                let webview = unsafe { &*native.inner().cast::<NSView>() };
                let root = window.contentView().unwrap();
                let top = root.bounds().origin.y + root.bounds().size.height;
                // 从最外层开始，避免空原生工具栏遮住网页按钮而根视图测试仍通过。
                let frame_view = unsafe { root.superview() }.unwrap();
                let hit = |x, y| {
                    frame_view
                        .hitTest(
                            frame_view.convertPoint_fromView(NSPoint::new(x, top - y), Some(&root)),
                        )
                        .unwrap()
                };
                for y in [16.0, 40.0, 51.0] {
                    assert_eq!(
                        hit(200.0, y).tag(),
                        0x53535442,
                        "整条顶部栏空白维持原生拖拽"
                    );
                }
                let control_hit = hit(440.0, 26.0);
                if enabled {
                    assert!(
                        control_hit.isDescendantOf(webview),
                        "顶部控件必须穿透原生拖拽层"
                    );
                } else {
                    assert_eq!(control_hit.tag(), 0x53535442, "卸载控件后恢复空白拖拽");
                }
                assert!(
                    hit(200.0, 60.0).isDescendantOf(webview),
                    "标题栏下方禁止扩展原生拖拽"
                );
                tx.send(()).unwrap();
            })
            .unwrap();
        rx.await.unwrap();
    }
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
                let appearance = window::set_titlebar_color(window.clone(), dark).await.unwrap();
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
                    let appearance = window::set_titlebar_color(window.clone(), color).await.unwrap();
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
                check_titlebar_controls(&window).await;
                let normal_titlebar_height = appearance.titlebar_height;
                for fullscreen in [true, false] {
                    window.set_fullscreen(fullscreen).unwrap();
                    tokio::time::sleep(Duration::from_millis(1500)).await;
                    let appearance = window::set_titlebar_color(window.clone(), dark).await.unwrap();
                    assert_eq!(snapshot(&window, appearance.material).await, original);
                    println!("[appearance] fullscreen={fullscreen}; titlebar-height={}; traffic-lights-right={}",
                        appearance.titlebar_height, appearance.traffic_lights_right);
                    assert!(appearance.traffic_lights_right >= 0.0 && appearance.traffic_lights_right <= 96.0,
                        "全屏工具栏不能把交通灯坐标误换算成屏幕偏移");
                    if fullscreen {
                        // 加入原生工具栏后，系统可以保留工具栏，也可以按偏好自动隐藏。
                        assert!(appearance.titlebar_height >= 0.0 && appearance.titlebar_height <= normal_titlebar_height,
                            "全屏不能叠加额外的标题栏高度");
                    } else {
                        assert_eq!(appearance.titlebar_height, normal_titlebar_height, "退出全屏后恢复同一栏高");
                    }
                }
                check_titlebar_controls(&window).await;
                println!("PASS: full-window WebView, native button hit testing, stable WebView/material/drag-view/buttons, theme sync, hide/show, resize, fullscreen, titlebar controls and drag boundary");
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
