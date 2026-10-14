//! RF-312: Android-only, non-default IPC transport observation.
//! The script preserves the original receiver, arguments, return and error.
//! It records command names and send timestamps, never payloads or responses.

const INITIALIZATION_SCRIPT: &str = concat!(";\n", include_str!("android_native_perf/observer.js"));

// Android's fallback injects this as inline HTML. The HTML parser normalizes
// CRLF/CR to LF, so the framework must hash those same bytes for CSP. Windows
// checkouts may otherwise block the entire IPC initializer, not only this probe.
pub fn initialization_script() -> String {
    normalize_line_endings(INITIALIZATION_SCRIPT)
}

fn normalize_line_endings(script: &str) -> String {
    script.replace("\r\n", "\n").replace('\r', "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lf_content_is_byte_identical() {
        let script = ";\nconst value = 'public';\n";
        assert_eq!(normalize_line_endings(script), script);
    }

    #[test]
    fn crlf_and_cr_preserve_code_and_trailing_line_breaks() {
        let expected = ";\nconst value = '公开';\n\n// end\n";
        assert_eq!(
            normalize_line_endings(&expected.replace('\n', "\r\n")),
            expected
        );
        assert_eq!(
            normalize_line_endings(&expected.replace('\n', "\r")),
            expected
        );
    }

    #[test]
    fn normalization_is_idempotent_for_mixed_line_endings() {
        let normalized = normalize_line_endings("a\r\nb\rc\n\r\n");
        assert_eq!(normalized, "a\nb\nc\n\n");
        assert_eq!(normalize_line_endings(&normalized), normalized);
    }

    #[test]
    fn embedded_script_has_no_cr_and_keeps_original_source_content() {
        let script = initialization_script();
        assert!(script.starts_with(";\n"));
        assert!(!script.contains('\r'));
        assert_eq!(
            script.replace('\n', ""),
            INITIALIZATION_SCRIPT.replace(['\r', '\n'], "")
        );
    }
}
