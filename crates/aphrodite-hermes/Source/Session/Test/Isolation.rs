//! Per-session state isolation: two session_ids must resolve to DIFFERENT
//! `AphroditeState` instances.

use std::ffi::{CStr, CString};

use crate::{
	Session::Registry::Struct::{DEFAULT_SESSION, session_state},
	aphrodite_hermes_call_hook,
	aphrodite_hermes_free_string,
	with_session,
	with_shared,
};

// ── Per-session state registry probe (gateway shared-plugin fix): two
// different session_ids must resolve to DIFFERENT AphroditeState
// instances. A compression stored under session A (via the production
// transform_tool_result hook path) must NOT appear in session B's
// catalog, nor in the default (tool) session - the pre-fix single global
// state made every session see everything. ──
#[test]
fn probe_per_session_state_isolation() {
	let _g = crate::test_guard();
	// Reset the default (tool) session so its catalog count is
	// deterministic (other tests leave markers behind).
	with_shared(aphrodite::Session::OnSessionStart);
	// DIRECT read (bypasses with_shared): the reset must have cleared the
	// real default state, not a throwaway fallback clone.
	let post_reset = {
		let s = session_state(DEFAULT_SESSION);
		let g = s.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
		g.RecentMarkers.len()
	};
	assert_eq!(
		post_reset, 0,
		"reset must clear the real default session state (not a fallback clone)"
	);

	// Compress under session "probe-a" through the hook path Hermes
	// actually fires (transform_tool_result with session_id in kwargs).
	let content = "probe-isolation-payload-xyz\n".repeat(400);
	let args = serde_json::json!({
		"tool_name": "terminal",
		"result": content,
		"session_id": "probe-a",
	})
	.to_string();
	let ptr = aphrodite_hermes_call_hook(
		CString::new("transform_tool_result").unwrap().as_ptr(),
		CString::new(args).unwrap().as_ptr(),
	);
	let hook_result = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
	aphrodite_hermes_free_string(ptr);
	// Compression happened: the hook returns the marker string.
	let marker:String = serde_json::from_str(&hook_result).expect("session A compress must return a marker");
	assert!(marker.starts_with("<<<CCR:"), "marker shape: {marker}");

	// Session B sees none of A's entries.
	let cat_b = with_session("probe-b", |s| serde_json::json!({"total": s.RecentMarkers.len()}))
		.expect("session B lock must be free");
	assert_eq!(cat_b["total"], 0, "session B must NOT see session A's compression");

	// Session A sees exactly its own entry.
	let cat_a = with_session("probe-a", |s| serde_json::json!({"total": s.RecentMarkers.len()}))
		.expect("session A lock must be free");
	assert_eq!(cat_a["total"], 1, "session A must see its own compression");

	// The default (tool) session - the with_shared resolution target in
	// tests, where last_session() is empty - must NOT see it either.
	let cat_def = with_shared(|s| serde_json::json!({"total": s.RecentMarkers.len()}));
	assert_eq!(
		cat_def["total"],
		0,
		"default session must NOT see session A's compression: last_session={:?}",
		crate::Debug::Session::Last::Fn()
	);
}
