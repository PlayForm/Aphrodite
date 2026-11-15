//! The session-start arm.

use std::ffi::{CStr, CString};

use crate::aphrodite_hermes_call_hook;

#[test]
fn test_call_hook_session_start() {
	let _g = crate::test_guard();
	let hook = CString::new("session_start").unwrap();
	let args = CString::new("{}").unwrap();
	let result_ptr = aphrodite_hermes_call_hook(hook.as_ptr(), args.as_ptr());
	let result = unsafe { CStr::from_ptr(result_ptr) }.to_string_lossy().into_owned();
	let v:serde_json::Value = serde_json::from_str(&result).unwrap();
	assert_eq!(v["status"], "ok");
	crate::aphrodite_hermes_free_string(result_ptr);
}
