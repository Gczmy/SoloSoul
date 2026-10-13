//! Opt-in Windows caption extension. Call only on the owning UI thread.
//! DWM owns caption buttons; the renderer supplies only bounded interactive rectangles.
use super::{TitlebarControlRect, WindowsFrameLayout};
use std::cell::RefCell;
use windows::core::BOOL;
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::{
        Dwm::{
            DwmDefWindowProc, DwmExtendFrameIntoClientArea, DwmGetWindowAttribute,
            DWMWA_CAPTION_BUTTON_BOUNDS,
        },
        Gdi::{
            ClientToScreen, CombineRgn, CreateRectRgn, DeleteObject, GetWindowRgn, RedrawWindow,
            ScreenToClient, SetWindowRgn, HGDIOBJ, RDW_FRAME, RDW_INVALIDATE, RDW_UPDATENOW,
            RGN_DIFF,
        },
    },
    UI::{
        Controls::MARGINS,
        HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi},
        Shell::{DefSubclassProc, GetWindowSubclass, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::*,
    },
};

const SUBCLASS_ID: usize = 0x5353_1142;
const HOST_SUBCLASS_ID: usize = 0x5353_1143;
const FRAME_SYNC: u32 = WM_APP + 0x1142;
pub const HEADER_HEIGHT: f64 = 52.0;

struct State {
    controls: Vec<TitlebarControlRect>,
    webview: HWND,
}

fn webview_child(hwnd: HWND) -> Result<HWND, String> {
    struct Search {
        parent: HWND,
        found: Vec<HWND>,
    }
    unsafe extern "system" fn collect(child: HWND, data: LPARAM) -> BOOL {
        // SAFETY: EnumChildWindows synchronously borrows this UI-thread search.
        unsafe {
            let search = &mut *(data.0 as *mut Search);
            let mut name = [0u16; 64];
            let length = GetClassNameW(child, &mut name);
            if GetParent(child).ok() == Some(search.parent)
                && String::from_utf16_lossy(&name[..length.max(0) as usize]) == "WRY_WEBVIEW"
            {
                search.found.push(child);
            }
            BOOL(1)
        }
    }
    let mut search = Search {
        parent: hwnd,
        found: Vec::new(),
    };
    // SAFETY: Enumerates only descendants of the retained owned window.
    unsafe {
        let _ = EnumChildWindows(
            Some(hwnd),
            Some(collect),
            LPARAM((&mut search as *mut Search) as isize),
        );
    }
    if search.found.len() != 1 {
        return Err("Expected one owned Wry WebView host".into());
    }
    let child = search.found[0];
    // Refuse an existing region; it belongs to another adapter and cannot be discarded.
    unsafe {
        let region = CreateRectRgn(0, 0, 0, 0);
        if region.0.is_null() {
            return Err("Cannot allocate WebView region probe".into());
        }
        let previous = GetWindowRgn(child, region);
        let _ = DeleteObject(HGDIOBJ(region.0));
        if previous.0 != 0 {
            return Err("WebView host already has a window region".into());
        }
    }
    Ok(child)
}

