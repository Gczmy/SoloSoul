//! 仅 owned 测试进程的原生文件选择器；不枚举窗口正文，不操作外部窗口。
use super::super::{publish, Outcome};
use serde_json::{json, Value};
use std::{
    path::Path,
    time::{Duration, Instant},
};
use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::*;
const MAX_CONTROLS: usize = 128;
fn hwnd(value: usize) -> HWND {
    HWND(value as *mut std::ffi::c_void)
}
fn pid(window: HWND) -> u32 {
    let mut value = 0;
    unsafe {
        GetWindowThreadProcessId(window, Some(&mut value));
    }
    value
}
fn class(window: HWND) -> Outcome<String> {
    let mut text = [0u16; 64];
    let n = unsafe { GetClassNameW(window, &mut text) };
    if n <= 0 || n as usize >= text.len() - 1 {
        return Err("ocr-picker-class-invalid");
    }
    String::from_utf16(&text[..n as usize]).map_err(|_| "ocr-picker-class-invalid")
}
struct Enumeration {
    main: usize,
    windows: Vec<usize>,
    overflow: bool,
}
unsafe extern "system" fn dialogs(window: HWND, data: LPARAM) -> BOOL {
    // 同步 EnumWindows 的上下文在回调全部返回前保持有效；外部 HWND 只查询 PID。
    let context = unsafe { &mut *(data.0 as *mut Enumeration) };
    if pid(window) == std::process::id()
        && unsafe { IsWindowVisible(window) }.as_bool()
        && unsafe { GetWindow(window, GW_OWNER) }
            .is_ok_and(|owner| owner.0 as usize == context.main)
        && class(window).is_ok_and(|value| value == "#32770")
    {
        context.windows.push(window.0 as usize);
        if context.windows.len() > 4 {
            context.overflow = true;
            return BOOL(0);
        }
    }
    BOOL(1)
}
unsafe extern "system" fn children(window: HWND, data: LPARAM) -> BOOL {
    let context = unsafe { &mut *(data.0 as *mut Enumeration) };
    if pid(window) != std::process::id() || context.windows.len() >= MAX_CONTROLS {
        context.overflow = true;
        return BOOL(0);
    }
    context.windows.push(window.0 as usize);
    BOOL(1)
}
fn identity(main: usize, dialog: usize) -> Outcome<()> {
    let m = hwnd(main);
    let d = hwnd(dialog);
    if !unsafe { IsWindow(Some(m)) }.as_bool()
        || !unsafe { IsWindow(Some(d)) }.as_bool()
        || pid(m) != std::process::id()
        || pid(d) != std::process::id()
        || class(d)? != "#32770"
        || !unsafe { IsWindowVisible(d) }.as_bool()
        || !unsafe { GetWindow(d, GW_OWNER) }.is_ok_and(|owner| owner == m)
        || unsafe { GetAncestor(GetForegroundWindow(), GA_ROOT) } != d
    {
        return Err("ocr-picker-owner-or-foreground-mismatch");
    }
    Ok(())
}
fn controls(dialog: usize) -> Outcome<Vec<Value>> {
    let mut context = Enumeration {
        main: dialog,
        windows: vec![],
        overflow: false,
    };
    unsafe {
        let _ = EnumChildWindows(
            Some(hwnd(dialog)),
            Some(children),
            LPARAM(&mut context as *mut _ as isize),
        );
    }
    if context.overflow || context.windows.is_empty() {
        return Err("ocr-picker-controls-over-budget");
    }
    context
        .windows
        .into_iter()
        .map(|handle| control_snapshot(dialog, handle))
        .collect()
}
fn control_snapshot(dialog: usize, handle: usize) -> Outcome<Value> {
    let window = hwnd(handle);
    // 先验证进程与顶层归属，再读取这一个 owned 控件的结构；不读取正文。
    if !unsafe { IsWindow(Some(window)) }.as_bool()
        || pid(window) != std::process::id()
        || unsafe { GetAncestor(window, GA_ROOT) } != hwnd(dialog)
    {
        return Err("ocr-picker-control-owner-mismatch");
    }
    let parent = unsafe { GetParent(window) }.map_err(|_| "ocr-picker-parent-unavailable")?;
    let style = unsafe { GetWindowLongW(window, GWL_STYLE) } as u32;
    let name = class(window)?;
    let password = name == "Edit" && style & ES_PASSWORD as u32 != 0;
    Ok(
        json!({"hwnd":handle,"parent":parent.0 as usize,"class":name,"id":unsafe { GetDlgCtrlID(window) },"visible":unsafe { IsWindowVisible(window) }.as_bool(),"enabled":style&WS_DISABLED.0==0,"password":password}),
    )
}
fn action_control(main: usize, dialog: usize, tree: &[Value], mut handle: usize) -> Outcome<()> {
    identity(main, dialog)?;
    // 每次消息前重查目标和父链，拒绝 HWND 复用、重挂或状态变化。
    for _ in 0..8 {
        if handle == dialog {
            return Ok(());
        }
        let expected = tree
            .iter()
            .find(|v| v["hwnd"] == handle)
            .ok_or("ocr-picker-control-parent-invalid")?;
        if control_snapshot(dialog, handle)? != *expected {
            return Err("ocr-picker-controls-changed");
        }
        handle = expected["parent"]
            .as_u64()
            .ok_or("ocr-picker-control-invalid")? as usize;
    }
    Err("ocr-picker-control-parent-invalid")
}
// 明确的现代 Windows 文件名组合框，不按标题/任意 Edit 猜测。未知结构保留原件并失败。
fn select(tree: &[Value], dialog: usize) -> Outcome<(usize, usize)> {
    let combos: Vec<_> = tree
        .iter()
        .filter(|v| {
            v["class"] == "ComboBoxEx32"
                && v["id"] == 1148
                && v["visible"] == true
                && v["enabled"] == true
        })
        .collect();
    if combos.len() != 1 {
        return Err("ocr-picker-filename-combo-unavailable");
    }
    let combo = combos[0]["hwnd"]
        .as_u64()
        .ok_or("ocr-picker-control-invalid")? as usize;
    let mut edits = vec![];
    for v in tree.iter().filter(|v| {
        v["class"] == "Edit"
            && v["visible"] == true
            && v["enabled"] == true
            && v["password"] == false
    }) {
        let mut parent = v["parent"].as_u64().ok_or("ocr-picker-control-invalid")? as usize;
        for _ in 0..8 {
            if parent == combo {
                edits.push(v["hwnd"].as_u64().ok_or("ocr-picker-control-invalid")? as usize);
                break;
            }
            if parent == dialog {
                break;
            }
            let entry = tree.iter().find(|v| v["hwnd"] == parent);
            let Some(next) = entry.and_then(|v| v["parent"].as_u64()) else {
                return Err("ocr-picker-control-parent-invalid");
            };
            parent = next as usize;
        }
    }
    let buttons: Vec<_> = tree
        .iter()
        .filter(|v| {
            v["parent"] == dialog
                && v["class"] == "Button"
                && v["id"] == 1
                && v["visible"] == true
                && v["enabled"] == true
        })
        .collect();
    if edits.len() != 1 || buttons.len() != 1 {
        return Err("ocr-picker-controls-ambiguous");
    }
    Ok((
        edits[0],
        buttons[0]["hwnd"]
            .as_u64()
            .ok_or("ocr-picker-control-invalid")? as usize,
    ))
}
pub(super) fn choose(main: usize, root: &Path, started: Instant, opening: f64) -> Outcome<Value> {
    let began = Instant::now();
    let dialog = loop {
        if began.elapsed() > Duration::from_secs(25) {
            return Err("ocr-picker-open-timeout");
        }
        if pid(hwnd(main)) != std::process::id() {
            return Err("ocr-picker-main-owner-mismatch");
        }
        let mut context = Enumeration {
            main,
            windows: vec![],
            overflow: false,
        };
        unsafe { EnumWindows(Some(dialogs), LPARAM(&mut context as *mut _ as isize)) }
            .map_err(|_| "ocr-picker-enumeration-failed")?;
        if context.overflow || context.windows.len() > 1 {
            return Err("ocr-picker-dialog-ambiguous");
        }
        if let Some(value) = context.windows.first() {
            break *value;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    identity(main, dialog)?;
    let opened = started.elapsed().as_secs_f64() * 1000.0;
    let tree = controls(dialog)?;
    publish(
        root,
        "native-perf-ocr-picker-structure.json",
        &json!({"schemaVersion":1,"scope":"owned-ocr-native-file-picker-structure","pid":std::process::id(),"mainHwnd":main,"dialogHwnd":dialog,"directOwnerVerified":true,"foregroundVerified":true,"maxControls":MAX_CONTROLS,"controls":tree}),
    )?;
    let (edit, button) = select(&tree, dialog)?;
    super::asset_intact(root)?;
    identity(main, dialog)?;
    if controls(dialog)? != tree {
        return Err("ocr-picker-controls-changed");
    }
    let path = super::input_path(root);
    let path = path.to_str().ok_or("ocr-picker-path-invalid")?;
    let plain = path.strip_prefix(r"\\?\").unwrap_or(path);
    let mut text: Vec<u16> = plain.encode_utf16().chain(Some(0)).collect();
    let mut result = 0usize;
    action_control(main, dialog, &tree, edit)?;
    let sent = unsafe {
        SendMessageTimeoutW(
            hwnd(edit),
            WM_SETTEXT,
            WPARAM(0),
            LPARAM(text.as_mut_ptr() as isize),
            SMTO_ABORTIFHUNG | SMTO_ERRORONEXIT,
            2000,
            Some(&mut result),
        )
    };
    if sent.0 == 0 {
        // 超时不能证明接收线程已用完同进程指针。只在该失败进程保留这份有界公开路径。
        std::mem::forget(text);
        return Err("ocr-picker-settext-timeout");
    }
    if result == 0 {
        return Err("ocr-picker-settext-failed");
    }
    identity(main, dialog)?;
    let mut actual = vec![0u16; text.len() + 1];
    let mut received = 0usize;
    action_control(main, dialog, &tree, edit)?;
    let read = unsafe {
        SendMessageTimeoutW(
            hwnd(edit),
            WM_GETTEXT,
            WPARAM(actual.len()),
            LPARAM(actual.as_mut_ptr() as isize),
            SMTO_ABORTIFHUNG | SMTO_ERRORONEXIT,
            2000,
            Some(&mut received),
        )
    };
    if read.0 == 0 {
        std::mem::forget(actual);
        return Err("ocr-picker-readback-timeout");
    }
    if received != text.len() - 1 || actual[..received] != text[..received] {
        return Err("ocr-picker-path-readback-mismatch");
    }
    super::asset_intact(root)?;
    identity(main, dialog)?;
    action_control(main, dialog, &tree, button)?;
    let selected = started.elapsed().as_secs_f64() * 1000.0;
    unsafe { PostMessageW(Some(hwnd(button)), BM_CLICK, WPARAM(0), LPARAM(0)) }
        .map_err(|_| "ocr-picker-open-click-failed")?;
    let closing = Instant::now();
    while unsafe { IsWindow(Some(hwnd(dialog))) }.as_bool() {
        if closing.elapsed() > Duration::from_secs(5) {
            return Err("ocr-picker-close-timeout");
        }
        if pid(hwnd(dialog)) != std::process::id() {
            return Err("ocr-picker-dialog-owner-changed");
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    super::asset_intact(root)?;
    Ok(
        json!({"schemaVersion":1,"scope":"owned-ocr-native-file-picker","pid":std::process::id(),"mainHwnd":main,"dialogHwnd":dialog,"directOwnerVerified":true,"foregroundVerified":true,"controls":tree,"filenameEditHwnd":edit,"openButtonHwnd":button,"inputMethod":"WM_SETTEXT+readback+BM_CLICK","inputPathVerified":true,"actionStartedAtMs":opening,"openedObservedAtMs":opened,"selectionSubmittedAtMs":selected,"closedObservedAtMs":started.elapsed().as_secs_f64()*1000.0,"closedVerified":true}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    fn control(handle: usize, parent: usize, name: &str, id: i32) -> Value {
        json!({"hwnd":handle,"parent":parent,"class":name,"id":id,"visible":true,"enabled":true,"password":false})
    }
    fn tree() -> Vec<Value> {
        vec![
            control(2, 1, "ComboBoxEx32", 1148),
            control(3, 2, "ComboBox", 1001),
            control(4, 3, "Edit", 1001),
            control(5, 1, "Button", 1),
        ]
    }
    #[test]
    fn exact_native_filename_chain_and_button_are_unique() {
        assert_eq!(select(&tree(), 1).unwrap(), (4, 5));
    }
    #[test]
    fn unrelated_or_hidden_password_edits_and_ambiguous_buttons_are_rejected() {
        let mut v = tree();
        v[2]["password"] = json!(true);
        assert!(select(&v, 1).is_err());
        let mut v = tree();
        v[2]["parent"] = json!(1);
        assert!(select(&v, 1).is_err());
        let mut v = tree();
        v[3]["visible"] = json!(false);
        assert!(select(&v, 1).is_err());
        let mut v = tree();
        v.push(control(6, 3, "Edit", 1001));
        assert!(select(&v, 1).is_err());
        let mut v = tree();
        v.push(control(6, 1, "Button", 1));
        assert!(select(&v, 1).is_err());
        let mut v = tree();
        v[0]["id"] = json!(1001);
        assert!(select(&v, 1).is_err());
    }
}
