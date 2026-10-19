use tauri::Manager;
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::{
        Shell::{DefSubclassProc, GetWindowSubclass, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::{WM_NCDESTROY, WM_SETTINGCHANGE, WM_SYSCOLORCHANGE, WM_THEMECHANGED},
    },
};

const SUBCLASS_ID: usize = 0x53535448;

unsafe extern "system" fn procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _: usize,
    cookie: usize,
) -> LRESULT {
    if message == WM_NCDESTROY {
        // SAFETY: cookie 的所有权仅属于此 subclass，在窗口 UI 线程注销后释放一次。
        unsafe {
            let _ = RemoveWindowSubclass(hwnd, Some(procedure), SUBCLASS_ID);
            drop(Box::from_raw(cookie as *mut tauri::AppHandle));
        }
    } else if matches!(
        message,
        WM_SETTINGCHANGE | WM_THEMECHANGED | WM_SYSCOLORCHANGE
    ) {
        // 不解引用消息的可选 lParam 指针；设置通知较少，统一重读并去重。
        // SAFETY: subclass 存续期间 cookie 是同一线程内存活的 AppHandle。
        super::notify(unsafe { &*(cookie as *const tauri::AppHandle) });
    }
    // SAFETY: 始终交给系统及其他 subclass，保留 Tao、Snap、DPI 与窗框行为。
    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
}

pub(super) fn install(app: &tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Main window unavailable")?;
    let hwnd = window.hwnd().map_err(|error| error.to_string())?;
    let mut existing = 0;
    // SAFETY: setup 在 HWND 所属 UI 线程执行；此 observer 不依赖一体窗框是否启用。
    unsafe {
        if GetWindowSubclass(hwnd, Some(procedure), SUBCLASS_ID, Some(&mut existing)).as_bool() {
            return Ok(());
        }
        let pointer = Box::into_raw(Box::new(app.clone()));
        if !SetWindowSubclass(hwnd, Some(procedure), SUBCLASS_ID, pointer as usize).as_bool() {
            drop(Box::from_raw(pointer));
            return Err("Cannot observe Windows system appearance".into());
        }
    }
    Ok(())
}