fn clip_caption(hwnd: HWND) -> Result<(), String> {
    let Some(pointer) = state(hwnd) else {
        return Ok(());
    };
    // No OS call while the cookie is borrowed: SetWindowRgn can dispatch messages.
    let child = unsafe { &*pointer }
        .try_borrow()
        .map_err(|_| "Frame is busy")?
        .webview;
    // SAFETY: The cookie identifies only the retained owned Wry host; temporary
    // GDI regions are deleted on failure or transferred to Windows on success.
    unsafe {
        if !IsWindow(Some(child)).as_bool() {
            return Err("WebView host no longer exists".into());
        }
        if GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 & WS_CAPTION.0 == 0 {
            if SetWindowRgn(child, None, true) == 0 {
                return Err("Cannot restore full-screen WebView region".into());
            }
            return Ok(());
        }
        let mut caption = RECT::default();
        let mut parent = RECT::default();
        let mut host = RECT::default();
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CAPTION_BUTTON_BOUNDS,
            (&mut caption as *mut RECT).cast(),
            std::mem::size_of::<RECT>() as u32,
        )
        .map_err(|e| e.to_string())?;
        GetWindowRect(hwnd, &mut parent).map_err(|e| e.to_string())?;
        GetWindowRect(child, &mut host).map_err(|e| e.to_string())?;
        let top_resize = if IsZoomed(hwnd).as_bool() {
            0
        } else {
            let dpi = GetDpiForWindow(hwnd);
            GetSystemMetricsForDpi(SM_CYFRAME, dpi) + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi)
        };
        let region = CreateRectRgn(
            0,
            top_resize,
            host.right - host.left,
            host.bottom - host.top,
        );
        let buttons = CreateRectRgn(
            caption.left + parent.left - host.left,
            caption.top + parent.top - host.top,
            caption.right + parent.left - host.left,
            caption.bottom + parent.top - host.top,
        );
        if region.0.is_null() || buttons.0.is_null() {
            if !region.0.is_null() {
                let _ = DeleteObject(HGDIOBJ(region.0));
            }
            if !buttons.0.is_null() {
                let _ = DeleteObject(HGDIOBJ(buttons.0));
            }
            return Err("Cannot allocate caption exclusion region".into());
        }
        let combined = CombineRgn(Some(region), Some(region), Some(buttons), RGN_DIFF);
        let _ = DeleteObject(HGDIOBJ(buttons.0));
        if combined.0 == 0 || SetWindowRgn(child, Some(region), true) == 0 {
            let _ = DeleteObject(HGDIOBJ(region.0));
            return Err("Cannot exclude native caption buttons from WebView".into());
        }
        // Windows owns the region and clips foreign renderer descendants too.
    }
    Ok(())
}

pub fn enabled(hwnd: HWND) -> bool {
    state(hwnd).is_some()
}

fn state(hwnd: HWND) -> Option<*mut RefCell<State>> {
    let mut data = 0usize;
    // SAFETY: Only retrieves the cookie belonging to this module's subclass/id.
    unsafe { GetWindowSubclass(hwnd, Some(procedure), SUBCLASS_ID, Some(&mut data)) }
        .as_bool()
        .then_some(data as *mut RefCell<State>)
}

pub fn layout(hwnd: HWND) -> Result<WindowsFrameLayout, String> {
    // SAFETY: Caller retains the owning window and executes on its UI thread.
    unsafe {
        let dpi = GetDpiForWindow(hwnd);
        if dpi == 0 {
            return Err("Cannot read window DPI".into());
        }
        let caption_visible = (GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 & WS_CAPTION.0) != 0;
        if !caption_visible {
            return Ok(WindowsFrameLayout {
                enabled: state(hwnd).is_some(),
                header_height: 0.0,
                dpi,
                maximized: IsZoomed(hwnd).as_bool(),
                caption_bounds: [0.0; 4],
            });
        }
        let mut bounds = RECT::default();
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CAPTION_BUTTON_BOUNDS,
            (&mut bounds as *mut RECT).cast(),
            std::mem::size_of::<RECT>() as u32,
        )
        .map_err(|e| e.to_string())?;
        let mut window = RECT::default();
        GetWindowRect(hwnd, &mut window).map_err(|e| e.to_string())?;
        let mut origin = POINT::default();
        if !ClientToScreen(hwnd, &mut origin).as_bool() {
            return Err("Cannot locate client origin".into());
        }
        let scale = f64::from(dpi) / 96.0;
        Ok(WindowsFrameLayout {
            enabled: state(hwnd).is_some(),
            header_height: HEADER_HEIGHT,
            dpi,
            maximized: IsZoomed(hwnd).as_bool(),
            caption_bounds: [
                f64::from(bounds.left + window.left - origin.x) / scale,
                f64::from(bounds.top + window.top - origin.y) / scale,
                f64::from(bounds.right - bounds.left) / scale,
                f64::from(bounds.bottom - bounds.top) / scale,
            ],
        })
    }
}

