//! A panic inside a `guarded()` body must become error JSON, never an unwind
//! across the `extern "C"` boundary.

use std::ffi::CStr;

use crate::{Guard::Fn::guarded, aphrodite_hermes_free_string};

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
