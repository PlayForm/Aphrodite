//! The `post_llm_call` arm: the turn must be archived, not just counted.

use std::ffi::{CStr, CString};

use crate::ABI::{CallHook::aphrodite_hermes_call_hook, FreeString::aphrodite_hermes_free_string};

// ── T13 (F11): the production hook-dispatch path Python actually calls
// (`aphrodite_hermes_call_hook("post_llm_call", ...)`) must archive the
// turn's marker, not just advance the counter - a prior fix routed
// `Hook::post_llm_call` through `crates/aphrodite/Source/Library.rs`'s
// separate FFI dispatch, but this crate's own `call_hook` bypassed it
// entirely by calling `next_turn` directly, so `aphrodite_diff` still
// returned zero turns in real Hermes usage even after that fix. ──
#[test]
fn test_call_hook_post_llm_call_archives_turn_for_aphrodite_diff() {
	let _g = crate::Session::Guard::Fn::test_guard();
	aphrodite_hermes_call_hook(
		CString::new("session_start").unwrap().as_ptr(),
		CString::new("{}").unwrap().as_ptr(),
	);

	let compress_args = CString::new(serde_json::json!({"content": "x".repeat(5000)}).to_string()).unwrap();
	let compress_ptr = crate::ABI::DispatchTool::aphrodite_hermes_dispatch_tool(
		CString::new("aphrodite_compress").unwrap().as_ptr(),
		compress_args.as_ptr(),
	);
	unsafe { CStr::from_ptr(compress_ptr) }.to_string_lossy().into_owned();
	aphrodite_hermes_free_string(compress_ptr);

	let hook_ptr = aphrodite_hermes_call_hook(
		CString::new("post_llm_call").unwrap().as_ptr(),
		CString::new("{}").unwrap().as_ptr(),
	);
	aphrodite_hermes_free_string(hook_ptr);

	let diff_ptr = crate::ABI::DispatchTool::aphrodite_hermes_dispatch_tool(
		CString::new("aphrodite_diff").unwrap().as_ptr(),
		CString::new("{}").unwrap().as_ptr(),
	);
	let diff_result = unsafe { CStr::from_ptr(diff_ptr) }.to_string_lossy().into_owned();
	let v:serde_json::Value = serde_json::from_str(&diff_result).unwrap();
	aphrodite_hermes_free_string(diff_ptr);

	assert_eq!(v["total"], 1, "aphrodite_diff must report the archived turn: {v:?}");
}
