//! Wedged-lock degrade: a held session mutex must fail open, fast.

use std::ffi::{CStr, CString};

use crate::{Session::Registry::Struct::session_state, aphrodite_hermes_call_hook, aphrodite_hermes_free_string};

// ── Wedged-lock degrade probe (gateway shared-plugin fix): simulate an
// abandoned worker by HOLDING a session's state mutex, then call the
// hook for THAT session. It must return the fail-open value within the
// bounded-lock window (~100ms), never block toward Hermes' 50s hook
// timeout (the pre-fix behavior that hung the gateway). ──
#[test]
fn probe_wedged_session_lock_degrades_fail_open() {
	let _g = crate::test_guard();
	// Simulate the abandoned worker: hold this session's lock.
	let state = session_state("probe-wedged");
	let _wedged_guard = state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);

	// pre_tool_call for the wedged session -> null (no directive; the
	// tool proceeds normally).
	let t0 = std::time::Instant::now();
	let args = serde_json::json!({
		"tool_name": "terminal",
		"args": {"command": "cargo build"},
		"session_id": "probe-wedged",
	})
	.to_string();
	let ptr = aphrodite_hermes_call_hook(
		CString::new("pre_tool_call").unwrap().as_ptr(),
		CString::new(args).unwrap().as_ptr(),
	);
	let result = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
	aphrodite_hermes_free_string(ptr);
	let elapsed_ms = t0.elapsed().as_millis();
	assert_eq!(result, "null", "wedged pre_tool_call must fail open to null, got: {result}");
	assert!(
		elapsed_ms < 500,
		"wedged hook must return within ~200ms (bounded lock), took {elapsed_ms}ms"
	);

	// transform_tool_result for the wedged session -> the ORIGINAL
	// content string (never erase output).
	let content = "wedged-original-output-line\n".repeat(50);
	let args = serde_json::json!({
		"tool_name": "terminal",
		"result": content,
		"session_id": "probe-wedged",
	})
	.to_string();
	let t0 = std::time::Instant::now();
	let ptr = aphrodite_hermes_call_hook(
		CString::new("transform_tool_result").unwrap().as_ptr(),
		CString::new(args).unwrap().as_ptr(),
	);
	let hook_result = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
	aphrodite_hermes_free_string(ptr);
	let elapsed_ms = t0.elapsed().as_millis();
	let returned:String =
		serde_json::from_str(&hook_result).expect("wedged transform must return the original content string");
	assert_eq!(
		returned, content,
		"wedged transform_tool_result must return the ORIGINAL content, never erase it"
	);
	assert!(
		elapsed_ms < 500,
		"wedged transform must return within ~200ms (bounded lock), took {elapsed_ms}ms"
	);
}