pub fn configure(
    hwnd: HWND,
    enabled: Option<bool>,
    controls: Vec<TitlebarControlRect>,
) -> Result<(), String> {
    if enabled.is_none() && state(hwnd).is_none() {
        return Ok(());
    }
    if controls.len() > 128
        || controls.iter().any(|r| {
            ![r.x, r.y, r.width, r.height].iter().all(|v| v.is_finite())
                || r.x < 0.0
                || r.y < 0.0
                || r.width < 0.0
                || r.height < 0.0
                || !(r.x + r.width).is_finite()
                || !(r.y + r.height).is_finite()
                || r.y + r.height > HEADER_HEIGHT
        })
    {
        return Err("Invalid Windows titlebar control regions".into());
    }
    let old = state(hwnd);
    if enabled == Some(false) {
        if let Some(pointer) = old {
            let child = unsafe { &*pointer }
                .try_borrow()
                .map_err(|_| "Frame is busy")?
                .webview;
            // SAFETY: Unique cookie is removed before freeing it; no callback can retain it.
            unsafe {
                if !RemoveWindowSubclass(hwnd, Some(procedure), SUBCLASS_ID).as_bool() {
                    return Err("Cannot remove Windows frame subclass".into());
                }
                drop(Box::from_raw(pointer));
                let _ = RemoveWindowSubclass(child, Some(host_procedure), HOST_SUBCLASS_ID);
                if IsWindow(Some(child)).as_bool() && SetWindowRgn(child, None, true) == 0 {
                    return Err("Cannot restore original WebView region".into());
                }
                DwmExtendFrameIntoClientArea(hwnd, &MARGINS::default())
                    .map_err(|e| e.to_string())?;
                refresh(hwnd)?;
            }
        }
        return Ok(());
    }
    if let Some(pointer) = old {
        // SAFETY: The cookie is live on this UI thread; no OS call while borrowed.
        unsafe { &*pointer }
            .try_borrow_mut()
            .map_err(|_| "Frame is busy")?
            .controls = controls;
        return Ok(());
    }
    if enabled != Some(true) {
        return Ok(());
    }
    // Validate native geometry before touching the frame. Unsupported hosts fail closed.
    let _ = layout(hwnd)?;
    let webview = webview_child(hwnd)?;
    let pointer = Box::into_raw(Box::new(RefCell::new(State { controls, webview })));
    // SAFETY: Cookie ownership passes to the subclass; WM_NCDESTROY releases it once.
    unsafe {
        if !SetWindowSubclass(hwnd, Some(procedure), SUBCLASS_ID, pointer as usize).as_bool() {
            drop(Box::from_raw(pointer));
            return Err("Cannot install Windows frame subclass".into());
        }
        if !SetWindowSubclass(
            webview,
            Some(host_procedure),
            HOST_SUBCLASS_ID,
            hwnd.0 as usize,
        )
        .as_bool()
        {
            let _ = configure(hwnd, Some(false), Vec::new());
            return Err("Cannot observe WebView host resize".into());
        }
        if let Err(error) = extend(hwnd)
            .and_then(|()| refresh(hwnd))
            .and_then(|()| clip_caption(hwnd))
        {
            let _ = configure(hwnd, Some(false), Vec::new());
            return Err(error);
        }
    }
    Ok(())
}

pub(super) fn refresh_appearance(hwnd: HWND) -> Result<(), String> {
    extend(hwnd)?;
    // SAFETY: Called on the retained window's UI thread after its theme has changed.
    unsafe { refresh(hwnd)? };
    clip_caption(hwnd)?;
    // Repaint the native frame without moving/resizing or reinstalling its subclass.
    unsafe {
        if !RedrawWindow(
            Some(hwnd),
            None,
            None,
            RDW_INVALIDATE | RDW_FRAME | RDW_UPDATENOW,
        )
        .as_bool()
        {
            return Err("Cannot redraw Windows caption".into());
        }
    }
    Ok(())
}

unsafe fn refresh(hwnd: HWND) -> Result<(), String> {
    // SAFETY: Valid owned HWND; frame recalculation only, preserves placement and z order.
    unsafe {
        SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        )
    }
    .map_err(|e| e.to_string())
}

fn extend(hwnd: HWND) -> Result<(), String> {
    // SAFETY: DWM receives a bounded, DPI-scaled margin; no global setting changes.
    unsafe {
        let height = (HEADER_HEIGHT * f64::from(GetDpiForWindow(hwnd)) / 96.0).round() as i32;
        DwmExtendFrameIntoClientArea(
            hwnd,
            &MARGINS {
                cyTopHeight: height,
                ..Default::default()
            },
        )
        .map_err(|e| e.to_string())
    }
}

fn screen_point(value: isize) -> POINT {
    POINT {
        x: (value as u16 as i16) as i32,
        y: ((value >> 16) as u16 as i16) as i32,
    }
}

