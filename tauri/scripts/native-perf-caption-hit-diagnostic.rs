//! Read-only metadata for a refused caption hit in the external Windows verifier.
//! This file is included by the isolated verifier, never by the application.
//! HWND relationships are transient observations and cannot authorize input.

use serde_json::{json, Value};
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::UI::WindowsAndMessaging::{
    GetAncestor, GetForegroundWindow, GetWindowThreadProcessId, IsWindow, GA_ROOT, GA_ROOTOWNER,
};

fn window_metadata(hwnd: HWND) -> Value {
    let mut pid = 0;
    // These getters accept foreign HWNDs; no text, pixels or process handles are read.
    let thread = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    json!({
        "hwnd": hwnd.0 as usize,
        "windowExistsAtRead": unsafe { IsWindow(Some(hwnd)) }.as_bool(),
        "threadId": thread,
        "pid": pid,
        "threadAndPidAvailable": thread != 0 && pid != 0,
    })
}

/// `hit` must be the result of the original guard's WindowFromPoint call.
/// Record the failing observation without a second hit test or any input action.
/// Handles can change while these getters execute; this is not identity proof.
pub fn rejected_caption_hit(target: HWND, point: POINT, hit: HWND) -> Value {
    let root = unsafe { GetAncestor(hit, GA_ROOT) };
    let root_owner = unsafe { GetAncestor(hit, GA_ROOTOWNER) };
    let foreground = unsafe { GetForegroundWindow() };
    json!({
        "schemaVersion": 1,
        "scope": "rf1130-refused-caption-hit-metadata",
        "pointScreen": [point.x, point.y],
        "target": window_metadata(target),
        "hit": window_metadata(hit),
        "hitRoot": window_metadata(root),
        "hitRootOwner": window_metadata(root_owner),
        "foreground": window_metadata(foreground),
        "hitEqualsTarget": hit == target,
        "hitRootEqualsTarget": root == target,
        "hitRootOwnerEqualsTarget": root_owner == target,
        "atomicIdentityProof": false,
        "authorizesInput": false,
        "diagnosticOnly": true,
        "inputEventsSentByDiagnostic": 0,
        "pixelsRead": 0,
        "windowTextRead": false,
        "systemSettingsChanged": false,
    })
}
