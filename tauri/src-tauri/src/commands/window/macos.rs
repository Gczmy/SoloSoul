//! AppKit 操作只在 with_webview / run_on_main_thread 回调中执行。
use super::{calculate_luminance, TitlebarColor, WindowAppearance};
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    NSColor, NSLayoutAttribute, NSLayoutConstraint, NSLayoutRelation, NSView, NSWindow,
    NSWorkspace,
};
use objc2_foundation::NSArray;
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

#[path = "macos_titlebar.rs"]
pub(super) mod titlebar;

// 每个窗口最多安装一个材质视图，主题同步不重复挂载/移动 WebView。
static MATERIALS: Mutex<BTreeMap<String, &'static str>> = Mutex::new(BTreeMap::new());
static ACCESSIBILITY: AtomicU8 = AtomicU8::new(u8::MAX);

/// 原生玻璃保留极小的非零底色，不让窗口进入完全 clear 的合成路径。
/// macOS 26.6 的独立 AppKit 对照中，alpha=0 会在恢复时闪黑，0.001 不闪且玻璃正常。
/// 使用公开 NSColor/NSWindow API；不改变玻璃、阴影或 WindowServer 私有图层设置。
fn apply_window_backing(ns_window: &NSWindow, color: &TitlebarColor, opaque: bool) {
    let background = if opaque {
        NSColor::colorWithRed_green_blue_alpha(
            f64::from(color.red) / 255.0,
            f64::from(color.green) / 255.0,
            f64::from(color.blue) / 255.0,
            1.0,
        )
    } else {
        NSColor::whiteColor().colorWithAlphaComponent(0.001)
    };
    if ns_window.isOpaque() != opaque {
        ns_window.setOpaque(opaque);
    }
    if ns_window.backgroundColor() != background {
        ns_window.setBackgroundColor(Some(&background));
    }
}

/// 网页背景覆盖完整窗口；交通灯避让由独立的标题栏高度处理。
/// 约束只安装一次，不在聚焦或主题同步时改变 WebView 层级。
fn constrain_webview(ns_window: &NSWindow, webview: &NSView) -> Result<(), String> {
    let root = ns_window
        .contentView()
        .ok_or("Window content view unavailable")?;
    let constraints = [
        NSLayoutAttribute::Left,
        NSLayoutAttribute::Right,
        NSLayoutAttribute::Top,
        NSLayoutAttribute::Bottom,
    ]
    .map(|edge| {
        // SAFETY: WebView 已在该 NSWindow 的内容视图中；两侧使用相同的边缘属性。
        unsafe {
            NSLayoutConstraint::constraintWithItem_attribute_relatedBy_toItem_attribute_multiplier_constant(
                webview,
                edge,
                NSLayoutRelation::Equal,
                Some(&root),
                edge,
                1.0,
                0.0,
            )
        }
    });
    webview.setTranslatesAutoresizingMaskIntoConstraints(false);
    NSLayoutConstraint::activateConstraints(&NSArray::from_retained_slice(&constraints));
    titlebar::install(ns_window, &root)?;
    root.layoutSubtreeIfNeeded();
    Ok(())
}

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
    let appearance = NSAppearance::appearanceNamed(appearance_name);
    // 启动后的系统偏好轮询可能重复请求同一外观，避免无变化时触发材质重新合成。
    if ns_window.appearance().as_deref() != appearance.as_deref() {
        ns_window.setAppearance(appearance.as_deref());
    }
    let mut materials = MATERIALS
        .lock()
        .map_err(|_| "Window appearance state unavailable")?;
    let material = match materials.entry(window.label().to_owned()) {
        std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
        std::collections::btree_map::Entry::Vacant(entry) => {
            constrain_webview(ns_window, content)?;
            entry.insert("solid")
        }
    };
    // 尊重系统辅助功能偏好；应用不再提供独立的不透明兼容模式。
    if reduce_transparency || high_contrast {
        if *material == "liquid-glass" {
            clear_liquid_glass(window).map_err(|e| e.to_string())?;
        }
        if *material == "vibrancy" {
            clear_vibrancy(window).map_err(|e| e.to_string())?;
        }
        *material = "solid";
    } else if *material == "solid" {
        // 首次安装及不透明→玻璃切换时，先建立非零底色，再挂载原生材质。
        apply_window_backing(ns_window, &color, false);
        // 库在调用 NSGlassEffectView 前检查 macOS 26；失败使用老系统公开的 Sidebar 材质。
        // 仅在 Wry 的内容容器底层添加背景，不把 WKWebView 移入玻璃视图。
        // 保持 WebView 的父视图与布局约束稳定，避免材质同步改变恢复时的视图层级。
        // 完整窗口使用圆角玻璃，标题栏与内容区共享同一块材质。
        if apply_liquid_glass(window, LiquidGlassOptions::default().radius(16.0)).is_ok() {
            *material = "liquid-glass";
        } else if apply_vibrancy(window, NSVisualEffectMaterial::Sidebar, None, None).is_ok() {
            *material = "vibrancy";
        }
    }
    let opaque = *material == "solid";
    // 主题轮询保持同一底色；材质安装失败时仍恢复真正不透明的回退背景。
    apply_window_backing(ns_window, &color, opaque);
    Ok(WindowAppearance {
        material,
        platform: "macos",
        reduce_motion,
        high_contrast,
        titlebar_height: titlebar::height(ns_window),
        traffic_lights_right: titlebar::traffic_lights_right(ns_window),
    })
}
