use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Classify content — returns a preview string.
/// Caller owns the returned string and must free it with `aphrodite_free_string`.
#[no_mangle]
pub extern "C" fn aphrodite_classify(content: *const c_char) -> *mut c_char {
    let input = unsafe { CStr::from_ptr(content) }.to_string_lossy();
    let preview = classify(&input);
    CString::new(preview).unwrap().into_raw()
}

/// Free a string previously returned by any aphrodite_* function.
#[no_mangle]
pub extern "C" fn aphrodite_free_string(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(s);
    }
}

/// Get the library version.
#[no_mangle]
pub extern "C" fn aphrodite_version() -> *mut c_char {
    CString::new(env!("CARGO_PKG_VERSION")).unwrap().into_raw()
}

// ── Internal logic (what you'd swap by rebuilding) ──────────────────────────

fn classify(content: &str) -> String {
    let lines = content.lines().count();
    let chars = content.len();
    let first_line = content.lines().next().unwrap_or("").chars().take(60).collect::<String>();

    if content.contains("error") || content.contains("Error") {
        format!("[error:{}L {}B] {}", lines, chars, first_line)
    } else if content.contains("warning") {
        format!("[warn:{}L {}B] {}", lines, chars, first_line)
    } else {
        format!("[ok:{}L {}B] {}", lines, chars, first_line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_error() {
        let r = classify("error: something broke\nline 2");
        assert!(r.starts_with("[error:"));
    }

    #[test]
    fn test_classify_ok() {
        let r = classify("hello world\nall good");
        assert!(r.starts_with("[ok:"));
    }
}
