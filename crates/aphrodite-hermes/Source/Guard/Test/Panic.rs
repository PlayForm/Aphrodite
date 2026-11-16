//! A panic inside a `guarded()` body must become error JSON, never an unwind
//! across the `extern "C"` boundary.

use std::ffi::CStr;

use crate::{ABI::FreeString::aphrodite_hermes_free_string, Guard::Fn::guarded};

// ── T2 (F1): a panic anywhere inside a `guarded()` body must surface as
// an error JSON, not unwind across the extern "C" boundary. ──
#[test]
fn test_guarded_converts_panic_to_error_json() {
	let ptr = guarded(|| panic!("deliberate test panic"));
	let json = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
	let v:serde_json::Value = serde_json::from_str(&json).unwrap();
	assert!(v["error"].as_str().unwrap().contains("panicked"));
	aphrodite_hermes_free_string(ptr);
}

// ── A payload whose `Drop` panics must not unwind a second time. The guard
// leaks the payload rather than dropping it, so the answer is still error JSON;
// dropping it here (the pre-fix `unwrap_or_else(|_| ...)` shape) would run
// `Boom::drop` inside the unwind and abort the harness instead of failing. ──
#[test]
fn test_guarded_survives_a_payload_that_panics_on_drop() {
	struct Boom;

	impl Drop for Boom {
		fn drop(&mut self) { panic!("payload drop must never run") }
	}

	let ptr = guarded(std::panic::AssertUnwindSafe(|| -> *mut std::os::raw::c_char {
		std::panic::panic_any(Boom)
	}));

	let json = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
	let v:serde_json::Value = serde_json::from_str(&json).unwrap();
	assert!(
		v["error"].as_str().unwrap().contains("panicked"),
		"a panicking payload must still produce error JSON, got: {json}"
	);
	aphrodite_hermes_free_string(ptr);
}