unsafe extern "system" fn procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    // SAFETY: comctl32 calls this only while the installed cookie and HWND are live.
    unsafe {
        if message == FRAME_SYNC {
            if clip_caption(hwnd).is_err() {
                let _ = configure(hwnd, Some(false), Vec::new());
            }
            return LRESULT(0);
        }
        if message == WM_NCDESTROY {
            let child = (&*(data as *const RefCell<State>))
                .try_borrow()
                .ok()
                .map(|s| s.webview);
            let _ = RemoveWindowSubclass(hwnd, Some(procedure), SUBCLASS_ID);
            if let Some(child) = child {
                if IsWindow(Some(child)).as_bool() {
                    let _ = RemoveWindowSubclass(child, Some(host_procedure), HOST_SUBCLASS_ID);
                    let _ = SetWindowRgn(child, None, false);
                }
            }
            drop(Box::from_raw(data as *mut RefCell<State>));
            return DefSubclassProc(hwnd, message, wparam, lparam);
        }
        let caption_visible = (GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 & WS_CAPTION.0) != 0;
        if caption_visible {
            // DWM owns the complete caption mouse sequence, not just hit testing.
            // Forward hover, press and release so restore/minimize/close and Snap work.
            let mut result = LRESULT(0);
            let handled = DwmDefWindowProc(hwnd, message, wparam, lparam, &mut result).as_bool();
            if message == WM_NCHITTEST {
                let mut point = screen_point(lparam.0);
                let _ = ScreenToClient(hwnd, &mut point);
                if let Ok(geometry) = layout(hwnd) {
                    let scale = f64::from(geometry.dpi) / 96.0;
                    let inset = (GetSystemMetricsForDpi(SM_CXFRAME, geometry.dpi)
                        + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, geometry.dpi))
                        as f64
                        / scale;
                    if !geometry.maximized && point.y >= 0 && f64::from(point.y) / scale < inset {
                        let mut client = RECT::default();
                        let hit = if GetClientRect(hwnd, &mut client).is_ok() {
                            if f64::from(point.x) / scale < inset {
                                HTTOPLEFT
                            } else if f64::from(client.right - point.x) / scale <= inset {
                                HTTOPRIGHT
                            } else {
                                HTTOP
                            }
                        } else {
                            HTTOP
                        };
                        return LRESULT(hit as isize);
                    }
                    if let Some(hit) = caption_hit(
                        geometry.caption_bounds,
                        inset,
                        f64::from(point.x) / scale,
                        f64::from(point.y) / scale,
                    ) {
                        // DWM can decline caption hit testing after the client is
                        // extended, particularly maximized. Keep native button messages.
                        return LRESULT(hit as isize);
                    }
                }
            }
            if handled {
                return result;
            }
        }
        if caption_visible && message == WM_NCCALCSIZE && wparam.0 != 0 && lparam.0 != 0 {
            let params = lparam.0 as *mut NCCALCSIZE_PARAMS;
            let original_top = (*params).rgrc[0].top;
            // Preserve native left/right/bottom resize borders and maximized monitor insets.
            let result = DefSubclassProc(hwnd, message, wparam, lparam);
            let inset = if IsZoomed(hwnd).as_bool() {
                let dpi = GetDpiForWindow(hwnd);
                GetSystemMetricsForDpi(SM_CYFRAME, dpi)
                    + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi)
            } else {
                0
            };
            (*params).rgrc[0].top = original_top + inset;
            return result;
        }
        if caption_visible && message == WM_NCHITTEST {
            let native = DefSubclassProc(hwnd, message, wparam, lparam);
            if matches!(
                native.0 as u32,
                HTLEFT
                    | HTRIGHT
                    | HTTOP
                    | HTBOTTOM
                    | HTTOPLEFT
                    | HTTOPRIGHT
                    | HTBOTTOMLEFT
                    | HTBOTTOMRIGHT
            ) {
                return native;
            }
            let mut point = screen_point(lparam.0);
            let _ = ScreenToClient(hwnd, &mut point);
            let scale = f64::from(GetDpiForWindow(hwnd)) / 96.0;
            let x = f64::from(point.x) / scale;
            let y = f64::from(point.y) / scale;
            if (0.0..HEADER_HEIGHT).contains(&y) {
                let cell = &*(data as *const RefCell<State>);
                if let Ok(value) = cell.try_borrow() {
                    if value
                        .controls
                        .iter()
                        .any(|r| x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height)
                    {
                        return LRESULT(HTCLIENT as isize);
                    }
                } else {
                    return LRESULT(HTCLIENT as isize);
                }
                return LRESULT(HTCAPTION as isize);
            }
            return native;
        }
        let result = DefSubclassProc(hwnd, message, wparam, lparam);
        if message == WM_ACTIVATE || message == WM_DPICHANGED || message == WM_DWMCOMPOSITIONCHANGED
        {
            let _ = extend(hwnd);
        }
        if ((message == WM_SIZE && wparam.0 != SIZE_MINIMIZED as usize) || message == WM_DPICHANGED)
            && clip_caption(hwnd).is_err()
        {
            let _ = configure(hwnd, Some(false), Vec::new());
        }
        result
    }
}

