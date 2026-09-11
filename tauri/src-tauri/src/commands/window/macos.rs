//! AppKit 操作只在 with_webview / run_on_main_thread 回调中执行。
use super::{calculate_luminance, TitlebarColor, WindowAppearance};
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    NSColor, NSView, NSWindow, NSWorkspace,
};
use std::collections::BTreeMap;
use std::sync::{
    atomic::{AtomicU8, Ordering},
    Mutex,
};
use tauri::Emitter;
use window_vibrancy::{
    apply_liquid_glass, apply_vibrancy, clear_liquid_glass, clear_vibrancy, LiquidGlassOptions,
    NSVisualEffectMaterial,
};

// 每个窗口最多安装一个材质视图，主题同步不重复挂载/移动 WebView。
static MATERIALS: Mutex<BTreeMap<String, &'static str>> = Mutex::new(BTreeMap::new());
static ACCESSIBILITY: AtomicU8 = AtomicU8::new(u8::MAX);

fn accessibility() -> (bool, bool, bool) {
    let workspace = NSWorkspace::sharedWorkspace();
    (
        workspace.accessibilityDisplayShouldReduceTransparency(),
        workspace.accessibilityDisplayShouldIncreaseContrast(),
        workspace.accessibilityDisplayShouldReduceMotion(),
    )
}

pub fn poll_accessibility(app: &tauri::AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let (transparency, contrast, motion) = accessibility();
        let flags = u8::from(transparency) | (u8::from(contrast) << 1) | (u8::from(motion) << 2);
        if ACCESSIBILITY.swap(flags, Ordering::Relaxed) != flags {
            let _ = handle.emit("native-appearance-changed", ());
        }
    });
}

pub fn apply(
    window: &tauri::WebviewWindow,
    native: tauri::webview::PlatformWebview,
    color: TitlebarColor,
) -> Result<WindowAppearance, String> {
    let (reduce_transparency, high_contrast, reduce_motion) = accessibility();
    // SAFETY: PlatformWebview 在主线程回调中提供有效 NSWindow/WKWebView；仅在该回调内借用。
    let ns_window = unsafe { &*native.ns_window().cast::<NSWindow>() };
    let content = unsafe { &*native.inner().cast::<NSView>() };
    let appearance_name = unsafe {
        if calculate_luminance(&color) < 128.0 {
            NSAppearanceNameDarkAqua
        } else {
            NSAppearanceNameAqua
        }
    };
    ns_window.setAppearance(NSAppearance::appearanceNamed(appearance_name).as_deref());
    let mut materials = MATERIALS
        .lock()
        .map_err(|_| "Window appearance state unavailable")?;
    let material = materials
        .entry(window.label().to_owned())
        .or_insert("solid");
    if reduce_transparency || high_contrast {
        if *material == "liquid-glass" {
            clear_liquid_glass(window).map_err(|e| e.to_string())?;
        }
        if *material == "vibrancy" {
            clear_vibrancy(window).map_err(|e| e.to_string())?;
        }
        *material = "solid";
    } else if *material == "solid" {
        // 库在调用 NSGlassEffectView 前检查 macOS 26；失败使用老系统公开的 Sidebar 材质。
        if apply_liquid_glass(window, LiquidGlassOptions::default().content_view(content)).is_ok() {
            *material = "liquid-glass";
        } else if apply_vibrancy(window, NSVisualEffectMaterial::Sidebar, None, None).is_ok() {
            *material = "vibrancy";
        }
    }
    ns_window.setOpaque(*material == "solid");
    let background = NSColor::colorWithRed_green_blue_alpha(
        f64::from(color.red) / 255.0,
        f64::from(color.green) / 255.0,
        f64::from(color.blue) / 255.0,
        if *material == "solid" { 1.0 } else { 0.0 },
    );
    ns_window.setBackgroundColor(Some(&background));
    Ok(WindowAppearance {
        material: *material,
        platform: "macos",
        reduce_motion,
        high_contrast,
    })
}
