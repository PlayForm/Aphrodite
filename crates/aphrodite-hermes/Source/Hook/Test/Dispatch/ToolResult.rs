//! The `transform_tool_result` arm: telemetry and lossless retrieval.

use std::ffi::{CStr, CString};

use crate::{aphrodite_hermes_call_hook, aphrodite_hermes_free_string, with_shared};

// ── 05-T8 (P2): the bridge must parse the status/error kwargs Hermes ships
// on a tool result and record a failing telemetry event. ──
#[test]
fn test_call_hook_tool_result_records_error_event() {
	let _g = crate::test_guard();
	aphrodite_hermes_call_hook(
		CString::new("session_start").unwrap().as_ptr(),
		CString::new("{}").unwrap().as_ptr(),
	);
	let args = serde_json::json!({
		"tool_name": "terminal",
		"result": "error[E0382]: borrow of moved value\n".repeat(50),
		"status": "error",
		"error_type": "compile_error",
		"error_message": "E0382: borrow of moved value",
		"args": {"command": "cargo build"},
	})
	.to_string();
	let hook_ptr = aphrodite_hermes_call_hook(
		CString::new("transform_tool_result").unwrap().as_ptr(),
		CString::new(args).unwrap().as_ptr(),
	);
	aphrodite_hermes_free_string(hook_ptr);

	with_shared(|state| {
		let ev = state.ToolEvents.back().expect("bridge must record a tool event");
		assert!(!ev.Okay, "status=error must record a failing event");
		assert!(ev.ErrorSig.is_some(), "a failing event must carry an error_sig");
	});
}

// ── 01-F1: `transform_tool_result` is the hook Hermes fires on EVERY
// tool result, automatically - not the rarely-called `aphrodite_compress`
// tool. It must unwrap Hermes JSON wrappers just like the tool path does,
// or every automatic compression of a wrapped terminal/search/patch
// result regresses to a useless "[json:1items 1L]" preview.
#[test]
fn test_call_hook_transform_tool_result_unwraps_hermes_wrapper() {
	let _g = crate::test_guard();
	aphrodite_hermes_call_hook(
		CString::new("session_start").unwrap().as_ptr(),
		CString::new("{}").unwrap().as_ptr(),
	);

	let wrapped = serde_json::json!({"output": "x".repeat(5000), "exit_code": 1}).to_string();
	let args = serde_json::json!({"tool_name": "terminal", "result": wrapped}).to_string();
	let hook_ptr = aphrodite_hermes_call_hook(
		CString::new("transform_tool_result").unwrap().as_ptr(),
		CString::new(args).unwrap().as_ptr(),
	);
	let hook_result = unsafe { CStr::from_ptr(hook_ptr) }.to_string_lossy().into_owned();
	let marker_str:String = serde_json::from_str(&hook_result).expect("a marker string, not null");
	aphrodite_hermes_free_string(hook_ptr);

	assert!(
		!marker_str.contains("[json:"),
		"hook path must unwrap the Hermes wrapper for preview, got marker: {marker_str}"
	);

	let (hash, preview) = with_shared(|state| {
		let last = state.RecentMarkers.back().expect("hook must record a marker");
		(last.hash.clone(), last.Preview.clone())
	});
	assert!(
		!preview.starts_with("[json:"),
		"recorded preview must reflect the unwrapped payload: {preview}"
	);

	let retrieved = crate::Tool::Dispatch::Fn("aphrodite_retrieve", &serde_json::json!({"hash": hash}).to_string());
	assert_eq!(retrieved["found"], true);
	assert_eq!(
		retrieved["content"].as_str().unwrap(),
		wrapped,
		"retrieve must return the original wrapper losslessly, not just the extracted output"
	);
}
