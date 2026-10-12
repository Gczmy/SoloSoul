//! RF-312: Android-only, non-default IPC transport observation.
//! The script preserves the original receiver, arguments, return and error.
//! It records command names and send timestamps, never payloads or responses.

pub const INITIALIZATION_SCRIPT: &str =
    concat!(";\n", include_str!("android_native_perf/observer.js"));
