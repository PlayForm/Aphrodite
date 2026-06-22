//! aphrodite-hermes: Hermes Agent-specific integration crate.
//!
//! This crate handles all Hermes-specific concerns — tool schemas,
//! hook dispatch, skill registration — leaving the core `aphrodite`
//! crate as a pure, agent-agnostic compression engine.
//!
//! Architecture:
//!   Python plugin (thin loader) → ctypes → libaphrodite_hermes.dylib
//!                                           ├─ Tool dispatch (compress, retrieve, stats, etc.)
//!                                           ├─ Hook dispatch (session_start, transform, terminal)
//!                                           └─ Skill registration
//!                                           ↓ (depends on)
//!                                    aphrodite crate (rlib)
//!                                           ├─ Core compression (hooks, state, marker)
//!                                           ├─ Resolution (resolve, stage2, struct)
//!                                           └─ Catalog, session, prefetch, config

mod tools;
mod schemas;
mod skills;

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

// Re-export core aphrodite types for convenience
pub use aphrodite::state::AphroditeState;

// ── C ABI helpers ──────────────────────────────────────────

unsafe fn cstr_to_string(ptr: *const c_char) -> String {
    if ptr.is_null() { String::new() } else { CStr::from_ptr(ptr).to_string_lossy().into_owned() }
}

fn to_c_string(s: &str) -> *mut c_char {
    CString::new(s).map(|c| c.into_raw()).unwrap_or(std::ptr::null_mut())
}

fn to_json_error(msg: &str) -> *mut c_char {
    to_c_string(&serde_json::json!({"error": msg}).to_string())
}

// ── Tool dispatch C ABI ────────────────────────────────────

/// Dispatch an aphrodite tool call by name.
/// Returns JSON result string. Caller must free with aphrodite_hermes_free_string.
#[no_mangle]
pub extern "C" fn aphrodite_hermes_dispatch_tool(
    tool_name: *const c_char,
    args_json: *const c_char,
) -> *mut c_char {
    let name = unsafe { cstr_to_string(tool_name) };
    let args = unsafe { cstr_to_string(args_json) };

    let result = tools::dispatch(&name, &args);
    match serde_json::to_string(&result) {
        Ok(json) => to_c_string(&json),
        Err(e) => to_json_error(&format!("serialize error: {}", e)),
    }
}

/// List all registered Hermes tool schemas as JSON array.
#[no_mangle]
pub extern "C" fn aphrodite_hermes_list_tools() -> *mut c_char {
    let schemas = schemas::all_schemas();
    to_c_string(&serde_json::to_string(&schemas).unwrap_or_default())
}

/// List all bundled skill names and descriptions as JSON.
#[no_mangle]
pub extern "C" fn aphrodite_hermes_list_skills() -> *mut c_char {
    let skills = skills::all_skills();
    to_c_string(&serde_json::to_string(&skills).unwrap_or_default())
}

/// Get a single tool schema by name.
#[no_mangle]
pub extern "C" fn aphrodite_hermes_get_schema(
    tool_name: *const c_char,
) -> *mut c_char {
    let name = unsafe { cstr_to_string(tool_name) };
    match schemas::get_schema(&name) {
        Some(s) => to_c_string(&serde_json::to_string(&s).unwrap_or_default()),
        None => to_json_error(&format!("unknown tool: {}", name)),
    }
}

/// Free a string returned by any aphrodite_hermes_* function.
#[no_mangle]
pub extern "C" fn aphrodite_hermes_free_string(s: *mut c_char) {
    if !s.is_null() { unsafe { let _ = CString::from_raw(s); } }
}

/// Version of this crate.
#[no_mangle]
pub extern "C" fn aphrodite_hermes_version() -> *mut c_char {
	to_c_string(&serde_json::json!({"version": env!("CARGO_PKG_VERSION")}).to_string())
}

// ── Hook dispatch C ABI ────────────────────────────────────

