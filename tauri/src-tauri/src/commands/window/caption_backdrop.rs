//! One mouse-transparent child surface for the background; never captures user pixels.
use super::{TitlebarBackdropColor, WindowBackdropLayer};
use std::cell::RefCell;
use windows::{
    core::w,
    Win32::{
        Foundation::{
            COLORREF, HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM,
        },
        Graphics::Gdi::*,
        UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
    },
};
const CLASS: windows::core::PCWSTR = w!("SoloSoul.CaptionBackdrop.v1");
const PROPERTY: windows::core::PCWSTR = w!("SoloSoul.CaptionBackdrop.v1");
const COLOR_PROPERTY: windows::core::PCWSTR = w!("SoloSoul.CaptionBackdrop.Rgba");
#[derive(Default)]
struct State {
    layers: Vec<WindowBackdropLayer>,
    dirty: bool,
    geometry: Option<(i32, i32, u32, bool)>,
}

fn owned_layer(owner: HWND) -> Result<Option<HWND>, String> {
    unsafe {
        let handle = GetPropW(owner, PROPERTY);
        if handle.is_invalid() {
            return Ok(None);
        }
        let layer = HWND(handle.0);
        let mut class = [0u16; 64];
        let length = GetClassNameW(layer, &mut class);
        if !IsWindow(Some(layer)).as_bool()
            || GetParent(layer).ok() != Some(owner)
            || GetWindowLongPtrW(layer, GWL_STYLE) as u32 & WS_CHILD.0 == 0
            || String::from_utf16_lossy(&class[..length.max(0) as usize])
                != "SoloSoul.CaptionBackdrop.v1"
        {
            return Err("Caption backdrop ownership changed".into());
        }
        Ok(Some(layer))
    }
}
fn state(layer: HWND) -> Result<&'static RefCell<State>, String> {
    // SAFETY: Only called for our retained private child, on its UI thread.
    let pointer = unsafe { GetWindowLongPtrW(layer, GWLP_USERDATA) } as *mut RefCell<State>;
    if pointer.is_null() {
        return Err("Window backdrop state unavailable".into());
    }
    Ok(unsafe { &*pointer })
}
pub(super) fn clear(owner: HWND) -> Result<(), String> {
    if let Some(layer) = owned_layer(owner)? {
        unsafe {
            RemovePropW(owner, PROPERTY).map_err(|e| e.to_string())?;
            DestroyWindow(layer).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
pub(super) fn prepare(owner: HWND) -> Result<(), String> {
    if owned_layer(owner)?.is_some() {
        return Ok(());
    }
    unsafe {
        let instance = HINSTANCE(GetWindowLongPtrW(owner, GWLP_HINSTANCE) as *mut _);
        let class = WNDCLASSW {
            lpfnWndProc: Some(procedure),
            hInstance: instance,
            lpszClassName: CLASS,
            ..Default::default()
        };
        if RegisterClassW(&class) == 0 {
            let mut previous = WNDCLASSW::default();
            GetClassInfoW(Some(instance), CLASS, &mut previous).map_err(|e| e.to_string())?;
            if previous.lpfnWndProc.map(|p| p as usize) != Some(procedure as *const () as usize) {
                return Err("Window backdrop class is owned by another adapter".into());
            }
        }
        let layer = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE,
            CLASS,
            w!(""),
            WS_CHILD,
            0,
            0,
            1,
            1,
            Some(owner),
            None,
            Some(instance),
            None,
        )
        .map_err(|e| e.to_string())?;
        SetWindowLongPtrW(
            layer,
            GWLP_USERDATA,
            Box::into_raw(Box::new(RefCell::new(State::default()))) as isize,
        );
        // Exercises actual layered-child support before installing this frame.
        if let Err(error) = upload(layer, 1, 1, &[0]).and_then(|()| {
            SetPropW(owner, PROPERTY, Some(HANDLE(layer.0))).map_err(|e| e.to_string())
        }) {
            let _ = DestroyWindow(layer);
            return Err(error);
        }
    }
    Ok(())
}
pub(super) fn set(owner: HWND, color: TitlebarBackdropColor) -> Result<(), String> {
    prepare(owner)?;
    let layer = owned_layer(owner)?.ok_or("Window backdrop unavailable")?;
    let layers = color.layers.unwrap_or_else(|| {
        vec![WindowBackdropLayer {
            red: color.red,
            green: color.green,
            blue: color.blue,
            alpha: color.alpha,
            full_window: false,
            exclusions: Vec::new(),
        }]
    });
    {
        let mut state = state(layer)?
            .try_borrow_mut()
            .map_err(|_| "Window backdrop is busy")?;
        state.layers = layers;
        state.dirty = true;
    }
    let packed = u32::from(color.red)
        | (u32::from(color.green) << 8)
        | (u32::from(color.blue) << 16)
        | (u32::from(color.alpha) << 24);
    unsafe {
        SetPropW(
            layer,
            COLOR_PROPERTY,
            Some(HANDLE(packed as usize as *mut _)),
        )
        .map_err(|e| e.to_string())?;
    }
    if let Err(error) = sync(owner) {
        let _ = clear(owner);
        return Err(error);
    }
    Ok(())
}
pub(super) fn sync(owner: HWND) -> Result<(), String> {
    let Some(layer) = owned_layer(owner)? else {
        return Ok(());
    };
    let (layers, dirty, previous) = {
        let state = state(layer)?
            .try_borrow()
            .map_err(|_| "Window backdrop is busy")?;
        (state.layers.clone(), state.dirty, state.geometry)
    };
    unsafe {
        if !layers.iter().any(|l| l.alpha > 0)
            || !IsWindowVisible(owner).as_bool()
            || IsIconic(owner).as_bool()
        {
            let _ = ShowWindow(layer, SW_HIDE);
            return Ok(());
        }
        let mut client = RECT::default();
        GetClientRect(owner, &mut client).map_err(|e| e.to_string())?;
        let full = layers.iter().any(|l| l.full_window);
        let width = client.right;
        let height = if full {
            client.bottom
        } else {
            client.bottom.min(
                (super::windows_frame::HEADER_HEIGHT * f64::from(GetDpiForWindow(owner)) / 96.0)
                    .ceil() as i32,
            )
        };
        let dpi = GetDpiForWindow(owner);
        let caption = GetWindowLongPtrW(owner, GWL_STYLE) as u32 & WS_CAPTION.0 != 0;
        let geometry = (width, height, dpi, caption);
        if dirty || previous != Some(geometry) {
            let rectangles = if layers.iter().any(|l| !l.full_window) {
                super::windows_frame::caption_rectangles(owner)?
            } else {
                Vec::new()
            };
            let pixels = super::backdrop_raster::compose(
                width as usize,
                height as usize,
                f64::from(dpi) / 96.0,
                &layers,
                &rectangles,
            )?;
            SetWindowPos(
                layer,
                Some(HWND_TOP),
                0,
                0,
                width,
                height,
                SWP_NOACTIVATE | SWP_NOREDRAW,
            )
            .map_err(|e| e.to_string())?;
            upload(layer, width, height, &pixels)?; // A single atomic surface update, including native caption pixels.
            let mut state = state(layer)?
                .try_borrow_mut()
                .map_err(|_| "Window backdrop is busy")?;
            state.geometry = Some(geometry);
            state.dirty = false;
        }
        let _ = ShowWindow(layer, SW_SHOWNOACTIVATE);
    }
    Ok(())
}
fn upload(layer: HWND, width: i32, height: i32, pixels: &[u32]) -> Result<(), String> {
    if width <= 0
        || height <= 0
        || (width as usize).checked_mul(height as usize) != Some(pixels.len())
    {
        return Err("Invalid backdrop bitmap dimensions".into());
    }
    // SAFETY: A checked, top-down 32-bit DIB containing only generated colors and cutouts.
    // No screen read or card content; every selected object/DC is restored and released.
    unsafe {
        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() {
            return Err("Cannot allocate backdrop DC".into());
        }
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let bitmap = match CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0) {
            Ok(bitmap) => bitmap,
            Err(e) => {
                let _ = DeleteDC(dc);
                return Err(e.to_string());
            }
        };
        let previous = SelectObject(dc, HGDIOBJ(bitmap.0));
        let result = (|| {
            if previous.is_invalid() || previous.0 as isize == -1 || bits.is_null() {
                return Err("Cannot select backdrop bitmap".into());
            }
            std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits.cast::<u32>(), pixels.len());
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            UpdateLayeredWindow(
                layer,
                None,
                None,
                Some(&SIZE {
                    cx: width,
                    cy: height,
                }),
                Some(dc),
                Some(&POINT::default()),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            )
            .map_err(|e| e.to_string())
        })();
        let _ = SelectObject(dc, previous);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(dc);
        result
    }
}
unsafe extern "system" fn procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        if message == WM_NCHITTEST {
            return LRESULT(HTTRANSPARENT as isize);
        }
        if message == WM_MOUSEACTIVATE {
            return LRESULT(MA_NOACTIVATE as isize);
        }
        if message == WM_NCDESTROY {
            let pointer = SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) as *mut RefCell<State>;
            if !pointer.is_null() {
                drop(Box::from_raw(pointer));
            }
        }
        DefWindowProcW(hwnd, message, wparam, lparam)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_layered_child_creation_reuses_its_handle_and_hides_on_close() {
        struct Parent(HWND);
        impl Drop for Parent {
            fn drop(&mut self) {
                unsafe {
                    let _ = DestroyWindow(self.0);
                }
            }
        }
        unsafe {
            let parent = Parent(
                CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    w!("STATIC"),
                    w!("Public caption lifecycle test"),
                    WS_OVERLAPPEDWINDOW,
                    0,
                    0,
                    900,
                    600,
                    None,
                    None,
                    None,
                    None,
                )
                .unwrap(),
            );
            prepare(parent.0).unwrap();
            let layer = owned_layer(parent.0).unwrap().unwrap();
            assert_eq!(GetParent(layer).unwrap(), parent.0);
            set(
                parent.0,
                TitlebarBackdropColor {
                    red: 31,
                    green: 28,
                    blue: 24,
                    alpha: 102,
                    layers: None,
                },
            )
            .unwrap();
            assert_eq!(
                (GetPropW(layer, COLOR_PROPERTY).0 as usize >> 24) & 255,
                102
            );
            set(
                parent.0,
                TitlebarBackdropColor {
                    red: 0,
                    green: 0,
                    blue: 0,
                    alpha: 0,
                    layers: Some(vec![]),
                },
            )
            .unwrap();
            prepare(parent.0).unwrap();
            assert_eq!(owned_layer(parent.0).unwrap(), Some(layer));
            assert!(!IsWindowVisible(layer).as_bool());
            clear(parent.0).unwrap();
            assert!(!IsWindow(Some(layer)).as_bool());
            assert!(owned_layer(parent.0).unwrap().is_none());
        }
    }
}
