//! Windows 原生外壳保留系统非客户区，确保标题栏按钮、Snap 与 DPI 行为由系统处理。
use super::{calculate_luminance, TitlebarColor, WindowAppearance};
use ::windows::{
    core::{w, BOOL},
    Win32::{
        Graphics::Dwm::{
            DwmSetWindowAttribute, DWMSBT_TRANSIENTWINDOW, DWMWA_CAPTION_COLOR,
            DWMWA_COLOR_DEFAULT, DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_USE_IMMERSIVE_DARK_MODE,
        },
        System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD},
        UI::{
            Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW},
            WindowsAndMessaging::{
                SystemParametersInfoW, SPI_GETCLIENTAREAANIMATION, SPI_GETHIGHCONTRAST,
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
            },
        },
    },
};
use std::sync::atomic::{AtomicU8, Ordering};
use tauri::Emitter;

static ACCESSIBILITY: AtomicU8 = AtomicU8::new(u8::MAX);

fn accessibility() -> (bool, bool, bool) {
    let mut transparency = 1u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    let mut contrast = HIGHCONTRASTW {
        cbSize: std::mem::size_of::<HIGHCONTRASTW>() as u32,
        ..Default::default()
    };
    let mut animation = BOOL(1);
    // SAFETY: 仅查询当前用户系统偏好；输出指针指向正确大小的局部值，调用期间有效。
    unsafe {
        let _ = RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            w!("EnableTransparency"),
            RRF_RT_REG_DWORD,
            None,
            Some((&mut transparency as *mut u32).cast()),
            Some(&mut size),
        );
        let _ = SystemParametersInfoW(
            SPI_GETHIGHCONTRAST,
            contrast.cbSize,
            Some((&mut contrast as *mut HIGHCONTRASTW).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
        let _ = SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some((&mut animation as *mut BOOL).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
    }
    (
        transparency == 0,
        contrast.dwFlags.contains(HCF_HIGHCONTRASTON),
        !animation.as_bool(),
    )
}

pub fn poll_accessibility(app: &tauri::AppHandle) {
    let (transparency, contrast, motion) = accessibility();
    let flags = u8::from(transparency) | (u8::from(contrast) << 1) | (u8::from(motion) << 2);
    if ACCESSIBILITY.swap(flags, Ordering::Relaxed) != flags {
        let _ = app.emit("native-appearance-changed", ());
    }
}

pub fn apply(
    window: &tauri::WebviewWindow,
    color: TitlebarColor,
) -> Result<WindowAppearance, String> {
    let (reduce_transparency, high_contrast, reduce_motion) = accessibility();
    let dark = calculate_luminance(&color) < 128.0;
    // 同步 Tauri 保存的窗口主题，不能只改 DWM 属性：否则窗口仍跟随系统主题，
    // Windows 设置变更时 tao 会将 Mica/标题栏重新写回系统的浅深色。
    // 应用的 system 模式由现有主题轮询解析后传入，此处始终使用已解析的颜色。
    window
        .set_theme(Some(if dark {
            tauri::Theme::Dark
        } else {
            tauri::Theme::Light
        }))
        .map_err(|e| e.to_string())?;
    let hwnd = window.hwnd().map_err(|e| e.to_string())?;
    // 优先使用 Windows 11 22H2 的公开 Desktop Acrylic API。
    // 不调用旧版 Acrylic 的未公开接口，避免 Windows 10/早期 Windows 11 拖动卡顿。
    let material = if reduce_transparency || high_contrast {
        "solid"
    } else {
        // SAFETY: HWND 来自 Tauri；枚举指针与大小符合 DWMWA_SYSTEMBACKDROP_TYPE 契约。
        let acrylic = unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE,
                std::ptr::from_ref(&DWMSBT_TRANSIENTWINDOW).cast(),
                std::mem::size_of_val(&DWMSBT_TRANSIENTWINDOW) as u32,
            )
            .is_ok()
        };
        if acrylic {
            "acrylic"
        } else if window_vibrancy::apply_mica(window, Some(dark)).is_ok() {
            "mica"
        } else {
            "solid"
        }
    };
    if material == "solid" {
        // clear_mica 在支持 SystemBackdrop 的系统上清除同一 DWM 背景，包括 Acrylic。
        let _ = window_vibrancy::clear_mica(window);
    }
    // WebView2 的背景必须也透明，CSS 才能透出 DWM 材质；旧系统与禁用透明度使用实色。
    window
        .set_background_color(Some(tauri::window::Color(
            color.red,
            color.green,
            color.blue,
            if material == "solid" { 255 } else { 0 },
        )))
        .map_err(|e| e.to_string())?;
    // Acrylic 由 DWM 连续绘制到系统标题栏，不能再用实色盖住原生材质。
    // Mica/实色回退沿用主题标题栏；高对比度仍由系统决定颜色。
    let caption = if high_contrast || material == "acrylic" {
        DWMWA_COLOR_DEFAULT
    } else {
        u32::from(color.red) | (u32::from(color.green) << 8) | (u32::from(color.blue) << 16)
    };
    let dark_mode = u32::from(dark);
    // SAFETY: HWND 来自 Tauri，传给 DWM 的两个值均为 API 要求的 DWORD。
    // 旧 Windows 不支持的 DWM 属性可失败，此时保留系统标题栏默认颜色。
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_CAPTION_COLOR,
            (&caption as *const u32).cast(),
            std::mem::size_of::<u32>() as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            (&dark_mode as *const u32).cast(),
            std::mem::size_of::<u32>() as u32,
        );
    }
    Ok(WindowAppearance {
        material,
        platform: "windows",
        reduce_motion,
        high_contrast,
        titlebar_height: 0.0,
        traffic_lights_right: 0.0,
    })
}