/// Call a Hermes hook by name with JSON args.
/// Returns JSON result.
#[no_mangle]
pub extern "C" fn aphrodite_hermes_call_hook(
    hook_name: *const c_char,
    args_json: *const c_char,
) -> *mut c_char {
    let name = unsafe { cstr_to_string(hook_name) };
    let args = unsafe { cstr_to_string(args_json) };

    // Parse args as JSON object
    let parsed: serde_json::Value = match serde_json::from_str(&args) {
        Ok(v) => v,
        Err(e) => return to_json_error(&format!("invalid args: {}", e)),
    };

    let content = parsed.get("content").and_then(|v| v.as_str()).unwrap_or("");
    let tool = parsed.get("tool_name").and_then(|v| v.as_str()).unwrap_or("unknown");

    // Create a new state for stateless hook calls
    let mut state = AphroditeState::default();

    let result = match name.as_ref() {
        "session_start" => {
            let r = aphrodite::session::on_session_start(&mut state);
            serde_json::to_value(&r).unwrap_or_default()
        }
        "transform_tool_result" => {
            aphrodite::hooks::transform_tool_result(&mut state, content, tool)
        }
        "transform_terminal_output" => {
            aphrodite::hooks::transform_terminal_output(&mut state, content)
        }
        "pre_llm_call" => {
            aphrodite::hooks::pre_llm_call(&state)
        }
        "post_llm_call" => {
            aphrodite::hooks::post_llm_call(&mut state)
        }
        _ => {
            return to_json_error(&format!("unknown hook: {}", name));
        }
    };

    to_c_string(&serde_json::to_string(&result).unwrap_or_default())
}

/// Return all tool schemas as a JSON array.
#[no_mangle]
pub extern "C" fn aphrodite_hermes_get_schemas() -> *mut c_char {
	let schemas = schemas::all_schemas();
	to_c_string(&serde_json::json!(schemas).to_string())
}

/// Return hook names as a JSON array.
#[no_mangle]
pub extern "C" fn aphrodite_hermes_get_hooks() -> *mut c_char {
	to_c_string(&serde_json::json!([
		"session_start",
		"transform_tool_result",
		"transform_terminal_output",
		"pre_llm_call",
		"post_llm_call"
	]).to_string())
}

/// Probe proxy health via TCP connect.
#[no_mangle]
pub extern "C" fn aphrodite_hermes_proxy_health() -> *mut c_char {
	use std::net::TcpStream;
	use std::time::Duration;
	let timeout = Duration::from_secs(1);
	let token_alive = TcpStream::connect_timeout(
		&"127.0.0.1:9798".parse().unwrap(), timeout
	).is_ok();
	let cache_alive = TcpStream::connect_timeout(
		&"127.0.0.1:9797".parse().unwrap(), timeout
	).is_ok();
	to_c_string(&serde_json::json!({
		"token": {"alive": token_alive},
		"cache": {"alive": cache_alive},
	}).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_version_is_semver() {
        let json_ptr = aphrodite_hermes_version();
        let json = unsafe { CStr::from_ptr(json_ptr) }.to_string_lossy().into_owned();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        let ver = v["version"].as_str().unwrap();
        assert!(
            ver.starts_with("0.") || ver.starts_with("1."),
            "expected semver starting with 0. or 1., got: {ver}"
        );
        aphrodite_hermes_free_string(json_ptr);
    }

    #[test]
    fn test_list_tools_returns_array() {
        let json_ptr = aphrodite_hermes_list_tools();
        let json = unsafe { CStr::from_ptr(json_ptr) }.to_string_lossy().into_owned();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(v.is_array());
        assert!(v.as_array().unwrap().len() >= 10);
        aphrodite_hermes_free_string(json_ptr);
    }

    #[test]
    fn test_list_skills_returns_array() {
        let json_ptr = aphrodite_hermes_list_skills();
        let json = unsafe { CStr::from_ptr(json_ptr) }.to_string_lossy().into_owned();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(v.is_array());
        aphrodite_hermes_free_string(json_ptr);
    }

    #[test]
    fn test_dispatch_unknown_tool() {
        let name = CString::new("nonexistent").unwrap();
        let args = CString::new("{}").unwrap();
        let result_ptr = aphrodite_hermes_dispatch_tool(name.as_ptr(), args.as_ptr());
        let result = unsafe { CStr::from_ptr(result_ptr) }.to_string_lossy().into_owned();
        assert!(result.contains("error"));
        aphrodite_hermes_free_string(result_ptr);
    }

    #[test]
    fn test_call_hook_session_start() {
        let hook = CString::new("session_start").unwrap();
        let args = CString::new("{}").unwrap();
        let result_ptr = aphrodite_hermes_call_hook(hook.as_ptr(), args.as_ptr());
        let result = unsafe { CStr::from_ptr(result_ptr) }.to_string_lossy().into_owned();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["status"], "ok");
        aphrodite_hermes_free_string(result_ptr);
    }

    #[test]
    fn test_get_schema_known_tool() {
        let name = CString::new("aphrodite_compress").unwrap();
        let result_ptr = aphrodite_hermes_get_schema(name.as_ptr());
        let result = unsafe { CStr::from_ptr(result_ptr) }.to_string_lossy().into_owned();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["name"], "aphrodite_compress");
        aphrodite_hermes_free_string(result_ptr);
    }
}
