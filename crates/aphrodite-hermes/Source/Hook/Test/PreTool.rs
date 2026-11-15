//! The `pre_tool_call` rewrites: auto-backgrounding and chain splitting.

use std::ffi::{CStr, CString};

use crate::{aphrodite_hermes_call_hook, aphrodite_hermes_free_string, with_shared};

// ── Poll-worker: pre_tool_call auto-backgrounding tests ──

#[test]
fn test_pre_tool_call_auto_backgrounds_terminal() {
	let _g = crate::test_guard();
	aphrodite_hermes_call_hook(
		CString::new("session_start").unwrap().as_ptr(),
		CString::new("{}").unwrap().as_ptr(),
	);
	with_shared(|state| state.poll_worker_enabled = true);

	let args = serde_json::json!({
		"tool_name": "terminal",
		"args": {"command": "cargo build --release"},
	})
	.to_string();
	let ptr = aphrodite_hermes_call_hook(
		CString::new("pre_tool_call").unwrap().as_ptr(),
		CString::new(args).unwrap().as_ptr(),
	);
	let result = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
	aphrodite_hermes_free_string(ptr);

	let v:serde_json::Value = serde_json::from_str(&result).unwrap();
	assert_eq!(v["action"], "modify", "pre_tool_call must return modify action: {result}");
	assert_eq!(v["args"]["background"], true);
	assert_eq!(v["args"]["notify_on_complete"], true);
}

#[test]
fn test_pre_tool_call_does_not_background_poll_action() {
	let _g = crate::test_guard();
	aphrodite_hermes_call_hook(
		CString::new("session_start").unwrap().as_ptr(),
		CString::new("{}").unwrap().as_ptr(),
	);
	with_shared(|state| state.poll_worker_enabled = true);

	let args = serde_json::json!({
		"tool_name": "process",
		"args": {"action": "poll"},
	})
	.to_string();
	let ptr = aphrodite_hermes_call_hook(
		CString::new("pre_tool_call").unwrap().as_ptr(),
		CString::new(args).unwrap().as_ptr(),
	);
	let result = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
	aphrodite_hermes_free_string(ptr);

	assert_eq!(result, "null", "process poll must pass through unchanged");
}

#[test]
fn test_pre_tool_call_disabled_passes_through() {
	let _g = crate::test_guard();
	aphrodite_hermes_call_hook(
		CString::new("session_start").unwrap().as_ptr(),
		CString::new("{}").unwrap().as_ptr(),
	);
	with_shared(|state| state.poll_worker_enabled = false);

	let args = serde_json::json!({
		"tool_name": "terminal",
		"args": {"command": "cargo build --release"},
	})
	.to_string();
	let ptr = aphrodite_hermes_call_hook(
		CString::new("pre_tool_call").unwrap().as_ptr(),
		CString::new(args).unwrap().as_ptr(),
	);
	let result = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
	aphrodite_hermes_free_string(ptr);

	assert_eq!(result, "null", "disabled flag must pass through: {result}");
}

// ── Tier 1 teaching loop: the adaptive split threshold gates the
// rewrite - chains below `chain_split_min_segments` pass through
// untouched (no markers), chains at/above it get rewritten. ──
#[test]
fn test_pre_tool_call_chain_split_respects_adaptive_threshold() {
	let _g = crate::test_guard();
	aphrodite_hermes_call_hook(
		CString::new("session_start").unwrap().as_ptr(),
		CString::new("{}").unwrap().as_ptr(),
	);
	with_shared(|state| {
		state.poll_worker_enabled = false;
		state.chain_split_enabled = true;
		// Threshold raised to 3: a 2-segment chain must NOT be split.
		state.chain_split_min_segments = 3;
	});

	// Below threshold: 2 segments → pass through (null).
	let args2 = serde_json::json!({
		"tool_name": "terminal",
		"args": {"command": "cd x && cargo build"},
	})
	.to_string();
	let ptr = aphrodite_hermes_call_hook(
		CString::new("pre_tool_call").unwrap().as_ptr(),
		CString::new(args2).unwrap().as_ptr(),
	);
	let result2 = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
	aphrodite_hermes_free_string(ptr);
	assert_eq!(result2, "null", "2-segment chain below threshold must pass through: {result2}");

	// At/above threshold: 3 segments → rewritten with markers.
	let args3 = serde_json::json!({
		"tool_name": "terminal",
		"args": {"command": "cd x && cargo build && cargo test"},
	})
	.to_string();
	let ptr = aphrodite_hermes_call_hook(
		CString::new("pre_tool_call").unwrap().as_ptr(),
		CString::new(args3).unwrap().as_ptr(),
	);
	let result3 = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
	aphrodite_hermes_free_string(ptr);
	let v3:serde_json::Value = serde_json::from_str(&result3).unwrap();
	assert_eq!(v3["action"], "modify", "3-segment chain must be rewritten: {result3}");
	assert!(
		v3["args"]["command"].as_str().unwrap().contains("__APHRODITE_SEG__"),
		"rewritten command must carry segment markers: {result3}"
	);
}