unsafe extern "system" fn host_procedure(
    child: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    // SAFETY: The host shares the parent UI thread. Its cookie is only an HWND;
    // no allocation is owned here. Parent removal and child destruction detach it.
    unsafe {
        if message == WM_NCDESTROY {
            let _ = RemoveWindowSubclass(child, Some(host_procedure), HOST_SUBCLASS_ID);
        }
        let result = DefSubclassProc(child, message, wparam, lparam);
        let parent = HWND(data as *mut _);
        // DWM updates caption geometry after the nested size messages return.
        // Queue one sync on the owned parent instead of reading stale button bounds.
        if message == WM_SIZE
            && enabled(parent)
            && PostMessageW(Some(parent), FRAME_SYNC, WPARAM(0), LPARAM(0)).is_err()
        {
            let _ = configure(parent, Some(false), Vec::new());
        }
        result
    }
}

fn caption_hit(bounds: [f64; 4], inset: f64, x: f64, y: f64) -> Option<u32> {
    let [left, top, width, height] = bounds;
    let start = left + inset;
    if width <= inset || x < start || x >= left + width || y < top || y >= top + height {
        return None;
    }
    let button = ((x - start) / ((width - inset) / 3.0)).floor() as usize;
    [HTMINBUTTON, HTMAXBUTTON, HTCLOSE].get(button).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn caption_buttons_use_native_bounds_and_resize_inset() {
        for (bounds, inset) in [
            ([755.0, 0.0, 146.0, 30.0], 8.0),
            ([1773.0, -8.0, 146.0, 30.0], 8.0),
        ] {
            let start = bounds[0] + inset;
            for (offset, hit) in [(23.0, HTMINBUTTON), (69.0, HTMAXBUTTON), (115.0, HTCLOSE)] {
                assert_eq!(caption_hit(bounds, inset, start + offset, 14.0), Some(hit));
            }
            assert_eq!(caption_hit(bounds, inset, bounds[0], 14.0), None);
            assert_eq!(
                caption_hit(bounds, inset, bounds[0] + bounds[2], 14.0),
                None
            );
            assert_eq!(
                caption_hit(bounds, inset, start, bounds[1] + bounds[3]),
                None
            );
        }
    }
    #[test]
    fn signed_screen_coordinates_include_monitors_left_and_above_primary() {
        for (x, y) in [(-1920, -1080), (-1, -1), (1920, 1080)] {
            let packed = ((y as u16 as usize) << 16) | (x as u16 as usize);
            let actual = screen_point(packed as isize);
            assert_eq!((actual.x, actual.y), (x, y));
        }
    }
    #[test]
    fn invalid_renderer_regions_do_not_touch_native_handle() {
        for r in [
            TitlebarControlRect {
                x: 0.0,
                y: 0.0,
                width: f64::NAN,
                height: 1.0,
            },
            TitlebarControlRect {
                x: 0.0,
                y: 51.0,
                width: 20.0,
                height: 2.0,
            },
            TitlebarControlRect {
                x: -1.0,
                y: 0.0,
                width: 20.0,
                height: 20.0,
            },
        ] {
            assert!(configure(HWND::default(), Some(true), vec![r]).is_err());
        }
    }
    #[test]
    fn legacy_registration_cannot_enable_a_caption_extension() {
        assert!(configure(
            HWND::default(),
            None,
            vec![TitlebarControlRect {
                x: 0.0,
                y: 80.0,
                width: 10.0,
                height: 20.0,
            }]
        )
        .is_ok());
        assert!(!enabled(HWND::default()));
    }
}
