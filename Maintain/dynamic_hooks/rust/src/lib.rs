use std::ffi::{CStr, CString};
use std::os::raw::c_char;

// ── Public C ABI (the stable surface) ─────────────────────────────────────
//   These signatures NEVER change. New hooks are added inside `call_hook`.

/// Return JSON array of hook names this dylib handles.
/// Caller must free with aphrodite_free_string.
#[no_mangle]
pub extern "C" fn aphrodite_hooks() -> *mut c_char {
    CString::new(r#"["session_start","transform_tool_result","transform_terminal_output"]"#)
        .unwrap()
        .into_raw()
}

/// Dispatch a hook call. Returns JSON result or error.
/// - hook_name: one of the strings from aphrodite_hooks()
/// - json_args: JSON object with hook-specific fields
/// Caller must free with aphrodite_free_string.
#[no_mangle]
pub extern "C" fn aphrodite_call_hook(
    hook_name: *const c_char,
    json_args: *const c_char,
) -> *mut c_char {
    let name = unsafe { CStr::from_ptr(hook_name) }.to_string_lossy();
    let args = unsafe { CStr::from_ptr(json_args) }.to_string_lossy();

    let result = match name.as_ref() {
        "session_start" => on_session_start(&args),
        "transform_tool_result" => transform_tool_result(&args),
        "transform_terminal_output" => transform_terminal_output(&args),
        other => format!(r#"{{"error":"unknown hook: {}"}}"#, other),
    };

    CString::new(result).unwrap().into_raw()
}

/// Get dylib version.
#[no_mangle]
pub extern "C" fn aphrodite_version() -> *mut c_char {
    CString::new(env!("CARGO_PKG_VERSION")).unwrap().into_raw()
}

/// Free a string returned by any aphrodite_* function.
#[no_mangle]
pub extern "C" fn aphrodite_free_string(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(s);
    }
}

// ── Hook implementations (this is what you edit + rebuild) ─────────────────

fn on_session_start(_args: &str) -> String {
    format!(
        r#"{{"status":"ok","msg":"💋 aphrodite v{} - dylib loaded"}}"#,
        env!("CARGO_PKG_VERSION")
    )
}

fn transform_tool_result(args: &str) -> String {
    let content = extract_field(args, "content").unwrap_or_default();
    let preview = classify(&content);
    format!(r#"{{"status":"ok","preview":"{}"}}"#, escape_json(&preview))
}

fn transform_terminal_output(args: &str) -> String {
    let content = extract_field(args, "content").unwrap_or_default();
    let preview = classify(&content);
    format!(r#"{{"status":"ok","preview":"{}"}}"#, escape_json(&preview))
}

// ── Classifier (your compression logic lives here) ─────────────────────────

fn classify(content: &str) -> String {
    let lines = content.lines().count();
    let chars = content.len();
    let first_line = content
        .lines()
        .next()
        .unwrap_or("")
        .chars()
        .take(60)
        .collect::<String>();

    if content.contains("error") || content.contains("Error") {
        format!("[error:{}L {}B] {}", lines, chars, first_line)
    } else if content.contains("warning") {
        format!("[warn:{}L {}B] {}", lines, chars, first_line)
    } else {
        format!("[ok:{}L {}B] {}", lines, chars, first_line)
    }
}

// ── Tiny JSON helpers (zero-dependency, no serde) ──────────────────────────

fn extract_field(json: &str, key: &str) -> Option<String> {
    let pat = format!(r#""{}""#, key);
    let start = json.find(&pat)? + pat.len();
    let after = &json[start..];
    let colon = after.find(':')?;
    let val_start = after[colon + 1..].trim_start();

    if val_start.starts_with('"') {
        let end = val_start[1..].find('"')?;
        Some(unescape(&val_start[1..=end]))
    } else {
        let end = val_start
            .find(|c: char| c == ',' || c == '}')
            .unwrap_or(val_start.len());
        Some(val_start[..end].trim().to_string())
    }
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn unescape(s: &str) -> String {
    s.replace("\\\"", "\"")
        .replace("\\\\", "\\")
        .replace("\\n", "\n")
        .replace("\\r", "\r")
        .replace("\\t", "\t")
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hooks_list() {
        let hooks = unsafe { CStr::from_ptr(aphrodite_hooks()) }.to_string_lossy();
        assert!(hooks.contains("session_start"));
        assert!(hooks.contains("transform_tool_result"));
        assert!(hooks.contains("transform_terminal_output"));
    }

    #[test]
    fn test_call_unknown_hook() {
        let r = aphrodite_call_hook(
            CString::new("bogus").unwrap().as_ptr(),
            CString::new("{}").unwrap().as_ptr(),
        );
        let out = unsafe { CStr::from_ptr(r) }.to_string_lossy();
        assert!(out.contains("unknown hook"));
        aphrodite_free_string(r);
    }

    #[test]
    fn test_call_session_start() {
        let r = aphrodite_call_hook(
            CString::new("session_start").unwrap().as_ptr(),
            CString::new(r#"{"session_id":"abc"}"#).unwrap().as_ptr(),
        );
        let out = unsafe { CStr::from_ptr(r) }.to_string_lossy();
        assert!(out.contains("dylib loaded"));
        aphrodite_free_string(r);
    }

    #[test]
    fn test_call_transform_tool_result() {
        let r = aphrodite_call_hook(
            CString::new("transform_tool_result").unwrap().as_ptr(),
            CString::new(r#"{"content":"error: broke\nline2","tool_name":"test"}"#)
                .unwrap()
                .as_ptr(),
        );
        let out = unsafe { CStr::from_ptr(r) }.to_string_lossy();
        assert!(out.contains("[error:"));
        aphrodite_free_string(r);
    }

    #[test]
    fn test_classify_error() {
        let r = classify("error: something broke\nline 2");
        assert!(r.starts_with("[error:"));
    }

    #[test]
    fn test_extract_field() {
        let j = r#"{"content":"hello world","tool":"test"}"#;
        assert_eq!(extract_field(j, "content"), Some("hello world".into()));
        assert_eq!(extract_field(j, "tool"), Some("test".into()));
        assert_eq!(extract_field(j, "missing"), None);
    }
}
